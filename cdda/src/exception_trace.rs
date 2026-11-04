// SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only Windows x86 vectored exception observer; installed only for
//! --launch --trace after verifying the exact known game's Glide hook.
//! Never handles, suppresses or resumes an exception.
use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool,Ordering};

#[repr(C)]
struct ExceptionRecord{
    code:u32,
    flags:u32,
    nested:*mut ExceptionRecord,
    address:*mut c_void,
    parameter_count:u32,
    information:[usize;15],
}
#[repr(C)]
struct ExceptionPointers{
    record:*mut ExceptionRecord,
    context:*mut c_void,
}
const CONTINUE_SEARCH:i32=0;
const ACCESS_VIOLATION:u32=0xc000_0005;
static INSTALLED:OnceLock<bool>=OnceLock::new();
static REPORTING:AtomicBool=AtomicBool::new(false);

#[link(name="kernel32")]
extern "system" {
    fn AddVectoredExceptionHandler(
        first:u32,
        handler:Option<unsafe extern "system" fn(*mut ExceptionPointers)->i32>
    )->*mut c_void;
    fn GetCurrentThreadId()->u32;
    fn GetLastError()->u32;
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
    // Logging can itself fault: avoid recursion, never consume the exception.
    if REPORTING.swap(true,Ordering::AcqRel){return CONTINUE_SEARCH;}
    let tid=unsafe{GetCurrentThreadId()};
    let access=if error.code==ACCESS_VIOLATION && error.parameter_count>=2 {
        format!("access_type={} access_address={:#x}",
            error.information[0],error.information[1])
    }else{String::from("access=not_applicable")};
    crate::log(&format!(
        "EXCEPTION_FIRST_CHANCE code={:#010x} tid={tid} instruction={:?} {access} (observer_only=1)",
        error.code,error.address
    ));
    REPORTING.store(false,Ordering::Release);
    CONTINUE_SEARCH
}
pub fn maybe_install(){
    if std::env::var("CYBER_GLIDE_TRACE").as_deref()!=Ok("1"){return;}
    INSTALLED.get_or_init(||unsafe {
        let handler=AddVectoredExceptionHandler(1,Some(observe));
        if handler.is_null(){
            crate::log(&format!("EXCEPTION_OBSERVER unavailable Win32={}",GetLastError()));
            false
        }else{
            crate::log("EXCEPTION_OBSERVER installed (first-chance, never suppresses exceptions)");
            true
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_crash_class_exceptions_are_reported(){
        assert!(interesting(ACCESS_VIOLATION));
        assert!(interesting(0xc000_00fd));
        assert!(!interesting(0x8000_0003));
    }
}
