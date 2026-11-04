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

use std::ffi::OsString;
use std::fs;
use std::path::{Path,PathBuf};

const NAME:&str="CyberGladiatorsFix";
const HELP:&str="CyberGladiatorsFix - Windows x64 installer with experimental x86 CDDA FLAC bridge

Usage:
  CyberGladiatorsFix.exe
  CyberGladiatorsFix.exe <disc.cue>
  CyberGladiatorsFix.exe --verify
  CyberGladiatorsFix.exe --upgrade-cdda
  CyberGladiatorsFix.exe --max-graphics
  CyberGladiatorsFix.exe --launch [--3dfx-logo|--trace]\n  CyberGladiatorsFix.exe --launch --trace --passthru-noop
  CyberGladiatorsFix.exe --launch --trace --no-glide-hook
  CyberGladiatorsFix.exe --launch --trace --nglide-d3d9
  CyberGladiatorsFix.exe --launch --trace --dxvk-defer-surface
  CyberGladiatorsFix.exe --launch --trace --external-intro
  CyberGladiatorsFix.exe --launch --trace --dxvk-desktop-refresh
  CyberGladiatorsFix.exe --launch --trace --nglide-vulkan
  CyberGladiatorsFix.exe --launch --trace --video-delay=100|250|500|1000
  CyberGladiatorsFix.exe --help
  CyberGladiatorsFix.exe --version

No arguments: discover exactly one CUE in the current working directory;
read one or more .bin tracks referenced by that CUE; standalone ISO/BIN is unsupported;
install CYBER/ into CyberGladiatorsFix beside the patcher, patch CYBER.EXE,
create CYBER.EXE.bak, install the local x86 CDDA bridge, generate CYBER.INI with maximum graphics, and extract
available CUE AUDIO tracks as TrackNN.flac,
and optionally copy a user-installed nGlide glide.dll.
If CyberGladiatorsFix exists, install x86 CDDA DLL and upgrade known USA
speed/no-CD/intro patches; never re-extract the game or FLACs. Keep cyber.exe.bak.
--upgrade-cdda explicitly approves a one-time backed-up replacement of an unknown x86 CDDA DLL.\nAfter installation, the saved SHA-256 receipt verifies subsequent updates without a hard-coded hash list.\n--max-graphics edits only two validated quality fields in an existing CYBER.INI
and saves cyber.ini.bak. The installer creates CYBER.INI on fresh installs;
CyberGladiatorsFix.ini is separate text: [Video] SkipIntro=0 or 1;
[Audio] EnableCDDA=0 (safe system WinMM) or 1 (experimental FLAC bridge).
[Integrity] AllowModifiedIQ=0 (default) or 1 (explicit launch-only
permission to retain same-size changes in iq/iq00.dat..iq99.dat).
With AllowModifiedIQ=1, each mismatch is warned with both SHA-256 values;
all non-IQ files, file sizes, paths, and the original CD manifest remain strict.
--verify ALWAYS checks all original CD asset hashes, ignoring this permission.
Make a backup of changed IQ files before enabling this option.
The --launch command reads SkipIntro and EnableCDDA before starting the game;
Both --launch and --launch --trace now use NGLIDE_BACKEND=0 (automatic).
--launch --trace additionally captures Wine exceptions and DLL loads in
CyberGladiatorsFix.log beside this launcher.
--launch --trace --passthru-noop tests bypassing only grSstPassthruMode(1)
after the intro AVI, using an in-memory diagnostic hook; no files are patched.
--launch --trace --no-glide-hook preserves AVI, window and crash telemetry
but leaves the game's Glide function pointer unchanged. Requires
SkipIntro=0 and EnableCDDA=1. Compare with --launch --trace to isolate
effects of the diagnostic Glide hook.
--launch --trace --nglide-d3d9 forces nGlide's DirectX9 backend
(NGLIDE_BACKEND=1) with DXVK HUD and DXVK logging, to verify whether
a properly configured Wine prefix loads DXVK's d3d9.dll.
--launch --trace --dxvk-defer-surface additionally enables DXVK's
current d3d9.deferSurfaceCreation=True option. It delays Vulkan surface
creation until the first D3D9 present, useful for mixed-API games.
This is an unverified workaround, not an automatic fix.
--launch --trace --external-intro is an experimental AVI/DXVK
isolation test. It plays splash4.avi via a separate bundled x86 MCI
helper, waits for the helper to exit, then runs CYBER.EXE through DXVK
with the in-game introduction skipped. The helper only reads the AVI.
The user's SkipIntro=0 INI setting is preserved; recognized EXE patch
bytes are restored after the game exits. No other videos are changed.
Requires CyberGladiatorsIntro.exe from the same Windows build, an
installed Wine Video-for-Windows/Cinepak decoder, EnableCDDA=1,
SkipIntro=0, DXVK enabled, and PROTON_USE_WINED3D unset.
The intro player writes CyberGladiatorsFix.log and fails closed.
--launch --trace --dxvk-desktop-refresh keeps DirectX9 / DXVK but
asks nGlide for the desktop refresh rate instead of forcing 60 Hz.
This tests 60/120 Hz mode switches seen in the provided DXVK log.
--launch --trace --nglide-vulkan forces nGlide's own Vulkan backend
(NGLIDE_BACKEND=2), which bypasses DXVK's Direct3D9 translation.
These three renderer tests disable the diagnostic Glide hook.
For DXVK, disable PROTON_USE_WINED3D=1 and enable DXVK in Lutris.
Look for DXVK HUD or cyber_d3d9.log beside cyber.exe to confirm use.
--launch --trace --video-delay=N waits N milliseconds after native WinMM
closes the opening AVI and BEFORE the game can restore Glide; only
100, 250, 500 or 1000 are allowed. Requires SkipIntro=0 and EnableCDDA=1.
This is a diagnostic experiment, not a permanent video compatibility fix.
Close the game before manually changing saved settings.
--verify is read-only; --launch verifies first; --3dfx-logo additionally
requires a user-provided 32-bit 3DfxSpl.dll. The bundled-build x86
CyberGladiatorsCDDA.dll must be beside this installer. The bridge writes
CyberGladiatorsFix.log next to the game. No game or nGlide files bundled.
Unrecognized CD editions/executables are refused rather than modified.
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
            if name.to_ascii_lowercase().ends_with("_d3d9.log")
                && entry.file_type().is_ok_and(|kind|kind.is_file()){
                logs.push(entry.path());
            }
        }
    }
    if logs.is_empty(){
        show("DXVK_EVIDENCE UNCONFIRMED: no fresh *_d3d9.log in the temporary trace folder; renderer selection alone does not prove DXVK loaded",root);
    } else {
        logs.sort();
        let mut all_ok=true;
        for path in logs {
            let label=path.file_name().unwrap_or_default().to_string_lossy();
            match fs::metadata(&path){
                Ok(meta) if meta.len()<=2*1024*1024=>{
                    match fs::read(&path) {
                        Ok(bytes)=>{
                            show(&format!("DXVK_EVIDENCE DLL_LOADED: {} ({} bytes); DXVK text follows in this session log",label,bytes.len()),root);
                            diagnostics::log(root,"DXVK",&String::from_utf8_lossy(&bytes));
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
fn dispatch(args:&[OsString],root:&Path)->Result<(),String>{
    let version=format!("{}-{}",env!("CARGO_PKG_VERSION"),env!("CYBER_COMMIT"));
    match args{
        [one] if one.to_str()==Some("--version")=>{println!("{NAME} {version}");return Ok(());},
        [one] if one.to_str()==Some("--help")=>{println!("{NAME} {version}\n{HELP}");return Ok(());},
        [one] if one.to_str()==Some("--verify")=>{let r=verify::verify(root)?;show(&r,root);return Ok(());},
        [one] if one.to_str()==Some("--max-graphics")=>{
            let r=graphics::maximize_saved(&root.join(install::DIR))?;
            show(&r,root);return Ok(());
        },
        [one] if one.to_str()==Some("--launch")=>{
            let mut child=verify::launch(root,false,false,false)?;
            show(&format!("Started CYBER.EXE (PID {})",child.id()),root);
            return wait_game(child,root);
        },
        [one,two] if one.to_str()==Some("--launch") && two.to_str()==Some("--trace")=>{
            let mut child=verify::launch(root,false,true,false)?;
            show(&format!("Started CYBER.EXE diagnostic mode (PID {}); nGlide backend AUTO, Wine trace at {}",
                child.id(),root.join("CyberGladiatorsFix.log").display()),root);
            return wait_diagnostic(child,root);
        },
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
            && three.to_str()==Some("--external-intro")=>{
            // Isolate 1996 MCIAVI and DXVK in different Wine processes.
            // This is an explicit experiment, never a normal-launch default.
            intro::play_before_game(root)?;
            let exe=root.join(install::DIR).join("cyber.exe");
            let launched=verify::launch_after_external_intro(root);
            let (child,dxvk_log_dir)=match launched {
                Ok(pair)=>pair,
                Err(e)=>{
                    // Recover the exact known in-game intro state if a
                    // failed game spawn left its runtime switch enabled.
                    let _=patch::set_skip_intro(&exe,false);
                    return Err(e);
                }
            };
            show(&format!(
                "INTRO_EXTERNAL GAME_STARTED pid={} skip_internal_avi=1 in-process nGlide/DXVK (game INI retains SkipIntro=0); DXVK log folder={}",
                child.id(),
                dxvk_log_dir.as_ref().map(|p|p.display().to_string()).unwrap_or_else(||"missing".into())
            ),root);
            let game_result=wait_diagnostic(child,root);
            report_dxvk_evidence(root,dxvk_log_dir);
            let restored=patch::set_skip_intro(&exe,false);
            match restored {
                Ok(_) => show("INTRO_EXTERNAL RESTORED in-game intro patch to SkipIntro=0 after game exit; game INI was unchanged",root),
                Err(ref e) => show(&format!("INTRO_EXTERNAL RESTORE_FAILED: {e}; next normal launch will reapply the INI setting"),root),
            }
            restored?;
            return game_result;
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
            let (child,dxvk_log_dir)=verify::launch_renderer_ab(root,backend,dialog,desktop_refresh)?;
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
        [one,two] if one.to_str()==Some("--launch") && two.to_str()==Some("--3dfx-logo")=>{
            let mut child=verify::launch(root,true,false,false)?;
            show(&format!("Started CYBER.EXE with 3dfx splash (PID {})",child.id()),root);
            return wait_game(child,root);
        },
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
    fn help_mentions_debug_trace_command(){
        assert!(HELP.contains("--launch [--3dfx-logo|--trace]"));
        assert!(HELP.contains("CyberGladiatorsFix.log"));
        assert!(HELP.contains("NGLIDE_BACKEND=0"));
        assert!(HELP.contains("--launch --trace --passthru-noop"));
        assert!(HELP.contains("--upgrade-cdda"));
        assert!(HELP.contains("--video-delay=100|250|500|1000"));
        assert!(HELP.contains("--no-glide-hook"));
        assert!(HELP.contains("--nglide-d3d9"));
        assert!(HELP.contains("--dxvk-defer-surface"));
        assert!(HELP.contains("--external-intro"));
        assert!(HELP.contains("--dxvk-desktop-refresh"));
        assert!(HELP.contains("--nglide-vulkan"));
    }
}
