// SPDX-License-Identifier: GPL-3.0-or-later
//! SHA-256-gated retail timing, no-CD and startup-intro fixes for CYBER.EXE.
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

pub const ORIGINAL: &str = "226a04a745addbfe77709aa7ec56bf5c17792c524b31882da7653e1973775624";
pub const SPEED_ONLY: &str = "520d516a572f3287458337e8cef1231b5645f59e3952b3f568a2aab8f71af832";
pub const NO_CD_ONLY: &str = "d397053de4b1a8573bd851915e7aeb3773f3b7010a08b85162c28ea94bccdaf9";
pub const INTRO_ONLY: &str = "d8329312a8f63f5efea1720b422429f530799c6887f31c18ba71f970390900b6";
pub const PATCHED: &str = "71c8cb080a8d11001a91c370d2248dd9819cfd00b4b919cd7bbcb7699a96eedb";
pub const PATCHED_SKIP_INTRO: &str = "05215ec70462c0e34de7a5d3f9bf32adebb0a6a17b2946bcb768efb1db1b328f";
/// Legacy verified variant with direct system WinMM import (upgrade support).
pub const PATCHED_NO_CDDA: &str = INTRO_ONLY;
pub const PATCHED_NO_CDDA_SKIP: &str = "339c2a0a51ab48849bfc12d07d1caf9745a6fef43b80e195c8d404aeb80e2ff9";
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

// On the USA executable, startup video is skipped when the obsolete Sierra
// installer did not write [Config] CDROM to sierra.ini. Both AVI files are
// already installed locally; bypass that obsolete INI gate, and choose the
// included 640x480 SPLASH4.AVI rather than the 320x240 fallback.
//
// Raw offsets 0x63FB = VA 0x406DFB: JE return -> NOP NOP
//             0x640F = VA 0x406E0F: JL low-resolution AVI -> NOP NOP
// These edits do NOT supply the Windows Cinepak/VfW MCI codec; if Wine
// cannot decode Cinepak, the AVI may still fail to play.
const INTRO_GATES: [(usize,[u8;2],[u8;2]);2] = [
    (0x63fb,[0x74,0x2f],[0x90,0x90]),
    (0x640f,[0x7c,0x0c],[0x90,0x90]),
];

#[derive(Copy,Clone,PartialEq,Eq,Debug)]
enum Edition { Original, SpeedOnly, NoCdOnly, IntroOnly }

// Exact USA PE: import descriptor WINMM name RVA field at raw 0x4E434.
// Its original name RVA is 0x3ED40C. Raw 0x4EF00 of .idata is
// verified zero padding and can hold our 24-byte DLL name.
const WINMM_NAME_POINTER:usize=0x4e434;
const OLD_WINMM_RVA:u32=0x3ed40c;
const CDDA_NAME_RVA:u32=0x3edb00;
const CDDA_NAME_OFFSET:usize=0x4ef00;
const CDDA_NAME:&[u8;24]=b"CyberGladiatorsFix.dll\0\0";
const LEGACY_CDDA_NAME:&[u8;24]=b"CyberGladiatorsCDDA.dll\0";
/// Verify the renamed imported WinMM module using already pinned SHA-256:
/// virtual reversal of ONLY the 24-byte name slot yields the trusted old hash.
fn canonical_digest(data:&[u8])->String{
    if data.len()!=LEN{return digest(data);}
    let rva=u32::from_le_bytes(data[WINMM_NAME_POINTER..WINMM_NAME_POINTER+4].try_into().unwrap());
    if rva==CDDA_NAME_RVA &&
       &data[CDDA_NAME_OFFSET..CDDA_NAME_OFFSET+24]==CDDA_NAME{
        let mut old=data.to_vec();
        old[CDDA_NAME_OFFSET..CDDA_NAME_OFFSET+24].copy_from_slice(LEGACY_CDDA_NAME);
        digest(&old)
    }else{digest(data)}
}
fn validate_import(data:&[u8])->Result<(),String>{
    let name=u32::from_le_bytes(data[WINMM_NAME_POINTER..WINMM_NAME_POINTER+4]
        .try_into().unwrap());
    if name!=OLD_WINMM_RVA ||
        data[CDDA_NAME_OFFSET..CDDA_NAME_OFFSET+CDDA_NAME.len()]
            .iter().any(|&b|b!=0) {
        return Err("Unexpected WinMM import descriptor or nonempty CDDA slot".into());
    }
    Ok(())
}
/// All instruction preimages are checked before mutating any bytes.
fn patch_bytes(data: &mut [u8], stage: Edition) -> Result<(),String> {
    if data.len()!=LEN {return Err(format!("Unexpected CYBER.EXE size: {}",data.len()));}
    for (offset,original,speed) in CHANGES {
        let expected=if stage==Edition::Original {original} else {speed};
        if data[offset..offset+4]!=expected || data[offset-1]!=0xe8 {
            return Err(format!("Unexpected timing instructions at 0x{offset:X}; patch refused"));
        }
    }
    let (cd_offset, cd_old, cd_new)=CD_CHECK;
    let expected=if matches!(stage,Edition::NoCdOnly|Edition::IntroOnly) {cd_new} else {cd_old};
    if data[cd_offset..cd_offset+5]!=expected {
        return Err(format!("Unexpected CD-check instructions at 0x{cd_offset:X}; patch refused"));
    }
    for (offset,old,new) in INTRO_GATES {
        let expected=if stage==Edition::IntroOnly {new} else {old};
        if data[offset..offset+2]!=expected {
            return Err(format!("Unexpected intro instructions at 0x{offset:X}; patch refused"));
        }
    }
    validate_import(data)?;
    if stage==Edition::Original {
        for (offset,_,replacement) in CHANGES {
            data[offset..offset+4].copy_from_slice(&replacement);
        }
    }
    if !matches!(stage,Edition::NoCdOnly|Edition::IntroOnly) {
        data[cd_offset..cd_offset+5].copy_from_slice(&cd_new);
    }
    if stage!=Edition::IntroOnly {
        for (offset,_,new) in INTRO_GATES {
            data[offset..offset+2].copy_from_slice(&new);
        }
    }
    data[WINMM_NAME_POINTER..WINMM_NAME_POINTER+4].copy_from_slice(&CDDA_NAME_RVA.to_le_bytes());
    data[CDDA_NAME_OFFSET..CDDA_NAME_OFFSET+CDDA_NAME.len()].copy_from_slice(CDDA_NAME);
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
    let hash=canonical_digest(&data);
    if [SPEED_ONLY,NO_CD_ONLY].contains(&hash.as_str()) {
        return Err("Existing CYBER.EXE requires CDDA upgrade. Place CyberGladiatorsCDDA.dll beside CyberGladiatorsFix.exe and rerun patcher without arguments".into());
    }
    if ![PATCHED,PATCHED_SKIP_INTRO,PATCHED_NO_CDDA,PATCHED_NO_CDDA_SKIP].contains(&hash.as_str()) {
        return Err(format!("Unsupported CYBER.EXE: SHA-256 {hash}; expected the recognized CDDA patch with intro enabled/disabled"));
    }
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

/// Atomically replace an already validated game EXE; roll back if publishing fails.
fn replace_exe(path:&Path,patched:&[u8])->Result<(),String>{
    let tmp=path.with_extension("EXE.tmp");
    if tmp.exists(){return Err(format!("Temporary patch file already exists: {}",tmp.display()));}
    {
        use std::io::Write;
        let mut f=fs::OpenOptions::new().write(true).create_new(true).open(&tmp).map_err(|e|e.to_string())?;
        if let Err(e)=f.write_all(patched).and_then(|_|f.sync_all()) {
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
        return Err(format!("Cannot install patched exe: {e}; original restore: {restore:?}; original backup: {}",backup_path(path).display()));
    }
    let _=fs::remove_file(&retired);
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
    if [PATCHED,PATCHED_SKIP_INTRO,PATCHED_NO_CDDA,PATCHED_NO_CDDA_SKIP].contains(&hash.as_str()) {
        verify_exe(path)?;
        if &data[CDDA_NAME_OFFSET..CDDA_NAME_OFFSET+24]==LEGACY_CDDA_NAME{
            let mut updated=data;
            updated[CDDA_NAME_OFFSET..CDDA_NAME_OFFSET+24].copy_from_slice(CDDA_NAME);
            if canonical_digest(&updated)!=hash{
                return Err("WinMM import migration checksum mismatch; original EXE unchanged".into());
            }
            replace_exe(path,&updated)?;
            verify_exe(path)?;
            return Ok("Verified legacy WinMM import migrated to CyberGladiatorsFix.dll");
        }
        return Ok("Already patched (verified)");
    }
    let stage=match hash.as_str() {
        ORIGINAL=>Edition::Original,
        SPEED_ONLY=>Edition::SpeedOnly,
        NO_CD_ONLY=>Edition::NoCdOnly,
        INTRO_ONLY=>Edition::IntroOnly,
        _=>return Err(format!("Unsupported CYBER.EXE: SHA-256 {hash} ({} bytes). Executable unchanged.",data.len())),
    };
    let mut patched=data.clone();
    patch_bytes(&mut patched,stage)?;
    if canonical_digest(&patched)!=PATCHED {return Err("Internal checksum mismatch: patch refused".into());}
    let backup=backup_path(path);
    if backup.exists() {
        if file_hash(&backup)? != ORIGINAL {return Err("Existing .bak has unexpected contents; refusing to overwrite".into());}
    } else {
        // An existing speed-only installation must already have the retail
        // original backup; it is unsafe to back up the speed-patched bytes
        // under a name that claims to contain the untouched original.
        if stage!=Edition::Original {
            return Err("Original CYBER.EXE.bak missing; cannot upgrade an existing installation safely".into());
        }
        // create_new never overwrites user files; sync backup before touching exe.
        use std::io::Write;
        let mut f=fs::OpenOptions::new().write(true).create_new(true).open(&backup)
            .map_err(|e|format!("Cannot create original backup: {e}"))?;
        f.write_all(&data).map_err(|e|e.to_string())?;
        f.sync_all().map_err(|e|e.to_string())?;
    }
    replace_exe(path,&patched)?;
    verify_exe(path)?;
    Ok(match stage {
        Edition::Original => "Speed, no-CD, startup video and CDDA bridge applied",
        Edition::SpeedOnly => "No-CD, startup video and CDDA bridge added to existing speed fix",
        Edition::NoCdOnly => "Startup video and CDDA bridge added to existing no-CD installation",
        Edition::IntroOnly => "CDDA bridge added to existing speed/no-CD/intro installation",
    })
}

// The original JE 0x2F at raw offset 0x63FB branches to the intro
// function epilogue at raw 0x642C. Its patched NOPs enable the intro;
// JMP 0x2F skips it unconditionally without affecting other AVI playback.
const SKIP_INTRO_OFFSET:usize=0x63fb;
const INTRO_PLAY_BYTES:[u8;2]=[0x90,0x90];
const INTRO_SKIP_BYTES:[u8;2]=[0xeb,0x2f];

/// Applies the [Video] SkipIntro setting before the game starts. Both
/// exact SHA-256 executable variants are supported and the retail backup
/// remains untouched. No game assets, IQ data or cyber.ini are modified.
/// Whether the exact verified x86 WinMM bridge import is installed, regardless
/// of the user's independent EnableCDDA music preference.
pub fn cdda_active(path:&Path)->Result<bool,String>{
    verify_exe(path)?;
    let hash=canonical_digest(&fs::read(path).map_err(|e|e.to_string())?);
    Ok(hash==PATCHED || hash==PATCHED_SKIP_INTRO)
}

fn expected_runtime_hash(intro_skip:bool,cdda_enabled:bool)->&'static str{
    match (intro_skip,cdda_enabled){
        (false,true)=>PATCHED,
        (true,true)=>PATCHED_SKIP_INTRO,
        (false,false)=>PATCHED_NO_CDDA,
        (true,false)=>PATCHED_NO_CDDA_SKIP,
    }
}
fn is_intro_skipped(hash:&str)->bool{
    hash==PATCHED_SKIP_INTRO || hash==PATCHED_NO_CDDA_SKIP
}

/// Switch between the recognized WinMM bridge import and the legacy system
/// WinMM import. Four SHA-256 variants are supported for safe upgrades.
/// The launcher always enables the verified bridge for native Cinepak video;
/// FLAC music selection is a separate per-process option, not an EXE patch.
/// The retail backup and original CD resources are never changed.
pub fn set_cdda(path:&Path,enabled:bool)->Result<&'static str,String>{
    verify_exe(path)?;
    let original=fs::read(path).map_err(|e|e.to_string())?;
    let hash=canonical_digest(&original);
    let current=hash==PATCHED||hash==PATCHED_SKIP_INTRO;
    let legacy=current&&&original[CDDA_NAME_OFFSET..CDDA_NAME_OFFSET+24]==LEGACY_CDDA_NAME;
    if current==enabled && !legacy{
        return Ok(if enabled{"CyberGladiatorsFix.dll already imported"}else{"Legacy system WinMM import already selected"});
    }
    let mut updated=original;
    let name=u32::from_le_bytes(updated[WINMM_NAME_POINTER..WINMM_NAME_POINTER+4]
        .try_into().unwrap());
    let expected_name=if current{CDDA_NAME_RVA}else{OLD_WINMM_RVA};
    let expected_bytes:&[u8;24]=if legacy{LEGACY_CDDA_NAME}
        else if current{CDDA_NAME}else{&[0u8;24]};
    if name!=expected_name ||
       updated[CDDA_NAME_OFFSET..CDDA_NAME_OFFSET+CDDA_NAME.len()]!=*expected_bytes{
        return Err("Unexpected WinMM import name; refusing CDDA toggle".into());
    }
    let new_rva=if enabled{CDDA_NAME_RVA}else{OLD_WINMM_RVA};
    updated[WINMM_NAME_POINTER..WINMM_NAME_POINTER+4].copy_from_slice(&new_rva.to_le_bytes());
    if enabled{
        updated[CDDA_NAME_OFFSET..CDDA_NAME_OFFSET+CDDA_NAME.len()].copy_from_slice(CDDA_NAME);
    }else{
        updated[CDDA_NAME_OFFSET..CDDA_NAME_OFFSET+CDDA_NAME.len()].fill(0);
    }
    if canonical_digest(&updated)!=expected_runtime_hash(is_intro_skipped(&hash),enabled){
        return Err("CDDA toggle checksum mismatch; CYBER.EXE unchanged".into());
    }
    replace_exe(path,&updated)?;
    verify_exe(path)?;
    Ok(if enabled{"WinMM video/CDDA bridge import enabled"}else{"Legacy system WinMM import restored"})
}

/// Applies the [Video] SkipIntro setting before game start, independently of
/// CDDA mode, without modifying cyber.ini, other videos or any IQ files.
pub fn set_skip_intro(path:&Path,skip:bool)->Result<&'static str,String>{
    verify_exe(path)?;
    let original=fs::read(path).map_err(|e|e.to_string())?;
    let hash=canonical_digest(&original);
    let cdda=hash==PATCHED||hash==PATCHED_SKIP_INTRO;
    if skip==is_intro_skipped(&hash){
        return Ok(if skip{"Startup intro already disabled"}else{"Startup intro already enabled"});
    }
    let (from,to)=if skip {
        (INTRO_PLAY_BYTES,INTRO_SKIP_BYTES)
    }else{
        (INTRO_SKIP_BYTES,INTRO_PLAY_BYTES)
    };
    if original[SKIP_INTRO_OFFSET..SKIP_INTRO_OFFSET+2]!=from{
        return Err("Unexpected intro bytes; refusing to modify CYBER.EXE".into());
    }
    let mut updated=original;
    updated[SKIP_INTRO_OFFSET..SKIP_INTRO_OFFSET+2].copy_from_slice(&to);
    if canonical_digest(&updated)!=expected_runtime_hash(skip,cdda){
        return Err("Startup intro checksum mismatch; CYBER.EXE unchanged".into());
    }
    replace_exe(path,&updated)?;
    verify_exe(path)?;
    Ok(if skip{"Startup intro disabled by CyberGladiatorsFix.ini"}else{"Startup intro enabled by CyberGladiatorsFix.ini"})
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
    #[test] fn known_hashes_are_all_distinct(){
        let values=[ORIGINAL,SPEED_ONLY,NO_CD_ONLY,PATCHED,PATCHED_SKIP_INTRO,PATCHED_NO_CDDA_SKIP];
        for (i,a) in values.iter().enumerate(){
            for b in values.iter().skip(i+1){assert_ne!(a,b);}
        }
    }
    #[test] fn cd_check_patch_skips_volume_label_probe(){
        assert_eq!(0x401000 + CD_CHECK.0 - 0x600,0x405416);
        assert_eq!(CD_CHECK.1,[0xe8,0x4f,0xff,0xff,0xff]);
        assert_eq!(CD_CHECK.2,[0xb8,0x01,0x00,0x00,0x00]);
    }
    #[test] fn intro_gate_offsets_match_disassembled_branches(){
        assert_eq!(0x401000 + INTRO_GATES[0].0 - 0x600,0x406dfb);
        assert_eq!(0x401000 + INTRO_GATES[1].0 - 0x600,0x406e0f);
        assert_eq!(INTRO_GATES[0].1,[0x74,0x2f]);
        assert_eq!(INTRO_GATES[1].1,[0x7c,0x0c]);
    }
    fn synthetic_original()->Vec<u8>{
        let mut data=vec![0u8;LEN];
        for (offset,old,_) in CHANGES {
            data[offset-1]=0xe8;
            data[offset..offset+4].copy_from_slice(&old);
        }
        data[CD_CHECK.0..CD_CHECK.0+5].copy_from_slice(&CD_CHECK.1);
        for (offset,old,_) in INTRO_GATES {
            data[offset..offset+2].copy_from_slice(&old);
        }
        data[WINMM_NAME_POINTER..WINMM_NAME_POINTER+4].copy_from_slice(&OLD_WINMM_RVA.to_le_bytes());
        data
    }
    fn prepare_stage(stage:Edition)->Vec<u8>{
        let mut data=synthetic_original();
        if stage!=Edition::Original {
            for (offset,_,new) in CHANGES {data[offset..offset+4].copy_from_slice(&new);}
        }
        if matches!(stage,Edition::NoCdOnly|Edition::IntroOnly) {
            data[CD_CHECK.0..CD_CHECK.0+5].copy_from_slice(&CD_CHECK.2);
        }
        if stage==Edition::IntroOnly {
            for (offset,_,new) in INTRO_GATES {data[offset..offset+2].copy_from_slice(&new);}
        }
        data
    }
    #[test] fn patches_original_in_one_pass(){
        let mut data=prepare_stage(Edition::Original);
        patch_bytes(&mut data,Edition::Original).unwrap();
        for (offset,_,new) in CHANGES {assert_eq!(&data[offset..offset+4],&new);}
        assert_eq!(&data[CD_CHECK.0..CD_CHECK.0+5],&CD_CHECK.2);
        for (offset,_,new) in INTRO_GATES {assert_eq!(&data[offset..offset+2],&new);}
    }
    #[test] fn upgrades_speed_only_without_touching_timing_patches(){
        let mut data=prepare_stage(Edition::SpeedOnly);
        let speed_before=data[0x3110..0x3130].to_vec();
        patch_bytes(&mut data,Edition::SpeedOnly).unwrap();
        assert_eq!(&data[0x3110..0x3130],speed_before.as_slice());
        assert_eq!(&data[CD_CHECK.0..CD_CHECK.0+5],&CD_CHECK.2);
    }
    #[test] fn upgrades_no_cd_only_without_touching_existing_patches(){
        let mut data=prepare_stage(Edition::NoCdOnly);
        let old_timing=data[0x3110..0x3130].to_vec();
        let old_cd=data[CD_CHECK.0..CD_CHECK.0+5].to_vec();
        patch_bytes(&mut data,Edition::NoCdOnly).unwrap();
        assert_eq!(&data[0x3110..0x3130],old_timing.as_slice());
        assert_eq!(&data[CD_CHECK.0..CD_CHECK.0+5],old_cd.as_slice());
        for (offset,_,new) in INTRO_GATES {assert_eq!(&data[offset..offset+2],&new);}
    }
    #[test] fn refuses_unexpected_cd_check_without_partial_mutation(){
        let mut data=prepare_stage(Edition::Original);
        data[CD_CHECK.0]=0x90;
        let unchanged=data.clone();
        let err=patch_bytes(&mut data,Edition::Original).unwrap_err();
        assert!(err.contains("CD-check"));
        assert_eq!(data,unchanged);
    }
    #[test] fn upgrades_intro_without_modifying_existing_speed_or_video_fixes(){
        let mut data=prepare_stage(Edition::IntroOnly);
        let before=data.clone();
        patch_bytes(&mut data,Edition::IntroOnly).unwrap();
        assert_eq!(&data[0x3110..0x3130],&before[0x3110..0x3130]);
        assert_eq!(&data[CD_CHECK.0..CD_CHECK.0+5],&before[CD_CHECK.0..CD_CHECK.0+5]);
        assert_eq!(&data[INTRO_GATES[0].0..INTRO_GATES[0].0+2],&before[INTRO_GATES[0].0..INTRO_GATES[0].0+2]);
        assert_eq!(&data[CDDA_NAME_OFFSET..CDDA_NAME_OFFSET+CDDA_NAME.len()],CDDA_NAME);
    }
    #[test] fn new_import_is_canonicalized_to_known_hash(){
        assert_eq!(CDDA_NAME.len(),24);
        assert_eq!(CDDA_NAME[22],0);
        assert_eq!(CDDA_NAME[23],0);
        let mut fresh=prepare_stage(Edition::IntroOnly);
        patch_bytes(&mut fresh,Edition::IntroOnly).unwrap();
        let mut old=fresh.clone();
        old[CDDA_NAME_OFFSET..CDDA_NAME_OFFSET+24].copy_from_slice(LEGACY_CDDA_NAME);
        assert_eq!(canonical_digest(&fresh),digest(&old));
        fresh[0x200]=1;
        assert_ne!(canonical_digest(&fresh),digest(&old));
    }
    #[test] fn refuses_nonempty_import_name_padding(){
        let mut data=prepare_stage(Edition::IntroOnly);
        data[CDDA_NAME_OFFSET]=0x42;
        let before=data.clone();
        assert!(patch_bytes(&mut data,Edition::IntroOnly).is_err());
        assert_eq!(data,before);
    }
    #[test] fn refuses_unexpected_intro_branch_without_partial_mutation(){
        let mut data=prepare_stage(Edition::NoCdOnly);
        data[INTRO_GATES[0].0]=0x90;
        let unchanged=data.clone();
        let err=patch_bytes(&mut data,Edition::NoCdOnly).unwrap_err();
        assert!(err.contains("intro"));
        assert_eq!(data,unchanged);
    }    #[test] fn all_four_runtime_states_are_distinct(){
        let values=[PATCHED,PATCHED_SKIP_INTRO,PATCHED_NO_CDDA,PATCHED_NO_CDDA_SKIP];
        for (i,v) in values.iter().enumerate(){
            for other in values.iter().skip(i+1){assert_ne!(v,other);}
        }
        assert_eq!(expected_runtime_hash(false,false),INTRO_ONLY);
        assert_eq!(expected_runtime_hash(true,false),PATCHED_NO_CDDA_SKIP);
        assert_eq!(expected_runtime_hash(false,true),PATCHED);
        assert_eq!(expected_runtime_hash(true,true),PATCHED_SKIP_INTRO);
        assert!(!is_intro_skipped(PATCHED_NO_CDDA));
        assert!(is_intro_skipped(PATCHED_NO_CDDA_SKIP));
    }
    #[test] fn intro_skip_branch_targets_existing_epilogue(){
        assert_eq!(SKIP_INTRO_OFFSET+2+0x2f,0x642c);
        assert_eq!(INTRO_SKIP_BYTES,[0xeb,0x2f]);
        assert_eq!(INTRO_PLAY_BYTES,[0x90,0x90]);
        assert_ne!(PATCHED,PATCHED_SKIP_INTRO);
    }

}
