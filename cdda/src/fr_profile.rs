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
//! Windows FR a73ff789 observed early CRT environment flags missing.
//! Identify the patched FR game from its unique PE import and code bytes
//! instead of relying on launcher-provided flags at CRT initialization.
use std::ffi::{c_char,c_void,CStr};
use std::sync::atomic::{AtomicBool,AtomicU32,AtomicUsize,Ordering};

const IMAGE_BASE:usize=0x0040_0000;
const PAGE_READWRITE:u32=4;
static ORIGINAL:AtomicUsize=AtomicUsize::new(0);
static INSTALLED:AtomicBool=AtomicBool::new(false);
// Loader-safe installation diagnostic: 0 = constructor not called;
// 3 = PE or code guard mismatch; 4 = import layout mismatch;
// 5 = unresolved IAT; 6 = VirtualProtect failed; 7 = hooked.
// Initialization no longer depends on environment variables.
static HOOK_STATE:AtomicU32=AtomicU32::new(0);
static REPORTED:AtomicBool=AtomicBool::new(false);
static CALLS:AtomicU32=AtomicU32::new(0);
static FALLBACKS:AtomicU32=AtomicU32::new(0);

#[link(name="kernel32")]
unsafe extern "system" {
    fn GetModuleHandleA(name:*const c_char)->*mut c_void;
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
// These RVA values were verified directly from the 20 November 2026
// authenticated French retail minidump, whose original backup is already
// fail-closed SHA-256-checked by the launcher. The generic import walk in
// the first candidate failed to attach (issue #30, log 3af698a9).
// Validate the full KERNEL32 import-descriptor / ILT / function name before
// writing exactly one game-owned IAT pointer. Never touch system DLLs.
const FR_IMPORT_DESCRIPTOR_RVA:usize=0x3ed000;
const FR_KERNEL32_ILT_RVA:u32=0x3ed08c;
const FR_KERNEL32_DLL_RVA:u32=0x3ed3f4;
const FR_KERNEL32_IAT_RVA:u32=0x3ed240;
const FR_PROFILE_NAME_POINTER_RVA:usize=0x3ed140;
const FR_PROFILE_NAME_RVA:u32=0x3ed754;
const FR_PROFILE_IAT_RVA:usize=0x3ed2f4;
fn profile_name_is_expected(raw_name:u32)->bool{raw_name==FR_PROFILE_NAME_RVA}
fn fr_intro_signature(first:u16,second:u16)->bool{
    first==0x2f74 && second==0x9090 // JE +0x2F, NOP NOP
}
unsafe fn hook()->bool{
    let base=GetModuleHandleA(std::ptr::null()) as usize;
    if base!=IMAGE_BASE || read_u16(base)!=0x5a4d {
        HOOK_STATE.store(3,Ordering::Release);return false;
    }
    let pe=read_u32(base+0x3c) as usize;
    if !(0x40..=0x100000).contains(&pe)||read_u32(base+pe)!=0x4550 {
        HOOK_STATE.store(3,Ordering::Release);return false;
    }
    let opt=base+pe+24;
    if read_u16(opt)!=0x10b {
        HOOK_STATE.store(3,Ordering::Release);return false;
    }
    let size=read_u32(opt+56) as usize;
    if !(FR_PROFILE_NAME_RVA as usize + 2 + b"GetPrivateProfileStringA".len() + 1..=0x10000000).contains(&size) {
        HOOK_STATE.store(3,Ordering::Release);return false;
    }
    // Only inspect bytes at the known FR intro branches AFTER validating
    // the PE image size so unrelated small executables fail closed.
    if !fits(size,0x6df5,2)||!fits(size,0x6e09,2)
        ||!fr_intro_signature(read_u16(base+0x6df5),read_u16(base+0x6e09)){
        HOOK_STATE.store(3,Ordering::Release);return false;
    }
    // Validate that this is the exact PE32 import descriptor layout we
    // observed in the known FR retail crash dump, not another game.
    let imports=read_u32(opt+104) as usize;
    let descriptor=FR_IMPORT_DESCRIPTOR_RVA;
    if imports!=descriptor
        || !fits(size,descriptor,20)
        || read_u32(base+descriptor)!=FR_KERNEL32_ILT_RVA
        || read_u32(base+descriptor+12)!=FR_KERNEL32_DLL_RVA
        || read_u32(base+descriptor+16)!=FR_KERNEL32_IAT_RVA
        || !matches_ascii(base,size,FR_KERNEL32_DLL_RVA as usize,b"kernel32.dll",true)
        || !fits(size,FR_PROFILE_NAME_POINTER_RVA,4)
        || !profile_name_is_expected(read_u32(base+FR_PROFILE_NAME_POINTER_RVA))
        || !matches_ascii(base,size,FR_PROFILE_NAME_RVA as usize+2,b"GetPrivateProfileStringA",false)
        || !fits(size,FR_PROFILE_IAT_RVA,4)
    {
        HOOK_STATE.store(4,Ordering::Release);return false;
    }
    let slot=(base+FR_PROFILE_IAT_RVA) as *mut u32;
    let original=slot.read();
    // If the image's imports have not yet been fixed up by the loader,
    // refuse to store a function address that is merely an image RVA.
    if original==0 || (base..base+size).contains(&(original as usize))
        || original as usize==profile_lookup as usize
    {
        HOOK_STATE.store(5,Ordering::Release);return false;
    }
    let mut old_protect=0;
    if VirtualProtect(slot.cast(),4,PAGE_READWRITE,&mut old_protect)==0 {
        HOOK_STATE.store(6,Ordering::Release);return false;
    }
    ORIGINAL.store(original as usize,Ordering::Release);
    slot.write_volatile(profile_lookup as usize as u32);
    let mut ignored=0;
    let _=VirtualProtect(slot.cast(),4,old_protect,&mut ignored);
    HOOK_STATE.store(7,Ordering::Release);
    true
}
pub(crate) unsafe extern "C" fn initialize(){
    // Windows FR reported missing env gating at CRT time. Restrict by
    // verified FR PE layout, import descriptors and guarded patched bytes.
    // US and arbitrary executables fail closed without file modification.
    if INSTALLED.load(Ordering::Acquire){return;}
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
        let reason=match HOOK_STATE.load(Ordering::Acquire){
            0=>"early_initializer_not_called",
            1=>"legacy_flag_missing_not_expected",
            2=>"legacy_edition_check_not_expected",
            3=>"invalid_pe_image",
            4=>"fr_import_signature_mismatch",
            5=>"unresolved_or_already_hooked_IAT",
            6=>"virtualprotect_failed",
            7=>"installed_but_flag_out_of_sync",
            _=>"unknown",
        };
        crate::log(&format!("FR_PROFILE_HOOK installed=0 reason={reason} init_stage={}",HOOK_STATE.load(Ordering::Acquire)));
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
    fn only_original_je_and_secondary_intro_nops_activate_shim(){
        assert!(fr_intro_signature(0x2f74,0x9090));
        assert!(!fr_intro_signature(0x9090,0x9090)); // legacy forced intro
        assert!(!fr_intro_signature(0x2feb,0x9090)); // SkipIntro=1
        assert!(!fr_intro_signature(0x2f74,0x0c7c)); // original retail
    }
    #[test]
    fn french_import_profile_thunk_matches_verified_dump_offsets(){
        // Verified 2026-11-20 French CYBER.EXE: KERNEL32 ILT entry 45.
        assert_eq!(FR_PROFILE_NAME_POINTER_RVA,FR_KERNEL32_ILT_RVA as usize+45*4);
        assert_eq!(FR_PROFILE_IAT_RVA,FR_KERNEL32_IAT_RVA as usize+45*4);
        assert_eq!(FR_PROFILE_NAME_RVA,0x3ed754);
        assert!(profile_name_is_expected(0x3ed754));
        assert!(!profile_name_is_expected(0x3ed756));
    }
    #[test]
    fn only_empty_return_with_enough_room_gets_local_fallback(){
        assert!(should_fallback(0,3,true));
        assert!(!should_fallback(1,3,true));
        assert!(!should_fallback(0,2,true));
        assert!(!should_fallback(0,80,false));
    }
}
