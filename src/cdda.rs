// SPDX-License-Identifier: GPL-3.0-or-later
//! Place the separately built, project-owned x86 CDDA bridge alongside
//! CYBER.EXE, never in the Wine or Windows system directories.
use std::fs::{self,File,OpenOptions};
use std::io::{Read,Write};
use std::path::Path;
use sha2::{Digest,Sha256};

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
// Exact hashes of our TWO previously distributed, broken 32-bit builds.
// Their WinMM forwarding exports point to a JMP-self (EB FE) loop.
// Never overwrite a third-party or unknown DLL based on its name alone.
const BROKEN_BRIDGES:[&str;2]=[
    "055e634eb9f9472978ce930eabf38b0d8790542df7fceba1e8c9d61876462f71",
    "d9cb494e83bb0e8552f238ceddbe048a7a0ab8afae585c853b5c3a0d7858bdbb",
];
fn digest(path:&Path)->Result<String,String>{
    let bytes=fs::read(path).map_err(|e|format!("Cannot hash {}: {e}",path.display()))?;
    Ok(format!("{:x}",Sha256::digest(bytes)))
}
fn broken(hash:&str)->bool{BROKEN_BRIDGES.contains(&hash)}
pub fn verify(game:&Path)->Result<(),String>{
    let path=game.join(DLL_NAME);
    check(&path)?;
    if broken(&digest(&path)?) {
        return Err(format!(
            "The installed {DLL_NAME} has a known WinMM forwarding hang. Update BOTH files from the latest ZIP, then run CyberGladiatorsFix.exe without arguments to upgrade the DLL."
        ));
    }
    Ok(())
}
fn install_new(source:&Path,target:&Path,expected:&str)->Result<(),String>{
    let mut input=File::open(source).map_err(|e|e.to_string())?;
    let mut output=OpenOptions::new().write(true).create_new(true).open(target)
        .map_err(|e|format!("Cannot create {}: {e}",target.display()))?;
    if let Err(e)=std::io::copy(&mut input,&mut output)
        .and_then(|_|output.flush()).and_then(|_|output.sync_all()){
        drop(output);let _=fs::remove_file(target);
        return Err(format!("Copy CDDA bridge: {e}"));
    }
    drop(output);
    if let Err(e)=check(target).and_then(|_|{
        if digest(target)?==expected {Ok(())}
        else {Err("CDDA DLL checksum changed during copy".into())}
    }){
        let _=fs::remove_file(target);
        return Err(e);
    }
    Ok(())
}
pub fn try_install(root:&Path,game:&Path)->Result<(),String>{
    let source=root.join(DLL_NAME);
    check(&source).map_err(|e|format!(
        "{e}. Copy the x86 {DLL_NAME} from the same latest ZIP beside CyberGladiatorsFix.exe."
    ))?;
    let wanted=digest(&source)?;
    if broken(&wanted){
        return Err(format!(
            "The {DLL_NAME} beside the patcher is from the broken v0.2.0/v0.2.1 build; replace BOTH files with the new ZIP before upgrading."
        ));
    }
    let target=game.join(DLL_NAME);
    if !target.exists() && !target.is_symlink(){
        return install_new(&source,&target,&wanted);
    }
    check(&target)?;
    let existing=digest(&target)?;
    if existing==wanted{return Ok(());}
    if !broken(&existing){
        return Err(format!(
            "Existing {DLL_NAME} has an unrecognized SHA-256 ({existing}); refusing to overwrite. Replace it manually only after making a backup."
        ));
    }
    // Prior project-owned DLL is recognized as unsafe: stage its replacement,
    // keep the old bytes in a one-time backup, and roll back on rename failure.
    let staged=game.join("CyberGladiatorsCDDA.dll.updating.tmp");
    let backup=game.join("CyberGladiatorsCDDA.dll.previous.bak");
    if staged.exists()||staged.is_symlink()||backup.exists()||backup.is_symlink(){
        return Err("CDDA upgrade staging/backup path already exists; refusing overwrite".into());
    }
    install_new(&source,&staged,&wanted)?;
    if let Err(e)=fs::rename(&target,&backup){
        let _=fs::remove_file(&staged);
        return Err(format!("Cannot preserve old CDDA DLL: {e}"));
    }
    if let Err(e)=fs::rename(&staged,&target){
        let restored=fs::rename(&backup,&target);
        let _=fs::remove_file(&staged);
        return Err(format!("Cannot install new CDDA DLL: {e}; restore={restored:?}"));
    }
    verify(game)?;
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
    fn identifies_exact_previously_distributed_broken_bridges(){
        assert!(broken("055e634eb9f9472978ce930eabf38b0d8790542df7fceba1e8c9d61876462f71"));
        assert!(broken("d9cb494e83bb0e8552f238ceddbe048a7a0ab8afae585c853b5c3a0d7858bdbb"));
        assert!(!broken("71c8cb080a8d11001a91c370d2248dd9819cfd00b4b919cd7bbcb7699a96eedb"));
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
