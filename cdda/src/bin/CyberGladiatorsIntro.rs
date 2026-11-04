// SPDX-License-Identifier: GPL-3.0-or-later
//! Process-isolated x86 MCI/Video-for-Windows intro player.
//! Present the AVI inside our own borderless screen-sized Win32 window.
//! The helper and every MCI/DirectDraw object exit BEFORE DXVK starts in
//! CYBER.EXE. No Wine virtual desktop, graphics mode, registry or game file
//! is created/changed by this helper.
#![cfg(windows)]
#![windows_subsystem = "windows"]

use std::ffi::{c_char,c_void,CString};
use std::{fs,ptr,thread};
use std::time::{Duration,Instant};

type Handle=*mut c_void;

#[repr(C)]
struct Point { x:i32,y:i32 }
#[repr(C)]
struct Message {
    hwnd:Handle,
    message:u32,
    wparam:usize,
    lparam:isize,
    time:u32,
    pt:Point,
    // Win32 MSG has an additional DWORD after POINT on 32-bit Windows.
    lprivate:u32,
}
#[repr(C)]
struct WindowClassA {
    style:u32,
    window_proc:Option<unsafe extern "system" fn(Handle,u32,usize,isize)->isize>,
    class_extra:i32,
    window_extra:i32,
    instance:Handle,
    icon:Handle,
    cursor:Handle,
    background:Handle,
    menu_name:*const c_char,
    class_name:*const c_char,
}
const CLASS_NAME:&[u8]=b"CyberGladiatorsIntroFullscreen\0";
const WINDOW_TITLE:&[u8]=b"Cyber Gladiators - Opening Video\0";
const WS_POPUP:u32=0x8000_0000;
const WS_CLIPCHILDREN:u32=0x0200_0000;
const WS_EX_TOPMOST:u32=0x0000_0008;
const SW_SHOW:i32=5;
const SWP_SHOWWINDOW:u32=0x0040;
const WM_DESTROY:u32=0x0002;
const WM_QUIT:u32=0x0012;
const BLACK_BRUSH:i32=4;

#[link(name="winmm")]
unsafe extern "system" {
    fn mciSendStringA(command:*const c_char,reply:*mut c_char,len:u32,callback:Handle)->u32;
    fn mciGetErrorStringA(code:u32,message:*mut c_char,len:u32)->i32;
}
#[link(name="kernel32")]
unsafe extern "system" {
    fn GetModuleHandleA(module:*const c_char)->Handle;
    fn GetLastError()->u32;
}
#[link(name="gdi32")]
unsafe extern "system" {
    fn GetStockObject(index:i32)->Handle;
}
#[link(name="user32")]
unsafe extern "system" {
    fn SetProcessDPIAware()->i32;
    fn GetSystemMetrics(index:i32)->i32;
    fn RegisterClassA(class:*const WindowClassA)->u16;
    fn CreateWindowExA(
        ex_style:u32,class:*const c_char,title:*const c_char,style:u32,
        x:i32,y:i32,w:i32,h:i32,parent:Handle,menu:Handle,
        instance:Handle,parameter:Handle
    )->Handle;
    fn DestroyWindow(window:Handle)->i32;
    fn DefWindowProcA(window:Handle,message:u32,wparam:usize,lparam:isize)->isize;
    fn PostQuitMessage(code:i32);
    fn ShowWindow(window:Handle,command:i32)->i32;
    fn UpdateWindow(window:Handle)->i32;
    fn SetWindowPos(window:Handle,insert_after:Handle,x:i32,y:i32,w:i32,h:i32,flags:u32)->i32;
    fn SetForegroundWindow(window:Handle)->i32;
    fn PeekMessageA(message:*mut Message,window:Handle,first:u32,last:u32,remove:u32)->i32;
    fn TranslateMessage(message:*const Message)->i32;
    fn DispatchMessageA(message:*const Message)->isize;
}
unsafe extern "system" fn window_proc(hwnd:Handle,msg:u32,wparam:usize,lparam:isize)->isize{
    if msg==WM_DESTROY {PostQuitMessage(0);return 0;}
    DefWindowProcA(hwnd,msg,wparam,lparam)
}
struct FullscreenHost { hwnd:Handle,w:i32,h:i32 }
impl FullscreenHost {
    fn create()->Result<Self,String>{
        // Prevent Windows display scaling from making a borderless window
        // smaller than the actual screen. Failure is not fatal on older Wine.
        unsafe{let _=SetProcessDPIAware();}
        let (w,h)=unsafe{(GetSystemMetrics(0),GetSystemMetrics(1))};
        if !(320..=16384).contains(&w)||!(240..=16384).contains(&h){
            return Err(format!("Cannot determine full-screen dimensions: {w}x{h}"));
        }
        let instance=unsafe{GetModuleHandleA(ptr::null())};
        if instance.is_null(){return Err("GetModuleHandleA failed".into());}
        let class=WindowClassA{
            style:0,
            window_proc:Some(window_proc),
            class_extra:0,window_extra:0,instance,
            icon:ptr::null_mut(),cursor:ptr::null_mut(),
            background:unsafe{GetStockObject(BLACK_BRUSH)},
            menu_name:ptr::null(),
            class_name:CLASS_NAME.as_ptr().cast(),
        };
        if unsafe{RegisterClassA(&class)}==0{
            return Err(format!("Cannot register intro window class: Win32 error {}",unsafe{GetLastError()}));
        }
        let hwnd=unsafe{
            CreateWindowExA(
                WS_EX_TOPMOST,CLASS_NAME.as_ptr().cast(),WINDOW_TITLE.as_ptr().cast(),
                WS_POPUP|WS_CLIPCHILDREN,
                0,0,w,h,ptr::null_mut(),ptr::null_mut(),instance,ptr::null_mut()
            )
        };
        if hwnd.is_null(){
            return Err(format!("Cannot create full-screen intro window: Win32 error {}",unsafe{GetLastError()}));
        }
        unsafe{
            // Topmost + popup uses ordinary Win32 windows, NOT Wine's
            // optional explorer.exe virtual desktop or a display-mode reset.
            let _=SetWindowPos(hwnd,(-1isize) as Handle,0,0,w,h,SWP_SHOWWINDOW);
            let _=ShowWindow(hwnd,SW_SHOW);
            let _=UpdateWindow(hwnd);
            let _=SetForegroundWindow(hwnd);
        }
        println!("INTRO_FULLSCREEN host_hwnd={} display={}x{} mode=borderless_popup source=primary_screen",hwnd as usize,w,h);
        Ok(Self{hwnd,w,h})
    }
}
impl Drop for FullscreenHost {
    fn drop(&mut self){
        if !self.hwnd.is_null(){unsafe{let _=DestroyWindow(self.hwnd);}}
    }
}
#[derive(Debug,PartialEq,Eq)]
struct VideoRect {x:i32,y:i32,w:i32,h:i32}
fn fit_aspect(screen_w:i32,screen_h:i32,video_w:i32,video_h:i32)->Result<VideoRect,String>{
    if [screen_w,screen_h,video_w,video_h].iter().any(|n|*n<=0){
        return Err("Invalid AVI source or display dimensions".into());
    }
    // Integer arithmetic preserves the 4:3 source aspect ratio, keeping
    // black pillarboxes rather than stretching 640x480 to 16:9.
    let (w,h)=if i64::from(screen_w)*i64::from(video_h)
        <= i64::from(screen_h)*i64::from(video_w){
        (screen_w,(i64::from(screen_w)*i64::from(video_h)/i64::from(video_w)) as i32)
    }else{
        ((i64::from(screen_h)*i64::from(video_w)/i64::from(video_h)) as i32,screen_h)
    };
    Ok(VideoRect{x:(screen_w-w)/2,y:(screen_h-h)/2,w,h})
}
fn parse_source_rect(s:&str)->Result<(i32,i32),String>{
    let parts=s.split_ascii_whitespace().collect::<Vec<_>>();
    if parts.len()!=4{return Err(format!("Unexpected MCI source rectangle {s:?}"));}
    let w=parts[2].parse::<i32>().map_err(|e|e.to_string())?;
    let h=parts[3].parse::<i32>().map_err(|e|e.to_string())?;
    if w<=0||h<=0{return Err(format!("Invalid MCI source dimensions {w}x{h}"));}
    Ok((w,h))
}
fn mci(command:&str)->Result<String,String>{
    let input=CString::new(command).map_err(|_|"NUL in MCI command".to_owned())?;
    let mut reply=[0u8;256];
    let result=unsafe{
        mciSendStringA(input.as_ptr(),reply.as_mut_ptr().cast(),reply.len() as u32,ptr::null_mut())
    };
    let end=reply.iter().position(|b|*b==0).unwrap_or(reply.len());
    let text=String::from_utf8_lossy(&reply[..end]).trim().to_owned();
    // MCI mode polling can be hundreds of lines; log only transitions in
    // the caller so the captured evidence is small and readable.
    if result!=0||!command.eq_ignore_ascii_case("status movie mode"){
        println!("INTRO_MCI command={command:?} result={result} reply={text:?}");
    }
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
        if unsafe{PeekMessageA(msg.as_mut_ptr(),ptr::null_mut(),0,0,1)}==0{return Ok(());}
        let msg=unsafe{msg.assume_init()};
        if msg.message==WM_QUIT{return Err("Video player received WM_QUIT".into());}
        unsafe{TranslateMessage(&msg);DispatchMessageA(&msg);}
    }
}
fn play()->Result<(),String>{
    let meta=fs::symlink_metadata("splash4.avi")
        .map_err(|e|format!("Cannot find splash4.avi in game directory: {e}"))?;
    if !meta.is_file()||meta.file_type().is_symlink(){
        return Err("splash4.avi must be an existing regular file".into());
    }
    let host=FullscreenHost::create()?;
    // No detached Wine/MCI popup. Create the MCI video as a child of the
    // full-screen host; give the same HWND to MCI as its drawing destination.
    // The AVI decoder remains in this short-lived process, never CYBER.EXE.
    // Do not open a second, unparented "avivideo" device. Unlike the
    // original game's two-device sequence, this isolated helper only needs
    // the file device explicitly parented to our fullscreen host. Opening
    // an unparented device may allocate an unwanted default video window.
    let run=(||->Result<(),String>{
        mci(&format!(
            "open splash4.avi type avivideo alias movie parent {} style child",host.hwnd as usize
        ))?;
        let playback=(||->Result<(),String>{
            mci(&format!("window movie handle {}",host.hwnd as usize))?;
            mci("window movie state show")?;
            let source=mci("where movie source max")?;
            let (video_w,video_h)=parse_source_rect(&source)?;
            let rect=fit_aspect(host.w,host.h,video_w,video_h)?;
            println!("INTRO_FULLSCREEN destination=x:{} y:{} w:{} h:{} source={}x{} aspect=preserved",rect.x,rect.y,rect.w,rect.h,video_w,video_h);
            mci(&format!("put movie destination at {} {} {} {}",rect.x,rect.y,rect.w,rect.h))?;
            // Read-only confirmation of the driver's actual destination.
            // Do not turn a missing MCI query into a playback failure.
            if let Ok(applied)=mci("where movie destination"){
                println!("INTRO_FULLSCREEN applied_destination={applied:?}");
            }
            mci("play movie window")?;
            let began=Instant::now();
            let mut last_mode=String::new();
            let mut seen_playing=false;
            loop {
                pump_messages()?;
                if began.elapsed()>Duration::from_secs(90){
                    return Err("External AVI playback exceeded 90 seconds".into());
                }
                let mode=mci("status movie mode")?;
                if mode!=last_mode{
                    println!("INTRO_VIDEO mode={mode:?} elapsed_ms={}",began.elapsed().as_millis());
                    last_mode=mode.clone();
                }
                if mode.eq_ignore_ascii_case("playing"){seen_playing=true;}
                if mode.eq_ignore_ascii_case("stopped"){
                    if !seen_playing&&began.elapsed()<Duration::from_millis(500){
                        return Err("AVI stopped immediately before playback was observed; check Wine Cinepak codec".into());
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
    // FullscreenHost is dropped here. Even if video failed, the helper
    // exits before CYBER.EXE starts. No persistent desktop/mode changes.
    run
}
fn main(){
    if std::env::args_os().len()!=1{
        eprintln!("CyberGladiatorsIntro: no arguments supported (launch from game's directory)");
        std::process::exit(2);
    }
    println!("INTRO_VIDEO isolated_x86_mci=1 fullscreen_host=1");
    match play(){
        Ok(())=>{},
        Err(e)=>{eprintln!("INTRO_VIDEO ERROR: {e}");std::process::exit(1);}
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fullscreen_destination_preserves_cinepak_aspect(){
        assert_eq!(fit_aspect(1920,1080,640,480).unwrap(),VideoRect{x:240,y:0,w:1440,h:1080});
        assert_eq!(fit_aspect(640,480,640,480).unwrap(),VideoRect{x:0,y:0,w:640,h:480});
        assert_eq!(fit_aspect(1280,1024,640,480).unwrap(),VideoRect{x:0,y:32,w:1280,h:960});
        assert_eq!(fit_aspect(1366,768,640,480).unwrap(),VideoRect{x:171,y:0,w:1024,h:768});
        assert!(fit_aspect(0,1080,640,480).is_err());
        assert!(fit_aspect(1920,1080,0,480).is_err());
    }
    #[test]
    fn mci_rect_parser_rejects_bad_values(){
        assert_eq!(parse_source_rect("0 0 640 480").unwrap(),(640,480));
        assert!(parse_source_rect("640x480").is_err());
        assert!(parse_source_rect("0 0 -1 480").is_err());
    }
    #[test]
    fn win32_message_layout_is_complete_on_x86(){
        assert_eq!(std::mem::size_of::<Message>(),32);
    }
}
