// SPDX-License-Identifier: GPL-3.0-or-later
//! One-click, fail-closed installer/launcher for a verified retail CD image
//! or an existing installation. Never rewrite an existing game installation
//! after an integrity error. A named per-folder mutex prevents two sessions
//! from racing DLL staging, FLAC migration or game process startup.
use crate::{audio,cdda,install,migration,patch,settings,verify};
use sha2::{Digest,Sha256};
use std::ffi::c_void;
use std::fs;
use std::path::{Path,PathBuf};

#[link(name="kernel32")]
extern "system" {
    fn CreateMutexW(attributes:*mut c_void,owner:i32,name:*const u16)->*mut c_void;
    fn GetLastError()->u32;
    fn CloseHandle(handle:*mut c_void)->i32;
}
const ERROR_ALREADY_EXISTS:u32=183;
pub struct SessionLock(*mut c_void);
impl Drop for SessionLock{
    fn drop(&mut self){if !self.0.is_null(){unsafe{CloseHandle(self.0);}}}
}
pub fn lock(root:&Path)->Result<SessionLock,String>{
    let source=root.to_string_lossy().to_lowercase();
    let hash=format!("{:x}",Sha256::digest(source.as_bytes()));
    let name=format!("Local\\CyberGladiatorsFix-{}",&hash[..24]);
    let wide:Vec<u16>=name.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe{
        let handle=CreateMutexW(std::ptr::null_mut(),0,wide.as_ptr());
        if handle.is_null(){return Err(format!("Cannot create exclusive game-session mutex: {}",GetLastError()));}
        if GetLastError()==ERROR_ALREADY_EXISTS {
            CloseHandle(handle);
            return Err("CyberGladiatorsFix is already installing or running this game in another instance".into());
        }
        Ok(SessionLock(handle))
    }
}
fn installed(root:&Path)->Result<bool,String>{
    let p=root.join(install::DIR);
    match fs::symlink_metadata(&p){
        Ok(m)=>{
            if !m.is_dir()||m.file_type().is_symlink(){
                return Err(format!("Game destination is not a safe directory: {}",p.display()));
            }
            Ok(true)
        },
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(false),
        Err(e)=>Err(format!("Cannot inspect game directory: {e}"))
    }
}
pub fn prepare(root:&Path,selected:Option<&Path>)->Result<(),String>{
    let game=root.join(install::DIR);
    if installed(root)?{
        if selected.is_some(){
            return Err("Game already installed; an explicit CD path cannot silently replace it. Use --verify to check existing files".into());
        }
        crate::show("AUTO_DETECT existing_game=1 disc_search=skipped",root);
        settings::ensure(&game)?;
        // Prove every original game resource and exact SHA-256-gated EXE
        // against the previous manifest before moving any old FLAC files.
        crate::show(&format!("AUTO_INTEGRITY_PRECHECK {}",verify::verify_install_content(root)?),root);
        let moved=migration::migrate(&game)?;
        if moved>0{crate::show(&format!("AUTO_MUSIC_MIGRATION moved_tracks={moved} destination=music/ legacy_manifest_backup=preserved"),root);}
        cdda::try_install(root,&game)?;
        let exe=game.join("cyber.exe");
        let result=patch::apply(&exe)?;
        patch::set_cdda(&exe,true)?;
        patch::set_skip_intro(&exe,settings::read(&game)?)?;
        crate::show(&format!("AUTO_PATCH {result} imported_bridge=CyberGladiatorsFix.dll"),root);
    }else{
        let cue=match selected{
            Some(path)=>path.to_path_buf(),
            None=>install::find_cue(root).map_err(|e|format!(
                "No installed game and no unambiguous CUE/BIN disc image beside CyberGladiatorsFix.exe: {e}"
            ))?
        };
        if !cue.extension().is_some_and(|s|s.eq_ignore_ascii_case("cue")){
            return Err("A retail CD image must currently be a .cue referring to its .bin tracks; standalone .iso is not supported".into());
        }
        crate::show(&format!("AUTO_DISC_FOUND path={} type=CUE_BIN",cue.display()),root);
        let report=install::install(&cue,&game,root)?;
        crate::show(&format!("AUTO_INSTALL {report}"),root);
    }
    // Check again independently, including the newly migrated music/
    // entries and immutable original CYBER.EXE.bak.
    crate::show(&format!("AUTO_INTEGRITY_POSTCHECK {}",verify::verify_install_content(root)?),root);
    Ok(())
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn absent_game_is_distinguished_from_existing_regular_dir(){
        let root=std::env::temp_dir().join(format!("CyberAuto-{}-{}",
            std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir(&root).unwrap();
        assert!(!installed(&root).unwrap());
        fs::create_dir(root.join(install::DIR)).unwrap();
        assert!(installed(&root).unwrap());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn only_one_named_mutex_per_installation_can_be_held(){
        let root=std::env::temp_dir().join(format!("CyberGladiatorsMutex-{}",
            std::process::id()));
        let guard=lock(&root).unwrap();
        assert!(lock(&root).is_err());
        drop(guard);
        assert!(lock(&root).is_ok());
    }
}
