// SPDX-License-Identifier: GPL-3.0-or-later
//! Self-contained x64 distribution of two privately built 32-bit components.
//! A single downloaded CyberGladiatorsFix.exe contains the verified x86 CDDA
//! DLL and the isolated MCI player. Runtime extraction is deliberately
//! separate from the user's original game files and cannot overwrite them.
//!
//! The Windows workflow MUST build cdda/ x86 before the root x64 crate.
use std::fs::{self,OpenOptions};
use std::io::Write;
use std::path::{Path,PathBuf};
use sha2::{Digest,Sha256};

pub const CDDA_DLL:&[u8]=include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/cdda/target/i686-pc-windows-msvc/release/CyberGladiatorsCDDA.dll"
));
pub const INTRO_EXE:&[u8]=include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/cdda/target/i686-pc-windows-msvc/release/CyberGladiatorsIntro.exe"
));

const DLL_NAME:&str="CyberGladiatorsCDDA.dll";
const INTRO_NAME:&str="CyberGladiatorsIntro.exe";

fn pe32_valid(bytes:&[u8],dll:bool)->bool{
    if !(4096..=16*1024*1024).contains(&bytes.len()) || &bytes[..2]!=b"MZ" {return false;}
    let offset=u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as usize;
    let Some(end)=offset.checked_add(26) else{return false;};
    if end>bytes.len() || &bytes[offset..offset+4]!=b"PE\0\0" {return false;}
    let machine=u16::from_le_bytes(bytes[offset+4..offset+6].try_into().unwrap());
    let flags=u16::from_le_bytes(bytes[offset+22..offset+24].try_into().unwrap());
    let optional=u16::from_le_bytes(bytes[offset+24..offset+26].try_into().unwrap());
    machine==0x14c && optional==0x10b && ((flags&0x2000)!=0)==dll
}
pub fn verify()->Result<(),String>{
    if !pe32_valid(CDDA_DLL,true){return Err("Bundled CDDA DLL is not a valid Windows x86 PE32 DLL".into());}
    if !pe32_valid(INTRO_EXE,false){return Err("Bundled intro helper is not a valid Windows x86 PE32 EXE".into());}
    Ok(())
}
pub fn hash(bytes:&[u8])->String{format!("{:x}",Sha256::digest(bytes))}
pub fn describe()->Result<String,String>{
    verify()?;
    Ok(format!(
        "EMBEDDED_BUNDLE x86_CDDADLL_bytes={} sha256={} x86_IntroEXE_bytes={} sha256={}",
        CDDA_DLL.len(),hash(CDDA_DLL),INTRO_EXE.len(),hash(INTRO_EXE)
    ))
}

/// A unique, privately created temp directory for the short-lived MCI player.
/// Files are removed on Drop, after the child has completed or was killed.
/// No preexisting path is deleted or overwritten.
pub struct TemporaryIntro {dir:PathBuf,path:PathBuf}
impl TemporaryIntro {
    pub fn create()->Result<Self,String>{
        verify()?;
        let base=std::env::temp_dir();
        // Cross-platform Windows process id + fine-grained timestamp gives a
        // private create_new directory. Failure is reported, never reused.
        let nanos=std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e|e.to_string())?.as_nanos();
        let dir=base.join(format!("CyberGladiatorsFix-Intro-{}-{nanos}",std::process::id()));
        fs::create_dir(&dir).map_err(|e|format!("Cannot create private intro runtime at {}: {e}",dir.display()))?;
        let path=dir.join(INTRO_NAME);
        let result=(||->Result<(),String>{
            let mut out=OpenOptions::new().create_new(true).write(true).open(&path)
                .map_err(|e|format!("Cannot stage bundled video player: {e}"))?;
            out.write_all(INTRO_EXE).and_then(|_|out.sync_all())
                .map_err(|e|format!("Cannot stage bundled video player bytes: {e}"))?;
            drop(out);
            let bytes=fs::read(&path).map_err(|e|format!("Cannot verify staged video player: {e}"))?;
            if hash(&bytes)!=hash(INTRO_EXE){return Err("Bundled video player's staged SHA-256 mismatch".into());}
            Ok(())
        })();
        if let Err(e)=result {
            let _=fs::remove_file(&path);
            let _=fs::remove_dir(&dir);
            return Err(e);
        }
        Ok(Self{dir,path})
    }
    pub fn path(&self)->&Path{&self.path}
}
impl Drop for TemporaryIntro{
    fn drop(&mut self){
        let _=fs::remove_file(&self.path);
        let _=fs::remove_dir(&self.dir);
    }
}
pub fn dll_name()->&'static str{DLL_NAME}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn embedded_components_are_correct_x86_pe_images(){
        verify().unwrap();
        assert_ne!(hash(CDDA_DLL),hash(INTRO_EXE));
    }
    #[test]
    fn temporary_intro_is_private_and_removed_on_drop(){
        let dir;
        {
            let temp=TemporaryIntro::create().unwrap();
            assert!(temp.path().exists());
            assert_eq!(fs::read(temp.path()).unwrap(),INTRO_EXE);
            dir=temp.dir.clone();
        }
        assert!(!dir.exists());
    }
}
