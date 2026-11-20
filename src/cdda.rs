// SPDX-License-Identifier: GPL-3.0-or-later
//! Embedded, owner-managed x86 WinMM bridge beside game/cyber.exe.
//! Replacements do not leave .bak files; an unknown DLL is never overwritten.
use std::borrow::Cow;
use std::fs::{self,OpenOptions};
use std::io::Write;
use std::path::Path;
use sha2::{Digest,Sha256};

pub const DLL_NAME:&str="fix.dll";
fn digest(bytes:&[u8])->String{format!("{:x}",Sha256::digest(bytes))}
fn check_bytes(bytes:&[u8])->Result<(),String>{
    if bytes.len()<4096 || bytes.len()>16*1024*1024 || &bytes[..2]!=b"MZ"{
        return Err(format!("{DLL_NAME} is not a valid PE32 module"));
    }
    let off=u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as usize;
    if off.checked_add(26).is_none_or(|n|n>bytes.len())
        ||&bytes[off..off+4]!=b"PE\0\0"
        ||u16::from_le_bytes(bytes[off+4..off+6].try_into().unwrap())!=0x14c
        ||u16::from_le_bytes(bytes[off+22..off+24].try_into().unwrap())&0x2000==0
        ||u16::from_le_bytes(bytes[off+24..off+26].try_into().unwrap())!=0x10b{
        return Err(format!("{DLL_NAME} must be a Windows x86 PE32 DLL"));
    }
    Ok(())
}
fn read_regular(path:&Path)->Result<Option<Vec<u8>>,String>{
    match fs::symlink_metadata(path){
        Ok(meta)=>{
            if !meta.is_file()||meta.file_type().is_symlink(){
                return Err(format!("Unsafe DLL or receipt path: {}",path.display()));
            }
            fs::read(path).map(Some).map_err(|e|e.to_string())
        },
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(None),
        Err(e)=>Err(format!("Cannot inspect {}: {e}",path.display()))
    }
}
fn read_receipt(root:&Path)->Result<Option<String>,String>{
    let path=crate::layout::dll_receipt(root);
    let Some(bytes)=read_regular(&path)? else{return Ok(None)};
    let text=std::str::from_utf8(&bytes).map_err(|e|e.to_string())?;
    let Some(hash)=text.strip_suffix('\n') else{return Err("Invalid fix.dll ownership receipt".into())};
    if hash.len()!=64||!hash.bytes().all(|c|c.is_ascii_hexdigit()&&!c.is_ascii_uppercase()){
        return Err("Malformed fix.dll ownership hash".into());
    }
    Ok(Some(hash.to_owned()))
}
fn vacant(path:&Path)->Result<(),String>{
    if fs::symlink_metadata(path).is_ok()||path.is_symlink(){
        return Err(format!("Refusing occupied temporary path: {}",path.display()));
    }
    Ok(())
}
fn install_bytes(path:&Path,data:&[u8])->Result<(),String>{
    check_bytes(data)?;
    let mut file=OpenOptions::new().write(true).create_new(true).open(path)
        .map_err(|e|format!("Cannot stage fix.dll {}: {e}",path.display()))?;
    if let Err(e)=file.write_all(data).and_then(|_|file.sync_all()){
        drop(file);
        let _=fs::remove_file(path);
        return Err(e.to_string());
    }
    Ok(())
}
fn record_receipt(root:&Path,hash:&str)->Result<(),String>{
    let file=crate::layout::dll_receipt(root);
    if read_receipt(root)?.as_deref()==Some(hash){return Ok(());}
    let temp=file.with_extension("sha256.tmp");
    let retired=file.with_extension("sha256.retired.tmp");
    vacant(&temp)?;vacant(&retired)?;
    let mut output=OpenOptions::new().write(true).create_new(true).open(&temp).map_err(|e|e.to_string())?;
    if let Err(e)=writeln!(output,"{hash}").and_then(|_|output.sync_all()){
        drop(output);let _=fs::remove_file(&temp);return Err(e.to_string());
    }
    drop(output);
    let had_previous=read_regular(&file)?.is_some();
    if had_previous {
        if let Err(e)=fs::rename(&file,&retired){let _=fs::remove_file(&temp);return Err(e.to_string());}
    }
    if let Err(e)=fs::rename(&temp,&file){
        if had_previous{let _=fs::rename(&retired,&file);}
        let _=fs::remove_file(&temp);
        return Err(e.to_string());
    }
    if had_previous{fs::remove_file(&retired).map_err(|e|format!("Temporary receipt removal: {e}"))?;}
    Ok(())
}
fn source_bytes(root:&Path)->Result<Cow<'static,[u8]>,String>{
    #[cfg(test)]{fs::read(root.join(DLL_NAME)).map(Cow::Owned).map_err(|e|e.to_string())}
    #[cfg(not(test))]{
        let _=root;
        crate::bundle::verify()?;
        Ok(Cow::Borrowed(crate::bundle::CDDA_DLL))
    }
}
pub fn verify(game:&Path)->Result<(),String>{
    let root=game.parent().ok_or("Invalid game directory")?;
    let file=game.join(DLL_NAME);
    let bytes=read_regular(&file)?.ok_or_else(||format!("Missing {}",file.display()))?;
    check_bytes(&bytes)?;
    let actual=digest(&bytes);
    let receipt=read_receipt(root)?;
    if receipt.as_deref()!=Some(&actual){
        return Err(format!("Unrecognized fix.dll contents or missing ownership receipt: {}",file.display()));
    }
    #[cfg(not(test))]
    if actual!=digest(crate::bundle::CDDA_DLL){
        return Err("Installed fix.dll differs from this build's verified bundled bridge".into());
    }
    Ok(())
}
/// Replace only a DLL whose installed hash agrees with our own state receipt.
/// Temporary retired files exist only during the transaction and are removed.
pub fn try_install(root:&Path,game:&Path)->Result<(),String>{
    let source=source_bytes(root)?;
    check_bytes(&source)?;
    let wanted=digest(&source);
    let target=game.join(DLL_NAME);
    let existing=read_regular(&target)?;
    let receipt=read_receipt(root)?;
    if let Some(ref data)=existing {
        check_bytes(data)?;
        let prior=digest(data);
        if receipt.as_deref()!=Some(prior.as_str()){
            return Err("Refusing to replace unknown fix.dll (ownership receipt mismatch)".into());
        }
        if prior==wanted{return Ok(());}
    }else if receipt.is_some(){
        return Err("fix.dll is missing but an ownership receipt exists; refusing silent repair".into());
    }
    let staged=game.join("fix.dll.staged.tmp");
    let retired=game.join("fix.dll.retired.tmp");
    vacant(&staged)?;vacant(&retired)?;
    install_bytes(&staged,&source)?;
    if digest(&fs::read(&staged).map_err(|e|e.to_string())?)!=wanted{
        let _=fs::remove_file(&staged);return Err("Staged fix.dll hash mismatch".into());
    }
    if existing.is_some() {
        if let Err(e)=fs::rename(&target,&retired){let _=fs::remove_file(&staged);return Err(e.to_string());}
    }
    if let Err(e)=fs::rename(&staged,&target){
        if existing.is_some(){let _=fs::rename(&retired,&target);}
        let _=fs::remove_file(&staged);
        return Err(format!("Cannot activate fix.dll: {e}"));
    }
    if let Err(e)=record_receipt(root,&wanted){
        let _=fs::remove_file(&target);
        if existing.is_some(){let _=fs::rename(&retired,&target);}
        return Err(format!("Cannot record fix.dll update: {e}"));
    }
    if existing.is_some(){
        fs::remove_file(&retired).map_err(|e|format!("Cannot remove temporary fix.dll copy: {e}"))?;
    }
    Ok(())
}
#[cfg(test)]
mod tests{
    use super::*;
    fn pe()->Vec<u8>{
        let mut bytes=vec![0;4096];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        bytes[0x84..0x86].copy_from_slice(&0x14cu16.to_le_bytes());
        bytes[0x96..0x98].copy_from_slice(&0x2000u16.to_le_bytes());
        bytes[0x98..0x9a].copy_from_slice(&0x10bu16.to_le_bytes());
        bytes
    }
    #[test] fn bridge_is_named_fix(){
        assert_eq!(DLL_NAME,"fix.dll");
        assert!(check_bytes(&pe()).is_ok());
    }
    #[test] fn owned_dll_updates_without_backup(){
        let root=std::env::temp_dir().join(format!("cyber-owned-dll-{}",std::process::id()));
        let _=fs::remove_dir_all(&root);
        fs::create_dir_all(crate::layout::state(&root)).unwrap();
        let game=crate::layout::game(&root);
        fs::create_dir(&game).unwrap();
        let mut old=pe();
        fs::write(root.join(DLL_NAME),&old).unwrap();
        try_install(&root,&game).unwrap();
        assert!(game.join(DLL_NAME).exists());
        old[2000]=42;
        fs::write(root.join(DLL_NAME),&old).unwrap();
        try_install(&root,&game).unwrap();
        assert_eq!(fs::read(game.join(DLL_NAME)).unwrap(),old);
        assert!(!game.join("fix.dll.bak").exists());
        assert!(!game.join("fix.dll.retired.tmp").exists());
        assert_eq!(read_receipt(&root).unwrap(),Some(digest(&old)));
        fs::remove_dir_all(root).unwrap();
    }
    #[test] fn unknown_dll_is_preserved(){
        let root=std::env::temp_dir().join(format!("cyber-foreign-dll-{}",std::process::id()));
        let _=fs::remove_dir_all(&root);
        fs::create_dir_all(crate::layout::state(&root)).unwrap();
        let game=crate::layout::game(&root);fs::create_dir(&game).unwrap();
        let x=pe();
        fs::write(&game.join(DLL_NAME),&x).unwrap();
        fs::write(root.join(DLL_NAME),&x).unwrap();
        assert!(try_install(&root,&game).is_err());
        assert_eq!(fs::read(game.join(DLL_NAME)).unwrap(),x);
        fs::remove_dir_all(root).unwrap();
    }
}
