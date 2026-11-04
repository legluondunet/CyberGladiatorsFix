// SPDX-License-Identifier: GPL-3.0-or-later
//! SHA-256-gated, byte-exact retail speed fix and no-CD check for CYBER.EXE.
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

pub const ORIGINAL: &str = "226a04a745addbfe77709aa7ec56bf5c17792c524b31882da7653e1973775624";
pub const SPEED_ONLY: &str = "520d516a572f3287458337e8cef1231b5645f59e3952b3f568a2aab8f71af832";
pub const PATCHED: &str = "d397053de4b1a8573bd851915e7aeb3773f3b7010a08b85162c28ea94bccdaf9";
pub const LEN: usize = 351_776;
const CHANGES: [(usize, [u8;4], [u8;4]); 2] = [
    (0x3116, [0xc4,0xff,0xff,0xff], [0x0e,0x30,0x04,0x00]),
    (0x3125, [0xb5,0xff,0xff,0xff], [0xff,0x2f,0x04,0x00]),
];
// CYBER.EXE v1.0: CALL 0x40536A at VA 0x405416 (raw offset 0x4A16).
// That routine scans CD-ROM drives for a volume labeled "CYBER".
// Replace only this CALL with MOV EAX,1. The following TEST EAX,EAX
// then falls through instead of looping through the "insert CD" dialog.
// This does not emulate physical CD audio, and gameplay is not yet validated.
const CD_CHECK: (usize, [u8;5], [u8;5]) = (
    0x4a16,
    [0xe8,0x4f,0xff,0xff,0xff],
    [0xb8,0x01,0x00,0x00,0x00],
);

/// Validate all byte preimages before changing anything.
fn patch_bytes(data: &mut [u8], already_speed_fixed: bool) -> Result<(),String> {
    if data.len()!=LEN {return Err(format!("Unexpected CYBER.EXE size: {}",data.len()));}
    for (offset,original,speed) in CHANGES {
        let expected=if already_speed_fixed {speed} else {original};
        if data[offset..offset+4]!=expected || data[offset-1]!=0xe8 {
            return Err(format!("Unexpected timing instructions at 0x{offset:X}; patch refused"));
        }
    }
    let (cd_offset, cd_old, cd_new)=CD_CHECK;
    if data[cd_offset..cd_offset+5]!=cd_old {
        return Err(format!("Unexpected CD-check instructions at 0x{cd_offset:X}; patch refused"));
    }
    if !already_speed_fixed {
        for (offset,_,replacement) in CHANGES {
            data[offset..offset+4].copy_from_slice(&replacement);
        }
    }
    data[cd_offset..cd_offset+5].copy_from_slice(&cd_new);
    Ok(())
}

pub fn digest(data: &[u8]) -> String { format!("{:x}", Sha256::digest(data)) }
pub fn file_hash(path:&Path)->Result<String,String>{
    let data=fs::read(path).map_err(|e|format!("{}: {e}",path.display()))?;
    Ok(digest(&data))
}
pub fn backup_path(path:&Path)->std::path::PathBuf {
    let mut name=path.file_name().unwrap_or_default().to_os_string();
    name.push(".bak");path.with_file_name(name)
}
/// Refuse unknown editions, missing/invalid original backups, and unexpected bytes.
pub fn verify_exe(path:&Path)->Result<(),String>{
    let meta=fs::symlink_metadata(path).map_err(|e|e.to_string())?;
    if !meta.is_file()||meta.file_type().is_symlink(){
        return Err(format!("CYBER.EXE must be a regular file: {}",path.display()));
    }
    let data=fs::read(path).map_err(|e|e.to_string())?;
    let hash=digest(&data);
    if hash!=PATCHED {return Err(format!("Unsupported or unpatched CYBER.EXE: SHA-256 {hash}; expected {PATCHED}"));}
    let backup=backup_path(path);
    let backup_meta=fs::symlink_metadata(&backup)
        .map_err(|e|format!("Original backup missing ({}): {e}",backup.display()))?;
    if !backup_meta.is_file()||backup_meta.file_type().is_symlink(){
        return Err(format!("Invalid or symlinked original backup: {}",backup.display()));
    }
    let src=fs::read(&backup).map_err(|e|format!("Original backup missing ({}): {e}",backup.display()))?;
    if digest(&src)!=ORIGINAL{return Err(format!("Original backup is incompatible: {}",backup.display()));}
    Ok(())
}

pub fn apply(path:&Path)->Result<&'static str,String>{
    if path.file_name().is_none_or(|n|!n.to_string_lossy().eq_ignore_ascii_case("CYBER.EXE")){
        return Err("The executable must be named CYBER.EXE".into());
    }
    let meta=fs::symlink_metadata(path).map_err(|e|e.to_string())?;
    if !meta.is_file()||meta.file_type().is_symlink(){return Err("CYBER.EXE must be a regular file".into());}
    let data=fs::read(path).map_err(|e|e.to_string())?;
    let hash=digest(&data);
    if hash==PATCHED {verify_exe(path)?;return Ok("Already patched (verified)");}
    let already_speed_fixed=hash==SPEED_ONLY;
    if (hash!=ORIGINAL && !already_speed_fixed) || data.len()!=LEN {
        return Err(format!("Unsupported CYBER.EXE: SHA-256 {hash} ({} bytes). Executable unchanged.",data.len()));
    }
    let mut patched=data.clone();
    patch_bytes(&mut patched,already_speed_fixed)?;
    if digest(&patched)!=PATCHED {return Err("Internal checksum mismatch: patch refused".into());}
    let backup=backup_path(path);
    if backup.exists() {
        if file_hash(&backup)? != ORIGINAL {return Err("Existing .bak has unexpected contents; refusing to overwrite".into());}
    } else {
        // An existing speed-only installation must already have the retail
        // original backup; it is unsafe to back up the speed-patched bytes
        // under a name that claims to contain the untouched original.
        if already_speed_fixed {
            return Err("Original CYBER.EXE.bak missing; cannot upgrade speed-only installation safely".into());
        }
        // create_new never overwrites user files; sync backup before touching exe.
        use std::io::Write;
        let mut f=fs::OpenOptions::new().write(true).create_new(true).open(&backup)
            .map_err(|e|format!("Cannot create original backup: {e}"))?;
        f.write_all(&data).map_err(|e|e.to_string())?;
        f.sync_all().map_err(|e|e.to_string())?;
    }
    let tmp=path.with_extension("EXE.tmp");
    if tmp.exists(){return Err(format!("Temporary patch file already exists: {}",tmp.display()));}
    {
        use std::io::Write;
        let mut f=fs::OpenOptions::new().write(true).create_new(true).open(&tmp).map_err(|e|e.to_string())?;
        if let Err(e)=f.write_all(&patched).and_then(|_|f.sync_all()) {
            let _=fs::remove_file(&tmp);return Err(e.to_string());
        }
    }
    // Windows rename cannot replace an existing EXE. In a staged installation
    // this operation is safe; on an existing installation, original .bak survives.
    let retired=path.with_extension("EXE.old.tmp");
    if retired.exists(){let _=fs::remove_file(&tmp);return Err("Temporary old EXE already exists; refusing".into());}
    fs::rename(path,&retired).map_err(|e|format!("Cannot move original aside: {e}"))?;
    if let Err(e)=fs::rename(&tmp,path){
        let restore=fs::rename(&retired,path);
        return Err(format!("Cannot install patched exe: {e}; original restore: {restore:?}; original backup: {}",backup.display()));
    }
    let _=fs::remove_file(&retired);
    verify_exe(path)?;
    Ok(if already_speed_fixed {
        "No-CD check added to existing speed fix"
    } else {
        "Speed + no-CD check applied (QueryPerformanceCounter -> GetTickCount)"
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn patch_addresses_point_to_gettickcount(){
        // CODE section starts at VA 0x401000, raw offset 0x600.
        for (offset,_,new) in CHANGES {
            let call_va=0x401000u64 + offset as u64-0x600 - 1;
            let target=(call_va+5) as i64 + i32::from_le_bytes(new) as i64;
            assert_eq!(target,0x446b28);
        }
    }
    #[test] fn original_hash_is_distinct(){
        assert_ne!(ORIGINAL,SPEED_ONLY);
        assert_ne!(SPEED_ONLY,PATCHED);
        assert_ne!(ORIGINAL,PATCHED);
    }
    #[test] fn cd_check_patch_skips_volume_label_probe(){
        // CODE section RVA/offset mapping: VA 0x405416 => raw offset 0x4A16.
        assert_eq!(0x401000 + CD_CHECK.0 - 0x600,0x405416);
        assert_eq!(CD_CHECK.1,[0xe8,0x4f,0xff,0xff,0xff]);
        assert_eq!(CD_CHECK.2,[0xb8,0x01,0x00,0x00,0x00]); // mov eax,1
    }
    fn synthetic_original()->Vec<u8>{
        let mut data=vec![0u8;LEN];
        for (offset,old,_) in CHANGES {
            data[offset-1]=0xe8;
            data[offset..offset+4].copy_from_slice(&old);
        }
        data[CD_CHECK.0..CD_CHECK.0+5].copy_from_slice(&CD_CHECK.1);
        data
    }
    #[test] fn patches_original_in_one_pass(){
        let mut data=synthetic_original();
        patch_bytes(&mut data,false).unwrap();
        for (offset,_,new) in CHANGES {assert_eq!(&data[offset..offset+4],&new);}
        assert_eq!(&data[CD_CHECK.0..CD_CHECK.0+5],&CD_CHECK.2);
    }
    #[test] fn upgrades_speed_only_without_touching_timing_patches(){
        let mut data=synthetic_original();
        for (offset,_,new) in CHANGES {data[offset..offset+4].copy_from_slice(&new);}
        let speed_before=data[0x3110..0x3130].to_vec();
        patch_bytes(&mut data,true).unwrap();
        assert_eq!(&data[0x3110..0x3130],speed_before.as_slice());
        assert_eq!(&data[CD_CHECK.0..CD_CHECK.0+5],&CD_CHECK.2);
    }
    #[test] fn refuses_unexpected_cd_check_without_partial_mutation(){
        let mut data=synthetic_original();
        data[CD_CHECK.0]=0x90;
        let unchanged=data.clone();
        let err=patch_bytes(&mut data,false).unwrap_err();
        assert!(err.contains("CD-check"));
        assert_eq!(data,unchanged);
    }
}
