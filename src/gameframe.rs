// SPDX-License-Identifier: GPL-3.0-or-later
//! Default GameFrame renderer managed by the one-file launcher.
//! Launch policy: preserve any pre-existing glide.dll in a unique .bak,
//! install the verified embedded GameFrame, and keep it installed after exit.
//! Never overwrite a user's existing backup or follow a Glide DLL symlink.
use sha2::{Digest,Sha256};
use std::fs::{self,OpenOptions};
use std::io::{Read,Write};
use std::path::{Path,PathBuf};

const MAX_BACKUP_SLOTS:u32=256;

fn digest(bytes:&[u8])->String{format!("{:x}",Sha256::digest(bytes))}
/// Stream the digest; an unknown old DLL can have *any* size, and may
/// be unreadable. Neither situation is a reason to refuse its .bak move.
fn file_digest(path:&Path)->std::io::Result<String>{
    let mut input=fs::File::open(path)?;
    let mut hash=Sha256::new();
    let mut buffer=[0u8;65536];
    loop{
        let count=input.read(&mut buffer)?;
        if count==0{break;}
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}",hash.finalize()))
}

/// Compare only the actual bytes of an existing Glide against the embedded
/// SHA-256. A different or unreadable file is preserved and replaced.
fn read_regular(path:&Path)->Result<Vec<u8>,String>{
    fs::read(path).map_err(|e|format!("Cannot read {}: {e}",path.display()))
}
#[derive(Clone)]
struct Observed{
    sha256:Option<String>,
    len:u64,
    link:bool,
}
fn read_optional(path:&Path)->Result<Option<Observed>,String>{
    match fs::symlink_metadata(path){
        Ok(meta)=>{
            if meta.is_dir(){return Err(format!(
                "Cannot use a directory as glide.dll: {}",path.display()));}
            let link=meta.file_type().is_symlink();
            // Never follow an external symlink target. Rename the link
            // itself to a backup, then install the real embedded GameFrame.
            // An unreadable or unusual file is simply an unknown version.
            let sha256=if link{None}else{file_digest(path).ok()};
            Ok(Some(Observed{sha256,len:meta.len(),link}))
        },
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(None),
        Err(e)=>Err(format!("Cannot inspect {}: {e}",path.display())),
    }
}
fn still_same(previous:Option<&Observed>,current:Option<&Observed>)->bool{
    match (previous,current){
        (None,None)=>true,
        (Some(a),Some(b))=>{
            if a.len!=b.len||a.link!=b.link{return false;}
            match (&a.sha256,&b.sha256){
                (Some(x),Some(y))=>x==y,
                (None,None)=>true, // Preserve opaque source via rename
                _=>false,
            }
        },
        _=>false,
    }
}
fn vacant(path:&Path)->Result<(),String>{
    match fs::symlink_metadata(path){
        Ok(_)=>Err(format!("Backup/staging destination {} already exists; refusing overwrite",path.display())),
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(()),
        Err(e)=>Err(format!("Cannot inspect {}: {e}",path.display())),
    }
}

/// Windows MoveFileW (without MOVEFILE_REPLACE_EXISTING) rejects an
/// occupied destination atomically. This also works under Wine. Calling
/// std::fs::rename alone could silently replace an existing backup on
/// some platforms if another process creates the destination meanwhile.
#[cfg(windows)]
#[link(name="kernel32")]
unsafe extern "system" {
    fn MoveFileW(source:*const u16,destination:*const u16)->i32;
}
fn move_no_replace(source:&Path,destination:&Path)->Result<(),String>{
    vacant(destination)?;
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let from=source.as_os_str().encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();
        let to=destination.as_os_str().encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>();
        let ok=unsafe{MoveFileW(from.as_ptr(),to.as_ptr())};
        if ok==0{
            return Err(format!("Cannot move {} to {} without replacement: {}",
                source.display(),destination.display(),std::io::Error::last_os_error()));
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        // The production launcher is Windows-only; retained for unit
        // testing of the path and data-safety policy on developer hosts.
        fs::rename(source,destination).map_err(|e|format!(
            "Cannot move {} to {}: {e}",source.display(),destination.display()))
    }
}
fn backup_path(target:&Path,hash:Option<&str>)->Result<(PathBuf,bool),String>{
    if let Some(h)=hash{
        if h.len()!=64||!h.bytes().all(|b|b.is_ascii_hexdigit()){
            return Err("Invalid Glide SHA-256 backup suffix".into());
        }
    }
    let primary=target.with_file_name("glide.dll.bak");
    if vacant(&primary).is_ok(){return Ok((primary,false));}
    let prefix=hash.map(|h|&h[..16]).unwrap_or("unknown");
    for n in 0..MAX_BACKUP_SLOTS{
        let name=if n==0{
            format!("glide.dll.{prefix}.bak")
        }else{
            format!("glide.dll.{prefix}.{n}.bak")
        };
        let candidate=target.with_file_name(name);
        if vacant(&candidate).is_ok(){return Ok((candidate,true));}
    }
    Err(format!("No free glide.dll .bak slot ({MAX_BACKUP_SLOTS} attempted); no file overwritten"))
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
            return Err("This build does not embed GameFrame Glide; use the one-EXE GameFrame Windows x64 package".into());
        }
        if !crate::bundle::pe32_dll_valid(embedded){
            return Err("Embedded GameFrame is not a valid Windows x86 PE32 DLL".into());
        }
        let expected=digest(embedded);
        let target=game.join("glide.dll");
        let old=read_optional(&target)?;
        let old_hash=old.as_ref().and_then(|f|f.sha256.as_deref());
        if old_hash==Some(expected.as_str()){
            crate::show(&format!(
                "GAMEFRAME_GLIDE_POLICY action=keep_existing existing_embedded=1 sha256={expected} previous_backups_untouched=1 retained_after_exit=1"
            ),root);
            return Ok(Self{target,backup:None,expected,root:root.to_path_buf(),active:true});
        }
        let (backup,collision)=if old.is_some(){
            let (path,collision)=backup_path(&target,old_hash)?;
            (Some(path),collision)
        }else{
            (None,false)
        };
        let stage=game.join("glide.dll.gameframe-staging.tmp");
        vacant(&stage)?;
        let mut out=OpenOptions::new().create_new(true).write(true).open(&stage)
            .map_err(|e|format!("Cannot stage embedded GameFrame Glide: {e}"))?;
        if let Err(e)=out.write_all(embedded).and_then(|_|out.sync_all()){
            drop(out);
            let _=fs::remove_file(&stage);
            return Err(format!("Cannot write GameFrame DLL: {e}"));
        }
        drop(out);
        let staged=read_regular(&stage);
        if !matches!(staged,Ok(ref bytes) if
            crate::bundle::pe32_dll_valid(bytes)&&digest(bytes)==expected){
            let _=fs::remove_file(&stage);
            return Err("Embedded GameFrame DLL failed staged PE32/SHA-256 validation".into());
        }
        // Reject a last-moment file change. In particular, a third-party
        // DLL appearing after a clean-install check must never be discarded.
        let now=read_optional(&target);
        let still_unchanged=match now{
            Ok(ref current)=>still_same(old.as_ref(),current.as_ref()),
            Err(_)=>false,
        };
        if !still_unchanged{
            let _=fs::remove_file(&stage);
            return Err("Installed Glide DLL changed while staging GameFrame; no backup was overwritten".into());
        }
        if let Some(ref saved)=backup{
            if let Err(e)=move_no_replace(&target,saved){
                let _=fs::remove_file(&stage);
                return Err(format!("Cannot back up existing Glide DLL: {e}"));
            }
            if !matches!(read_optional(saved),Ok(Some(ref archived)) if
                still_same(old.as_ref(),Some(archived))){
                let rollback=move_no_replace(saved,&target);
                let _=fs::remove_file(&stage);
                return Err(format!("Old Glide backup changed unexpectedly; rollback={rollback:?}"));
            }
        }
        if let Err(e)=move_no_replace(&stage,&target){
            let rollback=backup.as_ref().map(|path|move_no_replace(path,&target));
            let _=fs::remove_file(&stage);
            return Err(format!("Cannot activate GameFrame Glide; {e}; restore_original_rollback={rollback:?}"));
        }
        if let Some(ref saved)=backup{
            crate::show(&format!(
                "GAMEFRAME_GLIDE_BACKUP action=rename original_sha256={} path={} collision_slot={} bytes_preserved=1 existing_backups_untouched=1",
                old_hash.unwrap_or("unreadable_or_link"),saved.display(),u8::from(collision)
            ),root);
        }
        crate::show(&format!(
            "GAMEFRAME_GLIDE_POLICY action={} previous_sha256={} installed_embedded_sha256={expected} backup={} keep_embedded_after_exit=1 backup_retained_after_exit=1 game_launch_prepared=1",
            if old.is_some(){"replace_unknown_or_old"}else{"install_missing"},
            old_hash.unwrap_or("unreadable_or_link"),
            backup.as_ref().map_or_else(||"none".into(),|p|p.display().to_string())
        ),root);
        Ok(Self{target,backup,expected,root:root.to_path_buf(),active:true})
    }
    /// Finalize an application session without restoring the previous
    /// renderer. The old DLL stays at its .bak path for manual recovery.
    /// Never replace a DLL that changed while the retail game was running.
    pub fn finish(&mut self)->Result<(),String>{
        if !self.active{return Ok(());}
        let active=read_regular(&self.target)?;
        if digest(&active)!=self.expected{
            return Err("Active glide.dll changed while the game was running; preserving it and all .bak archives".into());
        }
        self.active=false;
        crate::show(&format!(
            "GAMEFRAME_GLIDE_FINISH embedded_retained=1 original_backup_retained={} backup={} original_restored=0 sha256={}",
            u8::from(self.backup.is_some()),
            self.backup.as_ref().map_or_else(||"none".into(),|p|p.display().to_string()),
            self.expected
        ),&self.root);
        Ok(())
    }
}
impl Drop for Session{
    fn drop(&mut self){
        if self.active{
            if let Err(e)=self.finish(){
                crate::show(&format!("GAMEFRAME_GLIDE_FINISH_ERROR {e}; existing files left untouched"),&self.root);
            }
        }
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    fn temp_fixture(label:&str)->PathBuf{
        let dir=std::env::temp_dir().join(format!(
            "CyberGameFrameBackup-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .unwrap().as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        dir
    }
    #[test]
    fn matching_regular_bytes_are_accepted_even_if_metadata_hints_vary(){
        let dir=temp_fixture("exact");
        let path=dir.join("glide.dll");
        fs::write(&path,b"exact payload").unwrap();
        let seen=read_optional(&path).unwrap().unwrap();
        assert_eq!(seen.sha256.as_deref(),Some(digest(b"exact payload").as_str()));
        assert!(still_same(Some(&seen),Some(&seen)));
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn collision_names_are_hash_qualified_and_never_overwritten(){
        let dir=temp_fixture("slots");
        let target=dir.join("glide.dll");
        let hash=digest(b"example older glide");
        assert_eq!(backup_path(&target,Some(&hash)).unwrap(),(dir.join("glide.dll.bak"),false));
        fs::write(dir.join("glide.dll.bak"),b"first archive").unwrap();
        let next=dir.join(format!("glide.dll.{}.bak",&hash[..16]));
        assert_eq!(backup_path(&target,Some(&hash)).unwrap(),(next.clone(),true));
        fs::write(&next,b"second archive").unwrap();
        assert_eq!(backup_path(&target,Some(&hash)).unwrap(),
            (dir.join(format!("glide.dll.{}.1.bak",&hash[..16])),true));
        // A pre-existing destination cannot be overwritten by the mover.
        let active=dir.join("glide.dll");
        fs::write(&active,b"original").unwrap();
        assert!(move_no_replace(&active,&dir.join("glide.dll.bak")).is_err());
        assert_eq!(fs::read(&active).unwrap(),b"original");
        assert_eq!(fs::read(dir.join("glide.dll.bak")).unwrap(),b"first archive");
        fs::remove_dir_all(dir).unwrap();
    }
    #[cfg(gameframe_embedded)]
    #[test]
    fn unknown_glide_is_backed_up_and_gameframe_stays_after_exit(){
        let dir=temp_fixture("unknown");
        let target=dir.join("glide.dll");
        let original=crate::bundle::CDDA_DLL.to_vec();
        fs::write(&target,&original).unwrap();
        let mut session=Session::begin(&dir,&dir).unwrap();
        assert_eq!(fs::read(&target).unwrap(),crate::bundle::GAMEFRAME_GLIDE_DLL);
        assert_eq!(fs::read(dir.join("glide.dll.bak")).unwrap(),original);
        session.finish().unwrap();
        assert_eq!(fs::read(&target).unwrap(),crate::bundle::GAMEFRAME_GLIDE_DLL);
        assert_eq!(fs::read(dir.join("glide.dll.bak")).unwrap(),original);
        let mut again=Session::begin(&dir,&dir).unwrap();
        again.finish().unwrap();
        assert_eq!(fs::read(dir.join("glide.dll.bak")).unwrap(),original);
        fs::remove_dir_all(dir).unwrap();
    }
    #[cfg(gameframe_embedded)]
    #[test]
    fn existing_backup_is_not_overwritten_and_new_backup_is_unique(){
        let dir=temp_fixture("occupied");
        let target=dir.join("glide.dll");
        let existing=b"historic backup or unknown file";
        fs::write(dir.join("glide.dll.bak"),existing).unwrap();
        let previous=b"unknown and malformed older glide".to_vec();
        fs::write(&target,&previous).unwrap();
        let hash=digest(&previous);
        let new_backup=dir.join(format!("glide.dll.{}.bak",&hash[..16]));
        let mut session=Session::begin(&dir,&dir).unwrap();
        assert_eq!(fs::read(&new_backup).unwrap(),previous);
        assert_eq!(fs::read(dir.join("glide.dll.bak")).unwrap(),existing);
        session.finish().unwrap();
        assert_eq!(fs::read(&target).unwrap(),crate::bundle::GAMEFRAME_GLIDE_DLL);
        assert_eq!(fs::read(&new_backup).unwrap(),previous);
        fs::remove_dir_all(dir).unwrap();
    }
    #[cfg(gameframe_embedded)]
    #[test]
    fn existing_staging_path_blocks_change_to_original(){
        let dir=temp_fixture("stage");
        let target=dir.join("glide.dll");
        fs::write(&target,b"other renderer").unwrap();
        let stage=dir.join("glide.dll.gameframe-staging.tmp");
        fs::write(&stage,b"occupied").unwrap();
        assert!(Session::begin(&dir,&dir).is_err());
        assert_eq!(fs::read(&target).unwrap(),b"other renderer");
        assert_eq!(fs::read(&stage).unwrap(),b"occupied");
        fs::remove_dir_all(dir).unwrap();
    }
    #[cfg(gameframe_embedded)]
    #[test]
    fn missing_glide_installs_and_keeps_embedded_even_if_old_bak_exists(){
        let dir=temp_fixture("missing");
        let backup=dir.join("glide.dll.bak");
        fs::write(&backup,b"untouched prior archive").unwrap();
        let mut session=Session::begin(&dir,&dir).unwrap();
        assert_eq!(fs::read(dir.join("glide.dll")).unwrap(),crate::bundle::GAMEFRAME_GLIDE_DLL);
        session.finish().unwrap();
        assert_eq!(fs::read(&backup).unwrap(),b"untouched prior archive");
        assert_eq!(fs::read(dir.join("glide.dll")).unwrap(),crate::bundle::GAMEFRAME_GLIDE_DLL);
        fs::remove_dir_all(dir).unwrap();
    }
    #[cfg(gameframe_embedded)]
    #[test]
    fn active_dll_changed_during_game_is_not_overwritten_or_restored(){
        let dir=temp_fixture("changed");
        let target=dir.join("glide.dll");
        fs::write(&target,b"older source").unwrap();
        let mut session=Session::begin(&dir,&dir).unwrap();
        fs::write(&target,b"third party replaced during run").unwrap();
        assert!(session.finish().is_err());
        assert_eq!(fs::read(&target).unwrap(),b"third party replaced during run");
        assert_eq!(fs::read(dir.join("glide.dll.bak")).unwrap(),b"older source");
        drop(session);
        fs::remove_dir_all(dir).unwrap();
    }
}
