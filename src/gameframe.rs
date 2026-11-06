// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit GameFrame selection from a one-file launcher.
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
// Hash-qualified GameFrame builds that were actually shipped in our Windows
// CI artifacts. Only an exact SHA-256 match authorizes reclaiming an
// interrupted experiment. Unknown/proprietary Glide DLLs remain untouched.
fn known_project_gameframe(hash:&str)->bool{
    matches!(hash,
        "7f2e99707e23954e194537d0133c9a48cdd4a8fbb41f55254dc6daa01d2747fe" | // build 69
        "5a9ed7553d8858f561c50ec42393f6dfb6fe53e6d477ea925e154594ddc1d8ce" | // build 70
        "72dfef957a6f97d8a18aec0103d8fe19e1a7397342ae48267ce3bf3252a34b1c" | // build 76
        "25f97548c42d2344794a738d22dd108dede43294215a14ba7a285afca11fab19" | // build 78-80
        "63f030a23c61a8f6e691d4fe4c654f4e2e812217d62267e1bf104a67099d9697" | // build 81
        "70a6c54624ca818143e75736828ce2c2a3241919d77d611d436aefca39858966" | // build 83
        "65ecc8df4147306a78f88a248b16d91386f379b3c32d93166b6b2dea534f5338" | // build 84
        "f846570c4e7076204c74bc45fa74c54d6dd7381fc2ace92737907d74ca98d8ea"   // build 85, validated under Wine/Proton
    )
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
        }else{
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
            crate::show(&format!(
                "GAMEFRAME_INTERRUPTED_RECOVERY prior_gameframe_archived={} baseline_backup_verified={} original_bytes_preserved=1",
                archived.display(),backup.as_ref().is_some_and(|p|p.exists()) as u8
            ),root);
        }
        crate::show(&format!("GAMEFRAME_BUNDLE installed_from_exe=1 sha256={expected} original_backup={} temporary_until_game_exit=1",
            backup.as_ref().map_or_else(||"none".into(),|p|p.display().to_string())),root);
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
        assert!(!known_project_gameframe(&"a".repeat(64)));
        let hash="7f2e99707e23954e194537d0133c9a48cdd4a8fbb41f55254dc6daa01d2747fe";
        let target=Path::new(r"C:\Games\CyberGladiatorsFix\glide.dll");
        assert!(backup_filename_matches(&old_backup(target,hash),hash));
        assert!(!backup_filename_matches(&old_backup(target,hash),&"a".repeat(64)));
        assert_ne!(retired_experiment(target,hash),old_backup(target,hash));
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
