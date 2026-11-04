// SPDX-License-Identifier: GPL-3.0-or-later
//! Isolated x86 MCI/Video-for-Windows intro player for Cyber Gladiators.
//! Runs to completion *before* CYBER.EXE is started, so Wine's AVI/DirectDraw
//! objects cannot survive into nGlide's DXVK-backed Direct3D9 device.
//! Experimental opt-in; no game assets, runtime DLLs or registry are modified.
#![cfg(windows)]
use std::ffi::{c_char,c_void,CString};
use std::fs;
use std::time::{Duration,Instant};
use std::thread;

#[repr(C)]
struct Point { x:i32, y:i32 }
#[repr(C)]
struct Message {
    hwnd:*mut c_void,
    message:u32,
    wparam:usize,
    lparam:isize,
    time:u32,
    pt:Point,
}
#[link(name="winmm")]
unsafe extern "system" {
    fn mciSendStringA(command:*const c_char,reply:*mut c_char,len:u32,callback:*mut c_void)->u32;
    fn mciGetErrorStringA(code:u32,message:*mut c_char,len:u32)->i32;
}
#[link(name="user32")]
unsafe extern "system" {
    fn PeekMessageA(message:*mut Message,window:*mut c_void,first:u32,last:u32,remove:u32)->i32;
    fn TranslateMessage(message:*const Message)->i32;
    fn DispatchMessageA(message:*const Message)->isize;
}
fn mci(command:&str)->Result<String,String>{
    let input=CString::new(command).map_err(|_|"NUL in MCI command".to_owned())?;
    let mut reply=[0u8;256];
    let result=unsafe{
        mciSendStringA(input.as_ptr(),reply.as_mut_ptr().cast(),reply.len() as u32,std::ptr::null_mut())
    };
    let end=reply.iter().position(|b|*b==0).unwrap_or(reply.len());
    let text=String::from_utf8_lossy(&reply[..end]).trim().to_owned();
    println!("INTRO_MCI command={command:?} result={result} reply={text:?}");
    if result!=0 {
        let mut buffer=[0u8;256];
        let detail=unsafe{
            if mciGetErrorStringA(result,buffer.as_mut_ptr().cast(),buffer.len() as u32)==0{
                format!("MCI error {result}")
            }else{
                let end=buffer.iter().position(|b|*b==0).unwrap_or(buffer.len());
                String::from_utf8_lossy(&buffer[..end]).into_owned()
            }
        };
        return Err(format!("{command:?}: {detail} (code {result})"));
    }
    Ok(text)
}
fn pump_messages()->Result<(),String>{
    loop {
        let mut msg=std::mem::MaybeUninit::<Message>::uninit();
        if unsafe{PeekMessageA(msg.as_mut_ptr(),std::ptr::null_mut(),0,0,1)}==0 {return Ok(());}
        let msg=unsafe{msg.assume_init()};
        if msg.message==0x0012{return Err("Video player received WM_QUIT".into());}
        unsafe{TranslateMessage(&msg);DispatchMessageA(&msg);}
    }
}
fn play()->Result<(),String>{
    // The original CD asset was already validated by the x64 launcher.
    // The helper only reads it and uses the existing Wine/system MCI codec.
    let metadata=fs::symlink_metadata("splash4.avi")
        .map_err(|e|format!("Cannot find splash4.avi in game directory: {e}"))?;
    if !metadata.is_file()||metadata.file_type().is_symlink(){
        return Err("splash4.avi must be an existing regular file".into());
    }
    // Replicate the original game's MCIAVI sequence, but in a separate
    // process using the MCI driver's own popup window (no CYBER.EXE HWND).
    mci("open avivideo")?;
    let run=(||->Result<(),String>{
        mci("open splash4.avi type avivideo alias movie style popup")?;
        let playback=(||->Result<(),String>{
            mci("window movie state show")?;
            let rect=mci("where movie source max")?;
            println!("INTRO_VIDEO source={rect:?}");
            mci("put movie destination at 0 0 640 480")?;
            mci("play movie window")?;
            let began=Instant::now();
            let mut last_mode=String::new();
            let mut seen_playing=false;
            loop {
                pump_messages()?;
                if began.elapsed()>Duration::from_secs(90) {
                    return Err("External AVI playback exceeded 90 seconds".into());
                }
                let mode=mci("status movie mode")?;
                if mode!=last_mode {println!("INTRO_VIDEO mode={mode:?} elapsed_ms={}",began.elapsed().as_millis());last_mode=mode.clone();}
                if mode.eq_ignore_ascii_case("playing") {seen_playing=true;}
                if mode.eq_ignore_ascii_case("stopped") {
                    // A codec failure may report immediate 'stopped' even
                    // though open/play returned MCI success. Do not launch
                    // the game and claim the movie completed in that case.
                    if !seen_playing && began.elapsed()<Duration::from_millis(500){
                        return Err("AVI stopped immediately before playback was observed; check the Wine Cinepak codec".into());
                    }
                    println!("INTRO_VIDEO COMPLETE elapsed_ms={} observed_playing={seen_playing}",began.elapsed().as_millis());
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(100));
            }
        })();
        let _=mci("close movie");
        playback
    })();
    let _=mci("close avivideo");
    run
}
fn main(){
    if std::env::args_os().len()!=1{
        eprintln!("CyberGladiatorsIntro: no arguments supported (launch from game's directory)");
        std::process::exit(2);
    }
    println!("INTRO_VIDEO isolated_x86_mci=1");
    match play(){
        Ok(())=>{},
        Err(e)=>{eprintln!("INTRO_VIDEO ERROR: {e}");std::process::exit(1);}
    }
}
