// SPDX-License-Identifier: GPL-3.0-or-later
//! Strict, edition-specific patch state machine for the verified French retail CD.
//! Never apply US offsets to this PE. Original French assets remain untouched.
//! All executable states are identified by exact precomputed SHA-256 hashes.
use crate::patch;
use std::fs::{self,OpenOptions};
use std::io::Write;
use std::path::Path;

pub const ORIGINAL:&str="12db5e29120832ebc304695ecfa38f6c0d825afe2640433714d27f624da53cdb";
pub const PATCHED:&str="8d70d1fb4b3be6e5a5e5cac132b3beafcc501482aaa9dd8217437fef72bb85f5";
pub const PATCHED_SKIP:&str="66f25c59b314a916a02f215b1dde48b5e820bb8f2c130418258fd1e27ba1ac15";
pub const SYSTEM_WINMM:&str="b87ffbe50ebb1162f97fce0c203c288d0cfa8f74a0c1743c8121d04b2eddcf0e";
pub const SYSTEM_WINMM_SKIP:&str="888d85ee215b2d902fce77fd67f18603db8598f207b358e64a911d7498fc0610";
pub const LEN:usize=352_288;
// French PE CODE: VA=0x401000+(raw-0x600), not VA=0x400000+raw.
// The replacement CALLs target the disassembled GetTickCount thunk 0x446B68.
const SPEED:[(usize,[u8;4],[u8;4]);2]=[
    (0x31d7,[0xc4,0xff,0xff,0xff],[0x8d,0x2f,0x04,0x00]),
    (0x31e6,[0xb5,0xff,0xff,0xff],[0x7e,0x2f,0x04,0x00]),
];
const CD_CHECK:(usize,[u8;5],[u8;5])=(
    0x4ad5,[0xe8,0x4c,0xff,0xff,0xff],[0xb8,0x01,0x00,0x00,0x00]
);
const INTRO_GATES:[(usize,[u8;2],[u8;2]);2]=[
    (0x63f5,[0x74,0x2f],[0x90,0x90]),
    (0x6409,[0x7c,0x0c],[0x90,0x90])
];
const SKIP_OFFSET:usize=0x63f5;
const WINMM_NAME_PTR:usize=0x4e634;
const WINMM_OLD_RVA:u32=0x3ed40c;
const WINMM_NEW_RVA:u32=0x3edb00;
const WINMM_SLOT:usize=0x4f100;
const BRIDGE_NAME:&[u8;24]=b"CyberGladiatorsFix.dll\0\0";

fn digest(data:&[u8])->String{patch::digest(data)}
fn recognized(hash:&str)->bool{
    matches!(hash,ORIGINAL|PATCHED|PATCHED_SKIP|SYSTEM_WINMM|SYSTEM_WINMM_SKIP)
}
fn valid_patched(hash:&str)->bool{
    matches!(hash,PATCHED|PATCHED_SKIP|SYSTEM_WINMM|SYSTEM_WINMM_SKIP)
}
fn safe_read(path:&Path)->Result<Vec<u8>,String>{
    let m=fs::symlink_metadata(path).map_err(|e|format!("Cannot inspect {}: {e}",path.display()))?;
    if !m.is_file()||m.file_type().is_symlink(){
        return Err(format!("French game file must be a regular file: {}",path.display()));
    }
    fs::read(path).map_err(|e|format!("Cannot read {}: {e}",path.display()))
}
/// Recognize a French original or a patched game with its original verified
/// backup. If a backup is exactly the French retail EXE, *always* use the
/// FR fail-closed path even if the running EXE has been unexpectedly changed.
pub fn owns(path:&Path)->Result<bool,String>{
    let hash=digest(&safe_read(path)?);
    if recognized(&hash){return Ok(true);}
    let backup=patch::backup_path(path);
    match fs::symlink_metadata(&backup){
        Ok(m)=>{
            if !m.is_file()||m.file_type().is_symlink(){
                return Err(format!("Invalid original backup: {}",backup.display()));
            }
            Ok(digest(&safe_read(&backup)?)==ORIGINAL)
        },
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(false),
        Err(e)=>Err(format!("Cannot inspect original backup: {e}"))
    }
}
fn valid_import(data:&[u8],bridge:bool)->bool{
    if data.len()!=LEN{return false;}
    let rva=u32::from_le_bytes(data[WINMM_NAME_PTR..WINMM_NAME_PTR+4].try_into().unwrap());
    let name=&data[WINMM_SLOT..WINMM_SLOT+24];
    if bridge{rva==WINMM_NEW_RVA && name==BRIDGE_NAME}
    else{rva==WINMM_OLD_RVA && name.iter().all(|v|*v==0)}
}
fn original_preimages(data:&[u8])->Result<(),String>{
    if data.len()!=LEN{return Err("Unexpected French CYBER.EXE size".into());}
    for (off,old,_) in SPEED{
        if data[off-1]!=0xe8||data[off..off+4]!=old{
            return Err(format!("French timing instruction mismatch at {off:#x}"));
        }
    }
    if data[CD_CHECK.0..CD_CHECK.0+5]!=CD_CHECK.1{
        return Err("French CD-check instruction mismatch".into());
    }
    for (off,old,_) in INTRO_GATES{
        if data[off..off+2]!=old{
            return Err(format!("French intro instruction mismatch at {off:#x}"));
        }
    }
    if !valid_import(data,false){
        return Err("French WINMM import descriptor or staging slot is unexpected".into());
    }
    Ok(())
}
/// Checked byte edits occur only after ALL preimages are validated.
fn patch_original(data:&mut [u8])->Result<(),String>{
    original_preimages(data)?;
    for (off,_,new) in SPEED{data[off..off+4].copy_from_slice(&new);}
    data[CD_CHECK.0..CD_CHECK.0+5].copy_from_slice(&CD_CHECK.2);
    for (off,_,new) in INTRO_GATES{data[off..off+2].copy_from_slice(&new);}
    data[WINMM_NAME_PTR..WINMM_NAME_PTR+4].copy_from_slice(&WINMM_NEW_RVA.to_le_bytes());
    data[WINMM_SLOT..WINMM_SLOT+24].copy_from_slice(BRIDGE_NAME);
    Ok(())
}
pub fn verify_exe(path:&Path)->Result<(),String>{
    let b=safe_read(path)?;
    let hash=digest(&b);
    if !valid_patched(&hash){
        return Err(format!("Unsupported French CYBER.EXE SHA-256 {hash}; unknown or unpatched binary"));
    }
    if !valid_import(&b,matches!(hash.as_str(),PATCHED|PATCHED_SKIP)){
        return Err("French WinMM import is not the expected verified variant".into());
    }
    if digest(&safe_read(&patch::backup_path(path))?)!=ORIGINAL{
        return Err("Original French CYBER.EXE.bak does not match verified retail CD".into());
    }
    Ok(())
}
fn replace_exe(path:&Path,content:&[u8])->Result<(),String>{
    let staged=path.with_extension("EXE.tmp");
    let retired=path.with_extension("EXE.old.tmp");
    for other in [&staged,&retired]{
        match fs::symlink_metadata(other){
            Ok(_)=>return Err(format!("Refusing to overwrite patch temporary file: {}",other.display())),
            Err(e) if e.kind()==std::io::ErrorKind::NotFound=>{},
            Err(e)=>return Err(format!("Cannot inspect {}: {e}",other.display()))
        }
    }
    let mut file=OpenOptions::new().create_new(true).write(true).open(&staged)
        .map_err(|e|format!("Cannot stage French patch: {e}"))?;
    if let Err(e)=file.write_all(content).and_then(|_|file.sync_all()){
        drop(file);let _=fs::remove_file(&staged);
        return Err(format!("Cannot stage French patch: {e}"));
    }
    drop(file);
    if let Err(e)=fs::rename(path,&retired){
        let _=fs::remove_file(&staged);
        return Err(format!("Cannot preserve previous French EXE: {e}"));
    }
    if let Err(e)=fs::rename(&staged,path){
        let restore=fs::rename(&retired,path);
        let _=fs::remove_file(&staged);
        return Err(format!("Cannot install French patch: {e}; rollback={restore:?}"));
    }
    if let Err(e)=fs::remove_file(&retired){
        // A successful replacement is usable, but never silently claim all
        // temporary files were removed when Windows denies deletion.
        return Err(format!("French EXE patched; previous temporary copy {} remains: {e}",retired.display()));
    }
    Ok(())
}
pub fn apply(path:&Path)->Result<&'static str,String>{
    if path.file_name().is_none_or(|n|!n.to_string_lossy().eq_ignore_ascii_case("CYBER.EXE")){
        return Err("The executable must be named CYBER.EXE".into());
    }
    let data=safe_read(path)?;
    let hash=digest(&data);
    if valid_patched(&hash){
        verify_exe(path)?;
        return Ok("French edition already patched (verified)");
    }
    if hash!=ORIGINAL{
        return Err(format!("Unsupported French CYBER.EXE SHA-256 {hash}; unchanged"));
    }
    let mut patched=data.clone();
    patch_original(&mut patched)?;
    if digest(&patched)!=PATCHED{
        return Err("French patched executable checksum mismatch; unchanged".into());
    }
    let backup=patch::backup_path(path);
    match fs::symlink_metadata(&backup){
        Ok(_)=>{
            if digest(&safe_read(&backup)?)!=ORIGINAL{
                return Err("French original backup is not the recognized retail executable".into());
            }
        },
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>{
            let mut file=OpenOptions::new().write(true).create_new(true).open(&backup)
                .map_err(|e|format!("Cannot create French original backup: {e}"))?;
            file.write_all(&data).and_then(|_|file.sync_all())
                .map_err(|e|format!("Cannot save French original backup: {e}"))?;
        },
        Err(e)=>return Err(format!("Cannot inspect French original backup: {e}")),
    }
    replace_exe(path,&patched)?;
    verify_exe(path)?;
    Ok("French retail speed/no-CD/intro and WinMM bridge patch applied")
}
pub fn cdda_active(path:&Path)->Result<bool,String>{
    verify_exe(path)?;
    let hash=digest(&safe_read(path)?);
    Ok(matches!(hash.as_str(),PATCHED|PATCHED_SKIP))
}
fn runtime_hash(skip:bool,bridge:bool)->&'static str{
    match (skip,bridge){
        (false,true)=>PATCHED,(true,true)=>PATCHED_SKIP,
        (false,false)=>SYSTEM_WINMM,(true,false)=>SYSTEM_WINMM_SKIP,
    }
}
pub fn set_cdda(path:&Path,bridge:bool)->Result<&'static str,String>{
    verify_exe(path)?;
    let original=safe_read(path)?;
    let hash=digest(&original);
    let skip=matches!(hash.as_str(),PATCHED_SKIP|SYSTEM_WINMM_SKIP);
    let currently=matches!(hash.as_str(),PATCHED|PATCHED_SKIP);
    if currently==bridge{return Ok("French WinMM import already selected");}
    if !valid_import(&original,currently){
        return Err("French import name unexpectedly changed".into());
    }
    let mut data=original;
    data[WINMM_NAME_PTR..WINMM_NAME_PTR+4].copy_from_slice(
        &if bridge{WINMM_NEW_RVA}else{WINMM_OLD_RVA}.to_le_bytes()
    );
    if bridge{data[WINMM_SLOT..WINMM_SLOT+24].copy_from_slice(BRIDGE_NAME);}
    else{data[WINMM_SLOT..WINMM_SLOT+24].fill(0);}
    if digest(&data)!=runtime_hash(skip,bridge){
        return Err("French WinMM import checksum mismatch; unchanged".into());
    }
    replace_exe(path,&data)?;
    verify_exe(path)?;
    Ok("French WinMM import updated")
}
pub fn set_skip_intro(path:&Path,skip:bool)->Result<&'static str,String>{
    verify_exe(path)?;
    let original=safe_read(path)?;
    let hash=digest(&original);
    let bridge=matches!(hash.as_str(),PATCHED|PATCHED_SKIP);
    let current_skip=matches!(hash.as_str(),PATCHED_SKIP|SYSTEM_WINMM_SKIP);
    if current_skip==skip{
        return Ok(if skip{"French startup intro already disabled"}else{"French startup intro already enabled"});
    }
    let mut data=original;
    let old=if current_skip{[0xeb,0x2f]}else{[0x90,0x90]};
    if data[SKIP_OFFSET..SKIP_OFFSET+2]!=old{
        return Err("French intro branch unexpectedly changed; unchanged".into());
    }
    data[SKIP_OFFSET..SKIP_OFFSET+2].copy_from_slice(
        &if skip{[0xeb,0x2f]}else{[0x90,0x90]}
    );
    if digest(&data)!=runtime_hash(skip,bridge){
        return Err("French intro toggle checksum mismatch; unchanged".into());
    }
    replace_exe(path,&data)?;
    verify_exe(path)?;
    Ok(if skip{"French startup intro disabled"}else{"French startup intro enabled"})
}
#[cfg(test)]
mod tests{
    use super::*;
    fn sample()->Vec<u8>{
        let mut data=vec![0u8;LEN];
        for (off,old,_) in SPEED{
            data[off-1]=0xe8;
            data[off..off+4].copy_from_slice(&old);
        }
        data[CD_CHECK.0..CD_CHECK.0+5].copy_from_slice(&CD_CHECK.1);
        for (off,old,_) in INTRO_GATES{data[off..off+2].copy_from_slice(&old);}
        data[WINMM_NAME_PTR..WINMM_NAME_PTR+4].copy_from_slice(&WINMM_OLD_RVA.to_le_bytes());
        data
    }
    #[test] fn french_call_displacements_target_own_gettickcount_thunk(){
        for (offset,_,disp) in SPEED{
            let call_va=0x401000i64+offset as i64-0x600-1;
            assert_eq!(call_va+5+i32::from_le_bytes(disp) as i64,0x446b68);
        }
    }
    #[test] fn french_patches_all_verified_preimages(){
        let mut data=sample();
        patch_original(&mut data).unwrap();
        for (off,_,new) in SPEED{assert_eq!(data[off..off+4],new);}
        assert_eq!(data[CD_CHECK.0..CD_CHECK.0+5],CD_CHECK.2);
        for (off,_,new) in INTRO_GATES{assert_eq!(data[off..off+2],new);}
        assert!(valid_import(&data,true));
    }
    #[test] fn french_unknown_instructions_fail_before_changes(){
        for off in [SPEED[0].0,CD_CHECK.0,INTRO_GATES[0].0,WINMM_NAME_PTR,WINMM_SLOT]{
            let mut data=sample();
            data[off]^=0x20;
            let before=data.clone();
            assert!(patch_original(&mut data).is_err(),"offset {off:#x}");
            assert_eq!(data,before);
        }
    }
    #[test] fn french_four_runtime_hashes_are_distinct(){
        let hashes=[PATCHED,PATCHED_SKIP,SYSTEM_WINMM,SYSTEM_WINMM_SKIP,ORIGINAL];
        for (i,h) in hashes.iter().enumerate(){
            assert_eq!(h.len(),64);
            assert!(hashes.iter().skip(i+1).all(|other|other!=h));
        }
        for skip in [false,true]{for bridge in [false,true]{
            let hash=runtime_hash(skip,bridge);
            assert!(valid_patched(hash));
        }}
    }
    #[test] fn french_intro_skip_targets_correct_epilogue(){
        assert_eq!(SKIP_OFFSET+2+0x2f,0x6426);
    }
}
