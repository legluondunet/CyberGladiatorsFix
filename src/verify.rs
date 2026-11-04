// SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only checks. Asset hashes are recorded during extraction and are
//! independently recomputed before launch, including the original backup.
use crate::{install::{DIR,MANIFEST},cdda,nglide,patch,settings};
use sha2::{Digest,Sha256};
use std::fs::{self,File};
use std::io::{Read,Write};
use std::path::{Path,Component};
use std::process::{Child,Command,Stdio};
fn file_hash(path:&Path)->Result<(u64,String),String>{
    let m=fs::symlink_metadata(path).map_err(|e|format!("Missing {}: {e}",path.display()))?;
    if !m.is_file()||m.file_type().is_symlink(){return Err(format!("Not a regular file: {}",path.display()));}
    let mut f=File::open(path).map_err(|e|e.to_string())?;
    let mut digest=Sha256::new();
    let mut n=0u64;
    let mut b=[0u8;64*1024];
    loop {let got=f.read(&mut b).map_err(|e|e.to_string())?;
        if got==0 {break;}digest.update(&b[..got]);n+=got as u64;
    }
    Ok((n,format!("{:x}",digest.finalize())))
}
fn asset_mismatch_message(name:&str,expected_size:u64,expected_sha256:&str,actual_size:u64,actual_sha256:&str)->String{
    format!(
        "Game asset has changed: {name} (expected size={expected_size}, SHA-256={expected_sha256}; actual size={actual_size}, SHA-256={actual_sha256}). Original CD assets must not be silently overwritten or trusted as mutable; preserve the changed file for comparison."
    )
}
fn safe_manifest_path(s:&str)->bool{
    let p=Path::new(s);
    if s.is_empty()||s.contains(['\t','\n','\r',':','\\'])||p.is_absolute(){return false;}
    p.components().all(|c|matches!(c,Component::Normal(_)))
}
pub fn verify(root:&Path)->Result<String,String>{
    let game=root.join(DIR);
    let meta=fs::symlink_metadata(&game).map_err(|e|format!("Missing game directory: {e}"))?;
    if !meta.is_dir()||meta.file_type().is_symlink(){return Err("Game folder is not a regular directory".into());}
    patch::verify_exe(&game.join("cyber.exe"))?;
    let manifest=fs::read_to_string(game.join(MANIFEST)).map_err(|e|format!("Manifest missing: {e}"))?;
    if manifest.len()>4*1024*1024||!manifest.starts_with("# CyberGladiatorsFix manifest v1\n"){
        return Err("Missing or invalid extraction manifest".into());
    }
    let mut count=0usize;let mut bytes=0u64;
    let mut seen=std::collections::BTreeSet::<String>::new();
    for line in manifest.lines().skip(1){
        let mut parts=line.split('\t');
        let name=parts.next().ok_or("Empty manifest row")?;
        let size=parts.next().ok_or("Missing manifest size")?.parse::<u64>().map_err(|e|e.to_string())?;
        let hash=parts.next().ok_or("Missing manifest SHA-256")?;
        if parts.next().is_some()||hash.len()!=64||!hash.bytes().all(|c|c.is_ascii_hexdigit())||
            !safe_manifest_path(name)||!seen.insert(name.to_ascii_lowercase()) {
            return Err(format!("Invalid manifest entry: {name}"));
        }
        let mut path=game.clone();
        // A local manifest must never cause verification to traverse a
        // symlinked directory outside the installation.
        for component in Path::new(name).components(){
            let Component::Normal(part)=component else{return Err(format!("Unsafe manifest path: {name}"))};
            path.push(part);
            let meta=fs::symlink_metadata(&path).map_err(|e|format!("Missing manifest path {}: {e}",path.display()))?;
            if meta.file_type().is_symlink(){return Err(format!("Symlink in manifest path: {}",path.display()));}
        }
        let (actual_size,actual_hash)=file_hash(&path)?;
        if actual_size!=size||actual_hash!=hash {
            return Err(asset_mismatch_message(name,size,hash,actual_size,&actual_hash));
        }
        count+=1;bytes+=size;
    }
    nglide::verify(&game)?;
    let audio=if patch::cdda_active(&game.join("cyber.exe"))? {
        cdda::verify(&game)?;
        "CDDA bridge enabled and verified"
    }else{"native WinMM (CDDA bypass)"};
    Ok(format!("Verified CYBER.EXE patch + backup, {count} CD resources ({bytes} bytes), nGlide and {audio}"))
}
fn require_cd_audio_for_passthru(passthru_noop:bool,enable_cdda:bool)->Result<(),String>{
    if passthru_noop && !enable_cdda {
        return Err("--passthru-noop requires [Audio] EnableCDDA=1 in CyberGladiatorsFix.ini (game folder); no files were patched".into());
    }
    Ok(())
}
pub fn launch(root:&Path,logo:bool,trace:bool,passthru_noop:bool)->Result<Child,String>{
    launch_with_video_delay(root,logo,trace,passthru_noop,0)
}
pub fn launch_with_video_delay(root:&Path,logo:bool,trace:bool,passthru_noop:bool,video_delay_ms:u64)->Result<Child,String>{
    launch_with_options(root,logo,trace,passthru_noop,video_delay_ms,false)
}
pub fn launch_with_options(root:&Path,logo:bool,trace:bool,passthru_noop:bool,video_delay_ms:u64,no_glide_hook:bool)->Result<Child,String>{
    launch_with_renderer_test(root,logo,trace,passthru_noop,video_delay_ms,no_glide_hook,None,false)
}
/// Opt-in comparison of nGlide Direct3D9 (DXVK under a configured Wine prefix)
/// versus nGlide's direct Vulkan renderer. Never assume D3D9 implies DXVK.
pub fn launch_renderer_ab(root:&Path,backend:&str,dxvk_defer_surface:bool)->Result<Child,String>{
    launch_with_renderer_test(root,false,true,false,0,true,Some(backend),dxvk_defer_surface)
}
fn validate_renderer_test(backend:Option<&str>,dxvk_defer_surface:bool,trace:bool,no_glide_hook:bool,logo:bool,passthru_noop:bool,video_delay_ms:u64)->Result<(),String>{
    if backend.is_some_and(|b|!matches!(b,"1"|"2")) || (dxvk_defer_surface && backend!=Some("1")){
        return Err("Invalid rendering experiment: nGlide backend must be 1 (DirectX) or 2 (Vulkan); deferred surface creation requires DirectX".into());
    }
    if backend.is_some() && (!trace||!no_glide_hook||logo||passthru_noop||video_delay_ms!=0){
        return Err("Renderer experiments require trace mode with the diagnostic Glide hook disabled".into());
    }
    Ok(())
}
fn launch_with_renderer_test(root:&Path,logo:bool,trace:bool,passthru_noop:bool,video_delay_ms:u64,no_glide_hook:bool,backend_override:Option<&str>,dxvk_defer_surface:bool)->Result<Child,String>{
    validate_renderer_test(backend_override,dxvk_defer_surface,trace,no_glide_hook,logo,passthru_noop,video_delay_ms)?;
    if no_glide_hook && (!trace||logo||passthru_noop||video_delay_ms!=0){
        return Err("--no-glide-hook requires --launch --trace without other experiments".into());
    }
    if video_delay_ms!=0 && (!trace||logo||passthru_noop||!matches!(video_delay_ms,100|250|500|1000)){
        return Err("Video close delay requires --launch --trace --video-delay=100|250|500|1000".into());
    }
    if passthru_noop && (!trace || logo) {
        return Err("--passthru-noop is a trace-only diagnostic experiment".into());
    }
    let game=root.join(DIR);
    // For old installations, generate the absent *patcher* INI with
    // SkipIntro=0. Never touch the game's binary cyber.ini.
    settings::ensure(&game)?;
    let skip=settings::read(&game)?;
    let enable_cdda=settings::read_cdda(&game)?;
    if no_glide_hook && (!enable_cdda || skip){
        return Err("--no-glide-hook requires [Audio] EnableCDDA=1 and [Video] SkipIntro=0".into());
    }
    if video_delay_ms!=0 && (!enable_cdda || skip){
        return Err("--video-delay requires [Video] SkipIntro=0 and [Audio] EnableCDDA=1".into());
    }
    // The diagnostic Glide hook lives exclusively inside the x86 CDDA bridge.
    // A fresh extraction defaults EnableCDDA=0, so running the NO-OP A/B
    // experiment then silently skips the hook and gives a misleading result.
    // Fail before any change to the game EXE if the bridge is not requested.
    require_cd_audio_for_passthru(passthru_noop,enable_cdda)?;
    let exe=game.join("cyber.exe");
    patch::verify_exe(&exe)?;
    // If the user explicitly opts into CDDA, copy/upgrade the project's
    // verified DLL first. Never switch the import descriptor to an absent DLL.
    if enable_cdda {cdda::try_install(root,&game)?;}
    // Prepare the exact import and intro variants *before* verification and
    // launch; disabled CDDA must not load or require the 32-bit DLL.
    patch::set_cdda(&exe,enable_cdda)?;
    patch::set_skip_intro(&exe,skip)?;
    verify(root)?;
    if logo{nglide::check_logo(&game)?;}
    if passthru_noop {
        let hash=cdda::matching_installed_hash(root,&game)?;
        // Log the actual game-folder INI, not a similarly named file beside
        // the patcher. The previous cumulative test lacked this distinction.
        crate::show(&format!(
            "PASS-THROUGH A/B PREREQUISITES: game_ini={} SkipIntro={} EnableCDDA={}; bundled and installed CDDA DLL SHA-256={} (identical); trace=1 no-op=1",
            game.join("CyberGladiatorsFix.ini").display(),
            if skip{1}else{0},
            if enable_cdda{1}else{0},
            hash
        ),root);
    }
    let mut cmd=Command::new(game.join("cyber.exe"));
    cmd.current_dir(&game);
    // Prevent ambient environment variables from changing regular launches.
    cmd.env_remove("CYBER_VIDEO_CLOSE_DELAY_MS");
    cmd.env_remove("CYBER_GLIDE_HOOK_DISABLED");
    if trace {
        // Diagnoses graphics/exception handling without modifying game bytes
        // or the Wine prefix. The trace is opt-in, stored beside the patcher,
        // and replaced by each new --launch --trace invocation.
        nglide::configure_diagnostic(&mut cmd,logo);
        if let Some(backend)=backend_override {
            // 1=DirectX9 and 2=direct Vulkan. nGlide uses its own Vulkan
            // renderer for mode 2, not DXVK. The D3D9 branch uses DXVK only
            // if the Wine prefix actually provides DXVK's d3d9.dll.
            cmd.env("NGLIDE_BACKEND",backend);
            // A/B baselines must not inherit a prior inline DXVK_CONFIG
            // value, otherwise the baseline might also use the tweak.
            cmd.env_remove("DXVK_CONFIG");
            if backend=="1" {
                // On Wine, DXVK does not necessarily write standalone logs
                // without DXVK_LOG_PATH; force them into the game's working
                // directory so a fresh cyber_d3d9.log can prove DXVK loaded.
                // Never remove or overwrite logs ourselves.
                cmd.env("DXVK_LOG_PATH",".");
                cmd.env("DXVK_HUD","version,devinfo,fps");
                cmd.env("DXVK_LOG_LEVEL","info");
                if dxvk_defer_surface {
                    // Current DXVK option for games that begin rendering through
                    // another graphics API. Defer creating the VkSurface
                    // until the first D3D9 present. No persistent changes.
                    cmd.env("DXVK_CONFIG","d3d9.deferSurfaceCreation = True");
                }
            }
        }
        cmd.env("WINEDEBUG","+seh,+loaddll");
        // Only the debug launch observes the native 3dfx-to-VGA transition.
        // The x86 CDDA DLL refuses unknown game/Glide pointers and leaves
        // all game files and normal --launch behavior untouched.
        cmd.env("CYBER_GLIDE_TRACE","1");
        if no_glide_hook {
            cmd.env("CYBER_GLIDE_HOOK_DISABLED","1");
        }
        if video_delay_ms!=0{
            cmd.env("CYBER_VIDEO_CLOSE_DELAY_MS",video_delay_ms.to_string());
        }
        if passthru_noop {
            // Explicit A/B experiment: the 32-bit bridge suppresses only
            // grSstPassthruMode(1), in memory, after the movie has closed.
            // The option is never enabled on ordinary --launch.
            cmd.env("CYBER_GLIDE_PASSTHRU_NOOP","1");
        }
        let log=root.join("CyberGladiatorsFix-WineTrace.log");
        if log.is_symlink(){return Err("Refusing symlink Wine trace log".into());}
        let mut output=fs::OpenOptions::new().create(true).write(true).truncate(true)
            .open(&log).map_err(|e|format!("Cannot create Wine trace log: {e}"))?;
        // Proton consumes this option before starting Wine; we record the
        // value visible to the launcher, not an unverified claim about which
        // d3d9.dll implementation was actually loaded.
        let proton_wined3d=match std::env::var("PROTON_USE_WINED3D").as_deref(){
            Ok("1")=>"1",Ok("0")=>"0",Ok("")=>"empty",
            Ok(_)=>"other",Err(_)=>"unset",
        };
        writeln!(output,"CyberGladiatorsFix --launch --trace")
            .and_then(|_|writeln!(output,"WINEDEBUG=+seh,+loaddll; NGLIDE_BACKEND={} {}; DXVK_DEFER_SURFACE={dxvk_defer_surface}; DXVK_DIAGNOSTICS={}; DXVK_LOG_PATH={}; CYBER_GLIDE_TRACE=1; CYBER_GLIDE_PASSTHRU_NOOP={}; CYBER_VIDEO_CLOSE_DELAY_MS={video_delay_ms}; CYBER_GLIDE_HOOK_DISABLED={}; PROTON_USE_WINED3D={proton_wined3d} (launcher env, not backend proof); CDDA enabled={enable_cdda}; SkipIntro={skip}",backend_override.unwrap_or("0"),if backend_override.is_some(){"(forced)"}else{"(auto)"},if backend_override==Some("1"){"HUD+log"}else{"off"},if backend_override==Some("1"){"."}else{"inherited"},if passthru_noop{1}else{0},if no_glide_hook{1}else{0}))
            .and_then(|_|output.flush())
            .map_err(|e|format!("Cannot initialize Wine trace: {e}"))?;
        let errors=output.try_clone().map_err(|e|format!("Cannot capture Wine stderr: {e}"))?;
        cmd.stdout(Stdio::from(output));
        cmd.stderr(Stdio::from(errors));
    }else{
        nglide::configure(&mut cmd,logo);
    }
    cmd.spawn().map_err(|e|format!("Unable to start CYBER.EXE: {e}"))
}

#[cfg(test)]
mod experimental_launch_tests {
    use super::*;
    #[test]
    fn asset_change_error_reports_both_size_and_hash_without_approving_changes(){
        let err=asset_mismatch_message("iq/iq40.dat",1024,"oldhash",2048,"newhash");
        assert!(err.starts_with("Game asset has changed: iq/iq40.dat"));
        assert!(err.contains("expected size=1024, SHA-256=oldhash"));
        assert!(err.contains("actual size=2048, SHA-256=newhash"));
    }
    #[test]
    fn renderer_ab_requires_explicit_safe_trace_and_valid_backends(){
        for backend in [Some("1"),Some("2")] {
            assert!(validate_renderer_test(backend,false,true,true,false,false,0).is_ok());
        }
        assert!(validate_renderer_test(Some("1"),true,true,true,false,false,0).is_ok());
        assert!(validate_renderer_test(Some("2"),true,true,true,false,false,0).is_err());
        assert!(validate_renderer_test(Some("0"),false,true,true,false,false,0).is_err());
        assert!(validate_renderer_test(Some("1"),false,true,false,false,false,0).is_err());
        assert!(validate_renderer_test(None,true,true,true,false,false,0).is_err());
    }
    #[test]
    fn video_delay_is_bounded_and_trace_only(){
        assert!(matches!(100u64,100|250|500|1000));
        assert!(matches!(250u64,100|250|500|1000));
        assert!(matches!(500u64,100|250|500|1000));
        assert!(matches!(1000u64,100|250|500|1000));
        assert!(!matches!(10000u64,100|250|500|1000));
    }
    #[test]
    fn pass_through_experiment_refuses_native_winmm_without_cd_audio_proxy() {
        let failure=require_cd_audio_for_passthru(true,false).unwrap_err();
        assert!(failure.contains("EnableCDDA=1"));
        assert!(require_cd_audio_for_passthru(true,true).is_ok());
        assert!(require_cd_audio_for_passthru(false,false).is_ok());
        assert!(require_cd_audio_for_passthru(false,true).is_ok());
    }
}
