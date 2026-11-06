// SPDX-License-Identifier: GPL-3.0-or-later
//! Manifest-checked, reversible migration of legacy root/TrackNN.flac files
//! into music/TrackNN.flac. Never move an unverified user file or overwrite
//! a destination, a receipt, or game-generated IQ resources.
use crate::install::MANIFEST;
use sha2::{Digest,Sha256};
use std::fs::{self,File,OpenOptions};
use std::io::{Read,Write};
use std::path::{Path,PathBuf};

fn track_name(s:&str)->bool{
    let Some(n)=s.strip_prefix("Track").and_then(|s|s.strip_suffix(".flac"))else{return false;};
    n.len()==2 && n.bytes().all(|b|b.is_ascii_digit()) &&
        n.parse::<u8>().is_ok_and(|n|(2..=99).contains(&n))
}
fn regular(path:&Path)->Result<(),String>{
    let m=fs::symlink_metadata(path).map_err(|e|format!("Missing music file {}: {e}",path.display()))?;
    if !m.is_file()||m.file_type().is_symlink(){return Err(format!("Not a safe regular music file: {}",path.display()));}
    Ok(())
}
fn file_hash(path:&Path)->Result<(u64,String),String>{
    regular(path)?;
    let mut input=File::open(path).map_err(|e|e.to_string())?;
    let mut hash=Sha256::new();let mut len=0u64;
    let mut buf=[0u8;64*1024];
    loop{
        let n=input.read(&mut buf).map_err(|e|e.to_string())?;
        if n==0{break;}len+=n as u64;hash.update(&buf[..n]);
    }
    Ok((len,format!("{:x}",hash.finalize())))
}
fn absent(path:&Path)->Result<(),String>{
    match fs::symlink_metadata(path){
        Ok(_)=>Err(format!("Refusing to overwrite existing {}",path.display())),
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(()),
        Err(e)=>Err(format!("Cannot inspect {}: {e}",path.display()))
    }
}
struct Track{old:PathBuf,new:PathBuf,size:u64,hash:String}
pub fn migrate(game:&Path)->Result<usize,String>{
    let manifest=game.join(MANIFEST);
    regular(&manifest)?;
    let input=fs::read_to_string(&manifest).map_err(|e|format!("Cannot read manifest: {e}"))?;
    if input.len()>4*1024*1024||!input.starts_with("# CyberGladiatorsFix manifest v1\n"){
        return Err("Unsupported manifest: refusing audio migration".into());
    }
    let mut tracks=Vec::<Track>::new();
    let mut updated=String::new();
    let mut keys=std::collections::BTreeSet::new();
    for (i,line) in input.lines().enumerate(){
        if i==0{updated.push_str(line);updated.push('\n');continue;}
        let parts:Vec<_>=line.split('\t').collect();
        if parts.len()!=3{return Err("Malformed manifest row: audio migration refused".into());}
        let key=parts[0].to_ascii_lowercase();
        if !keys.insert(key){return Err("Duplicate manifest entry: audio migration refused".into());}
        if track_name(parts[0]){
            let size=parts[1].parse::<u64>().map_err(|e|e.to_string())?;
            if parts[2].len()!=64||!parts[2].bytes().all(|b|b.is_ascii_hexdigit()){
                return Err("Invalid legacy track hash in manifest".into());
            }
            tracks.push(Track{
                old:game.join(parts[0]),
                new:game.join("music").join(parts[0]),
                size,hash:parts[2].to_owned()
            });
            updated.push_str("music/");
        }
        updated.push_str(line);updated.push('\n');
    }
    if tracks.is_empty(){return Ok(0);}
    if tracks.len()>98{return Err("Unexpected number of legacy CD tracks".into());}
    for t in &tracks{
        if keys.contains(&format!("music/{}",t.old.file_name().unwrap().to_string_lossy()).to_ascii_lowercase()){
            return Err("Manifest has both legacy and music/ track entries".into());
        }
        let (size,hash)=file_hash(&t.old)?;
        if size!=t.size||hash!=t.hash{
            return Err(format!("Legacy music track has changed: {}. No files moved.",t.old.display()));
        }
        absent(&t.new)?;
    }
    let music=game.join("music");
    let mut created=false;
    match fs::symlink_metadata(&music){
        Ok(m)=>{
            if !m.is_dir()||m.file_type().is_symlink()||fs::read_dir(&music).map_err(|e|e.to_string())?.next().is_some(){
                return Err("Existing music/ is nonempty or unsafe: refusing legacy migration".into());
            }
        }
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>{
            fs::create_dir(&music).map_err(|e|format!("Cannot create music/: {e}"))?;
            created=true;
        }
        Err(e)=>return Err(format!("Cannot inspect music/: {e}"))
    }
    let staged=game.join("CyberGladiatorsFix.manifest.music.tmp");
    let backup=game.join("CyberGladiatorsFix.manifest.pre-music.bak");
    if let Err(e)=absent(&staged).and_then(|_|absent(&backup)){
        if created{let _=fs::remove_dir(&music);}return Err(e);
    }
    let prepared=(||->Result<(),String>{
        let mut output=OpenOptions::new().write(true).create_new(true).open(&staged)
            .map_err(|e|format!("Cannot stage music manifest: {e}"))?;
        output.write_all(updated.as_bytes()).and_then(|_|output.sync_all()).map_err(|e|e.to_string())?;
        Ok(())
    })();
    if let Err(e)=prepared{
        let _=fs::remove_file(&staged);
        if created{let _=fs::remove_dir(&music);}
        return Err(e);
    }
    let mut done=Vec::new();
    for t in &tracks{
        if let Err(e)=fs::rename(&t.old,&t.new){
            for index in done.into_iter().rev(){
                let track:&Track=&tracks[index];
                let _=fs::rename(&track.new,&track.old);
            }
            let _=fs::remove_file(&staged);
            if created{let _=fs::remove_dir(&music);}
            return Err(format!("Cannot relocate {}: {e}; migration rolled back",t.old.display()));
        }
        done.push(done.len());
    }
    if let Err(e)=fs::rename(&manifest,&backup){
        for t in tracks.iter().rev(){let _=fs::rename(&t.new,&t.old);}
        let _=fs::remove_file(&staged);
        if created{let _=fs::remove_dir(&music);}
        return Err(format!("Cannot preserve previous manifest: {e}; migration rolled back"));
    }
    if let Err(e)=fs::rename(&staged,&manifest){
        let restore=fs::rename(&backup,&manifest);
        for t in tracks.iter().rev(){let _=fs::rename(&t.new,&t.old);}
        return Err(format!("Cannot publish migrated music manifest: {e}; original manifest restore={restore:?}"));
    }
    // Last check proves a successful migration before launch. Retain original
    // receipt as a separate .bak to protect user data if recovery is required.
    for t in &tracks {
        let (n,h)=file_hash(&t.new)?;
        if n!=t.size||h!=t.hash{return Err(format!("Moved music file failed integrity: {}",t.new.display()));}
    }
    Ok(tracks.len())
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn migrate_verified_root_tracks_preserves_data_and_manifest_backup(){
        let d=std::env::temp_dir().join(format!("CyberMusic-{}-{}",std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir(&d).unwrap();
        let bytes=b"demo audio";
        let hash=format!("{:x}",Sha256::digest(bytes));
        fs::write(d.join("Track02.flac"),bytes).unwrap();
        fs::write(d.join(MANIFEST),format!(
            "# CyberGladiatorsFix manifest v1\nTrack02.flac\t{}\t{hash}\n",bytes.len())).unwrap();
        assert_eq!(migrate(&d).unwrap(),1);
        assert_eq!(fs::read(d.join("music/Track02.flac")).unwrap(),bytes);
        assert_eq!(migrate(&d).unwrap(),0);
        assert!(d.join("CyberGladiatorsFix.manifest.pre-music.bak").exists());
        assert!(!d.join("Track02.flac").exists());
        fs::remove_dir_all(&d).unwrap();
    }
    #[test]
    fn refuse_modified_audio_without_moving(){
        let d=std::env::temp_dir().join(format!("CyberMusicBad-{}-{}",std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir(&d).unwrap();
        fs::write(d.join("Track02.flac"),b"bad").unwrap();
        fs::write(d.join(MANIFEST),format!(
            "# CyberGladiatorsFix manifest v1\nTrack02.flac\t3\t{}\n","a".repeat(64))).unwrap();
        assert!(migrate(&d).is_err());
        assert!(d.join("Track02.flac").exists());
        assert!(!d.join("music").exists());
        fs::remove_dir_all(&d).unwrap();
    }
}