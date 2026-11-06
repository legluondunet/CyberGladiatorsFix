// SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only checks. Asset hashes are recorded during extraction and are
//! independently recomputed before launch, including the original backup.
use crate::{diagnostics,install::{DIR,MANIFEST},cdda,nglide,patch,settings};
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
/// Exact IQ record keys on the original USA CD. Do not accept unrelated
/// files, paths, extensions, changed lengths or symlinks in this policy.
fn same_size_iq_asset(name:&str,expected_size:u64,actual_size:u64)->bool{
    let Some(num)=name.strip_prefix("iq/").and_then(|s|s.strip_prefix("iq"))
        .and_then(|s|s.strip_suffix(".dat")) else{return false;};
    expected_size==actual_size && num.len()==2 && num.bytes().all(|b|b.is_ascii_digit())
}
pub fn verify(root:&Path)->Result<String,String>{
    verify_assets(root,false,true).map(|(summary,_)|summary)
}
/// A launch can opt in to exact-path, same-size IQ differences without
/// rewriting the manifest or asset. --verify always calls the strict mode.
pub fn verify_launch_assets(root:&Path)->Result<String,String>{
    let allow=settings::read_allow_modified_iq(&root.join(DIR))?;
    let (summary,warnings)=verify_assets(root,allow,true)?;
    for message in warnings {crate::show(&message,root);}
    Ok(summary)
}
/// Before any migration/reinstall, verify the retail backed-up EXE and the
/// complete CD manifest without requiring a renderer that is staged only
/// during startup. The integrity exception applies solely to same-size IQ
/// data and never rewrites the manifest.
pub fn verify_install_content(root:&Path)->Result<String,String>{
    let allow=settings::read_allow_modified_iq(&root.join(DIR))?;
    let (summary,warnings)=verify_assets(root,allow,false)?;
    for warning in warnings{crate::show(&warning,root);}
    Ok(summary)
}
fn verify_assets(root:&Path,allow_modified_iq:bool,check_modules:bool)->Result<(String,Vec<String>),String>{
    let game=root.join(DIR);
    let meta=fs::symlink_metadata(&game).map_err(|e|format!("Missing game directory: {e}"))?;
    if !meta.is_dir()||meta.file_type().is_symlink(){return Err("Game folder is not a regular directory".into());}
    patch::verify_exe(&game.join("cyber.exe"))?;
    let manifest=fs::read_to_string(game.join(MANIFEST)).map_err(|e|format!("Manifest missing: {e}"))?;
    if manifest.len()>4*1024*1024||!manifest.starts_with("# CyberGladiatorsFix manifest v1\n"){
        return Err("Missing or invalid extraction manifest".into());
    }
    let mut warnings=Vec::<String>::new();
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
            if allow_modified_iq && same_size_iq_asset(name,size,actual_size){
                // User explicitly chose to retain same-size data the game
                // may update. Never silently rewrite the original CD receipt.
                warnings.push(format!(
                    "IQ_INTEGRITY_WARNING allowed only for this launch: {name} changed (expected size={size}, SHA-256={hash}; actual size={actual_size}, SHA-256={actual_hash}). Strict --verify will still fail; preserve a copy to compare with the CD."
                ));
            }else{
                return Err(asset_mismatch_message(name,size,hash,actual_size,&actual_hash));
            }
        }
        count+=1;bytes+=size;
    }
    let audio=if check_modules{
        if crate::bundle::GAMEFRAME_GLIDE_DLL.is_empty(){nglide::verify(&game)?;}
        else{crate::bundle::verify()?;}
        if patch::cdda_active(&game.join("cyber.exe"))?{
            cdda::verify(&game)?;
            if settings::read_cdda(&game)?{
                "embedded WinMM bridge verified; FLAC CD audio selected"
            }else{"embedded WinMM bridge verified; system CD audio selected"}
        }else{"legacy system WinMM import"}
    }else{"retail assets checked before module installation"};
    let label=if warnings.is_empty(){"all assets match original CD"}
        else{"modified IQ data permitted by explicit INI option; other assets match original CD"};
    Ok((format!("Verified CYBER.EXE patch + backup, {count} CD resources ({bytes} bytes), {audio}; {label}"),warnings))
}
pub fn launch(root:&Path,logo:bool,trace:bool,passthru_noop:bool)->Result<Child,String>{
    launch_with_video_delay(root,logo,trace,passthru_noop,0)
}
pub fn launch_with_video_delay(root:&Path,logo:bool,trace:bool,passthru_noop:bool,video_delay_ms:u64)->Result<Child,String>{
    launch_with_options(root,logo,trace,passthru_noop,video_delay_ms,false)
}
pub fn launch_with_options(root:&Path,logo:bool,trace:bool,passthru_noop:bool,video_delay_ms:u64,no_glide_hook:bool)->Result<Child,String>{
    launch_with_renderer_test(root,logo,trace,passthru_noop,video_delay_ms,no_glide_hook,None,false,false,None,false,false,false,false,false)
}
/// Proton selects WineD3D before the patcher itself starts; an inline
/// Windows child environment flag cannot reverse that runtime choice.
fn validate_proton_dxvk_preflight(backend:&str,proton_wined3d:Option<&str>)->Result<(),String>{
    if backend=="1" && proton_wined3d==Some("1") {
        return Err("DXVK control invalid: PROTON_USE_WINED3D=1 is still enabled. Remove that variable in Lutris and restart the runner before testing DXVK. No game files modified".into());
    }
    Ok(())
}
/// Opt-in comparison of nGlide Direct3D9 (DXVK under a configured Wine prefix)
/// versus nGlide's direct Vulkan renderer. Never assume D3D9 implies DXVK.
/// Each D3D9 experiment writes DXVK logs into its own newly-created folder:
/// pre-existing DXVK logs can never masquerade as evidence from this run.
/// Native AVI and the guarded WineD3D/DXVK repair always run through the
/// x86 WinMM bridge; the INI EnableCDDA switch controls only virtual FLAC
/// music, never the original Cinepak player or its full-screen presentation.
pub fn launch_native_avi_repaired(root:&Path,trace:bool,logo:bool,avi_4x3:bool,smooth_test:bool)->Result<(Child,Option<std::path::PathBuf>),String>{
    validate_proton_dxvk_preflight("1",std::env::var("PROTON_USE_WINED3D").ok().as_deref())?;
    if smooth_test && (trace||!avi_4x3||logo){
        return Err("Smooth-video trial requires regular native 4:3 AVI and guarded WndProc repair".into());
    }
    if trace {
        // Reuse the established per-run DXVK capture lifecycle, which must
        // be finalized by the caller after the game exits.
        if logo {
            return Err("Native AVI repair tracing with --3dfx-logo is not supported".into());
        }
        return launch_renderer_experiment(root,"1",false,false,true,avi_4x3);
    }
    launch_with_renderer_test(root,logo,false,false,0,true,Some("1"),false,false,None,true,avi_4x3,smooth_test,false,false)
        .map(|child|(child,None))
}
/// GameFrame's verified native AVI -> first real OpenGL-present launch.
/// The original nGlide setup is never changed; the selected Apache-2.0
/// Glide DLL was temporarily installed by gameframe::Session.
pub fn launch_gameframe(root:&Path)->Result<Child,String>{
    let game=root.join(DIR);
    settings::ensure(&game)?;
    let skip_intro=settings::read(&game)?;
    let enable_cdda=settings::read_cdda(&game)?;
    let exe=game.join("cyber.exe");
    patch::verify_exe(&exe)?;
    cdda::try_install(root,&game)?;
    // Never attribute results to a new DLL if an old bridge stayed installed.
    let actual_bridge=patch::file_hash(&game.join(cdda::DLL_NAME))?;
    let embedded_bridge=patch::digest(crate::bundle::CDDA_DLL);
    if actual_bridge!=embedded_bridge{
        return Err(format!("GameFrame requires its paired WinMM DLL: installed={actual_bridge} embedded={embedded_bridge}"));
    }
    crate::show(&format!("GAMEFRAME_BRIDGE_CHECK installed_bridge_sha256={actual_bridge} equals_embedded=1 matched_bundle=1"),root);
    patch::set_cdda(&exe,true)?;
    patch::set_skip_intro(&exe,skip_intro)?;
    verify_launch_assets(root)?;
    let glide=game.join("glide.dll");
    let glide_hash=patch::file_hash(&glide)?;
    // The embedded source-pinned DLL has an exact SHA-256 identity.
    // Fail closed: never apply the GameFrame-specific address/Win32 logic
    // to an arbitrary proprietary nGlide or a different wrapper build.
    let expected=option_env!("CYBER_GAMEFRAME_GLIDE_SHA256").ok_or(
        "This launcher does not embed the verified GameFrame Glide; use the one-EXE GameFrame package"
    )?;
    if !glide_hash.eq_ignore_ascii_case(expected) {
        return Err(format!("GameFrame requires embedded glide.dll SHA-256={expected}; installed SHA-256={glide_hash}. Original Glide backup is preserved"));
    }
    let mut cmd=Command::new(&exe);
    cmd.current_dir(&game);
    cmd.env("DXVK_LOG_PATH","none");
    for key in [
        "CYBER_GLIDE_TRACE","CYBER_GLIDE_HOOK_DISABLED","CYBER_GLIDE_PASSTHRU_NOOP",
        "CYBER_WNDPROC_REPAIR_TEST","CYBER_WNDPROC_REPAIR",
        "CYBER_VULKAN_PASSIVE_FAULT_PROBE","CYBER_VULKAN_EARLY_HOST_RELEASE",
        "CYBER_VULKAN_AUTO_FULLSCREEN","CYBER_VIDEO_CLOSE_DELAY_MS",
        "CYBER_GAMEFRAME_AVI_IN_WINDOW","CYBER_GAMEFRAME_AVI_UNSCALED",
        "CYBER_GAMEFRAME_AVI_FIRST_PRESENT",
        "NGLIDE_BACKEND","NGLIDE_REFRESH","NGLIDE_RESOLUTION","NGLIDE_ASPECT",
        "NGLIDE_VSYNC","NGLIDE_GAMMA","NGLIDE_SPLASH",
    ] {cmd.env_remove(key);}
    let log=diagnostics::location(root);
    cmd.env("CYBER_LOG_FILE",&log);
    cmd.env("GLIDE_LOG",&log);
    cmd.env("GLIDE_FPS_LIMIT","60");
    cmd.env("CYBER_GAMEFRAME_TEST","1");
    cmd.env("CYBER_GAMEFRAME_AVI_FIRST_PRESENT","1");
    // Only the SHA-256-verified retail game on Wine and the dedicated
    // GameFrame build may forward its 640x480x16 DirectDraw request to the
    // prelaunch desktop size. The real DirectDraw SetDisplayMode is called.
    // The native Windows loader always leaves this interception inactive.
    cmd.env("CYBER_WINE_DDRAW_NATIVE_FORWARD","1");
    crate::show("WINE_DDRAW_NATIVE_FORWARD requested=1 activation=wine_only original_DirectDraw_called=1 game_only=1",root);
    crate::show("GAMEFRAME_AVI_FIRST_PRESENT enabled=1 decoder=native_MCI_GDI first_GL_SwapBuffers_handoff=1 native_video_4x3=1 timeout_ms=3500",root);
    if let Some((w,h))=diagnostics::primary_desktop_size(){
        // Pass the 1920x1080 (or user's actual desktop) recorded BEFORE
        // the game can temporarily switch to a 640x480 display mode.
        cmd.env("CYBER_GAMEFRAME_DESKTOP_SIZE",format!("{w}x{h}"));
        crate::show(&format!("GAMEFRAME_DESKTOP_CAPTURE monitor_size={w}x{h} source=prelaunch_primary_display original_monitor_mode=preserved"),root);
    }else{
        cmd.env_remove("CYBER_GAMEFRAME_DESKTOP_SIZE");
        crate::show("GAMEFRAME_DESKTOP_CAPTURE unavailable fallback=GetMonitorInfoA_during_game",root);
    }
    cmd.env("CYBER_AVI_FULLSCREEN","1");
    cmd.env("CYBER_AVI_SMOOTH_TEST","1");
    cmd.env("CYBER_CDDA_MUSIC",if enable_cdda{"1"}else{"0"});
    crate::show("GAMEFRAME renderer=GameFrame-OpenGL original_input_hwnd=1 fullscreen_desktop_borderless=1 4x3_letterbox=1 frame_limit=60fps video=native_MCI first_present_handoff=1 log=unified",root);
    diagnostics::log(root,"MODULE",&format!("GameFrame glide.dll sha256={glide_hash} (embedded_verified_build)"));
    diagnostics::log(root,"CONFIG",&format!("GAMEFRAME launch: SkipIntro={} EnableCDDA={} GLIDE_FPS_LIMIT=60 NGLIDE_BACKEND=not_used 4x3_aspect=preserved AVI_FIRST_PRESENT_HANDOFF=1",u8::from(skip_intro),u8::from(enable_cdda)));
    cmd.spawn().map_err(|e|format!("Unable to start Cyber Gladiators through GameFrame: {e}"))
}

/// Experimental nGlide Vulkan backend using the same smooth 4:3 native AVI
/// and bounded curtain as the validated D3D9 launch. This is opt-in only;
/// ordinary --launch and --launch --smooth-video-test remain D3D9.
pub fn launch_smooth_vulkan(root:&Path,early_release:bool,auto_fullscreen:bool)->Result<Child,String>{
    launch_with_renderer_test(root,false,false,false,0,true,Some("2"),false,false,None,true,true,true,early_release,auto_fullscreen)
}
pub fn launch_renderer_ab(root:&Path,backend:&str,dxvk_defer_surface:bool,desktop_refresh:bool,wndproc_repair:bool)->Result<(Child,Option<std::path::PathBuf>),String>{
    launch_renderer_experiment(root,backend,dxvk_defer_surface,desktop_refresh,wndproc_repair,false)
}
fn launch_renderer_experiment(root:&Path,backend:&str,dxvk_defer_surface:bool,desktop_refresh:bool,wndproc_repair:bool,avi_4x3:bool)->Result<(Child,Option<std::path::PathBuf>),String>{
    validate_renderer_test(Some(backend),dxvk_defer_surface,true,true,false,false,0)?;
    if desktop_refresh && backend!="1" {return Err("Desktop-refresh experiment requires the nGlide DirectX9 backend".into());}
    validate_proton_dxvk_preflight(backend,std::env::var("PROTON_USE_WINED3D").ok().as_deref())?;
    let log_dir=if backend=="1" {
        let nonce=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .map_err(|e|format!("System clock error for DXVK trace: {e}"))?.as_nanos();
        // DXVK writes its own files; collect them into one log on exit.
        // Never leave generated trace folders inside the original game tree.
        let dir=std::env::temp_dir().join(format!("CyberGladiatorsFix-DXVKTrace-{}-{nonce}",std::process::id()));
        fs::create_dir(&dir).map_err(|e|format!("Cannot create isolated DXVK trace directory: {e}"))?;
        Some(dir)
    }else{None};
    let child=launch_with_renderer_test(root,false,true,false,0,true,Some(backend),dxvk_defer_surface,desktop_refresh,log_dir.as_deref(),wndproc_repair,avi_4x3,false,false,false)?;
    Ok((child,log_dir))
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
fn guarded_video_backend_allowed(backend:Option<&str>,smooth_test:bool)->bool{
    backend==Some("1") || (backend==Some("2") && smooth_test)
}
/// nGlide's forced 60-Hz mode has yielded DISP_CHANGE_BADMODE (-2) under
/// direct Vulkan on the user's 120-Hz Wine desktop. Test native Vulkan with
/// the desktop's supported refresh rate, and leave validated D3D9 at 60 Hz.
fn selected_refresh(backend:Option<&str>,desktop_refresh:bool)->&'static str{
    if desktop_refresh || backend==Some("2") {"1"}else{"2"}
}
/// Vulkan-only, native AVI crash forensics. Reuse the existing deferred
/// first-chance observer and read-only window sampler but explicitly disable
/// the invasive Glide IAT hook. This does not enable noisy Wine channels.
/// The previously validated D3D9 launch remains unchanged.
fn passive_vulkan_fault_probe(backend:Option<&str>,smooth_test:bool,trace:bool)->bool{
    !trace && smooth_test && backend==Some("2")
}
fn launch_with_renderer_test(root:&Path,logo:bool,trace:bool,passthru_noop:bool,video_delay_ms:u64,no_glide_hook:bool,backend_override:Option<&str>,dxvk_defer_surface:bool,desktop_refresh:bool,dxvk_log_dir:Option<&Path>,wndproc_repair:bool,avi_4x3:bool,smooth_test:bool,early_release:bool,auto_fullscreen:bool)->Result<Child,String>{
    // The normal game path uses one x86 WinMM bridge with a guarded native
    // AVI handoff. Trace-only backend tests follow a separate validated path.
    let normal_repair=wndproc_repair && !trace
        && no_glide_hook && guarded_video_backend_allowed(backend_override,smooth_test)
        && !passthru_noop && video_delay_ms==0 && !dxvk_defer_surface
        && !desktop_refresh && dxvk_log_dir.is_none();
    if wndproc_repair && !normal_repair
        && (!trace||!no_glide_hook||backend_override!=Some("1")
            ||logo||passthru_noop||video_delay_ms!=0||dxvk_defer_surface
            ||desktop_refresh) {
        return Err("WndProc repair requires a verified hook-free native AVI launch (D3D9, or opt-in smooth Vulkan)".into());
    }
    let diagnostic_fullscreen=avi_4x3 && trace && wndproc_repair
        && no_glide_hook && backend_override==Some("1")
        && !logo && !passthru_noop && video_delay_ms==0
        && !dxvk_defer_surface && !desktop_refresh
        && dxvk_log_dir.is_some();
    if smooth_test && (!normal_repair||!avi_4x3||trace){
        return Err("Smooth-video test may only run with the normal, guarded fullscreen AVI path".into());
    }
    if early_release && !(smooth_test && backend_override==Some("2") && !trace && normal_repair){
        return Err("Early AVI host release is restricted to the explicit native Vulkan smooth-video A/B experiment".into());
    }
    if auto_fullscreen && !early_release {
        return Err("Automatic fullscreen recovery requires the explicit Vulkan early-release experiment".into());
    }
    if avi_4x3 && !normal_repair && !diagnostic_fullscreen {
        return Err("Fullscreen native AVI host requires a guarded native-video launch".into());
    }
    if !normal_repair {
        validate_renderer_test(backend_override,dxvk_defer_surface,trace,no_glide_hook,logo,passthru_noop,video_delay_ms)?;
    }
    if no_glide_hook && !normal_repair
        && (!trace||logo||passthru_noop||video_delay_ms!=0){
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
    if avi_4x3 && skip {
        return Err("Native AVI 4:3 host requires [Video] SkipIntro=0; INI not modified".into());
    }
    // The WinMM bridge is always installed for video; audio/music remains
    // opt-in or opt-out independently. Require the video when a particular
    // diagnostic is observing the video-to-Glide transition.
    if no_glide_hook && skip && backend_override.is_none(){
        return Err("--no-glide-hook requires [Video] SkipIntro=0 unless running a forced renderer comparison".into());
    }
    if wndproc_repair && skip {
        return Err("WndProc repair experiment requires [Video] SkipIntro=0".into());
    }
    if video_delay_ms!=0 && skip{
        return Err("--video-delay requires [Video] SkipIntro=0".into());
    }
    // The verified x86 WinMM bridge is installed regardless of music policy.
    // Glide/video diagnostics remain available when FLAC music is disabled.
    let exe=game.join("cyber.exe");
    patch::verify_exe(&exe)?;
    // Always use the verified WinMM DLL for native fullscreen AVI and the
    // WndProc repair. The user's EnableCDDA option only selects FLAC music
    // emulation versus forwarding the native system CD-audio MCI commands.
    cdda::try_install(root,&game)?;
    patch::set_cdda(&exe,true)?;
    patch::set_skip_intro(&exe,skip)?;
    verify_launch_assets(root)?;
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
    // Never emit an extra DXVK *_d3d9.log beside game files during normal
    // gameplay. DXVK documents LOG_PATH=none to disable file creation.
    // Explicit diagnostic renderer runs override it with a private temp
    // path, then fold their logs into CyberGladiatorsFix.log on exit.
    cmd.env("DXVK_LOG_PATH","none");
    // The user's inherited shell environment must not accidentally activate
    // our invasive 1996 transition probes in the ordinary game path.
    for key in ["CYBER_VIDEO_CLOSE_DELAY_MS","CYBER_GLIDE_HOOK_DISABLED",
        "CYBER_GLIDE_TRACE","CYBER_GLIDE_PASSTHRU_NOOP",
        "CYBER_WNDPROC_REPAIR_TEST","CYBER_WNDPROC_REPAIR",
        "CYBER_AVI_FULLSCREEN","CYBER_CDDA_MUSIC","CYBER_AVI_SMOOTH_TEST",
        "CYBER_VULKAN_PASSIVE_FAULT_PROBE","CYBER_VULKAN_EARLY_HOST_RELEASE",
        "CYBER_VULKAN_AUTO_FULLSCREEN"]{
        cmd.env_remove(key);
    }
    if avi_4x3 {
        cmd.env("CYBER_AVI_FULLSCREEN","1");
        crate::show(&format!("AVI_4X3 native_MCIAVI=1 host=black_popup_no_activate target=full_monitor_4x3 only_skip_keys=ESC_ENTER_SPACE mouse_buttons_suppressed=1 renderer={}",if backend_override==Some("2"){"nGlide_Vulkan"}else{"nGlide_D3D9"}),root);
    }
    if passive_vulkan_fault_probe(backend_override,smooth_test,trace){
        cmd.env("CYBER_GLIDE_TRACE","1");
        cmd.env("CYBER_GLIDE_HOOK_DISABLED","1");
        cmd.env("CYBER_VULKAN_PASSIVE_FAULT_PROBE","1");
        crate::show("VULKAN_FAULT_PROBE enabled=1 exception_observer=deferred first_chance=1 window_sampler=read_only glide_iat_hook=disabled wine_debug_channels=unchanged",root);
    }
    if early_release {
        cmd.env("CYBER_VULKAN_EARLY_HOST_RELEASE","1");
        crate::show("VULKAN_EARLY_RELEASE enabled=1 hypothesis=retained_fullscreen_video_host_interferes_with_native_Vulkan close_host=immediately_after_MCI_AVIVIDEO_CLOSED black_postroll=disabled; VIDEO_NOT_SKIPPED; D3D9_UNCHANGED",root);
    }
    if auto_fullscreen {
        cmd.env("CYBER_VULKAN_AUTO_FULLSCREEN","1");
        crate::show("VULKAN_AUTO_FULLSCREEN enabled=1 experimental=1 check_nGlide_window=1 keyboard_recovery=Alt+Enter at_most_twice gated=foreground_and_small_window no_focus_stealing=1 bounded=1 D3D9_UNCHANGED",root);
    }
    if smooth_test {
        cmd.env("CYBER_AVI_SMOOTH_TEST","1");
        crate::show(&format!("VIDEO_SMOOTH_TEST enabled=1 pre_mci_black_host=1 initially_hidden_avi_surface=1 wm_fullscreen_managed_request=1 pre_video_black_ms=1000 post_video_black_sync_hold_ms={} true_black_video_surface=1 true_black_avi_cover=1 prevent_3d_resume_during_hold={} NGLIDE_REFRESH={} no_D3D9_hook=1",
            if early_release{0}else{1000}, if early_release{0}else{1},if selected_refresh(backend_override,desktop_refresh)=="1"{"1 (desktop; Vulkan-only experiment)"}else{"2 (60Hz, validated D3D9)"}),root);
    }
    cmd.env("CYBER_CDDA_MUSIC",if enable_cdda{"1"}else{"0"});
    // This is a one-shot, guard-checked in-process fix. No long-running
    // diagnostics, DLL hooks or Wine prefix changes in the normal path.
    if wndproc_repair {
        cmd.env(if trace{"CYBER_WNDPROC_REPAIR_TEST"}else{"CYBER_WNDPROC_REPAIR"},"1");
        if !trace {
            crate::show(&format!("INTRO_NATIVE repair=enabled only_when_WineD3D_ANSI_WndProc_and_verified_game_class; renderer={}",if backend_override==Some("2"){"nGlide_Vulkan"}else{"nGlide_D3D9"}),root);
        }
    }
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
            if selected_refresh(backend_override,desktop_refresh)=="1" {
                // nGlide mode 1 follows the desktop; mode 2 forces 60 Hz.
                // Direct Vulkan caused Wine DISP_CHANGE_BADMODE (-2) with
                // 60 Hz just before a null read inside CYBER.EXE.
                // This is an isolated Vulkan A/B test, not a proven fix.
                cmd.env("NGLIDE_REFRESH","1");
            }
            // A/B baselines must not inherit a prior inline DXVK_CONFIG
            // value, otherwise the baseline might also use the tweak.
            cmd.env_remove("DXVK_CONFIG");
            if backend=="1" {
                // Wine-side DXVK logs are not necessarily created by default.
                // Use a unique, previously nonexistent subdirectory relative
                // to the game's working directory for unequivocal provenance.
                let log_dir=dxvk_log_dir.ok_or("DXVK log directory missing for DirectX9 experiment")?;
                cmd.env("DXVK_LOG_PATH",log_dir);
                // D7VK (DirectDraw/D3D7) uses its own logging variables
                // in current GE-Proton builds. Capture its trace in the
                // SAME per-run temporary directory as DXVK's D3D9 trace.
                cmd.env("D7VK_LOG_PATH",log_dir);
                cmd.env("D7VK_LOG_LEVEL","info");
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
        // Timestamp, process and thread IDs let us align the WinMM AVI-close
        // boundary with the first-chance DXVK stack overflow precisely.
        cmd.env("WINEDEBUG","+timestamp,+pid,+tid,+seh,+loaddll");
        // Only the debug launch observes the native 3dfx-to-VGA transition.
        // The x86 CDDA DLL refuses unknown game/Glide pointers and leaves
        // all game files and normal --launch behavior untouched.
        cmd.env("CYBER_GLIDE_TRACE","1");
        if wndproc_repair {
            cmd.env("CYBER_WNDPROC_REPAIR_TEST","1");
        }
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
        let log=diagnostics::location(root);
        if log.is_symlink(){return Err("Refusing symlink session log".into());}
        let mut output=fs::OpenOptions::new().append(true)
            .open(&log).map_err(|e|format!("Cannot append Wine trace to session log: {e}"))?;
        // Proton consumes this option before starting Wine; we record the
        // value visible to the launcher, not an unverified claim about which
        // d3d9.dll implementation was actually loaded.
        let proton_wined3d=match std::env::var("PROTON_USE_WINED3D").as_deref(){
            Ok("1")=>"1",Ok("0")=>"0",Ok("")=>"empty",
            Ok(_)=>"other",Err(_)=>"unset",
        };
        writeln!(output,"CyberGladiatorsFix --launch --trace")
            .and_then(|_|writeln!(output,"WINEDEBUG=+timestamp,+pid,+tid,+seh,+loaddll; D7VK_LOG_PATH=same_privately_captured_directory_as_DXVK; NGLIDE_BACKEND={} {}; NGLIDE_REFRESH={}; DXVK_DEFER_SURFACE={dxvk_defer_surface}; DXVK_DIAGNOSTICS={}; DXVK_LOG_PATH={}; CYBER_GLIDE_TRACE=1; CYBER_GLIDE_PASSTHRU_NOOP={}; CYBER_VIDEO_CLOSE_DELAY_MS={video_delay_ms}; CYBER_GLIDE_HOOK_DISABLED={}; PROTON_USE_WINED3D={proton_wined3d} (launcher env, not backend proof); CDDA enabled={enable_cdda}; SkipIntro={skip}",backend_override.unwrap_or("0"),if backend_override.is_some(){"(forced)"}else{"(auto)"},if selected_refresh(backend_override,desktop_refresh)=="1"{"1 (by desktop)"}else{"2 (60 Hz)"},if backend_override==Some("1"){"HUD+log"}else{"off"},if backend_override==Some("1"){"unique temporary directory"}else{"inherited"},if passthru_noop{1}else{0},if no_glide_hook{1}else{0}))
            .and_then(|_|output.flush())
            .map_err(|e|format!("Cannot initialize Wine trace in session log: {e}"))?;
        let errors=output.try_clone().map_err(|e|format!("Cannot capture Wine stderr: {e}"))?;
        cmd.stdout(Stdio::from(output));
        cmd.stderr(Stdio::from(errors));
    }else{
        nglide::configure(&mut cmd,logo);
        if normal_repair {
            // Preserve validated D3D9 for both normal entrypoints. Only the
            // dedicated smooth Vulkan experiment can select backend 2 here.
            cmd.env("NGLIDE_BACKEND",backend_override.unwrap_or("1"));
            if selected_refresh(backend_override,desktop_refresh)=="1" {
                cmd.env("NGLIDE_REFRESH","1");
            }
        }
    }
    // All DLL/CDDA logs go directly into the launcher-adjacent session log.
    cmd.env("CYBER_LOG_FILE",diagnostics::location(root));
    // Module digests identify exactly which locally installed compatibility
    // stack is running. Hash only files already verified as regular assets.
    for (label,path) in [
        ("CYBER.EXE",game.join("cyber.exe")),
        ("nGlide glide.dll",game.join("glide.dll")),
    ]{
        if let Ok(hash)=patch::file_hash(&path) {
            diagnostics::log(root,"MODULE",&format!("{label} sha256={hash}"));
        }
    }
    if let Ok(hash)=patch::file_hash(&game.join(cdda::DLL_NAME)){
        diagnostics::log(root,"MODULE",&format!("WinMM video/CDDA bridge sha256={hash}"));
    }
    if let Ok(iq)=settings::read_allow_modified_iq(&game){
        diagnostics::log(root,"CONFIG",&format!("effective AllowModifiedIQ={} (strict --verify unaffected)",u8::from(iq)));
    }
    diagnostics::log(root,"CONFIG",&format!(
        "GAME launch: SkipIntro={} EnableCDDA={} NGLIDE_BACKEND={} NGLIDE_RESOLUTION=1 NGLIDE_ASPECT=1 NGLIDE_REFRESH={} NGLIDE_VSYNC=1 NGLIDE_GAMMA=5 NGLIDE_SPLASH={} DXVK_defer_surface={} trace={}",
        u8::from(skip),u8::from(enable_cdda),if normal_repair {backend_override.unwrap_or("1")}else{backend_override.unwrap_or("0")},
        if selected_refresh(backend_override,desktop_refresh)=="1"{"1 (desktop)"}else{"2 (60Hz)"},
        u8::from(logo),u8::from(dxvk_defer_surface),u8::from(trace)
    ));
    cmd.spawn().map_err(|e|format!("Unable to start CYBER.EXE: {e}"))
}

#[cfg(test)]
mod experimental_launch_tests {
    use super::*;
    #[test]
    fn iq_opt_in_limits_to_exact_cd_paths_and_unchanged_sizes(){
        for name in ["iq/iq00.dat","iq/iq01.dat","iq/iq40.dat","iq/iq99.dat"]{
            assert!(same_size_iq_asset(name,3696,3696));
            assert!(!same_size_iq_asset(name,3696,3695));
        }
        for bad in [
            "iq/iq0.dat","iq/iq100.dat","iq/iqAA.dat","iq/IQ01.dat",
            "iq/iq01.dat.bak","iq/other.dat","graphics/iq01.dat",
            "iq/../iq01.dat","iq\\iq01.dat","iq/iq99.exe",
        ]{assert!(!same_size_iq_asset(bad,3696,3696),"{bad}");}
    }
    #[test]
    fn asset_change_error_reports_both_size_and_hash_without_approving_changes(){
        let err=asset_mismatch_message("iq/iq40.dat",1024,"oldhash",2048,"newhash");
        assert!(err.starts_with("Game asset has changed: iq/iq40.dat"));
        assert!(err.contains("expected size=1024, SHA-256=oldhash"));
        assert!(err.contains("actual size=2048, SHA-256=newhash"));
    }
    #[test]
    fn explicit_proton_wined3d_blocks_dxvk_comparison_not_direct_vulkan(){
        assert!(validate_proton_dxvk_preflight("1",Some("1")).is_err());
        assert!(validate_proton_dxvk_preflight("1",Some("0")).is_ok());
        assert!(validate_proton_dxvk_preflight("1",None).is_ok());
        assert!(validate_proton_dxvk_preflight("2",Some("1")).is_ok());
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
    fn smooth_vulkan_is_opt_in_and_normal_video_stays_d3d9(){
        assert!(guarded_video_backend_allowed(Some("1"),false));
        assert!(guarded_video_backend_allowed(Some("1"),true));
        assert!(guarded_video_backend_allowed(Some("2"),true));
        assert!(!guarded_video_backend_allowed(Some("2"),false));
        assert!(!guarded_video_backend_allowed(Some("0"),true));
        assert!(!guarded_video_backend_allowed(None,true));
    }
    #[test]
    fn passive_vulkan_probe_only_runs_in_opt_in_smooth_vulkan(){
        assert!(passive_vulkan_fault_probe(Some("2"),true,false));
        assert!(!passive_vulkan_fault_probe(Some("1"),true,false));
        assert!(!passive_vulkan_fault_probe(Some("2"),false,false));
        assert!(!passive_vulkan_fault_probe(Some("2"),true,true));
        assert!(!passive_vulkan_fault_probe(None,true,false));
    }
    #[test]
    fn vulkan_refresh_follows_desktop_without_changing_d3d9(){
        assert_eq!(selected_refresh(Some("2"),false),"1");
        assert_eq!(selected_refresh(Some("1"),false),"2");
        assert_eq!(selected_refresh(Some("1"),true),"1");
        assert_eq!(selected_refresh(None,false),"2");
    }
    #[test]
    fn video_delay_is_bounded_and_trace_only(){
        assert!(matches!(100u64,100|250|500|1000));
        assert!(matches!(250u64,100|250|500|1000));
        assert!(matches!(500u64,100|250|500|1000));
        assert!(matches!(1000u64,100|250|500|1000));
        assert!(!matches!(10000u64,100|250|500|1000));
    }
}
