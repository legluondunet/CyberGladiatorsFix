// SPDX-License-Identifier: GPL-3.0-or-later
//! SHA-256-gated, byte-exact retail speed fix for CYBER.EXE.
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

pub const ORIGINAL: &str = "226a04a745addbfe77709aa7ec56bf5c17792c524b31882da7653e1973775624";
pub const PATCHED: &str = "520d516a572f3287458337e8cef1231b5645f59e3952b3f568a2aab8f71af832";
pub const LEN: usize = 351_776;
const CHANGES: [(usize, [u8;4], [u8;4]); 2] = [
    (0x3116, [0xc4,0xff,0xff,0xff], [0x0e,0x30,0x04,0x00]),
    (0x3125, [0xb5,0xff,0xff,0xff], [0xff,0x2f,0x04,0x00]),
];

pub fn digest(data: &[u8]) -> String { format!("{:x}", Sha256::digest(data)) }
pub fn file_hash(path:&Path)->Result<String,String>{
    let data=fs::read(path).map_err(|e|format!("{}: {e}",path.display()))?;
    Ok(digest(&data))
}
pub fn backup_path(path:&Path)->std::path::PathBuf {
    let mut name=path.file_name().unwrap_or_default().to_os_string();
    name.push(".bak");path.with_file_name(name)
}
/// Refuse unknown editions, already patched files without valid originals, and unexpected bytes.
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
    if hash!=ORIGINAL || data.len()!=LEN {
        return Err(format!("Unsupported CYBER.EXE: SHA-256 {hash} ({} bytes). Original file was not modified.",data.len()));
    }
    // Both calls jump to the existing GetTickCount import thunk at VA 0x00446B28.
    // This removes reliance on the old QueryPerformanceCounter / >>10 conversion.
    for (offset,old,_) in CHANGES {
        if data[offset..offset+4] != old {return Err(format!("Unexpected instructions at 0x{offset:X}; patch refused"));}
        if data[offset-1]!=0xe8 {return Err(format!("Missing CALL opcode before 0x{offset:X}"));}
    }
    let backup=backup_path(path);
    if backup.exists() {
        if file_hash(&backup)? != ORIGINAL {return Err("Existing .bak has unexpected contents; refusing to overwrite".into());}
    } else {
        // create_new never overwrites user files; sync backup before touching exe
        use std::io::Write;
        let mut f=fs::OpenOptions::new().write(true).create_new(true).open(&backup)
            .map_err(|e|format!("Cannot create original backup: {e}"))?;
        f.write_all(&data).map_err(|e|e.to_string())?;
        f.sync_all().map_err(|e|e.to_string())?;
    }
    let mut patched=data;
    for (offset,_,replacement) in CHANGES{patched[offset..offset+4].copy_from_slice(&replacement);}
    if digest(&patched)!=PATCHED{return Err("Internal checksum mismatch: patch refused".into());}
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
    Ok("Speed fix applied (QueryPerformanceCounter -> GetTickCount)")
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
    #[test] fn original_hash_is_distinct(){assert_ne!(ORIGINAL,PATCHED);}
}
