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
const PAGE_READWRITE:u32=0x04;
const GWL_STYLE:i32=-16;
const GWL_EXSTYLE:i32=-20;

static INSTALLED:OnceLock<bool>=OnceLock::new();
static ORIGINAL:AtomicUsize=AtomicUsize::new(0);
static GAME_WINDOW_THREAD:AtomicU32=AtomicU32::new(0);
const THREAD_QUERY_INFORMATION:u32=0x0040;
const SYNCHRONIZE:u32=0x0010_0000;
const WAIT_OBJECT_0:u32=0;

#[link(name="kernel32")]
extern "system" {
    fn GetModuleHandleA(name:*const u8)->*mut c_void;
    fn GetProcAddress(module:*mut c_void,name:*const u8)->*mut c_void;
    fn VirtualProtect(addr:*mut c_void,bytes:usize,new_protection:u32,old:*mut u32)->i32;
    fn GetCurrentProcessId()->u32;
    fn OpenThread(access:u32,inherit:i32,thread_id:u32)->*mut c_void;
    fn GetExitCodeThread(thread:*mut c_void,code:*mut u32)->i32;
    fn WaitForSingleObject(handle:*mut c_void,timeout_ms:u32)->u32;
    fn CloseHandle(handle:*mut c_void)->i32;
    fn GetLastError()->u32;
}
#[link(name="user32")]
extern "system" {
    fn GetWindowLongA(hwnd:*mut c_void,index:i32)->i32;
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
                "hwnd={hwnd:?} tid={tid} visible={} minimized={} style={:#010x} ex_style={:#010x} rect={geometry}",
                IsWindowVisible(hwnd),IsIconic(hwnd),
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
    report(&format!("PASSTHRU BEGIN mode={mode} (0=VGA, 1=3dfx)"));
    if should_bypass(mode,experimental_bypass_enabled()){
        report("PASSTHRU BYPASSED mode=1 (diagnostic A/B test)");
    } else {
        let original=ORIGINAL.load(Ordering::Acquire);
        if original!=0{
            let actual:unsafe extern "system" fn(i32)=std::mem::transmute(original);
            actual(mode);
        }
    }
    report(&format!("PASSTHRU END mode={mode}"));
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
            "WINDOW {stage} game_hwnd={hwnd:?} valid={} owner_pid={owner_pid} owner_tid={owner_tid} visible={} foreground={foreground:?} state_3dfx={voodoo_active} state_vga={vga_active}",
            if valid{1}else{0},if valid{IsWindowVisible(hwnd)}else{0}
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
    }
    game_thread_snapshot(stage);
}

/// Observe whether the game restores its 1996 full-screen window style after
/// WinMM has closed the AVI. Only read Windows state; no window alterations.
pub fn after_video_close(){
    if !enabled(){return;}
    if unsafe{GetModuleHandleA(std::ptr::null())} as usize != EXPECTED_IMAGE_BASE {return;}
    window_snapshot("immediately after MCI close");
    // Dense, read-only sampling for the first 500 ms, where the last
    // reproducible trace shows DirectDraw objects disappearing and the game
    // window thread exiting. No hooks are installed on COM or Glide here.
    thread::spawn(||{
        let start=Instant::now();
        for delay in [10u64,25,50,75,100,125,150,200,250,300,350,400,450,500,
                      750,1000,1500,4000,8000] {
            let target=Duration::from_millis(delay);
            if let Some(remaining)=target.checked_sub(start.elapsed()){
                thread::sleep(remaining);
            }
            window_snapshot(&format!("{} ms after MCI close",delay));
        }
    });
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn known_original_addresses_match_va(){
        assert_eq!(EXPECTED_IMAGE_BASE+PASSTHRU_WRAPPER_RVA,0x0043_519d);
        assert_eq!(EXPECTED_IMAGE_BASE+PASSTHRU_SLOT_RVA,0x0044_ab4c);
        assert_eq!(EXPECTED_IMAGE_BASE+MAIN_WINDOW_RVA,0x0052_8018);
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
    fn tracing_is_disabled_unless_opted_in(){
        // CI never sets the explicitly project-scoped variable.
        if std::env::var_os("CYBER_GLIDE_TRACE").is_none(){
            assert!(!enabled());
        }
    }
}
