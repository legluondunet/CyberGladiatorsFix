// Opt-in native Windows French-edition experiment, injected into the pinned
// GameFrame x86 Glide wrapper at build time.
//
// The exact Windows-native FR CYBER.EXE is SHA-256 validated by the x64
// launcher before spawn. At grGlideInit, all three 640x480x16 SetDisplayMode
// argument preimages are verified in the loaded process before any write.
// Replace ONLY literal arguments in process memory, not the COM vtable,
// Windows APIs or game files. The original DirectDraw SetDisplayMode executes.
//
// WARNING: an original DirectDraw 1 driver may reject desktop x32 modes.
// This test is opt-in, fail-closed, native-Windows-FR-only, and never the
// default. No fake DD_OK, no runtime code detours, no cross-process patch.
#[link(name = "kernel32")]
unsafe extern "system" {
    fn VirtualProtect(address: *mut std::ffi::c_void, len: usize,
        protection: u32, previous: *mut u32) -> i32;
    fn FlushInstructionCache(process: *mut std::ffi::c_void,
        address: *const std::ffi::c_void, len: usize) -> i32;
    fn GetCurrentProcess() -> *mut std::ffi::c_void;
}
const CYBER_TEST_EXECUTE_READWRITE: u32 = 0x40;
const CYBER_MODE_WRAPPER_BYTES: [u8; 15] = [
    0xb9, 0x10, 0, 0, 0, // ECX=16 bpp
    0xba, 0xe0, 0x01, 0, 0, // EDX=480
    0xb8, 0x80, 0x02, 0, 0, // EAX=640
];
const CYBER_MODE_DIRECT_BYTES: [u8; 12] = [
    0x6a, 0x10, // PUSH 16 bpp
    0x68, 0xe0, 0x01, 0, 0, // PUSH 480
    0x68, 0x80, 0x02, 0, 0, // PUSH 640
];
const CYBER_MODE_SITES: [(usize, &[u8]); 3] = [
    (0x0040_50af, &CYBER_MODE_WRAPPER_BYTES),
    (0x0040_6a68, &CYBER_MODE_WRAPPER_BYTES),
    (0x0043_6411, &CYBER_MODE_DIRECT_BYTES),
];

fn cyber_native_mode_arguments(w: u32, h: u32, depth: u32) -> ([u8; 15], [u8; 12]) {
    let mut wrapper = CYBER_MODE_WRAPPER_BYTES;
    wrapper[1..5].copy_from_slice(&depth.to_le_bytes());
    wrapper[6..10].copy_from_slice(&h.to_le_bytes());
    wrapper[11..15].copy_from_slice(&w.to_le_bytes());
    let mut direct = CYBER_MODE_DIRECT_BYTES;
    direct[1] = depth as u8;
    direct[3..7].copy_from_slice(&h.to_le_bytes());
    direct[8..12].copy_from_slice(&w.to_le_bytes());
    (wrapper, direct)
}

fn cyber_native_mode_requested() -> Option<(u32, u32, u32)> {
    if std::env::var("CYBER_GAMEFRAME_TEST").as_deref() != Ok("1")
        || std::env::var("CYBER_GAME_EDITION").as_deref() != Ok("FR")
        || std::env::var("CYBER_GAMEFRAME_ALT_TAB_NATIVE_FIX").as_deref() != Ok("1")
        || std::env::var("CYBER_FR_NATIVE_MODE_EXPERIMENT").as_deref() != Ok("1") {
        return None;
    }
    let dimensions = std::env::var("CYBER_FR_NATIVE_DISPLAY_MODE").ok()?;
    let parts: Vec<u32> = dimensions
        .split('x').map(str::parse::<u32>).collect::<Result<_,_>>().ok()?;
    match parts.as_slice() {
        [w,h,32] if (640..=7680).contains(w) && (480..=4320).contains(h)
            && !(*w == 640 && *h == 480) => Some((*w,*h,32)),
        _ => None,
    }
}

unsafe fn cyber_replace_process_bytes(address: usize, expected: &[u8],
    replacement: &[u8]) -> bool {
    if expected.len() != replacement.len() { return false; }
    let mut previous = 0u32;
    if VirtualProtect(address as *mut _, expected.len(),
        CYBER_TEST_EXECUTE_READWRITE, &mut previous) == 0 {
        return false;
    }
    let ptr=address as *mut u8;
    let current=std::slice::from_raw_parts(ptr,expected.len());
    if current != expected {
        let mut ignored=0u32;
        let _=VirtualProtect(address as *mut _,expected.len(),previous,&mut ignored);
        return false;
    }
    std::ptr::copy_nonoverlapping(replacement.as_ptr(),ptr,replacement.len());
    let flushed=FlushInstructionCache(
        GetCurrentProcess(),address as *const _,replacement.len()
    ) != 0;
    let mut ignored=0u32;
    let protected=VirtualProtect(address as *mut _,expected.len(),
        previous,&mut ignored) != 0;
    flushed && protected
}

fn cyber_fr_native_mode_test_apply() {
    let Some((w,h,depth))=cyber_native_mode_requested() else{return;};
    // The verified 32-bit retail game is intentionally not ASLR relocated.
    let base=unsafe{crate::win32::GetModuleHandleA(std::ptr::null())} as usize;
    if base != 0x0040_0000 {
        log!("CYBER_FR_NATIVE_MODE_TEST refused=unexpected_image_base {base:#x}");
        return;
    }
    // All three signatures are checked BEFORE any process-memory mutation.
    for &(address,bytes) in &CYBER_MODE_SITES {
        let actual=unsafe{std::slice::from_raw_parts(address as *const u8,bytes.len())};
        if actual!=bytes{
            log!("CYBER_FR_NATIVE_MODE_TEST refused=signature_mismatch address={address:#x} changed=0");
            return;
        }
    }
    let (wrapper,direct)=cyber_native_mode_arguments(w,h,depth);
    let replacements: [&[u8];3]=[&wrapper,&wrapper,&direct];
    let mut applied=0usize;
    for (index, &(address,original)) in CYBER_MODE_SITES.iter().enumerate() {
        if !unsafe{cyber_replace_process_bytes(address,original,replacements[index])}{
            // Include the failing site: its bytes may already have been
            // written even when the instruction-cache flush/protection reset
            // reported an error. Roll back in reverse order.
            for j in (0..=index).rev(){
                let (old_address, old_bytes)=CYBER_MODE_SITES[j];
                let _=unsafe{cyber_replace_process_bytes(
                    old_address,replacements[j],old_bytes
                )};
            }
            log!("CYBER_FR_NATIVE_MODE_TEST status=refused stage=patch_failed index={index} rollback_attempted=1");
            return;
        }
        applied+=1;
    }
    log!("CYBER_FR_NATIVE_MODE_TEST status=applied edition=FR runtime=nativeWindows game_ram_only=1 original_mode=640x480x16 requested_mode={w}x{h}x{depth} callsites=0x4050af,0x406a68,0x436411 real_DirectDraw_call_preserved=1 no_vtable_hook=1 original_exe_unchanged=1");
}
