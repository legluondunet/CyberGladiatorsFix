// SPDX-License-Identifier: GPL-3.0-or-later
//! Load the user's installed 32-bit nGlide DLL; never bundle proprietary files.
use std::fs;
use std::path::{Path,PathBuf};
use std::process::Command;

pub const OFFICIAL:&str="https://www.zeus-software.com/downloads/nglide";
fn valid_pe32_dll(path:&Path)->Result<(),String>{
    let meta=fs::symlink_metadata(path).map_err(|e|e.to_string())?;
    if !meta.is_file()||meta.file_type().is_symlink()||!(1024..=16*1024*1024).contains(&meta.len()){
        return Err(format!("Invalid Windows 32-bit DLL: {}",path.display()));
    }
    let f=fs::read(path).map_err(|e|e.to_string())?;
    if f.len()<256||&f[..2]!=b"MZ"{return Err("Not a Windows DLL".into());}
    let o=u32::from_le_bytes(f[0x3c..0x40].try_into().unwrap()) as usize;
    if o.checked_add(26).is_none_or(|v|v>f.len()) || &f[o..o+4]!=b"PE\0\0" ||
        u16::from_le_bytes(f[o+4..o+6].try_into().unwrap())!=0x14c ||
        u16::from_le_bytes(f[o+22..o+24].try_into().unwrap())&0x2000==0 ||
        u16::from_le_bytes(f[o+24..o+26].try_into().unwrap())!=0x10b {
        return Err("DLL must be x86 PE32 (not an EXE or x64 DLL)".into());
    }
    Ok(())
}
fn source(root:&Path)->Option<PathBuf>{
    let nearby=root.join("glide.dll");
    if valid_pe32_dll(&nearby).is_ok(){return Some(nearby);}
    for variable in ["WINDIR","SystemRoot"]{
        if let Some(base)=std::env::var_os(variable){
            let p=PathBuf::from(base).join("SysWOW64").join("glide.dll");
            if valid_pe32_dll(&p).is_ok(){return Some(p);}
        }
    }
    None
}
pub fn try_install(root:&Path,game:&Path)->Result<bool,String>{
    let target=game.join("glide.dll");
    if target.exists()||target.is_symlink(){valid_pe32_dll(&target)?;return Ok(true);}
    let Some(src)=source(root) else {return Ok(false)};
    fs::copy(&src,&target).map_err(|e|format!("Copy nGlide: {e}"))?;
    if let Err(e)=valid_pe32_dll(&target){let _=fs::remove_file(&target);return Err(e);}
    Ok(true)
}
pub fn verify(game:&Path)->Result<(),String>{
    valid_pe32_dll(&game.join("glide.dll"))
        .map_err(|e|format!("nGlide glide.dll missing or invalid: {e}. Install separately from {OFFICIAL}"))
}
pub fn check_logo(game:&Path)->Result<(),String>{
    valid_pe32_dll(&game.join("3DfxSpl.dll"))
        .map_err(|e|format!("--3dfx-logo requires the optional 32-bit 3DfxSpl.dll in game folder: {e}"))
}
/// Match ScorchedPlanetFix's per-process nGlide backend policy.
/// Finding the Vulkan loader does not guarantee a functioning Vulkan driver;
/// the renderer may still need a manual fallback for unusual Wine setups.
#[cfg(windows)]
fn has_vulkan_loader()->bool{
    use std::ffi::c_void;
    #[link(name="kernel32")]
    extern "system"{
        fn LoadLibraryW(file:*const u16)->*mut c_void;
        fn FreeLibrary(handle:*mut c_void)->i32;
    }
    let path:Vec<u16>="vulkan-1.dll".encode_utf16().chain(std::iter::once(0)).collect();
    unsafe{
        let m=LoadLibraryW(path.as_ptr());
        if m.is_null(){false}else{FreeLibrary(m);true}
    }
}
fn configure_with_backend(command:&mut Command,logo:bool,backend:&str){
    for (key,value) in [
        ("NGLIDE_BACKEND",backend), // 2=Vulkan if loader present; 0=automatic fallback
        ("NGLIDE_RESOLUTION","1"), // Desktop resolution
        ("NGLIDE_ASPECT","1"), // Aspect ratio correction
        ("NGLIDE_REFRESH","2"), // 60 Hz
        ("NGLIDE_VSYNC","1"), // On
        ("NGLIDE_GAMMA","5"), // 1.0
        ("NGLIDE_SPLASH",if logo{"1"}else{"0"}),
    ]{command.env(key,value);}
}
pub fn configure(command:&mut Command,logo:bool){
    let backend=if has_vulkan_loader(){"2"}else{"0"};
    configure_with_backend(command,logo,backend);
}
/// Diagnostic-only backend: let nGlide select a renderer automatically,
/// rather than forcing Vulkan merely because vulkan-1.dll is loadable.
/// All other per-game settings stay identical.
pub fn configure_diagnostic(command:&mut Command,logo:bool){
    configure_with_backend(command,logo,"0");
}

#[cfg(test)]
mod process_config_tests {
    use super::*;
    use std::ffi::OsStr;
    fn env_value<'a>(cmd:&'a Command,key:&str)->Option<&'a OsStr>{
        cmd.get_envs().find(|(k,_)|*k==OsStr::new(key)).and_then(|(_,v)|v)
    }
    #[test]
    fn matches_scorched_planet_process_options(){
        let mut command=Command::new("cyber.exe");
        configure_with_backend(&mut command,false,"2");
        for (key,value) in [
            ("NGLIDE_BACKEND","2"),("NGLIDE_RESOLUTION","1"),
            ("NGLIDE_ASPECT","1"),("NGLIDE_REFRESH","2"),
            ("NGLIDE_VSYNC","1"),("NGLIDE_GAMMA","5"),
            ("NGLIDE_SPLASH","0"),
        ]{assert_eq!(env_value(&command,key),Some(OsStr::new(value)));}
        assert_eq!(command.get_envs().count(),7);
    }
    #[test]
    fn trace_mode_forces_auto_backend_only(){
        let mut normal=Command::new("cyber.exe");
        configure_with_backend(&mut normal,false,"2");
        let mut trace=Command::new("cyber.exe");
        configure_diagnostic(&mut trace,false);
        let n=normal.get_envs().collect::<std::collections::BTreeMap<_,_>>();
        let d=trace.get_envs().collect::<std::collections::BTreeMap<_,_>>();
        assert_eq!(d.get(std::ffi::OsStr::new("NGLIDE_BACKEND"))
            .copied().flatten(),Some(std::ffi::OsStr::new("0")));
        assert_eq!(n.len(),d.len());
        for (key,val) in n.iter(){
            if key.to_string_lossy()!="NGLIDE_BACKEND"{
                assert_eq!(d.get(key),Some(val));
            }
        }
    }
    #[test]
    fn auto_fallback_and_opt_in_logo(){
        let mut command=Command::new("cyber.exe");
        configure_with_backend(&mut command,true,"0");
        assert_eq!(env_value(&command,"NGLIDE_BACKEND"),Some(OsStr::new("0")));
        assert_eq!(env_value(&command,"NGLIDE_SPLASH"),Some(OsStr::new("1")));
    }
}
