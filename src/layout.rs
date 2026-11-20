// SPDX-License-Identifier: GPL-3.0-or-later
//! Installer-rooted unified layout. No legacy layout migration.
use std::path::{Path,PathBuf};

pub const GAME:&str="game";
pub const MUSIC:&str="music";
pub const STATE:&str="state";
pub const DOCS:&str="docs";
pub const CONFIG:&str="CyberGladiatorsFix.ini";
pub const MANIFEST:&str="CyberGladiatorsFix.manifest";
pub const DLL_RECEIPT:&str="CyberGladiatorsFix.sha256";
pub const GLIDE_RECEIPT:&str="CyberGladiatorsFix.gameframe.sha256";

pub fn game(root:&Path)->PathBuf{root.join(GAME)}
pub fn music(root:&Path)->PathBuf{root.join(MUSIC)}
pub fn state(root:&Path)->PathBuf{root.join(STATE)}
pub fn docs(root:&Path)->PathBuf{root.join(DOCS)}
pub fn config(root:&Path)->PathBuf{root.join(CONFIG)}
pub fn manifest(root:&Path)->PathBuf{state(root).join(MANIFEST)}
pub fn dll_receipt(root:&Path)->PathBuf{state(root).join(DLL_RECEIPT)}
pub fn glide_receipt(root:&Path)->PathBuf{state(root).join(GLIDE_RECEIPT)}

#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn paths_share_one_root(){
        let root=Path::new(r"C:\Cyber");
        assert_eq!(game(root),root.join("game"));
        assert_eq!(music(root),root.join("music"));
        assert_eq!(manifest(root),root.join("state").join(MANIFEST));
        assert_eq!(config(root),root.join(CONFIG));
    }
}

/// Generate fix-owned reference documents and an informational installation receipt.
/// Called on staging only for new installations, never by read-only commands.
pub fn ensure_support_files(root:&Path,edition:&str)->Result<(),String>{
    use std::fs::{self,OpenOptions};
    use std::io::Write;
    fn write_if_missing(path:&Path,data:&str)->Result<(),String>{
        match fs::symlink_metadata(path){
            Ok(m) if m.is_file()&&!m.file_type().is_symlink()=>return Ok(()),
            Ok(_)=>return Err(format!("Unsafe project file {}",path.display())),
            Err(e) if e.kind()==std::io::ErrorKind::NotFound=>{},
            Err(e)=>return Err(e.to_string())
        }
        let mut file=OpenOptions::new().write(true).create_new(true).open(path)
            .map_err(|e|e.to_string())?;
        file.write_all(data.as_bytes()).and_then(|_|file.sync_all())
            .map_err(|e|e.to_string())
    }
    for dir in [state(root),docs(root)]{
        match fs::symlink_metadata(&dir){
            Ok(m) if m.is_dir()&&!m.file_type().is_symlink()=>{},
            Ok(_)=>return Err(format!("Unsafe project directory {}",dir.display())),
            Err(e) if e.kind()==std::io::ErrorKind::NotFound=>
                fs::create_dir(&dir).map_err(|e|e.to_string())?,
            Err(e)=>return Err(e.to_string())
        }
    }
    let docs=docs(root);
    write_if_missing(&docs.join("README.md"),include_str!("../README.md"))?;
    write_if_missing(&docs.join("LICENSE-GPL-3.0"),include_str!("../LICENSE"))?;
    write_if_missing(&docs.join("LICENSE-MIT"),include_str!("../LICENSE-MIT"))?;
    write_if_missing(&docs.join("LICENSE-APACHE"),include_str!("../third_party/game-frame/LICENSE-APACHE"))?;
    write_if_missing(&docs.join("CyberGladiatorsFix.ini.example"),crate::settings::DEFAULT)?;
    write_if_missing(&docs.join("build-info.txt"),&format!(
        "CyberGladiatorsFix {}\nCommit={}\nLayout=unified-1\nEdition={}\n",
        env!("CARGO_PKG_VERSION"),env!("CYBER_COMMIT"),edition))?;
    write_if_missing(&state(root).join("CyberGladiatorsFix.install-receipt.txt"),
        &format!("Layout=unified-1\nEdition={}\nOriginalRetailBackup=game/CYBER.EXE.bak\n",edition))?;
    Ok(())
}

#[cfg(test)]
mod support_file_tests{
    use super::*;
    use std::fs;
    #[test] fn writes_state_and_docs_without_creating_named_fix_dir(){
        let root=std::env::temp_dir().join(format!("cyber-layout-support-{}",std::process::id()));
        let _=fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        ensure_support_files(&root,"USA retail").unwrap();
        assert!(docs(&root).join("README.md").is_file());
        assert!(docs(&root).join("LICENSE-APACHE").is_file());
        assert!(docs(&root).join("CyberGladiatorsFix.ini.example").is_file());
        assert!(state(&root).join("CyberGladiatorsFix.install-receipt.txt").is_file());
        assert!(!root.join("CyberGladiatorsFix").exists());
        let before=fs::read(docs(&root).join("README.md")).unwrap();
        ensure_support_files(&root,"USA retail").unwrap();
        assert_eq!(fs::read(docs(&root).join("README.md")).unwrap(),before);
        fs::remove_dir_all(root).unwrap();
    }
}
