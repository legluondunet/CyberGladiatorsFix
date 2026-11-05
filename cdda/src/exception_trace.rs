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
// EDI=0x9C, EBX=0xA4, EBP=0xB4, EIP=0xB8, ESP=0xC4.
// These offsets apply only to the x86 Windows CONTEXT layout.
const EDI_OFFSET:usize=0x9c;
const EBX_OFFSET:usize=0xa4;
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
static CONTEXT_EBX:AtomicU32=AtomicU32::new(0);
static CONTEXT_EDI:AtomicU32=AtomicU32::new(0);
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
    fn GetModuleHandleA(name:*const u8)->*mut c_void;
    fn GetModuleFileNameA(module:*mut c_void,name:*mut u8,size:u32)->u32;
    fn GetCurrentProcess()->*mut c_void;
    fn ReadProcessMemory(process:*mut c_void,address:*const c_void,
        output:*mut c_void,size:usize,read:*mut usize)->i32;
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
        CONTEXT_EBX.store(unsafe{std::ptr::read_unaligned(context.add(EBX_OFFSET) as *const u32)},Ordering::Relaxed);
        CONTEXT_EDI.store(unsafe{std::ptr::read_unaligned(context.add(EDI_OFFSET) as *const u32)},Ordering::Relaxed);
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
/// A minimal in-memory PE identity snapshot of the D3D9 implementation
/// actually mapped in the x86 game process. This is read-only: never load a
/// missing module, touch the Windows registry, patch a DLL or run code in it.
///
/// Path and PE timestamp/size only *fingerprint* an image for comparison;
/// they must not be presented as proof that it is DXVK. Use DXVK's own
/// app_d3d9.log / HUD for positive DXVK identification.
fn pe_identity(image:&[u8])->Option<(u16,u32,u32)>{
    if image.len()<0x100 || image.get(..2)?!=b"MZ" {return None;}
    let offset=u32::from_le_bytes(image.get(0x3c..0x40)?.try_into().ok()?) as usize;
    if offset>0x800 || offset.checked_add(0x54)? > image.len(){return None;}
    if image.get(offset..offset+4)?!=b"PE\0\0"{return None;}
    let machine=u16::from_le_bytes(image.get(offset+4..offset+6)?.try_into().ok()?);
    let timestamp=u32::from_le_bytes(image.get(offset+8..offset+12)?.try_into().ok()?);
    let size_image=u32::from_le_bytes(image.get(offset+0x50..offset+0x54)?.try_into().ok()?);
    Some((machine,timestamp,size_image))
}
fn loaded_pe_identity(name:&std::ffi::CStr,label:&str,stage:&str){
    let module=unsafe{GetModuleHandleA(name.as_ptr().cast())};
    if module.is_null(){
        crate::log(&format!("{label} stage={stage:?} loaded=0"));
        return;
    }
    let mut out=[0u8;1024];
    let len=unsafe{GetModuleFileNameA(module,out.as_mut_ptr(),out.len() as u32)} as usize;
    let path=if len>0 && len<out.len(){
        String::from_utf8_lossy(&out[..len]).into_owned()
    }else{String::from("path_unavailable")};
    let header=unsafe{std::slice::from_raw_parts(module as *const u8,4096)};
    let details=match pe_identity(header){
        Some((machine,timestamp,size))=>format!(
            "machine={machine:#06x} timestamp={timestamp:#010x} size_of_image={size:#x}"),
        None=>String::from("pe_header_unavailable"),
    };
    crate::log(&format!(
        "{label} stage={stage:?} loaded=1 base={:#x} path={path:?} {details} (fingerprint_only_not_provider_proof)",
        module as usize
    ));
}
/// Snapshot the actual legacy video / 3D providers mapped in the same
/// 32-bit process, never LoadLibrary: this must not change runtime behavior.
/// GE-Proton may include D7VK's ddraw.dll alongside DXVK's d3d9.dll, but
/// only these in-process facts and independent provider logs can show it.
pub fn d3d9_identity(stage:&str){
    if std::env::var("CYBER_GLIDE_TRACE").as_deref()!=Ok("1"){return;}
    loaded_pe_identity(c"d3d9.dll","D3D9_IDENTITY",stage);
    loaded_pe_identity(c"ddraw.dll","DDRAW_IDENTITY",stage);
    loaded_pe_identity(c"mciavi32.dll","MCIAVI_IDENTITY",stage);
}
// All potentially expensive work is strictly outside the VEH: our sampler
// has its own healthy stack. A stack overflow may already have destroyed
// the failing thread's stack; all reads fail safely through the Win32 API.
fn read_memory(address:usize,output:&mut [u8])->usize{
    if address<0x10000||output.is_empty(){return 0;}
    let mut read=0usize;
    let ok=unsafe{ReadProcessMemory(
        GetCurrentProcess(),address as *const c_void,
        output.as_mut_ptr().cast(),output.len(),&mut read)};
    if ok!=0{read.min(output.len())}else{0}
}
fn candidate_module_rva(address:usize)->Option<String>{
    if address<0x10000 {return None;}
    let mut module=std::ptr::null_mut();
    if unsafe{GetModuleHandleExA(FROM_ADDRESS|UNCHANGED_REFCOUNT,
        address as *const u8,&mut module)}==0||module.is_null(){return None;}
    let mut name=[0u8;512];
    let len=unsafe{GetModuleFileNameA(module,name.as_mut_ptr(),name.len() as u32)} as usize;
    if len==0||len>=name.len(){return None;}
    let path=String::from_utf8_lossy(&name[..len]);
    let basename=path.rsplit(['\\','/']).next().unwrap_or("unknown");
    let relevant=["d3d9.dll","glide.dll","cyber.exe","ddraw.dll",
        "winmm.dll","user32.dll","ntdll.dll","gdi32.dll","dxgi.dll"];
    if !relevant.iter().any(|n|basename.eq_ignore_ascii_case(n)){return None;}
    Some(format!("{basename}+0x{:x}",address.saturating_sub(module as usize)))
}
fn window_message_name(value:u32)->&'static str{
    match value{
        0x0002=>"WM_DESTROY",0x0005=>"WM_SIZE",
        0x0006=>"WM_ACTIVATE",0x001c=>"WM_ACTIVATEAPP",
        0x0046=>"WM_WINDOWPOSCHANGING",0x0047=>"WM_WINDOWPOSCHANGED",
        0x007d=>"WM_STYLECHANGED",0x007e=>"WM_DISPLAYCHANGE",
        0x0083=>"WM_NCCALCSIZE",0x0086=>"WM_NCACTIVATE",
        0x0112=>"WM_SYSCOMMAND",_=>"unidentified",
    }
}
fn windowproc_fault_probe(eip:usize,ebp:usize,ebx:u32,edi:u32){
    // GE-Proton DXVK d3d9_window.cpp checks WM_NCCALCSIZE (0x83)
    // and wparam=1 at the start of its D3D9WindowProc.
    // This machine-code fingerprint must match before interpreting
    // EBX as the message and EDI as its wparam.
    let mut code=[0u8;24];
    let n=read_memory(eip,&mut code);
    let signature=n>=9 && code[..9]==[0x89,0x45,0x90,0x81,0xfb,0x83,0,0,0];
    crate::log(&format!(
        "DXVK_WINDOWPROC_CANDIDATE signature={} register_ebx={ebx:#x} message={} register_edi={edi:#x} wparam_candidate={} (interpret_only_if_signature_matches)",
        u8::from(signature),window_message_name(ebx),edi
    ));
    if !signature{return;}
    // The observed x86 function uses an EBP stack frame. The normal
    // stdcall arguments begin at EBP+8; do not assume the frame survived
    // the overflow and validate candidates against the register values.
    let mut args=[0u8;16];
    if read_memory(ebp.saturating_add(8),&mut args)==args.len(){
        let arg=|i:usize|u32::from_le_bytes(args[i..i+4].try_into().unwrap());
        crate::log(&format!(
            "DXVK_WINDOWPROC_ARGS inferred_hwnd={:#x} inferred_message={:#x} ({}) inferred_wparam={:#x} inferred_lparam={:#x} regs_match={} (unverified_frame)",
            arg(0),arg(4),window_message_name(arg(4)),arg(8),arg(12),
            u8::from(arg(4)==ebx&&arg(8)==edi)
        ));
    }else{
        crate::log("DXVK_WINDOWPROC_ARGS unavailable (stack frame unreadable)");
    }
}
fn exception_forensics(eip:usize,esp:usize,ebp:usize,ebx:u32,edi:u32){
    // Record exact machine code *at* the faulting instruction, not a guess
    // about the DXVK symbol or cause. Reading can fail if the process is
    // already tearing down its DLLs.
    let mut bytes=[0u8;64];
    let n=read_memory(eip,&mut bytes);
    if n>0 {
        let hex=bytes[..n].iter().map(|byte|format!("{byte:02x}"))
            .collect::<Vec<_>>().join(" ");
        crate::log(&format!(
            "DXVK_FAULT_CODE eip={} sampled_bytes={n} hex={hex} (read_only, not_symbolized)",
            candidate_module_rva(eip).unwrap_or_else(||format!("{eip:#x}"))
        ));
    }else{
        crate::log("DXVK_FAULT_CODE unavailable (faulting code already unmapped)");
    }
    windowproc_fault_probe(eip,ebp,ebx,edi);
    // Sample a bounded region of the *faulting thread's* stack using
    // ReadProcessMemory. Raw stack data can include private game/user data,
    // so log ONLY recognizable module+RVA address candidates, never the
    // contents of arbitrary stack words. This is a heuristic snapshot,
    // not a validated unwind or proof of recursive call frames.
    const MAX_WORDS:usize=128;
    let mut stack=[0u8;MAX_WORDS*4];
    let mut n=read_memory(esp,&mut stack);
    if n==0{
        // A stack-overflow ESP can sit at a guard page boundary. A second
        // bounded probe above it may still recover a small readable range.
        n=read_memory(esp.saturating_add(128),&mut stack);
    }
    let mut candidates=Vec::new();
    for (i,word) in stack[..n-n%4].chunks_exact(4).enumerate() {
        let address=u32::from_le_bytes(word.try_into().unwrap()) as usize;
        if let Some(module)=candidate_module_rva(address) {
            candidates.push(format!("slot={i}:{module}"));
            if candidates.len()>=36{break;}
        }
    }
    crate::log(&format!(
        "DXVK_STACK_CANDIDATES exception_esp={esp:#x} bytes_sampled={n} module_addresses={} [{}] heuristic_only_not_backtrace=1",
        candidates.len(),candidates.join(", ")
    ));
    // When available, an x86 EBP-chain can provide more useful return
    // addresses than raw stack words. Optimized DXVK can omit EBP frames,
    // so absence of a chain is inconclusive.
    let mut frame=ebp;
    let mut chain=Vec::new();
    for _ in 0..20{
        if frame<0x10000 {break;}
        let mut pair=[0u8;8];
        if read_memory(frame,&mut pair)!=pair.len(){break;}
        let next=u32::from_le_bytes(pair[..4].try_into().unwrap()) as usize;
        let ret=u32::from_le_bytes(pair[4..].try_into().unwrap()) as usize;
        if let Some(name)=candidate_module_rva(ret){
            chain.push(name);
        }
        if next<=frame || next-frame>0x100000{break;}
        frame=next;
    }
    crate::log(&format!(
        "DXVK_EBP_CHAIN starting_ebp={ebp:#x} probable_returns={} [{}] optimized_code_may_omit_ebp=1",
        chain.len(),chain.join(" <- ")
    ));
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
    let ebx=CONTEXT_EBX.load(Ordering::Relaxed);
    let edi=CONTEXT_EDI.load(Ordering::Relaxed);
    let access_type=ACCESS_TYPE.load(Ordering::Relaxed);
    let access_address=ACCESS_ADDRESS.load(Ordering::Relaxed);
    let access=if code==ACCESS_VIOLATION && access_type!=usize::MAX {
        format!("type={access_type} address={access_address:#x}")
    }else{String::from("not_applicable")};
    crate::log(&format!(
        "EXCEPTION_CAPTURE stage={stage:?} first_chance_code={code:#010x} tid={thread} instruction={address:#x} module=[{}] context_eip={eip:#x} context_esp={esp:#x} context_ebp={ebp:#x} access={access} observer_only=1",
        resolve_module(address)
    ));
    if code==STACK_OVERFLOW {
        exception_forensics(eip as usize,esp as usize,ebp as usize,ebx,edi);
    }
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
    fn pe_identity_is_bounds_checked_and_reports_stable_image_metadata(){
        let mut data=[0u8;4096];
        data[..2].copy_from_slice(b"MZ");
        data[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        data[0x80..0x84].copy_from_slice(b"PE\0\0");
        data[0x84..0x86].copy_from_slice(&0x14cu16.to_le_bytes());
        data[0x88..0x8c].copy_from_slice(&0x11223344u32.to_le_bytes());
        data[0xd0..0xd4].copy_from_slice(&0x00200000u32.to_le_bytes());
        assert_eq!(pe_identity(&data),Some((0x14c,0x11223344,0x00200000)));
        data[0x3c..0x40].copy_from_slice(&0x7ffu32.to_le_bytes());
        assert!(pe_identity(&data).is_none());
        assert!(pe_identity(&data[..16]).is_none());
    }
    #[test]
    fn only_relevant_exceptions_are_captured(){
        assert!(interesting(ACCESS_VIOLATION));
        assert!(interesting(STACK_OVERFLOW));
        assert!(!interesting(0x8000_0003));
        assert!(!interesting(0x4001_0006));
    }
    #[test]
    fn maps_window_messages_used_by_the_dxvk_window_procedure(){
        assert_eq!(window_message_name(0x83),"WM_NCCALCSIZE");
        assert_eq!(window_message_name(0x1c),"WM_ACTIVATEAPP");
        assert_eq!(window_message_name(0x47),"WM_WINDOWPOSCHANGED");
        assert_eq!(window_message_name(0xffff),"unidentified");
        assert_eq!((EDI_OFFSET,EBX_OFFSET,EBP_OFFSET),(156,164,180));
    }
    #[test]
    fn x86_context_offsets_are_aligned_and_expected(){
        assert_eq!((EBP_OFFSET,EIP_OFFSET,ESP_OFFSET),(180,184,196));
        assert_eq!(EIP_OFFSET%std::mem::align_of::<u32>(),0);
    }
}
