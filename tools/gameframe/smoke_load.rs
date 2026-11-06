// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows x86 load-only smoke check for GameFrame's experimental glide.dll.
//! Never calls grSstOpen or alters display modes; only verifies the Windows
//! loader can initialize the DLL and locate original Glide 2.11 exports.
use std::ffi::{c_char,c_void};

#[link(name="kernel32")]
unsafe extern "system" {
    fn LoadLibraryA(path:*const c_char)->*mut c_void;
    fn GetProcAddress(module:*mut c_void,proc:*const c_char)->*mut c_void;
    fn FreeLibrary(module:*mut c_void)->i32;
    fn GetLastError()->u32;
}

fn main() {
    let module=unsafe{LoadLibraryA(c"glide.dll".as_ptr())};
    if module.is_null() {
        panic!("GameFrame Glide failed Windows LoadLibraryA: {}",unsafe{GetLastError()});
    }
    let names=[
        c"_grGlideInit@0",c"_grSstOpen@24",c"_grBufferSwap@4",
        c"_grSstQueryHardware@4",c"_grLfbGetWritePtr@4",c"_grLfbEnd@0",
    ];
    for n in names {
        let result=unsafe{GetProcAddress(module,n.as_ptr())};
        assert!(!result.is_null(),"Missing export {:?}",n);
        println!("Loaded {:?}",n);
    }
    assert_ne!(unsafe{FreeLibrary(module)},0,"Could not unload experimental Glide DLL");
    println!("GameFrame Glide Windows x86 LoadLibrary smoke: PASS");
}
