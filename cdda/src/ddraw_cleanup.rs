// SPDX-License-Identifier: GPL-3.0-or-later
//! Authenticated FR and US retail in-memory null-safe COM surface cleanup.
//! Windows issues #30 and #32: independent FR and USA minidumps both
//! confirm EIP 0x004023DF dereferencing EAX=0 (read address 0),
//! in the game's unconditional Unlock/FlipToGDISurface cleanup routine.
//! In both binaries, 0x00403508 is the return address of the same callsite,
//! and globals 0x00447294 (surface) / 0x0044728C (draw) are both zero.
//! The US and FR introduction branch offsets differ by six bytes.
//! Select a *separate*, strictly verified introduction signature for each
//! retail edition before redirecting the common original cleanup call.
//! The game calls that routine at 0x00403503, including before CreateSurface
//! succeeds. Redirect only this exact callsite: when both COM globals are
//! non-null, call the original function unmodified; when not initialized,
//! there is nothing to unlock/flip, so skip that cleanup call only.
//! No on-disk game changes, OS hooks, files, registry, or physical modes.
use std::ffi::{c_char,c_void};
use std::sync::atomic::{AtomicBool,AtomicU32,Ordering};

const BASE:usize=0x0040_0000;
const CALL_RVA:usize=0x3503;
const FUNCTION_RVA:usize=0x23d4;
const SURFACE_RVA:usize=0x47294;
const DDRAW_RVA:usize=0x4728c;
const FR_INTRO_RVA:usize=0x6df5;
const FR_OTHER_INTRO_RVA:usize=0x6e09;
const US_INTRO_RVA:usize=0x6dfb;
const US_OTHER_INTRO_RVA:usize=0x6e0f;
const INTRO_PLAY:[u8;2]=[0x90,0x90];
const INTRO_SKIP:[u8;2]=[0xeb,0x2f];
const FR_EDITION:u32=1;
const US_EDITION:u32=2;
const PAGE_EXECUTE_READWRITE:u32=0x40;
const ORIGINAL_CALL:[u8;5]=[0xe8,0xcc,0xee,0xff,0xff];
const FUNCTION_PREFIX:[u8;10]=[0x55,0x8b,0xec,0x6a,0x00,0xa1,0x94,0x72,0x44,0x00];
static INSTALLED:AtomicBool=AtomicBool::new(false);
static STATUS:AtomicU32=AtomicU32::new(0);
static REPORTED:AtomicBool=AtomicBool::new(false);
static EDITION:AtomicU32=AtomicU32::new(0);
static CALLS:AtomicU32=AtomicU32::new(0);
static SKIPPED:AtomicU32=AtomicU32::new(0);

#[link(name="kernel32")]
unsafe extern "system" {
    fn GetModuleHandleA(name:*const c_char)->*mut c_void;
    fn GetCurrentProcess()->*mut c_void;
    fn VirtualProtect(ptr:*mut c_void,size:usize,protect:u32,old:*mut u32)->i32;
    fn FlushInstructionCache(process:*mut c_void,base:*const c_void,size:usize)->i32;
}
fn fits(size:usize,offset:usize,len:usize)->bool{
    offset.checked_add(len).is_some_and(|end|end<=size)
}
unsafe fn same_bytes(at:usize,bytes:&[u8])->bool{
    bytes.iter().enumerate().all(|(i,&v)|((at+i) as *const u8).read()==v)
}
fn call_delta(next:u32,to:u32)->[u8;4]{to.wrapping_sub(next).to_le_bytes()}
fn should_skip_cleanup(surface:u32,draw:u32)->bool{surface==0 || draw==0}
fn edition_label(edition:u32)->&'static str{
    match edition{FR_EDITION=>"FR",US_EDITION=>"US",_=>"UNKNOWN"}
}
// Return one of the exact known retail profiles; skip-mode uses a different
// first branch and must never install this intro-only cleanup guard.
fn edition_for_intro(fr_first:&[u8;2],fr_other:&[u8;2],
                     us_first:&[u8;2],us_other:&[u8;2])->(u32,bool){
    let fr_other_ok=*fr_other==INTRO_PLAY;
    let us_other_ok=*us_other==INTRO_PLAY;
    let fr_intro=fr_other_ok && *fr_first==INTRO_PLAY;
    let us_intro=us_other_ok && *us_first==INTRO_PLAY;
    if fr_intro && !us_intro {return (FR_EDITION,true);}
    if us_intro && !fr_intro {return (US_EDITION,true);}
    let fr_skip=fr_other_ok && *fr_first==INTRO_SKIP;
    let us_skip=us_other_ok && *us_first==INTRO_SKIP;
    if fr_skip && !us_skip {return (FR_EDITION,false);}
    if us_skip && !fr_skip {return (US_EDITION,false);}
    // Fail closed for ambiguous or unknown code. Never assume edition.
    (0,false)
}
unsafe extern "C" fn guarded_cleanup(){
    CALLS.fetch_add(1,Ordering::Relaxed);
    // The game originally performs both operations unconditionally. A
    // missing COM surface implies Unlock would dereference 0x00000000.
    // Skip only this *uninitialized cleanup*; never synthesize a surface.
    let surface=((BASE+SURFACE_RVA) as *const u32).read_unaligned();
    let ddraw=((BASE+DDRAW_RVA) as *const u32).read_unaligned();
    if should_skip_cleanup(surface,ddraw){
        let n=SKIPPED.fetch_add(1,Ordering::Relaxed)+1;
        if n<=4 || n%100==0 {
            crate::log(&format!(
                "{}_COM_CLEANUP skipped_uninitialized=1 call={n} surface_null={} ddraw_null={} no_synthetic_surface=1",
                edition_label(EDITION.load(Ordering::Acquire)),
                u8::from(surface==0),u8::from(ddraw==0)
            ));
        }
        return;
    }
    let original:unsafe extern "C" fn()=std::mem::transmute(BASE+FUNCTION_RVA);
    original();
}
unsafe fn install()->bool{
    let base=GetModuleHandleA(std::ptr::null()) as usize;
    if base!=BASE || (base as *const u16).read_unaligned()!=0x5a4d {
        STATUS.store(1,Ordering::Release);return false;
    }
    let pe=(base+0x3c) as *const u32;
    let pe_offset=pe.read_unaligned() as usize;
    if !(0x40..=0x100000).contains(&pe_offset)
        || ((base+pe_offset) as *const u32).read_unaligned()!=0x4550 {
        STATUS.store(1,Ordering::Release);return false;
    }
    let opt=base+pe_offset+24;
    if (opt as *const u16).read_unaligned()!=0x10b {
        STATUS.store(1,Ordering::Release);return false;
    }
    let size=((opt+56) as *const u32).read_unaligned() as usize;
    if !(0x100000..=0x10000000).contains(&size)
        || !fits(size,CALL_RVA,ORIGINAL_CALL.len())
        || !fits(size,FUNCTION_RVA,FUNCTION_PREFIX.len())
        || !fits(size,FR_INTRO_RVA,2)||!fits(size,FR_OTHER_INTRO_RVA,2)
        || !fits(size,US_INTRO_RVA,2)||!fits(size,US_OTHER_INTRO_RVA,2)
        || !fits(size,SURFACE_RVA,4)||!fits(size,DDRAW_RVA,4){
        STATUS.store(1,Ordering::Release);return false;
    }
    let fr_first=[
        ((base+FR_INTRO_RVA) as *const u8).read(),
        ((base+FR_INTRO_RVA+1) as *const u8).read()
    ];
    let fr_other=[
        ((base+FR_OTHER_INTRO_RVA) as *const u8).read(),
        ((base+FR_OTHER_INTRO_RVA+1) as *const u8).read()
    ];
    let us_first=[
        ((base+US_INTRO_RVA) as *const u8).read(),
        ((base+US_INTRO_RVA+1) as *const u8).read()
    ];
    let us_other=[
        ((base+US_OTHER_INTRO_RVA) as *const u8).read(),
        ((base+US_OTHER_INTRO_RVA+1) as *const u8).read()
    ];
    let (edition,intro_enabled)=edition_for_intro(
        &fr_first,&fr_other,&us_first,&us_other
    );
    EDITION.store(edition,Ordering::Release);
    if !intro_enabled
        || !same_bytes(base+CALL_RVA,&ORIGINAL_CALL)
        || !same_bytes(base+FUNCTION_RVA,&FUNCTION_PREFIX){
        STATUS.store(2,Ordering::Release);return false;
    }
    // Both independently captured minidumps report this exact game callsite.
    // Repoint only if the edition-specific intro bytes and original cleanup
    // call/prologue are still identical. No other memory locations change.
    let target=(guarded_cleanup as usize) as u32;
    let delta=call_delta((base+CALL_RVA+5) as u32,target);
    let patched=[0xe8,delta[0],delta[1],delta[2],delta[3]];
    let addr=(base+CALL_RVA) as *mut u8;
    let mut protect=0;
    if VirtualProtect(addr.cast(),5,PAGE_EXECUTE_READWRITE,&mut protect)==0{
        STATUS.store(3,Ordering::Release);return false;
    }
    std::ptr::copy_nonoverlapping(patched.as_ptr(),addr,5);
    let _=FlushInstructionCache(GetCurrentProcess(),addr.cast(),5);
    let mut ignored=0;
    let _=VirtualProtect(addr.cast(),5,protect,&mut ignored);
    STATUS.store(4,Ordering::Release);
    true
}
pub(crate) unsafe extern "C" fn initialize(){
    if INSTALLED.load(Ordering::Acquire){return;}
    INSTALLED.store(install(),Ordering::Release);
}
#[used]
#[link_section=".CRT$XCU"]
static EARLY_COM_CLEANUP_GUARD:unsafe extern "C" fn()=initialize;

pub fn report_once(){
    if REPORTED.swap(true,Ordering::AcqRel){return;}
    let status=STATUS.load(Ordering::Acquire);
    crate::log(&format!(
        "{}_COM_CLEANUP_GUARD installed={} status={} total_calls={} skipped={} game_callsite=0x00403503 memory_only=1",
        edition_label(EDITION.load(Ordering::Acquire)),
        u8::from(INSTALLED.load(Ordering::Acquire)),status,
        CALLS.load(Ordering::Relaxed),SKIPPED.load(Ordering::Relaxed)
    ));
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn original_shared_callsite_targets_vulnerable_cleanup(){
        assert_eq!(ORIGINAL_CALL[0],0xe8);
        assert_eq!(&ORIGINAL_CALL[1..],&call_delta((BASE+CALL_RVA+5) as u32,(BASE+FUNCTION_RVA) as u32));
        assert_eq!(BASE+SURFACE_RVA,0x447294);
        assert_eq!(BASE+DDRAW_RVA,0x44728c);
    }
    #[test]
    fn both_pinned_retail_intro_profiles_are_supported_and_distinct(){
        assert_eq!(US_INTRO_RVA-FR_INTRO_RVA,6);
        assert_eq!(US_OTHER_INTRO_RVA-FR_OTHER_INTRO_RVA,6);
        // Retail FR and USA use identical cleanup target but distinct
        // exact intro branch addresses. Never select an ambiguous image.
        assert_eq!(edition_for_intro(&INTRO_PLAY,&INTRO_PLAY,&[0x74,0x2f],&[0x7c,0x0c]),(FR_EDITION,true));
        assert_eq!(edition_for_intro(&[0x74,0x2f],&[0x7c,0x0c],&INTRO_PLAY,&INTRO_PLAY),(US_EDITION,true));
        assert_eq!(edition_for_intro(&INTRO_SKIP,&INTRO_PLAY,&[0x74,0x2f],&[0x7c,0x0c]),(FR_EDITION,false));
        assert_eq!(edition_for_intro(&[0x74,0x2f],&[0x7c,0x0c],&INTRO_SKIP,&INTRO_PLAY),(US_EDITION,false));
        assert_eq!(edition_for_intro(&INTRO_PLAY,&INTRO_PLAY,&INTRO_PLAY,&INTRO_PLAY),(0,false));
        assert_eq!(edition_for_intro(&[0x74,0x2f],&[0x7c,0x0c],&[0x74,0x2f],&[0x7c,0x0c]),(0,false));
    }
    #[test]
    fn a_null_surface_or_draw_is_not_safe_for_unlock_or_flip(){
        assert!(should_skip_cleanup(0,0));
        assert!(should_skip_cleanup(1,0));
        assert!(should_skip_cleanup(0,1));
        assert!(!should_skip_cleanup(1,1));
    }
    #[test]
    fn PE_bounds_fail_closed(){
        assert!(fits(0x100000,0x3503,5));
        assert!(!fits(4,3,2));
        assert!(!fits(0x100000,usize::MAX,10));
    }
}
