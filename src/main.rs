// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows console patcher/installer; supports Wine on Linux.
#[cfg(not(windows))]
compile_error!("CyberGladiatorsFix targets Windows x64 (and Wine on Linux). Build for a Windows target.");
mod cue;
mod cdda;
mod audio;
mod install;
mod edition;
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
use std::fs::{File,OpenOptions};
use std::io::Write;
use isomage::{detect_and_parse_filesystem,TreeNode};
use std::path::{Path,PathBuf};

const NAME:&str="CyberGladiatorsFix";
const HELP:&str=r"CyberGladiatorsFix — open-source Windows x64 installer and launcher (Wine compatible)

Usage:
  CyberGladiatorsFix.exe               Auto-detect verified installed game or CUE/BIN, install if missing, then play
  CyberGladiatorsFix.exe --install-only   Install or safely upgrade without starting the game
  CyberGladiatorsFix.exe --inspect image.iso
  CyberGladiatorsFix.exe --inspect image.cue [track.bin|track.img]
  CyberGladiatorsFix.exe --launch-no-verify   Launch installed game without any file integrity or presence preflight
  CyberGladiatorsFix.exe --verify      Strict immutable assets and executable; IQ presence
  CyberGladiatorsFix.exe --verbose     Detailed diagnostics (combinable)
  CyberGladiatorsFix.exe --version
  CyberGladiatorsFix.exe --help       (also -h)

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
Legacy nGlide/DXVK command paths have been retired; normal game sessions
use only the source-pinned GameFrame wrapper.
";
fn show(s:&str,root:&Path){
    println!("{s}");
    diagnostics::log(root,"Launcher",s);
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
/// The standard one-click path uses the embedded, SHA-256-verified GameFrame
/// renderer. Retired nGlide/DXVK command paths are not shipped.
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
/// Unsupported-CD support report, without game installation, edition
/// matching, runtime logging or changes to the supplied ISO/CUE/BIN/IMG.
fn inspect_disc(root:&Path,args:&[OsString],verbose:bool)->Result<(),String>{
    if args.len()<2 || args.len()>3{return Err("Usage: --inspect image.iso | image.cue [track.bin|track.img]".into());}
    let image=PathBuf::from(&args[1]);
    let image=if image.is_absolute(){image}else{root.join(image)};
    let is_cue=image.extension().is_some_and(|e|e.eq_ignore_ascii_case("cue"));
    let is_iso=image.extension().is_some_and(|e|e.eq_ignore_ascii_case("iso"));
    if !is_cue && !is_iso{return Err("Inspect input must be an ISO or CUE".into());}
    let raw=if let Some(arg)=args.get(2){
        if !is_cue {return Err("BIN/IMG override requires CUE".into());}
        let candidate=PathBuf::from(arg);
        let candidate=if candidate.is_absolute(){candidate}else{root.join(candidate)};
        if !candidate.extension().is_some_and(|e|e.eq_ignore_ascii_case("bin") ||
                                                   e.eq_ignore_ascii_case("img")){
            return Err("Explicit track must be BIN or IMG".into());
        }
        Some(candidate)
    }else{None};
    let destination=root.join("CyberGladiatorsFix_inspect.log");
    if fs::symlink_metadata(&destination).is_ok_and(|m|!m.is_file() || m.file_type().is_symlink()){
        return Err("Inspect log path must be a regular file".into());
    }
    let mut report=OpenOptions::new().create(true).write(true).truncate(true)
        .open(&destination).map_err(|e|format!("Cannot write inspect log: {e}"))?;
    writeln!(report,"CyberGladiatorsFix INSPECT v1\nSource={}\nTrack={}",
        image.display(),raw.as_ref().map(|p|p.display().to_string()).unwrap_or_default())
        .map_err(|e|e.to_string())?;
    let result=(||->Result<(),String>{
        let m=fs::symlink_metadata(&image).map_err(|e|e.to_string())?;
        if !m.is_file() || m.file_type().is_symlink(){return Err("Source image is not a regular file".into());}
        if let Some(ref selected)=raw {
            let m=fs::symlink_metadata(selected).map_err(|e|e.to_string())?;
            if !m.is_file() || m.file_type().is_symlink(){return Err("Explicit track is not a regular file".into());}
        }
        let normalized=if is_cue {
            let tracks=cue::tracks(&image)?;
            writeln!(report,"CUE_TRACKS={}",tracks.len()).map_err(|e|e.to_string())?;
            for track in tracks {
                writeln!(report,"TRACK={:02}\tMODE={}\tFILE={}\tINDEX01={}",
                    track.number,track.mode,track.file.replace(['\r','\n','\t'],"_"),track.index01)
                    .map_err(|e|e.to_string())?;
            }
            Some(cue::normalize_with_data(&image,root,raw.as_deref())?)
        }else{None};
        let data=normalized.as_ref().map(|x|x.0.as_path()).unwrap_or(&image);
        let mut file=File::open(data).map_err(|e|e.to_string())?;
        let len=file.metadata().map_err(|e|e.to_string())?.len();
        let tree=detect_and_parse_filesystem(&mut file,&data.to_string_lossy())
            .map_err(|e|format!("Unknown/invalid CD filesystem: {e}"))?;
        writeln!(report,"DataBytes={len}\nPATH\tSIZE_BYTES\tOFFSET\tTYPE")
            .map_err(|e|e.to_string())?;
        fn walk(node:&TreeNode,path:&Path,depth:usize,count:&mut usize,
                out:&mut File,verbose:bool)->Result<(),String>{
            if depth>40{return Err("Inspect filesystem nesting limit reached".into());}
            for child in &node.children{
                *count+=1;
                if *count>25000{return Err("Inspect file count limit reached".into());}
                let name=child.name.replace(['\r','\n','\t'],"_");
                let relative=path.join(name);
                let kind=if child.is_directory{"DIRECTORY"}else{"FILE"};
                writeln!(out,"{}\t{}\t{}\t{}",relative.display(),child.size,
                    child.file_location.unwrap_or(0),kind).map_err(|e|e.to_string())?;
                if verbose{println!("INSPECT: {} bytes={} {}",relative.display(),child.size,kind);}
                if child.is_directory{walk(child,&relative,depth+1,count,out,verbose)?;}
            }
            Ok(())
        }
        let mut count=0;
        walk(&tree,Path::new(""),0,&mut count,&mut report,verbose)?;
        writeln!(report,"Entries={count}\nINSPECT_RESULT=COMPLETE")
            .map_err(|e|e.to_string())?;
        Ok(())
    })();
    if let Err(ref failure)=result{
        let _=writeln!(report,"INSPECT_RESULT=FAILED\nError={failure}");
    }
    report.flush().map_err(|e|e.to_string())?;
    println!("Inspection report: {}",destination.display());
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
        [one] if one.to_str()==Some("--version")=>{println!("{NAME} {version}");return Ok(());},
        [one] if matches!(one.to_str(),Some("--help"|"-h"))=>{println!("{NAME} {version}\n{HELP}");return Ok(());},
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
        _=>{},
    }
    Err(format!("Invalid option(s). Positional CUE/ISO media and unknown flags are not supported; use --inspect for diagnostics.\n{HELP}"))
}
fn main(){
    let root=std::env::current_exe().ok().and_then(|p|p.parent().map(Path::to_path_buf))
        .unwrap_or_else(||PathBuf::from("."));
    let original:Vec<OsString>=std::env::args_os().skip(1).collect();
    if original.iter().filter(|arg|*arg=="--verbose").count()>1 {
        eprintln!("{NAME}: --verbose specified twice");std::process::exit(2);
    }
    let verbose=original.iter().any(|arg|arg=="--verbose");
    let args:Vec<OsString>=original.into_iter().filter(|arg|arg!="--verbose").collect();
    if args.first().is_some_and(|a|a=="--inspect"){
        if let Err(e)=inspect_disc(&root,&args,verbose){
            eprintln!("{NAME}: inspection error: {e}");std::process::exit(2);
        }
        return;
    }
    // Help, version and explicit --inspect do not reset the installation log.
    let read_only=matches!(args.as_slice(),[one] if matches!(one.to_str(),Some("--help"|"-h"|"--version")));
    if !read_only {
        if let Err(e)=diagnostics::start_session(&root,&args){
            eprintln!("{NAME}: ERROR: {e}");
            std::process::exit(1);
        }
        if verbose {
            let msg=format!("VERBOSE mode=on source=launcher game_dir={} running_under_wine={}",
                root.join(install::DIR).display(),!diagnostics::is_native_windows());
            diagnostics::log(&root,"VERBOSE",&msg);
            println!("{msg}");
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
    fn removed_disc_inspection_commands_are_rejected(){
        let root=Path::new("C:/CyberGladiatorsFix-test-nonexistent");
        for arguments in [
            vec![OsString::from("--inspect-disc")],
            vec![OsString::from("--inspect-disc"),OsString::from("game.cue")],
        ] {
            let err=dispatch(&arguments,root).unwrap_err();
            assert!(err.starts_with("Invalid option(s)."),"{err}");
        }
    }
    #[test]
    fn obsolete_nglide_and_dxvk_switches_are_rejected(){
        let root=Path::new("C:/CyberGladiatorsFix-test-nonexistent");
        for args in [
            vec!["--launch"],
            vec!["--trace"],
            vec!["--dxvk-rootcause"],
            vec!["--launch","--smooth-video-test"],
            vec!["--launch","--gameframe-test"],
            vec!["--launch","--trace","--nglide-vulkan"],
        ] {
            let arguments=args.iter().map(OsString::from).collect::<Vec<_>>();
            assert!(dispatch(&arguments,root).unwrap_err().starts_with("Invalid option(s)."),"{args:?}");
        }
    }
    #[test]
    fn help_lists_only_one_click_controls(){
        for expected in ["CyberGladiatorsFix.exe               Auto-detect",
            "--install-only","--launch-no-verify","--verify",
            "--version","--help",
            "music/TrackNN.flac","CyberGladiatorsFix.dll",
            "CyberGladiatorsFix_installer.log","CyberGladiatorsFix.log","GameFrame"]{
            assert!(HELP.contains(expected),"{expected}");
        }
        for removed in ["--inspect-disc","--max-graphics","--check-bundle","--upgrade-cdda"]{
            assert!(!HELP.contains(removed),"{removed}");
        }
        assert!(!HELP.lines().any(|line| line.trim()=="CyberGladiatorsFix.exe --launch"));
        assert!(!HELP.contains("--us-virtual-ddraw-test"));
        assert!(!HELP.contains("--avi-in-game-window"));
        assert!(!HELP.contains("--nglide-vulkan"));
        assert!(!HELP.contains("CyberGladiatorsIntro.exe"));
    }
}
