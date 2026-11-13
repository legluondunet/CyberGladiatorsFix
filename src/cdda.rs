// SPDX-License-Identifier: GPL-3.0-or-later
//! Place the separately built, project-owned x86 CDDA bridge alongside
//! CYBER.EXE, never in the Wine or Windows system directories.
use std::fs::{self,File,OpenOptions};
use std::borrow::Cow;
use std::io::{Read,Write};
use std::path::Path;
use sha2::{Digest,Sha256};

pub const DLL_NAME:&str="CyberGladiatorsFix.dll";
const RECEIPT_NAME:&str="CyberGladiatorsFix.sha256";
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
const BROKEN_BRIDGES:[&str;2]=[
    "055e634eb9f9472978ce930eabf38b0d8790542df7fceba1e8c9d61876462f71", // v0.2.1
    "d9cb494e83bb0e8552f238ceddbe048a7a0ab8afae585c853b5c3a0d7858bdbb", // prototype
];
/// In normal builds the exact DLL is compiled into CyberGladiatorsFix.exe.
/// Unit tests retain a file-backed fixture to exercise upgrade and rollback.
fn source_bytes(root:&Path)->Result<Cow<'static,[u8]>,String>{
    #[cfg(test)]
    {Ok(Cow::Owned(fs::read(root.join(DLL_NAME)).map_err(|e|format!("Cannot read test DLL: {e}"))?))}
    #[cfg(not(test))]
    {
        let _=root;
        crate::bundle::verify()?;
        Ok(Cow::Borrowed(crate::bundle::CDDA_DLL))
    }
}
fn validate_bytes(bytes:&[u8])->Result<(),String>{
    if !(4096..=16*1024*1024).contains(&bytes.len())||&bytes[..2]!=b"MZ"{
        return Err("Bundled CDDA DLL is missing a valid DOS header or has an invalid size".into());
    }
    let o=u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as usize;
    if o.checked_add(26).is_none_or(|v|v>bytes.len())||&bytes[o..o+4]!=b"PE\0\0"||
       u16::from_le_bytes(bytes[o+4..o+6].try_into().unwrap())!=0x14c||
       u16::from_le_bytes(bytes[o+22..o+24].try_into().unwrap())&0x2000==0||
       u16::from_le_bytes(bytes[o+24..o+26].try_into().unwrap())!=0x10b{
        return Err("Bundled CDDA module must be a PE32 x86 DLL".into());
    }
    Ok(())
}
fn digest(path:&Path)->Result<String,String>{
    let bytes=fs::read(path).map_err(|e|format!("Cannot hash {}: {e}",path.display()))?;
    Ok(format!("{:x}",Sha256::digest(bytes)))
}
/// A same-version EXE/DLL pairing is required for diagnostic telemetry.
/// Older x86 DLLs can have the same file name and valid PE header yet lack
/// both the required environment switch and the in-memory NO-OP hook.
fn broken(hash:&str)->bool{BROKEN_BRIDGES.contains(&hash)}
// A locally recorded digest is an integrity receipt, not a signature or
// proof of publisher identity. Automatic upgrades always preserve the prior
// DLL in a SHA-qualified backup and fail closed on a malformed receipt.
fn read_receipt(game:&Path)->Result<Option<String>,String>{
    let path=game.join(RECEIPT_NAME);
    if !path.exists() && !path.is_symlink(){return Ok(None);}
    let m=fs::symlink_metadata(&path).map_err(|e|e.to_string())?;
    if !m.is_file()||m.file_type().is_symlink()||m.len()!=65{
        return Err("Invalid CDDA digest receipt; refusing automatic upgrade".into());
    }
    let contents=fs::read_to_string(&path).map_err(|e|e.to_string())?;
    let Some(hash)=contents.strip_suffix('\n') else{
        return Err("Malformed CDDA digest receipt".into());
    };
    if hash.len()!=64 || !hash.bytes().all(|b|b.is_ascii_hexdigit() && !b.is_ascii_uppercase()){
        return Err("Malformed CDDA digest receipt".into());
    }
    Ok(Some(hash.to_owned()))
}
fn record_receipt(game:&Path,hash:&str)->Result<(),String>{
    let path=game.join(RECEIPT_NAME);
    let old=read_receipt(game)?;
    if old.as_deref()==Some(hash){return Ok(());}
    let temp=game.join("CyberGladiatorsFix.sha256.tmp");
    if temp.exists()||temp.is_symlink(){
        return Err("Existing CDDA digest receipt staging file; refusing overwrite".into());
    }
    let mut output=OpenOptions::new().write(true).create_new(true).open(&temp)
        .map_err(|e|format!("Cannot stage CDDA receipt: {e}"))?;
    if let Err(e)=writeln!(output,"{hash}").and_then(|_|output.sync_all()){
        drop(output);let _=fs::remove_file(&temp);
        return Err(format!("Cannot write CDDA digest receipt: {e}"));
    }
    drop(output);
    if path.exists(){
        // Windows cannot rename over an existing target. Temporarily retire
        // it and restore on failure, never follow a symlink.
        let retired=game.join("CyberGladiatorsFix.sha256.previous.tmp");
        if retired.exists()||retired.is_symlink(){let _=fs::remove_file(&temp);return Err("CDDA receipt backup staging path exists".into());}
        if let Err(e)=fs::rename(&path,&retired){let _=fs::remove_file(&temp);return Err(format!("Cannot retire CDDA receipt: {e}"));}
        if let Err(e)=fs::rename(&temp,&path){let restore=fs::rename(&retired,&path);let _=fs::remove_file(&temp);return Err(format!("Cannot update CDDA receipt: {e}; restore={restore:?}"));}
        let _=fs::remove_file(&retired);
    } else if let Err(e)=fs::rename(&temp,&path){
        let _=fs::remove_file(&temp);
        return Err(format!("Cannot install CDDA receipt: {e}"));
    }
    Ok(())
}
fn prior_version_backup(game:&Path,hash:&str)->std::path::PathBuf {
    // Deterministic per-version backup: does not conflict with the existing
    // .previous.bak created when v0.2.1 was updated, and never overwrites it.
    game.join(format!("CyberGladiatorsFix.dll.previous-{}.bak",&hash[..12]))
}
/// Fail safely with the exact path that needs inspection. A .tmp file may
/// be the only recoverable copy of an interrupted upgrade: never remove it
/// automatically just to make a later attempt succeed.
fn require_vacant(path:&Path,role:&str)->Result<(),String>{
    match fs::symlink_metadata(path){
        Ok(_)=>Err(format!("{role} already exists at {}. Preserve and inspect this file; refusing overwrite",path.display())),
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(()),
        Err(e)=>Err(format!("Cannot inspect {role} at {}: {e}",path.display())),
    }
}
/// A previously completed upgrade may already have saved the *same* old DLL
/// under its stable hash-qualified filename. Reuse only when it is a normal,
/// valid x86 DLL with the full expected SHA-256, never merely a hash prefix.
/// In that case use a short-lived rollback path for this transaction.
fn existing_backup_matches(path:&Path,expected:&str)->Result<bool,String>{
    match fs::symlink_metadata(path){
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(false),
        Err(e)=>Err(format!("Cannot inspect existing CDDA backup at {}: {e}",path.display())),
        Ok(meta)=>{
            if !meta.is_file()||meta.file_type().is_symlink(){
                return Err(format!("CDDA backup path is not a regular file: {}. Refusing overwrite",path.display()));
            }
            check(path).map_err(|e|format!("Existing CDDA backup at {} is invalid: {e}. Refusing overwrite",path.display()))?;
            let actual=digest(path)?;
            if actual!=expected{
                return Err(format!("CDDA backup at {} has unexpected SHA-256 (expected {expected}; actual {actual}). Refusing overwrite",path.display()));
            }
            Ok(true)
        }
    }
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
fn install_new(source:&[u8],target:&Path,expected:&str)->Result<(),String>{
    validate_bytes(source)?;
    let mut output=OpenOptions::new().write(true).create_new(true).open(target)
        .map_err(|e|format!("Cannot create {}: {e}",target.display()))?;
    if let Err(e)=output.write_all(source).and_then(|_|output.flush()).and_then(|_|output.sync_all()){
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
/// Install or upgrade with SHA-qualified, non-overwriting backup and rollback.
/// Unknown valid x86 DLLs are preserved before replacement; no CLI approval
/// is required, and a malformed receipt still blocks modification.
pub fn try_install(root:&Path,game:&Path)->Result<(),String>{
    let source=source_bytes(root)?;
    validate_bytes(&source)?;
    let wanted=format!("{:x}",Sha256::digest(&source));
    if broken(&wanted){
        return Err(format!(
            "The {DLL_NAME} beside the patcher is from the broken v0.2.0/v0.2.1 build; replace BOTH files with the new ZIP before upgrading."
        ));
    }
    let target=game.join(DLL_NAME);
    if !target.exists() && !target.is_symlink(){
        install_new(&source,&target,&wanted)?;
        return record_receipt(game,&wanted);
    }
    check(&target)?;
    let existing=digest(&target)?;
    // Validate any existing receipt before touching a DLL or its backup.
    read_receipt(game)?;
    if existing==wanted{
        record_receipt(game,&existing)?;
        return Ok(());
    }
    // Upgrade an older or unknown valid x86 bridge DLL. Preserve the exact
    // prior version under
    // a unique hash-qualified backup; never overwrite previous backups.
    let staged=game.join("CyberGladiatorsFix.dll.updating.tmp");
    let backup=prior_version_backup(game,&existing);
    require_vacant(&staged,"CDDA upgrade staging file")?;
    let reuse_backup=existing_backup_matches(&backup,&existing)?;
    // A matching archived copy already preserves the exact old DLL. Avoid
    // deleting or overwriting it: temporarily retire the *installed* copy,
    // and remove only the temporary duplicate after successful publication.
    let rollback=game.join("CyberGladiatorsFix.dll.rollback.tmp");
    if reuse_backup {require_vacant(&rollback,"CDDA upgrade rollback file")?;}
    let retired=if reuse_backup{&rollback}else{&backup};
    install_new(&source,&staged,&wanted)?;
    if let Err(e)=fs::rename(&target,retired){
        let _=fs::remove_file(&staged);
        return Err(format!("Cannot preserve currently installed CDDA DLL at {} to {}: {e}",target.display(),retired.display()));
    }
    if let Err(e)=fs::rename(&staged,&target){
        let restored=fs::rename(retired,&target);
        let _=fs::remove_file(&staged);
        return Err(format!("Cannot install new CDDA DLL at {}: {e}; restore from {}: {restored:?}",target.display(),retired.display()));
    }
    // Receipt write or final validation may fail independently of DLL copy.
    // In that case roll back, preserving the original SHA-qualified backup.
    if let Err(e)=verify(game).and_then(|_|record_receipt(game,&wanted)){
        let removed=fs::remove_file(&target);
        let restored=if removed.is_ok(){fs::rename(retired,&target)}
            else{Err(std::io::Error::other("cannot remove new DLL to restore prior version"))};
        return Err(format!("CDDA upgrade validation/receipt failed: {e}; remove new DLL={removed:?}; restore prior version from {}={restored:?}",retired.display()));
    }
    if reuse_backup {
        // This duplicate was created by *this* transaction; the original
        // version is still safely preserved in the verified backup file.
        if let Err(e)=fs::remove_file(&rollback){
            crate::show(&format!("CDDA_UPGRADE_WARNING installed new DLL, but temporary duplicate at {} could not be removed: {e}; preserve it for review",rollback.display()),root);
        }else{
            crate::show(&format!("CDDA_UPGRADE reused verified existing backup at {} (SHA-256 {existing}); no archived backup overwritten",backup.display()),root);
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dll_name_is_consistent_with_game_import_patch(){
        assert_eq!(DLL_NAME,"CyberGladiatorsFix.dll");
        assert_eq!(DLL_NAME.len()+1,23);
    }
    #[test]
    fn identifies_exact_previously_distributed_broken_bridges(){
        assert!(broken("055e634eb9f9472978ce930eabf38b0d8790542df7fceba1e8c9d61876462f71"));
        assert!(broken("d9cb494e83bb0e8552f238ceddbe048a7a0ab8afae585c853b5c3a0d7858bdbb"));
        assert!(!broken("71c8cb080a8d11001a91c370d2248dd9819cfd00b4b919cd7bbcb7699a96eedb"));
    }
    #[test]
    fn keeps_distinct_backup_files_for_prior_versions(){
        let game=Path::new("C:/Cyber");
        let a=prior_version_backup(game,"567e748733018c8ae91c1f31e6e90f30abebbf33cf838ea4a90e7aeeeeb055d0");
        let b=prior_version_backup(game,"acb07fcd2bfe789eae5927a9ab1730563a73948bb9aaaed2fc4e95c97c21df13");
        assert_ne!(a,b);
        assert!(a.to_string_lossy().contains("567e74873301"));
        assert_ne!(a,game.join("CyberGladiatorsFix.dll.previous.bak"));
    }
    #[test]
    fn upgrade_reuses_verified_backup_from_previous_cycle(){
        let root=std::env::temp_dir().join(format!("cyber-cdda-repeated-upgrade-{}",std::process::id()));
        let _=fs::remove_dir_all(&root);
        let game=root.join("CyberGladiatorsFix");
        fs::create_dir_all(&game).unwrap();
        let source=root.join(DLL_NAME);
        let target=game.join(DLL_NAME);
        let mut old=vec![0u8;4096];
        old[0..2].copy_from_slice(b"MZ");
        old[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        old[0x80..0x84].copy_from_slice(b"PE\0\0");
        old[0x84..0x86].copy_from_slice(&0x014cu16.to_le_bytes());
        old[0x96..0x98].copy_from_slice(&0x2000u16.to_le_bytes());
        old[0x98..0x9a].copy_from_slice(&0x010bu16.to_le_bytes());
        let mut new=old.clone();
        new[1024]=0x42;
        fs::write(&target,&old).unwrap();
        fs::write(&source,&new).unwrap();
        let old_hash=digest(&target).unwrap();
        let saved=prior_version_backup(&game,&old_hash);
        fs::write(&saved,&old).unwrap();
        // This was the collision that previously prevented the user
        // from upgrading their game DLL on a subsequent cycle.
        try_install(&root,&game).unwrap();
        assert_eq!(fs::read(&target).unwrap(),new);
        assert_eq!(fs::read(&saved).unwrap(),old);
        assert_eq!(read_receipt(&game).unwrap().as_deref(),Some(digest(&source).unwrap().as_str()));
        assert!(!game.join("CyberGladiatorsFix.dll.rollback.tmp").exists());
        assert!(!game.join("CyberGladiatorsFix.dll.updating.tmp").exists());
        // Idempotence: running the same ZIP again is safe.
        try_install(&root,&game).unwrap();
        assert_eq!(fs::read(&saved).unwrap(),old);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn refuses_colliding_staging_file_with_its_exact_path(){
        let dir=std::env::temp_dir().join(format!("cyber-cdda-collision-{}",std::process::id()));
        let _=fs::remove_dir_all(&dir);
        fs::create_dir(&dir).unwrap();
        let staged=dir.join("CyberGladiatorsFix.dll.updating.tmp");
        assert!(require_vacant(&staged,"CDDA upgrade staging file").is_ok());
        fs::write(&staged,b"preserve").unwrap();
        let err=require_vacant(&staged,"CDDA upgrade staging file").unwrap_err();
        assert!(err.contains(&staged.display().to_string()));
        assert_eq!(fs::read(&staged).unwrap(),b"preserve");
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn matching_old_dll_backup_is_reused_without_overwriting_it(){
        let dir=std::env::temp_dir().join(format!("cyber-cdda-archived-{}",std::process::id()));
        let _=fs::remove_dir_all(&dir);
        fs::create_dir(&dir).unwrap();
        let backup=dir.join("previous.bak");
        assert!(!existing_backup_matches(&backup,"0123456789").unwrap());
        // Minimal valid PE32 x86 DLL header, matching our 4-KiB verifier.
        let mut data=vec![0u8;4096];
        data[0..2].copy_from_slice(b"MZ");
        data[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        data[0x80..0x84].copy_from_slice(b"PE\0\0");
        data[0x84..0x86].copy_from_slice(&0x014cu16.to_le_bytes());
        data[0x96..0x98].copy_from_slice(&0x2000u16.to_le_bytes());
        data[0x98..0x9a].copy_from_slice(&0x010bu16.to_le_bytes());
        fs::write(&backup,&data).unwrap();
        let hash=digest(&backup).unwrap();
        assert!(existing_backup_matches(&backup,&hash).unwrap());
        let mismatch=existing_backup_matches(&backup,&"0".repeat(64)).unwrap_err();
        assert!(mismatch.contains(&backup.display().to_string()));
        assert!(mismatch.contains(&hash));
        assert_eq!(fs::read(&backup).unwrap(),data);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn receipt_roundtrip_and_malformed_rejected(){
        let root=std::env::temp_dir().join(format!("cdda-receipt-test-{}",std::process::id()));
        let _=fs::remove_dir_all(&root);fs::create_dir(&root).unwrap();
        let hash="5c5e1b4e21c7964364846cb811a3263ca0285d394a5e35f348c7450a463c735c";
        assert_eq!(read_receipt(&root).unwrap(),None);
        record_receipt(&root,hash).unwrap();
        assert_eq!(read_receipt(&root).unwrap().as_deref(),Some(hash));
        record_receipt(&root,hash).unwrap();
        fs::write(root.join(RECEIPT_NAME),"not-an-integrity-receipt").unwrap();
        assert!(read_receipt(&root).is_err());
        fs::remove_dir_all(root).unwrap();
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
