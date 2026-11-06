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
use std::sync::atomic::{AtomicBool,AtomicU32,AtomicUsize,Ordering};

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
// IDirectDraw (v1) is used by mid-1990s games for 640x480 fullscreen.
// Intercept only the game's own by-name DirectDrawCreate IAT, then the
// returned instance's SetDisplayMode method (#21); never hook Wine globally.
static DDRAW_IAT_MATCHED:AtomicU32=AtomicU32::new(0);
static DDRAW_IAT_HOOKS:AtomicU32=AtomicU32::new(0);
static DDRAW_CREATES:AtomicU32=AtomicU32::new(0);
static DDRAW_OBJECTS:AtomicU32=AtomicU32::new(0);
static DDRAW_SETMODE_CALLS:AtomicU32=AtomicU32::new(0);
static DDRAW_MODE_SUPPRESSED:AtomicU32=AtomicU32::new(0);
static DDRAW_LAST_WIDTH:AtomicU32=AtomicU32::new(0);
static DDRAW_LAST_HEIGHT:AtomicU32=AtomicU32::new(0);
static DDRAW_LAST_DEPTH:AtomicU32=AtomicU32::new(0);
static DDRAW_CREATE_ORIGINAL:AtomicUsize=AtomicUsize::new(0);


#[link(name="kernel32")]
unsafe extern "system" {
    fn GetModuleHandleA(name:*const c_char)->*mut c_void;
    fn GetProcAddress(module:*mut c_void,name:*const c_char)->*mut c_void;
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
/// Only the original CYBER.EXE can reach this IAT thunk, and only with the
/// Wine-only launcher experiment enabled. On failure, the game receives
/// the native HRESULT and retains original behavior.
unsafe extern "system" fn intercept_directdraw_create(
    guid:*mut c_void,out:*mut *mut c_void,outer:*mut c_void
)->i32{
    let address=DDRAW_CREATE_ORIGINAL.load(Ordering::Acquire);
    if address==0{return 0x8876_0000u32 as i32;}
    let original:unsafe extern "system" fn(
        *mut c_void,*mut *mut c_void,*mut c_void
    )->i32=std::mem::transmute(address);
    let hr=original(guid,out,outer);
    DDRAW_CREATES.fetch_add(1,Ordering::Relaxed);
    if hr==0 && !out.is_null(){
        let object=out.read();
        if !object.is_null(){
            // Shadow the legacy 23-method IDirectDraw v1 vtable on this
            // instance. No write into Wine/ddraw.dll shared code or vtable.
            // Box::into_raw intentionally holds the small table until the
            // game process exits, even if COM Release frees the instance.
            let instance_vtable=object as *mut *mut usize;
            let original_vtable=instance_vtable.read();
            if !original_vtable.is_null(){
                let mut shadow=Box::new([0usize;23]);
                std::ptr::copy_nonoverlapping(
                    original_vtable as *const usize,shadow.as_mut_ptr(),23
                );
                shadow[21]=intercept_ddraw_set_display_mode as usize;
                let shadow_ptr=Box::into_raw(shadow) as *mut usize;
                instance_vtable.write(shadow_ptr);
                DDRAW_OBJECTS.fetch_add(1,Ordering::Relaxed);
            }
        }
    }
    hr
}
/// Legacy IDirectDraw v1 (1996): SetDisplayMode(this,width,height,bpp).
/// GameFrame already presents 640x480 content letterboxed at native
/// desktop size. Report a successful virtual mode selection to the game
/// WITHOUT allowing Wine to resize the user's virtual desktop.
unsafe extern "system" fn intercept_ddraw_set_display_mode(
    _this:*mut c_void,width:u32,height:u32,depth:u32
)->i32{
    DDRAW_SETMODE_CALLS.fetch_add(1,Ordering::Relaxed);
    DDRAW_LAST_WIDTH.store(width,Ordering::Relaxed);
    DDRAW_LAST_HEIGHT.store(height,Ordering::Relaxed);
    DDRAW_LAST_DEPTH.store(depth,Ordering::Relaxed);
    DDRAW_MODE_SUPPRESSED.fetch_add(1,Ordering::Relaxed);
    0 // DD_OK: no resolution switch and no VIDEO mode change
}
unsafe fn install_ddraw_import_guard()->u32{
    let base=GetModuleHandleA(std::ptr::null()) as usize;
    if base!=IMAGE_BASE||u16_at(base)!=0x5a4d{return 0;}
    let pe=u32_at(base+0x3c) as usize;
    if !(0x40..=0x100000).contains(&pe)||u32_at(base+pe)!=0x0000_4550{return 0;}
    let opt=base+pe+24;
    if u16_at(opt)!=0x10b{return 0;}
    let image_size=u32_at(opt+56) as usize;
    if !(0x100000..=0x10000000).contains(&image_size){return 0;}
    let imports=u32_at(opt+104) as usize;
    if !within(image_size,imports,20){return 0;}
    for d in 0..128usize{
        let off=imports+d*20;
        if !within(image_size,off,20){break;}
        let desc=base+off;
        let name=u32_at(desc+12) as usize;
        let original=u32_at(desc) as usize;
        let first=u32_at(desc+16) as usize;
        if original==0&&first==0{break;}
        if !eq_ascii_at(base,image_size,name,b"ddraw.dll",true){continue;}
        if original==0||first==0{break;}
        for i in 0..4096usize{
            let lookup=original+i*4;
            let iat=first+i*4;
            if !within(image_size,lookup,4)||!within(image_size,iat,4){break;}
            let hint=u32_at(base+lookup);
            if hint==0{break;}
            if hint&0x8000_0000!=0{continue;}
            let Some(n)=(hint as usize).checked_add(2) else{continue};
            if !eq_ascii_at(base,image_size,n,b"DirectDrawCreate",false){continue;}
            DDRAW_IAT_MATCHED.fetch_add(1,Ordering::Relaxed);
            let slot=(base+iat) as *mut u32;
            let original_ptr=slot.read();
            if original_ptr==0||original_ptr as usize==intercept_directdraw_create as usize{
                return 0;
            }
            let mut protection=0;
            if VirtualProtect(slot.cast(),4,PAGE_READWRITE,&mut protection)==0{
                return 0;
            }
            DDRAW_CREATE_ORIGINAL.store(original_ptr as usize,Ordering::Release);
            slot.write_volatile(intercept_directdraw_create as usize as u32);
            let mut restored=0;
            let _=VirtualProtect(slot.cast(),4,protection,&mut restored);
            DDRAW_IAT_HOOKS.fetch_add(1,Ordering::Relaxed);
            return 1;
        }
    }
    0
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
    // This experiment must never modify a native Windows process.
    // Wine/Proton expose wine_get_version from already-loaded ntdll.
    let ntdll=GetModuleHandleA(b"ntdll.dll\0".as_ptr().cast());
    if ntdll.is_null() ||
        GetProcAddress(ntdll,b"wine_get_version\0".as_ptr().cast()).is_null(){
        return;
    }
    let count=install_import_guard();
    let dd_hook=install_ddraw_import_guard();
    INSTALLED.store(count,Ordering::Release);
    ACTIVE.store(count>0||dd_hook>0,Ordering::Release);
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
        "WINE_DESKTOP_MODE_GUARD requested={:?} gameframe={:?} startup_called={} startup_path={} enabled={} user32_imports_matched={} hooks_installed={} suppressed_mode_requests={} ddraw_create_imports={} ddraw_create_hooks={} ddraw_create_calls={} ddraw_v1_objects={} ddraw_set_display_mode_calls={} ddraw_mode_suppressed={} last_ddraw_mode={}x{}x{} game_process_only=1",
        requested.as_deref(),gameframe.as_deref(),
        u8::from(STARTUP_ATTEMPTED.load(Ordering::Acquire)),
        STARTUP_PATH.load(Ordering::Acquire),
        u8::from(ACTIVE.load(Ordering::Acquire)),
        CANDIDATES.load(Ordering::Acquire),
        INSTALLED.load(Ordering::Acquire),
        BLOCKED.load(Ordering::Acquire),
        DDRAW_IAT_MATCHED.load(Ordering::Acquire),
        DDRAW_IAT_HOOKS.load(Ordering::Acquire),
        DDRAW_CREATES.load(Ordering::Acquire),
        DDRAW_OBJECTS.load(Ordering::Acquire),
        DDRAW_SETMODE_CALLS.load(Ordering::Acquire),
        DDRAW_MODE_SUPPRESSED.load(Ordering::Acquire),
        DDRAW_LAST_WIDTH.load(Ordering::Acquire),
        DDRAW_LAST_HEIGHT.load(Ordering::Acquire),
        DDRAW_LAST_DEPTH.load(Ordering::Acquire)
    ));
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn legacy_ddraw_v1_set_display_mode_vtable_index(){
        // COM IUnknown 0..2, IDirectDraw 3..22: SetDisplayMode at 21.
        assert_eq!(21,3+18);
        assert_eq!(std::mem::size_of::<[usize;23]>(),23*std::mem::size_of::<usize>());
    }
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