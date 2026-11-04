// SPDX-License-Identifier: GPL-3.0-or-later
//! Place the separately built, project-owned x86 CDDA bridge alongside
//! CYBER.EXE, never in the Wine or Windows system directories.
use std::fs::{self,File,OpenOptions};
use std::io::{Read,Write};
use std::path::Path;

pub const DLL_NAME:&str="CyberGladiatorsCDDA.dll";
fn check(path:&Path)->Result<(),String>{
    let m=fs::symlink_metadata(path)
        .map_err(|e|format!("Missing {DLL_NAME} at {}: {e}",path.display()))?;
    if !m.is_file()||m.file_type().is_symlink()||!(4096..=16*1024*1024).contains(&m.len()){
        return Err(format!("Expected a regular 32-bit DLL at {}",path.display()));
    }
    let mut f=File::open(path).map_err(|e|e.to_string())?;
    let mut header=[0u8;4096];
    let bytes=f.read(&mut header).map_err(|e|e.to_string())?;
    if bytes<256 || &header[0..2]!=b"MZ" {
        return Err(format!("{DLL_NAME} is not a PE executable"));
    }
    let o=u32::from_le_bytes(header[0x3c..0x40].try_into().unwrap()) as usize;
    if o.checked_add(26).is_none_or(|n|n>bytes)||
        &header[o..o+4]!=b"PE\0\0"||
        u16::from_le_bytes(header[o+4..o+6].try_into().unwrap())!=0x14c ||
        u16::from_le_bytes(header[o+22..o+24].try_into().unwrap())&0x2000==0 ||
        u16::from_le_bytes(header[o+24..o+26].try_into().unwrap())!=0x10b {
        return Err(format!("{DLL_NAME} must be a Windows 32-bit x86 DLL"));
    }
    Ok(())
}
pub fn verify(game:&Path)->Result<(),String>{check(&game.join(DLL_NAME))}
pub fn try_install(root:&Path,game:&Path)->Result<(),String>{
    let target=game.join(DLL_NAME);
    if target.exists()||target.is_symlink(){
        return check(&target);
    }
    let source=root.join(DLL_NAME);
    check(&source).map_err(|e|format!(
        "{e}. Put the x86 {DLL_NAME} from the same GitHub Actions ZIP alongside CyberGladiatorsFix.exe."))?;
    let mut input=File::open(&source).map_err(|e|e.to_string())?;
    let mut output=OpenOptions::new().write(true).create_new(true).open(&target)
        .map_err(|e|format!("Cannot create {DLL_NAME} in game folder: {e}"))?;
    if let Err(e)=std::io::copy(&mut input,&mut output).and_then(|_|output.flush())
        .and_then(|_|output.sync_all()){
        drop(output);let _=fs::remove_file(&target);
        return Err(format!("Copy CDDA bridge: {e}"));
    }
    drop(output);
    if let Err(e)=check(&target){
        let _=fs::remove_file(&target);
        return Err(e);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dll_name_is_consistent_with_game_import_patch(){
        assert_eq!(DLL_NAME,"CyberGladiatorsCDDA.dll");
        assert_eq!(DLL_NAME.len()+1,24);
    }
    #[test]
    fn rejects_invalid_x86_modules(){
        let dir=std::env::temp_dir().join(format!("cdda-pe-test-{}",std::process::id()));
        let _=fs::remove_dir_all(&dir);
        fs::create_dir(&dir).unwrap();
        let dll=dir.join(DLL_NAME);
        fs::write(&dll,b"not a PE DLL").unwrap();
        assert!(check(&dll).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
