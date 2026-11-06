// SPDX-License-Identifier: GPL-3.0-or-later
//! 32-bit, game-specific WinMM import proxy. Only MCI commands addressed to a
//! virtual CD-audio device are intercepted, controlled by the launcher's
//! CYBER_CDDA_MUSIC setting. When disabled, CDDA commands also forward to
//! native WinMM: the same DLL still handles native fullscreen MCIAVI and
//! the validated WineD3D/DXVK WndProc transition. This avoids a separate AVI
//! player and preserves the caller's music preference independently of video.
//! This DLL is loaded by an exact SHA-256-gated import-name patch to CYBER.EXE;
//! it is never installed as a system-wide winmm.dll.
#![cfg(windows)]
mod player;
mod video_trace;
mod glide_trace;
mod exception_trace;

use player::{Command as AudioCommand, Shared as AudioShared};
use std::ffi::{c_char,c_void,CStr};
use std::fs::{self,OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{mpsc,Arc,Mutex,OnceLock};
use std::sync::atomic::{AtomicU32,Ordering};

type Dword=u32;
type Ptr=usize;
type Handle=*mut c_void;
const MCI_OPEN:Dword=0x803;
const MCI_CLOSE:Dword=0x804;
const MCI_PLAY:Dword=0x806;
const MCI_SEEK:Dword=0x807;
const MCI_STOP:Dword=0x808;
const MCI_PAUSE:Dword=0x809;
const MCI_GETDEVCAPS:Dword=0x80b;
const MCI_SET:Dword=0x80d;
const MCI_STATUS:Dword=0x814;
const MCI_RESUME:Dword=0x855;
const MCI_OPEN_TYPE:Dword=0x2000;
const MCI_OPEN_TYPE_ID:Dword=0x1000;
const MCI_DEVTYPE_CD_AUDIO:usize=516;
const MCI_FROM:Dword=0x4;
const MCI_TO:Dword=0x8;
const MCI_SET_TIME_FORMAT:Dword=0x400;
const MCI_STATUS_ITEM:Dword=0x100;
const MCI_TRACK:Dword=0x10;
const MCI_FORMAT_TMSF:Dword=10;
const DEVICE_ID:Dword=0xcda0;
const MCI_STATUS_LENGTH:Dword=1;
const MCI_STATUS_POSITION:Dword=2;
const MCI_STATUS_NUMBER_OF_TRACKS:Dword=3;
const MCI_STATUS_MODE:Dword=4;
const MCI_STATUS_MEDIA_PRESENT:Dword=5;
const MCI_STATUS_TIME_FORMAT:Dword=6;
const MCI_STATUS_READY:Dword=7;
const MCI_STATUS_CURRENT_TRACK:Dword=8;
const MCI_CDA_STATUS_TYPE_TRACK:Dword=0x4001;
const MCI_CDA_TRACK_AUDIO:Dword=0;
const MCI_CDA_TRACK_OTHER:Dword=1;
const MCI_MODE_STOP:Dword=525;
const MCI_MODE_PLAY:Dword=526;
const MCI_MODE_PAUSE:Dword=529;
const MCIERR_UNSUPPORTED_FUNCTION:Dword=274;
const MCIERR_MISSING_PARAMETER:Dword=286;
const MCIERR_OUTOFRANGE:Dword=282;

#[repr(C)]
struct OpenParms {
    callback:Ptr,
    device_id:Dword,
    device_type:*const c_char,
    element:*const c_char,
    alias:*const c_char,
}
#[repr(C)]
struct PlayParms{callback:Ptr, from:Dword, to:Dword}
#[repr(C)]
struct SetParms{callback:Ptr,time_format:Dword,audio:Dword}
#[repr(C)]
struct StatusParms{callback:Ptr,returned:Ptr,item:Dword,track:Dword}
#[repr(C)]
struct SeekParms{callback:Ptr,to:Dword}

struct Controller {
    sender:mpsc::Sender<AudioCommand>,
    shared:Arc<AudioShared>,
    time_format:Dword,
}
static CONTROLLER:OnceLock<Mutex<Option<Controller>>>=OnceLock::new();
static TRACE_SESSION:OnceLock<()>=OnceLock::new();

fn controller()->&'static Mutex<Option<Controller>> {
    CONTROLLER.get_or_init(||Mutex::new(None))
}
/// Only the launcher's explicit "0" disables FLAC music; a direct legacy
/// CYBER.EXE invocation without the variable keeps existing CDDA behavior.
fn music_enabled_for(value:Option<&str>)->bool{value!=Some("0")}
fn music_enabled()->bool{
    music_enabled_for(std::env::var("CYBER_CDDA_MUSIC").ok().as_deref())
}
fn game_dir()->PathBuf {
    std::env::current_exe().ok().and_then(|p|p.parent().map(|x|x.to_path_buf()))
        .unwrap_or_else(||PathBuf::from("."))
}
fn log(message:&str){
    // Launcher overrides this path with its own adjacent session log. If
    // CYBER.EXE was started directly (without the launcher), still use the
    // conventional parent of the game folder; never create a CDDA-only log.
    let path=std::env::var_os("CYBER_LOG_FILE").map(PathBuf::from)
        .unwrap_or_else(||{
            let game=game_dir();
            game.parent().unwrap_or(&game).join("CyberGladiatorsFix.log")
        });
    if let Ok(mut f)=OpenOptions::new().append(true).create(true).open(path){
        let _=writeln!(f,"[CDDA] {message}");
    }
}
// WinMM exports can be called before the first video or CD track opens.
// Stamp the *actual* game's CDDA log once for each trace process, so that
// a previous run's cumulative lines cannot masquerade as new diagnostics.
// No I/O happens in DllMain or under the Windows loader lock.
fn mark_trace_session_once(){
    if std::env::var("CYBER_GLIDE_TRACE").as_deref()!=Ok("1"){return;}
    TRACE_SESSION.get_or_init(||{
        let pid=std::process::id();
        let exe=std::env::current_exe().ok().and_then(|p|p.file_name().map(|v|v.to_string_lossy().to_string()))
            .unwrap_or_else(||"unknown".into());
        let delay=std::env::var("CYBER_VIDEO_CLOSE_DELAY_MS")
            .unwrap_or_else(|_|"0".to_owned());
        let no_op=std::env::var("CYBER_GLIDE_PASSTHRU_NOOP")
            .unwrap_or_else(|_|"0".to_owned());
        log(&format!(
            "TRACE_SESSION BEGIN pid={pid} exe={exe} video_delay_ms={delay} passthru_noop={no_op} bridge_log=CyberGladiatorsFix.log"
        ));
    });
}
unsafe fn cd_type_matches(flags:Dword,data:Ptr)->bool {
    if flags&MCI_OPEN_TYPE==0||data==0{return false;}
    let typ=(*(data as *const OpenParms)).device_type as usize;
    if flags&MCI_OPEN_TYPE_ID!=0 {
        return (typ&0xffff)==MCI_DEVTYPE_CD_AUDIO;
    }
    if typ<0x10000{return false;}
    CStr::from_ptr(typ as *const c_char)
        .to_bytes().eq_ignore_ascii_case(b"cdaudio")
}
fn track_path(track:u8)->PathBuf {
    game_dir().join(format!("Track{track:02}.flac"))
}
fn available_count()->usize{
    (2u8..=15).filter(|&t|track_path(t).is_file()).count()
}
fn open_virtual(params:Ptr)->Dword {
    if params==0{return MCIERR_MISSING_PARAMETER;}
    if available_count()==0 {
        log("MCI_OPEN: no Track02.flac .. Track15.flac next to CYBER.EXE");
        return MCIERR_UNSUPPORTED_FUNCTION;
    }
    let mut ctl=controller().lock().unwrap_or_else(|e|e.into_inner());
    if ctl.is_none() {
        let (tx,rx)=mpsc::channel();
        let shared=Arc::new(AudioShared {
            mode:AtomicU32::new(MCI_MODE_STOP),
            track:AtomicU32::new(2),
            frames:AtomicU32::new(0),
        });
        let state=Arc::clone(&shared);
        let folder=game_dir();
        if std::thread::Builder::new()
            .name("CyberGladiators-CDDA".into())
            .spawn(move||player::run(rx,state,folder)).is_err(){
                log("MCI_OPEN: unable to start FLAC playback thread");
                return MCIERR_UNSUPPORTED_FUNCTION;
        }
        log(&format!("MCI_OPEN: virtual CD audio ready; available FLAC tracks={}",available_count()));
        *ctl=Some(Controller{sender:tx,shared,time_format:MCI_FORMAT_TMSF});
    }
    // MCI_OPEN_PARMS.device_id is the DWORD immediately after callback.
    unsafe {(*(params as *mut OpenParms)).device_id=DEVICE_ID;}
    0
}
fn tmsf_track(x:u32)->u8 {(x&0xff) as u8}
fn status_value(item:u32, track:u32, state:&Controller)->Result<u32,u32>{
    Ok(match item {
        MCI_STATUS_NUMBER_OF_TRACKS => 15,  // Track 01 is data; 02..15 are CDDA.
        MCI_STATUS_CURRENT_TRACK => state.shared.track.load(Ordering::Relaxed),
        MCI_STATUS_MODE => state.shared.mode.load(Ordering::Relaxed),
        MCI_STATUS_TIME_FORMAT => state.time_format,
        MCI_STATUS_MEDIA_PRESENT|MCI_STATUS_READY => 1,
        MCI_CDA_STATUS_TYPE_TRACK => if track==1 {MCI_CDA_TRACK_OTHER} else {MCI_CDA_TRACK_AUDIO},
        MCI_STATUS_POSITION => state.shared.track.load(Ordering::Relaxed) & 0xff,
        MCI_STATUS_LENGTH => 0, // The game uses MCI_FROM/TO, not STATUS length.
        _ => return Err(MCIERR_UNSUPPORTED_FUNCTION),
    })
}
fn virtual_command(message:u32,flags:u32,param:usize)->u32{
    let mut lock=controller().lock().unwrap_or_else(|e|e.into_inner());
    let Some(ref mut ctl)=*lock else{return MCIERR_UNSUPPORTED_FUNCTION;};
    match message{
        MCI_SET => {
            if flags&MCI_SET_TIME_FORMAT!=0{
                if param==0{return MCIERR_MISSING_PARAMETER;}
                let fmt=unsafe{(*(param as *const SetParms)).time_format};
                if fmt!=MCI_FORMAT_TMSF {return MCIERR_UNSUPPORTED_FUNCTION;}
                ctl.time_format=fmt;
            }
            0
        }
        MCI_PLAY=>{
            if flags&MCI_FROM!=0 {
                if param==0{return MCIERR_MISSING_PARAMETER;}
                let from=unsafe{(*(param as *const PlayParms)).from};
                let track=tmsf_track(from);
                if !(2..=15).contains(&track) {return MCIERR_OUTOFRANGE;}
                if !track_path(track).is_file() {
                    log(&format!("MCI_PLAY: missing Track{track:02}.flac"));
                    return MCIERR_UNSUPPORTED_FUNCTION;
                }
                // CYBER.EXE sends FROM TMSF(track,0,0,0) and
                // TO TMSF(track+1,0,0,0) using flags 0x0C.
                let stop=if flags&MCI_TO!=0 {
                    let to=unsafe{(*(param as *const PlayParms)).to};
                    Some(to)
                }else{None};
                log(&format!("MCI_PLAY: track={track:02} TO={stop:?} flags=0x{flags:X}"));
                ctl.shared.track.store(track as u32,Ordering::Relaxed);
                ctl.shared.frames.store(0,Ordering::Relaxed);
                ctl.shared.mode.store(MCI_MODE_PLAY,Ordering::Relaxed);
                if ctl.sender.send(AudioCommand::Play{track,to:stop}).is_err(){
                    return MCIERR_UNSUPPORTED_FUNCTION;
                }
            }else{
                log("MCI_PLAY: resume (without MCI_FROM)");
                ctl.shared.mode.store(MCI_MODE_PLAY,Ordering::Relaxed);
                if ctl.sender.send(AudioCommand::Resume).is_err(){
                    return MCIERR_UNSUPPORTED_FUNCTION;
                }
            }
            0
        }
        MCI_STOP=>{
            log("MCI_STOP");
            ctl.shared.mode.store(MCI_MODE_STOP,Ordering::Relaxed);
            let _=ctl.sender.send(AudioCommand::Stop);
            0
        }
        MCI_PAUSE=>{
            log("MCI_PAUSE");
            ctl.shared.mode.store(MCI_MODE_PAUSE,Ordering::Relaxed);
            let _=ctl.sender.send(AudioCommand::Pause);
            0
        }
        MCI_RESUME=>{
            ctl.shared.mode.store(MCI_MODE_PLAY,Ordering::Relaxed);
            let _=ctl.sender.send(AudioCommand::Resume);
            0
        }
        MCI_SEEK=>{
            if param==0{return MCIERR_MISSING_PARAMETER;}
            let to=unsafe{(*(param as *const SeekParms)).to};
            log(&format!("MCI_SEEK: to=0x{to:08X}"));
            let track=tmsf_track(to);
            if !(2..=15).contains(&track){return MCIERR_OUTOFRANGE;}
            ctl.shared.mode.store(MCI_MODE_STOP,Ordering::Relaxed);
            let _=ctl.sender.send(AudioCommand::Stop);
            ctl.shared.track.store(track as u32,Ordering::Relaxed);
            0
        }
        MCI_STATUS=>{
            if flags&MCI_STATUS_ITEM==0||param==0{return MCIERR_MISSING_PARAMETER;}
            let p=unsafe{&mut *(param as *mut StatusParms)};
            match status_value(p.item,p.track,ctl) {
                Ok(x)=>{p.returned=x as usize;0},
                Err(e)=>e,
            }
        }
        MCI_GETDEVCAPS=>0,
        MCI_CLOSE=>{
            log("MCI_CLOSE");
            let _=ctl.sender.send(AudioCommand::Shutdown);
            *lock=None;
            0
        }
        _=>{log(&format!("MCI: unsupported virtual-CD command 0x{message:X}"));MCIERR_UNSUPPORTED_FUNCTION}
    }
}

// Never import WinMM functions under the same names that this DLL exports.
// MSVC/LTO can resolve those references to *our own* exported functions,
// producing self-recursive wrappers (observed as EB FE infinite-loop stubs
// in v0.2.1). Resolve system WinMM explicitly from the Windows system
// directory and cache its procedures instead. This does not affect the
// separate waveOut imports in player.rs (none of those are exported here).
#[link(name="kernel32")]
extern "system" {
    fn GetSystemDirectoryW(buffer:*mut u16,size:u32)->u32;
    fn LoadLibraryW(name:*const u16)->*mut c_void;
    fn GetProcAddress(module:*mut c_void,name:*const c_char)->*mut c_void;
}
static SYSTEM_WINMM:OnceLock<Option<usize>>=OnceLock::new();
fn system_winmm()->Option<usize> {
    *SYSTEM_WINMM.get_or_init(||unsafe{
        // A local winmm.dll override is forbidden. Absolute system-directory
        // loading avoids resolving to CyberGladiatorsCDDA.dll or the game dir.
        let mut path=[0u16;1024];
        let n=GetSystemDirectoryW(path.as_mut_ptr(),path.len() as u32) as usize;
        if n==0 || n>=path.len()-12 {
            log("CDDA: GetSystemDirectoryW failed");
            return None;
        }
        let suffix:Vec<u16>="\\winmm.dll".encode_utf16().chain(std::iter::once(0)).collect();
        path[n..n+suffix.len()].copy_from_slice(&suffix);
        let module=LoadLibraryW(path.as_ptr());
        if module.is_null(){
            log("CDDA: could not load system WINMM.dll");
            None
        }else{Some(module as usize)}
    })
}
fn system_proc(name:&[u8])->Option<usize>{
    let module=system_winmm()?;
    let ptr=unsafe{GetProcAddress(module as *mut c_void,name.as_ptr() as *const c_char)};
    if ptr.is_null() {
        log(&format!("CDDA: WinMM lacks {}",String::from_utf8_lossy(name)));
        None
    }else{Some(ptr as usize)}
}
macro_rules! forward_winmm {
    ($function:ident($($arg:ident:$typ:ty),*) -> $return:ty, $failure:expr) => {
        #[no_mangle]
        #[allow(non_snake_case)]
        pub unsafe extern "system" fn $function($($arg:$typ),*)->$return {
            mark_trace_session_once();
            let Some(address)=system_proc(concat!(stringify!($function),"\0").as_bytes()) else {
                return $failure;
            };
            let function:unsafe extern "system" fn($($typ),*)->$return=
                std::mem::transmute(address);
            function($($arg),*)
        }
    };
}
// The 1996 game's 15 WinMM imports: forward 14 unchanged; intercept CD
// commands only in mciSendCommandA. The full x86 ABI is preserved.
/// Forward the original AVI MCI function unchanged. Read-only telemetry
/// determines whether the black screen occurs inside "status movie mode"
/// polling, MCI "close movie", or the subsequent renderer restore.
#[no_mangle]
pub unsafe extern "system" fn mciSendStringA(
    cmd:*const c_char,out:*mut c_char,len:u32,hwnd:Handle
)->u32 {
    let entry=std::time::Instant::now();
    mark_trace_session_once();
    let Some(address)=system_proc(b"mciSendStringA\0") else {
        return MCIERR_UNSUPPORTED_FUNCTION;
    };
    let native:unsafe extern "system" fn(*const c_char,*mut c_char,u32,Handle)->u32=
        std::mem::transmute(address);
    // Game MCI command strings are valid NUL-terminated ANSI strings.
    let command=if cmd.is_null(){None}else{
        CStr::from_ptr(cmd).to_str().ok().map(str::to_owned)
    };
    if let Some(ref text)=command {
        if text.trim().eq_ignore_ascii_case("open avivideo"){
            video_trace::timing("MCI_AVIVIDEO_OPEN_BEGIN");
            exception_trace::maybe_install();
            glide_trace::maybe_install();
        }
        if let Some(entry)=video_trace::before(text){log(&entry);}
    }
    // Preserve the original WinMM timeline. 4:3 is established by rewriting
    // only three recognized MCI window/rectangle commands. The opt-in smooth
    // experiment prepares a black host earlier and releases it later.
    // The temporary host must exist BEFORE the native WinMM "open avivideo"
    // can briefly paint its uninitialized gray window. This is strictly
    // opt-in, and falls back harmlessly if the verified game HWND is absent.
    if let Some(ref text)=command {video_trace::before_mci_open(text);}
    let substitute=command.as_deref().and_then(video_trace::rewrite_for_4_3);
    // Opt-in trial: hold the already-painted black AVI host for one second
    // before starting the original MCI movie. In the validated normal launch
    // this function is a no-op, and WinMM playback is forwarded immediately.
    if let Some(ref text)=command {video_trace::before_mci_play(text);}
    let mut result=native(substitute.as_ref().map_or(cmd,|v|v.as_ptr()),out,len,hwnd);
    if let Some(ref text)=command {
        video_trace::on_4_3_command_result(text,substitute.is_some(),result);
        if result!=0 && substitute.is_some() && video_trace::movie_open_game_hwnd_for_retry(text){
            // An unsupported child-style MCI driver must not prevent game
            // startup. Revert to the original AVI open and log the fallback.
            result=native(cmd,out,len,hwnd);
            log(&format!("AVI_4X3 MCI_OPEN_FALLBACK original_result={result}"));
        }
    }
    if let Some(ref text)=command{
        let reply=if result==0 && !out.is_null() && len>0 {
            // Bounds-check against the actual WinMM output buffer length.
            let bytes=std::slice::from_raw_parts(out as *const u8,(len as usize).min(256));
            let end=bytes.iter().position(|&b|b==0).unwrap_or(bytes.len());
            String::from_utf8_lossy(&bytes[..end]).into_owned()
        }else{String::new()};
        if let Some(entry)=video_trace::after(text,result,&reply){log(&entry);}
        if result==0 && text.trim().eq_ignore_ascii_case("close avivideo"){
            // The normal launch has already removed its video host.
            // Only --smooth-video-test may leave a no-activate black
            // curtain temporarily visible. The guarded WndProc repair stays
            // at the exact same MCI close boundary as the validated build.
            glide_trace::after_video_close();
            glide_trace::maybe_repair_windowproc_after_video_close();
            // Preserve the known-good guarded repair point, then hold the
            // already-painted black host for two real seconds while CYBER.EXE
            // is still inside native MCI close. Only after that hold returns
            // may the original game resume its Glide/D3D9 initialization.
            // Unlike post-close window timers, this cannot be visually
            // bypassed by the game raising its fullscreen renderer early.
            video_trace::hold_post_video_black_before_3d();
            glide_trace::maybe_delay_after_video_close();
        }
    }
    // One monotonic total-call duration for each non-polled video command.
    // The very frequent "status movie mode" queries are intentionally
    // excluded to keep the single session log small.
    if let Some(ref text)=command {
        let lower=text.trim().to_ascii_lowercase();
        if (lower.starts_with("open ")||lower.starts_with("play movie")
            ||lower.starts_with("close ")||lower.starts_with("window movie")
            ||lower.starts_with("put movie"))
            && (lower.contains("movie")||lower.contains("avivideo")
                ||lower.contains("splash4.avi")){
            log(&format!("VIDEO_TIMING mci_command={lower:?} total_call_ms={} result={result}",
                entry.elapsed().as_millis()));
        }
    }
    result
}
forward_winmm!(joyGetPosEx(id:u32,info:*mut c_void)->u32, 1);
forward_winmm!(joyGetDevCapsA(id:usize,caps:*mut c_void,size:u32)->u32, 1);
forward_winmm!(mmioSetInfo(h:Handle,info:*mut c_void,flags:u32)->u32, 1);
forward_winmm!(mmioSeek(h:Handle,offset:i32,origin:i32)->i32, -1);
forward_winmm!(mmioRead(h:Handle,out:*mut c_char,count:i32)->i32, -1);
forward_winmm!(mmioGetInfo(h:Handle,info:*mut c_void,flags:u32)->u32, 1);
forward_winmm!(mmioDescend(h:Handle,chunk:*mut c_void,parent:*const c_void,flags:u32)->u32, 1);
forward_winmm!(mmioCreateChunk(h:Handle,chunk:*mut c_void,flags:u32)->u32, 1);
forward_winmm!(mmioOpenA(name:*mut c_char,info:*mut c_void,flags:u32)->Handle, std::ptr::null_mut());
forward_winmm!(mmioAscend(h:Handle,chunk:*mut c_void,flags:u32)->u32, 1);
forward_winmm!(mmioAdvance(h:Handle,info:*mut c_void,flags:u32)->u32, 1);
forward_winmm!(mmioWrite(h:Handle,data:*const c_char,count:i32)->i32, -1);
forward_winmm!(mmioClose(h:Handle,flags:u32)->u32, 1);

#[no_mangle]
pub unsafe extern "system" fn mciSendCommandA(id:u32,message:u32,flags:usize,param:usize)->u32 {
    mark_trace_session_once();
    // Keep video services active even with music disabled: only the CD-audio
    // device uses the optional virtual FLAC implementation.
    if music_enabled() {
        if message==MCI_OPEN && cd_type_matches(flags as u32,param) {
            return open_virtual(param);
        }
        if id==DEVICE_ID {
            return virtual_command(message,flags as u32,param);
        }
    }
    let Some(address)=system_proc(b"mciSendCommandA\0") else {
        return MCIERR_UNSUPPORTED_FUNCTION;
    };
    let function:unsafe extern "system" fn(u32,u32,usize,usize)->u32=
        std::mem::transmute(address);
    function(id,message,flags,param)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_music_forwards_native_cd_audio_but_keeps_video_proxy(){
        assert!(!music_enabled_for(Some("0")));
        assert!(music_enabled_for(Some("1")));
        assert!(music_enabled_for(None),"direct legacy invocation remains compatible");
        assert!(music_enabled_for(Some("unknown")),"only an explicit zero disables FLAC");
    }
    #[test]
    fn mci_tmsf_track_is_low_byte(){
        assert_eq!(tmsf_track(0x0000_0002),2);
        assert_eq!(tmsf_track(0x0000_000f),15);
    }
    #[test]
    fn forwarded_mmio_seek_uses_real_system_winmm(){
        // On Windows, mmioSeek(NULL, ...) must return -1 immediately.
        // v0.2.1 compiled all 14 relays into JMP $ (EB FE), so this
        // regression test would hang in that broken build.
        assert!(system_winmm().is_some(),"WINMM.dll not available");
        let value=unsafe{mmioSeek(std::ptr::null_mut(),0,0)};
        assert_eq!(value,-1);
    }
    #[test]
    fn forwarded_mci_video_string_returns_instead_of_recursing(){
        // Missing MCI alias: real WinMM reports a nonzero error promptly.
        // Never intercept mciSendStringA, which CYBER.EXE uses for AVI.
        let value=unsafe{
            mciSendStringA(b"status unavailable_cyber_test_device mode\0".as_ptr()
                as *const c_char,std::ptr::null_mut(),0,std::ptr::null_mut())
        };
        assert_ne!(value,0);
    }
    #[test]
    fn devtype_cd_audio_matches_win32_id(){
        assert_eq!(MCI_DEVTYPE_CD_AUDIO,516);
        assert_eq!(MCI_OPEN_TYPE,0x2000);
        assert_eq!(MCI_FORMAT_TMSF,10);
        assert_eq!(MCI_FROM|MCI_TO,12);
    }
}
