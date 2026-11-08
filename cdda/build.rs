// SPDX-License-Identifier: GPL-3.0-or-later
// The isolated Win32 DirectDraw v1 COM implementation uses the canonical
// Microsoft ddraw.h interface declarations. Building through cc/MSVC pins
// the method ABI without modifying the system DirectDraw DLL or its vtables.
fn main(){
    println!("cargo:rerun-if-changed=native/fr_virtual_ddraw.cpp");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref()==Ok("windows"){
        let mut compiler=cc::Build::new();
        compiler.cpp(true)
            .file("native/fr_virtual_ddraw.cpp")
            .flag_if_supported("/EHsc")
            .flag_if_supported("/GR")
            .flag_if_supported("/std:c++17")
            .compile("cyber_fr_virtual_ddraw");
        println!("cargo:rustc-link-lib=user32");
    }
}
