// SPDX-License-Identifier: GPL-3.0-or-later
//! Opt-in trace for the Voodoo1/VGA pass-through transition around AVI clips.
//! Installed ONLY with CYBER_GLIDE_TRACE=1 (set by --launch --trace), on the
//! exact known 1996 x86 executable. It changes a single *in-memory* function
//! pointer only after matching the live GLIDE export and saves its original.
//! No file is modified, and normal --launch never installs this observer.
use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize,Ordering};
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

#[link(name="kernel32")]
extern "system" {
    fn GetModuleHandleA(name:*const u8)->*mut c_void;
    fn GetProcAddress(module:*mut c_void,name:*const u8)->*mut c_void;
    fn VirtualProtect(addr:*mut c_void,bytes:usize,new_protection:u32,old:*mut u32)->i32;
}
#[link(name="user32")]
extern "system" {
    fn GetWindowLongA(hwnd:*mut c_void,index:i32)->i32;
    fn IsWindow(hwnd:*mut c_void)->i32;
    fn IsWindowVisible(hwnd:*mut c_void)->i32;
}

fn enabled()->bool{
    std::env::var("CYBER_GLIDE_TRACE").is_ok_and(|v|v=="1")
}
fn report(message:&str){crate::log(&format!("GLIDE_RESTORE {message}"));}

/// Exact ABI of _grSstPassthruMode@4. Do not alter the mode or the return
/// behavior: this observer only brackets the existing nGlide call.
unsafe extern "system" fn passthru_probe(mode:i32) {
    report(&format!("PASSTHRU BEGIN mode={mode} (0=VGA, 1=3dfx)"));
    let original=ORIGINAL.load(Ordering::Acquire);
    if original!=0{
        let actual:unsafe extern "system" fn(i32)=std::mem::transmute(original);
        actual(mode);
    }
    report(&format!("PASSTHRU END mode={mode}"));
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
        match install(){
            Ok(())=>{report("pass-through call boundary tracing active");true},
            Err(e)=>{report(&format!("pass-through tracing unavailable: {e}"));false},
        }
    });
    if !installed {
        // The video-MCI trace remains usable even if this optional hook was
        // rejected, so never fail the game's normal video initialization.
    }
}

fn window_snapshot(stage:&str) {
    unsafe {
        let hwnd=std::ptr::read_volatile((EXPECTED_IMAGE_BASE+MAIN_WINDOW_RVA) as *const usize)
            as *mut c_void;
        if hwnd.is_null()||IsWindow(hwnd)==0 {
            report(&format!("WINDOW {stage} invalid HWND={hwnd:?}"));
            return;
        }
        let style=GetWindowLongA(hwnd,GWL_STYLE) as u32;
        let ex_style=GetWindowLongA(hwnd,GWL_EXSTYLE) as u32;
        let visible=IsWindowVisible(hwnd);
        report(&format!(
            "WINDOW {stage} hwnd={hwnd:?} style={style:#010x} ex_style={ex_style:#010x} visible={visible}"));
    }
}

/// Observe whether the game restores its 1996 full-screen window style after
/// WinMM has closed the AVI. Only read Windows state; no window alterations.
pub fn after_video_close(){
    if !enabled(){return;}
    if unsafe{GetModuleHandleA(std::ptr::null())} as usize != EXPECTED_IMAGE_BASE {return;}
    window_snapshot("immediately after MCI close");
    thread::spawn(||{
        let start=Instant::now();
        for delay in [100u64,500,1500,4000] {
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
    fn tracing_is_disabled_unless_opted_in(){
        // CI never sets the explicitly project-scoped variable.
        if std::env::var_os("CYBER_GLIDE_TRACE").is_none(){
            assert!(!enabled());
        }
    }
}
