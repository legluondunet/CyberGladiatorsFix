// SPDX-License-Identifier: GPL-3.0-or-later
//! Authenticated FR retail-only in-memory null-safe COM surface cleanup.
//! Native Windows issue #30: crash dump EIP 0x004023DF dereferences EAX=0
//! in the game's unconditional Unlock/FlipToGDISurface cleanup routine.
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
const JE_RVA:usize=0x6df5;
const OTHER_INTRO_RVA:usize=0x6e09;
const PAGE_EXECUTE_READWRITE:u32=0x40;
const ORIGINAL_CALL:[u8;5]=[0xe8,0xcc,0xee,0xff,0xff];
const FUNCTION_PREFIX:[u8;10]=[0x55,0x8b,0xec,0x6a,0x00,0xa1,0x94,0x72,0x44,0x00];
static INSTALLED:AtomicBool=AtomicBool::new(false);
static STATUS:AtomicU32=AtomicU32::new(0);
static REPORTED:AtomicBool=AtomicBool::new(false);
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
unsafe extern "C" fn guarded_cleanup(){
    CALLS.fetch_add(1,Ordering::Relaxed);
    // The game originally performs both operations unconditionally. A
    // missing COM surface implies Unlock would dereference 0x00000000.
    // Skip only this *uninitialized cleanup*; never synthesize a surface.
    let surface=((BASE+SURFACE_RVA) as *const u32).read_unaligned();
    let ddraw=((BASE+DDRAW_RVA) as *const u32).read_unaligned();
    if surface==0 || ddraw==0{
        let n=SKIPPED.fetch_add(1,Ordering::Relaxed)+1;
        if n<=4 || n%100==0 {
            crate::log(&format!(
                "FR_COM_CLEANUP skipped_uninitialized=1 call={n} surface_null={} ddraw_null={} no_synthetic_surface=1",
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
        || !fits(size,JE_RVA,2)||!fits(size,OTHER_INTRO_RVA,2)
        || !fits(size,SURFACE_RVA,4)||!fits(size,DDRAW_RVA,4){
        STATUS.store(1,Ordering::Release);return false;
    }
    if !same_bytes(base+CALL_RVA,&ORIGINAL_CALL)
        || !same_bytes(base+FUNCTION_RVA,&FUNCTION_PREFIX)
        || !same_bytes(base+JE_RVA,&[0x90,0x90])
        || !same_bytes(base+OTHER_INTRO_RVA,&[0x90,0x90]){
        STATUS.store(2,Ordering::Release);return false;
    }
    // The game code is not executing yet. Repoint a *single*, fixed FR
    // relative CALL from its vulnerable cleanup to our checked wrapper.
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
        "FR_COM_CLEANUP_GUARD installed={} status={} total_calls={} skipped={} game_callsite=0x00403503 memory_only=1",
        u8::from(INSTALLED.load(Ordering::Acquire)),status,
        CALLS.load(Ordering::Relaxed),SKIPPED.load(Ordering::Relaxed)
    ));
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn original_french_callsite_targets_vulnerable_cleanup(){
        assert_eq!(ORIGINAL_CALL[0],0xe8);
        assert_eq!(&ORIGINAL_CALL[1..],&call_delta((BASE+CALL_RVA+5) as u32,(BASE+FUNCTION_RVA) as u32));
        assert_eq!(BASE+SURFACE_RVA,0x447294);
        assert_eq!(BASE+DDRAW_RVA,0x44728c);
    }
    #[test]
    fn PE_bounds_fail_closed(){
        assert!(fits(0x100000,0x3503,5));
        assert!(!fits(4,3,2));
        assert!(!fits(0x100000,usize::MAX,10));
    }
}
