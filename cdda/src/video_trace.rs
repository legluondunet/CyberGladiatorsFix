// SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only MCI-video instrumentation. The original CYBER.EXE polls
//! mciSendStringA("status movie mode") after "play movie window", then
//! calls "close movie" / "close avivideo" before restoring graphics.
//! Wine can hang either in the polling loop or during the display transition.
//! Log the exact boundary without altering any return value or MCI command.
use std::sync::{Mutex,OnceLock};
use std::time::{Duration,Instant};

#[derive(Default)]
struct MovieState {
    active:bool,
    polls:u64,
    last_mode:String,
    started:Option<Instant>,
}
static STATE:OnceLock<Mutex<MovieState>>=OnceLock::new();
fn state()->&'static Mutex<MovieState>{
    STATE.get_or_init(||Mutex::new(MovieState::default()))
}

fn command_is_video(command:&str)->bool{
    let trimmed=command.trim().to_ascii_lowercase();
    trimmed.split_ascii_whitespace().any(|word|word=="movie"||word=="avivideo")
}

pub fn before(command:&str)->Option<String>{
    if !command_is_video(command){return None;}
    let cmd=command.trim().to_ascii_lowercase();
    if cmd=="status movie mode"{return None;} // avoid 100s of lines/second.
    Some(format!("MCI_VIDEO BEGIN {command:?}"))
}
pub fn after(command:&str,code:u32,reply:&str)->Option<String>{
    if !command_is_video(command){return None;}
    let mut shared=state().lock().unwrap_or_else(|e|e.into_inner());
    after_in(&mut shared,command,code,reply)
}
fn after_in(state:&mut MovieState,command:&str,code:u32,reply:&str)->Option<String>{
    if !command_is_video(command){return None;}
    let cmd=command.trim().to_ascii_lowercase();
    if cmd=="play movie window" {
        state.active=code==0;
        state.started=if code==0{Some(Instant::now())}else{None};
        state.polls=0;
        state.last_mode.clear();
    }
    if cmd=="status movie mode"{
        state.polls=state.polls.saturating_add(1);
        let reply=reply.trim();
        let changed=state.last_mode!=reply;
        if changed{state.last_mode=reply.to_string();}
        // The 40-second SPLASH4.AVI is played at ~15 fps. If Wine never
        // stops reporting "playing" this 400-poll heartbeat reveals it.
        if !changed && state.polls%400!=0{return None;}
        let elapsed=state.started.map(|t|t.elapsed().as_secs()).unwrap_or(0);
        return Some(format!(
            "MCI_VIDEO STATUS polls={} elapsed={}s result={} mode={reply:?}",
            state.polls,elapsed,code));
    }
    let elapsed=state.started.map(|t|t.elapsed().as_secs()).unwrap_or(0);
    if cmd=="close movie"||cmd=="close avivideo" {
        state.active=false;
    }
    Some(format!("MCI_VIDEO END {command:?} result={code} reply={reply:?} elapsed={elapsed}s"))
}


// Native 4:3 AVI host and opt-in smooth handoff. These Win32 operations
// happen inside CYBER.EXE on the original MCI calling thread. The game's own AVI playback
// and the field-validated after-video WineD3D/DXVK WndProc repair are preserved.
// A fullscreen host is used only when the launcher verified the game,
// SkipIntro=0 and the in-process WinMM/CDDA bridge.
use std::ffi::{c_void,CString};
use std::sync::atomic::{AtomicBool,AtomicU32,Ordering};

const EXPECTED_GAME_BASE:usize=0x0040_0000;
// Per-retail-edition addresses validated from the original PE32 code.
// Fall back to US for old diagnostic aliases without CYBER_GAME_EDITION.
fn hwnd_rva_for(edition:Option<&str>)->usize{
    if edition==Some("FR"){0x0012_8118}else{0x0012_8018}
}
fn class_proc_rva_for(edition:Option<&str>)->usize{
    if edition==Some("FR"){0x0000_604e}else{0x0000_5f82}
}
fn game_hwnd_rva()->usize{
    hwnd_rva_for(std::env::var("CYBER_GAME_EDITION").ok().as_deref())
}
fn game_class_proc_rva()->usize{
    class_proc_rva_for(std::env::var("CYBER_GAME_EDITION").ok().as_deref())
}
const GCL_WNDPROC:i32=-24;
const GWL_WNDPROC:i32=-4;
const GWL_STYLE:i32=-16;
const SWP_NOZORDER:u32=0x0004;
const FROM_ADDRESS:u32=0x0000_0004;
const UNCHANGED_REFCOUNT:u32=0x0000_0002;
const INPUT_KEYBOARD:u32=1;
const KEYEVENTF_KEYUP:u32=2;
const VK_MENU:u16=0x12;
const WM_VULKAN_RECOVERY_DELAY_MS:u64=900;
const WM_VULKAN_RECOVERY_SETTLE_MS:u64=1000;
const MONITOR_DEFAULTTONEAREST:u32=2;
const WS_POPUP:u32=0x8000_0000;
const WS_CHILD:u32=0x4000_0000;
const WS_VISIBLE:u32=0x1000_0000;
const WS_CLIPCHILDREN:u32=0x0200_0000;
const SS_BLACKRECT:u32=0x0000_0004;
const WS_EX_TOOLWINDOW:u32=0x0000_0080;
const WS_EX_APPWINDOW:u32=0x0004_0000;
const SWP_NOACTIVATE:u32=0x0010;
const SWP_NOSIZE:u32=0x0001;
const SWP_NOMOVE:u32=0x0002;
const SWP_FRAMECHANGED:u32=0x0020;
const SWP_SHOWWINDOW:u32=0x0040;
const WS_EX_NOACTIVATE:u32=0x0800_0000;
const WS_EX_TOPMOST:u32=0x0000_0008;
const WM_TIMER:u32=0x0113;
const WM_GAMEFRAME_FIRST_GL_PRESENT:u32=0x8000+0x4c;
const GAMEFRAME_FIRST_GL_PROP:&[u8]=b"CyberGladiators_GameFrame_GlPresentedPending\0";
const GAMEFRAME_HANDOFF_TIMEOUT_MS:u64=3500;
static GAMEFRAME_FIRST_GL_ARMED:AtomicBool=AtomicBool::new(false);
static GAMEFRAME_HANDOFF_START:OnceLock<Mutex<Option<Instant>>>=OnceLock::new();
const SW_HIDE:i32=0;
const SW_SHOW:i32=5;
const VIDEO_LEAD_IN_MS:u64=1000; // only used by --smooth-video-test
// One deterministic 1-second hold runs in the MCI close call *before* the
// verified game can return to nGlide / resume its 3D display.
const INITIAL_FRAME_COVER_MS:u32=300; // trial: let Cinepak decode behind black
const TRANSITION_DELAY_MS:u64=1000; // only used by --smooth-video-test
// Never expose the desktop between the final black host and nGlide startup.
// Once the 1-second hold ends, let 3D initialize underneath the retained host.
// The original game's 3dfx-state DWORD comes from the existing trace module.
// This flag is not a first-present detector: demand a stable grace interval
// and impose a bounded timeout, rather than leaving an opaque popup forever.
const GAME_3DFX_ACTIVE_RVA:usize=0x0004_727c;
const HANDOFF_TIMER_ID:usize=0x435c;
const HANDOFF_POLL_MS:u32=50;
const HANDOFF_STABLE_3D_MS:u64=1500;
const HANDOFF_MAX_MS:u64=3500;
const WM_PAINT:u32=0x000f;
const WM_ERASEBKGND:u32=0x0014;
const BLACK_BRUSH:i32=4;
const WH_KEYBOARD:i32=2;
const WH_MOUSE:i32=7;
const WM_MOUSEMOVE:u32=0x0200;
const WM_LBUTTONDOWN:u32=0x0201;
const WM_LBUTTONUP:u32=0x0202;
const WM_LBUTTONDBLCLK:u32=0x0203;
const WM_RBUTTONDOWN:u32=0x0204;
const WM_RBUTTONUP:u32=0x0205;
const WM_RBUTTONDBLCLK:u32=0x0206;
const WM_MBUTTONDOWN:u32=0x0207;
const WM_MBUTTONUP:u32=0x0208;
const WM_MBUTTONDBLCLK:u32=0x0209;
const WM_MOUSEWHEEL:u32=0x020a;
const WM_XBUTTONDOWN:u32=0x020b;
const WM_XBUTTONUP:u32=0x020c;
const WM_XBUTTONDBLCLK:u32=0x020d;
const WM_MOUSEHWHEEL:u32=0x020e;
const VK_ESCAPE:usize=0x1b;
const VK_RETURN:usize=0x0d;
const VK_SPACE:usize=0x20;
const HOST_CLASS:&[u8]=b"CyberGladiatorsFix_AVI_4x3_Host_v2\0";
const VIRTUAL_VIDEO_CLASS:&[u8]=b"CyberGladiatorsFix_FR_MCI_VideoSurface_v1\0";
const VIRTUAL_MCI_TIMER_ID:usize=0x4d34;
const VIRTUAL_MCI_TIMER_MS:u32=50;
static HOST_CLASS_REGISTERED:OnceLock<bool>=OnceLock::new();
static VIRTUAL_VIDEO_CLASS_REGISTERED:OnceLock<bool>=OnceLock::new();
static VIRTUAL_MCI_SURFACE_PLAYING:AtomicBool=AtomicBool::new(false);
static VIRTUAL_MCI_SURFACE_IN_PAINT:AtomicBool=AtomicBool::new(false);
static VIRTUAL_MCI_SURFACE_PAINT_COUNT:AtomicU32=AtomicU32::new(0);
static VIRTUAL_MCI_SURFACE_TIMER_COUNT:AtomicU32=AtomicU32::new(0);
static KEY_FILTER_ACTIVE:AtomicBool=AtomicBool::new(false);
static VIRTUAL_AVI_PUMP_POLLS:AtomicU32=AtomicU32::new(0);
static VIRTUAL_AVI_PUMP_MESSAGES:AtomicU32=AtomicU32::new(0);
static VIRTUAL_AVI_MCI_REFRESH:OnceLock<Mutex<VirtualMciRefresh>>=OnceLock::new();
#[derive(Default)]
struct VirtualMciRefresh {
    last:Option<Instant>,
    requests:u32,
    failures:u32,
    disabled:bool,
}
fn virtual_mci_refresh()->&'static Mutex<VirtualMciRefresh>{
    VIRTUAL_AVI_MCI_REFRESH.get_or_init(||Mutex::new(VirtualMciRefresh::default()))
}
unsafe extern "C" {
    // The canonical Win32 MSG ABI is compiled in the project-owned
    // C++ bridge; this Rust module never guesses MSG structure alignment.
    fn cyber_virtual_avi_pump_messages()->u32;
}
static KEY_FILTER_BLOCKED:AtomicU32=AtomicU32::new(0);
static KEY_FILTER_ALLOWED:AtomicU32=AtomicU32::new(0);
static MOUSE_FILTER_BLOCKED:AtomicU32=AtomicU32::new(0);
#[derive(Default)]
struct Timing {
    origin:Option<Instant>,
    previous:Option<Instant>,
}
static TIMING:OnceLock<Mutex<Timing>>=OnceLock::new();
/// In-process monotonic milestone clock. This distinguishes the AVI close,
/// guarded WndProc repair and curtain release without assuming a 3D frame
/// was actually presented. Initial milestones are also logged during normal
/// launch, enabling exact A/B comparisons from a single session log.
pub fn timing(event:&str){
    if !video_enabled(){return;}
    let now=Instant::now();
    let (total,delta)={
        let mut t=TIMING.get_or_init(||Mutex::new(Timing::default()))
            .lock().unwrap_or_else(|e|e.into_inner());
        let origin=*t.origin.get_or_insert(now);
        let delta=t.previous.map(|prev|now.duration_since(prev).as_millis()).unwrap_or(0);
        t.previous=Some(now);
        (now.duration_since(origin).as_millis(),delta)
    };
    video_log(&format!("TIMING event={event} from_first_mci_ms={total} since_previous_ms={delta}"));
}

#[repr(C)]
#[derive(Default,Copy,Clone)]
struct WinRect {left:i32,top:i32,right:i32,bottom:i32}
#[repr(C)]
struct MonitorInfo {
    cb_size:u32,
    monitor:WinRect,
    work:WinRect,
    flags:u32,
}
#[repr(C)]
struct WndClassA {
    style:u32,
    wnd_proc:Option<unsafe extern "system" fn(*mut c_void,u32,usize,isize)->isize>,
    class_extra:i32,window_extra:i32,
    instance:*mut c_void,icon:*mut c_void,cursor:*mut c_void,
    background:*mut c_void,menu_name:*const u8,class_name:*const u8,
}
// Standard Win32 DEVMODEW; matching the launcher's ABI declaration.
// EnumDisplaySettingsW is a read-only query (never a mode request).
#[repr(C)]
struct DevModeW {
    name:[u16;32],
    spec_version:u16,driver_version:u16,size:u16,driver_extra:u16,
    fields:u32,
    position_x:i32,position_y:i32,orientation:u32,fixed_output:u32,
    color:u16,duplex:u16,y_resolution:u16,tt_option:u16,collate:u16,
    form_name:[u16;32],log_pixels:u16,
    bits_per_pel:u32,pels_width:u32,pels_height:u32,
    display_flags:u32,frequency:u32,
    icm_method:u32,icm_intent:u32,media_type:u32,dither_type:u32,
    reserved1:u32,reserved2:u32,panning_width:u32,panning_height:u32,
}
#[repr(C)]
struct PaintStruct {
    dc:*mut c_void,erase:i32,paint:WinRect,
    restore:i32,inc_update:i32,reserved:[u8;32],
}
// Win32 x86 INPUT: DWORD type followed by a 24-byte union. The binary
// shipped to CYBER.EXE is a verified x86 DLL; do not assume x64 layout.
#[repr(C)]
#[derive(Copy,Clone)]
struct WinInput { kind:u32,payload:[u32;6] }
fn keyboard_input(vkey:u16,release:bool)->WinInput {
    let mut payload=[0u32;6];
    payload[0]=u32::from(vkey);
    payload[1]=if release{KEYEVENTF_KEYUP}else{0};
    WinInput{kind:INPUT_KEYBOARD,payload}
}
#[link(name="kernel32")]
extern "system" {
    fn GetModuleHandleA(module:*const u8)->*mut c_void;
    fn GetCurrentProcessId()->u32;
    fn GetCurrentThreadId()->u32;
    fn GetLastError()->u32;
    fn GetModuleHandleExA(flags:u32,address:*const u8,module:*mut *mut c_void)->i32;
}
#[link(name="gdi32")]
extern "system" {
    fn GetStockObject(index:i32)->*mut c_void;
}
#[link(name="user32")]
extern "system" {
    fn DefWindowProcA(hwnd:*mut c_void,msg:u32,wparam:usize,lparam:isize)->isize;
    fn RegisterClassA(class:*const WndClassA)->u16;
    fn BeginPaint(hwnd:*mut c_void,paint:*mut PaintStruct)->*mut c_void;
    fn EndPaint(hwnd:*mut c_void,paint:*const PaintStruct)->i32;
    fn FillRect(dc:*mut c_void,rect:*const WinRect,brush:*mut c_void)->i32;
    fn InvalidateRect(hwnd:*mut c_void,rect:*const WinRect,erase:i32)->i32;
    fn UpdateWindow(hwnd:*mut c_void)->i32;
    fn SetWindowsHookExA(id_hook:i32,
        callback:Option<unsafe extern "system" fn(i32,usize,isize)->isize>,
        module:*mut c_void,thread_id:u32)->*mut c_void;
    fn CallNextHookEx(hook:*mut c_void,code:i32,wparam:usize,lparam:isize)->isize;
    fn UnhookWindowsHookEx(hook:*mut c_void)->i32;
    fn ShowWindow(hwnd:*mut c_void,mode:i32)->i32;
    fn IsWindow(hwnd:*mut c_void)->i32;
    fn SetWindowPos(hwnd:*mut c_void,insert_after:*mut c_void,
        x:i32,y:i32,width:i32,height:i32,flags:u32)->i32;
    fn GetWindowRect(hwnd:*mut c_void,rect:*mut WinRect)->i32;
    fn EnumWindows(callback:Option<unsafe extern "system" fn(*mut c_void,isize)->i32>,
        context:isize)->i32;
    fn IsWindowVisible(hwnd:*mut c_void)->i32;
    fn GetWindowThreadProcessId(hwnd:*mut c_void,pid:*mut u32)->u32;
    fn GetWindowLongA(hwnd:*mut c_void,index:i32)->i32;
    fn SetWindowLongA(hwnd:*mut c_void,index:i32,value:i32)->i32;
    fn GetForegroundWindow()->*mut c_void;
    fn GetSystemMetrics(index:i32)->i32;
    fn EnumDisplaySettingsW(device:*const u16,mode:i32,out:*mut DevModeW)->i32;
    fn IsIconic(hwnd:*mut c_void)->i32;
    fn SendInput(count:u32,inputs:*const WinInput,input_size:i32)->u32;
    fn GetClassLongA(hwnd:*mut c_void,index:i32)->u32;
    fn MonitorFromWindow(hwnd:*mut c_void,flags:u32)->*mut c_void;
    fn GetMonitorInfoA(monitor:*mut c_void,info:*mut MonitorInfo)->i32;
    fn CreateWindowExA(exstyle:u32,class_name:*const u8,title:*const u8,style:u32,
        x:i32,y:i32,width:i32,height:i32,parent:*mut c_void,
        menu:*mut c_void,instance:*mut c_void,param:*mut c_void)->*mut c_void;
    fn DestroyWindow(hwnd:*mut c_void)->i32;
    fn GetClientRect(hwnd:*mut c_void,rect:*mut WinRect)->i32;
    fn SetPropA(hwnd:*mut c_void,name:*const u8,data:*mut c_void)->i32;
    fn RemovePropA(hwnd:*mut c_void,name:*const u8)->*mut c_void;
    fn SetTimer(hwnd:*mut c_void,id:usize,timeout_ms:u32,callback:*const c_void)->usize;
    fn KillTimer(hwnd:*mut c_void,id:usize)->i32;
}
#[derive(Clone,Copy)]
struct NativeHost {
    host:usize,
    surface:usize,
    first_frame_cover:usize, // smooth test only; opaque sibling above video
    first_frame_cover_started:Option<Instant>,
    native_fr_last_active:Option<bool>,
    game:usize,
    owner_thread:u32,
    video_width:i32,
    video_height:i32,
    keyboard_hook:usize,
    mouse_hook:usize,
    holding_for_transition:bool,
    handoff_poll_armed:bool,
    handoff_started:Option<Instant>,
    handoff_fullscreen_since:Option<Instant>,
    handoff_polls:u32,
    curtain_raise_attempts:u32,
    curtain_raise_failures:u32,
}
static VIDEO_HOST:OnceLock<Mutex<Option<NativeHost>>>=OnceLock::new();
fn video_host()->&'static Mutex<Option<NativeHost>> {
    VIDEO_HOST.get_or_init(||Mutex::new(None))
}
fn video_enabled()->bool {
    std::env::var("CYBER_AVI_FULLSCREEN").as_deref()==Ok("1")
}
fn smooth_enabled()->bool{
    video_enabled() && std::env::var("CYBER_AVI_SMOOTH_TEST").as_deref()==Ok("1")
}
fn gameframe_first_present_handoff()->bool{
    smooth_enabled()&&std::env::var("CYBER_GAMEFRAME_TEST").as_deref()==Ok("1")
        &&std::env::var("CYBER_GAMEFRAME_AVI_FIRST_PRESENT").as_deref()==Ok("1")
}
fn video_log(message:&str){crate::log(&format!("AVI_4X3 {message}"));}

// A top-level STATIC SS_BLACKRECT window can leave its uncovered client area
// white in Wine. Give the 4:3 host its own black-background Win32 class and
// explicitly paint on both erase and paint messages. This is a *separate*
// temporary HWND; the game's original WndProc is never subclassed here.
unsafe extern "system" fn black_host_proc(
    hwnd:*mut c_void,msg:u32,wparam:usize,lparam:isize
)->isize{
    match msg{
        WM_TIMER if wparam==HANDOFF_TIMER_ID=>{
            if gameframe_first_present_handoff() {
                if GAMEFRAME_FIRST_GL_ARMED.load(Ordering::Acquire) {
                    let timed_out=GAMEFRAME_HANDOFF_START.get()
                        .and_then(|lock|lock.lock().ok().and_then(|start|*start))
                        .is_some_and(|at|at.elapsed()>=Duration::from_millis(GAMEFRAME_HANDOFF_TIMEOUT_MS));
                    if timed_out {
                        GAMEFRAME_FIRST_GL_ARMED.store(false,Ordering::Release);
                        video_log("GAMEFRAME_FIRST_GL_HANDOFF timed_out=1 reason=no_present_ack_or_win32_pump timeout_ms=3500");
                        close_host("gameframe_first_present_timeout");
                    }
                }
            }else{
                poll_3d_handoff();
            }
            0
        }
        WM_GAMEFRAME_FIRST_GL_PRESENT if gameframe_first_present_handoff()=>{
            if GAMEFRAME_FIRST_GL_ARMED.swap(false,Ordering::AcqRel){
                video_log("GAMEFRAME_FIRST_GL_HANDOFF notified=1 stage=after_real_OpenGL_SwapBuffers on_game_UI_thread=1 release_video_host=1");
                close_host("gameframe_first_GL_presented");
            }
            0
        }
        WM_ERASEBKGND=>{
            let mut rect=WinRect::default();
            if GetClientRect(hwnd,&mut rect)!=0 {
                let _=FillRect(wparam as *mut c_void,&rect,GetStockObject(BLACK_BRUSH));
            }
            1
        }
        WM_PAINT=>{
            let mut paint=PaintStruct{
                dc:std::ptr::null_mut(),erase:0,paint:WinRect::default(),
                restore:0,inc_update:0,reserved:[0;32]
            };
            let dc=BeginPaint(hwnd,&mut paint);
            if !dc.is_null(){
                let mut rect=WinRect::default();
                if GetClientRect(hwnd,&mut rect)!=0 {
                    let _=FillRect(dc,&rect,GetStockObject(BLACK_BRUSH));
                }
                EndPaint(hwnd,&paint);
            }
            0
        }
        _=>DefWindowProcA(hwnd,msg,wparam,lparam),
    }
}
// Unlike the separate black popup and first-frame curtain, a window
// supplied by the application via MCI_WINDOW is responsible for WM_PAINT.
// The previous experimental path reused black_host_proc for the actual AVI
// destination, overwriting MCI's frames in WM_PAINT. Windows documents that
// the application-owned video HWND must repaint invalid regions. Keep a
// dedicated WndProc only for verified Windows-FR DirectDraw virtualization;
// preserve the known-good original host/cover paths for US and Wine.
fn virtual_mci_surface_enabled()->bool{
    smooth_enabled()
        && std::env::var("CYBER_FR_VIRTUAL_DDRAW").as_deref()==Ok("1")
        && std::env::var("CYBER_GAME_EDITION").as_deref()==Ok("FR")
        && std::env::var("CYBER_GAMEFRAME_ALT_TAB_NATIVE_FIX").as_deref()==Ok("1")
}
unsafe extern "system" fn virtual_video_surface_proc(
    hwnd:*mut c_void,msg:u32,wparam:usize,lparam:isize
)->isize{
    match msg {
        WM_ERASEBKGND=>1, // never black-out an already decoded MCI frame
        WM_TIMER if wparam==VIRTUAL_MCI_TIMER_ID=>{
            if VIRTUAL_MCI_SURFACE_PLAYING.load(Ordering::Acquire){
                let tick=VIRTUAL_MCI_SURFACE_TIMER_COUNT.fetch_add(1,Ordering::Relaxed)+1;
                let queued=InvalidateRect(hwnd,std::ptr::null(),0);
                if tick==1 || tick%300==0{
                    video_log(&format!(
                        "FR_VIRTUAL_VIDEO_TIMER ticks={tick} invalidate={queued}                          interval_ms={VIRTUAL_MCI_TIMER_MS} repaint_target=video_surface_only"
                    ));
                }
            }
            0
        }
        WM_PAINT=>{
            let mut paint=PaintStruct{
                dc:std::ptr::null_mut(),erase:0,paint:WinRect::default(),
                restore:0,inc_update:0,reserved:[0;32]
            };
            let dc=BeginPaint(hwnd,&mut paint);
            if !dc.is_null(){
                if VIRTUAL_MCI_SURFACE_PLAYING.load(Ordering::Acquire) {
                    // Native WinMM, invoked in the owning UI thread with the
                    // *BeginPaint* HDC as the MCI_WINDOW documentation requires.
                    // No host mutex, no HWND reparent, no changing monitor mode.
                    if !VIRTUAL_MCI_SURFACE_IN_PAINT.swap(true,Ordering::AcqRel){
                        let code=crate::native_mci_paint_to_dc(dc);
                        let count=VIRTUAL_MCI_SURFACE_PAINT_COUNT
                            .fetch_add(1,Ordering::Relaxed)+1;
                        if count<=3 || count%300==0 || code!=Some(0){
                            video_log(&format!(
                                "FR_VIRTUAL_VIDEO_PAINT calls={count}                                  mci_update_result={code:?} owner_thread=1                                  beginpaint_hdc_valid=1 black_fill=0"
                            ));
                        }
                        VIRTUAL_MCI_SURFACE_IN_PAINT.store(false,Ordering::Release);
                    }
                }else{
                    // Before MCI play starts, retain the existing black
                    // initial surface without drawing over video frames.
                    let mut rect=WinRect::default();
                    if GetClientRect(hwnd,&mut rect)!=0{
                        let _=FillRect(dc,&rect,GetStockObject(BLACK_BRUSH));
                    }
                }
                EndPaint(hwnd,&paint);
            }
            0
        }
        _=>DefWindowProcA(hwnd,msg,wparam,lparam)
    }
}
unsafe fn register_virtual_video_class(instance:*mut c_void)->bool{
    *VIRTUAL_VIDEO_CLASS_REGISTERED.get_or_init(||{
        let class=WndClassA{
            style:0,wnd_proc:Some(virtual_video_surface_proc),
            class_extra:0,window_extra:0,instance,
            icon:std::ptr::null_mut(),cursor:std::ptr::null_mut(),
            background:std::ptr::null_mut(),
            menu_name:std::ptr::null(),class_name:VIRTUAL_VIDEO_CLASS.as_ptr()
        };
        let atom=RegisterClassA(&class);
        if atom==0{
            video_log(&format!("FR_VIRTUAL_VIDEO_CLASS_FAILED win32={}",GetLastError()));
            false
        }else{
            video_log("FR_VIRTUAL_VIDEO_CLASS registered=1 mci_destination_paints=video_frames                 parent_and_cover=black windowed=1 nonexclusive=1");
            true
        }
    })
}
unsafe fn stop_virtual_mci_video_surface(reason:&str){
    if !VIRTUAL_MCI_SURFACE_PLAYING.swap(false,Ordering::AcqRel){return;}
    let h=*video_host().lock().unwrap_or_else(|e|e.into_inner());
    let timer_stopped=if let Some(h)=h {
        if h.owner_thread==GetCurrentThreadId()
            && IsWindow(h.surface as *mut c_void)!=0{
            KillTimer(h.surface as *mut c_void,VIRTUAL_MCI_TIMER_ID)
        }else{0}
    }else{0};
    video_log(&format!(
        "FR_VIRTUAL_VIDEO_STOP reason={reason} timer_stopped={timer_stopped}          paint_calls={} timer_ticks={} physical_display_mode_unchanged=1",
        VIRTUAL_MCI_SURFACE_PAINT_COUNT.load(Ordering::Relaxed),
        VIRTUAL_MCI_SURFACE_TIMER_COUNT.load(Ordering::Relaxed)
    ));
}
unsafe fn register_black_host_class(instance:*mut c_void)->bool{
    *HOST_CLASS_REGISTERED.get_or_init(||{
        let class=WndClassA{
            style:0, wnd_proc:Some(black_host_proc),
            class_extra:0,window_extra:0,instance,
            icon:std::ptr::null_mut(),cursor:std::ptr::null_mut(),
            background:GetStockObject(BLACK_BRUSH),
            menu_name:std::ptr::null(),class_name:HOST_CLASS.as_ptr()
        };
        let atom=RegisterClassA(&class);
        if atom==0{
            video_log(&format!("HOST_CLASS_FAILED win32={}",GetLastError()));
            false
        }else{
            video_log("HOST_CLASS registered background=BLACK_BRUSH wm_paint=explicit");
            true
        }
    })
}
// Only the three explicitly approved skip keys reach the original game's
// keyboard message loop while the native AVI is playing. Mouse *button*
// messages (including wheel / double-click) are suppressed on that same
// thread; mouse movement and other applications are unaffected. A thread-local
// WH_KEYBOARD hook is intentionally limited to the game's UI thread: no
// global keyboard hooks, persistent changes, or DXVK WndProc modifications.
// GetAsyncKeyState/DirectInput bypass this API; validation under Wine is needed.
fn allowed_movie_skip_key(virtual_key:usize)->bool{
    matches!(virtual_key,VK_ESCAPE|VK_RETURN|VK_SPACE)
}
unsafe extern "system" fn movie_keyboard_filter(
    code:i32,key:usize,flags:isize
)->isize{
    if code>=0 && KEY_FILTER_ACTIVE.load(Ordering::Relaxed){
        if allowed_movie_skip_key(key){
            KEY_FILTER_ALLOWED.fetch_add(1,Ordering::Relaxed);
        }else{
            KEY_FILTER_BLOCKED.fetch_add(1,Ordering::Relaxed);
            return 1;
        }
    }
    CallNextHookEx(std::ptr::null_mut(),code,key,flags)
}
// A WH_MOUSE hook runs only for this game's thread and can suppress mouse
// buttons before they reach its Win32 message loop. If MCI creates a video
// window on a *different* thread, those messages are not intercepted; the
// diagnostic count records whether the Wine video uses this input path.
fn blocked_movie_mouse_message(message:u32)->bool{
    matches!(message,
        WM_LBUTTONDOWN|WM_LBUTTONUP|WM_LBUTTONDBLCLK|
        WM_RBUTTONDOWN|WM_RBUTTONUP|WM_RBUTTONDBLCLK|
        WM_MBUTTONDOWN|WM_MBUTTONUP|WM_MBUTTONDBLCLK|
        WM_XBUTTONDOWN|WM_XBUTTONUP|WM_XBUTTONDBLCLK|
        WM_MOUSEWHEEL|WM_MOUSEHWHEEL)
}
unsafe extern "system" fn movie_mouse_filter(
    code:i32,message:usize,info:isize
)->isize{
    if code>=0 && KEY_FILTER_ACTIVE.load(Ordering::Relaxed)
        && blocked_movie_mouse_message(message as u32){
        MOUSE_FILTER_BLOCKED.fetch_add(1,Ordering::Relaxed);
        return 1;
    }
    CallNextHookEx(std::ptr::null_mut(),code,message,info)
}
fn enable_movie_keys(){
    let mut guard=video_host().lock().unwrap_or_else(|e|e.into_inner());
    let Some(host)=guard.as_mut() else{return;};
    unsafe{
        if host.owner_thread!=GetCurrentThreadId(){
            video_log("KEY_FILTER_FAILED wrong_video_thread");
            return;
        }
        if host.keyboard_hook!=0{return;}
        KEY_FILTER_BLOCKED.store(0,Ordering::Relaxed);
        KEY_FILTER_ALLOWED.store(0,Ordering::Relaxed);
        MOUSE_FILTER_BLOCKED.store(0,Ordering::Relaxed);
        let hook=SetWindowsHookExA(WH_KEYBOARD,Some(movie_keyboard_filter),
            std::ptr::null_mut(),host.owner_thread);
        if hook.is_null(){
            video_log(&format!("KEY_FILTER_FAILED win32={}",GetLastError()));
            return;
        }
        host.keyboard_hook=hook as usize;
        // Install both filters before enabling them: mouse input must never
        // be filtered outside the AVI playback window. Leave the ordinary
        // keyboard policy intact if a Wine version rejects the mouse hook.
        let mouse=SetWindowsHookExA(WH_MOUSE,Some(movie_mouse_filter),
            std::ptr::null_mut(),host.owner_thread);
        if mouse.is_null(){
            video_log(&format!("MOUSE_FILTER_FAILED win32={} no_mouse_suppression",GetLastError()));
        }else{
            host.mouse_hook=mouse as usize;
        }
        KEY_FILTER_ACTIVE.store(true,Ordering::Release);
        video_log(&format!(
            "KEY_FILTER_ACTIVE allow=[ESC,ENTER,SPACE] other_game_thread_key_events=suppressed mouse_buttons_blocked_on_game_thread={}",
            u8::from(!mouse.is_null())
        ));
    }
}

fn fit_4_3(width:i32,height:i32)->Option<(i32,i32,i32,i32)>{
    if width<=0||height<=0||width>16384||height>16384{return None;}
    let (w,h)=if i64::from(width)*3 > i64::from(height)*4 {
        ((i64::from(height)*4/3) as i32,height)
    }else{
        (width,(i64::from(width)*3/4) as i32)
    };
    let w=w as i32;
    Some(((width-w)/2,(height-h)/2,w,h))
}
fn movie_open_game_hwnd(text:&str)->Option<usize>{
    // Restrict to the exact known sequence: never rewrite CD audio, other
    // AVI files, aliases, or a window handle unrelated to the verified game.
    let parts:Vec<&str>=text.split_ascii_whitespace().collect();
    if parts.len()!=8
        || !parts[..4].iter().zip(["open","splash4.avi","alias","movie"])
            .all(|(a,b)|a.eq_ignore_ascii_case(b))
        || !parts[4].eq_ignore_ascii_case("parent")
        || !parts[6].eq_ignore_ascii_case("style")
        || !parts[7].eq_ignore_ascii_case("popup") {return None;}
    parts[5].parse::<usize>().ok()
}
pub fn movie_open_game_hwnd_for_retry(text:&str)->bool{
    movie_open_game_hwnd(text).is_some()
}
fn exact_handle_command(text:&str)->Option<usize>{
    let parts:Vec<&str>=text.split_ascii_whitespace().collect();
    if parts.len()!=4 || !parts[..3].iter().zip(["window","movie","handle"])
        .all(|(a,b)|a.eq_ignore_ascii_case(b)){return None;}
    parts[3].parse::<usize>().ok()
}
fn destination_command(text:&str)->bool {
    let parts:Vec<&str>=text.split_ascii_whitespace().collect();
    parts.len()==8
        && parts[..5].iter().zip(["put","movie","destination","at","0"])
            .all(|(a,b)|a.eq_ignore_ascii_case(b))
        && parts[5..]==["0","640","480"]
}
// Wine's X11 bridge is more likely to publish _NET_WM_STATE_FULLSCREEN
// for an independent, managed application window than for an owned utility
// popup. Using the NORMAL window type (not a transient DIALOG) lets the
// Linux WM hide its panel for this monitor-filling temporary host.
// The validated normal launch keeps the original topmost tool-window style.
fn host_exstyle(smooth:bool)->u32 {
    WS_EX_NOACTIVATE|WS_EX_TOPMOST|
        if smooth{WS_EX_APPWINDOW}else{WS_EX_TOOLWINDOW}
}
fn video_surface_style(smooth:bool)->u32 {
    // The ordinary validated launch retains STATIC/SS_BLACKRECT. The opt-in
    // path paints with a genuinely black stock brush instead of Wine's
    // theme-dependent COLOR_3DDKSHADOW (which can look gray).
    WS_CHILD|if smooth{0}else{SS_BLACKRECT|WS_VISIBLE}
}
fn video_surface_class_for(smooth:bool,virtual_fr:bool)->&'static [u8]{
    if smooth && virtual_fr{VIRTUAL_VIDEO_CLASS}
    else if smooth{HOST_CLASS}else{b"STATIC\0"}
}
fn video_surface_class(smooth:bool)->&'static [u8]{
    video_surface_class_for(smooth,virtual_mci_surface_enabled())
}
fn first_frame_cover_style()->u32{
    // Our custom black-painted child must not use SS_BLACKRECT: under Wine,
    // that STATIC style is implemented with GetSysColor(COLOR_3DDKSHADOW).
    WS_CHILD|WS_VISIBLE
}
unsafe fn prepare_host(original_window:usize)->Option<NativeHost>{
    let base=GetModuleHandleA(std::ptr::null()) as usize;
    if base!=EXPECTED_GAME_BASE {
        video_log(&format!("SKIP unsupported_image_base={base:#x}"));
        return None;
    }
    let game=std::ptr::read_volatile((base+game_hwnd_rva()) as *const usize);
    let hwnd=game as *mut c_void;
    let mut pid=0u32;
    if original_window!=game || IsWindow(hwnd)==0
        || GetWindowThreadProcessId(hwnd,&mut pid)!=GetCurrentThreadId()
        || pid!=GetCurrentProcessId()
        || GetClassLongA(hwnd,GCL_WNDPROC) as usize!=base+game_class_proc_rva() {
        video_log("SKIP invalid_game_window_or_class_or_thread");
        return None;
    }
    let monitor=MonitorFromWindow(hwnd,MONITOR_DEFAULTTONEAREST);
    let mut info=MonitorInfo{
        cb_size:std::mem::size_of::<MonitorInfo>() as u32,
        monitor:WinRect::default(),work:WinRect::default(),flags:0
    };
    if monitor.is_null() || GetMonitorInfoA(monitor,&mut info)==0 {
        video_log(&format!("SKIP GetMonitorInfoA_failed={}",GetLastError()));
        return None;
    }
    // Wine-staging may report the game's 640x480 mode even though its
    // compositor can present the prelaunch desktop geometry. In contrast,
    // native Windows FR reports an actual 640x480 display mode while the AVI
    // starts: constructing a 1536x960 window there clips most of the movie.
    // Respect the *current* monitor on native FR, while keeping the previous
    // prelaunch-desktop override on Wine/Proton and other tested paths.
    // Neither path requests or intercepts a display-mode change.
    if gameframe_first_present_handoff() {
        let reported=info.monitor;
        let raw=std::env::var("CYBER_GAMEFRAME_DESKTOP_SIZE").ok();
        let captured=gameframe_prelaunch_desktop(raw.as_deref());
        // The launcher enables this flag only for verified native-Windows FR.
        // It is never set on Wine/Proton or for the US edition.
        let native_fr=std::env::var("CYBER_GAMEFRAME_ALT_TAB_NATIVE_FIX").as_deref()==Ok("1")
            && std::env::var("CYBER_GAME_EDITION").as_deref()==Ok("FR");
        let (selected,used)=choose_gameframe_avi_monitor(reported,captured,true,native_fr);
        info.monitor=selected;
        video_log(&format!(
            "GAMEFRAME_AVI_DESKTOP_GEOMETRY transient_monitor=({},{} {}x{}) prelaunch_desktop={} selected_host=({},{} {}x{}) used_prelaunch={} native_fr_current_mode={} change_display_settings=0",
            reported.left,reported.top,
            reported.right-reported.left,reported.bottom-reported.top,
            raw.as_deref().unwrap_or("missing"),
            selected.left,selected.top,
            selected.right-selected.left,selected.bottom-selected.top,
            u8::from(used),u8::from(native_fr)
        ));
    }
    let screen_w=info.monitor.right.checked_sub(info.monitor.left)?;
    let screen_h=info.monitor.bottom.checked_sub(info.monitor.top)?;
    let (x,y,w,h)=fit_4_3(screen_w,screen_h)?;
    // Keep the custom class separate from the game's known subclass chain.
    // The explicitly black-painted background covers both pillarboxes.
    if !register_black_host_class(base as *mut c_void){return None;}
    let smooth=smooth_enabled();
    let virtual_video=smooth&&virtual_mci_surface_enabled();
    if virtual_video && !register_virtual_video_class(base as *mut c_void){
        video_log("FR_VIRTUAL_VIDEO_SETUP_FAILED reason=class_registration");
        return None;
    }
    // The smooth experiment requests an independent managed NORMAL fullscreen
    // popup (rather than an owned tool-window/dialog). This prevents the
    // Linux desktop panel from staying above the video on WMs which honor
    // EWMH fullscreen. WS_EX_NOACTIVATE preserves keyboard focus on the game.
    let owner=if smooth{std::ptr::null_mut()}else{hwnd};
    let host=CreateWindowExA(host_exstyle(smooth),
        HOST_CLASS.as_ptr(),b"\0".as_ptr(),
        WS_POPUP|WS_VISIBLE|WS_CLIPCHILDREN,
        info.monitor.left,info.monitor.top,screen_w,screen_h,owner,
        std::ptr::null_mut(),base as *mut c_void,std::ptr::null_mut());
    if host.is_null() {
        video_log(&format!("SKIP fullscreen_host_create_failed={}",GetLastError()));
        return None;
    }
    // In smooth mode use our already-registered custom window class to paint
    // the MCI parent solid RGB(0,0,0), independent of Wine's system theme.
    // Keep it hidden from the very first CreateWindowExA invocation.
    let surface=CreateWindowExA(0,video_surface_class(smooth).as_ptr(),b"\0".as_ptr(),
        video_surface_style(smooth)|if virtual_video{WS_CLIPCHILDREN}else{0},
        x,y,w,h,host,
        std::ptr::null_mut(),std::ptr::null_mut(),std::ptr::null_mut());
    if surface.is_null(){
        let err=GetLastError();
        DestroyWindow(host);
        video_log(&format!("SKIP video_surface_create_failed={err}"));
        return None;
    }
    let mut client=WinRect::default();
    if GetClientRect(surface,&mut client)==0
        || client.right!=w || client.bottom!=h {
        DestroyWindow(host);
        video_log("SKIP unexpected_video_surface_client_size");
        return None;
    }
    // Cover the MCI surface with the SAME explicit-black painted class.
    // The previous STATIC/SS_BLACKRECT cover used COLOR_3DDKSHADOW under
    // Wine and therefore appeared as a gray rectangle during BOTH pauses.
    // The ordinary launch never creates this overlay.
    let first_frame_cover=if smooth {
        let overlay=CreateWindowExA(0,HOST_CLASS.as_ptr(),b"\0".as_ptr(),
            first_frame_cover_style(),
            x,y,w,h,host,
            std::ptr::null_mut(),std::ptr::null_mut(),std::ptr::null_mut());
        if overlay.is_null(){
            video_log(&format!(
                "FIRST_FRAME_COVER_FAILED win32={} fallback=immediate_video_reveal",
                GetLastError()
            ));
        } else {
            video_log(&format!(
                "FIRST_FRAME_COVER_CREATED hwnd={:#x} pos=({x},{y}) size={w}x{h} paint=explicit_RGB000 stock_brush=BLACK_BRUSH theme_dependent=0",
                overlay as usize
            ));
        }
        overlay as usize
    }else{0};
    // The EWMH fullscreen hint is applied by Wine to managed full-monitor
    // windows. Reaffirm the exact monitor geometry and TOPMOST z-order only
    // in the opt-in mode. No ChangeDisplaySettings / refresh switch occurs.
    if smooth {
        let positioned=SetWindowPos(host,(-1isize) as *mut c_void,
            info.monitor.left,info.monitor.top,screen_w,screen_h,
            SWP_NOACTIVATE|SWP_FRAMECHANGED|SWP_SHOWWINDOW);
        let mut rect=WinRect::default();
        let geometry_ok=GetWindowRect(host,&mut rect);
        video_log(&format!(
            "WM_FULLSCREEN_REQUEST managed_app_window=1 independent_owner=1 no_activate=1 setpos={positioned} rect_valid={geometry_ok} rect=({},{} {}x{}) monitor=({},{} {}x{}) source_surface_initially_hidden=1",
            rect.left,rect.top,rect.right-rect.left,rect.bottom-rect.top,
            info.monitor.left,info.monitor.top,screen_w,screen_h
        ));
    }
    // Force a fresh black paint before Wine's MCI window becomes visible.
    let _=InvalidateRect(host,std::ptr::null(),1);
    let _=UpdateWindow(host);
    video_log(&format!(
        "HOST_CREATED game={game:#x} host={:#x} surface={:#x} monitor=({},{} {}x{}) video_4_3=({x},{y} {w}x{h}) fullscreen_black=painted no_activate=1 smooth_managed={}",
        host as usize,surface as usize,
        info.monitor.left,info.monitor.top,screen_w,screen_h,u8::from(smooth)));
    Some(NativeHost{
        host:host as usize,surface:surface as usize,
        first_frame_cover,first_frame_cover_started:None,native_fr_last_active:None,
        game,owner_thread:GetCurrentThreadId(),
        video_width:w,video_height:h,
        keyboard_hook:0,mouse_hook:0,
        holding_for_transition:false,
        handoff_poll_armed:false,handoff_started:None,
        handoff_fullscreen_since:None,handoff_polls:0,
        curtain_raise_attempts:0,
        curtain_raise_failures:0
    })
}
// The game must regain its keyboard and mouse immediately after MCI closes,
// even if the opt-in curtain stays in front of the 3D startup for a short
// period. Never continue filtering while 3D input is active.
unsafe fn stop_movie_input(h:&mut NativeHost){
    KEY_FILTER_ACTIVE.store(false,Ordering::Release);
    if h.keyboard_hook!=0{
        let unhooked=UnhookWindowsHookEx(h.keyboard_hook as *mut c_void);
        video_log(&format!(
            "KEY_FILTER_STOPPED unhooked={unhooked} allowed_events={} blocked_events={}",
            KEY_FILTER_ALLOWED.load(Ordering::Relaxed),
            KEY_FILTER_BLOCKED.load(Ordering::Relaxed)
        ));
        h.keyboard_hook=0;
    }
    if h.mouse_hook!=0{
        let unhooked=UnhookWindowsHookEx(h.mouse_hook as *mut c_void);
        video_log(&format!(
            "MOUSE_FILTER_STOPPED unhooked={unhooked} blocked_events={}",
            MOUSE_FILTER_BLOCKED.load(Ordering::Relaxed)
        ));
        h.mouse_hook=0;
    }
}
/// Only the explicit Vulkan+early-release+auto-fullscreen experiment can
/// attempt the exact nGlide keyboard mode toggle observed to work manually.
fn vulkan_fullscreen_recovery_opted_in()->bool {
    smooth_enabled()
        && std::env::var("CYBER_VULKAN_EARLY_HOST_RELEASE").as_deref()==Ok("1")
        && std::env::var("CYBER_VULKAN_AUTO_FULLSCREEN").as_deref()==Ok("1")
}
fn game_window_for_vulkan_recovery(hwnd:*mut c_void)->Result<(WinRect,WinRect),&'static str>{
    unsafe {
        if IsWindow(hwnd)==0||IsWindowVisible(hwnd)==0||IsIconic(hwnd)!=0 {
            return Err("game_window_not_visible_or_minimized");
        }
        let mut pid=0u32;
        if GetWindowThreadProcessId(hwnd,&mut pid)==0||pid!=GetCurrentProcessId(){
            return Err("unexpected_game_window_owner");
        }
        if GetForegroundWindow()!=hwnd{return Err("game_not_foreground");}
        let glide=GetModuleHandleA(b"glide.dll\0".as_ptr());
        if glide.is_null(){return Err("nglide_not_loaded");}
        let wndproc=GetWindowLongA(hwnd,GWL_WNDPROC) as u32 as usize;
        let mut owner=std::ptr::null_mut();
        if wndproc<0x10000
            || GetModuleHandleExA(FROM_ADDRESS|UNCHANGED_REFCOUNT,
                wndproc as *const u8,&mut owner)==0
            || owner!=glide {
            return Err("game_wndproc_not_nglide");
        }
        let monitor=MonitorFromWindow(hwnd,MONITOR_DEFAULTTONEAREST);
        if monitor.is_null(){return Err("no_monitor");}
        let mut info=MonitorInfo{
            cb_size:std::mem::size_of::<MonitorInfo>() as u32,
            monitor:WinRect::default(),work:WinRect::default(),flags:0,
        };
        let mut rect=WinRect::default();
        if GetMonitorInfoA(monitor,&mut info)==0||GetWindowRect(hwnd,&mut rect)==0 {
            return Err("monitor_or_window_rect_unavailable");
        }
        Ok((rect,info.monitor))
    }
}
fn send_nglide_alt_enter(hwnd:*mut c_void,attempt:u32)->bool {
    let Ok((before,monitor))=game_window_for_vulkan_recovery(hwnd) else{
        video_log(&format!("VULKAN_AUTO_FULLSCREEN attempt={attempt} SKIP reason=window_or_focus_changed"));
        return false;
    };
    if covers_monitor(before,monitor){
        video_log(&format!("VULKAN_AUTO_FULLSCREEN attempt={attempt} SKIP reason=already_fullscreen"));
        return false;
    }
    // The same Alt+Enter hotkey documented by nGlide, not forced window
    // resizing. SendInput sets Win32 keyboard state, unlike PostMessage.
    // Never inject input when another application owns the foreground.
    let inputs=[
        keyboard_input(VK_MENU,false),keyboard_input(VK_RETURN as u16,false),
        keyboard_input(VK_RETURN as u16,true),keyboard_input(VK_MENU,true),
    ];
    let sent=unsafe{SendInput(
        inputs.len() as u32,inputs.as_ptr(),std::mem::size_of::<WinInput>() as i32
    )};
    video_log(&format!(
        "VULKAN_AUTO_FULLSCREEN attempt={attempt} sent={sent}/4 before=({},{})-({},{}) monitor=({},{})-({},{}) input=Alt+Enter foreground_verified=1",
        before.left,before.top,before.right,before.bottom,
        monitor.left,monitor.top,monitor.right,monitor.bottom
    ));
    sent==4
}
fn start_vulkan_fullscreen_recovery(){
    if !vulkan_fullscreen_recovery_opted_in(){return;}
    // This is the verified original CYBER.EXE and its original window pointer.
    // Never change the game's WndProc, patch nGlide or touch other processes.
    if unsafe{GetModuleHandleA(std::ptr::null())} as usize!=EXPECTED_GAME_BASE {
        video_log("VULKAN_AUTO_FULLSCREEN SKIP unexpected_exe_base");
        return;
    }
    let hwnd=unsafe{std::ptr::read_volatile(
        (EXPECTED_GAME_BASE+game_hwnd_rva()) as *const usize)} as *mut c_void;
    let hwnd_value=hwnd as usize;
    video_log("VULKAN_AUTO_FULLSCREEN scheduled=1 initial_wait_ms=900 max_toggles=2 monitor_checked=1 foreground_guard=1 independent_game_thread=1");
    std::thread::spawn(move||{
        std::thread::sleep(Duration::from_millis(WM_VULKAN_RECOVERY_DELAY_MS));
        let hwnd=hwnd_value as *mut c_void;
        match game_window_for_vulkan_recovery(hwnd){
            Ok((rect,screen)) if covers_monitor(rect,screen)=>{
                video_log("VULKAN_AUTO_FULLSCREEN SKIP already_fullscreen_after_900ms");
                return;
            },
            Err(reason)=>{
                video_log(&format!("VULKAN_AUTO_FULLSCREEN SKIP reason={reason}"));
                return;
            },
            Ok(_)=>{},
        };
        for attempt in 1..=2 {
            if !send_nglide_alt_enter(hwnd,attempt){break;}
            std::thread::sleep(Duration::from_millis(WM_VULKAN_RECOVERY_SETTLE_MS));
            match game_window_for_vulkan_recovery(hwnd){
                Ok((rect,screen))=>{
                    let fullscreen=covers_monitor(rect,screen);
                    video_log(&format!(
                        "VULKAN_AUTO_FULLSCREEN RESULT attempt={attempt} rect=({},{})-({},{}) fullscreen={} next={}",
                        rect.left,rect.top,rect.right,rect.bottom,
                        u8::from(fullscreen),if fullscreen{"stop"}else if attempt<2{"second_toggle"}else{"stop_limit"}
                    ));
                    if fullscreen{break;}
                },
                Err(reason)=>{
                    video_log(&format!("VULKAN_AUTO_FULLSCREEN RESULT attempt={attempt} SKIP reason={reason}"));
                    break;
                },
            }
        }
    });
}
/// GameFrame's real game HWND can be 648x513 again while WinMM plays
/// the intro, even though grSstOpen initially made it fullscreen. Waiting
/// for the 30-frame swap poll exposes this small window after the video.
/// Correct it BEFORE destroying the still-visible AVI host, on the actual
/// game UI thread. The existing GameFrame per-frame checker remains the
/// bounded fallback if the game resizes again afterwards.
// The prelaunch snapshot is authoritative on the primary monitor for
// Wine/Proton GameFrame: those environments may report a temporary 640x480
// mode while still presenting to the original desktop. Native Windows FR
// genuinely uses the reported 640x480 mode at MCI open, and sizing beyond it
// clips the video until an Alt+Tab restores the original resolution.
fn choose_gameframe_avi_monitor(
    monitor:WinRect,prelaunch:Option<(i32,i32)>,gameframe:bool,native_fr:bool
)->(WinRect,bool){
    if gameframe && !native_fr && monitor.left==0 && monitor.top==0 {
        if let Some((w,h))=prelaunch {
            return (WinRect{left:0,top:0,right:w,bottom:h},true);
        }
    }
    (monitor,false)
}
fn gameframe_prelaunch_desktop(raw:Option<&str>)->Option<(i32,i32)>{
    let (w,h)=raw?.split_once('x')?;
    let (w,h)=(w.parse::<i32>().ok()?,h.parse::<i32>().ok()?);
    ((640..=16384).contains(&w)&&(480..=16384).contains(&h)).then_some((w,h))
}
fn gameframe_popup_style(style:u32)->u32{
    // Remove only window caption and sizable frame. Preserve game-specific
    // bits, input ownership, clipping and WndProc.
    (style & !0x00c4_0000u32)|WS_POPUP|WS_VISIBLE
}
fn should_defer_native_fr_handoff_resize(
    phase:&str,first_present:bool,fr_native:bool,width:i32,height:i32
)->bool{
    phase=="after_MCI_close_before_first_gl_present" && first_present
        && fr_native && width==640 && height==480
}
fn gameframe_prepare_window(phase:&str){
    if std::env::var("CYBER_GAMEFRAME_TEST").as_deref()!=Ok("1"){return;}
    // Native FR DirectDraw still owns the physical 640x480x16 mode at
    // "close avivideo". Expanding the *game* HWND to the prelaunch logical
    // desktop here was premature: the game resizes it back to 656x519
    // before the first real GL SwapBuffers, so GameFrame immediately
    // expands it AGAIN. Keep the black AVI host in place, and let the
    // already validated GameFrame first-present guard perform exactly one
    // final fullscreen adjustment immediately before that GL swap.
    //
    // This is solely a WINDOW sizing change, not a DirectDraw mode hook.
    // Wine/Proton and US keep their previously validated early handoff.
    let native_fr=std::env::var("CYBER_GAMEFRAME_ALT_TAB_NATIVE_FIX").as_deref()==Ok("1")
        && std::env::var("CYBER_GAME_EDITION").as_deref()==Ok("FR");
    if should_defer_native_fr_handoff_resize(
        phase,gameframe_first_present_handoff(),native_fr,
        unsafe{GetSystemMetrics(0)},unsafe{GetSystemMetrics(1)}
    ) {
        video_log("GAMEFRAME_HANDOFF_PREPARE stage=after_MCI_close_before_first_gl_present deferred_to_first_GL_swap=1 reason=native_FR_current_640x480 skip_redundant_SetWindowPos=1 display_mode_change=0");
        return;
    }
    let raw=std::env::var("CYBER_GAMEFRAME_DESKTOP_SIZE").ok();
    let Some((desktop_w,desktop_h))=gameframe_prelaunch_desktop(raw.as_deref()) else{
        video_log("GAMEFRAME_HANDOFF_PREPARE SKIP invalid_prelaunch_desktop_size");
        return;
    };
    unsafe {
        let base=GetModuleHandleA(std::ptr::null()) as usize;
        if base!=EXPECTED_GAME_BASE{
            video_log("GAMEFRAME_HANDOFF_PREPARE SKIP unexpected_exe_base");
            return;
        }
        let hwnd=std::ptr::read_volatile((base+game_hwnd_rva()) as *const u32)
            as usize as *mut c_void;
        let mut pid=0u32;
        let tid=if !hwnd.is_null() && IsWindow(hwnd)!=0{
            GetWindowThreadProcessId(hwnd,&mut pid)
        }else{0};
        if tid==0||tid!=GetCurrentThreadId()||pid!=GetCurrentProcessId(){
            video_log("GAMEFRAME_HANDOFF_PREPARE SKIP invalid_hwnd_or_owner_thread");
            return;
        }
        if GetClassLongA(hwnd,GCL_WNDPROC) as usize!=base+game_class_proc_rva(){
            video_log("GAMEFRAME_HANDOFF_PREPARE SKIP unexpected_game_class");
            return;
        }
        let monitor=MonitorFromWindow(hwnd,MONITOR_DEFAULTTONEAREST);
        let mut info=MonitorInfo{
            cb_size:std::mem::size_of::<MonitorInfo>() as u32,
            monitor:WinRect::default(),work:WinRect::default(),flags:0
        };
        if monitor.is_null()||GetMonitorInfoA(monitor,&mut info)==0
            ||info.monitor.left!=0||info.monitor.top!=0{
            // Prelaunch desktop dimensions describe the primary screen only.
            video_log("GAMEFRAME_HANDOFF_PREPARE SKIP unsupported_non_primary_monitor");
            return;
        }
        let mut before=WinRect::default();
        if GetWindowRect(hwnd,&mut before)==0{
            video_log("GAMEFRAME_HANDOFF_PREPARE SKIP GetWindowRect_failed");
            return;
        }
        let style=GetWindowLongA(hwnd,GWL_STYLE) as u32;
        let desired=gameframe_popup_style(style);
        let already_fullscreen=before.left==0&&before.top==0
            &&before.right==desktop_w&&before.bottom==desktop_h;
        if already_fullscreen {
            video_log(&format!("GAMEFRAME_HANDOFF_PREPARE stage={phase} fullscreen_already_correct=1 changed=0"));
            return;
        }
        // The host is still present and topmost. This changes only the
        // original game's window style and geometry, WITHOUT activating it,
        // altering Z order or changing the display mode. Do not keep or
        // reposition a second permanent rendering/window proxy.
        let changed_style=style!=desired;
        if changed_style{
            let _=SetWindowLongA(hwnd,GWL_STYLE,desired as i32);
        }
        let flags=SWP_NOZORDER|SWP_NOACTIVATE|
            if changed_style{SWP_FRAMECHANGED}else{0};
        let ok=SetWindowPos(hwnd,std::ptr::null_mut(),0,0,desktop_w,desktop_h,flags);
        let mut after=WinRect::default();
        let readback=GetWindowRect(hwnd,&mut after);
        video_log(&format!(
            "GAMEFRAME_HANDOFF_PREPARE stage={phase} hwnd={:#x} old_rect=({},{},{},{}) new_rect=({},{},{},{}) desktop={}x{} old_style={style:#010x} new_style={desired:#010x} SetWindowPos={ok} GetWindowRect={readback} display_mode_change=0 no_activate=1",
            hwnd as usize,before.left,before.top,before.right,before.bottom,
            after.left,after.top,after.right,after.bottom,desktop_w,desktop_h
        ));
    }
}

fn close_host(reason:&str){
    let previous=video_host().lock().unwrap_or_else(|e|e.into_inner()).take();
    if let Some(mut h)=previous{
        GAMEFRAME_FIRST_GL_ARMED.store(false,Ordering::Release);
        // close_host already removed the host from the registry; disable
        // the timer directly with the saved HWND. Never let it repaint an
        // MCI device that has already been closed.
        VIRTUAL_MCI_SURFACE_PLAYING.store(false,Ordering::Release);
        unsafe{
            if GetCurrentThreadId()==h.owner_thread
                && IsWindow(h.surface as *mut c_void)!=0{
                let _=KillTimer(h.surface as *mut c_void,VIRTUAL_MCI_TIMER_ID);
            }
        }
        unsafe{
            KEY_FILTER_ACTIVE.store(false,Ordering::Release);
            if GetCurrentThreadId()!=h.owner_thread{
                video_log(&format!("WARNING host_not_destroyed_other_thread reason={reason}"));
                return;
            }
            stop_movie_input(&mut h);
            if gameframe_first_present_handoff(){
                let _=RemovePropA(h.host as *mut c_void,GAMEFRAME_FIRST_GL_PROP.as_ptr());
                if let Some(lock)=GAMEFRAME_HANDOFF_START.get(){
                    *lock.lock().unwrap_or_else(|e|e.into_inner())=None;
                }
            }
            if h.handoff_poll_armed {
                let _=KillTimer(h.host as *mut c_void,HANDOFF_TIMER_ID);
                h.handoff_poll_armed=false;
            }
            if h.holding_for_transition {
                video_log(&format!(
                    "CURTAIN_TOPMOST_SUMMARY attempts={} failures={} mode=synchronous_then_3d_ready_handoff polls={}",
                    h.curtain_raise_attempts,h.curtain_raise_failures,h.handoff_polls
                ));
            }
            let destroyed=DestroyWindow(h.host as *mut c_void);
            video_log(&format!("HOST_CLOSED reason={reason} destroy_result={destroyed}"));
            timing("HOST_DESTROYED");
        }
    }
}
/// The exit curtain is prepared in the original MCI close thread and kept
/// visible with a synchronous hold AFTER the guarded WndProc repair. This
/// prevents the game from reaching the next nGlide/3D phase until the hold
/// ends, rather than repeatedly raising a competing fullscreen window.
fn should_pump_virtual_avi(status_ok:bool,is_status:bool,virtual_fr:bool,keys_active:bool)->bool{
    status_ok&&is_status&&virtual_fr&&keys_active
}
/// The legacy game busy-polls WinMM throughout its roughly 40-second AVI
/// without processing ordinary window messages. This can make a genuine
/// NON-exclusive borderless HWND appear hung to Windows even while the
/// movie continues playing. The experiment pumps up to 16 queued messages
/// on the same UI thread at each MCI status poll, instead of synthesizing
/// a second window loop. The C++ adapter preserves WM_QUIT and guards
/// against nested pumps, without editing the real display mode.
pub fn pump_virtual_avi_messages(command:&str,result:u32){
    let virtual_fr=std::env::var("CYBER_FR_VIRTUAL_DDRAW").as_deref()==Ok("1")
        &&std::env::var("CYBER_GAME_EDITION").as_deref()==Ok("FR")
        &&gameframe_first_present_handoff();
    if !should_pump_virtual_avi(
        result==0,command.trim().eq_ignore_ascii_case("status movie mode"),
        virtual_fr,KEY_FILTER_ACTIVE.load(Ordering::Acquire)
    ){return;}
    let owner={
        let guard=video_host().lock().unwrap_or_else(|e|e.into_inner());
        guard.as_ref().map(|h|h.owner_thread)
    };
    if owner!=Some(unsafe{GetCurrentThreadId()}){return;}
    let count=unsafe{cyber_virtual_avi_pump_messages()};
    let polls=VIRTUAL_AVI_PUMP_POLLS.fetch_add(1,Ordering::Relaxed)+1;
    VIRTUAL_AVI_PUMP_MESSAGES.fetch_add(count,Ordering::Relaxed);
    if polls==1 || polls%400==0 {
        video_log(&format!(
            "FR_VIRTUAL_AVI_MESSAGE_PUMP polls={polls} dispatched_total={} last_batch={count}             max_per_status=16 normal_mode=disabled win32_msg_queue_serviced=1",
            VIRTUAL_AVI_PUMP_MESSAGES.load(Ordering::Relaxed)
        ));
    }
}
/// MCIAVI is still playing after a frozen frame, and Alt+Tab temporarily
/// makes the image advance. Window-message pumping alone did not fix the
/// renderer. In this *experimental FR non-exclusive mode only*, periodically
/// ask the **native MCI driver** to repaint the current frame to a valid DC.
/// Do not invalidate our black-painted host: painting the parent would erase
/// the video rather than refreshing it.
///
/// The returned HWND is owned by the same MCI/game UI thread. The WinMM
/// caller must release the DC, perform the native MCI update without holding
/// any host mutex, and report the return code. An unsupported update is
/// disabled after the first failure to preserve ordinary playback.
fn mci_refresh_is_due(now:Instant,last:&mut Option<Instant>,interval:Duration)->bool{
    match *last {
        Some(previous) if now.saturating_duration_since(previous)<interval=>false,
        _=>{
            *last=Some(now);
            true
        }
    }
}
pub fn virtual_fr_movie_repaint_target(command:&str,code:u32,reply:&str)->Option<usize>{
    if code!=0 || !command.trim().eq_ignore_ascii_case("status movie mode")
        || !reply.trim().eq_ignore_ascii_case("playing")
        || std::env::var("CYBER_FR_VIRTUAL_DDRAW").as_deref()!=Ok("1")
        || std::env::var("CYBER_GAME_EDITION").as_deref()!=Ok("FR")
        || !gameframe_first_present_handoff()
        || !KEY_FILTER_ACTIVE.load(Ordering::Acquire){
        return None;
    }
    let host=(*video_host().lock().unwrap_or_else(|e|e.into_inner()))?;
    unsafe {
        if host.owner_thread!=GetCurrentThreadId()
            || IsWindow(host.surface as *mut c_void)==0
            || IsWindowVisible(host.surface as *mut c_void)==0 {
            return None;
        }
    }
    let mut guard=virtual_mci_refresh().lock().unwrap_or_else(|e|e.into_inner());
    if guard.disabled{return None;}
    // Avoid any repaint while the original AVI pre-roll cover is active.
    // Initial call starts the clock without issuing MCI commands.
    let now=Instant::now();
    if guard.last.is_none(){
        guard.last=Some(now);
        return None;
    }
    if !mci_refresh_is_due(now,&mut guard.last,Duration::from_millis(500)){
        return None;
    }
    guard.requests=guard.requests.saturating_add(1);
    Some(host.surface)
}
pub fn virtual_fr_movie_repaint_result(code:u32,has_dc:bool){
    let mut guard=virtual_mci_refresh().lock().unwrap_or_else(|e|e.into_inner());
    if !has_dc||code!=0 {
        guard.failures=guard.failures.saturating_add(1);
        guard.disabled=true; // fail closed; do not repeat unsupported updates
    }
    if guard.requests<=3 || guard.requests%20==0 || guard.disabled {
        video_log(&format!(
            "FR_VIRTUAL_MCI_FRAME_REFRESH requests={} result={code} hdc_acquired={}              disabled={} failures={} interval_ms=500 paint_method=native_MCI_UPDATE              physical_resolution_unchanged=1",
            guard.requests,u8::from(has_dc),u8::from(guard.disabled),guard.failures
        ));
    }
}
// The original game polls mciSendStringA("status movie mode") many times
// per second while its UI thread is NOT pumping ordinary Win32 WM_TIMER
// messages. Unlike the after-video curtain, a Win32 timer cannot reliably
// uncover the first decoded image during playback. Use the verified MCI
// status-poll callback instead, on the same owning game UI thread.
// This is a bounded time-based heuristic, NOT real first-frame detection.
fn first_frame_cover_is_due(start:Instant,now:Instant)->bool{
    now.saturating_duration_since(start)>=Duration::from_millis(u64::from(INITIAL_FRAME_COVER_MS))
}
fn maybe_release_first_frame_cover(command:&str){
    if !smooth_enabled()||!command.eq_ignore_ascii_case("status movie mode"){return;}
    let due={
        let mut guard=video_host().lock().unwrap_or_else(|e|e.into_inner());
        let Some(h)=guard.as_mut() else{return;};
        if h.owner_thread!=unsafe{GetCurrentThreadId()}{
            return;
        }
        let Some(start)=h.first_frame_cover_started else{return;};
        let now=Instant::now();
        if !first_frame_cover_is_due(start,now){return;}
        h.first_frame_cover_started=None;
        Some((h.first_frame_cover,now.saturating_duration_since(start).as_millis()))
    };
    if let Some((cover,elapsed))=due{
        unsafe{
            if cover!=0 && IsWindow(cover as *mut c_void)!=0{
                let hidden=ShowWindow(cover as *mut c_void,SW_HIDE);
                video_log(&format!(
                    "FIRST_FRAME_COVER_RELEASED reason=mci_status_poll elapsed_ms={elapsed} previously_visible={hidden}"
                ));
                timing("FIRST_FRAME_COVER_RELEASED");
            }
        }
    }
}
fn should_arm_curtain(smooth:bool,mci_close_result:u32)->bool{
    smooth && mci_close_result==0
}
// Game/Glide can promote its 3D window above the no-activate video host
// when entering the menu. A 2000 ms timer only proves the host exists;
// it does NOT prove that it is visibly above the game during those seconds.
// Reassert a *temporary* topmost position while the curtain is active.
// No SetForegroundWindow, focus stealing, D3D9 hook or WndProc subclassing.
unsafe fn promote_curtain(h:&mut NativeHost)->i32 {
    if IsWindow(h.host as *mut c_void)==0{return 0;}
    let promoted=SetWindowPos(h.host as *mut c_void,(-1isize) as *mut c_void,
        0,0,0,0,
        SWP_NOACTIVATE|SWP_NOMOVE|SWP_NOSIZE|SWP_SHOWWINDOW);
    h.curtain_raise_attempts=h.curtain_raise_attempts.saturating_add(1);
    if promoted==0{
        h.curtain_raise_failures=h.curtain_raise_failures.saturating_add(1);
    }
    promoted
}
// A stable 3dfx flag alone does not mean Wine has put the fullscreen 3D
// window on screen: removing our topmost curtain can expose the desktop panel
// (a brief light strip at the bottom). Before releasing the curtain, inspect
// *other* visible top-level windows belonging to CYBER.EXE and require one
// to cover the monitor occupied by our temporary host. This is read-only:
// do not activate, resize, or reorder any game/nGlide window.
fn covers_monitor(rect:WinRect,monitor:WinRect)->bool{
    rect.left<=monitor.left.saturating_add(1)
        && rect.top<=monitor.top.saturating_add(1)
        && rect.right>=monitor.right.saturating_sub(1)
        && rect.bottom>=monitor.bottom.saturating_sub(1)
}
struct FullscreenScan {
    pid:u32,
    curtain:usize,
    monitor:WinRect,
    fullscreen:usize,
}
unsafe extern "system" fn scan_fullscreen_window(hwnd:*mut c_void,context:isize)->i32{
    let scan=&mut *(context as *mut FullscreenScan);
    if hwnd as usize==scan.curtain || IsWindowVisible(hwnd)==0{return 1;}
    let mut pid=0;
    let _=GetWindowThreadProcessId(hwnd,&mut pid);
    if pid!=scan.pid{return 1;}
    let mut rect=WinRect::default();
    if GetWindowRect(hwnd,&mut rect)!=0 && covers_monitor(rect,scan.monitor){
        scan.fullscreen=hwnd as usize;
        return 0;
    }
    1
}
unsafe fn visible_fullscreen_game_window(h:&NativeHost)->Option<usize>{
    let mut monitor=WinRect::default();
    if GetWindowRect(h.host as *mut c_void,&mut monitor)==0{return None;}
    let mut scan=FullscreenScan{
        pid:GetCurrentProcessId(),curtain:h.host,monitor,fullscreen:0,
    };
    let _=EnumWindows(Some(scan_fullscreen_window),
        (&mut scan as *mut FullscreenScan) as isize);
    (scan.fullscreen!=0).then_some(scan.fullscreen)
}
// The game's existing trace code identifies this exact (verified x86 EXE)
// state field as state_3dfx. A nonzero value is a *startup hint*, not proof
// that nGlide has already submitted a visible buffer.
fn decide_handoff(elapsed_ms:u64,stable_3dfx_ms:Option<u64>,game_valid:bool)->&'static str {
    if !game_valid{return "game_window_gone";}
    if stable_3dfx_ms.is_some_and(|age|age>=HANDOFF_STABLE_3D_MS){
        return "3dfx_stable";
    }
    if elapsed_ms>=HANDOFF_MAX_MS{return "bounded_timeout";}
    "wait"
}
fn poll_3d_handoff(){
    if !smooth_enabled(){return;}
    let outcome={
        let mut guard=video_host().lock().unwrap_or_else(|e|e.into_inner());
        let Some(h)=guard.as_mut() else{return;};
        if !h.handoff_poll_armed || !h.holding_for_transition
            || h.owner_thread!=unsafe{GetCurrentThreadId()}{return;}
        let Some(start)=h.handoff_started else{return;};
        let now=Instant::now();
        let game_valid=unsafe{IsWindow(h.game as *mut c_void)}!=0;
        let game_visible=game_valid&&unsafe{IsWindowVisible(h.game as *mut c_void)}!=0;
        let flag=if game_valid{
            unsafe{std::ptr::read_volatile(
                (EXPECTED_GAME_BASE+GAME_3DFX_ACTIVE_RVA) as *const u32)}
        }else{0};
        h.handoff_polls=h.handoff_polls.saturating_add(1);
        // A visible game HWND can still be a small window while Wine/nGlide
        // is rearranging the compositor. Wait for a monitor-sized HWND too.
        let fullscreen=if flag!=0 && game_visible{
            unsafe{visible_fullscreen_game_window(h)}
        }else{None};
        if let Some(fullscreen_hwnd)=fullscreen{
            if h.handoff_fullscreen_since.is_none(){
                h.handoff_fullscreen_since=Some(now);
                video_log(&format!(
                    "HANDOFF_FULLSCREEN_READY flag={flag} game_visible=1 fullscreen_hwnd={fullscreen_hwnd:#x} polling_for_stability_ms={HANDOFF_STABLE_3D_MS}"
                ));
                timing("HANDOFF_FULLSCREEN_READY");
            }
        }else{
            h.handoff_fullscreen_since=None;
        }
        let elapsed=now.saturating_duration_since(start).as_millis() as u64;
        let stable=h.handoff_fullscreen_since
            .map(|t|now.saturating_duration_since(t).as_millis() as u64);
        let decision=decide_handoff(elapsed,stable,game_valid);
        if decision=="wait"{return;}
        Some((decision.to_owned(),elapsed,flag,game_visible,fullscreen,h.handoff_polls))
    };
    if let Some((reason,elapsed,flag,visible,fullscreen,polls))=outcome{
        video_log(&format!(
            "HANDOFF_RELEASE reason={reason} elapsed_ms={elapsed} game_3dfx_flag={flag} game_visible={} fullscreen_hwnd={:#x} polls={polls}",
            u8::from(visible),fullscreen.unwrap_or(0)
        ));
        timing("HANDOFF_RELEASE");
        close_host(&format!("3d_handoff_{reason}"));
    }
}
/// Called after the original guarded WineD3D/DXVK WndProc repair, before
/// returning to CYBER.EXE from `mciSendStringA("close avivideo")`.
/// Hold for the user-validated one second, then let CYBER.EXE initialize
/// its original nGlide renderer BEHIND the retained full-monitor black host.
/// Release that host only after the game's 3dfx flag has been nonzero for a
/// stable grace interval with a monitor-covering game window, or after a
/// bounded fallback timeout (no hooks).
pub fn hold_post_video_black_before_3d(){
    if !smooth_enabled(){return;}
    if gameframe_first_present_handoff(){
        // The alternative first-present path has already armed the black
        // GDI host with a bounded timer and must NOT block game startup.
        video_log("GAMEFRAME_FIRST_GL_HANDOFF post_close_sleep_skipped=1 original_game_can_initialize_OpenGL=1");
        return;
    }
    let armed={
        let mut guard=video_host().lock().unwrap_or_else(|e|e.into_inner());
        match guard.as_mut(){
            Some(h) if h.holding_for_transition
                && h.owner_thread==unsafe{GetCurrentThreadId()}
                && unsafe{IsWindow(h.host as *mut c_void)}!=0=>{
                let raised=unsafe{promote_curtain(h)};
                video_log(&format!(
                    "CURTAIN_PRE_3D_REASSERT topmost_result={raised} active=1"
                ));
                true
            },
            _=>false,
        }
    };
    if !armed{
        video_log("CURTAIN_SYNC_SKIP no_owned_active_host; falling_back_to_game");
        return;
    }
    timing("POST_VIDEO_BLACK_HOLD_BEGIN");
    video_log(&format!(
        "POST_VIDEO_BLACK_HOLD mode=synchronous_before_3d requested_ms={TRANSITION_DELAY_MS}"
    ));
    // No host mutex is held while blocking the game, just like the verified
    // one-second pre-video black hold.
    let started=Instant::now();
    std::thread::sleep(Duration::from_millis(TRANSITION_DELAY_MS));
    video_log(&format!(
        "POST_VIDEO_BLACK_HOLD_END actual_ms={} before_game_3d_resume=1",
        started.elapsed().as_millis()
    ));
    timing("POST_VIDEO_BLACK_HOLD_END");

    // The old path closed the host HERE and exposed the compositor desktop
    // before nGlide's first framebuffer was ready. Do not close it until
    // 3D startup has had a chance to complete, with an explicit timeout.
    let polling={
        let mut guard=video_host().lock().unwrap_or_else(|e|e.into_inner());
        if let Some(h)=guard.as_mut(){
            if h.holding_for_transition
                && h.owner_thread==unsafe{GetCurrentThreadId()}
                && unsafe{IsWindow(h.host as *mut c_void)}!=0{
                let timer=unsafe{SetTimer(h.host as *mut c_void,HANDOFF_TIMER_ID,
                    HANDOFF_POLL_MS,std::ptr::null())};
                if timer!=0{
                    h.handoff_poll_armed=true;
                    h.handoff_started=Some(Instant::now());
                    h.handoff_fullscreen_since=None;
                    h.handoff_polls=0;
                    true
                }else{
                    video_log(&format!(
                        "HANDOFF_POLL_FAILED win32={} fallback=immediate_host_close",
                        unsafe{GetLastError()}
                    ));
                    false
                }
            }else{false}
        }else{false}
    };
    if polling{
        video_log(&format!(
            "HANDOFF_WAIT_BEGIN mode=retain_black_host_until_fullscreen_window poll_ms={HANDOFF_POLL_MS} stable_ms={HANDOFF_STABLE_3D_MS} max_ms={HANDOFF_MAX_MS} first_present_verified=0"
        ));
        timing("HANDOFF_WAIT_BEGIN");
    }else{
        close_host("post_video_sync_hold_poll_unavailable");
    }
}
/// The native Wine MCIAVI driver draws via GDI. In a CHILD of the active
/// OpenGL game HWND, its 1920x1080 client is still clipped by Wine's
/// 640x480 HDC clip box. Reuse the already proven separate GDI fullscreen
/// MCI host, but keep it black above the game until the first REAL OpenGL
/// SwapBuffers completes. The GameFrame render thread will post a narrow
/// Win32 message back to THIS host; its original owner UI thread destroys
/// the host. Bounded timer fallback avoids an indefinitely black window.
fn gameframe_arm_first_present_handoff()->bool{
    if !gameframe_first_present_handoff(){return false;}
    if !start_transition_curtain(){return false;}
    let mut guard=video_host().lock().unwrap_or_else(|e|e.into_inner());
    let Some(h)=guard.as_mut() else{return false;};
    if h.owner_thread!=unsafe{GetCurrentThreadId()}{
        return false;
    }
    unsafe{
        let host=h.host as *mut c_void;
        if IsWindow(host)==0{return false;}
        let tagged=SetPropA(host,GAMEFRAME_FIRST_GL_PROP.as_ptr(),1usize as *mut c_void);
        if tagged==0{
            video_log(&format!("GAMEFRAME_FIRST_GL_HANDOFF property_failed={} fallback=close",GetLastError()));
            drop(guard);
            close_host("gameframe_first_present_property_failed");
            return true;
        }
        let clock=GAMEFRAME_HANDOFF_START.get_or_init(||Mutex::new(None));
        *clock.lock().unwrap_or_else(|e|e.into_inner())=Some(Instant::now());
        GAMEFRAME_FIRST_GL_ARMED.store(true,Ordering::Release);
        let timer=SetTimer(host,HANDOFF_TIMER_ID,HANDOFF_POLL_MS,std::ptr::null());
        if timer==0{
            video_log(&format!("GAMEFRAME_FIRST_GL_HANDOFF timer_failed={} fallback=close",GetLastError()));
            GAMEFRAME_FIRST_GL_ARMED.store(false,Ordering::Release);
            drop(guard);
            close_host("gameframe_first_present_timer_failed");
            return true;
        }
        h.handoff_poll_armed=true;
        video_log(&format!("GAMEFRAME_FIRST_GL_HANDOFF armed=1 GDI_fullscreen_host={:#x} native_MCI_video_closed=1 black_host_retained=1 wait_for_post_SwapBuffers=1 fallback_timeout_ms={} no_sync_hold=1",
            h.host,GAMEFRAME_HANDOFF_TIMEOUT_MS));
        timing("GAMEFRAME_FIRST_PRESENT_WAIT_BEGIN");
        true
    }
}

fn start_transition_curtain()->bool {
    let mut guard=video_host().lock().unwrap_or_else(|e|e.into_inner());
    let Some(h)=guard.as_mut() else{return false;};
    unsafe{
        if GetCurrentThreadId()!=h.owner_thread || IsWindow(h.host as *mut c_void)==0{
            video_log("SMOOTH_ABORT invalid_owner_or_host");
            return false;
        }
        stop_movie_input(h);
        // Playback can end before the first-frame cover deadline. A leftover
        // cover must never intercept input or outlive the movie.
        if h.first_frame_cover_started.take().is_some(){
            video_log("FIRST_FRAME_COVER_CANCELLED reason=movie_closed before_status_poll_release");
        }
        let hidden=ShowWindow(h.surface as *mut c_void,SW_HIDE);
        let covered=if h.first_frame_cover!=0 &&
            IsWindow(h.first_frame_cover as *mut c_void)!=0{
            ShowWindow(h.first_frame_cover as *mut c_void,SW_SHOW);
            SetWindowPos(h.first_frame_cover as *mut c_void,std::ptr::null_mut(),
                0,0,0,0,
                SWP_NOACTIVATE|SWP_NOMOVE|SWP_NOSIZE|SWP_SHOWWINDOW)
        }else{0};
        let _=InvalidateRect(h.host as *mut c_void,std::ptr::null(),1);
        let _=UpdateWindow(h.host as *mut c_void);
        let promoted=promote_curtain(h);
        h.holding_for_transition=true;
        video_log(&format!(
            "SMOOTH_CURTAIN_ARMED mode=synchronous_before_3d requested_ms={TRANSITION_DELAY_MS} topmost_initial={promoted} mci_hide_previous_visible={hidden} black_child_restack={covered} no_activate=1 no_D3D9_hook=1"
        ));
        timing("CURTAIN_ARMED_AFTER_AVI_CLOSE");
        true
    }
}
/// Attempt to cover the earliest transition BEFORE the game's splash open.
/// Failing here is harmless: the existing open-splash handler still creates
/// the ordinary 4:3 host and preserves the validated game path.
fn early_black_host() {
    if !smooth_enabled(){return;}
    if video_host().lock().unwrap_or_else(|e|e.into_inner()).is_some(){return;}
    unsafe{
        let base=GetModuleHandleA(std::ptr::null()) as usize;
        if base!=EXPECTED_GAME_BASE{
            video_log("SMOOTH_EARLY_SKIP unknown_game_image");
            return;
        }
        let game=std::ptr::read_volatile((base+game_hwnd_rva()) as *const usize);
        let Some(h)=prepare_host(game) else{
            video_log("SMOOTH_EARLY_SKIP game_window_not_ready; fallback=normal_MCI_open");
            return;
        };
        video_log("SMOOTH_EARLY_BLACK_READY before_open_avivideo=1 video_surface_hidden_from_create=1");
        timing("EARLY_BLACK_HOST_READY");
        *video_host().lock().unwrap_or_else(|e|e.into_inner())=Some(h);
    }
}
/// Called before the native 'open avivideo': the earliest verified moment
/// in the game's video sequence, before the WinMM backend can display an
/// initial gray surface. This is only enabled for --smooth-video-test.
// Distinguish the currently active Windows display mode from the user's
// saved desktop mode. This is pure telemetry: no ChangeDisplaySettings,
// DirectDraw COM hooks, GetDC ownership, focus changes, or SetWindowPos.
fn fr_read_display_mode(which:i32)->String {
    unsafe {
        let mut mode:DevModeW=std::mem::zeroed();
        mode.size=std::mem::size_of::<DevModeW>() as u16;
        if EnumDisplaySettingsW(std::ptr::null(),which,&mut mode)==0 {
            "unavailable".to_owned()
        } else {
            format!("{}x{}x{}@{}Hz",
                mode.pels_width,mode.pels_height,mode.bits_per_pel,mode.frequency)
        }
    }
}
fn trace_fr_mode(stage:&str){
    if std::env::var("CYBER_GAME_EDITION").as_deref()!=Ok("FR")
        || std::env::var("CYBER_GAMEFRAME_ALT_TAB_NATIVE_FIX").as_deref()!=Ok("1"){
        return;
    }
    unsafe {
        video_log(&format!(
            "FR_DISPLAY_MODE stage={stage} metrics={}x{} current={} registry={} read_only=1",
            GetSystemMetrics(0),GetSystemMetrics(1),
            fr_read_display_mode(-1),fr_read_display_mode(-2)
        ));
    }
}
// Passive geometry-only telemetry for the French native-Windows scenario.
// Never changes the display or hooks the DirectDraw COM object.
fn trace_fr_screen(stage:&str){
    if !smooth_enabled() || std::env::var("CYBER_GAME_EDITION").as_deref()!=Ok("FR")
        || std::env::var("CYBER_GAMEFRAME_ALT_TAB_NATIVE_FIX").as_deref()!=Ok("1"){
        return;
    }
    unsafe{
        video_log(&format!(
            "FR_NATIVE_SCREEN stage={stage} reported_screen={}x{} captured_desktop={} directdraw_hook=disabled",
            GetSystemMetrics(0),GetSystemMetrics(1),
            std::env::var("CYBER_GAMEFRAME_DESKTOP_SIZE").as_deref().unwrap_or("missing")
        ));
    }
    trace_fr_mode(stage);
}
pub fn before_mci_open(command:&str){
    if smooth_enabled() && command.trim().eq_ignore_ascii_case("open avivideo"){
        trace_fr_screen("before_avivideo_open");
        timing("PRE_MCI_AVIVIDEO_BLACK_REQUEST");
        early_black_host();
    }
}
// Once the original MCI movie device has been opened, hide its entire
// video parent surface. The separate fullscreen host remains painted black.
// MCI's subsequent "window movie state show" is allowed to operate normally;
// the parent surface is revealed only after successful MCI "play" below.
fn cover_pre_video_surface() {
    if !smooth_enabled(){return;}
    let h=*video_host().lock().unwrap_or_else(|e|e.into_inner());
    let Some(h)=h else{return;};
    unsafe {
        if GetCurrentThreadId()!=h.owner_thread || IsWindow(h.surface as *mut c_void)==0 {
            video_log("PRE_VIDEO_SKIP invalid_surface_or_thread");
            return;
        }
        let _=ShowWindow(h.surface as *mut c_void,SW_HIDE);
        let _=InvalidateRect(h.host as *mut c_void,std::ptr::null(),1);
        let _=UpdateWindow(h.host as *mut c_void);
        timing("PRE_VIDEO_SURFACE_HIDDEN");
    }
}
/// Runs before forwarding the one original play command to system WinMM.
/// Sleeping the game UI thread here is intentional: the AVI has not started,
/// the black host is already painted, and no D3D or window-procedure hook is
/// introduced. Never keep the host mutex locked while sleeping.
pub fn before_mci_play(command:&str) {
    if !smooth_enabled() || !command.trim().eq_ignore_ascii_case("play movie window"){
        return;
    }
    let h=*video_host().lock().unwrap_or_else(|e|e.into_inner());
    let Some(h)=h else{return;};
    unsafe {
        if GetCurrentThreadId()!=h.owner_thread
            || IsWindow(h.host as *mut c_void)==0
            || IsWindow(h.surface as *mut c_void)==0 {
            video_log("PRE_VIDEO_SKIP invalid_host_or_thread");
            return;
        }
        // Keep the parent surface hidden even if the MCI state-show command
        // exposed its own child after the initial cover request.
        let _=ShowWindow(h.surface as *mut c_void,SW_HIDE);
        let _=InvalidateRect(h.host as *mut c_void,std::ptr::null(),1);
        let _=UpdateWindow(h.host as *mut c_void);
    }
    trace_fr_screen("before_video_play");
    timing("PRE_VIDEO_BLACK_BEGIN");
    video_log(&format!("PRE_VIDEO_BLACK_HOLD duration_ms={VIDEO_LEAD_IN_MS}"));
    std::thread::sleep(Duration::from_millis(VIDEO_LEAD_IN_MS));
    timing("PRE_VIDEO_BLACK_END");
}
fn reveal_playing_video() {
    if !smooth_enabled(){return;}
    let h=*video_host().lock().unwrap_or_else(|e|e.into_inner());
    let Some(h)=h else{return;};
    unsafe {
        if GetCurrentThreadId()!=h.owner_thread||IsWindow(h.surface as *mut c_void)==0 {
            video_log("VIDEO_REVEAL_SKIP invalid_surface_or_thread");
            return;
        }
        // The MCI surface must be visible to actually produce its initial
        // decoded frames. The sibling cover stays above it while it starts.
        // A distinct MCI-aware WndProc repaints the real AVI destination:
        // never share the black parent's WM_PAINT implementation here.
        if virtual_mci_surface_enabled(){
            VIRTUAL_MCI_SURFACE_PLAYING.store(true,Ordering::Release);
            VIRTUAL_MCI_SURFACE_PAINT_COUNT.store(0,Ordering::Release);
            VIRTUAL_MCI_SURFACE_TIMER_COUNT.store(0,Ordering::Release);
        }
        let _=ShowWindow(h.surface as *mut c_void,SW_SHOW);
        let _=InvalidateRect(h.surface as *mut c_void,std::ptr::null(),0);
        let _=UpdateWindow(h.surface as *mut c_void);
        if virtual_mci_surface_enabled(){
            let timer=SetTimer(h.surface as *mut c_void,VIRTUAL_MCI_TIMER_ID,
                VIRTUAL_MCI_TIMER_MS,std::ptr::null());
            video_log(&format!(
                "FR_VIRTUAL_VIDEO_RENDER paint_class=separate                  parent_black=1 surface_black_during_play=0                  mci_update_in_wm_paint=1 timer_id={VIRTUAL_MCI_TIMER_ID}                  timer_ms={VIRTUAL_MCI_TIMER_MS} timer_armed={}                  no_desktop_mode_switch=1",
                u8::from(timer!=0)
            ));
        }
        video_log(&format!(
            "VIDEO_SURFACE_REVEALED visible={} after_successful_MCI_play=1 under_initial_black_cover={}",
            IsWindowVisible(h.surface as *mut c_void),
            u8::from(h.first_frame_cover!=0)
        ));
        timing("VIDEO_REVEALED_AFTER_PRE_ROLL");
        if h.first_frame_cover==0{return;}
        // MCI/WinMM may change child z-order when it exposes its surface.
        // Force the black sibling above the Cinepak parent without changing
        // focus, video geometry, renderer, or native MCI playback.
        let cover=h.first_frame_cover as *mut c_void;
        if IsWindow(cover)==0{return;}
        let stacked=SetWindowPos(cover,std::ptr::null_mut(),0,0,0,0,
            SWP_NOACTIVATE|SWP_NOMOVE|SWP_NOSIZE|SWP_SHOWWINDOW);
        if stacked!=0 {
            let mut guard=video_host().lock().unwrap_or_else(|e|e.into_inner());
            if let Some(active)=guard.as_mut(){
                if active.host==h.host {
                    active.first_frame_cover_started=Some(Instant::now());
                }
            }
            video_log(&format!(
                "FIRST_FRAME_COVER_ACTIVE delay_ms={INITIAL_FRAME_COVER_MS} cover_above_mci=1 release_on=mci_status_poll"
            ));
            timing("FIRST_FRAME_COVER_ARMED");
        }else{
            video_log(&format!(
                "FIRST_FRAME_COVER_FAILED setpos={stacked} win32={} fallback=immediate_reveal",
                GetLastError()
            ));
            let _=ShowWindow(cover,SW_HIDE);
        }
    }
}
/// Called before the real system WinMM entry point. Returns an alternative
/// MCI string only for the precise original splash-video command sequence.
pub fn rewrite_for_4_3(text:&str)->Option<CString>{
    if !video_enabled(){return None;}
    if let Some(game)=movie_open_game_hwnd(text){
        // The optional smooth test may have already painted the black host
        // at "open avivideo". Reuse it; never create two competing popups.
        let existing=*video_host().lock().unwrap_or_else(|e|e.into_inner());
        let h=if let Some(h)=existing{
            if smooth_enabled()&&h.game==game&&h.owner_thread==unsafe{GetCurrentThreadId()}{
                video_log("SMOOTH_EARLY_HOST_REUSED for_splash_open=1");
                h
            }else{
                // Fail open: never leave an early black popup covering an
                // unrecognized MCI target, even if its video cannot be
                // rewritten for this game's 4:3 host.
                video_log("SMOOTH_EARLY_HOST_MISMATCH destroy_and_fallback");
                close_host("early_host_mismatch");
                let h=unsafe{prepare_host(game)}?;
                *video_host().lock().unwrap_or_else(|e|e.into_inner())=Some(h);
                h
            }
        }else{
            let h=unsafe{prepare_host(game)}?;
            *video_host().lock().unwrap_or_else(|e|e.into_inner())=Some(h);
            h
        };
        let rewritten=format!("open splash4.avi alias movie parent {} style child",h.surface);
        let Ok(encoded)=CString::new(rewritten) else{
            close_host("MCI_open_rewrite_failed");
            return None;
        };
        video_log(&format!("MCI_REWRITE open_style=child target_surface={:#x}",h.surface));
        timing("AVI_OPEN_COMMAND_REWRITTEN");
        return Some(encoded);
    }
    let guard=video_host().lock().unwrap_or_else(|e|e.into_inner());
    let h=(*guard)?;
    if exact_handle_command(text)==Some(h.game){
        video_log(&format!("MCI_REWRITE window_handle={:#x} (video_surface)",h.surface));
        return CString::new(format!("window movie handle {}",h.surface)).ok();
    }
    if destination_command(text){
        video_log(&format!("MCI_REWRITE destination=0,0 {}x{} (surface-relative, 4x3=1)",
            h.video_width,h.video_height));
        return CString::new(format!(
            "put movie destination at 0 0 {} {}",
            h.video_width,h.video_height
        )).ok();
    }
    None
}

/// Native Windows FR switches the desktop out of the game's 640x480 mode
/// during Alt+Tab. The initial host was correctly sized to 640x480, but
/// without adapting it and the MCI destination on mode restoration the
/// movie remains a small window. Run only after the ORIGINAL MCI status
/// request has returned on the owner's thread; never hook DirectDraw or
/// change the display mode ourselves.
/// Returns a native MCI destination command to forward through the real
/// system WinMM export, without recursively entering our own interceptor.
pub fn native_fr_refresh_during_movie(command:&str,result:u32)->Option<CString>{
    if result!=0 || !command.trim().eq_ignore_ascii_case("status movie mode")
        || !gameframe_first_present_handoff()
        || std::env::var("CYBER_GAME_EDITION").as_deref()!=Ok("FR")
        || std::env::var("CYBER_GAMEFRAME_ALT_TAB_NATIVE_FIX").as_deref()!=Ok("1"){
        return None;
    }
    // Snapshot under the mutex and release it BEFORE synchronous Win32
    // SetWindowPos: native window messages can reenter the WinMM bridge.
    let h=(*video_host().lock().unwrap_or_else(|e|e.into_inner()))?;
    unsafe {
        if h.owner_thread!=GetCurrentThreadId()
            || IsWindow(h.host as *mut c_void)==0
            || IsWindow(h.surface as *mut c_void)==0 {
            return None;
        }
        // The host is a NOACTIVATE topmost window, while the actual
        // foreground belongs to the original game. On focus loss, lower
        // BOTH windows so neither covers the selected application. Do not
        // activate or minimize either window.
        let foreground=GetForegroundWindow();
        let active=foreground==h.game as *mut c_void
            || foreground==h.host as *mut c_void;
        if h.native_fr_last_active!=Some(active){
            const NO_MOVE_SIZE_ACTIVATE:u32=SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE;
            let host=h.host as *mut c_void;
            let game=h.game as *mut c_void;
            let (host_result,game_result)=if active{
                (SetWindowPos(host,(-1isize) as *mut c_void,
                    0,0,0,0,NO_MOVE_SIZE_ACTIVATE),-1)
            }else{
                let _=SetWindowPos(host,(-2isize) as *mut c_void,
                    0,0,0,0,NO_MOVE_SIZE_ACTIVATE);
                let host_result=SetWindowPos(host,1usize as *mut c_void,
                    0,0,0,0,NO_MOVE_SIZE_ACTIVATE);
                let _=SetWindowPos(game,(-2isize) as *mut c_void,
                    0,0,0,0,NO_MOVE_SIZE_ACTIVATE);
                let game_result=SetWindowPos(game,1usize as *mut c_void,
                    0,0,0,0,NO_MOVE_SIZE_ACTIVATE);
                (host_result,game_result)
            };
            video_log(&format!(
                "FR_NATIVE_VIDEO_FOCUS active={} foreground={:#x} host_zorder={} game_zorder={} no_activation=1",
                u8::from(active),foreground as usize,host_result,game_result
            ));
            trace_fr_mode("movie_focus_transition");
            if let Some(current)=video_host().lock().unwrap_or_else(|e|e.into_inner()).as_mut(){
                if current.host==h.host {current.native_fr_last_active=Some(active);}
            }
        }
        let (width,height)=(GetSystemMetrics(0),GetSystemMetrics(1));
        let Some((x,y,w,vh))=fit_4_3(width,height) else{return None;};
        let mut previous=WinRect::default();
        if GetWindowRect(h.host as *mut c_void,&mut previous)==0{return None;}
        let current_width=previous.right-previous.left;
        let current_height=previous.bottom-previous.top;
        if current_width==width && current_height==height
            && h.video_width==w && h.video_height==vh {
            return None;
        }
        // Preserve current z-order (including Alt+Tab background state).
        // Never show the black first-frame cover again after it is hidden.
        let no_focus_zorder=SWP_NOZORDER|SWP_NOACTIVATE;
        let host_ok=SetWindowPos(h.host as *mut c_void,std::ptr::null_mut(),
            0,0,width,height,no_focus_zorder);
        if host_ok==0 {
            video_log(&format!("FR_NATIVE_VIDEO_RESIZE_FAILED stage=host win32={}",GetLastError()));
            return None;
        }
        let surface_ok=SetWindowPos(h.surface as *mut c_void,std::ptr::null_mut(),
            x,y,w,vh,no_focus_zorder);
        if surface_ok==0 {
            video_log(&format!("FR_NATIVE_VIDEO_RESIZE_FAILED stage=surface win32={}",GetLastError()));
            return None;
        }
        if h.first_frame_cover!=0 && IsWindow(h.first_frame_cover as *mut c_void)!=0{
            let _=SetWindowPos(h.first_frame_cover as *mut c_void,std::ptr::null_mut(),
                x,y,w,vh,no_focus_zorder);
        }
        video_log(&format!(
            "FR_NATIVE_VIDEO_RESIZED old_host={}x{} new_host={}x{} video_4_3=({x},{y} {w}x{vh}) active={} mode_change=none no_activate=1",
            current_width,current_height,width,height,u8::from(active)
        ));
        trace_fr_mode("movie_resize");
        if let Some(current)=video_host().lock().unwrap_or_else(|e|e.into_inner()).as_mut(){
            if current.host==h.host {
                current.video_width=w;
                current.video_height=vh;
            }
        }
        CString::new(format!("put movie destination at 0 0 {w} {vh}")).ok()
    }
}
pub fn native_fr_resize_mci_result(code:u32){
    video_log(&format!("FR_NATIVE_VIDEO_MCI_DESTINATION result={code}"));
}

pub fn on_4_3_command_result(text:&str,rewritten:bool,result:u32){
    if !video_enabled(){return;}
    let command=text.trim();
    maybe_release_first_frame_cover(command);
    if command.eq_ignore_ascii_case("open avivideo") {
        if result==0{
            timing("MCI_AVIVIDEO_OPENED");
        }else if smooth_enabled(){
            close_host("native_avivideo_open_failed");
        }
    }
    if command.eq_ignore_ascii_case("window movie state show") && result==0{
        // Wine's MCIAVI state-show can re-show its parent. Re-hide it until
        // the one-second black pre-roll ends and actual playback succeeds.
        cover_pre_video_surface();
    }
    if rewritten && movie_open_game_hwnd(text).is_some() {
        if result!=0 {
            video_log(&format!("MCI_OPEN_REWRITTEN_FAILED code={result}; original_command_retry"));
            close_host("rewritten_MCI_open_failed");
        }else {
            cover_pre_video_surface();
        }
    }
    if command.eq_ignore_ascii_case("play movie window") && result==0{
        timing("MCI_PLAY_RETURNED");
        reveal_playing_video();
        enable_movie_keys();
    }
    if command.eq_ignore_ascii_case("close movie"){
        stop_virtual_mci_video_surface("close_movie");
        timing("MCI_MOVIE_CLOSED");
    }
    if command.eq_ignore_ascii_case("close avivideo"){
        timing("MCI_AVIVIDEO_CLOSED");
        trace_fr_mode("after_avivideo_close");
        // An isolated Vulkan-only A/B experiment: leave all native AVI
        // operations and pre-video black cover intact, but destroy the
        // temporary host immediately after the original MCI driver closed.
        // This intentionally exposes the normal compositor handoff and
        // excludes our retained fullscreen HWND from nGlide initialization.
        // The verified D3D9 and default Vulkan smooth paths are unchanged.
        let gameframe=smooth_enabled() && result==0 &&
            std::env::var("CYBER_GAMEFRAME_TEST").as_deref()==Ok("1");
        let early_release=smooth_enabled() && result==0 &&
            std::env::var("CYBER_VULKAN_EARLY_HOST_RELEASE").as_deref()==Ok("1");
        if gameframe && gameframe_first_present_handoff(){
            video_log("GAMEFRAME_VIDEO_HANDOFF mode=standalone_native_GDI_then_first_OpenGL_present video_host_does_not_parent_to_OpenGL_HWND=1");
            gameframe_prepare_window("after_MCI_close_before_first_gl_present");
            if !gameframe_arm_first_present_handoff(){
                close_host("gameframe_first_present_arm_failed");
            }
        }else if early_release {
            video_log("VULKAN_EARLY_RELEASE after_native_MCI_close=1 no_black_postroll=1 no_handoff_timer=1 renderer_can_initialize_without_host=1");
            close_host("vulkan_early_release_ab");
            start_vulkan_fullscreen_recovery();
        }else if !(should_arm_curtain(smooth_enabled(),result) && start_transition_curtain()){
            close_host(if result==0{"after_MCI_close"}else{"MCI_close_failed"});
        }
    }
}
#[cfg(test)]
mod fullscreen_tests {
    use super::*;
    #[test]
    fn american_and_french_game_window_profiles_are_distinct(){
        assert_eq!(hwnd_rva_for(Some("US")),0x0012_8018);
        assert_eq!(class_proc_rva_for(Some("US")),0x0000_5f82);
        assert_eq!(hwnd_rva_for(Some("FR")),0x0012_8118);
        assert_eq!(class_proc_rva_for(Some("FR")),0x0000_604e);
        // Legacy US diagnostic entry points have no edition variable.
        assert_eq!(hwnd_rva_for(None),0x0012_8018);
        assert_eq!(class_proc_rva_for(None),0x0000_5f82);
    }

    #[test]
    fn gameframe_early_fullscreen_geometry_and_style_are_strict(){
        assert_eq!(gameframe_prelaunch_desktop(Some("1920x1080")),Some((1920,1080)));
        assert_eq!(gameframe_prelaunch_desktop(Some("640x480")),Some((640,480)));
        assert_eq!(gameframe_prelaunch_desktop(Some("0x1080")),None);
        assert_eq!(gameframe_prelaunch_desktop(Some("1920x-1")),None);
        assert_eq!(gameframe_prelaunch_desktop(Some("1920X1080")),None);
        assert_eq!(gameframe_prelaunch_desktop(None),None);
        assert_eq!(gameframe_popup_style(0x14ce0000),0x940a0000);
        assert_eq!(gameframe_popup_style(0x940a0000),0x940a0000);
    }
    #[test]
    fn native_fr_defers_only_redundant_game_window_resize_before_3d(){
        let stage="after_MCI_close_before_first_gl_present";
        assert!(should_defer_native_fr_handoff_resize(stage,true,true,640,480));
        assert!(!should_defer_native_fr_handoff_resize(stage,true,false,640,480),
            "Proton and US must keep their validated handoff path");
        assert!(!should_defer_native_fr_handoff_resize(stage,false,true,640,480),
            "the first-present guard must be available as fallback");
        assert!(!should_defer_native_fr_handoff_resize(stage,true,true,1536,960),
            "if Windows is already back at desktop resolution, keep old behavior");
        assert!(!should_defer_native_fr_handoff_resize("before_mci",true,true,640,480),
            "limit deferral to the proven transition");
    }
    #[test]
    fn native_fr_read_only_display_mode_uses_win32_devmode_abi(){
        assert_eq!(std::mem::size_of::<DevModeW>(),220);
    }
    #[test]
    fn native_fr_avi_geometry_adapts_from_exclusive_mode_to_desktop(){
        assert_eq!(fit_4_3(640,480),Some((0,0,640,480)));
        assert_eq!(fit_4_3(1536,960),Some((128,0,1280,960)));
        assert_eq!(fit_4_3(1920,1080),Some((240,0,1440,1080)));
    }
    #[test]
    fn gameframe_avi_host_separates_native_fr_and_wine_display_modes(){
        let transient=WinRect{left:0,top:0,right:640,bottom:480};
        let desktop=gameframe_prelaunch_desktop(Some("1920x1080"));
        let (host,used)=choose_gameframe_avi_monitor(transient,desktop,true,false);
        assert!(used);
        assert_eq!((host.left,host.top,host.right,host.bottom),(0,0,1920,1080));
        assert_eq!(fit_4_3(host.right-host.left,host.bottom-host.top),
            Some((240,0,1440,1080)));

        // The FR native-Windows host cannot exceed the *active* 640x480
        // monitor: an oversized host clips the film until Alt+Tab.
        let (native_fr,used)=choose_gameframe_avi_monitor(transient,
            gameframe_prelaunch_desktop(Some("1536x960")),true,true);
        assert!(!used);
        assert_eq!((native_fr.right,native_fr.bottom),(640,480));
        assert_eq!(fit_4_3(native_fr.right,native_fr.bottom),
            Some((0,0,640,480)));
        let full=WinRect{left:0,top:0,right:1536,bottom:960};
        let (native_full,used)=choose_gameframe_avi_monitor(full,
            gameframe_prelaunch_desktop(Some("1536x960")),true,true);
        assert!(!used);
        assert_eq!((native_full.right,native_full.bottom),(1536,960));

        // The already-working Proton path remains at native desktop size.
        let proton=WinRect{left:0,top:0,right:1920,bottom:1080};
        let (same,used)=choose_gameframe_avi_monitor(proton,desktop,true,false);
        assert!(used);
        assert_eq!((same.right,same.bottom),(1920,1080));

        // Non-GameFrame, secondary displays and invalid/missing snapshots
        // must not use primary monitor coordinates.
        let (normal,used)=choose_gameframe_avi_monitor(transient,desktop,false,false);
        assert!(!used);
        assert_eq!((normal.right,normal.bottom),(640,480));
        let secondary=WinRect{left:1920,top:0,right:3200,bottom:1024};
        let (other,used)=choose_gameframe_avi_monitor(secondary,desktop,true,false);
        assert!(!used);
        assert_eq!((other.left,other.right),(1920,3200));
        let (fallback,used)=choose_gameframe_avi_monitor(transient,None,true,false);
        assert!(!used);
        assert_eq!((fallback.right,fallback.bottom),(640,480));
    }
    #[test]
    fn x86_sendinput_layout_and_alt_enter_events(){
        assert_eq!(std::mem::size_of::<WinInput>(),28);
        let inputs=[
            keyboard_input(VK_MENU,false),keyboard_input(VK_RETURN as u16,false),
            keyboard_input(VK_RETURN as u16,true),keyboard_input(VK_MENU,true),
        ];
        assert_eq!(inputs[0].payload[0],u32::from(VK_MENU));
        assert_eq!(inputs[1].payload[0],VK_RETURN as u32);
        assert_eq!(inputs[2].payload[1],KEYEVENTF_KEYUP);
        assert_eq!(inputs[3].payload[1],KEYEVENTF_KEYUP);
    }
    #[test]
    fn initial_frame_cover_releases_after_elapsed_time_not_window_timer(){
        let start=Instant::now();
        assert!(!first_frame_cover_is_due(start,start));
        assert!(!first_frame_cover_is_due(start,
            start+Duration::from_millis(u64::from(INITIAL_FRAME_COVER_MS)-1)));
        assert!(first_frame_cover_is_due(start,
            start+Duration::from_millis(u64::from(INITIAL_FRAME_COVER_MS))));
        assert!(first_frame_cover_is_due(start,start+Duration::from_secs(1)));
        assert!(INITIAL_FRAME_COVER_MS>=200);
        assert!(INITIAL_FRAME_COVER_MS<=500);
        assert_eq!(TRANSITION_DELAY_MS,1000);
    }
    #[test]
    fn smooth_avi_parent_and_cover_paint_explicit_black_without_theme(){
        assert_eq!(video_surface_class_for(true,false),HOST_CLASS);
        assert_eq!(video_surface_class_for(true,true),VIRTUAL_VIDEO_CLASS);
        assert_eq!(video_surface_class_for(false,true),b"STATIC\0");
        assert_eq!(video_surface_class(false),b"STATIC\0");
        assert_eq!(video_surface_style(true)&SS_BLACKRECT,0);
        assert_eq!(first_frame_cover_style()&SS_BLACKRECT,0);
        assert_ne!(first_frame_cover_style()&WS_VISIBLE,0);
        assert_ne!(video_surface_style(false)&SS_BLACKRECT,0);
        assert_eq!(BLACK_BRUSH,4);
    }
    #[test]
    fn smooth_window_is_managed_and_video_surface_initially_hidden(){
        assert_ne!(host_exstyle(true)&WS_EX_APPWINDOW,0);
        assert_eq!(host_exstyle(true)&WS_EX_TOOLWINDOW,0);
        assert_ne!(host_exstyle(true)&WS_EX_NOACTIVATE,0);
        assert_ne!(host_exstyle(true)&WS_EX_TOPMOST,0);
        assert_eq!(video_surface_style(true)&WS_VISIBLE,0);
        assert_eq!(host_exstyle(false)&WS_EX_APPWINDOW,0);
        assert_ne!(host_exstyle(false)&WS_EX_TOOLWINDOW,0);
        assert_ne!(video_surface_style(false)&WS_VISIBLE,0);
    }
    #[test]
    fn handoff_3d_has_readiness_grace_and_bounded_fallback(){
        assert_eq!(decide_handoff(100,None,true),"wait");
        assert_eq!(decide_handoff(600,Some(HANDOFF_STABLE_3D_MS-1),true),"wait");
        assert_eq!(decide_handoff(600,Some(HANDOFF_STABLE_3D_MS),true),"3dfx_stable");
        assert_eq!(decide_handoff(HANDOFF_MAX_MS,None,true),"bounded_timeout");
        assert_eq!(decide_handoff(10,None,false),"game_window_gone");
        assert!(HANDOFF_POLL_MS<=100);
        assert!(HANDOFF_MAX_MS<=4000);
    }
    #[test]
    fn fullscreen_window_must_cover_bottom_panel_and_all_screen_edges(){
        let monitor=WinRect{left:0,top:0,right:1920,bottom:1080};
        assert!(covers_monitor(monitor,monitor));
        assert!(covers_monitor(WinRect{left:-8,top:-8,right:1928,bottom:1088},monitor));
        assert!(!covers_monitor(WinRect{left:0,top:0,right:1920,bottom:1040},monitor));
        assert!(!covers_monitor(WinRect{left:0,top:0,right:1280,bottom:720},monitor));
        assert!(!covers_monitor(WinRect{left:20,top:0,right:1920,bottom:1080},monitor));
    }
    #[test]
    fn transition_curtain_holds_exactly_one_second_before_3d_resume(){
        assert_eq!(TRANSITION_DELAY_MS,1000);
        assert_eq!(VIDEO_LEAD_IN_MS,1000);
        assert_eq!(Duration::from_millis(TRANSITION_DELAY_MS),Duration::from_millis(VIDEO_LEAD_IN_MS));
    }
    #[test]
    fn transition_curtain_is_opt_in_and_requires_successful_mci_close(){
        assert!(!should_arm_curtain(false,0),"normal launch must close the host immediately");
        assert!(!should_arm_curtain(true,1),"failed MCI close must release the host");
        assert!(should_arm_curtain(true,0));
        assert_eq!(TRANSITION_DELAY_MS,1000);
        assert_eq!(VIDEO_LEAD_IN_MS,1000);
    }
    #[test]
    fn movie_mouse_filter_blocks_buttons_but_not_motion(){
        for message in [
            WM_LBUTTONDOWN,WM_LBUTTONUP,WM_LBUTTONDBLCLK,
            WM_RBUTTONDOWN,WM_RBUTTONUP,WM_RBUTTONDBLCLK,
            WM_MBUTTONDOWN,WM_MBUTTONUP,WM_MBUTTONDBLCLK,
            WM_XBUTTONDOWN,WM_XBUTTONUP,WM_XBUTTONDBLCLK,
            WM_MOUSEWHEEL,WM_MOUSEHWHEEL,
        ] {
            assert!(blocked_movie_mouse_message(message),"{message:#x}");
        }
        assert!(!blocked_movie_mouse_message(WM_MOUSEMOVE));
        assert!(!blocked_movie_mouse_message(WM_PAINT));
    }
    #[test]
    fn skip_filter_accepts_only_three_virtual_keys(){
        for key in [VK_ESCAPE,VK_RETURN,VK_SPACE]{
            assert!(allowed_movie_skip_key(key));
        }
        for key in [0x41,0x5a,0x09,0x08,0x25,0x26,0x70,0x12,0x10,0]{
            assert!(!allowed_movie_skip_key(key),"{key:#x}");
        }
    }
    #[test]
    fn aspect_ratio_letterboxes_without_stretching(){
        assert_eq!(fit_4_3(1920,1080),Some((240,0,1440,1080)));
        assert_eq!(fit_4_3(1280,720),Some((160,0,960,720)));
        assert_eq!(fit_4_3(1024,768),Some((0,0,1024,768)));
        assert_eq!(fit_4_3(1920,1200),Some((160,0,1600,1200)));
        assert_eq!(fit_4_3(0,1080),None);
    }
    #[test]
    fn rewrite_matches_only_the_game_splash_sequence(){
        assert_eq!(movie_open_game_hwnd(
            "open splash4.avi alias movie parent 131252 style popup"),Some(131252));
        assert_eq!(exact_handle_command("window movie handle 131252"),Some(131252));
        assert!(destination_command("put movie destination at 0 0 640 480"));
        assert!(!destination_command("put movie destination at 10 0 640 480"));
        assert_eq!(movie_open_game_hwnd("open other.avi alias movie parent 131252 style popup"),None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mci_repaint_throttle_never_repaints_faster_than_interval(){
        let now=Instant::now();
        let mut last=None;
        assert!(mci_refresh_is_due(now,&mut last,Duration::from_millis(500)));
        assert!(!mci_refresh_is_due(now+Duration::from_millis(499),
            &mut last,Duration::from_millis(500)));
        assert!(mci_refresh_is_due(now+Duration::from_millis(500),
            &mut last,Duration::from_millis(500)));
        assert!(!mci_refresh_is_due(now+Duration::from_millis(600),
            &mut last,Duration::from_millis(500)));
        assert!(mci_refresh_is_due(now+Duration::from_millis(1000),
            &mut last,Duration::from_millis(500)));
    }
    #[test]
    fn virtual_avi_message_pump_only_during_successful_status_poll(){
        assert!(should_pump_virtual_avi(true,true,true,true));
        for cond in [
            (false,true,true,true),(true,false,true,true),
            (true,true,false,true),(true,true,true,false)
        ] {
            assert!(!should_pump_virtual_avi(cond.0,cond.1,cond.2,cond.3));
        }
    }
    #[test]
    fn only_movie_mci_commands_are_traced(){
        assert!(before("open avivideo").is_some());
        assert!(before("play movie window").is_some());
        assert!(before("close movie").is_some());
        assert!(before("status movie mode").is_none());
        assert!(before("set c time format tmsf").is_none());
        assert!(after("status c mode",0,"playing").is_none());
    }
    #[test]
    fn logs_status_transitions_without_spamming_poll_loop(){
        let mut state=MovieState::default();
        let _=after_in(&mut state,"play movie window",0,"");
        assert!(after_in(&mut state,"status movie mode",0,"playing").unwrap().contains("playing"));
        for _ in 0..398{assert!(after_in(&mut state,"status movie mode",0,"playing").is_none());}
        let m=after_in(&mut state,"status movie mode",0,"playing").unwrap();
        assert!(m.contains("polls=400"),"{m}");
        assert!(after_in(&mut state,"status movie mode",0,"stopped").unwrap().contains("stopped"));
        assert!(after_in(&mut state,"close movie",0,"").unwrap().contains("close movie"));
    }
    #[test]
    fn captures_failed_video_commands(){
        let message=after("play movie window",277,"").unwrap();
        assert!(message.contains("result=277"));
        assert!(after("close avivideo",0,"").unwrap().contains("close avivideo"));
    }
}
