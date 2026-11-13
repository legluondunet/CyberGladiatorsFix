// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows console patcher/installer; supports Wine on Linux.
#[cfg(not(windows))]
compile_error!("CyberGladiatorsFix targets Windows x64 (and Wine on Linux). Build for a Windows target.");
mod cue;
mod cdda;
mod audio;
mod install;
mod edition;
mod nglide;
mod gameframe;
mod graphics;
mod settings;
mod patch;
mod fr_patch;
mod fr_ddraw_static;
mod verify;
mod diagnostics;
mod bundle;
mod migration;
mod autostart;

use std::ffi::OsString;
use std::fs;
use std::path::{Path,PathBuf};

const NAME:&str="CyberGladiatorsFix";
const HELP:&str=r"CyberGladiatorsFix — open-source Windows x64 installer and launcher (Wine compatible)

Usage:
  CyberGladiatorsFix.exe               Auto-detect verified installed game or CUE/BIN, install if missing, then play
  CyberGladiatorsFix.exe <disc.cue>    Install verified retail CD and play
  CyberGladiatorsFix.exe --install-only   Install or safely upgrade without starting the game
  CyberGladiatorsFix.exe --launch-no-verify   Launch installed game without any file integrity or presence preflight
  CyberGladiatorsFix.exe --verify      Strict immutable assets and executable; IQ presence
  CyberGladiatorsFix.exe --inspect-disc [disc.cue]   Read-only US/FR CD fingerprint (no installation)
  CyberGladiatorsFix.exe --version
  CyberGladiatorsFix.exe --help

No launch argument is required. On first use, keep the original .cue
and its .bin track files next to this EXE. CYBER.EXE and retail data are
extracted only from an authenticated, SHA-256-verified US or French retail
CUE/BIN image. Subsequent launches verify the installation and start automatically.

One Windows x64 EXE embeds the open-source GameFrame Glide x86 wrapper
(Apache-2.0) and the project's CyberGladiatorsFix.dll WinMM/CDDA bridge
(GPLv3). They are SHA-256 verified before being installed beside CYBER.EXE.
Any older or unknown glide.dll is renamed to an unused .bak backup before
the bundled verified GameFrame is installed; that GameFrame remains after exit.
Original lossless CD music is stored in music/TrackNN.flac.
No original game data is distributed. The launcher never rewrites IQ data.
During first installation original IQ files undergo strict SHA-256 checks.
Afterwards --verify and automatic launches require IQ files to be present
as regular files, without comparing size or digest. All other retail
assets, original EXE and audio remain strictly integrity checked in normal mode.
The opt-in --launch-no-verify bypasses installation, file preflights and repairs.

Two independent logs appear beside the launcher:
  CyberGladiatorsFix_installer.log (installer, verification and launcher)
  CyberGladiatorsFix.log (WinMM/CDDA and GameFrame game process).
Both contain the same session ID when a game session is started.
CDDA/FLAC music is always on.
The user INI contains only [Video] SkipIntro.
Both US and French retail discs are authenticated by exact EXE and resource
hashes before installing or patching. A single-BIN raw disc with a verified
150-sector zero pause before the first AUDIO INDEX 01 is accepted even when
its original CUE omits INDEX 00; the source CUE/BIN are never rewritten.
The same owned DirectDraw/MCI adapter is selected automatically for US/FR
under Windows and Wine/Proton. No special launch mode is necessary;
both original editions were field-tested on Windows 11 and Wine-staging 11.17.
Older nGlide/developer diagnostics remain available for forensic testing
but are never selected during normal one-click operation.
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
    let event=format!(
        "GAME_EXIT pid={pid} exit_status={status} elapsed_ms={}",
        start.elapsed().as_millis()
    );
    show(&event,root);
    diagnostics::log_runtime(root,"SESSION",&event);
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
        diagnostics::runtime_location(root).display()
    ),root);
    diagnostics::log_runtime(root,"SESSION",&format!(
        "DIAGNOSTIC_EXIT pid={pid} exit_status={status} session_id={}",
        diagnostics::session_id()
    ));
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
                                diagnostics::log_runtime(root,"DXVK_D3D9",&String::from_utf8_lossy(&bytes));
                            }else{
                                // The presence of a ddraw.log is not, alone,
                                // positive identification of D7VK.
                                show(&format!("VIDEO_PROVIDER_EVIDENCE DDRAW_OR_D3D7_LOG_FOUND: {} ({} bytes); verify provider/version in contents",label,bytes.len()),root);
                                diagnostics::log_runtime(root,"LEGACY_DDRAW",&String::from_utf8_lossy(&bytes));
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
/// The original AVI always plays inside CYBER.EXE, with a temporary 4:3
/// black host in the normal path. One x86 WinMM bridge provides video and
/// the guarded DXVK repair; EnableCDDA controls only music emulation.
/// Original game video and resource bytes remain untouched.
fn user_launch(root:&Path,trace:bool,logo:bool,smooth_test:bool)->Result<(),String>{
    let game=root.join(install::DIR);
    settings::ensure(&game)?;
    let skip=settings::read(&game)?;
    let enable_cdda=settings::CDDA_ENABLED;
    if smooth_test && (skip||trace||logo) {
        return Err("--smooth-video-test requires SkipIntro=0 without --trace or --3dfx-logo; INI unchanged".into());
    }
    if !skip {
        // Native AVI always stays in the original game process, regardless
        // of whether FLAC CDDA music was enabled in the existing user INI.
        // --trace retains the same 4:3 layout while capturing diagnostics;
        // --dxvk-rootcause remains the original unmodified MCI control.
        let (child,dxvk_logs)=verify::launch_native_avi_repaired(root,trace,logo,true,smooth_test)?;
        show(&format!(
            "INTRO_NATIVE GAME_STARTED pid={} renderer=nGlide-D3D9 guarded_wndproc_repair=1 trace={trace} external_helper=0 music_flac={}",
            child.id(),u8::from(enable_cdda)
        ),root);
        let result=if trace{wait_diagnostic(child,root)}else{wait_game(child,root)};
        report_dxvk_evidence(root,dxvk_logs);
        return result;
    }
    show("INTRO skipped by user's [Video] SkipIntro=1; launching game without AVI",root);
    let child=verify::launch(root,logo,trace,false)?;
    show(&format!("GAME_STARTED pid={} intro_skipped=1 trace={trace}",child.id()),root);
    if trace{wait_diagnostic(child,root)}else{wait_game(child,root)}
}
/// The standard one-click path uses the embedded, SHA-256-verified GameFrame
/// renderer. Legacy nGlide diagnostics are kept separately for developers.
fn user_gameframe_launch(root:&Path)->Result<(),String>{
    let game=root.join(install::DIR);
    settings::ensure(&game)?;
    let enable_cdda=settings::CDDA_ENABLED;
    let mut glide_session=gameframe::Session::begin(root,&game)?;
    // Start before spawning CYBER.EXE: its DirectDraw initialization may
    // switch the physical desktop mode before the first MCI AVI event.
    // This is passive, native-Windows/FR-only sampling; no hook or graphics
    // setting changes. A failed worker never prevents the working launch.
    let native_fr=diagnostics::is_native_windows()
        && crate::fr_patch::owns(&game.join("cyber.exe")).unwrap_or(false);
    let display_watch=if native_fr{
        diagnostics::DisplayModeWatch::start(root)
    }else{
        None
    };
    let child=verify::launch_gameframe(root)?;
    diagnostics::log_runtime(root,"SESSION",&format!(
        "GAME_STARTED pid={} session_id={}",child.id(),diagnostics::session_id()
    ));
    show(&format!(
        "INTRO_NATIVE GAME_STARTED pid={} renderer=GameFrame-OpenGL gameframe=1 native_video_4x3=1 first_present_handoff=1 music_flac={} WndProc_repair=disabled",
        child.id(),u8::from(enable_cdda)
    ),root);
    let result=wait_game(child,root);
    // Stop/join the read-only sampler before the final launcher snapshot.
    // It has no authority to restore or modify the user's desktop mode.
    drop(display_watch);
    // Read-only after-game audit for both native Windows retail editions;
    // the common DirectDraw adapter never requests a physical mode change.
    if diagnostics::is_native_windows(){
        show(&format!(
            "DISPLAY_MODE_SNAPSHOT stage=after_game_exit {}",
            diagnostics::prelaunch_display_modes()
        ),root);
    }
    // Keep the verified embedded GameFrame DLL after exit; any original
    // glide.dll stays archived in a unique .bak, never auto-restored.
    let finalized=glide_session.finish();
    match (result,finalized){
        (Err(game),Err(dll))=>Err(format!("{game}; GameFrame finalization failed: {dll}")),
        (Err(game),_)=>Err(game),
        (_,Err(dll))=>Err(dll),
        (Ok(()),Ok(()))=>Ok(()),
    }
}
/// Opt-in comparison of the validated smooth D3D9 handoff against nGlide's
/// Vulkan renderer, retaining the same native MCI AVI, black curtain and
/// guard-checked WndProc repair. No changes to the validated default launch.
fn smooth_vulkan_launch(root:&Path,early_release:bool,auto_fullscreen:bool)->Result<(),String>{
    let game=root.join(install::DIR);
    settings::ensure(&game)?;
    if settings::read(&game)?{
        return Err("--smooth-video-test --nglide-vulkan requires [Video] SkipIntro=0; INI unchanged".into());
    }
    let enable_cdda=settings::CDDA_ENABLED;
    let child=verify::launch_smooth_vulkan(root,early_release,auto_fullscreen)?;
    show(&format!(
        "INTRO_NATIVE GAME_STARTED pid={} renderer=nGlide-Vulkan smooth_video=1 early_host_release={} auto_fullscreen={} guarded_wndproc_repair=1 trace=false external_helper=0 music_flac={}",
        child.id(),u8::from(early_release),u8::from(auto_fullscreen),u8::from(enable_cdda)
    ),root);
    wait_game(child,root)
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
    // The single x86 WinMM bridge is loaded irrespective of the CDDA music
    // preference; audio opt-out does not disable AVI or exception telemetry.
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
        []=>{
            let _lock=autostart::lock(root)?;
            autostart::prepare(root,None)?;
            return user_gameframe_launch(root);
        },
        [one] if one.to_str()==Some("--install-only")=>{
            let _lock=autostart::lock(root)?;
            autostart::prepare(root,None)?;
            let game=root.join(install::DIR);
            let mut glide=gameframe::Session::begin(root,&game)?;
            glide.finish()?;
            show(&format!("AUTO_INSTALL_ONLY verified: {}",verify::verify(root)?),root);
            return Ok(());
        },
        [one] if one.to_str().is_some_and(|s|s.to_ascii_lowercase().ends_with(".cue"))=>{
            let _lock=autostart::lock(root)?;
            let source=PathBuf::from(one);
            autostart::prepare(root,Some(&source))?;
            return user_gameframe_launch(root);
        },
        [one] if one.to_str()==Some("--inspect-disc")=>{
            let cue=install::find_cue(root)?;
            return install::inspect_disc(&cue,root);
        },
        [one,two] if one.to_str()==Some("--inspect-disc")=>{
            return install::inspect_disc(&PathBuf::from(two),root);
        },
        [one] if one.to_str()==Some("--version")=>{println!("{NAME} {version}");return Ok(());},
        [one] if one.to_str()==Some("--help")=>{println!("{NAME} {version}\n{HELP}");return Ok(());},
        [one] if one.to_str()==Some("--dxvk-rootcause")=>return diagnose_dxvk(root,false),
        [one] if one.to_str()==Some("--dxvk-wndproc-repair-test")=>return diagnose_dxvk(root,true),
        [one] if one.to_str()==Some("--verify")=>{let r=verify::verify(root)?;show(&r,root);return Ok(());},
        [one] if one.to_str()==Some("--launch-no-verify")=>{
            // Explicit opt-in: do not scan the installation, hash any file,
            // install/repair modules, or even check whether assets are present.
            let _lock=autostart::lock(root)?;
            show("FAST_LAUNCH mode=no_verify installation=skipped integrity=skipped presence=skipped module_repair=skipped",root);
            let child=verify::launch_gameframe_unchecked(root)?;
            show(&format!("FAST_LAUNCH GAME_STARTED pid={}",child.id()),root);
            return wait_game(child,root);
        },
        [one] if one.to_str()==Some("--launch")=>{
            // Backward-compatible alias for the exact same one-click path.
            let _lock=autostart::lock(root)?;
            autostart::prepare(root,None)?;
            return user_gameframe_launch(root);
        },
        // Historical A/B names are aliases for the SAME common renderer.
        // No edition-specific or host-specific engine is selected here.
        [one,two] if one.to_str()==Some("--launch") && matches!(
            two.to_str(),
            Some("--gameframe"|"--gameframe-test"|"--fr-borderless-test"|"--us-virtual-ddraw-test")
        )=>return user_gameframe_launch(root),
        // Preserve original validated #37 invocation as a compatibility alias.
        [one,two,three] if one.to_str()==Some("--launch")
            && two.to_str()==Some("--gameframe-test")
            && three.to_str()==Some("--avi-first-present-handoff")=>
            return user_gameframe_launch(root),
        [one,two] if one.to_str()==Some("--launch") && two.to_str()==Some("--smooth-video-test")=>
            return user_launch(root,false,false,true),
        [one,two,three] if one.to_str()==Some("--launch")
            && two.to_str()==Some("--smooth-video-test")
            && three.to_str()==Some("--nglide-vulkan")=>
            return smooth_vulkan_launch(root,false,false),
        [one,two,three,four] if one.to_str()==Some("--launch")
            && two.to_str()==Some("--smooth-video-test")
            && three.to_str()==Some("--nglide-vulkan")
            && four.to_str()==Some("--vulkan-early-release")=>
            return smooth_vulkan_launch(root,true,false),
        [one,two,three,four,five] if one.to_str()==Some("--launch")
            && two.to_str()==Some("--smooth-video-test")
            && three.to_str()==Some("--nglide-vulkan")
            && four.to_str()==Some("--vulkan-early-release")
            && five.to_str()==Some("--vulkan-auto-fullscreen-test")=>
            return smooth_vulkan_launch(root,true,true),
        [one] if one.to_str()==Some("--trace")=>return user_launch(root,true,false,false),
        [one,two] if one.to_str()==Some("--launch") && two.to_str()==Some("--trace")=>
            return user_launch(root,true,false,false),
        [one,two,three] if one.to_str()==Some("--launch")
            && two.to_str()==Some("--trace")
            && three.to_str()==Some("--no-glide-hook")=>{
            let child=verify::launch_with_options(root,false,true,false,0,true)?;
            show(&format!(
                "Started CYBER.EXE native Glide control (no probe, PID {}); Wine trace at {}",
                child.id(),diagnostics::runtime_location(root).display()
            ),root);
            return wait_diagnostic(child,root);
        },
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
                child.id(),diagnostics::runtime_location(root).display()
            ),root);
            return wait_diagnostic(child,root);
        },
        [one,two,three] if one.to_str()==Some("--launch")
            && two.to_str()==Some("--trace") && three.to_str()==Some("--passthru-noop")=>{
            let mut child=verify::launch(root,false,true,true)?;
            show(&format!("Started CYBER.EXE diagnostic pass-through NO-OP mode (PID {}); Wine trace at {}",
                child.id(),diagnostics::runtime_location(root).display()),root);
            return wait_diagnostic(child,root);
        },
        [one,two] if one.to_str()==Some("--launch") && two.to_str()==Some("--3dfx-logo")=>
            return user_launch(root,false,true,false),
        _=>{},
    }
    if args.iter().any(|x|x.to_string_lossy().starts_with("--"))||args.len()>1{
        return Err(format!("Invalid option(s).\n{HELP}"));
    }
    let target=root.join(install::DIR);
    if target.is_symlink(){return Err("Refusing symlink game directory".into());}
    if target.exists(){
        if !args.is_empty(){return Err("Installation already exists: refusal to overwrite".into());}
        let exe=target.join("cyber.exe");
        // Validate the text settings before modifying the game binary.
        settings::ensure(&target)?;
        let skip_intro=settings::read(&target)?;
        cdda::try_install(root,&target)?;
        let status=patch::apply(&exe)?;
        let audio=patch::set_cdda(&exe,true)?;
        let intro=patch::set_skip_intro(&exe,skip_intro)?;
        let glide=nglide::try_install(root,&target)?;
        show(&format!("{status}. {audio}. {intro}. nGlide {}",if glide{"ready"}else{"missing"}),root);
        show("Installation ready (game not started). Launch with: CyberGladiatorsFix.exe --launch",root);
        return Ok(());
    }
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
    fn help_lists_only_one_click_controls(){
        for expected in ["CyberGladiatorsFix.exe               Auto-detect",
            "--install-only","--launch-no-verify","--verify","--inspect-disc",
            "--version","--help",
            "music/TrackNN.flac","CyberGladiatorsFix.dll",
            "CyberGladiatorsFix_installer.log","CyberGladiatorsFix.log","GameFrame"]{
            assert!(HELP.contains(expected),"{expected}");
        }
        for removed in ["--max-graphics","--check-bundle","--upgrade-cdda"]{
            assert!(!HELP.contains(removed),"{removed}");
        }
        assert!(!HELP.contains("--launch"));
        assert!(!HELP.contains("--us-virtual-ddraw-test"));
        assert!(!HELP.contains("--avi-in-game-window"));
        assert!(!HELP.contains("--nglide-vulkan"));
        assert!(!HELP.contains("CyberGladiatorsIntro.exe"));
    }
}
