// SPDX-License-Identifier: GPL-3.0-or-later
//! Wine-only DirectDraw v1 display-size compatibility for the verified game.
//!
//! The verified Cyber Gladiators retail game requests fullscreen exclusive
//! DirectDraw 640x480x16 twice before opening its native MCI AVI. Wine changes
//! its desktop from the launcher's captured native dimensions, clipping the
//! deliberately fullscreen 4:3 native video host.
//!
//! Do NOT return DD_OK without calling DirectDraw: that produced a black
//! screen in CI build 83 (DirectDraw state was not initialized).
//! Instead forward the *real* IDirectDraw v1::SetDisplayMode with unchanged
//! 16-bit depth but the desktop's prelaunch WIDTH and HEIGHT. This lets Wine
//! initialize DirectDraw state while keeping the native desktop dimensions.
//! This does NOT promise no internal color-depth transition; it only prevents
//! a 640x480 desktop-size request. Never used on native Windows or without
//! the validated GameFrame launch path.
//!
//! The only PE patch is to the exact CYBER.EXE ddraw.dll DirectDrawCreate IAT
//! thunk, in memory, before game startup. The returned single COM object's
//! vtable is privately copied. No Wine DLLs, shared vtables, original game
//! files or actual desktop registry settings are modified.
//! CRT startup routine is loader-safe: no heap, file I/O, TLS, or COM calls.
use std::ffi::{c_char,c_void};
use std::sync::atomic::{AtomicBool,AtomicU32,AtomicUsize,Ordering};

const IMAGE_BASE:usize=0x0040_0000;
const PAGE_READWRITE:u32=4;
const SET_DISPLAY_MODE_INDEX:usize=21; // IDirectDraw v1, 23 methods.
const VTABLE_ENTRIES:usize=23;
// Preserve the validated single-object behavior. If a different executable
// or broken COM provider repeatedly returns new objects, do not allocate
// unbounded shadow tables. Excess objects keep their original DirectDraw vtable.
const MAX_VTABLE_SHADOWS:u32=8;
static SHADOW_LIMIT_REACHED:AtomicU32=AtomicU32::new(0);
static INSTALLED:AtomicBool=AtomicBool::new(false);
static INITIALIZED:AtomicBool=AtomicBool::new(false);
static IMPORT_FOUND:AtomicBool=AtomicBool::new(false);
static ENABLED:AtomicBool=AtomicBool::new(false);
static NATIVE_VIRTUAL:AtomicBool=AtomicBool::new(false);
static VIRTUAL_OBJECTS:AtomicU32=AtomicU32::new(0);
static NATIVE_W:AtomicU32=AtomicU32::new(0);
static NATIVE_H:AtomicU32=AtomicU32::new(0);
static CREATE_FN:AtomicUsize=AtomicUsize::new(0);
static SETMODE_FN:AtomicUsize=AtomicUsize::new(0);
static CREATE_CALLS:AtomicU32=AtomicU32::new(0);
static VTABLE_SHADOWS:AtomicU32=AtomicU32::new(0);
static MODE_CALLS:AtomicU32=AtomicU32::new(0);
static MODES_FORWARDED:AtomicU32=AtomicU32::new(0);
static REPORTED:AtomicBool=AtomicBool::new(false);

unsafe extern "C" {
    // Compiled from canonical Microsoft DirectDraw v1 C++ definitions.
    // This allocates an owned COM object; never edits native vtables.
    fn cyber_wrap_existing_ddraw(out:*mut *mut c_void)->i32;
}
#[link(name="kernel32")]
unsafe extern "system" {
    fn GetModuleHandleA(name:*const c_char)->*mut c_void;
    fn GetProcAddress(module:*mut c_void,name:*const c_char)->*mut c_void;
    fn GetEnvironmentVariableA(name:*const c_char,value:*mut c_char,size:u32)->u32;
    fn VirtualProtect(address:*mut c_void,len:usize,protect:u32,previous:*mut u32)->i32;
}
#[link(name="user32")]
unsafe extern "system" {
    fn GetSystemMetrics(index:i32)->i32;
}
#[inline]
unsafe fn read_u16(at:usize)->u16{(at as *const u16).read_unaligned()}
#[inline]
unsafe fn read_u32(at:usize)->u32{(at as *const u32).read_unaligned()}
#[inline]
fn fits(size:usize,offset:usize,len:usize)->bool{
    offset.checked_add(len).is_some_and(|end|end<=size)
}
unsafe fn matches_ascii(base:usize,size:usize,offset:usize,name:&[u8],fold:bool)->bool{
    if !fits(size,offset,name.len()+1){return false;}
    for (n,&wanted) in name.iter().enumerate(){
        let got=((base+offset+n) as *const u8).read();
        if (if fold{got.to_ascii_lowercase()}else{got})!=wanted{return false;}
    }
    ((base+offset+name.len()) as *const u8).read()==0
}
unsafe fn env_is_one(key:&[u8])->bool{
    let mut out=[0i8;4];
    GetEnvironmentVariableA(key.as_ptr().cast(),out.as_mut_ptr(),out.len() as u32)==1
        && out[0]==b'1' as i8
}
unsafe fn env_is_fr()->bool{
    let mut buf=[0i8;5];
    let n=GetEnvironmentVariableA(
        b"CYBER_GAME_EDITION\0".as_ptr().cast(),
        buf.as_mut_ptr(),buf.len() as u32
    );
    n==2 && buf[0]==b'F' as i8 &&buf[1]==b'R' as i8
}
fn parse_native_desktop(input:&str)->Option<(u32,u32)>{
    let (w,h)=input.split_once('x')?;
    let (w,h)=(w.parse::<u32>().ok()?,h.parse::<u32>().ok()?);
    ((640..=16384).contains(&w)&&(480..=16384).contains(&h)).then_some((w,h))
}
#[inline]
fn requested_mode_to_native(
    width:u32,height:u32,bpp:u32,native_w:u32,native_h:u32
)->(u32,u32,bool){
    // Never alter color depth, refresh rate, or any unsupported mode.
    // Preserve the exact validated 640x480x16 -> native-size behavior.
    let replace=width==640&&height==480&&bpp==16
        &&native_w>=640&&native_h>=480;
    if replace{(native_w,native_h,true)}
    else{(width,height,false)}
}
unsafe fn captured_desktop()->Option<(u32,u32)>{
    let mut buf=[0i8;48];
    let size=GetEnvironmentVariableA(
        b"CYBER_GAMEFRAME_DESKTOP_SIZE\0".as_ptr().cast(),
        buf.as_mut_ptr(),buf.len() as u32
    ) as usize;
    if size==0||size>=buf.len(){return None;}
    let bytes=std::slice::from_raw_parts(buf.as_ptr() as *const u8,size);
    parse_native_desktop(std::str::from_utf8(bytes).ok()?)
}
unsafe fn hook_ddraw_create()->bool{
    let base=GetModuleHandleA(std::ptr::null()) as usize;
    if base!=IMAGE_BASE||read_u16(base)!=0x5a4d{return false;}
    let pe=read_u32(base+0x3c) as usize;
    if !(0x40..=0x100000).contains(&pe)||read_u32(base+pe)!=0x4550{return false;}
    let opt=base+pe+24;
    if read_u16(opt)!=0x10b{return false;} // retail PE32
    let image_size=read_u32(opt+56) as usize;
    if !(0x100000..=0x10000000).contains(&image_size){return false;}
    let directory=read_u32(opt+104) as usize;
    if !fits(image_size,directory,20){return false;}
    for d in 0..128usize{
        let off=directory+d*20;
        if !fits(image_size,off,20){break;}
        let desc=base+off;
        let oft=read_u32(desc) as usize;
        let dll=read_u32(desc+12) as usize;
        let ft=read_u32(desc+16) as usize;
        if oft==0&&ft==0{break;}
        if !matches_ascii(base,image_size,dll,b"ddraw.dll",true){continue;}
        if oft==0||ft==0{return false;}
        for t in 0..4096usize{
            let lookup=oft+t*4;
            let thunk=ft+t*4;
            if !fits(image_size,lookup,4)||!fits(image_size,thunk,4){break;}
            let item=read_u32(base+lookup);
            if item==0{break;}
            if item&0x80000000!=0{continue;}
            let Some(n)=(item as usize).checked_add(2)else{continue};
            if !matches_ascii(base,image_size,n,b"DirectDrawCreate",false){continue;}
            IMPORT_FOUND.store(true,Ordering::Release);
            let slot=(base+thunk) as *mut u32;
            let original=slot.read();
            if original==0||original as usize==forward_ddraw_create as usize{return false;}
            let mut old=0;
            if VirtualProtect(slot.cast(),4,PAGE_READWRITE,&mut old)==0{return false;}
            CREATE_FN.store(original as usize,Ordering::Release);
            slot.write_volatile(forward_ddraw_create as usize as u32);
            let mut restored=0;
            let _=VirtualProtect(slot.cast(),4,old,&mut restored);
            return true;
        }
        return false;
    }
    false
}
#[used]
#[link_section=".CRT$XCU"]
static EARLY_FORWARD_INIT:unsafe extern "C" fn()=initialize;

unsafe extern "C" fn initialize(){
    INITIALIZED.store(true,Ordering::Release);
    if !env_is_one(b"CYBER_GAMEFRAME_TEST\0"){return;}
    let ntdll=GetModuleHandleA(b"ntdll.dll\0".as_ptr().cast());
    let wine=!ntdll.is_null()
        &&!GetProcAddress(ntdll,b"wine_get_version\0".as_ptr().cast()).is_null();
    let native_fr_virtual=!wine
        &&env_is_one(b"CYBER_FR_VIRTUAL_DDRAW\0")
        &&env_is_fr();
    if native_fr_virtual {
        // Opt-in and FR only. Without the exact feature flag, native
        // Windows executes the unchanged, previously validated path.
        NATIVE_VIRTUAL.store(true,Ordering::Release);
        INSTALLED.store(hook_ddraw_create(),Ordering::Release);
        return;
    }
    if !wine||!env_is_one(b"CYBER_WINE_DDRAW_NATIVE_FORWARD\0"){return;}
    let Some((w,h))=captured_desktop()else{return;};
    NATIVE_W.store(w,Ordering::Release);
    NATIVE_H.store(h,Ordering::Release);
    ENABLED.store(true,Ordering::Release);
    INSTALLED.store(hook_ddraw_create(),Ordering::Release);
}
unsafe extern "system" fn forward_ddraw_create(
    guid:*mut c_void,out:*mut *mut c_void,outer:*mut c_void
)->i32{
    let addr=CREATE_FN.load(Ordering::Acquire);
    if addr==0{return 0x88760000u32 as i32;}
    let original:unsafe extern "system" fn(
        *mut c_void,*mut *mut c_void,*mut c_void
    )->i32=std::mem::transmute(addr);
    let hr=original(guid,out,outer);
    CREATE_CALLS.fetch_add(1,Ordering::Relaxed);
    if NATIVE_VIRTUAL.load(Ordering::Acquire) {
        if hr!=0{return hr;}
        if out.is_null() || out.read().is_null(){return 0x88760000u32 as i32;}
        let wrapped=cyber_wrap_existing_ddraw(out);
        if wrapped==0 {VIRTUAL_OBJECTS.fetch_add(1,Ordering::Relaxed);}
        super::log(&format!(
            "DDRAW_VIRTUAL_CREATE native_FR=1 native_vtable_modified=0 wrapper_owned=1 original_hr={hr:#010x} wrapped_hr={wrapped:#010x}"
        ));
        return wrapped;
    }
    let mut shadowed=false;
    if hr==0&&!out.is_null(){
        let instance=out.read();
        if !instance.is_null(){
            let vtable_slot=instance as *mut *mut usize;
            let old_vtable=vtable_slot.read();
            if !old_vtable.is_null(){
                let original_setmode=old_vtable.add(SET_DISPLAY_MODE_INDEX).read();
                if original_setmode!=0
                    && original_setmode!=forward_ddraw_setmode as usize{
                    let stored=SETMODE_FN.load(Ordering::Acquire);
                    if stored==0{
                        let _=SETMODE_FN.compare_exchange(
                            0,original_setmode,Ordering::AcqRel,Ordering::Acquire
                        );
                    }
                    // Fail open if another COM class uses a distinct method.
                    // Shadow tables are kept alive until process exit because
                    // the COM object may retain its vtable past this function.
                    if SETMODE_FN.load(Ordering::Acquire)==original_setmode{
                        let reserved=VTABLE_SHADOWS.fetch_update(
                            Ordering::AcqRel,Ordering::Acquire,
                            |n|(n<MAX_VTABLE_SHADOWS).then_some(n+1)
                        ).is_ok();
                        if reserved{
                            let mut clone=Box::new([0usize;VTABLE_ENTRIES]);
                            std::ptr::copy_nonoverlapping(
                                old_vtable,clone.as_mut_ptr(),VTABLE_ENTRIES
                            );
                            clone[SET_DISPLAY_MODE_INDEX]=forward_ddraw_setmode as usize;
                            let replacement=Box::into_raw(clone) as *mut usize;
                            vtable_slot.write(replacement);
                            shadowed=true;
                        }else{
                            SHADOW_LIMIT_REACHED.fetch_add(1,Ordering::Relaxed);
                        }
                    }
                }
            }
        }
    }
    super::log(&format!(
        "DDRAW_NATIVE_CREATE hr={hr:#010x} game_ddraw_create=1 vtable_shadowed={} native_desktop={}x{} vtable_shadow_limit={MAX_VTABLE_SHADOWS}",
        u8::from(shadowed),NATIVE_W.load(Ordering::Acquire),NATIVE_H.load(Ordering::Acquire)
    ));
    hr
}
unsafe extern "system" fn forward_ddraw_setmode(
    this:*mut c_void,width:u32,height:u32,bpp:u32
)->i32{
    MODE_CALLS.fetch_add(1,Ordering::Relaxed);
    let native_w=NATIVE_W.load(Ordering::Acquire);
    let native_h=NATIVE_H.load(Ordering::Acquire);
    let (target_w,target_h,intercept)=
        requested_mode_to_native(width,height,bpp,native_w,native_h);
    let original_ptr=SETMODE_FN.load(Ordering::Acquire);
    if original_ptr==0{return 0x88760000u32 as i32;}
    let original:unsafe extern "system" fn(*mut c_void,u32,u32,u32)->i32=
        std::mem::transmute(original_ptr);
    let before=(GetSystemMetrics(0),GetSystemMetrics(1));
    // IMPORTANT: call real Wine DirectDraw! Never return fake DD_OK.
    let hr=original(this,target_w,target_h,bpp);
    let after=(GetSystemMetrics(0),GetSystemMetrics(1));
    if intercept{
        MODES_FORWARDED.fetch_add(1,Ordering::Relaxed);
    }
    super::log(&format!(
        "DDRAW_NATIVE_SETMODE requested={}x{}x{} forwarded={}x{}x{} native_desktop={}x{} monitor_before={}x{} monitor_after={}x{} original_hr={:#010x} native_dimensions_requested={} not_a_noop=1",
        width,height,bpp,target_w,target_h,bpp,native_w,native_h,
        before.0,before.1,after.0,after.1,hr,u8::from(intercept)
    ));
    hr
}
pub fn report_once(){
    if REPORTED.swap(true,Ordering::AcqRel){return;}
    super::log(&format!(
        "DDRAW_NATIVE_SUMMARY initialized={} enabled={} native_FR_virtual={} virtual_objects={} import_found={} import_hooked={} DirectDrawCreate_calls={} vtables_shadowed={} SetDisplayMode_calls={} native_mode_forwarded={} native_desktop={}x{} shadow_limit_reached={}",
        u8::from(INITIALIZED.load(Ordering::Acquire)),
        u8::from(ENABLED.load(Ordering::Acquire)),
        u8::from(NATIVE_VIRTUAL.load(Ordering::Acquire)),
        VIRTUAL_OBJECTS.load(Ordering::Acquire),
        u8::from(IMPORT_FOUND.load(Ordering::Acquire)),
        u8::from(INSTALLED.load(Ordering::Acquire)),
        CREATE_CALLS.load(Ordering::Acquire),
        VTABLE_SHADOWS.load(Ordering::Acquire),
        MODE_CALLS.load(Ordering::Acquire),
        MODES_FORWARDED.load(Ordering::Acquire),
        NATIVE_W.load(Ordering::Acquire),
        NATIVE_H.load(Ordering::Acquire),
        SHADOW_LIMIT_REACHED.load(Ordering::Acquire)
    ));
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn desktop_geometry_requires_valid_native_dimensions(){
        assert_eq!(parse_native_desktop("1920x1080"),Some((1920,1080)));
        assert_eq!(parse_native_desktop("2560x1440"),Some((2560,1440)));
        assert_eq!(parse_native_desktop("640x480"),Some((640,480)));
        assert_eq!(parse_native_desktop("640x0"),None);
        assert_eq!(parse_native_desktop("garbage"),None);
    }
    #[test]
    fn only_rewrites_exact_legacy_640x480x16_to_captured_desktop(){
        assert_eq!(requested_mode_to_native(640,480,16,1920,1080),(1920,1080,true));
        assert_eq!(requested_mode_to_native(640,480,16,2560,1600),(2560,1600,true));
        assert_eq!(requested_mode_to_native(640,480,16,3840,2160),(3840,2160,true));
        assert_eq!(requested_mode_to_native(640,480,16,640,480),(640,480,true));
    }
    #[test]
    fn forwards_other_directdraw_requests_unchanged(){
        assert_eq!(requested_mode_to_native(640,480,32,1920,1080),(640,480,false));
        assert_eq!(requested_mode_to_native(800,600,16,1920,1080),(800,600,false));
        assert_eq!(requested_mode_to_native(640,480,8,1920,1080),(640,480,false));
        assert_eq!(requested_mode_to_native(640,480,16,0,0),(640,480,false));
        assert_eq!(requested_mode_to_native(640,480,16,320,240),(640,480,false));
    }
    #[test]
    fn shadow_allocation_is_bounded(){
        assert_eq!(MAX_VTABLE_SHADOWS,8);
        assert_eq!(VTABLE_ENTRIES,23);
    }
    #[test]
    fn first_directdraw_vtable_has_23_entries_and_set_mode_at_21(){
        assert_eq!(SET_DISPLAY_MODE_INDEX,21);
        assert_eq!(VTABLE_ENTRIES,23);
    }
}
