// SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only checks. Asset hashes are recorded during extraction and are
//! independently recomputed before launch, including the original backup.
use crate::{diagnostics,install::{DIR,MANIFEST},cdda,patch,settings};
use sha2::{Digest,Sha256};
use std::fs::{self,File};
use std::io::Read;
use std::path::{Path,Component};
use std::process::{Child,Command};
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
// Only files actually below the game's top-level iq/ directory may change
// after installation. Similar prefixes (e.g. iq_backup/) are immutable.
fn is_game_writable_iq(name:&str)->bool{
    name.split_once('/').is_some_and(|(dir,rest)|
        dir.eq_ignore_ascii_case("iq") && !rest.is_empty()
    )
}
/// An installed IQ file must be present and a normal file. Its current size
/// and hash belong to the running game, not to the original CD manifest.
/// All other resources remain byte-for-byte verified against that manifest.
fn verify_installed_resource(name:&str,expected_size:u64,expected_sha256:&str,path:&Path)
    ->Result<bool,String>{
    if is_game_writable_iq(name){
        let m=fs::symlink_metadata(path).map_err(|e|format!("Missing IQ resource {name}: {e}"))?;
        if m.file_type().is_symlink()||!m.is_file(){
            return Err(format!("IQ resource is not a regular file: {name}"));
        }
        return Ok(true);
    }
    let (actual_size,actual_hash)=file_hash(path)?;
    if actual_size!=expected_size||actual_hash!=expected_sha256 {
        return Err(asset_mismatch_message(name,expected_size,expected_sha256,actual_size,&actual_hash));
    }
    Ok(false)
}
/// Original CD assets are SHA-256-verified during fresh staged installation.
/// For existing installations and --verify, check game-writable iq/ files
/// for presence only, and keep strict hash checks on every other resource.
/// CDDA and integrity protections are internal and cannot be disabled in INI.
pub fn verify(root:&Path)->Result<String,String>{
    verify_assets(root,true)
}
pub fn verify_launch_assets(root:&Path)->Result<String,String>{
    verify_assets(root,true)
}
pub fn verify_install_content(root:&Path)->Result<String,String>{
    verify_assets(root,false)
}
fn verify_assets(root:&Path,check_modules:bool)->Result<String,String>{
    let game=root.join(DIR);
    let meta=fs::symlink_metadata(&game).map_err(|e|format!("Missing game directory: {e}"))?;
    if !meta.is_dir()||meta.file_type().is_symlink(){return Err("Game folder is not a regular directory".into());}
    patch::verify_exe(&game.join("cyber.exe"))?;
    let manifest=fs::read_to_string(game.join(MANIFEST)).map_err(|e|format!("Manifest missing: {e}"))?;
    if manifest.len()>4*1024*1024||!manifest.starts_with("# CyberGladiatorsFix manifest v1\n"){
        return Err("Missing or invalid extraction manifest".into());
    }
    let mut count=0usize;let mut bytes=0u64;let mut iq_files=0usize;
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
        if verify_installed_resource(name,size,hash,&path)?{iq_files+=1;}
        count+=1;bytes+=size;
    }
    let audio=if check_modules{
        if crate::bundle::GAMEFRAME_GLIDE_DLL.is_empty(){
            return Err("This release requires the embedded GameFrame Glide wrapper".into());
        }
        crate::bundle::verify()?;
        if patch::cdda_active(&game.join("cyber.exe"))?{
            cdda::verify(&game)?;
            "embedded WinMM bridge verified; FLAC CD audio selected"
        }else{"legacy system WinMM import"}
    }else{"retail assets checked before module installation"};
    Ok(format!("Verified CYBER.EXE patch + backup, {count} CD resources ({bytes} original manifest bytes), {audio}; immutable_resources_sha256_verified={} IQ_files_presence_only={iq_files} IQ_hash_and_size_not_checked_after_install=1; original CD IQ hashes remain recorded in manifest",count-iq_files))
}
fn default_owned_virtual_for_edition(edition:&str)->bool{
    matches!(edition,"US"|"FR")
}
/// Explicit, opt-in game launch without preflight. No installer, manifest,
/// file-presence sweep, SHA-256 hashing, EXE patching or DLL replacement.
/// Executable length is used *only* to select the correct US/FR runtime
/// address profile; a missing/unrecognized EXE is left to the OS loader.
/// Never accept a caller-provided CYBER_GAME_EDITION as a substitute.
fn unchecked_gameframe_command(root:&Path)->Command{
    let game=root.join(DIR);
    let exe=game.join("cyber.exe");
    let mut cmd=Command::new(&exe);
    cmd.current_dir(&game);

    let runtime_log=diagnostics::runtime_location(root);
    cmd.env("CYBER_LOG_FILE",&runtime_log);
    cmd.env("GLIDE_LOG",&runtime_log);
    cmd.env("CYBER_SESSION_ID",diagnostics::session_id());
    cmd.env("GLIDE_FPS_LIMIT","60");
    cmd.env("CYBER_CDDA_MUSIC","1");
    cmd.env("DXVK_LOG_PATH","none");

    // Normal GameFrame/DirectDraw behavior is available to either original
    // edition even if one of its resources changed. Do not hash any file.
    // Unknown EXE sizes are NOT passed through edition-specific address hooks.
    let edition=match fs::metadata(&exe).ok().map(|metadata|metadata.len()){
        Some(size) if size==crate::patch::LEN as u64=>Some("US"),
        Some(size) if size==crate::fr_patch::LEN as u64=>Some("FR"),
        _=>None,
    };
    for key in [
        "CYBER_GLIDE_TRACE","CYBER_GLIDE_HOOK_DISABLED","CYBER_GLIDE_PASSTHRU_NOOP",
        "CYBER_WNDPROC_REPAIR_TEST","CYBER_WNDPROC_REPAIR",
        "CYBER_VULKAN_PASSIVE_FAULT_PROBE","CYBER_VULKAN_EARLY_HOST_RELEASE",
        "CYBER_VULKAN_AUTO_FULLSCREEN","CYBER_VIDEO_CLOSE_DELAY_MS",
        "NGLIDE_BACKEND","NGLIDE_REFRESH","NGLIDE_RESOLUTION","NGLIDE_ASPECT",
        "NGLIDE_VSYNC","NGLIDE_GAMMA","NGLIDE_SPLASH",
        "CYBER_FR_VIRTUAL_DDRAW","CYBER_US_WINE_VIRTUAL_DDRAW",
        "CYBER_US_NATIVE_VIRTUAL_DDRAW","CYBER_WINE_DDRAW_NATIVE_FORWARD",
        "CYBER_GAME_EDITION","CYBER_VIRTUAL_DDRAW","CYBER_GAMEFRAME_TEST",
        "CYBER_GAMEFRAME_AVI_FIRST_PRESENT","CYBER_AVI_FULLSCREEN",
        "CYBER_AVI_SMOOTH_TEST","CYBER_GAMEFRAME_ALT_TAB_NATIVE_FIX",
    ]{cmd.env_remove(key);}
    if let Some(edition)=edition{
        cmd.env("CYBER_GAME_EDITION",edition);
        cmd.env("CYBER_VIRTUAL_DDRAW","1");
        cmd.env("CYBER_GAMEFRAME_TEST","1");
        cmd.env("CYBER_GAMEFRAME_AVI_FIRST_PRESENT","1");
        cmd.env("CYBER_AVI_FULLSCREEN","1");
        cmd.env("CYBER_AVI_SMOOTH_TEST","1");
        cmd.env("CYBER_GAMEFRAME_ALT_TAB_NATIVE_FIX",
            if diagnostics::is_native_windows(){"1"}else{"0"});
    }
    if let Some((w,h))=diagnostics::primary_desktop_size(){
        cmd.env("CYBER_GAMEFRAME_DESKTOP_SIZE",format!("{w}x{h}"));
    }else{
        cmd.env_remove("CYBER_GAMEFRAME_DESKTOP_SIZE");
    }
    cmd.env("CYBER_GAMEFRAME_MENU_SIGNATURE",format!(
        "CyberGladiatorsFix v{}-{}",env!("CARGO_PKG_VERSION"),env!("CYBER_COMMIT")
    ));
    cmd
}

/// Spawn the existing game directly. The OS reports a missing/invalid EXE
/// through spawn(); other missing game assets are for the game to handle.
/// Does not even read the user INI or the installed manifest.
pub fn launch_gameframe_unchecked(root:&Path)->Result<Child,String>{
    let mut cmd=unchecked_gameframe_command(root);
    // The game log is a separate session; a failed spawn never clears the
    // installer's log, and --verify never touches the previous game log.
    diagnostics::start_runtime_session(root)?;
    cmd.spawn().map_err(|e|format!(
        "Unable to start existing CYBER.EXE without verification: {e}"
    ))
}

pub fn launch_gameframe(root:&Path)->Result<Child,String>{
    let game=root.join(DIR);
    settings::ensure(&game)?;
    let skip_intro=settings::read(&game)?;
    let enable_cdda=settings::CDDA_ENABLED;
    let exe=game.join("cyber.exe");
    patch::verify_exe(&exe)?;
    let edition=if crate::fr_patch::owns(&exe)?{"FR"}else{"US"};
    crate::show(&format!("GAME_EDITION selected={edition} original_backup_sha256_verified=1 runtime_address_profile={edition}"),root);
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
    // Child profile is derived from a verified original EXE backup, never
    // from an inherited caller variable or a filename alone.
    cmd.env("CYBER_GAME_EDITION",edition);
    // The one-click launcher selects the SAME project-owned DirectDraw COM
    // adapter and MCI video surface for both verified US and FR retail
    // editions, on native Windows and Wine/Proton. Inherited variables
    // cannot select an unverified system-wide DirectDraw implementation.
    // Windows and Wine adapt the *window manager* focus policy only.
    let native_windows=diagnostics::is_native_windows();
    let native_fr=edition=="FR" && native_windows;
    cmd.env("CYBER_GAMEFRAME_ALT_TAB_NATIVE_FIX",if native_windows{"1"}else{"0"});
    let virtual_ddraw=default_owned_virtual_for_edition(edition);
    if !virtual_ddraw{return Err("Unrecognized retail edition for shared DirectDraw".into());}
    cmd.env("CYBER_VIRTUAL_DDRAW","1");
    for key in ["CYBER_FR_VIRTUAL_DDRAW","CYBER_US_WINE_VIRTUAL_DDRAW",
        "CYBER_US_NATIVE_VIRTUAL_DDRAW"] {cmd.env_remove(key);}
    crate::show(&format!(
        "VIRTUAL_DDRAW_RUNTIME enabled=1 default=1 edition={edition} wine={} shared_owned_COM=1 shared_MCI=1 single_codepath=1",
        u8::from(!native_windows)
    ),root);
    crate::show(&format!(
        "VIDEO_FOCUS_RUNTIME selected={} edition={edition} automatic=1 common_video_core=1",
        if native_windows{"native_windows"}else{"wine"}
    ),root);
    if native_windows {
        crate::show(&format!(
            "DISPLAY_MODE_SNAPSHOT stage=before_game_spawn edition={edition} {}",
            diagnostics::prelaunch_display_modes()
        ),root);
    }
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
    let runtime_log=diagnostics::runtime_location(root);
    cmd.env("CYBER_LOG_FILE",&runtime_log);
    cmd.env("GLIDE_LOG",&runtime_log);
    cmd.env("CYBER_SESSION_ID",diagnostics::session_id());
    cmd.env("GLIDE_FPS_LIMIT","60");
    cmd.env("CYBER_GAMEFRAME_TEST","1");
    cmd.env("CYBER_GAMEFRAME_AVI_FIRST_PRESENT","1");
    // Use the same checked-out source revision as --version and the log.
    // Set a fresh child-scoped value so inherited environment variables
    // cannot replace our signature with a stale or unrelated version.
    let menu_signature=format!(
        "CyberGladiatorsFix v{}-{}",env!("CARGO_PKG_VERSION"),env!("CYBER_COMMIT")
    );
    cmd.env("CYBER_GAMEFRAME_MENU_SIGNATURE",&menu_signature);
    crate::show(&format!(
        "GAMEFRAME_MENU_SIGNATURE enabled=1 source=launcher_version_commit placement=upper_right_edge_4px_transparent style=purple_menu_footer font=angular_mixed_case_7x9 detection=two_green_player_captions runtime=OpenGL text={menu_signature}"
    ),root);
    // The former Wine mode-forwarding fallback remains in diagnostic code,
    // but is off for ALL normal US/FR Windows/Wine launches.
    cmd.env("CYBER_WINE_DDRAW_NATIVE_FORWARD","0");
    crate::show("WINE_DDRAW_NATIVE_FORWARD requested=0 shared_owned_com_default=1 physical_mode_request=0",root);
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
    crate::show("GAMEFRAME renderer=GameFrame-OpenGL original_input_hwnd=1 fullscreen_desktop_borderless=1 4x3_letterbox=1 frame_limit=60fps video=native_MCI first_present_handoff=1 log=separate_runtime",root);
    diagnostics::log(root,"MODULE",&format!("GameFrame glide.dll sha256={glide_hash} (embedded_verified_build)"));
    diagnostics::log(root,"CONFIG",&format!("GAMEFRAME launch: SkipIntro={} EnableCDDA={} GLIDE_FPS_LIMIT=60 NGLIDE_BACKEND=not_used 4x3_aspect=preserved AVI_FIRST_PRESENT_HANDOFF=1",u8::from(skip_intro),u8::from(enable_cdda)));
    // The original FR backup's SHA-256 and PE32 structure have been pinned.
    // Inspect plausible x86 COM vtable callsites before spawning the game,
    // without modifying any file, display mode or COM object.
    if native_fr {crate::fr_ddraw_static::inspect(root,&exe);}
    // Prepare the independent game/DLL log only after launch prerequisites pass.
    diagnostics::start_runtime_session(root)?;
    cmd.spawn().map_err(|e|format!("Unable to start Cyber Gladiators through GameFrame: {e}"))
}

#[cfg(test)]
mod unchecked_launch_tests {
    use super::*;

    #[test]
    fn unchecked_launch_builds_command_without_existing_game_directory(){
        let root=std::env::temp_dir().join(format!(
            "CyberGladiators-unchecked-nonexistent-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .unwrap().as_nanos()
        ));
        assert!(!root.exists());
        let cmd=unchecked_gameframe_command(&root);
        assert_eq!(cmd.get_program(),root.join(DIR).join("cyber.exe").as_os_str());
        assert!(!root.exists());
        let child_env=cmd.get_envs().collect::<Vec<_>>();
        assert!(child_env.iter().any(|(key,value)|
            *key=="CYBER_LOG_FILE" &&
            value.is_some_and(|v|v.to_string_lossy().ends_with("CyberGladiatorsFix.log"))
        ));
    }
}

#[cfg(test)]
mod shared_default_ddraw_tests {
    use super::default_owned_virtual_for_edition;
    #[test]
    fn one_owned_com_profile_is_default_for_both_retail_editions(){
        for edition in ["US","FR"]{
            assert!(default_owned_virtual_for_edition(edition));
        }
        for unverified in ["","unknown","us","FR-DEMO","DE"]{
            assert!(!default_owned_virtual_for_edition(unverified));
        }
        // Edition, not an OS-specific command, controls the common engine.
    }
}
#[cfg(test)]
mod integrity_tests {
    use super::*;
    #[test]
    fn only_real_iq_directory_is_writable_after_install(){
        for name in ["iq/iq40.dat","IQ/iq40.dat","iq/subdir/state.dat"]{
            assert!(is_game_writable_iq(name),"{name}");
        }
        for name in ["iq","iq40.dat","iq_extra/a.dat","graphics/iq/iq40.dat",
                     "music/Track02.flac","iq\\iq40.dat","iq/"]{
            assert!(!is_game_writable_iq(name),"{name}");
        }
    }
    #[test]
    fn installed_iq_accepts_game_changes_but_not_missing_or_directory_entries(){
        let dir=std::env::temp_dir().join(format!("CyberIQPresence-{}",
            std::process::id()));
        let _=fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let iq=dir.join("iq40.dat");
        let clean=b"original";
        fs::write(&iq,clean).unwrap();
        let clean_hash=crate::patch::digest(clean);
        assert_eq!(verify_installed_resource("iq/iq40.dat",clean.len() as u64,&clean_hash,&iq),Ok(true));
        fs::write(&iq,b"modified").unwrap(); // same size, different hash
        assert_eq!(verify_installed_resource("iq/iq40.dat",clean.len() as u64,&clean_hash,&iq),Ok(true));
        fs::write(&iq,b"modified and longer").unwrap(); // different size, still the game's file
        assert_eq!(verify_installed_resource("iq/iq40.dat",clean.len() as u64,&clean_hash,&iq),Ok(true));
        assert!(verify_installed_resource("graphics/shell.fnt",clean.len() as u64,&clean_hash,&iq)
            .unwrap_err().contains("Game asset has changed"));
        fs::remove_file(&iq).unwrap();
        assert!(verify_installed_resource("iq/iq40.dat",clean.len() as u64,&clean_hash,&iq)
            .unwrap_err().contains("Missing IQ resource"));
        fs::create_dir(&iq).unwrap();
        assert!(verify_installed_resource("iq/iq40.dat",clean.len() as u64,&clean_hash,&iq)
            .unwrap_err().contains("not a regular file"));
        fs::remove_dir_all(&dir).unwrap();
    }
    #[test]
    fn asset_change_error_reports_both_size_and_hash_without_approving_changes(){
        let err=asset_mismatch_message("iq/iq40.dat",1024,"oldhash",2048,"newhash");
        assert!(err.starts_with("Game asset has changed: iq/iq40.dat"));
        assert!(err.contains("expected size=1024, SHA-256=oldhash"));
        assert!(err.contains("actual size=2048, SHA-256=newhash"));
    }

}
