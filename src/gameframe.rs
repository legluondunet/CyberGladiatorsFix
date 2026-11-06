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
fn pending_backup(game:&Path)->Result<Option<PathBuf>,String>{
    let mut hits=Vec::new();
    for entry in fs::read_dir(game).map_err(|e|e.to_string())?{
        let e=entry.map_err(|e|e.to_string())?;
        let n=e.file_name().to_string_lossy().to_string();
        if n.starts_with("glide.dll.pre-gameframe-")&&n.ends_with(".bak"){
            safe_file(&e.path())?;
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
        if old.as_deref().is_some_and(|b|digest(b)==expected){
            let backup=pending_backup(game)?;
            crate::show(&format!("GAMEFRAME_BUNDLE existing_matching_dll=1 recover_backup={} hash={expected}",u8::from(backup.is_some())),root);
            return Ok(Self{target,backup,expected,root:root.to_path_buf(),active:true});
        }
        if pending_backup(game)?.is_some(){
            return Err("A prior GameFrame backup exists and installed Glide differs from this build; preserve both and inspect manually".into());
        }
        let backup=old.as_ref().map(|b|old_backup(&target,&digest(b)));
        let stage=game.join("glide.dll.gameframe-staging.tmp");
        vacant(&stage)?;
        if let Some(ref p)=backup{vacant(p)?;}
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
        if let Some(ref p)=backup{
            if let Err(e)=fs::rename(&target,p){
                let _=fs::remove_file(&stage);
                return Err(format!("Cannot save original Glide DLL: {e}"));
            }
        }
        if let Err(e)=fs::rename(&stage,&target){
            let restored=backup.as_ref().map(|p|fs::rename(p,&target));
            let _=fs::remove_file(&stage);
            return Err(format!("Cannot activate GameFrame DLL: {e}; rollback={restored:?}"));
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
                fs::remove_file(&self.target).map_err(|e|format!("Cannot remove temporary Glide DLL: {e}"))?;
                self.active=false;
                crate::show("GAMEFRAME_RESTORE temporary_glide_dll_removed=1 original_was_missing=1",&self.root);
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
        assert!(!target.exists());
        fs::remove_dir(&d).unwrap();
    }
}
