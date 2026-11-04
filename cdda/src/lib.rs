// SPDX-License-Identifier: GPL-3.0-or-later
//! 32-bit, game-specific WinMM import proxy. Only MCI commands addressed to a
//! virtual CD-audio device are intercepted. All other functions, especially
//! MCI string commands used for AVI playback, are forwarded to real WinMM.
//! This DLL is loaded by an exact SHA-256-gated import-name patch to CYBER.EXE;
//! it is never installed as a system-wide winmm.dll.
#![cfg(windows)]
mod player;

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

fn controller()->&'static Mutex<Option<Controller>> {
    CONTROLLER.get_or_init(||Mutex::new(None))
}
fn game_dir()->PathBuf {
    std::env::current_exe().ok().and_then(|p|p.parent().map(|x|x.to_path_buf()))
        .unwrap_or_else(||PathBuf::from("."))
}
fn log(message:&str){
    let path=game_dir().join("CyberGladiatorsCDDA.log");
    if let Ok(mut f)=OpenOptions::new().append(true).create(true).open(path) {
        let _=writeln!(f,"{message}");
    }
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

// Both the DLL and WinMM are imported by the game separately: a 1996
// executable import descriptor is redirected from WINMM.dll to this DLL.
// Original WinMM functions are still loaded and called through real imports.
#[link(name="winmm")]
extern "system" {
    #[link_name="mciSendCommandA"] fn system_mci_send_command(id:u32,msg:u32,flags:usize,param:usize)->u32;
    #[link_name="mciSendStringA"] fn system_mci_send_string(command:*const c_char,out:*mut c_char,len:u32,hwnd:Handle)->u32;
    #[link_name="joyGetPosEx"] fn system_joy_get_pos_ex(id:u32,info:*mut c_void)->u32;
    #[link_name="joyGetDevCapsA"] fn system_joy_get_dev_caps(id:usize,caps:*mut c_void,size:u32)->u32;
    #[link_name="mmioSetInfo"] fn system_mmio_set_info(h:Handle,info:*mut c_void,flags:u32)->u32;
    #[link_name="mmioSeek"] fn system_mmio_seek(h:Handle,offset:i32,origin:i32)->i32;
    #[link_name="mmioRead"] fn system_mmio_read(h:Handle,out:*mut c_char,count:i32)->i32;
    #[link_name="mmioGetInfo"] fn system_mmio_get_info(h:Handle,info:*mut c_void,flags:u32)->u32;
    #[link_name="mmioDescend"] fn system_mmio_descend(h:Handle,chunk:*mut c_void,parent:*const c_void,flags:u32)->u32;
    #[link_name="mmioCreateChunk"] fn system_mmio_create_chunk(h:Handle,chunk:*mut c_void,flags:u32)->u32;
    #[link_name="mmioOpenA"] fn system_mmio_open(name:*mut c_char,info:*mut c_void,flags:u32)->Handle;
    #[link_name="mmioAscend"] fn system_mmio_ascend(h:Handle,chunk:*mut c_void,flags:u32)->u32;
    #[link_name="mmioAdvance"] fn system_mmio_advance(h:Handle,info:*mut c_void,flags:u32)->u32;
    #[link_name="mmioWrite"] fn system_mmio_write(h:Handle,data:*const c_char,count:i32)->i32;
    #[link_name="mmioClose"] fn system_mmio_close(h:Handle,flags:u32)->u32;
}

#[no_mangle]
pub unsafe extern "system" fn mciSendCommandA(id:u32,message:u32,flags:usize,param:usize)->u32 {
    // Never change any non-CD audio MCI device behavior.
    if message==MCI_OPEN && cd_type_matches(flags as u32,param){
        return open_virtual(param);
    }
    if id==DEVICE_ID {
        return virtual_command(message,flags as u32,param);
    }
    system_mci_send_command(id,message,flags,param)
}
#[no_mangle]
pub unsafe extern "system" fn mciSendStringA(cmd:*const c_char,out:*mut c_char,len:u32,hwnd:Handle)->u32{
    system_mci_send_string(cmd,out,len,hwnd)
}
#[no_mangle]
pub unsafe extern "system" fn joyGetPosEx(id:u32,info:*mut c_void)->u32{system_joy_get_pos_ex(id,info)}
#[no_mangle]
pub unsafe extern "system" fn joyGetDevCapsA(id:usize,caps:*mut c_void,size:u32)->u32{
    system_joy_get_dev_caps(id,caps,size)
}
#[no_mangle]
pub unsafe extern "system" fn mmioSetInfo(h:Handle,info:*mut c_void,flags:u32)->u32{system_mmio_set_info(h,info,flags)}
#[no_mangle]
pub unsafe extern "system" fn mmioSeek(h:Handle,offset:i32,origin:i32)->i32{system_mmio_seek(h,offset,origin)}
#[no_mangle]
pub unsafe extern "system" fn mmioRead(h:Handle,out:*mut c_char,count:i32)->i32{system_mmio_read(h,out,count)}
#[no_mangle]
pub unsafe extern "system" fn mmioGetInfo(h:Handle,info:*mut c_void,flags:u32)->u32{system_mmio_get_info(h,info,flags)}
#[no_mangle]
pub unsafe extern "system" fn mmioDescend(h:Handle,chunk:*mut c_void,parent:*const c_void,flags:u32)->u32{
    system_mmio_descend(h,chunk,parent,flags)
}
#[no_mangle]
pub unsafe extern "system" fn mmioCreateChunk(h:Handle,chunk:*mut c_void,flags:u32)->u32{
    system_mmio_create_chunk(h,chunk,flags)
}
#[no_mangle]
pub unsafe extern "system" fn mmioOpenA(name:*mut c_char,info:*mut c_void,flags:u32)->Handle{
    system_mmio_open(name,info,flags)
}
#[no_mangle]
pub unsafe extern "system" fn mmioAscend(h:Handle,chunk:*mut c_void,flags:u32)->u32{
    system_mmio_ascend(h,chunk,flags)
}
#[no_mangle]
pub unsafe extern "system" fn mmioAdvance(h:Handle,info:*mut c_void,flags:u32)->u32{
    system_mmio_advance(h,info,flags)
}
#[no_mangle]
pub unsafe extern "system" fn mmioWrite(h:Handle,data:*const c_char,count:i32)->i32{
    system_mmio_write(h,data,count)
}
#[no_mangle]
pub unsafe extern "system" fn mmioClose(h:Handle,flags:u32)->u32{system_mmio_close(h,flags)}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mci_tmsf_track_is_low_byte(){
        assert_eq!(tmsf_track(0x0000_0002),2);
        assert_eq!(tmsf_track(0x0000_000f),15);
    }
    #[test]
    fn devtype_cd_audio_matches_win32_id(){
        assert_eq!(MCI_DEVTYPE_CD_AUDIO,516);
        assert_eq!(MCI_OPEN_TYPE,0x2000);
        assert_eq!(MCI_FORMAT_TMSF,10);
        assert_eq!(MCI_FROM|MCI_TO,12);
    }
}
