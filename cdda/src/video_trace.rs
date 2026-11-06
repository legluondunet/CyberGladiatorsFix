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
const GAME_HWND_RVA:usize=0x0012_8018;
const GAME_CLASS_PROC_RVA:usize=0x0000_5f82;
const GCL_WNDPROC:i32=-24;
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
const HANDOFF_STABLE_3D_MS:u64=300;
const HANDOFF_MAX_MS:u64=2500;
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
static HOST_CLASS_REGISTERED:OnceLock<bool>=OnceLock::new();
static KEY_FILTER_ACTIVE:AtomicBool=AtomicBool::new(false);
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
#[repr(C)]
struct PaintStruct {
    dc:*mut c_void,erase:i32,paint:WinRect,
    restore:i32,inc_update:i32,reserved:[u8;32],
}
#[link(name="kernel32")]
extern "system" {
    fn GetModuleHandleA(module:*const u8)->*mut c_void;
    fn GetCurrentProcessId()->u32;
    fn GetCurrentThreadId()->u32;
    fn GetLastError()->u32;
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
    fn IsWindowVisible(hwnd:*mut c_void)->i32;
    fn GetWindowThreadProcessId(hwnd:*mut c_void,pid:*mut u32)->u32;
    fn GetClassLongA(hwnd:*mut c_void,index:i32)->u32;
    fn MonitorFromWindow(hwnd:*mut c_void,flags:u32)->*mut c_void;
    fn GetMonitorInfoA(monitor:*mut c_void,info:*mut MonitorInfo)->i32;
    fn CreateWindowExA(exstyle:u32,class_name:*const u8,title:*const u8,style:u32,
        x:i32,y:i32,width:i32,height:i32,parent:*mut c_void,
        menu:*mut c_void,instance:*mut c_void,param:*mut c_void)->*mut c_void;
    fn DestroyWindow(hwnd:*mut c_void)->i32;
    fn GetClientRect(hwnd:*mut c_void,rect:*mut WinRect)->i32;
    fn SetTimer(hwnd:*mut c_void,id:usize,timeout_ms:u32,callback:*const c_void)->usize;
    fn KillTimer(hwnd:*mut c_void,id:usize)->i32;
}
#[derive(Clone,Copy)]
struct NativeHost {
    host:usize,
    surface:usize,
    first_frame_cover:usize, // smooth test only; opaque sibling above video
    first_frame_cover_started:Option<Instant>,
    game:usize,
    owner_thread:u32,
    video_width:i32,
    video_height:i32,
    keyboard_hook:usize,
    mouse_hook:usize,
    holding_for_transition:bool,
    handoff_poll_armed:bool,
    handoff_started:Option<Instant>,
    handoff_3dfx_since:Option<Instant>,
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
            poll_3d_handoff();
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
fn video_surface_class(smooth:bool)->&'static [u8]{
    if smooth{HOST_CLASS}else{b"STATIC\0"}
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
    let game=std::ptr::read_volatile((base+GAME_HWND_RVA) as *const usize);
    let hwnd=game as *mut c_void;
    let mut pid=0u32;
    if original_window!=game || IsWindow(hwnd)==0
        || GetWindowThreadProcessId(hwnd,&mut pid)!=GetCurrentThreadId()
        || pid!=GetCurrentProcessId()
        || GetClassLongA(hwnd,GCL_WNDPROC) as usize!=base+GAME_CLASS_PROC_RVA {
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
    let screen_w=info.monitor.right.checked_sub(info.monitor.left)?;
    let screen_h=info.monitor.bottom.checked_sub(info.monitor.top)?;
    let (x,y,w,h)=fit_4_3(screen_w,screen_h)?;
    // Keep the custom class separate from the game's known subclass chain.
    // The explicitly black-painted background covers both pillarboxes.
    if !register_black_host_class(base as *mut c_void){return None;}
    let smooth=smooth_enabled();
    // The smooth experiment requests an independent managed NORMAL fullscreen
    // popup (rather than an owned tool-window/dialog). This prevents the
    // Linux desktop panel from staying above the video on WMs which honor
    // EWMH fullscreen. WS_EX_NOACTIVATE preserves keyboard focus on the game.
    let owner=if smooth{std::ptr::null_mut()}else{hwnd};
    let host=CreateWindowExA(host_exstyle(smooth),
        HOST_CLASS.as_ptr(),b"\0".as_ptr(),
        WS_POPUP|WS_VISIBLE|WS_CLIPCHILDREN,
        info.monitor.left,info.monitor.top,screen_w,screen_h,
        owner,std::ptr::null_mut(),base as *mut c_void,std::ptr::null_mut());
    if host.is_null() {
        video_log(&format!("SKIP fullscreen_host_create_failed={}",GetLastError()));
        return None;
    }
    // In smooth mode use our already-registered custom window class to paint
    // the MCI parent solid RGB(0,0,0), independent of Wine's system theme.
    // Keep it hidden from the very first CreateWindowExA invocation.
    let surface=CreateWindowExA(0,video_surface_class(smooth).as_ptr(),b"\0".as_ptr(),
        video_surface_style(smooth),x,y,w,h,host,
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
            first_frame_cover_style(),x,y,w,h,host,
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
        first_frame_cover,first_frame_cover_started:None,
        game,owner_thread:GetCurrentThreadId(),
        video_width:w,video_height:h,keyboard_hook:0,mouse_hook:0,
        holding_for_transition:false,
        handoff_poll_armed:false,handoff_started:None,
        handoff_3dfx_since:None,handoff_polls:0,
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
fn close_host(reason:&str){
    let previous=video_host().lock().unwrap_or_else(|e|e.into_inner()).take();
    if let Some(mut h)=previous{
        unsafe{
            KEY_FILTER_ACTIVE.store(false,Ordering::Release);
            if GetCurrentThreadId()!=h.owner_thread{
                video_log(&format!("WARNING host_not_destroyed_other_thread reason={reason}"));
                return;
            }
            stop_movie_input(&mut h);
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
        if flag!=0 && game_visible{
            if h.handoff_3dfx_since.is_none(){
                h.handoff_3dfx_since=Some(now);
                video_log(&format!(
                    "HANDOFF_3DFX_STATE_READY flag={flag} game_visible=1 polling_for_stability_ms={HANDOFF_STABLE_3D_MS}"
                ));
                timing("HANDOFF_3DFX_STATE_READY");
            }
        }else{
            h.handoff_3dfx_since=None;
        }
        let elapsed=now.saturating_duration_since(start).as_millis() as u64;
        let stable=h.handoff_3dfx_since
            .map(|t|now.saturating_duration_since(t).as_millis() as u64);
        let decision=decide_handoff(elapsed,stable,game_valid);
        if decision=="wait"{return;}
        Some((decision.to_owned(),elapsed,flag,game_visible,h.handoff_polls))
    };
    if let Some((reason,elapsed,flag,visible,polls))=outcome{
        video_log(&format!(
            "HANDOFF_RELEASE reason={reason} elapsed_ms={elapsed} game_3dfx_flag={flag} game_visible={} polls={polls}",
            u8::from(visible)
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
/// stable grace interval, or after a bounded fallback timeout (no hooks).
pub fn hold_post_video_black_before_3d(){
    if !smooth_enabled(){return;}
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
                    h.handoff_3dfx_since=None;
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
            "HANDOFF_WAIT_BEGIN mode=retain_black_host_while_3d_initializes poll_ms={HANDOFF_POLL_MS} stable_ms={HANDOFF_STABLE_3D_MS} max_ms={HANDOFF_MAX_MS} first_present_verified=0"
        ));
        timing("HANDOFF_WAIT_BEGIN");
    }else{
        close_host("post_video_sync_hold_poll_unavailable");
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
        let game=std::ptr::read_volatile((base+GAME_HWND_RVA) as *const usize);
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
pub fn before_mci_open(command:&str){
    if smooth_enabled() && command.trim().eq_ignore_ascii_case("open avivideo"){
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
        let _=ShowWindow(h.surface as *mut c_void,SW_SHOW);
        let _=UpdateWindow(h.surface as *mut c_void);
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
        video_log(&format!("MCI_REWRITE destination=0,0 {}x{} (surface-relative)",h.video_width,h.video_height));
        return CString::new(format!(
            "put movie destination at 0 0 {} {}",h.video_width,h.video_height
        )).ok();
    }
    None
}
pub fn on_4_3_command_result(text:&str,rewritten:bool,result:u32){
    if !video_enabled(){return;}
    let command=text.trim();
    // Called after each native WinMM status poll on the original game
    // thread: reliable even while that thread does not dispatch WM_TIMER.
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
        timing("MCI_MOVIE_CLOSED");
    }
    if command.eq_ignore_ascii_case("close avivideo"){
        timing("MCI_AVIVIDEO_CLOSED");
        if !(should_arm_curtain(smooth_enabled(),result) && start_transition_curtain()){
            close_host(if result==0{"after_MCI_close"}else{"MCI_close_failed"});
        }
    }
}
#[cfg(test)]
mod fullscreen_tests {
    use super::*;
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
        assert_eq!(video_surface_class(true),HOST_CLASS);
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
        assert!(HANDOFF_MAX_MS<=3000);
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
