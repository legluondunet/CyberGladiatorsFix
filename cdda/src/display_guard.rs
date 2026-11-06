//! Experimental Wine-only, game-scoped display-mode guard.
//!
//! Wine's virtual desktop can resize from the launcher's 1920x1080 snapshot
//! to the game's requested 640x480 before native MCI starts. Restoring an
//! AVI HWND to 1920x1080 cannot reverse that virtual-desktop mode change.
//!
//! This opt-in experiment patches ONLY CYBER.EXE's USER32 import thunks for
//! ChangeDisplaySettings{A,W,ExA,ExW}, and only during DLL_PROCESS_ATTACH.
//! It does not patch Wine, game files, DDRAW, or another process. If the
//! game's mode change travels through a different API, the log will expose
//! that this experiment did not catch it.
//!
//! Keep DLL_PROCESS_ATTACH loader-lock work allocation-free and log-free.
//! The first MCI command reports counters to the existing unified log.
use std::ffi::{c_char,c_void};
use std::sync::atomic::{AtomicBool,AtomicU32,Ordering};

const IMAGE_BASE:usize=0x0040_0000;
const PAGE_READWRITE:u32=0x04;
const DLL_PROCESS_ATTACH:u32=1;
static ACTIVE:AtomicBool=AtomicBool::new(false);
static INSTALLED:AtomicU32=AtomicU32::new(0);
static CANDIDATES:AtomicU32=AtomicU32::new(0);
static BLOCKED:AtomicU32=AtomicU32::new(0);
static REPORTED:AtomicBool=AtomicBool::new(false);
static STARTUP_ATTEMPTED:AtomicBool=AtomicBool::new(false);
static STARTUP_PATH:AtomicU32=AtomicU32::new(0);

#[link(name="kernel32")]
unsafe extern "system" {
    fn GetModuleHandleA(name:*const c_char)->*mut c_void;
    fn GetEnvironmentVariableA(name:*const c_char,value:*mut c_char,size:u32)->u32;
    fn VirtualProtect(address:*mut c_void,size:usize,new_protect:u32,old:*mut u32)->i32;
}
#[inline]
unsafe fn u16_at(addr:usize)->u16{(addr as *const u16).read_unaligned()}
#[inline]
unsafe fn u32_at(addr:usize)->u32{(addr as *const u32).read_unaligned()}
#[inline]
fn within(size:usize,offset:usize,len:usize)->bool{
    offset.checked_add(len).is_some_and(|end|end<=size)
}
unsafe fn eq_ascii_at(base:usize,size:usize,offset:usize,name:&[u8],casefold:bool)->bool{
    if !within(size,offset,name.len()+1){return false;}
    for (i,&wanted) in name.iter().enumerate(){
        let got=((base+offset+i) as *const u8).read();
        if (if casefold{got.to_ascii_lowercase()}else{got})!=wanted{return false;}
    }
    ((base+offset+name.len()) as *const u8).read()==0
}
fn select_api(name:&[u8])->Option<(usize,usize)>{
    // The exact imported name determines the stdcall ABI of its shim.
    // The IAT slot and stored original pointer are per API, not shared.
    match name{
        b"ChangeDisplaySettingsA"=>Some((guard_a as usize,0)),
        b"ChangeDisplaySettingsW"=>Some((guard_w as usize,1)),
        b"ChangeDisplaySettingsExA"=>Some((guard_ex_a as usize,2)),
        b"ChangeDisplaySettingsExW"=>Some((guard_ex_w as usize,3)),
        _=>None
    }
}
unsafe fn install_import_guard()->u32{
    let base=GetModuleHandleA(std::ptr::null()) as usize;
    if base!=IMAGE_BASE || u16_at(base)!=0x5a4d{return 0;}
    let pe=u32_at(base+0x3c) as usize;
    if pe<0x40 || pe>0x100000 || u32_at(base+pe)!=0x0000_4550{return 0;}
    let opt=base+pe+24;
    if u16_at(opt)!=0x10b{return 0;} // verified original PE32
    let image_size=u32_at(opt+56) as usize;
    if !(0x100000..=0x10000000).contains(&image_size){return 0;}
    let imports=u32_at(opt+104) as usize; // PE32 IMAGE_DIRECTORY_ENTRY_IMPORT
    if !within(image_size,imports,20){return 0;}
    let mut installed=0u32;
    for d in 0usize..128 {
        let Some(off)=imports.checked_add(d*20) else{break};
        if !within(image_size,off,20){break;}
        let descriptor=base+off;
        let original=u32_at(descriptor) as usize;
        let dll=u32_at(descriptor+12) as usize;
        let first=u32_at(descriptor+16) as usize;
        if original==0 && first==0{break;}
        if !eq_ascii_at(base,image_size,dll,b"user32.dll",true){continue;}
        if original==0 || first==0{continue;} // cannot recover by-name imports
        for t in 0usize..4096{
            let Some(lookup)=original.checked_add(t*4) else{break};
            let Some(iat)=first.checked_add(t*4) else{break};
            if !within(image_size,lookup,4)||!within(image_size,iat,4){break;}
            let raw=u32_at(base+lookup);
            if raw==0{break;}
            if raw&0x8000_0000!=0{continue;} // ordinal, not a name
            let Some(name_off)=(raw as usize).checked_add(2) else{continue};
            if !within(image_size,name_off,25){continue;}
            for (label,id) in [
                (b"ChangeDisplaySettingsA".as_slice(),0usize),
                (b"ChangeDisplaySettingsW".as_slice(),1),
                (b"ChangeDisplaySettingsExA".as_slice(),2),
                (b"ChangeDisplaySettingsExW".as_slice(),3),
            ]{
                if !eq_ascii_at(base,image_size,name_off,label,false){continue;}
                CANDIDATES.fetch_add(1,Ordering::Relaxed);
                let Some((replacement,which))=select_api(label) else{continue};
                if which!=id{continue;}
                let slot=(base+iat) as *mut u32;
                let before=slot.read();
                if before==0 || before as usize==replacement{continue;}
                let mut old=0;
                if VirtualProtect(slot.cast(),4,PAGE_READWRITE,&mut old)==0{continue;}
                slot.write_volatile(replacement as u32);
                let mut unused=0;
                let _=VirtualProtect(slot.cast(),4,old,&mut unused);
                // The verified game must not switch the desktop to its old
                // 640x480 mode, whether it passes NULL or DISPLAY1 by name.
                // No pass-through is needed for a game-scoped mode guard.
                installed+=1;
                break;
            }
        }
        break; // this game's USER32 descriptor is unique
    }
    installed
}
// Rust's cdylib entrypoint is not guaranteed to forward DLL_PROCESS_ATTACH
// to an exported function named DllMain. MSVC's CRT constructor table is
// executed before CYBER.EXE enters main(). Keep this strictly loader-safe:
// no heap, I/O, Rust TLS, DirectDraw COM calls or window management here.
#[used]
#[link_section = ".CRT$XCU"]
static EARLY_GUARD_INIT: unsafe extern "C" fn() = crt_guard_init;

unsafe extern "C" fn crt_guard_init(){
    install_at_startup(2);
}
unsafe fn install_at_startup(path:u32){
    if STARTUP_ATTEMPTED.swap(true,Ordering::AcqRel){return;}
    STARTUP_PATH.store(path,Ordering::Release);
    let mut value=[0i8;4];
    let requested=GetEnvironmentVariableA(
        b"CYBER_WINE_DESKTOP_MODE_GUARD\0".as_ptr().cast(),
        value.as_mut_ptr(),value.len() as u32)==1 && value[0]==b'1' as i8;
    if !requested{return;}
    let enabled=GetEnvironmentVariableA(
        b"CYBER_GAMEFRAME_TEST\0".as_ptr().cast(),
        value.as_mut_ptr(),value.len() as u32)==1 && value[0]==b'1' as i8;
    if !enabled{return;}
    let count=install_import_guard();
    INSTALLED.store(count,Ordering::Release);
    ACTIVE.store(count>0,Ordering::Release);
}
#[no_mangle]
pub unsafe extern "system" fn DllMain(
    _instance:*mut c_void,reason:u32,_reserved:*mut c_void
)->i32 {
    if reason==DLL_PROCESS_ATTACH{install_at_startup(1);}
    1
}
#[inline]
fn suppress_mode_request()->i32{
    BLOCKED.fetch_add(1,Ordering::Relaxed);
    0 // DISP_CHANGE_SUCCESSFUL, no physical or Wine virtual mode switch
}
unsafe extern "system" fn guard_a(_mode:*const c_void,_flags:u32)->i32{
    suppress_mode_request()
}
unsafe extern "system" fn guard_w(_mode:*const c_void,_flags:u32)->i32{
    suppress_mode_request()
}
unsafe extern "system" fn guard_ex_a(
    _name:*const c_char,_mode:*const c_void,_hwnd:*mut c_void,_flags:u32,_lparam:*mut c_void
)->i32{suppress_mode_request()}
unsafe extern "system" fn guard_ex_w(
    _name:*const u16,_mode:*const c_void,_hwnd:*mut c_void,_flags:u32,_lparam:*mut c_void
)->i32{suppress_mode_request()}

pub fn report_once(){
    if REPORTED.swap(true,Ordering::AcqRel){return;}
    let requested=std::env::var("CYBER_WINE_DESKTOP_MODE_GUARD").ok();
    let gameframe=std::env::var("CYBER_GAMEFRAME_TEST").ok();
    super::log(&format!(
        "WINE_DESKTOP_MODE_GUARD requested={:?} gameframe={:?} startup_called={} startup_path={} enabled={} user32_imports_matched={} hooks_installed={} suppressed_mode_requests={} other_renderers_untouched=1",
        requested.as_deref(),gameframe.as_deref(),
        u8::from(STARTUP_ATTEMPTED.load(Ordering::Acquire)),
        STARTUP_PATH.load(Ordering::Acquire),
        u8::from(ACTIVE.load(Ordering::Acquire)),
        CANDIDATES.load(Ordering::Acquire),
        INSTALLED.load(Ordering::Acquire),
        BLOCKED.load(Ordering::Acquire)
    ));
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn precise_api_names_and_bounds(){
        assert_eq!(select_api(b"ChangeDisplaySettingsA").map(|x|x.1),Some(0));
        assert_eq!(select_api(b"ChangeDisplaySettingsW").map(|x|x.1),Some(1));
        assert_eq!(select_api(b"ChangeDisplaySettingsExA").map(|x|x.1),Some(2));
        assert_eq!(select_api(b"ChangeDisplaySettingsExW").map(|x|x.1),Some(3));
        assert_eq!(select_api(b"ChangeDisplaySettings"),None);
        assert!(within(1024,1020,4));
        assert!(!within(1024,1020,5));
        assert!(!within(1024,usize::MAX,5));
    }
}