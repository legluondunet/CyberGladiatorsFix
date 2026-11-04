// SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only checks. Static asset hashes are recorded during extraction and
//! independently recomputed before launch, including the original backup.
//! The 100 retail IQ/iqNN.dat files are game-writable: enforce their original
//! size and safe regular-file status without demanding unchanged bytes.
use crate::{install::{DIR,MANIFEST},nglide,patch};
use sha2::{Digest,Sha256};
use std::fs::{self,File};
use std::io::Read;
use std::path::{Path,Component};
use std::process::{Child,Command};
fn file_hash(path:&Path)->Result<(u64,String),String>{
    let m=fs::symlink_metadata(path).map_err(|e|format!("Missing {}: {e}",path.display()))?;
    if !m.is_file()||m.file_type().is_symlink(){return Err(format!("Not a regular file: {}",path.display()));}
    let mut f=File::open(path).map_err(|e|e.to_string())?;
    let mut digest=Sha256::new();
    let mut n=0u64;
    let mut b=[0u8;64*1024];
    loop {let got=f.read(&mut b).map_err(|e|e.to_string())?;
        if got==0 {break;}digest.update(&b[..got]);n+=got as u64;
    }
    Ok((n,format!("{:x}",digest.finalize())))
}
fn safe_manifest_path(s:&str)->bool{
    let p=Path::new(s);
    if s.is_empty()||s.contains(['\t','\n','\r',':','\\'])||p.is_absolute(){return false;}
    p.components().all(|c|matches!(c,Component::Normal(_)))
}
const RETAIL_IQ_SIZE:u64=3_696;

/// The game can persist IQ state into these 100 retail files. Only paths
/// matching the precise USA image's iq/iq00.dat..iq/iq99.dat names, with
/// their original size, are eligible for mutable handling.
fn mutable_iq_data(name:&str,expected_size:u64)->bool{
    let b=name.as_bytes();
    b.len()==11 && b.starts_with(b"iq/iq") && b[5].is_ascii_digit() &&
        b[6].is_ascii_digit() && b[7..]==*b".dat" &&
        expected_size==RETAIL_IQ_SIZE
}

/// Returns true when a game-writable IQ file was verified by size instead
/// of its original CD digest. All other resources remain SHA-256 exact.
fn check_asset(name:&str,expected_size:u64,expected_hash:&str,
               actual_size:u64,actual_hash:&str)->Result<bool,String>{
    if actual_size!=expected_size {
        return Err(format!("Game asset size has changed: {name} (expected {expected_size}, got {actual_size})"));
    }
    if mutable_iq_data(name,expected_size) {return Ok(true);}
    if actual_hash!=expected_hash {
        return Err(format!("Game asset has changed: {name}"));
    }
    Ok(false)
}

pub fn verify(root:&Path)->Result<String,String>{
    let game=root.join(DIR);
    let meta=fs::symlink_metadata(&game).map_err(|e|format!("Missing game directory: {e}"))?;
    if !meta.is_dir()||meta.file_type().is_symlink(){return Err("Game folder is not a regular directory".into());}
    patch::verify_exe(&game.join("cyber.exe"))?;
    let manifest=fs::read_to_string(game.join(MANIFEST)).map_err(|e|format!("Manifest missing: {e}"))?;
    if manifest.len()>4*1024*1024||!manifest.starts_with("# CyberGladiatorsFix manifest v1\n"){
        return Err("Missing or invalid extraction manifest".into());
    }
    let mut count=0usize;let mut bytes=0u64;
    let mut mutable_count=0usize;
    let mut seen=std::collections::BTreeSet::<String>::new();
    for line in manifest.lines().skip(1){
        let mut parts=line.split('\t');
        let name=parts.next().ok_or("Empty manifest row")?;
        let size=parts.next().ok_or("Missing manifest size")?.parse::<u64>().map_err(|e|e.to_string())?;
        let hash=parts.next().ok_or("Missing manifest SHA-256")?;
        if parts.next().is_some()||hash.len()!=64||!hash.bytes().all(|c|c.is_ascii_hexdigit())||
            !safe_manifest_path(name)||!seen.insert(name.to_ascii_lowercase()) {
            return Err(format!("Invalid manifest entry: {name}"));
        }
        let mut path=game.clone();
        // A local manifest must never cause verification to traverse a
        // symlinked directory outside the installation.
        for component in Path::new(name).components(){
            let Component::Normal(part)=component else{return Err(format!("Unsafe manifest path: {name}"))};
            path.push(part);
            let meta=fs::symlink_metadata(&path).map_err(|e|format!("Missing manifest path {}: {e}",path.display()))?;
            if meta.file_type().is_symlink(){return Err(format!("Symlink in manifest path: {}",path.display()));}
        }
        let (actual_size,actual_hash)=file_hash(&path)?;
        if check_asset(name,size,hash,actual_size,&actual_hash)? {
            mutable_count+=1;
        }
        count+=1;bytes+=size;
    }
    nglide::verify(&game)?;
    Ok(format!("Verified CYBER.EXE patch + backup, {count} CD resources ({bytes} bytes), {mutable_count} game-writable IQ files (size/regular-file check), and nGlide"))
}
pub fn launch(root:&Path,logo:bool)->Result<Child,String>{
    verify(root)?;
    let game=root.join(DIR);
    if logo{nglide::check_logo(&game)?;}
    let mut cmd=Command::new(game.join("cyber.exe"));
    cmd.current_dir(&game);
    nglide::configure(&mut cmd,logo);
    cmd.spawn().map_err(|e|format!("Unable to start CYBER.EXE: {e}"))
}

#[cfg(test)]
mod mutable_iq_tests {
    use super::*;

    #[test]
    fn accepts_only_known_writable_iq_file_names(){
        for index in 0..100 {
            let name=format!("iq/iq{index:02}.dat");
            assert!(mutable_iq_data(&name,RETAIL_IQ_SIZE),"{name}");
        }
        for name in [
            "iq/iq41.dat.bak","iq/iq100.dat","iq/iq4.dat",
            "iq/iqab.dat","iq/../iq41.dat","graphics/iq41.dat",
            "iq/iq41.bin","IQ/iq41.dat","iq/iq41.dat/other",
            "fighters/iq41.dat",
        ] {
            assert!(!mutable_iq_data(name,RETAIL_IQ_SIZE),"{name}");
        }
        assert!(!mutable_iq_data("iq/iq41.dat",0));
        assert!(!mutable_iq_data("iq/iq41.dat",RETAIL_IQ_SIZE+1));
    }

    #[test]
    fn mutable_iq_accepts_content_change_but_not_truncation(){
        let name="iq/iq41.dat";
        assert_eq!(check_asset(name,RETAIL_IQ_SIZE,"old",RETAIL_IQ_SIZE,"game changed").unwrap(),true);
        assert!(check_asset(name,RETAIL_IQ_SIZE,"old",100,"game changed").unwrap_err().contains("size"));
        assert!(check_asset(name,RETAIL_IQ_SIZE,"old",RETAIL_IQ_SIZE+1,"game changed").is_err());
    }

    #[test]
    fn static_assets_and_flac_still_require_exact_sha256(){
        for name in ["graphics/shell.fnt","splash4.avi","fighters/plasma/model.dat",
                     "Track02.flac","iq/iq41.dat.bak","iq/iq100.dat"] {
            assert_eq!(check_asset(name,3696,"original",3696,"original").unwrap(),false);
            assert!(check_asset(name,3696,"original",3696,"modified").is_err(),"{name}");
        }
    }

    #[test]
    fn any_asset_must_keep_expected_size(){
        assert!(check_asset("graphics/shell.fnt",120,"hash",119,"hash").is_err());
        assert!(check_asset("Track02.flac",350,"hash",351,"hash").is_err());
    }
}
