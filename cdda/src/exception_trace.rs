// SPDX-License-Identifier: GPL-3.0-or-later
//! Trace-only first-chance exception observer for the verified x86 game.
//! A stack-overflow callback has little stack remaining: it MUST NOT format,
//! allocate, lock, or write files. It only records registers atomically, then
//! returns CONTINUE_SEARCH. The existing independent sampler resolves and
//! logs the captured address later on its own healthy thread.
use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32,AtomicUsize,Ordering};

#[repr(C)]
struct ExceptionRecord {
    code:u32,
    flags:u32,
    nested:*mut ExceptionRecord,
    address:*mut c_void,
    parameter_count:u32,
    information:[usize;15],
}
#[repr(C)]
struct ExceptionPointers {
    record:*mut ExceptionRecord,
    context:*mut c_void,
}
const CONTINUE_SEARCH:i32=0;
const ACCESS_VIOLATION:u32=0xc000_0005;
const STACK_OVERFLOW:u32=0xc000_00fd;
// Offsets in the documented 32-bit Windows CONTEXT (x86), not x64.
// EBP=0xB4, EIP=0xB8, ESP=0xC4. This DLL is built for i686 only.
const EBP_OFFSET:usize=0xb4;
const EIP_OFFSET:usize=0xb8;
const ESP_OFFSET:usize=0xc4;
const FROM_ADDRESS:u32=0x0000_0004;
const UNCHANGED_REFCOUNT:u32=0x0000_0002;

static INSTALLED:OnceLock<bool>=OnceLock::new();
static SEQUENCE:AtomicU32=AtomicU32::new(0);
static REPORTED:AtomicU32=AtomicU32::new(0);
static CODE:AtomicU32=AtomicU32::new(0);
static THREAD:AtomicU32=AtomicU32::new(0);
static INSTRUCTION:AtomicUsize=AtomicUsize::new(0);
static CONTEXT_EIP:AtomicU32=AtomicU32::new(0);
static CONTEXT_ESP:AtomicU32=AtomicU32::new(0);
static CONTEXT_EBP:AtomicU32=AtomicU32::new(0);
static ACCESS_TYPE:AtomicUsize=AtomicUsize::new(0);
static ACCESS_ADDRESS:AtomicUsize=AtomicUsize::new(0);

#[link(name="kernel32")]
extern "system" {
    fn AddVectoredExceptionHandler(
        first:u32,
        handler:Option<unsafe extern "system" fn(*mut ExceptionPointers)->i32>
    )->*mut c_void;
    fn GetCurrentThreadId()->u32;
    fn GetLastError()->u32;
    fn GetModuleHandleExA(flags:u32,name:*const u8,module:*mut *mut c_void)->i32;
    fn GetModuleFileNameA(module:*mut c_void,name:*mut u8,size:u32)->u32;
}
fn interesting(code:u32)->bool {
    matches!(code,0xc000_0005|0xc000_00fd|0xc000_001d|0xc000_0094|0xc000_0409)
}
unsafe extern "system" fn observe(pointers:*mut ExceptionPointers)->i32 {
    if pointers.is_null(){return CONTINUE_SEARCH;}
    let record=unsafe{(*pointers).record};
    if record.is_null(){return CONTINUE_SEARCH;}
    let error=unsafe{&*record};
    if !interesting(error.code){return CONTINUE_SEARCH;}
    // Allow a stack overflow to supersede a previous, possibly benign
    // first-chance access violation; otherwise capture the first event only.
    if SEQUENCE.load(Ordering::Acquire)!=0 && error.code!=STACK_OVERFLOW {
        return CONTINUE_SEARCH;
    }
    CODE.store(error.code,Ordering::Relaxed);
    THREAD.store(unsafe{GetCurrentThreadId()},Ordering::Relaxed);
    INSTRUCTION.store(error.address as usize,Ordering::Relaxed);
    ACCESS_TYPE.store(
        if error.code==ACCESS_VIOLATION && error.parameter_count>=2 {
            error.information[0]
        }else{usize::MAX},
        Ordering::Relaxed
    );
    ACCESS_ADDRESS.store(
        if error.code==ACCESS_VIOLATION && error.parameter_count>=2 {
            error.information[1]
        }else{0},
        Ordering::Relaxed
    );
    let context=unsafe{(*pointers).context} as *const u8;
    if !context.is_null(){
        // The operating system supplies a valid CONTEXT pointer. Read
        // registers only; do not unwind or dereference the exhausted stack.
        CONTEXT_EIP.store(unsafe{std::ptr::read_unaligned(context.add(EIP_OFFSET) as *const u32)},Ordering::Relaxed);
        CONTEXT_ESP.store(unsafe{std::ptr::read_unaligned(context.add(ESP_OFFSET) as *const u32)},Ordering::Relaxed);
        CONTEXT_EBP.store(unsafe{std::ptr::read_unaligned(context.add(EBP_OFFSET) as *const u32)},Ordering::Relaxed);
    }
    SEQUENCE.fetch_add(1,Ordering::Release);
    CONTINUE_SEARCH // Never consume or alter the exception.
}
fn resolve_module(address:usize)->String {
    if address==0{return String::from("unavailable");}
    let mut module=std::ptr::null_mut();
    let found=unsafe{
        GetModuleHandleExA(FROM_ADDRESS|UNCHANGED_REFCOUNT,address as *const u8,&mut module)
    };
    if found==0||module.is_null(){return String::from("unmapped_or_non_image");}
    let mut name=[0u8;512];
    let len=unsafe{GetModuleFileNameA(module,name.as_mut_ptr(),name.len() as u32)} as usize;
    let path=if len>0 && len<name.len(){
        String::from_utf8_lossy(&name[..len]).into_owned()
    }else{String::from("path_unavailable")};
    format!("{path} base={:#x} rva={:#x}",module as usize,address.saturating_sub(module as usize))
}
/// Called by the existing, read-only AVI-close sampler on a different thread.
/// Do not log from the vectored exception callback: stack may be exhausted.
pub fn snapshot(stage:&str){
    let sequence=SEQUENCE.load(Ordering::Acquire);
    if sequence==0 || REPORTED.swap(sequence,Ordering::AcqRel)==sequence {return;}
    let code=CODE.load(Ordering::Relaxed);
    let thread=THREAD.load(Ordering::Relaxed);
    let address=INSTRUCTION.load(Ordering::Relaxed);
    let eip=CONTEXT_EIP.load(Ordering::Relaxed);
    let esp=CONTEXT_ESP.load(Ordering::Relaxed);
    let ebp=CONTEXT_EBP.load(Ordering::Relaxed);
    let access_type=ACCESS_TYPE.load(Ordering::Relaxed);
    let access_address=ACCESS_ADDRESS.load(Ordering::Relaxed);
    let access=if code==ACCESS_VIOLATION && access_type!=usize::MAX {
        format!("type={access_type} address={access_address:#x}")
    }else{String::from("not_applicable")};
    crate::log(&format!(
        "EXCEPTION_CAPTURE stage={stage:?} first_chance_code={code:#010x} tid={thread} instruction={address:#x} module=[{}] context_eip={eip:#x} context_esp={esp:#x} context_ebp={ebp:#x} access={access} observer_only=1",
        resolve_module(address)
    ));
}
pub fn maybe_install(){
    if std::env::var("CYBER_GLIDE_TRACE").as_deref()!=Ok("1"){return;}
    INSTALLED.get_or_init(||unsafe {
        let handler=AddVectoredExceptionHandler(1,Some(observe));
        if handler.is_null(){
            crate::log(&format!("EXCEPTION_OBSERVER unavailable Win32={}",GetLastError()));
            false
        }else{
            crate::log("EXCEPTION_OBSERVER installed (deferred first-chance capture, never suppresses exceptions)");
            true
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_relevant_exceptions_are_captured(){
        assert!(interesting(ACCESS_VIOLATION));
        assert!(interesting(STACK_OVERFLOW));
        assert!(!interesting(0x8000_0003));
        assert!(!interesting(0x4001_0006));
    }
    #[test]
    fn x86_context_offsets_are_aligned_and_expected(){
        assert_eq!((EBP_OFFSET,EIP_OFFSET,ESP_OFFSET),(180,184,196));
        assert_eq!(EIP_OFFSET%std::mem::align_of::<u32>(),0);
    }
}
