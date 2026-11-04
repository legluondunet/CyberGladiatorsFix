// SPDX-License-Identifier: GPL-3.0-or-later
//! Opt-in, process-isolated startup AVI playback. The helper exits before
//! CYBER.EXE creates its Glide/DXVK device, so the two renderers never share
//! the original game's DirectDraw or window state.
use crate::{bundle,cdda,diagnostics,install::DIR,settings,verify};
use std::fs::{self,File,OpenOptions};
use std::io::{Read,Write};
use std::ffi::c_void;
use std::path::Path;
use std::process::{Child,Command,Stdio};
use std::thread;
use std::time::{Duration,Instant};

const PLAYER:&str="CyberGladiatorsIntro.exe";

fn validate_x86_exe(path:&Path)->Result<(),String>{
    let meta=fs::symlink_metadata(path).map_err(|e|format!("{PLAYER} missing: {e}"))?;
    if !meta.is_file()||meta.file_type().is_symlink()||!(4096..=16*1024*1024).contains(&meta.len()){
        return Err(format!("{PLAYER} must be a regular Windows x86 EXE (4 KiB..16 MiB), not a symlink"));
    }
    let mut bytes=[0u8;1024];
    File::open(path).and_then(|mut f|f.read_exact(&mut bytes))
        .map_err(|e|format!("Cannot read {PLAYER}: {e}"))?;
    if &bytes[..2]!=b"MZ"{return Err(format!("{PLAYER} has no MZ header"));}
    let pe=u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as usize;
    if pe.checked_add(26).is_none_or(|end|end>bytes.len())
        || &bytes[pe..pe+4]!=b"PE\0\0"
        || u16::from_le_bytes(bytes[pe+4..pe+6].try_into().unwrap())!=0x14c
        || u16::from_le_bytes(bytes[pe+22..pe+24].try_into().unwrap())&0x2000!=0
        || u16::from_le_bytes(bytes[pe+24..pe+26].try_into().unwrap())!=0x10b {
        return Err(format!("{PLAYER} must be a Windows x86 PE32 executable"));
    }
    Ok(())
}
/// The video helper owns the original full-screen window. On Esc, Space
/// or normal AVI completion it closes *all* MCI resources, paints that same
/// window black, and sends a READY signal. Only then may the separate game
/// process start. The helper is alive but has no Cinepak/MCI device while
/// DXVK initializes inside the other process.
pub struct IntroHandoff {
    staged:bundle::TemporaryIntro,
    player:Child,
    began:Instant,
    released:bool,
}

#[repr(C)]
#[derive(Default)]
struct Rect{left:i32,top:i32,right:i32,bottom:i32}
struct WindowSearch {pid:u32,found:*mut c_void}
#[link(name="user32")]
unsafe extern "system" {
    fn EnumWindows(callback:Option<unsafe extern "system" fn(*mut c_void,isize)->i32>,param:isize)->i32;
    fn EnumChildWindows(parent:*mut c_void,callback:Option<unsafe extern "system" fn(*mut c_void,isize)->i32>,param:isize)->i32;
    fn GetWindowThreadProcessId(hwnd:*mut c_void,pid:*mut u32)->u32;
    fn IsWindowVisible(hwnd:*mut c_void)->i32;
    fn GetWindowRect(hwnd:*mut c_void,rect:*mut Rect)->i32;
    fn SetForegroundWindow(hwnd:*mut c_void)->i32;
}
unsafe extern "system" fn find_game_window(hwnd:*mut c_void,param:isize)->i32{
    let query=&mut *(param as *mut WindowSearch);
    let mut pid=0u32;
    if GetWindowThreadProcessId(hwnd,&mut pid)==0||pid!=query.pid{return 1;}
    if IsWindowVisible(hwnd)==0{return 1;}
    let mut rect=Rect::default();
    if GetWindowRect(hwnd,&mut rect)==0{return 1;}
    if rect.right-rect.left<320||rect.bottom-rect.top<240{return 1;}
    query.found=hwnd;
    0 // first visible window belonging to the actual CYBER.EXE PID
}
/// Some Wine prefixes place the actual game window under explorer's
/// virtual-desktop parent. If no top-level window belongs to the game PID,
/// search descendants too; do not change the user's Wine desktop options.
unsafe extern "system" fn find_nested_game_window(hwnd:*mut c_void,param:isize)->i32{
    if find_game_window(hwnd,param)==0{return 0;}
    let query=&mut *(param as *mut WindowSearch);
    let _=EnumChildWindows(hwnd,Some(find_game_window),param);
    if query.found.is_null(){1}else{0}
}
fn visible_game_window(pid:u32)->Option<*mut c_void>{
    let mut query=WindowSearch{pid,found:std::ptr::null_mut()};
    unsafe{
        EnumWindows(Some(find_game_window),(&mut query as *mut WindowSearch) as isize);
        if query.found.is_null(){
            EnumWindows(Some(find_nested_game_window),(&mut query as *mut WindowSearch) as isize);
        }
    }
    (!query.found.is_null()).then_some(query.found)
}
impl Drop for IntroHandoff {
    fn drop(&mut self){
        if !self.released {
            // A failed or cancelled spawn must never strand a topmost black
            // window or keep a 32-bit helper running indefinitely.
            let _=self.player.kill();
            let _=self.player.wait();
        }
        // The staged bundle then removes only this invocation's private
        // helper and signal markers. No original game file is touched.
    }
}
impl IntroHandoff {
    /// Close the curtain only after the real game window becomes visible.
    /// If Wine does not expose a top-level HWND for this game's PID, use a
    /// bounded fallback rather than leaving the screen black indefinitely.
    pub fn reveal_when_ready(&mut self,game:&mut Child,root:&Path)->Result<(),String>{
        let pid=game.id();
        let started=Instant::now();
        let mut target=None;
        loop {
            if let Some(hwnd)=visible_game_window(pid){
                target=Some(hwnd);
                // First visible HWND can precede DXVK's first Present.
                // A short settling interval avoids flashing the loading
                // window under the black curtain.
                thread::sleep(Duration::from_millis(350));
                break;
            }
            if let Some(status)=game.try_wait().map_err(|e|format!("Cannot poll game process during handoff: {e}"))?{
                crate::show(&format!("INTRO_HANDOFF GAME_EARLY_EXIT status={status}; releasing curtain"),root);
                break;
            }
            if started.elapsed()>Duration::from_secs(12){
                crate::show("INTRO_HANDOFF FALLBACK no visible game window found for this PID within 12s; releasing curtain to avoid a stuck black screen",root);
                break;
            }
            thread::sleep(Duration::from_millis(40));
        }
        if let Some(hwnd)=target {
            let focused=unsafe{SetForegroundWindow(hwnd)}!=0;
            crate::show(&format!(
                "INTRO_HANDOFF WINDOW_READY pid={pid} hwnd={} focus_requested={} elapsed_ms={}",
                hwnd as usize,focused,started.elapsed().as_millis()
            ),root);
        }
        let release=self.staged.handoff_dir().join("intro.release");
        let mut out=OpenOptions::new().write(true).create_new(true).open(&release)
            .map_err(|e|format!("Cannot release video curtain: {e}"))?;
        out.write_all(b"GAME_WINDOW_READY\n").and_then(|_|out.sync_all())
            .map_err(|e|format!("Cannot commit video curtain release signal: {e}"))?;
        drop(out);
        let deadline=Instant::now();
        loop {
            if let Some(status)=self.player.try_wait()
                .map_err(|e|format!("Cannot inspect video handoff completion: {e}"))?{
                self.released=true;
                crate::show(&format!(
                    "INTRO_HANDOFF CURTAIN_CLOSED helper_status={status} elapsed_ms={}",
                    self.began.elapsed().as_millis()
                ),root);
                if !status.success(){return Err(format!("Video handoff player returned {status}"));}
                return Ok(());
            }
            if deadline.elapsed()>Duration::from_secs(6){
                return Err("Video player did not release its fullscreen curtain within 6 seconds; terminating helper safely".into());
            }
            thread::sleep(Duration::from_millis(30));
        }
    }
}

/// Preflight the installation and start the Cinepak video, returning only
/// after the helper reports that MCI has COMPLETELY CLOSED. The caller is
/// then free to spawn CYBER.EXE while the helper shows a black curtain.
pub fn start_handoff(root:&Path)->Result<IntroHandoff,String>{
    let game=root.join(DIR);
    settings::ensure(&game)?;
    if settings::read(&game)?{
        return Err("Isolated intro requires [Video] SkipIntro=0".into());
    }
    if std::env::var("PROTON_USE_WINED3D").as_deref()==Ok("1"){
        return Err("Cannot use the isolated D3D9/DXVK intro handoff with PROTON_USE_WINED3D=1. Remove this in Lutris and restart Wine/Proton before launching".into());
    }
    if settings::read_cdda(&game)?{cdda::try_install(root,&game)?;}
    verify::verify_launch_assets(root)?;
    let staged=bundle::TemporaryIntro::create()?;
    validate_x86_exe(staged.path())?;
    let log_path=diagnostics::location(root);
    let meta=fs::symlink_metadata(&log_path).map_err(|e|format!("Session log missing: {e}"))?;
    if !meta.is_file()||meta.file_type().is_symlink(){
        return Err("Refusing non-regular or symlink session log".into());
    }
    let output=OpenOptions::new().append(true).open(&log_path)
        .map_err(|e|format!("Cannot append intro diagnostics: {e}"))?;
    let error_output=output.try_clone().map_err(|e|e.to_string())?;
    let child=Command::new(staged.path())
        .arg("--handoff")
        .arg(staged.handoff_dir())
        .current_dir(&game)
        .stdin(Stdio::null())
        .stdout(Stdio::from(output))
        .stderr(Stdio::from(error_output))
        .spawn().map_err(|e|format!("Cannot start isolated intro player: {e}"))?;
    crate::show(&format!(
        "INTRO_HANDOFF started helper_pid={} mode=black_curtain log={}",
        child.id(),log_path.display()
    ),root);
    let mut handoff=IntroHandoff{
        staged,player:child,began:Instant::now(),released:false,
    };
    let ready=handoff.staged.handoff_dir().join("intro.ready");
    loop {
        if ready.exists(){
            // The player published this marker after close movie.
            let marker=fs::read_to_string(&ready)
                .map_err(|e|format!("Invalid intro completion marker: {e}"))?;
            if marker.trim()=="MCI_CLOSED"{
                crate::show(&format!(
                    "INTRO_HANDOFF VIDEO_FINISHED MCI_CLOSED=1 curtain_active=1 elapsed_ms={}",
                    handoff.began.elapsed().as_millis()
                ),root);
                return Ok(handoff);
            }
            if !marker.trim().is_empty() {
                return Err("Video helper published an invalid handoff marker".into());
            }
        }
        if let Some(status)=handoff.player.try_wait()
            .map_err(|e|format!("Cannot inspect video helper: {e}"))?{
            handoff.released=true; // child already exited; Drop only cleans files
            return Err(format!(
                "Isolated AVI helper exited before handoff ({status}); see {}/{}",
                root.display(),diagnostics::LOG_NAME
            ));
        }
        if handoff.began.elapsed()>Duration::from_secs(100){
            return Err("Video playback/handoff timed out after 100 seconds; game not launched".into());
        }
        thread::sleep(Duration::from_millis(30));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_helper_is_rejected_without_spawn(){
        let candidate=std::env::temp_dir().join(format!("missing-cyber-intro-{}.exe",std::process::id()));
        assert!(validate_x86_exe(&candidate).is_err());
    }
    #[test]
    fn non_pe_helper_is_rejected_without_spawn(){
        let candidate=std::env::temp_dir().join(format!("invalid-cyber-intro-{}.exe",std::process::id()));
        let _=fs::remove_file(&candidate);
        fs::write(&candidate,vec![0u8;4096]).unwrap();
        assert!(validate_x86_exe(&candidate).is_err());
        fs::remove_file(candidate).unwrap();
    }
}
