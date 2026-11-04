// SPDX-License-Identifier: GPL-3.0-or-later
//! Place the separately built, project-owned x86 CDDA bridge alongside
//! CYBER.EXE, never in the Wine or Windows system directories.
use std::fs::{self,File,OpenOptions};
use std::borrow::Cow;
use std::io::{Read,Write};
use std::path::Path;
use sha2::{Digest,Sha256};

pub const DLL_NAME:&str="CyberGladiatorsCDDA.dll";
const RECEIPT_NAME:&str="CyberGladiatorsCDDA.sha256";
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
const KNOWN_RELEASED_BRIDGES:[&str;24]=[
    "3f0e0a17aa86fd61c2d52a3341251b3c4e9e03fb08cbb1664f50f2a630d33b29", // forwarder validation ZIP
    "567e748733018c8ae91c1f31e6e90f30abebbf33cf838ea4a90e7aeeeeb055d0", // v0.2.2
    "acb07fcd2bfe789eae5927a9ab1730563a73948bb9aaaed2fc4e95c97c21df13", // v0.2.3
    "a0c93deff6981d34dd3cd964a0666d958f59c65ad3c8efe0bc2c09fd8e03d3d4", // v0.2.4
    "23b5ddba39763d5626796ed94b82ba8d17422751a4aac1f3edea0276f7b6a666", // v0.2.5
    "42df88a0aa88f1cfd24048d02df13478b4435af4d2d8ed4b53443fe334fc7af2", // v0.2.6
    "6d4ffd7ca461b5e41b0bf1ac3cda898d343284febe3085c4e1a40f832defbc6d", // v0.2.7, verified ZIP
    "abf5c9d60ff20d0dc5816c742fbc04ca8a1226385b45fe375288a52d96c9dfca", // v0.2.8, verified ZIP
    "a95c5787f3a9b5dc101e673777111257fcf0482bf4dce8340cb5b91b00f5026a", // v0.2.9, verified ZIP
    // Additional first-party GitHub Actions archives, exact DLL bytes
    // downloaded and SHA-256 checked (not guessed from version numbers).
    "dc305f22d37906ff964b9f382dd48034e5f92cd8ba733e80733fa085499c98cc", // CI run 80 (v0.2.7 intermediate)
    "00e184c0a23a1c8cf6ae3ab53c503f49a446357ebf6b7bdce75e354581f790db", // CI run 81 (v0.2.8 intermediate)
    "5825f1a5197dbb9f023b64cada5a5f576b15e2208c8054d9ae3301e64b6aa80f", // CI run 82 (v0.2.8 intermediate)
    "77b267074e1f3369afe739f9fccfd3e56a90feeeb30ad5ee5bc760afa5665fcb", // CI run 83 (v0.2.8 intermediate)
    "416ee060d20d221cbfd82d7f20e7db6f88a453de84125af6fe571d0942b1d7ba", // CI run 84 (v0.2.8 intermediate)
    "18be076cc930991808881e0647cdf2a2b011e3c735ee68671f9b98ab443eb679", // CI run 85 (v0.2.8 intermediate)
    "f7b26be8cf206e19b80c6418477804b6d25aaef4952e90b75bf7491506c54ab8", // CI run 87 (v0.2.8 diagnostic) USER INSTALLED
    "74767b741163f3a6424ec3f4b6d100df35c676f5095af53ff45c8792ad3e01da", // CI run 88 (v0.2.9 intermediate)
    "4a95a4185838ee5b2f6e3c279f17507fef69315d9d35478aed68bde488309e9c", // CI run 90 (v0.2.9 intermediate)
    "22103e79824f21907fdf55d0e46eda840bfa812123d446df4056a0bd3dc69ac7", // CI run 91 (v0.2.9 intermediate)
    "66bb6ecc7ba294fbdecd5e0d4888e8dfd4a8bc531fd2718959244325b6095778", // CI run 92 (v0.2.9 intermediate)
    "c1284c8ad3544ff774c62ec4adbf44cbc9d5b6f010e266e3e112b1510c93279a", // v0.2.10 verified ZIP
    "c1e9d05936d627c09e4e4d3eaa54d46f0818687f5f3d0affd67ca2e7da754c40", // v0.2.11, archive-verified ZIP
    "3d816b5f213a328ccb5b4a3a278c8650c1f304d6d91df555901f975faf1ea90a", // v0.2.12, verified ZIP (matches user log)
    "451f7fc3715ac39428779943e24ac823de42eef31d2e80077ad2899269e4066c", // preceding first-party test DLL; bundled/installed equality confirmed by diagnostic log
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
fn has_noop_capability(bytes:&[u8])->bool{
    [b"CYBER_GLIDE_PASSTHRU_NOOP".as_slice(),b"PASSTHRU BYPASSED mode=1".as_slice()]
        .into_iter().all(|needle|bytes.windows(needle.len()).any(|window|window==needle))
}
/// For a pass-through A/B test both the launcher-supplied and installed
/// x86 DLLs must be byte-identical. Otherwise an older installed proxy can
/// silently execute the non-NOOP nGlide function and invalidate the test.
pub fn matching_installed_hash(root:&Path,game:&Path)->Result<String,String>{
    let source=source_bytes(root)?;
    let installed=game.join(DLL_NAME);
    validate_bytes(&source)?;
    check(&installed)?;
    if !has_noop_capability(&source){
        return Err(format!(
            "Diagnostic cancelled: x86 {DLL_NAME} beside the patcher does not contain the pass-through NO-OP capability. Use both files from the same current Windows ZIP; no CD extraction is needed."
        ));
    }
    let expected=format!("{:x}",Sha256::digest(&source));
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
// A locally recorded digest is an integrity receipt, not a signature or
// proof of publisher identity. Unknown pre-receipt DLLs require explicit
// one-time consent; subsequent upgrades can verify the installed bytes.
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
    let temp=game.join("CyberGladiatorsCDDA.sha256.tmp");
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
        let retired=game.join("CyberGladiatorsCDDA.sha256.previous.tmp");
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
fn allows_upgrade(existing:&str,receipt:Option<&str>,consent:bool)->bool{
    consent || replaceable_known_release(existing) || receipt==Some(existing)
}
fn prior_version_backup(game:&Path,hash:&str)->std::path::PathBuf {
    // Deterministic per-version backup: does not conflict with the existing
    // .previous.bak created when v0.2.1 was updated, and never overwrites it.
    game.join(format!("CyberGladiatorsCDDA.dll.previous-{}.bak",&hash[..12]))
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
pub fn try_install(root:&Path,game:&Path)->Result<(),String>{
    try_install_with_consent(root,game,false)
}
pub fn try_install_with_consent(root:&Path,game:&Path,consent:bool)->Result<(),String>{
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
    let receipt=read_receipt(game)?;
    if existing==wanted{
        // Never silently bless a foreign, unrecognized DLL. The command
        // --upgrade-cdda grants explicit approval to establish the receipt.
        if allows_upgrade(&existing,receipt.as_deref(),consent){
            record_receipt(game,&existing)?;
        }
        return Ok(());
    }
    if !allows_upgrade(&existing,receipt.as_deref(),consent){
        return Err(format!(
            "Existing {DLL_NAME} has an unrecognized SHA-256 ({existing}); refusing to overwrite. Run --upgrade-cdda to explicitly approve a backed-up replacement of this x86 DLL."
        ));
    }
    // Upgrade any recognized older project-owned DLL, even if it was not one
    // of the original broken builds. Preserve the exact prior version under
    // a unique hash-qualified backup; never overwrite previous backups.
    let staged=game.join("CyberGladiatorsCDDA.dll.updating.tmp");
    let backup=prior_version_backup(game,&existing);
    require_vacant(&staged,"CDDA upgrade staging file")?;
    let reuse_backup=existing_backup_matches(&backup,&existing)?;
    // A matching archived copy already preserves the exact old DLL. Avoid
    // deleting or overwriting it: temporarily retire the *installed* copy,
    // and remove only the temporary duplicate after successful publication.
    let rollback=game.join("CyberGladiatorsCDDA.dll.rollback.tmp");
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
    fn diagnostic_bridge_requires_noop_markers(){
        let capable=b"CYBER_GLIDE_PASSTHRU_NOOP abc PASSTHRU BYPASSED mode=1";
        let old=b"CYBER_GLIDE_TRACE abc PASSTHRU BEGIN mode=1";
        assert!(has_noop_capability(capable));
        assert!(!has_noop_capability(old));
        assert!(!has_noop_capability(b"CYBER_GLIDE_PASSTHRU_NOOP"));
        assert!(!has_noop_capability(b"PASSTHRU BYPASSED mode=1"));
    }
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
            ("dc305f22d37906ff964b9f382dd48034e5f92cd8ba733e80733fa085499c98cc","CI run 80 (v0.2.7 intermediate)"),
            ("00e184c0a23a1c8cf6ae3ab53c503f49a446357ebf6b7bdce75e354581f790db","CI run 81 (v0.2.8 intermediate)"),
            ("5825f1a5197dbb9f023b64cada5a5f576b15e2208c8054d9ae3301e64b6aa80f","CI run 82 (v0.2.8 intermediate)"),
            ("77b267074e1f3369afe739f9fccfd3e56a90feeeb30ad5ee5bc760afa5665fcb","CI run 83 (v0.2.8 intermediate)"),
            ("416ee060d20d221cbfd82d7f20e7db6f88a453de84125af6fe571d0942b1d7ba","CI run 84 (v0.2.8 intermediate)"),
            ("18be076cc930991808881e0647cdf2a2b011e3c735ee68671f9b98ab443eb679","CI run 85 (v0.2.8 intermediate)"),
            ("f7b26be8cf206e19b80c6418477804b6d25aaef4952e90b75bf7491506c54ab8","CI run 87 (v0.2.8 diagnostic) USER INSTALLED"),
            ("74767b741163f3a6424ec3f4b6d100df35c676f5095af53ff45c8792ad3e01da","CI run 88 (v0.2.9 intermediate)"),
            ("4a95a4185838ee5b2f6e3c279f17507fef69315d9d35478aed68bde488309e9c","CI run 90 (v0.2.9 intermediate)"),
            ("22103e79824f21907fdf55d0e46eda840bfa812123d446df4056a0bd3dc69ac7","CI run 91 (v0.2.9 intermediate)"),
            ("66bb6ecc7ba294fbdecd5e0d4888e8dfd4a8bc531fd2718959244325b6095778","CI run 92 (v0.2.9 intermediate)"),
            ("c1284c8ad3544ff774c62ec4adbf44cbc9d5b6f010e266e3e112b1510c93279a","v0.2.10 verified ZIP"),
            ("c1e9d05936d627c09e4e4d3eaa54d46f0818687f5f3d0affd67ca2e7da754c40","v0.2.11 verified ZIP"),
            ("3d816b5f213a328ccb5b4a3a278c8650c1f304d6d91df555901f975faf1ea90a","v0.2.12 verified ZIP"),
            ("451f7fc3715ac39428779943e24ac823de42eef31d2e80077ad2899269e4066c","first-party trace build, matching local pair"),
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
        try_install_with_consent(&root,&game,true).unwrap();
        assert_eq!(fs::read(&target).unwrap(),new);
        assert_eq!(fs::read(&saved).unwrap(),old);
        assert_eq!(read_receipt(&game).unwrap().as_deref(),Some(digest(&source).unwrap().as_str()));
        assert!(!game.join("CyberGladiatorsCDDA.dll.rollback.tmp").exists());
        assert!(!game.join("CyberGladiatorsCDDA.dll.updating.tmp").exists());
        // Idempotence: running the same ZIP again is safe.
        try_install_with_consent(&root,&game,true).unwrap();
        assert_eq!(fs::read(&saved).unwrap(),old);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn refuses_colliding_staging_file_with_its_exact_path(){
        let dir=std::env::temp_dir().join(format!("cyber-cdda-collision-{}",std::process::id()));
        let _=fs::remove_dir_all(&dir);
        fs::create_dir(&dir).unwrap();
        let staged=dir.join("CyberGladiatorsCDDA.dll.updating.tmp");
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
    fn never_auto_overwrites_unknown_dll(){
        assert!(!replaceable_known_release("f9b9b6f601e5986d46d1f672fbbd010ca93dafb8087182b55a0fbf7f4ad8ce4d"));
        assert!(replaceable_known_release(BROKEN_BRIDGES[0]));
    }
    #[test]
    fn receipt_matches_only_the_exact_installed_digest(){
        let old="5c5e1b4e21c7964364846cb811a3263ca0285d394a5e35f348c7450a463c735c";
        let next="451f7fc3715ac39428779943e24ac823de42eef31d2e80077ad2899269e4066c";
        assert!(!allows_upgrade(old,None,false));
        assert!(allows_upgrade(old,None,true));
        assert!(allows_upgrade(old,Some(old),false));
        assert!(!allows_upgrade(old,Some(next),false));
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
