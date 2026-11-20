// SPDX-License-Identifier: GPL-3.0-or-later
//! GameFrame Glide x86 in game/, managed without persistent DLL backups.
//! Only an installed hash matching the state ownership receipt may be replaced.
use std::fs::{self,OpenOptions};
use std::io::Write;
use std::path::{Path,PathBuf};
use sha2::{Digest,Sha256};

fn digest(bytes:&[u8])->String{format!("{:x}",Sha256::digest(bytes))}
fn read_optional(path:&Path)->Result<Option<Vec<u8>>,String>{
    match fs::symlink_metadata(path){
        Ok(meta)=>{
            if !meta.is_file()||meta.file_type().is_symlink(){
                return Err(format!("Unsafe GameFrame path {}",path.display()));
            }
            fs::read(path).map(Some).map_err(|e|e.to_string())
        },
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(None),
        Err(e)=>Err(format!("Cannot inspect {}: {e}",path.display()))
    }
}
fn receipt_path(root:&Path)->PathBuf{crate::layout::glide_receipt(root)}
fn read_receipt(root:&Path)->Result<Option<String>,String>{
    let Some(data)=read_optional(&receipt_path(root))? else{return Ok(None)};
    let s=std::str::from_utf8(&data).map_err(|e|e.to_string())?;
    let h=s.strip_suffix('\n').ok_or("Malformed GameFrame ownership receipt")?;
    if h.len()!=64||!h.bytes().all(|b|b.is_ascii_hexdigit()&&!b.is_ascii_uppercase()){
        return Err("Invalid GameFrame receipt SHA-256".into());
    }
    Ok(Some(h.into()))
}
fn vacant(path:&Path)->Result<(),String>{
    match fs::symlink_metadata(path){
        Ok(_)=>Err(format!("Existing GameFrame temporary file: {}",path.display())),
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(()),
        Err(e)=>Err(e.to_string())
    }
}
fn record_receipt(root:&Path,hash:&str)->Result<(),String>{
    if read_receipt(root)?.as_deref()==Some(hash){return Ok(());}
    let path=receipt_path(root);
    let staged=path.with_extension("sha256.tmp");
    let retired=path.with_extension("sha256.retired.tmp");
    vacant(&staged)?;vacant(&retired)?;
    let mut f=OpenOptions::new().write(true).create_new(true).open(&staged)
        .map_err(|e|e.to_string())?;
    if let Err(e)=writeln!(f,"{hash}").and_then(|_|f.sync_all()){
        drop(f);let _=fs::remove_file(&staged);return Err(e.to_string());
    }
    drop(f);
    let previous=read_optional(&path)?.is_some();
    if previous {fs::rename(&path,&retired).map_err(|e|e.to_string())?;}
    if let Err(e)=fs::rename(&staged,&path){
        if previous{let _=fs::rename(&retired,&path);}
        let _=fs::remove_file(&staged);return Err(e.to_string());
    }
    if previous{fs::remove_file(&retired).map_err(|e|e.to_string())?;}
    Ok(())
}
pub fn verify(root:&Path,game:&Path)->Result<(),String>{
    crate::bundle::verify()?;
    let expected=digest(crate::bundle::GAMEFRAME_GLIDE_DLL);
    let path=game.join("glide.dll");
    let bytes=read_optional(&path)?.ok_or("Missing GameFrame glide.dll")?;
    if !crate::bundle::pe32_dll_valid(&bytes)||digest(&bytes)!=expected{
        return Err(format!("Unexpected GameFrame glide.dll at {}",path.display()));
    }
    if read_receipt(root)?.as_deref()!=Some(expected.as_str()){
        return Err("GameFrame ownership receipt missing or mismatched".into());
    }
    Ok(())
}
pub struct Session{root:PathBuf,target:PathBuf,expected:String,active:bool}
impl Session{
    pub fn begin(root:&Path,game:&Path)->Result<Self,String>{
        crate::bundle::verify()?;
        let embedded=crate::bundle::GAMEFRAME_GLIDE_DLL;
        if embedded.is_empty()||!crate::bundle::pe32_dll_valid(embedded){
            return Err("Missing verified embedded x86 GameFrame DLL".into());
        }
        let expected=digest(embedded);
        let target=game.join("glide.dll");
        let existing=read_optional(&target)?;
        let receipt=read_receipt(root)?;
        if let Some(ref bytes)=existing{
            let actual=digest(bytes);
            if !crate::bundle::pe32_dll_valid(bytes)||
                receipt.as_deref()!=Some(actual.as_str()){
                return Err(format!("Unknown glide.dll at {}: refusing replacement without proven ownership",target.display()));
            }
            if actual==expected {
                return Ok(Self{root:root.to_owned(),target,expected,active:true});
            }
        }else if receipt.is_some(){
            return Err("GameFrame DLL missing but owned receipt exists; refusing silent repair".into());
        }
        let staged=game.join("glide.dll.staged.tmp");
        let retired=game.join("glide.dll.retired.tmp");
        vacant(&staged)?;vacant(&retired)?;
        let mut f=OpenOptions::new().write(true).create_new(true).open(&staged)
            .map_err(|e|e.to_string())?;
        if let Err(e)=f.write_all(embedded).and_then(|_|f.sync_all()){
            drop(f);let _=fs::remove_file(&staged);return Err(e.to_string());
        }
        drop(f);
        if read_optional(&staged)?.as_deref()!=Some(embedded){
            let _=fs::remove_file(&staged);
            return Err("GameFrame staged bytes differ from embedded source".into());
        }
        if existing.is_some(){
            if let Err(e)=fs::rename(&target,&retired){
                let _=fs::remove_file(&staged);return Err(e.to_string());
            }
        }
        if let Err(e)=fs::rename(&staged,&target){
            if existing.is_some(){let _=fs::rename(&retired,&target);}
            let _=fs::remove_file(&staged);return Err(e.to_string());
        }
        if let Err(e)=record_receipt(root,&expected){
            let _=fs::remove_file(&target);
            if existing.is_some(){let _=fs::rename(&retired,&target);}
            return Err(format!("Cannot record GameFrame ownership: {e}"));
        }
        if existing.is_some(){
            fs::remove_file(&retired).map_err(|e|format!("Cannot remove GameFrame temporary copy: {e}"))?;
        }
        crate::show(&format!("GAMEFRAME_GLIDE_POLICY installed_bundled=1 no_persistent_backup=1 sha256={expected}"),root);
        Ok(Self{root:root.to_owned(),target,expected,active:true})
    }
    pub fn finish(&mut self)->Result<(),String>{
        if !self.active{return Ok(());}
        verify(&self.root,self.target.parent().ok_or("Missing GameFrame parent directory")?)?;
        self.active=false;
        Ok(())
    }
}
impl Drop for Session{
    fn drop(&mut self){
        if self.active{
            if let Err(e)=self.finish(){
                crate::show(&format!("GAMEFRAME_GLIDE_FINISH_ERROR {e}"),&self.root);
            }
        }
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn receipt_refuses_foreign_glide(){
        let root=std::env::temp_dir().join(format!("cyber-glide-policy-{}",std::process::id()));
        let _=fs::remove_dir_all(&root);
        fs::create_dir_all(crate::layout::state(&root)).unwrap();
        let game=crate::layout::game(&root);fs::create_dir(&game).unwrap();
        fs::write(game.join("glide.dll"),b"foreign").unwrap();
        assert!(Session::begin(&root,&game).is_err());
        assert_eq!(fs::read(game.join("glide.dll")).unwrap(),b"foreign");
        assert!(!game.join("glide.dll.bak").exists());
        fs::remove_dir_all(root).unwrap();
    }
    #[cfg(gameframe_embedded)]
    #[test] fn installs_glide_without_backup(){
        let root=std::env::temp_dir().join(format!("cyber-glide-install-{}",std::process::id()));
        let _=fs::remove_dir_all(&root);
        fs::create_dir_all(crate::layout::state(&root)).unwrap();
        let game=crate::layout::game(&root);fs::create_dir(&game).unwrap();
        let mut session=Session::begin(&root,&game).unwrap();
        session.finish().unwrap();
        assert!(verify(&root,&game).is_ok());
        assert!(!game.join("glide.dll.bak").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
