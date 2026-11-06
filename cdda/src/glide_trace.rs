// SPDX-License-Identifier: GPL-3.0-or-later
//! Opt-in trace for the Voodoo1/VGA pass-through transition around AVI clips.
//! Installed ONLY with CYBER_GLIDE_TRACE=1 (set by --launch --trace), on the
//! exact known 1996 x86 executable. It changes a single *in-memory* function
//! pointer only after matching the live GLIDE export and saves its original.
//! No file is modified, and normal --launch never installs this observer.
use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32,AtomicUsize,Ordering};
use std::thread;
use std::time::{Duration,Instant};

const EXPECTED_IMAGE_BASE:usize=0x0040_0000;
const PASSTHRU_WRAPPER_RVA:usize=0x0003_519d;
const PASSTHRU_SLOT_RVA:usize=0x0004_ab4c;
const MAIN_WINDOW_RVA:usize=0x0012_8018; // game HWND at 0x00528018.
const ORIGINAL_GAME_WNDPROC_RVA:usize=0x0000_5f82; // confirmed by user's class WndProc

const PAGE_READWRITE:u32=0x04;
const GWL_STYLE:i32=-16;
const GWL_EXSTYLE:i32=-20;
const GWL_WNDPROC:i32=-4;
const GCL_WNDPROC:i32=-24;

static INSTALLED:OnceLock<bool>=OnceLock::new();
static ORIGINAL:AtomicUsize=AtomicUsize::new(0);
static GAME_WINDOW_THREAD:AtomicU32=AtomicU32::new(0);
// State 0: never entered; 1: inside passthrough; 2: returned to our hook.
static PASSTHRU_STATE:AtomicU32=AtomicU32::new(0);
static PASSTHRU_THREAD:AtomicU32=AtomicU32::new(0);
const THREAD_QUERY_INFORMATION:u32=0x0040;
const SYNCHRONIZE:u32=0x0010_0000;
const WAIT_OBJECT_0:u32=0;

#[link(name="kernel32")]
extern "system" {
    fn GetModuleHandleA(name:*const u8)->*mut c_void;
    fn GetProcAddress(module:*mut c_void,name:*const u8)->*mut c_void;
    fn VirtualProtect(addr:*mut c_void,bytes:usize,new_protection:u32,old:*mut u32)->i32;
    fn GetCurrentProcessId()->u32;
    fn GetCurrentThreadId()->u32;
    fn OpenThread(access:u32,inherit:i32,thread_id:u32)->*mut c_void;
    fn GetExitCodeThread(thread:*mut c_void,code:*mut u32)->i32;
    fn WaitForSingleObject(handle:*mut c_void,timeout_ms:u32)->u32;
    fn CloseHandle(handle:*mut c_void)->i32;
    fn GetLastError()->u32;
    fn SetLastError(code:u32);
}
#[link(name="user32")]
extern "system" {
    fn GetWindowLongA(hwnd:*mut c_void,index:i32)->i32;
    fn GetWindowLongW(hwnd:*mut c_void,index:i32)->i32;
    fn SetWindowLongA(hwnd:*mut c_void,index:i32,value:i32)->i32;
    fn GetClassLongA(hwnd:*mut c_void,index:i32)->u32;
    fn GetClassLongW(hwnd:*mut c_void,index:i32)->u32;
    fn IsWindowUnicode(hwnd:*mut c_void)->i32;
    fn IsWindow(hwnd:*mut c_void)->i32;
    fn IsWindowVisible(hwnd:*mut c_void)->i32;
    fn IsIconic(hwnd:*mut c_void)->i32;
    fn GetForegroundWindow()->*mut c_void;
    fn GetWindowThreadProcessId(hwnd:*mut c_void,process_id:*mut u32)->u32;
    fn GetWindowRect(hwnd:*mut c_void,rect:*mut Rect)->i32;
    fn EnumWindows(callback:unsafe extern "system" fn(*mut c_void,isize)->i32,context:isize)->i32;
}
#[repr(C)]
#[derive(Default)]
struct Rect {
    left:i32,
    top:i32,
    right:i32,
    bottom:i32,
}
struct WindowScan {
    current_pid:u32,
    total:usize,
    sample:Vec<String>,
}
unsafe extern "system" fn collect_process_windows(hwnd:*mut c_void,context:isize)->i32 {
    let snapshot=&mut *(context as *mut WindowScan);
    let mut pid=0u32;
    let tid=GetWindowThreadProcessId(hwnd,&mut pid);
    if pid==snapshot.current_pid {
        snapshot.total+=1;
        if snapshot.sample.len()<8 {
            let mut rect=Rect::default();
            let geometry=if GetWindowRect(hwnd,&mut rect)!=0 {
                format!("({},{})-({},{})",rect.left,rect.top,rect.right,rect.bottom)
            } else {"unavailable".to_owned()};
            snapshot.sample.push(format!(
                "hwnd={hwnd:?} tid={tid} visible={} minimized={} wndproc={:#010x} style={:#010x} ex_style={:#010x} rect={geometry}",
                IsWindowVisible(hwnd),IsIconic(hwnd),
                GetWindowLongA(hwnd,GWL_WNDPROC) as u32,
                GetWindowLongA(hwnd,GWL_STYLE) as u32,
                GetWindowLongA(hwnd,GWL_EXSTYLE) as u32
            ));
        }
    }
    1
}

fn enabled()->bool{
    std::env::var("CYBER_GLIDE_TRACE").is_ok_and(|v|v=="1")
}
fn report(message:&str){crate::log(&format!("GLIDE_RESTORE {message}"));}

/// Exact ABI of _grSstPassthruMode@4. The default trace always delegates
/// to the original export without modifying arguments or return behavior.
/// An explicitly requested *diagnostic-only* A/B test can omit solely the
/// mode=1 call made after AVI cleanup (no on-disk patch).
unsafe extern "system" fn passthru_probe(mode:i32) {
    let tid=unsafe{GetCurrentThreadId()};
    PASSTHRU_THREAD.store(tid,Ordering::Release);
    PASSTHRU_STATE.store(1,Ordering::Release);
    report(&format!("PASSTHRU BEGIN mode={mode} tid={tid} (0=VGA, 1=3dfx)"));
    if should_bypass(mode,experimental_bypass_enabled()){
        report("PASSTHRU BYPASSED mode=1 (diagnostic A/B test)");
    } else {
        let original=ORIGINAL.load(Ordering::Acquire);
        if original!=0{
            let actual:unsafe extern "system" fn(i32)=std::mem::transmute(original);
            actual(mode);
        }
    }
    PASSTHRU_STATE.store(2,Ordering::Release);
    report(&format!("PASSTHRU END mode={mode} tid={tid}"));
}
fn experimental_bypass_enabled()->bool {
    enabled() && std::env::var("CYBER_GLIDE_PASSTHRU_NOOP").is_ok_and(|v|v=="1")
}
fn should_bypass(mode:i32,requested:bool)->bool{
    mode==1 && requested
}

/// Hook exactly one verified game-owned pointer; never change glide.dll,
/// system WinMM, the Wine prefix, or the on-disk CYBER.EXE.
fn install()->Result<(),String>{
    unsafe{
        let base=GetModuleHandleA(std::ptr::null()) as usize;
        if base!=EXPECTED_IMAGE_BASE {
            return Err(format!("unexpected EXE base {base:#x}, expected {EXPECTED_IMAGE_BASE:#x}"));
        }
        let signature=std::slice::from_raw_parts(
            (base+PASSTHRU_WRAPPER_RVA) as *const u8,5);
        if signature!=[0x55,0x8b,0xec,0x83,0x3d] {
            return Err("original grSstPassthruMode wrapper signature mismatch".into());
        }
        let glide=GetModuleHandleA(b"glide.dll\0".as_ptr());
        if glide.is_null(){return Err("Glide DLL has not been loaded".into());}
        let exact=GetProcAddress(glide,b"_grSstPassthruMode@4\0".as_ptr());
        if exact.is_null(){return Err("Glide DLL lacks _grSstPassthruMode@4".into());}
        report(&format!("GLIDE_NATIVE module_base={glide:?} passthru_export={exact:?}"));

        let slot=(base+PASSTHRU_SLOT_RVA) as *mut usize;
        let current=std::ptr::read_volatile(slot);
        if current!=exact as usize {
            return Err(format!("GLIDE pass-through pointer differs from resolved export: {current:#x}"));
        }
        let mut old=0u32;
        if VirtualProtect(slot.cast(),std::mem::size_of::<usize>(),PAGE_READWRITE,&mut old)==0 {
            return Err("VirtualProtect refused the game pass-through slot".into());
        }
        ORIGINAL.store(current,Ordering::Release);
        std::ptr::write_volatile(slot,passthru_probe as usize);
        let mut ignored=0u32;
        let restored=VirtualProtect(slot.cast(),std::mem::size_of::<usize>(),old,&mut ignored)!=0;
        if !restored {
            // Best-effort rollback. This page is still writable after a failed
            // permission restore; never leave a partially installed probe.
            std::ptr::write_volatile(slot,current);
            ORIGINAL.store(0,Ordering::Release);
            let _=VirtualProtect(slot.cast(),std::mem::size_of::<usize>(),old,&mut ignored);
            return Err("pass-through tracing unavailable (page-protection restore failed)".into());
        }
        Ok(())
    }
}
pub fn maybe_install(){
    if !enabled(){return;}
    // Control run: preserve MCI, window, and first-chance exception tracing
    // but never rewrite the game's imported Glide function pointer.
    // This lets us distinguish a native renderer failure from probe effects.
    if std::env::var("CYBER_GLIDE_HOOK_DISABLED").as_deref()==Ok("1"){
        INSTALLED.get_or_init(||{
            report("GLIDE_HOOK_DISABLED=1 (control run: no IAT slot modification)");
            false
        });
        return;
    }
    let installed=*INSTALLED.get_or_init(||{
        report(&format!(
            "diagnostic environment present: trace=1 passthru_noop={}",
            if experimental_bypass_enabled(){1}else{0}
        ));
        match install(){
            Ok(())=>{
                report("pass-through call boundary tracing active");
                if experimental_bypass_enabled(){
                    report("experimental mode=1 pass-through NO-OP enabled (trace-only)");
                }
                true
            },
            Err(e)=>{report(&format!("pass-through tracing unavailable: {e}"));false},
        }
    });
    if !installed {
        // The video-MCI trace remains usable even if this optional hook was
        // rejected, so never fail the game's normal video initialization.
    }
}

/// Experimental diagnosis of a WndProc cycle between WineD3D and DXVK.
/// The user's source-identified current WndProc is wined3d.dll+0xf4a20,
/// while the original ANSI window class procedure is cyber.exe+0x5f82.
/// Only replace the *current live game window's* pointer, on its owning
/// thread, immediately after WinMM has successfully closed its AVI.
/// No system DLLs, D3D devices, registry or files are modified.
fn repair_eligible(base:usize,class_proc:usize,unicode:i32,
    installed_proc_origin:&str)->bool {
    base==EXPECTED_IMAGE_BASE && class_proc==base+ORIGINAL_GAME_WNDPROC_RVA
        && unicode==0
        && installed_proc_origin.to_ascii_lowercase().starts_with("wined3d.dll")
}
pub fn maybe_repair_windowproc_after_video_close(){
    // Normal launches use the same guarded operation without the invasive
    // tracing sampler; the explicit experiment still sets this same flag.
    if std::env::var("CYBER_WNDPROC_REPAIR").as_deref()!=Ok("1")
        && std::env::var("CYBER_WNDPROC_REPAIR_TEST").as_deref()!=Ok("1") {
        return;
    }
    unsafe{
        let base=GetModuleHandleA(std::ptr::null()) as usize;
        if base!=EXPECTED_IMAGE_BASE{
            report(&format!("WNDPROC_REPAIR SKIP unsupported_exe_base={base:#x}"));
            return;
        }
        let hwnd=std::ptr::read_volatile((base+MAIN_WINDOW_RVA) as *const usize)
            as *mut c_void;
        let mut owner_pid=0u32;
        let owner_tid=if IsWindow(hwnd)!=0{
            GetWindowThreadProcessId(hwnd,&mut owner_pid)
        }else{0};
        if owner_pid!=GetCurrentProcessId()||owner_tid!=GetCurrentThreadId(){
            report("WNDPROC_REPAIR SKIP invalid_window_or_not_owning_thread");
            return;
        }
        let unicode=IsWindowUnicode(hwnd);
        let class_proc=GetClassLongA(hwnd,GCL_WNDPROC) as usize;
        let prior=GetWindowLongA(hwnd,GWL_WNDPROC) as u32 as usize;
        let origin=crate::exception_trace::window_proc_origin(prior);
        if !repair_eligible(base,class_proc,unicode,&origin){
            report(&format!(
                "WNDPROC_REPAIR SKIP unicode={unicode} class={class_proc:#x} current={prior:#x} origin={origin} (guard_rejected)"
            ));
            return;
        }
        SetLastError(0);
        let replaced=SetWindowLongA(hwnd,GWL_WNDPROC,class_proc as i32)
            as u32 as usize;
        let error=GetLastError();
        if replaced==0 && error!=0 {
            report(&format!("WNDPROC_REPAIR ERROR SetWindowLongA_win32={error}"));
            return;
        }
        let current=GetWindowLongA(hwnd,GWL_WNDPROC) as u32 as usize;
        // Refuse an unexpected race. Restore only if our replacement still
        // owns the current slot, never trample another WndProc subclass.
        if replaced!=prior||current!=class_proc{
            if current==class_proc {
                let _=SetWindowLongA(hwnd,GWL_WNDPROC,replaced as i32);
            }
            report(&format!(
                "WNDPROC_REPAIR ROLLBACK expected_old={prior:#x} actual_old={replaced:#x} current={current:#x}"
            ));
            return;
        }
        report(&format!(
            "WNDPROC_REPAIR APPLIED hwnd={hwnd:?} old={prior:#x} ({origin}) new={class_proc:#x} (cyber.exe original class WndProc) scope=in_process_only guarded=1"
        ));
        crate::video_trace::timing("WNDPROC_REPAIR_APPLIED");
    }
}

/// Query the original game window thread even after its HWND has been
/// destroyed. GetExitCodeThread==259 (STILL_ACTIVE) and WaitForSingleObject
/// must be reported independently: a real thread can exit with code 259.
fn game_thread_snapshot(stage:&str) {
    let tid=GAME_WINDOW_THREAD.load(Ordering::Acquire);
    if tid==0 {
        report(&format!("WINDOW_THREAD {stage} thread_id=unavailable"));
        return;
    }
    unsafe {
        let handle=OpenThread(THREAD_QUERY_INFORMATION|SYNCHRONIZE,0,tid);
        if handle.is_null() {
            report(&format!(
                "WINDOW_THREAD {stage} thread_id={tid} OpenThread failed Win32={}",
                GetLastError()
            ));
            return;
        }
        let mut code=0u32;
        let have_code=GetExitCodeThread(handle,&mut code)!=0;
        let wait=WaitForSingleObject(handle,0);
        let _=CloseHandle(handle);
        report(&format!(
            "WINDOW_THREAD {stage} tid={tid} exit_code={} code_valid={} signaled={} wait={wait:#x}",
            if have_code {code.to_string()} else {"unknown".to_owned()},
            if have_code{1}else{0}, if wait==WAIT_OBJECT_0{1}else{0}
        ));
    }
}
fn window_snapshot(stage:&str) {
    // Resolve crash address and CPU registers from a healthy thread,
    // not in the exhausted-stack exception handler.
    crate::exception_trace::snapshot(stage);
    let state=PASSTHRU_STATE.load(Ordering::Acquire);
    let tid=PASSTHRU_THREAD.load(Ordering::Acquire);
    report(&format!("PASSTHRU_STATE {stage} state={state} calling_tid={tid} (0=not-called,1=in-call-or-no-return,2=returned)"));
    unsafe {
        let hwnd=std::ptr::read_volatile((EXPECTED_IMAGE_BASE+MAIN_WINDOW_RVA) as *const usize)
            as *mut c_void;
        let valid=IsWindow(hwnd)!=0;
        let mut owner_pid=0u32;
        let owner_tid=if valid{GetWindowThreadProcessId(hwnd,&mut owner_pid)}else{0};
        if valid && owner_pid==GetCurrentProcessId() && owner_tid!=0{
            let _=GAME_WINDOW_THREAD.compare_exchange(
                0,owner_tid,Ordering::AcqRel,Ordering::Acquire
            );
        }
        let foreground=GetForegroundWindow();
        let voodoo_active=std::ptr::read_volatile(0x0044_727c as *const u32);
        let vga_active=std::ptr::read_volatile(0x0044_7278 as *const u32);
        let directdraw=std::ptr::read_volatile(0x0044_728c as *const u32);
        let directdraw_primary=std::ptr::read_volatile(0x0044_7290 as *const u32);
        let directdraw_secondary=std::ptr::read_volatile(0x0044_7294 as *const u32);
        let saved_mode=std::ptr::read_volatile(0x0052_8140 as *const u32);
        report(&format!(
            "GRAPHICS {stage} dd={directdraw:#x} dd_primary={directdraw_primary:#x} dd_secondary={directdraw_secondary:#x} saved_mode={saved_mode}"
        ));
        report(&format!(
            "WINDOW {stage} game_hwnd={hwnd:?} valid={} owner_pid={owner_pid} owner_tid={owner_tid} visible={} wndproc={} foreground={foreground:?} state_3dfx={voodoo_active} state_vga={vga_active}",
            if valid{1}else{0},if valid{IsWindowVisible(hwnd)}else{0},
            if valid{format!("{:#x}",GetWindowLongA(hwnd,GWL_WNDPROC) as u32)}else{"unavailable".into()}
        ));
        let mut windows=WindowScan{
            current_pid:GetCurrentProcessId(),
            total:0,
            sample:Vec::new()
        };
        let enumerated=EnumWindows(collect_process_windows,
            (&mut windows as *mut WindowScan) as isize);
        report(&format!(
            "WINDOW_SCAN {stage} pid={} enum_result={enumerated} top_level_total={} entries=[{}]",
            windows.current_pid,windows.total,windows.sample.join(" | ")
        ));
        if valid && (stage=="immediately after MCI close"||stage=="100 ms after MCI close") {
            let instance_a=GetWindowLongA(hwnd,GWL_WNDPROC) as u32 as usize;
            let instance_w=GetWindowLongW(hwnd,GWL_WNDPROC) as u32 as usize;
            let class_a=GetClassLongA(hwnd,GCL_WNDPROC) as usize;
            let class_w=GetClassLongW(hwnd,GCL_WNDPROC) as usize;
            let origin=crate::exception_trace::window_proc_origin;
            report(&format!(
                "WNDPROC_PROVENANCE {stage} unicode={} hwnd={hwnd:?} instance_a={instance_a:#x} ({}) instance_w={instance_w:#x} ({}) class_a={class_a:#x} ({}) class_w={class_w:#x} ({}) read_only=1",
                IsWindowUnicode(hwnd),origin(instance_a),origin(instance_w),
                origin(class_a),origin(class_w)
            ));
        }
    }
    game_thread_snapshot(stage);
}

/// Observe whether the game restores its 1996 full-screen window style after
/// WinMM has closed the AVI. Only read Windows state; no window alterations.
fn passive_vulkan_fault_probe(value:Option<&str>)->bool{value==Some("1")}
pub fn after_video_close(){
    if !enabled(){return;}
    if unsafe{GetModuleHandleA(std::ptr::null())} as usize != EXPECTED_IMAGE_BASE {return;}
    let passive_vulkan=passive_vulkan_fault_probe(
        std::env::var("CYBER_VULKAN_PASSIVE_FAULT_PROBE").ok().as_deref());
    if passive_vulkan {
        // After AVI close, the Vulkan crash may take over 20 seconds.
        // A separate healthy thread reads only the exception atomics;
        // do not format or log from the exception handler itself.
        thread::spawn(||{
            for _ in 0..350 {
                thread::sleep(Duration::from_millis(100));
                crate::exception_trace::snapshot("Vulkan passive 100ms exception poll");
            }
        });
        report("VULKAN_FAULT_PROBE watcher=100ms bounded_ms=35000 read_only=1 glide_iat_hook=disabled");
    }
    crate::exception_trace::d3d9_identity("immediately after MCI close");
    window_snapshot("immediately after MCI close");
    // Dense, read-only sampling for the first 500 ms, where the last
    // reproducible trace shows DirectDraw objects disappearing and the game
    // window thread exiting. No hooks are installed on COM or Glide here.
    thread::spawn(||{
        let start=Instant::now();
        let mut delays=vec![10u64,25,50,75,100,125,150,200,250,300,350,400,450,500,
            750,1000,1500,4000,8000];
        if passive_vulkan {
            // Delayed window-state samples remain read-only and bounded.
            delays.extend_from_slice(&[12000,16000,20000,22000,24000,30000]);
        }
        for delay in delays {
            let target=Duration::from_millis(delay);
            if let Some(remaining)=target.checked_sub(start.elapsed()){
                thread::sleep(remaining);
            }
            if delay==200 {
                crate::exception_trace::d3d9_identity("200 ms after MCI close");
            }
            window_snapshot(&format!("{} ms after MCI close",delay));
        }
    });
}
// Experimental timing test: hold the MCI close call AFTER real WinMM has
// successfully closed the video, but BEFORE the game can call Glide. Only a
// trace launch with an explicitly selected value may delay the game thread.
// The independent window sampler continues running during the delay.
fn supported_video_delay(ms:u64)->bool{matches!(ms,100|250|500|1000)}
fn video_close_delay_ms()->Option<u64>{
    if !enabled(){return None;}
    std::env::var("CYBER_VIDEO_CLOSE_DELAY_MS").ok()
        .and_then(|s|s.parse::<u64>().ok())
        .filter(|ms|supported_video_delay(*ms))
}
pub fn maybe_delay_after_video_close(){
    let Some(ms)=video_close_delay_ms() else{return;};
    report(&format!("VIDEO_CLOSE_DELAY BEGIN duration_ms={ms} (trace-only, no graphics calls)"));
    thread::sleep(Duration::from_millis(ms));
    report(&format!("VIDEO_CLOSE_DELAY END duration_ms={ms}"));
    window_snapshot("after video-close delay, before returning to game");
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn known_original_addresses_match_va(){
        assert_eq!(EXPECTED_IMAGE_BASE+PASSTHRU_WRAPPER_RVA,0x0043_519d);
        assert_eq!(EXPECTED_IMAGE_BASE+PASSTHRU_SLOT_RVA,0x0044_ab4c);
        assert_eq!(EXPECTED_IMAGE_BASE+MAIN_WINDOW_RVA,0x0052_8018);
        assert_eq!(EXPECTED_IMAGE_BASE+ORIGINAL_GAME_WNDPROC_RVA,0x0040_5f82);
    }
    #[test]
    fn experimental_bypass_only_affects_mode_one(){
        assert!(!should_bypass(0,true));
        assert!(should_bypass(1,true));
        assert!(!should_bypass(1,false));
        assert!(!should_bypass(2,true));
        assert!(!should_bypass(-1,true));
    }
    #[test]
    fn passive_vulkan_watcher_is_strict_opt_in(){
        assert!(passive_vulkan_fault_probe(Some("1")));
        assert!(!passive_vulkan_fault_probe(Some("0")));
        assert!(!passive_vulkan_fault_probe(None));
    }
    #[test]
    fn delay_is_bounded_and_trace_only(){
        // No delay is enabled by default; invalid/unbounded values are
        // rejected at launch and are never acted on by the bridge.
        assert!([100u64,250,500,1000].iter().all(|ms|supported_video_delay(*ms)));
        assert!(!supported_video_delay(0));
        assert!(!supported_video_delay(10000));
    }
    #[test]
    fn repair_rejects_any_unverified_window_procedure(){
        let base=EXPECTED_IMAGE_BASE;
        assert!(repair_eligible(base,0x0040_5f82,0,"wined3d.dll base=0x7afa0000"));
        assert!(!repair_eligible(base,0x0040_5f82,1,"wined3d.dll"));
        assert!(!repair_eligible(base,0x0040_5f82,0,"d3d9.dll+0xba1d0"));
        assert!(!repair_eligible(base,0x0040_5f82,0,"cyber.exe+0x5f82"));
        assert!(!repair_eligible(base,0x0040_5f83,0,"wined3d.dll"));
        assert!(!repair_eligible(0x0050_0000,0x0040_5f82,0,"wined3d.dll"));
    }
    #[test]
    fn tracing_is_disabled_unless_opted_in(){
        // CI never sets the explicitly project-scoped variable.
        if std::env::var_os("CYBER_GLIDE_TRACE").is_none(){
            assert!(!enabled());
        }
    }
}
