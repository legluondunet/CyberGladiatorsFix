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

use std::ffi::OsString;
use std::fs;
use std::path::{Path,PathBuf};
use std::io::Write;

const NAME:&str="CyberGladiatorsFix";
const HELP:&str="CyberGladiatorsFix - Windows x64 installer with experimental x86 CDDA FLAC bridge

Usage:
  CyberGladiatorsFix.exe
  CyberGladiatorsFix.exe <disc.cue>
  CyberGladiatorsFix.exe --verify
  CyberGladiatorsFix.exe --upgrade-cdda
  CyberGladiatorsFix.exe --max-graphics
  CyberGladiatorsFix.exe --launch [--3dfx-logo|--trace]\n  CyberGladiatorsFix.exe --launch --trace --passthru-noop
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
The --launch command reads SkipIntro and EnableCDDA before starting the game;
Both --launch and --launch --trace now use NGLIDE_BACKEND=0 (automatic).
--launch --trace additionally captures Wine exceptions and DLL loads in
CyberGladiatorsFix-WineTrace.log beside this launcher.
--launch --trace --passthru-noop tests bypassing only grSstPassthruMode(1)
after the intro AVI, using an in-memory diagnostic hook; no files are patched.
--launch --trace --video-delay=N waits N milliseconds after native WinMM
closes the opening AVI and BEFORE the game can restore Glide; only
100, 250, 500 or 1000 are allowed. Requires SkipIntro=0 and EnableCDDA=1.
This is a diagnostic experiment, not a permanent video compatibility fix.
Close the game before manually changing saved settings.
--verify is read-only; --launch verifies first; --3dfx-logo additionally
requires a user-provided 32-bit 3DfxSpl.dll. The bundled-build x86
CyberGladiatorsCDDA.dll must be beside this installer. The bridge writes
CyberGladiatorsCDDA.log next to the game. No game or nGlide files bundled.
Unrecognized CD editions/executables are refused rather than modified.
";
fn show(s:&str,root:&Path){
    println!("{s}");
    let p=root.join("CyberGladiatorsFix.log");
    if let Ok(mut f)=fs::OpenOptions::new().create(true).append(true).open(&p){let _=writeln!(f,"{s}");}
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
fn wait_diagnostic(mut child:std::process::Child,root:&Path)->Result<(),String>{
    let pid=child.id();
    let start=std::time::Instant::now();
    let status=child.wait().map_err(|e|e.to_string())?;
    show(&format!(
        "DIAGNOSTIC_EXIT pid={pid} exit_status={status} elapsed_ms={} expected_bridge_log={}",
        start.elapsed().as_millis(),
        root.join(install::DIR).join("CyberGladiatorsCDDA.log").display()
    ),root);
    if !status.success(){
        return Err(format!("Game exited with status {status}; see CyberGladiatorsFix-WineTrace.log"));
    }
    Ok(())
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
            let status=child.wait().map_err(|e|e.to_string())?;
            if !status.success(){return Err(format!("Game exited with status {status}"));}
            return Ok(());
        },
        [one,two] if one.to_str()==Some("--launch") && two.to_str()==Some("--trace")=>{
            let mut child=verify::launch(root,false,true,false)?;
            show(&format!("Started CYBER.EXE diagnostic mode (PID {}); nGlide backend AUTO, Wine trace at {}",
                child.id(),root.join("CyberGladiatorsFix-WineTrace.log").display()),root);
            return wait_diagnostic(child,root);
        },
        [one,two,three] if one.to_str()==Some("--launch")
            && two.to_str()==Some("--trace")
            && three.to_str().and_then(parse_video_delay).is_some()=>{
            let delay=three.to_str().and_then(parse_video_delay).unwrap();
            let mut child=verify::launch_with_video_delay(root,false,true,false,delay)?;
            show(&format!(
                "Started CYBER.EXE video-close delay experiment ({delay} ms, PID {}); Wine trace at {}",
                child.id(),root.join("CyberGladiatorsFix-WineTrace.log").display()
            ),root);
            return wait_diagnostic(child,root);
        },
        [one,two,three] if one.to_str()==Some("--launch")
            && two.to_str()==Some("--trace") && three.to_str()==Some("--passthru-noop")=>{
            let mut child=verify::launch(root,false,true,true)?;
            show(&format!("Started CYBER.EXE diagnostic pass-through NO-OP mode (PID {}); Wine trace at {}",
                child.id(),root.join("CyberGladiatorsFix-WineTrace.log").display()),root);
            return wait_diagnostic(child,root);
        },
        [one,two] if one.to_str()==Some("--launch") && two.to_str()==Some("--3dfx-logo")=>{
            let mut child=verify::launch(root,true,false,false)?;
            show(&format!("Started CYBER.EXE with 3dfx splash (PID {})",child.id()),root);
            let status=child.wait().map_err(|e|e.to_string())?;
            if !status.success(){return Err(format!("Game exited with status {status}"));}
            return Ok(());
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
    if let Err(e)=dispatch(&args,&root){
        show(&format!("{NAME}: ERROR: {e}"),&root);
        std::process::exit(1);
    }
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
        assert!(HELP.contains("CyberGladiatorsFix-WineTrace.log"));
        assert!(HELP.contains("NGLIDE_BACKEND=0"));
        assert!(HELP.contains("--launch --trace --passthru-noop"));
        assert!(HELP.contains("--upgrade-cdda"));
        assert!(HELP.contains("--video-delay=100|250|500|1000"));
    }
}
