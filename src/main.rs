// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows console patcher/installer; supports Wine on Linux.
#[cfg(not(windows))]
compile_error!("CyberGladiatorsFix targets Windows x64 (and Wine on Linux). Build for a Windows target.");
mod cue;
mod cdda;
mod audio;
mod install;
mod nglide;
mod graphics;
mod settings;
mod patch;
mod verify;
mod intro;
mod diagnostics;
mod bundle;

use std::ffi::OsString;
use std::fs;
use std::path::{Path,PathBuf};

const NAME:&str="CyberGladiatorsFix";
const HELP:&str=r"CyberGladiatorsFix — self-contained x64 installer, compatibility patch and game launcher

Usage:
  CyberGladiatorsFix.exe [<disc.cue>]    Install/update verified US retail CD
  CyberGladiatorsFix.exe --launch        Play game (isolated fullscreen intro)
  CyberGladiatorsFix.exe --trace         Play and collect detailed diagnostics
  CyberGladiatorsFix.exe --launch --3dfx-logo
  CyberGladiatorsFix.exe --verify        Strict original CD integrity check
  CyberGladiatorsFix.exe --max-graphics  Maximize verified game settings
  CyberGladiatorsFix.exe --upgrade-cdda  Approve backed-up unknown DLL upgrade
  CyberGladiatorsFix.exe --check-bundle  Validate embedded 32-bit components
  CyberGladiatorsFix.exe --dxvk-rootcause  Reproduce native AVI/DXVK crash (diagnostic)
  CyberGladiatorsFix.exe --dxvk-wndproc-repair-test  Test restoring original game WndProc after AVI (experimental)
  CyberGladiatorsFix.exe --version
  CyberGladiatorsFix.exe --help

One-file Windows distribution: the x86 CDDA DLL and isolated AVI player are
included inside this EXE. A matching native Wine/Proton configuration and
third-party nGlide are required to play. The game and nGlide are NOT bundled.

A fresh CyberGladiatorsFix.log is written beside this EXE per invocation.
When [Video] SkipIntro=0 (default), the movie plays in a borderless full-screen
window; Esc, Enter, Space or any ordinary key skips it. The MCI player exits
before the D3D9 game process starts. If SkipIntro=1, no video is played.
Music is optional: [Audio] EnableCDDA=1 enables the embedded FLAC bridge.
Original CD assets and CYBER.EXE.bak are never silently replaced.
[Integrity] AllowModifiedIQ=1 permits same-size IQ differences on launch only;
--verify always uses strict original hashes.
The game's binary CYBER.INI is separate from text CyberGladiatorsFix.ini.
For additional Wine diagnostics use --trace (also --launch --trace).
For developer-only experiments, see docs/DEVELOPMENT_HISTORY.md.
";
fn show(s:&str,root:&Path){
    println!("{s}");
    diagnostics::log(root,"Launcher",s);
}
fn require_cue(path:&Path)->Result<(),String>{
    if !path.extension().is_some_and(|s|s.eq_ignore_ascii_case("cue")){
        return Err("Expected a .cue sheet referencing one or more .bin files; ISO/BIN files cannot be selected directly".into());
    }
    Ok(())
}
fn parse_video_delay(arg:&str)->Option<u64>{
    let number=arg.strip_prefix("--video-delay=")?.parse::<u64>().ok()?;
    matches!(number,100|250|500|1000).then_some(number)
}
fn wait_game(mut child:std::process::Child,root:&Path)->Result<(),String>{
    let pid=child.id();
    let start=std::time::Instant::now();
    let status=child.wait().map_err(|e|e.to_string())?;
    show(&format!(
        "GAME_EXIT pid={pid} exit_status={status} elapsed_ms={}",
        start.elapsed().as_millis()
    ),root);
    if !status.success(){return Err(format!("Game exited with status {status}"));}
    Ok(())
}
fn wait_diagnostic(mut child:std::process::Child,root:&Path)->Result<(),String>{
    let pid=child.id();
    let start=std::time::Instant::now();
    let status=child.wait().map_err(|e|e.to_string())?;
    show(&format!(
        "DIAGNOSTIC_EXIT pid={pid} exit_status={status} elapsed_ms={} session_log={}",
        start.elapsed().as_millis(),
        diagnostics::location(root).display()
    ),root);
    if !status.success(){
        return Err(format!("Game exited with status {status}; see CyberGladiatorsFix.log"));
    }
    Ok(())
}
fn report_dxvk_evidence(root:&Path,dxvk_log_dir:Option<PathBuf>){
    let Some(dir)=dxvk_log_dir else{return;};
    // DXVK produces its own diagnostic files. They live outside the game
    // tree for just this run, and are folded into the ONE session log.
    if dir.is_symlink() {
        show("DXVK_EVIDENCE WARNING: temporary trace folder is a symlink; not reading or removing",root);
        return;
    }
    let mut logs=Vec::new();
    if let Ok(entries)=fs::read_dir(&dir) {
        for entry in entries.flatten(){
            let name=entry.file_name().to_string_lossy().to_string();
            // GE-Proton can load D7VK for the AVI's DirectDraw path and
            // DXVK for nGlide D3D9 within the same old Win32 process.
            // Preserve independently produced logs for BOTH providers.
            let lower=name.to_ascii_lowercase();
            if (lower.ends_with("_d3d9.log")||lower.ends_with("_ddraw.log")
                ||lower.ends_with("_d3d7.log"))
                && entry.file_type().is_ok_and(|kind|kind.is_file()){
                logs.push(entry.path());
            }
        }
    }
    let has_d3d9=logs.iter().any(|p|p.file_name().is_some_and(|n|
        n.to_string_lossy().to_ascii_lowercase().ends_with("_d3d9.log")));
    if !has_d3d9 {
        show("DXVK_EVIDENCE UNCONFIRMED: no fresh *_d3d9.log in the temporary trace folder; renderer selection alone does not prove DXVK loaded",root);
    }
    if !logs.is_empty() {
        logs.sort();
        let mut all_ok=true;
        for path in logs {
            let label=path.file_name().unwrap_or_default().to_string_lossy();
            match fs::metadata(&path){
                Ok(meta) if meta.len()<=2*1024*1024=>{
                    match fs::read(&path) {
                        Ok(bytes)=>{
                            let is_d3d9=label.to_ascii_lowercase().ends_with("_d3d9.log");
                            if is_d3d9 {
                                show(&format!("DXVK_EVIDENCE D3D9_LOG_FOUND: {} ({} bytes); direct renderer version evidence follows",label,bytes.len()),root);
                                diagnostics::log(root,"DXVK_D3D9",&String::from_utf8_lossy(&bytes));
                            }else{
                                // The presence of a ddraw.log is not, alone,
                                // positive identification of D7VK.
                                show(&format!("VIDEO_PROVIDER_EVIDENCE DDRAW_OR_D3D7_LOG_FOUND: {} ({} bytes); verify provider/version in contents",label,bytes.len()),root);
                                diagnostics::log(root,"LEGACY_DDRAW",&String::from_utf8_lossy(&bytes));
                            }
                        },
                        Err(e)=>{
                            all_ok=false;
                            show(&format!("DXVK_EVIDENCE WARNING: cannot read {}: {e}; preserving temporary files",label),root);
                        }
                    }
                },
                Ok(meta)=>{
                    all_ok=false;
                    show(&format!("DXVK_EVIDENCE WARNING: {} is too large ({} bytes) to import automatically; preserving temporary files",label,meta.len()),root);
                },
                Err(e)=>{
                    all_ok=false;
                    show(&format!("DXVK_EVIDENCE WARNING: cannot inspect {}: {e}",label),root);
                }
            }
        }
        if !all_ok{return;}
    }
    // Avoid unbounded accumulation of diagnostics inside the user's game.
    // Only remove the fresh per-launch temporary directory we created.
    let safe_name=dir.file_name().is_some_and(|name|
        name.to_string_lossy().starts_with("CyberGladiatorsFix-DXVKTrace-"));
    if safe_name && dir.parent()==Some(std::env::temp_dir().as_path()){
        if let Err(e)=fs::remove_dir_all(&dir){
            show(&format!("DXVK_EVIDENCE WARNING: could not clean temporary DXVK traces: {e}"),root);
        }
    }
}
/// Native AVI is now preferred for the field-validated WineD3D/DXVK case
/// while retaining the isolated-AVI fallback when the optional CDDA bridge
/// is disabled. The x86 bridge is the only place where the guarded fix runs;
/// the original video and game resource bytes remain untouched.
fn user_launch(root:&Path,trace:bool,logo:bool)->Result<(),String>{
    let game=root.join(install::DIR);
    settings::ensure(&game)?;
    if !settings::read(&game)?{
        if settings::read_cdda(&game)? {
            let (child,dxvk_logs)=verify::launch_native_avi_repaired(root,trace,logo)?;
            show(&format!(
                "INTRO_NATIVE GAME_STARTED pid={} renderer=nGlide-D3D9 guarded_wndproc_repair=1 trace={trace} external_helper=0",
                child.id()
            ),root);
            let result=if trace{wait_diagnostic(child,root)}else{wait_game(child,root)};
            report_dxvk_evidence(root,dxvk_logs);
            return result;
        }
        show("INTRO fallback=isolated_MCI reason=CDDA_bridge_disabled; no window-procedure fix available without the in-process bridge",root);
        return isolated_intro_launch(root,trace,logo);
    }
    show("INTRO skipped by user's [Video] SkipIntro=1; launching game without AVI",root);
    let child=verify::launch(root,logo,trace,false)?;
    show(&format!("GAME_STARTED pid={} intro_skipped=1 trace={trace}",child.id()),root);
    if trace{wait_diagnostic(child,root)}else{wait_game(child,root)}
}
/// One shared implementation for normal and trace isolated-intro launch.
/// The isolated AVI/DXVK approach is user-confirmed; native in-process
/// video remains reserved for the explicit root-cause diagnostic.
fn isolated_intro_launch(root:&Path,trace:bool,logo:bool)->Result<(),String>{
    intro::play_before_game(root)?;
    let exe=root.join(install::DIR).join("cyber.exe");
    let launched=verify::launch_after_external_intro(root,trace,logo);
    let (child,dxvk_logs)=match launched{
        Ok(pair)=>pair,
        Err(e)=>{
            // Attempt to restore the recognized original intro patch if
            // game creation failed after applying the skip override.
            let _=patch::set_skip_intro(&exe,false);
            return Err(e);
        }
    };
    show(&format!(
        "INTRO_EXTERNAL GAME_STARTED pid={} runtime_intro_skip=1 trace={} renderer=nGlide-D3D9 (DXVK when configured)",
        child.id(),trace
    ),root);
    let result=if trace{wait_diagnostic(child,root)}else{wait_game(child,root)};
    report_dxvk_evidence(root,dxvk_logs);
    let restore=patch::set_skip_intro(&exe,false);
    match &restore{
        Ok(_)=>show("INTRO_EXTERNAL RESTORED original in-game intro patch; user INI unchanged",root),
        Err(e)=>show(&format!("INTRO_EXTERNAL RESTORE_FAILED: {e}; next launch will reapply INI"),root),
    }
    restore?;
    result
}
/// Reproduce the ORIGINAL in-process MCIAVI -> nGlide D3D9/DXVK crash.
/// This explicitly bypasses our external-video curtain. No invasive Glide
/// pointer hook is enabled. The first-chance observer in the optional CDDA
/// bridge collects the faulting instruction and a bounded stack fingerprint.
fn diagnose_dxvk(root:&Path,wndproc_repair:bool)->Result<(),String>{
    let game=root.join(install::DIR);
    settings::ensure(&game)?;
    if settings::read(&game)? {
        return Err("DXVK root-cause reproduction requires [Video] SkipIntro=0 in the existing game INI; do not alter the original game assets".into());
    }
    if !settings::read_cdda(&game)? {
        return Err("DXVK root-cause stack observer requires [Audio] EnableCDDA=1 to load the 32-bit diagnostic bridge; no other file is modified".into());
    }
    show("DXVK_ROOTCAUSE BEGIN mode=native_in_game_AVI renderer=nGlide_D3D9 diagnostic_glide_hook=OFF external_video=OFF dxvk_deferred_surface=OFF",root);
    if wndproc_repair {
        show("DXVK_WNDPROC_REPAIR_TEST experimental=1 scope=game_process_only: after native AVI closes, restore original game ANSI window class proc ONLY if current Win32 WndProc resolves to WineD3D; abort on unexpected module, HWND, thread, Unicode or game class. No DXVK/Wine DLL or game files are modified",root);
    }else{
        show("DXVK_ROOTCAUSE Purpose: observe first-chance 0xC00000FD, capture fault instruction bytes, candidate x86 stack addresses and fresh DXVK D3D9 log; no fixes or workarounds applied",root);
    }
    let (child,dxvk_dir)=verify::launch_renderer_ab(root,"1",false,false,wndproc_repair)?;
    show(&format!("DXVK_ROOTCAUSE GAME_STARTED pid={} native_avi=ON (may reproduce known crash)",child.id()),root);
    let result=wait_diagnostic(child,root);
    report_dxvk_evidence(root,dxvk_dir);
    show("DXVK_ROOTCAUSE END; EXCEPTION_CAPTURE alone is not a symbolized call stack; see DXVK_STACK_CANDIDATES and DXVK_FAULT_CODE, then use WineDbg if necessary",root);
    result
}
fn dispatch(args:&[OsString],root:&Path)->Result<(),String>{
    let version=format!("{}-{}",env!("CARGO_PKG_VERSION"),env!("CYBER_COMMIT"));
    match args{
        [one] if one.to_str()==Some("--version")=>{println!("{NAME} {version}");return Ok(());},
        [one] if one.to_str()==Some("--help")=>{println!("{NAME} {version}\n{HELP}");return Ok(());},
        [one] if one.to_str()==Some("--dxvk-rootcause")=>return diagnose_dxvk(root,false),
        [one] if one.to_str()==Some("--dxvk-wndproc-repair-test")=>return diagnose_dxvk(root,true),
        [one] if one.to_str()==Some("--check-bundle")=>{
            let report=bundle::describe()?;
            show(&report,root);
            return Ok(());
        },
        [one] if one.to_str()==Some("--verify")=>{let r=verify::verify(root)?;show(&r,root);return Ok(());},
        [one] if one.to_str()==Some("--max-graphics")=>{
            let r=graphics::maximize_saved(&root.join(install::DIR))?;
            show(&r,root);return Ok(());
        },
        [one] if one.to_str()==Some("--launch")=>return user_launch(root,false,false),
        [one] if one.to_str()==Some("--trace")=>return user_launch(root,true,false),
        [one,two] if one.to_str()==Some("--launch") && two.to_str()==Some("--trace")=>
            return user_launch(root,true,false),
        [one,two,three] if one.to_str()==Some("--launch")
            && two.to_str()==Some("--trace")
            && three.to_str()==Some("--no-glide-hook")=>{
            let child=verify::launch_with_options(root,false,true,false,0,true)?;
            show(&format!(
                "Started CYBER.EXE native Glide control (no probe, PID {}); Wine trace at {}",
                child.id(),root.join("CyberGladiatorsFix.log").display()
            ),root);
            return wait_diagnostic(child,root);
        },
        [one,two,three] if one.to_str()==Some("--launch")
            && two.to_str()==Some("--trace")
            && three.to_str()==Some("--external-intro")=>
            return isolated_intro_launch(root,true,false),
        [one,two,three] if one.to_str()==Some("--launch")
            && two.to_str()==Some("--trace")
            && matches!(three.to_str(),Some("--nglide-d3d9"|"--dxvk-defer-surface"|"--dxvk-desktop-refresh"|"--nglide-vulkan"))=>{
            let (backend,dialog,desktop_refresh,label)=match three.to_str().unwrap(){
                "--nglide-d3d9"=>("1",false,false,"nGlide DirectX9 / DXVK control"),
                "--dxvk-defer-surface"=>("1",true,false,"nGlide DirectX9 / DXVK deferred-surface experiment"),
                "--dxvk-desktop-refresh"=>("1",false,true,"nGlide DirectX9 / DXVK desktop-refresh experiment"),
                "--nglide-vulkan"=>("2",false,false,"nGlide native Vulkan control"),
                _=>unreachable!(),
            };
            let (child,dxvk_log_dir)=verify::launch_renderer_ab(root,backend,dialog,desktop_refresh,false)?;
            show(&format!(
                "Started CYBER.EXE {label} (no Glide probe, PID {}); renderer={} NGLIDE_REFRESH={} DXVK_DEFER_SURFACE={} Wine trace at {}; isolated DXVK log folder={}",
                child.id(),backend,if desktop_refresh{"1 (desktop)"}else{"2 (60 Hz)"},dialog,root.join("CyberGladiatorsFix.log").display(),
                dxvk_log_dir.as_ref().map(|p|p.display().to_string()).unwrap_or_else(||"not applicable (native Vulkan)".into())
            ),root);
            let result=wait_diagnostic(child,root);
            report_dxvk_evidence(root,dxvk_log_dir);
            return result;
        },
        [one,two,three] if one.to_str()==Some("--launch")
            && two.to_str()==Some("--trace")
            && three.to_str().and_then(parse_video_delay).is_some()=>{
            let delay=three.to_str().and_then(parse_video_delay).unwrap();
            let mut child=verify::launch_with_video_delay(root,false,true,false,delay)?;
            show(&format!(
                "Started CYBER.EXE video-close delay experiment ({delay} ms, PID {}); Wine trace at {}",
                child.id(),root.join("CyberGladiatorsFix.log").display()
            ),root);
            return wait_diagnostic(child,root);
        },
        [one,two,three] if one.to_str()==Some("--launch")
            && two.to_str()==Some("--trace") && three.to_str()==Some("--passthru-noop")=>{
            let mut child=verify::launch(root,false,true,true)?;
            show(&format!("Started CYBER.EXE diagnostic pass-through NO-OP mode (PID {}); Wine trace at {}",
                child.id(),root.join("CyberGladiatorsFix.log").display()),root);
            return wait_diagnostic(child,root);
        },
        [one,two] if one.to_str()==Some("--launch") && two.to_str()==Some("--3dfx-logo")=>
            return user_launch(root,false,true),
        _=>{},
    }
    let upgrade_cdda=matches!(args,[one] if one.to_str()==Some("--upgrade-cdda"));
    if !upgrade_cdda && (args.iter().any(|x|x.to_string_lossy().starts_with("--"))||args.len()>1){
        return Err(format!("Invalid option(s).\n{HELP}"));
    }
    let target=root.join(install::DIR);
    if target.is_symlink(){return Err("Refusing symlink game directory".into());}
    if target.exists(){
        if !args.is_empty() && !upgrade_cdda{return Err("Installation already exists: refusal to overwrite".into());}
        let exe=target.join("cyber.exe");
        // Validate the text settings before modifying the game binary.
        settings::ensure(&target)?;
        let skip_intro=settings::read(&target)?;
        let enable_cdda=settings::read_cdda(&target)?;
        if upgrade_cdda && !enable_cdda {return Err("--upgrade-cdda requires [Audio] EnableCDDA=1".into());}
        if enable_cdda {cdda::try_install_with_consent(root,&target,upgrade_cdda)?;}
        let status=patch::apply(&exe)?;
        let audio=patch::set_cdda(&exe,enable_cdda)?;
        let intro=patch::set_skip_intro(&exe,skip_intro)?;
        let glide=nglide::try_install(root,&target)?;
        show(&format!("{status}. {audio}. {intro}. nGlide {}",if glide{"ready"}else{"missing"}),root);
        show("Installation ready (game not started). Launch with: CyberGladiatorsFix.exe --launch",root);
        return Ok(());
    }
    if upgrade_cdda{return Err("--upgrade-cdda requires an existing game installation".into());}
    let input=if let Some(arg)=args.first(){PathBuf::from(arg)}else{
        install::find_cue(&std::env::current_dir().map_err(|e|e.to_string())?)?
    };
    require_cue(&input)?;
    show(&format!("Selected CUE sheet: {}",input.display()),root);
    let status=install::install(&input,&target,root)?;
    show(&status,root);
    Ok(())
}
fn main(){
    let root=std::env::current_exe().ok().and_then(|p|p.parent().map(Path::to_path_buf))
        .unwrap_or_else(||PathBuf::from("."));
    let args:Vec<OsString>=std::env::args_os().skip(1).collect();
    // --help and --version are read-only queries; do not erase the last
    // diagnostic session just to display the CLI documentation.
    let read_only=matches!(args.as_slice(),[one] if matches!(one.to_str(),Some("--help"|"--version")));
    if !read_only {
        if let Err(e)=diagnostics::start_session(&root,&args){
            eprintln!("{NAME}: ERROR: {e}");
            std::process::exit(1);
        }
    }
    if let Err(e)=dispatch(&args,&root){
        show(&format!("{NAME}: ERROR: {e}"),&root);
        if !read_only {diagnostics::log(&root,"SESSION","END failure");}
        std::process::exit(1);
    }
    if !read_only {diagnostics::log(&root,"SESSION","END success");}
}

#[cfg(test)]
mod diagnostics_tests {
    use super::*;
    #[test]
    fn video_delay_cli_accepts_only_safe_bounded_values(){
        for ms in [100,250,500,1000] {
            assert_eq!(parse_video_delay(&format!("--video-delay={ms}")),Some(ms));
        }
        for arg in ["--video-delay=0","--video-delay=10000","--video-delay=-1",
                    "--video-delay=wat","--video-delay=500ms","--video-delay="] {
            assert_eq!(parse_video_delay(arg),None);
        }
    }
    #[test]
    fn help_lists_only_public_single_file_controls(){
        for expected in ["--launch","--trace","--verify","--max-graphics",
            "--upgrade-cdda","--check-bundle","--dxvk-rootcause",
            "--dxvk-wndproc-repair-test","--3dfx-logo","--version","--help",
            "CyberGladiatorsFix.log","isolated AVI player"]{
            assert!(HELP.contains(expected),"{expected}");
        }
        assert!(!HELP.contains("CyberGladiatorsIntro.exe"));
        assert!(!HELP.contains("CyberGladiatorsFix-WineTrace.log"));
        assert!(!HELP.contains("--video-delay="));
    }
}
