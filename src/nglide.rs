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
pub fn configure(command:&mut Command,logo:bool){
    for (key,value) in [
        ("NGLIDE_RESOLUTION","1"),("NGLIDE_ASPECT","1"),
        ("NGLIDE_REFRESH","2"),("NGLIDE_VSYNC","1"),
        ("NGLIDE_GAMMA","5"),("NGLIDE_SPLASH",if logo{"1"}else{"0"}),
    ]{command.env(key,value);}
}
