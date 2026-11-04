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
// Exact SHA-256 hashes of previously distributed *project-owned* 32-bit
// DLL binaries, verified against the archived GitHub Actions ZIP files.
// Only these known versions may be replaced automatically. The two oldest
// were broken: their WinMM forwarding exports looped infinitely (EB FE).
// Do not silently overwrite a third-party, modified or unknown DLL.
const BROKEN_BRIDGES:[&str;2]=[
    "055e634eb9f9472978ce930eabf38b0d8790542df7fceba1e8c9d61876462f71", // v0.2.1
    "d9cb494e83bb0e8552f238ceddbe048a7a0ab8afae585c853b5c3a0d7858bdbb", // prototype
];
const KNOWN_RELEASED_BRIDGES:[&str;9]=[
    "3f0e0a17aa86fd61c2d52a3341251b3c4e9e03fb08cbb1664f50f2a630d33b29", // forwarder validation ZIP
    "567e748733018c8ae91c1f31e6e90f30abebbf33cf838ea4a90e7aeeeeb055d0", // v0.2.2
    "acb07fcd2bfe789eae5927a9ab1730563a73948bb9aaaed2fc4e95c97c21df13", // v0.2.3
    "a0c93deff6981d34dd3cd964a0666d958f59c65ad3c8efe0bc2c09fd8e03d3d4", // v0.2.4
    "23b5ddba39763d5626796ed94b82ba8d17422751a4aac1f3edea0276f7b6a666", // v0.2.5
    "42df88a0aa88f1cfd24048d02df13478b4435af4d2d8ed4b53443fe334fc7af2", // v0.2.6
    "6d4ffd7ca461b5e41b0bf1ac3cda898d343284febe3085c4e1a40f832defbc6d", // v0.2.7, verified ZIP
    "abf5c9d60ff20d0dc5816c742fbc04ca8a1226385b45fe375288a52d96c9dfca", // v0.2.8, verified ZIP
    "a95c5787f3a9b5dc101e673777111257fcf0482bf4dce8340cb5b91b00f5026a", // v0.2.9, verified ZIP
];
fn digest(path:&Path)->Result<String,String>{
    let bytes=fs::read(path).map_err(|e|format!("Cannot hash {}: {e}",path.display()))?;
    Ok(format!("{:x}",Sha256::digest(bytes)))
}
/// For a pass-through A/B test both the launcher-supplied and installed
/// x86 DLLs must be byte-identical. Otherwise an older installed proxy can
/// silently execute the non-NOOP nGlide function and invalidate the test.
pub fn matching_installed_hash(root:&Path,game:&Path)->Result<String,String>{
    let source=root.join(DLL_NAME);
    let installed=game.join(DLL_NAME);
    check(&source)?;
    check(&installed)?;
    let expected=digest(&source)?;
    let actual=digest(&installed)?;
    if actual!=expected{
        return Err(format!(
            "Diagnostic cancelled: installed CDDA DLL SHA-256 {actual} differs from patcher DLL SHA-256 {expected}. No pass-through test can be trusted with different DLLs."
        ));
    }
    Ok(actual)
}
fn broken(hash:&str)->bool{BROKEN_BRIDGES.contains(&hash)}
fn replaceable_known_release(hash:&str)->bool {
    broken(hash)||KNOWN_RELEASED_BRIDGES.contains(&hash)
}
fn prior_version_backup(game:&Path,hash:&str)->std::path::PathBuf {
    // Deterministic per-version backup: does not conflict with the existing
    // .previous.bak created when v0.2.1 was updated, and never overwrites it.
    game.join(format!("CyberGladiatorsCDDA.dll.previous-{}.bak",&hash[..12]))
}
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
    if !replaceable_known_release(&existing){
        return Err(format!(
            "Existing {DLL_NAME} has an unrecognized SHA-256 ({existing}); refusing to overwrite. Replace it manually only after making a backup."
        ));
    }
    // Upgrade any recognized older project-owned DLL, even if it was not one
    // of the original broken builds. Preserve the exact prior version under
    // a unique hash-qualified backup; never overwrite previous backups.
    let staged=game.join("CyberGladiatorsCDDA.dll.updating.tmp");
    let backup=prior_version_backup(game,&existing);
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
    fn recognizes_archive_verified_prior_dll_versions(){
        for (hash,release) in [
            ("567e748733018c8ae91c1f31e6e90f30abebbf33cf838ea4a90e7aeeeeb055d0","v0.2.2"),
            ("acb07fcd2bfe789eae5927a9ab1730563a73948bb9aaaed2fc4e95c97c21df13","v0.2.3"),
            ("a0c93deff6981d34dd3cd964a0666d958f59c65ad3c8efe0bc2c09fd8e03d3d4","v0.2.4"),
            ("23b5ddba39763d5626796ed94b82ba8d17422751a4aac1f3edea0276f7b6a666","v0.2.5"),
            ("42df88a0aa88f1cfd24048d02df13478b4435af4d2d8ed4b53443fe334fc7af2","v0.2.6"),
            ("6d4ffd7ca461b5e41b0bf1ac3cda898d343284febe3085c4e1a40f832defbc6d","v0.2.7"),
            ("abf5c9d60ff20d0dc5816c742fbc04ca8a1226385b45fe375288a52d96c9dfca","v0.2.8"),
            ("a95c5787f3a9b5dc101e673777111257fcf0482bf4dce8340cb5b91b00f5026a","v0.2.9"),
            ("3f0e0a17aa86fd61c2d52a3341251b3c4e9e03fb08cbb1664f50f2a630d33b29","forwarder validation"),
        ] {
            assert!(replaceable_known_release(hash),"{release}");
            assert!(!broken(hash),"{release} was not an infinite-loop build");
        }
    }
    #[test]
    fn keeps_distinct_backup_files_for_prior_versions(){
        let game=Path::new("C:/Cyber");
        let a=prior_version_backup(game,"567e748733018c8ae91c1f31e6e90f30abebbf33cf838ea4a90e7aeeeeb055d0");
        let b=prior_version_backup(game,"acb07fcd2bfe789eae5927a9ab1730563a73948bb9aaaed2fc4e95c97c21df13");
        assert_ne!(a,b);
        assert!(a.to_string_lossy().contains("567e74873301"));
        assert_ne!(a,game.join("CyberGladiatorsCDDA.dll.previous.bak"));
    }
    #[test]
    fn never_auto_overwrites_unknown_dll(){
        assert!(!replaceable_known_release("f9b9b6f601e5986d46d1f672fbbd010ca93dafb8087182b55a0fbf7f4ad8ce4d"));
        assert!(replaceable_known_release(BROKEN_BRIDGES[0]));
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
