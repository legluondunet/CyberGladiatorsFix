// SPDX-License-Identifier: GPL-3.0-or-later
//! Default GameFrame renderer managed by the one-file launcher.
//! Preserve the user's installed Glide DLL exactly while the game runs;
//! never ship, modify, or silently overwrite proprietary nGlide.
use sha2::{Digest,Sha256};
use std::fs::{self,OpenOptions};
use std::io::Write;
use std::path::{Path,PathBuf};

fn digest(bytes:&[u8])->String{format!("{:x}",Sha256::digest(bytes))}
fn safe_file(path:&Path)->Result<Vec<u8>,String>{
    let m=fs::symlink_metadata(path).map_err(|e|format!("Cannot inspect {}: {e}",path.display()))?;
    if m.file_type().is_symlink()||!m.is_file()||!(4096..=16*1024*1024).contains(&m.len()){
        return Err(format!("Refusing invalid or symlinked Glide DLL: {}",path.display()));
    }
    let bytes=fs::read(path).map_err(|e|e.to_string())?;
    if !crate::bundle::pe32_dll_valid(&bytes){return Err("Glide DLL is not a Windows x86 PE32 DLL".into());}
    Ok(bytes)
}
fn vacant(path:&Path)->Result<(),String>{
    match fs::symlink_metadata(path){
        Ok(_)=>Err(format!("Backup/staging path {} already exists; refusing overwrite",path.display())),
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(()),
        Err(e)=>Err(format!("Cannot inspect {}: {e}",path.display())),
    }
}
fn old_backup(target:&Path,hash:&str)->PathBuf{
    target.with_file_name(format!("glide.dll.pre-gameframe-{}.bak",&hash[..16]))
}
// Hash-qualified GameFrame builds from published GitHub Actions artifacts.
// This exact allowlist is intentionally fail-closed: each new CI artifact
// must be recorded before it is recognized for interrupted-run recovery.
// Unknown/proprietary Glide DLLs remain untouched.
fn known_project_gameframe(hash:&str)->bool{
    matches!(hash,
        "7f2e99707e23954e194537d0133c9a48cdd4a8fbb41f55254dc6daa01d2747fe" | // build 69
        "5a9ed7553d8858f561c50ec42393f6dfb6fe53e6d477ea925e154594ddc1d8ce" | // build 70
        "72dfef957a6f97d8a18aec0103d8fe19e1a7397342ae48267ce3bf3252a34b1c" | // build 76
        "25f97548c42d2344794a738d22dd108dede43294215a14ba7a285afca11fab19" | // build 78-80
        "63f030a23c61a8f6e691d4fe4c654f4e2e812217d62267e1bf104a67099d9697" | // build 81
        "70a6c54624ca818143e75736828ce2c2a3241919d77d611d436aefca39858966" | // build 83
        "65ecc8df4147306a78f88a248b16d91386f379b3c32d93166b6b2dea534f5338" | // build 84
        "f846570c4e7076204c74bc45fa74c54d6dd7381fc2ace92737907d74ca98d8ea" | // build 85, Wine/Proton validated
        "320ae0bf580b39ba127fcf4deab6fefb8f87a60b0354fd7dcad107e1d35d1bc5" | // build 90, Wine/Proton validated
        "9431a729d2abfde1c590a07c5e23577b26a0bfd5702c3623fa2e7bb6abaf02fa" | // CI build 91
        "a227828483799bfe44cf5adaa8f228fe943c99b73eb62642ceddeac489baac73" | // CI build 92
        "33fe423a14533379cb88d835d38ddd7f2c17a1f030ee38b5b10621f7caca6f92" | // CI build 93
        "9a702f7c5f8e501fbc9e336d70e1fd96265e0e6e35740e2e00460cab07eeaa21" | // main CI build 94
        "8e42f1395902f2bdc4d3093c4533b34e4de8af97f622aeb60b623b6bd2b88804" | // FR CI build 117, Windows native confirmed
        "d7965b179dbbcc5261fe912f8ded7536ec32ad8950015d924e12305c2dfca327" | // FR native Alt+Tab CI build 120
        "8d03b4a23ede7719d5d8911b5df492bbfee6e2dc3cba951bcfdac3113b9d24e5" | // FR Alt+Tab user-tested CI build 121
        "c993729ed697f3ad1b03d6cbb543879f0aaa5efc93e78365ea7fe70bfe8f0d84" | // FR DirectDraw Windows CI build 123
        "2278040e6cae064cfd7e4a7cbc8236c9e253c92cf21dded19fe363b7d712e7c3" | // FR DirectDraw Windows crash CI build 124 (safe recovery only)
        "0698a6dfd07e1381b7c3b884c97cc81096ee8d4f56c50ddb851bc013bb1e5455" | // FR CI build 138, measured in native Windows user log
        "258a04192e04e4f02463fd2e671f0111253efc5925d460e93a025892535569ee" | // FR CI build 140, measured in native Windows user log
        "4bcc60e02b23b3db28e39d976cec1d6e78517dd8ae421b67b8b9d281a745836c" | // FR CI build 144, measured in native Windows user log
        "f64d7d5b92e73f778245bf2e2f3a626be306ba585a04ae64d2eca6034389859b" | // FR CI build 146, measured in native Windows user log
        "0626c6fa366c8410ff9d96f18c00ce5e5597f8a1d0d2f355981a2686d165f1db"   // failed native 32-bit experiment build 158 (recovery ONLY)
    )
}
/// A published project-built wrapper with no older user DLL to restore
/// should upgrade in place. Never infer this from a filename alone.
fn retain_new_project_build(previous_hash:&str,prior_user_backup:bool)->bool{
    !prior_user_backup && known_project_gameframe(previous_hash)
}
fn retired_experiment(target:&Path,hash:&str)->PathBuf{
    target.with_file_name(format!("glide.dll.previous-gameframe-{}.bak",&hash[..16]))
}
fn backup_filename_matches(path:&Path,hash:&str)->bool{
    path.file_name().is_some_and(|name|
        name.to_string_lossy()==format!("glide.dll.pre-gameframe-{}.bak",&hash[..16]))
}
fn pending_backup(game:&Path)->Result<Option<PathBuf>,String>{
    let mut hits=Vec::new();
    for entry in fs::read_dir(game).map_err(|e|e.to_string())?{
        let e=entry.map_err(|e|e.to_string())?;
        let n=e.file_name().to_string_lossy().to_string();
        if n.starts_with("glide.dll.pre-gameframe-")&&n.ends_with(".bak"){
            let bytes=safe_file(&e.path())?;
            let hash=digest(&bytes);
            if !backup_filename_matches(&e.path(),&hash){
                return Err(format!("GameFrame backup filename does not match its SHA-256; refusing to modify {}",e.path().display()));
            }
            hits.push(e.path());
        }
    }
    if hits.len()>1{return Err("Multiple prior GameFrame backups found; manual inspection required".into());}
    Ok(hits.pop())
}
pub struct Session{
    target:PathBuf,
    backup:Option<PathBuf>,
    expected:String,
    root:PathBuf,
    active:bool,
}
impl Session{
    pub fn begin(root:&Path,game:&Path)->Result<Self,String>{
        crate::bundle::verify()?;
        let embedded=crate::bundle::GAMEFRAME_GLIDE_DLL;
        if embedded.is_empty(){
            return Err("This build does not embed GameFrame Glide; use the dedicated GameFrame Windows x64 artifact".into());
        }
        let expected=digest(embedded);
        let target=game.join("glide.dll");
        let old=match fs::symlink_metadata(&target){
            Ok(_)=>Some(safe_file(&target)?),
            Err(e) if e.kind()==std::io::ErrorKind::NotFound=>None,
            Err(e)=>return Err(format!("Cannot inspect installed glide.dll: {e}")),
        };
        let pending=pending_backup(game)?;
        if old.as_deref().is_some_and(|b|digest(b)==expected){
            crate::show(&format!("GAMEFRAME_BUNDLE existing_matching_dll=1 recover_backup={} hash={expected}",u8::from(pending.is_some())),root);
            return Ok(Self{target,backup:pending,expected,root:root.to_path_buf(),active:true});
        }
        // A game terminated while an older experimental GameFrame build was
        // installed can leave the previous backup in place. We MAY recover
        // automatically, but only when the current DLL is byte-for-byte one
        // of our known GameFrame CI releases. Never assume an arbitrary
        // Glide DLL is a temporary experiment.
        let has_user_backup=pending.is_some();
        let (backup,retired)=if let Some(pending_backup)=pending{
            let previous=old.as_ref().ok_or(
                "A GameFrame backup exists but glide.dll is missing; refusing automated recovery"
            )?;
            let previous_hash=digest(previous);
            if !known_project_gameframe(&previous_hash){
                return Err(format!(
                    "GameFrame backup exists and active Glide SHA-256 {previous_hash} is not a known project build; preserve both for manual inspection"
                ));
            }
            let retired=retired_experiment(&target,&previous_hash);
            (Some(pending_backup),Some(retired))
        }else if let Some(previous)=old.as_ref().filter(|b|
            retain_new_project_build(&digest(b),false)){
            // Clean installations already have our prior verified GameFrame,
            // not a user-supplied Glide DLL. Archive its exact bytes and keep
            // the newer wrapper after exit, rather than restoring the old one.
            (None,Some(retired_experiment(&target,&digest(previous))))
        }else{
            // Unknown/proprietary Glide DLLs retain the existing safe path:
            // archive the ORIGINAL and restore it on normal game exit.
            (old.as_ref().map(|b|old_backup(&target,&digest(b))),None)
        };
        let stage=game.join("glide.dll.gameframe-staging.tmp");
        vacant(&stage)?;
        if let Some(ref p)=retired{vacant(p)?;}
        else if let Some(ref p)=backup{vacant(p)?;}
        let mut out=OpenOptions::new().create_new(true).write(true).open(&stage)
            .map_err(|e|format!("Cannot stage embedded GameFrame Glide: {e}"))?;
        if let Err(e)=out.write_all(embedded).and_then(|_|out.sync_all()){
            drop(out);
            let _=fs::remove_file(&stage);
            return Err(format!("Cannot write GameFrame DLL: {e}"));
        }
        drop(out);
        if !matches!(safe_file(&stage),Ok(ref bytes) if digest(bytes)==expected){
            let _=fs::remove_file(&stage);
            return Err("Embedded GameFrame DLL failed staged PE32/SHA-256 validation".into());
        }
        // In an interrupted older run, preserve BOTH versions: the saved
        // original backup stays at its verified location and the currently
        // installed old GameFrame is retired to a new hash-qualified archive.
        let displaced=retired.as_ref().or(backup.as_ref());
        if let Some(p)=displaced{
            if let Err(e)=fs::rename(&target,p){
                let _=fs::remove_file(&stage);
                return Err(format!("Cannot preserve previous Glide DLL: {e}"));
            }
        }
        if let Err(e)=fs::rename(&stage,&target){
            let restored=displaced.map(|p|fs::rename(p,&target));
            let _=fs::remove_file(&stage);
            return Err(format!("Cannot activate GameFrame DLL: {e}; rollback={restored:?}"));
        }
        if let Some(ref archived)=retired{
            if has_user_backup{
                crate::show(&format!(
                    "GAMEFRAME_INTERRUPTED_RECOVERY prior_gameframe_archived={} baseline_backup_verified={} original_bytes_preserved=1",
                    archived.display(),backup.as_ref().is_some_and(|p|p.exists()) as u8
                ),root);
            }else{
                crate::show(&format!(
                    "GAMEFRAME_PROJECT_UPGRADE prior_gameframe_archived={} retained_new_glide_on_exit=1 published_hash_verified=1",
                    archived.display()
                ),root);
            }
        }
        crate::show(&format!("GAMEFRAME_BUNDLE installed_from_exe=1 sha256={expected} original_backup={} restore_original_on_exit={}",
            backup.as_ref().map_or_else(||"none".into(),|p|p.display().to_string()),u8::from(backup.is_some())),root);
        Ok(Self{target,backup,expected,root:root.to_path_buf(),active:true})
    }
    pub fn restore(&mut self)->Result<(),String>{
        if !self.active{return Ok(());}
        let active=safe_file(&self.target)?;
        if digest(&active)!=self.expected{
            return Err("Installed Glide DLL changed while the game was running; refusing to discard it or its backup".into());
        }
        match &self.backup{
            Some(p)=>{
                let original=safe_file(p)?;
                if digest(&original)==self.expected{return Err("Backup unexpectedly contains GameFrame, refusing restoration".into());}
                let hold=self.target.with_file_name("glide.dll.gameframe-active.tmp");
                vacant(&hold)?;
                fs::rename(&self.target,&hold).map_err(|e|format!("Cannot retire temporary GameFrame DLL: {e}"))?;
                if let Err(e)=fs::rename(p,&self.target){
                    let rollback=fs::rename(&hold,&self.target);
                    return Err(format!("Cannot restore original Glide DLL: {e}; rollback={rollback:?}"));
                }
                self.active=false;
                if let Err(e)=fs::remove_file(&hold){
                    crate::show(&format!("GAMEFRAME_RESTORE_WARNING original restored but temporary {} remains: {e}",hold.display()),&self.root);
                }
                crate::show("GAMEFRAME_RESTORE original_glide_dll_restored=1 original_bytes_preserved=1",&self.root);
            },
            None=>{
                // No pre-existing third-party DLL: leave the validated
                // GameFrame wrapper installed after clean first boot.
                self.active=false;
                crate::show("GAMEFRAME_RESTORE embedded_gameframe_glide_retained=1 original_was_missing=1",&self.root);
            }
        }
        Ok(())
    }
}
impl Drop for Session{
    fn drop(&mut self){
        if self.active{
            if let Err(e)=self.restore(){
                crate::show(&format!("GAMEFRAME_RESTORE_ERROR {e}; original backup was not removed"),&self.root);
            }
        }
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn backup_path_is_deterministic_and_hash_qualified(){
        let target=Path::new(r"C:\Games\CyberGladiatorsFix\glide.dll");
        assert_eq!(old_backup(target,&"a".repeat(64)).file_name().unwrap(),
            "glide.dll.pre-gameframe-aaaaaaaaaaaaaaaa.bak");
    }
    #[test]
    fn interrupted_gameframe_requires_exact_known_hash_and_original_receipt(){
        assert!(known_project_gameframe("25f97548c42d2344794a738d22dd108dede43294215a14ba7a285afca11fab19"));
        assert!(known_project_gameframe("7f2e99707e23954e194537d0133c9a48cdd4a8fbb41f55254dc6daa01d2747fe"));
        assert!(known_project_gameframe("f846570c4e7076204c74bc45fa74c54d6dd7381fc2ace92737907d74ca98d8ea"));
        for hash in [
            "320ae0bf580b39ba127fcf4deab6fefb8f87a60b0354fd7dcad107e1d35d1bc5",
            "9431a729d2abfde1c590a07c5e23577b26a0bfd5702c3623fa2e7bb6abaf02fa",
            "a227828483799bfe44cf5adaa8f228fe943c99b73eb62642ceddeac489baac73",
            "33fe423a14533379cb88d835d38ddd7f2c17a1f030ee38b5b10621f7caca6f92",
            "9a702f7c5f8e501fbc9e336d70e1fd96265e0e6e35740e2e00460cab07eeaa21",
            "8e42f1395902f2bdc4d3093c4533b34e4de8af97f622aeb60b623b6bd2b88804",
            "d7965b179dbbcc5261fe912f8ded7536ec32ad8950015d924e12305c2dfca327",
            "8d03b4a23ede7719d5d8911b5df492bbfee6e2dc3cba951bcfdac3113b9d24e5",
            "c993729ed697f3ad1b03d6cbb543879f0aaa5efc93e78365ea7fe70bfe8f0d84",
            "2278040e6cae064cfd7e4a7cbc8236c9e253c92cf21dded19fe363b7d712e7c3",
        ]{
            assert!(known_project_gameframe(hash),"Published CI GameFrame digest missing: {hash}");
        }
        assert!(!known_project_gameframe(&"a".repeat(64)));
        let hash="7f2e99707e23954e194537d0133c9a48cdd4a8fbb41f55254dc6daa01d2747fe";
        let target=Path::new(r"C:\Games\CyberGladiatorsFix\glide.dll");
        assert!(backup_filename_matches(&old_backup(target,hash),hash));
        assert!(!backup_filename_matches(&old_backup(target,hash),&"a".repeat(64)));
        assert_ne!(retired_experiment(target,hash),old_backup(target,hash));
    }
    #[test]
    fn published_gameframe_upgrade_is_retained_but_unknown_dll_is_restored(){
        let original_fr_build="8e42f1395902f2bdc4d3093c4533b34e4de8af97f622aeb60b623b6bd2b88804";
        assert!(retain_new_project_build(original_fr_build,false));
        assert!(!retain_new_project_build(original_fr_build,true));
        assert!(!retain_new_project_build(&"a".repeat(64),false));
        // The crashing #124 DLL is still a verified *project* binary and
        // must be safely replaced, not mistaken for a user's original Glide.
        assert!(retain_new_project_build(
            "2278040e6cae064cfd7e4a7cbc8236c9e253c92cf21dded19fe363b7d712e7c3",
            false));
        // The last user-tested Alt+Tab build #121 must be upgraded without
        // treating its official GameFrame DLL as a third-party original.
        assert!(retain_new_project_build(
            "8d03b4a23ede7719d5d8911b5df492bbfee6e2dc3cba951bcfdac3113b9d24e5",
            false));
        assert!(retain_new_project_build(
            "d7965b179dbbcc5261fe912f8ded7536ec32ad8950015d924e12305c2dfca327",
            false));
    }
    #[cfg(gameframe_embedded)]
    #[test]
    fn embedded_dll_can_be_activated_and_old_bytes_restored(){
        let d=std::env::temp_dir().join(format!("CyberGameFrame-{}-{}",
            std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir(&d).unwrap();
        let before=crate::bundle::CDDA_DLL.to_vec();
        let target=d.join("glide.dll");
        fs::write(&target,&before).unwrap();
        let mut session=Session::begin(&d,&d).unwrap();
        assert_eq!(fs::read(&target).unwrap(),crate::bundle::GAMEFRAME_GLIDE_DLL);
        session.restore().unwrap();
        assert_eq!(fs::read(&target).unwrap(),before);
        fs::remove_file(&target).unwrap();
        let mut session=Session::begin(&d,&d).unwrap();
        assert!(target.exists());
        session.restore().unwrap();
        assert_eq!(fs::read(&target).unwrap(),crate::bundle::GAMEFRAME_GLIDE_DLL);
        // The one-log contract intentionally creates a session journal
        // beside the fixture. This directory was freshly generated only
        // for this test; clean it without touching external user files.
        fs::remove_dir_all(&d).unwrap();
    }
}
