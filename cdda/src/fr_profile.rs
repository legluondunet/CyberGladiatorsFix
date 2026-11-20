// SPDX-License-Identifier: GPL-3.0-or-later
//! Private, in-process compatibility for the authenticated French retail game's
//! Sierra profile lookup. No Windows-directory writes, registry changes or
//! modification of any system DLL. Intercepts only the game's PE32 import.
//!
//! WER issue #30: the original 74 2F JE at 0x406DF5 skips the introduction
//! when GetPrivateProfileStringA("Config","CDROM",...,"sierra.ini") is empty.
//! The previous NOP patch forced an unsafe path on a missing value. Preserve
//! the original branch and provide a game-local fallback *only* if the native
//! profile call returned zero. All other calls are forwarded byte-for-byte.
//!
//! This is a field-test hypothesis: the nonempty relative game path is
//! intended to permit the normal AVI. Do not equate unit tests with gameplay.
use std::ffi::{c_char,c_void,CStr};
use std::sync::atomic::{AtomicBool,AtomicU32,AtomicUsize,Ordering};

const IMAGE_BASE:usize=0x0040_0000;
const PAGE_READWRITE:u32=4;
static ORIGINAL:AtomicUsize=AtomicUsize::new(0);
static INSTALLED:AtomicBool=AtomicBool::new(false);
static REPORTED:AtomicBool=AtomicBool::new(false);
static CALLS:AtomicU32=AtomicU32::new(0);
static FALLBACKS:AtomicU32=AtomicU32::new(0);

#[link(name="kernel32")]
unsafe extern "system" {
    fn GetModuleHandleA(name:*const c_char)->*mut c_void;
    fn GetEnvironmentVariableA(name:*const c_char,value:*mut c_char,size:u32)->u32;
    fn VirtualProtect(address:*mut c_void,len:usize,protect:u32,previous:*mut u32)->i32;
}

unsafe fn read_u16(at:usize)->u16{(at as *const u16).read_unaligned()}
unsafe fn read_u32(at:usize)->u32{(at as *const u32).read_unaligned()}
fn fits(size:usize,offset:usize,len:usize)->bool{
    offset.checked_add(len).is_some_and(|end|end<=size)
}
unsafe fn matches_ascii(base:usize,size:usize,offset:usize,name:&[u8],fold:bool)->bool{
    if !fits(size,offset,name.len()+1){return false;}
    for (i,&expected) in name.iter().enumerate(){
        let found=((base+offset+i) as *const u8).read();
        if (if fold{found.to_ascii_lowercase()}else{found})!=expected{return false;}
    }
    ((base+offset+name.len()) as *const u8).read()==0
}
unsafe fn env_is_one(name:&[u8])->bool{
    let mut buf=[0i8;4];
    GetEnvironmentVariableA(name.as_ptr().cast(),buf.as_mut_ptr(),buf.len() as u32)==1
        && buf[0]==b'1' as i8
}
unsafe fn env_is_fr()->bool{
    let mut buf=[0i8;4];
    GetEnvironmentVariableA(
        b"CYBER_GAME_EDITION\0".as_ptr().cast(),buf.as_mut_ptr(),buf.len() as u32
    )==2&&buf[0]==b'F' as i8&&buf[1]==b'R' as i8
}
fn matching_profile_call(section:&[u8],key:&[u8],filename:&[u8])->bool{
    section.eq_ignore_ascii_case(b"Config")
        &&key.eq_ignore_ascii_case(b"CDROM")
        &&filename.eq_ignore_ascii_case(b"sierra.ini")
}
fn should_fallback(result:u32,buffer_len:u32,matching:bool)->bool{
    result==0 && matching && buffer_len>=3
}
unsafe extern "system" fn profile_lookup(
    section:*const c_char,key:*const c_char,default:*const c_char,
    output:*mut c_char,len:u32,filename:*const c_char
)->u32{
    let old=ORIGINAL.load(Ordering::Acquire);
    if old==0{return 0;}
    let native:unsafe extern "system" fn(
        *const c_char,*const c_char,*const c_char,*mut c_char,u32,*const c_char
    )->u32=std::mem::transmute(old);
    let result=native(section,key,default,output,len,filename);
    CALLS.fetch_add(1,Ordering::Relaxed);
    if output.is_null()||section.is_null()||key.is_null()||filename.is_null(){
        return result;
    }
    // The retail game's call uses known executable-owned static strings.
    // Match the exact Config/CDROM/sierra.ini tuple, never change other
    // process profile lookups or nonempty installer-provided CDROM values.
    let matching=matching_profile_call(
        CStr::from_ptr(section).to_bytes(),
        CStr::from_ptr(key).to_bytes(),
        CStr::from_ptr(filename).to_bytes()
    );
    if !should_fallback(result,len,matching){return result;}
    // The launcher sets the current directory to game/, which contains
    // splash4.avi. A relative game-root path is portable between Wine
    // and Windows and does not depend on a mounted original CD image.
    *output.add(0)=b'.' as c_char;
    *output.add(1)=b'\\' as c_char;
    *output.add(2)=0;
    FALLBACKS.fetch_add(1,Ordering::Relaxed);
    crate::log("FR_PROFILE_CDROM fallback=relative_game_directory original_return=0 no_global_ini_write=1");
    2
}
unsafe fn hook()->bool{
    let base=GetModuleHandleA(std::ptr::null()) as usize;
    if base!=IMAGE_BASE || read_u16(base)!=0x5a4d{return false;}
    let pe=read_u32(base+0x3c) as usize;
    if !(0x40..=0x100000).contains(&pe)||read_u32(base+pe)!=0x4550{return false;}
    let opt=base+pe+24;
    if read_u16(opt)!=0x10b{return false;}
    let size=read_u32(opt+56) as usize;
    if !(0x100000..=0x10000000).contains(&size){return false;}
    let imports=read_u32(opt+104) as usize;
    if !fits(size,imports,20){return false;}
    for i in 0..128usize{
        let descriptor=imports+i*20;
        if !fits(size,descriptor,20){break;}
        let source=read_u32(base+descriptor) as usize;
        let dll=read_u32(base+descriptor+12) as usize;
        let target=read_u32(base+descriptor+16) as usize;
        if source==0&&target==0{break;}
        if !matches_ascii(base,size,dll,b"kernel32.dll",true)
            &&!matches_ascii(base,size,dll,b"kernelbase.dll",true){continue;}
        if source==0||target==0{continue;}
        for n in 0..4096usize{
            let name_slot=source+n*4;
            let target_slot=target+n*4;
            if !fits(size,name_slot,4)||!fits(size,target_slot,4){break;}
            let ordinal_or_name=read_u32(base+name_slot);
            if ordinal_or_name==0{break;}
            if ordinal_or_name&0x8000_0000!=0{continue;}
            let Some(name)= (ordinal_or_name as usize).checked_add(2)else{continue};
            if !matches_ascii(base,size,name,b"GetPrivateProfileStringA",false){continue;}
            let slot=(base+target_slot) as *mut u32;
            let original=slot.read();
            if original==0 || original as usize==profile_lookup as usize {return false;}
            let mut previous=0;
            if VirtualProtect(slot.cast(),4,PAGE_READWRITE,&mut previous)==0{return false;}
            ORIGINAL.store(original as usize,Ordering::Release);
            slot.write_volatile(profile_lookup as usize as u32);
            let mut restored=0;
            let _=VirtualProtect(slot.cast(),4,previous,&mut restored);
            return true;
        }
    }
    false
}
unsafe extern "C" fn initialize(){
    if !env_is_one(b"CYBER_GAMEFRAME_TEST\0")
        || !env_is_fr()
        || !env_is_one(b"CYBER_FR_PROFILE_FALLBACK\0"){return;}
    INSTALLED.store(hook(),Ordering::Release);
}
// This stage runs before the retail EXE's first profile lookup. It does no
// allocations or file I/O under the loader lock, like the existing IAT hook.
#[used]
#[link_section=".CRT$XCU"]
static EARLY_PROFILE_HOOK:unsafe extern "C" fn()=initialize;

pub fn report_once(){
    if REPORTED.swap(true,Ordering::AcqRel){return;}
    if INSTALLED.load(Ordering::Acquire){
        crate::log(&format!(
            "FR_PROFILE_HOOK installed=1 calls={} fallbacks={} scope=exact_Config_CDROM_sierra.ini",
            CALLS.load(Ordering::Relaxed),FALLBACKS.load(Ordering::Relaxed)
        ));
    }else{
        crate::log("FR_PROFILE_HOOK installed=0 fallback=disabled_or_import_absent");
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn only_exact_legacy_game_profile_is_matched(){
        assert!(matching_profile_call(b"Config",b"CDROM",b"sierra.ini"));
        assert!(matching_profile_call(b"CONFIG",b"cdrom",b"SIERRA.INI"));
        assert!(!matching_profile_call(b"Video",b"CDROM",b"sierra.ini"));
        assert!(!matching_profile_call(b"Config",b"CDROM",b"other.ini"));
        assert!(!matching_profile_call(b"Config",b"Other",b"sierra.ini"));
    }
    #[test]
    fn only_empty_return_with_enough_room_gets_local_fallback(){
        assert!(should_fallback(0,3,true));
        assert!(!should_fallback(1,3,true));
        assert!(!should_fallback(0,2,true));
        assert!(!should_fallback(0,80,false));
    }
}
