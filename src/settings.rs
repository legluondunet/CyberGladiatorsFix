// SPDX-License-Identifier: GPL-3.0-or-later
//! Text settings for CyberGladiatorsFix, intentionally separate from the
//! game's 32-byte binary cyber.ini preferences file.
use std::fs::{self,OpenOptions};
use std::io::Write;
use std::path::Path;

pub const NAME:&str="CyberGladiatorsFix.ini";
const MAX_SIZE:u64=4096;
const DEFAULT:&str="; CyberGladiatorsFix preferences (NOT the game's binary cyber.ini)\r\n; 0 = play the opening AVI (default), 1 = skip only the opening AVI.\r\n[Video]\r\nSkipIntro=0\r\n";

fn parse(text:&str)->Result<bool,String>{
    let mut video=false;
    let mut seen_video=false;
    let mut choice:Option<bool>=None;
    for (line_no,raw) in text.trim_start_matches('\u{feff}').lines().enumerate(){
        let line=raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {continue;}
        if line.starts_with('[') {
            if !line.ends_with(']'){return Err(format!("Malformed INI section on line {}",line_no+1));}
            video=line[1..line.len()-1].eq_ignore_ascii_case("Video");
            if video {
                if seen_video {return Err("Duplicate [Video] section in CyberGladiatorsFix.ini".into());}
                seen_video=true;
            }
            continue;
        }
        let (key,value)=line.split_once('=')
            .ok_or_else(||format!("Malformed INI setting on line {}",line_no+1))?;
        if !video {continue;}
        if !key.trim().eq_ignore_ascii_case("SkipIntro"){
            return Err(format!("Unknown [Video] option: {}",key.trim()));
        }
        if choice.is_some(){return Err("Duplicate SkipIntro in CyberGladiatorsFix.ini".into());}
        choice=Some(match value.trim().to_ascii_lowercase().as_str() {
            "0"|"false"|"off"|"no"=>false,
            "1"|"true"|"on"|"yes"=>true,
            _=>return Err("Invalid SkipIntro value: use 0 or 1".into()),
        });
    }
    if !seen_video {return Err("CyberGladiatorsFix.ini must contain [Video]".into());}
    choice.ok_or_else(||"CyberGladiatorsFix.ini is missing SkipIntro=0 or SkipIntro=1".into())
}

/// Called only for a new installation or when launching/upgrading an older one.
/// No existing config, game settings or user files are ever overwritten.
pub fn ensure(game:&Path)->Result<(),String>{
    let path=game.join(NAME);
    match fs::symlink_metadata(&path){
        Ok(m)=>{
            if m.file_type().is_symlink()||!m.is_file(){
                return Err(format!("{NAME} must be a regular, non-symlinked file"));
            }
            read(game).map(|_|())
        }
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>{
            let mut file=OpenOptions::new().write(true).create_new(true).open(&path)
                .map_err(|e|format!("Cannot create {}: {e}",path.display()))?;
            if let Err(e)=file.write_all(DEFAULT.as_bytes()).and_then(|_|file.sync_all()){
                drop(file);
                let _=fs::remove_file(&path);
                return Err(format!("Cannot initialize {NAME}: {e}"));
            }
            Ok(())
        }
        Err(e)=>Err(format!("Cannot inspect {NAME}: {e}")),
    }
}
pub fn read(game:&Path)->Result<bool,String>{
    let path=game.join(NAME);
    let metadata=fs::symlink_metadata(&path)
        .map_err(|e|format!("Cannot find {}: {e}",path.display()))?;
    if !metadata.is_file()||metadata.file_type().is_symlink()||metadata.len()>MAX_SIZE{
        return Err(format!("{NAME} must be a regular text file of at most {MAX_SIZE} bytes"));
    }
    let contents=fs::read_to_string(&path).map_err(|e|format!("Cannot read {NAME}: {e}"))?;
    parse(&contents)
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn defaults_play_video_and_are_parseable(){
        assert!(!parse(DEFAULT).unwrap());
        assert_eq!(NAME,"CyberGladiatorsFix.ini");
    }
    #[test] fn accepts_skip_toggle_and_comments(){
        assert!(parse("[Video]\r\nSkipIntro=1\r\n").unwrap());
        assert!(parse("\u{feff}; comment\r\n[Video]\r\n skipintro = TRUE \r\n").unwrap());
        assert!(!parse("[Other]\nAny=Thing\n[Video]\nSkipIntro=off\n").unwrap());
    }
    #[test] fn fails_closed_on_ambiguous_settings(){
        for bad in [
            "[Video]\nSkipIntro=2",
            "[Video]\nSkipIntro=1\nSkipIntro=0",
            "[Video]\nExtra=1\nSkipIntro=0",
            "[Video]\n",
            "SkipIntro=1",
            "[Video]\nSkipIntro=0\n[Video]\nSkipIntro=1",
        ]{assert!(parse(bad).is_err(),"{bad}");}
    }
    #[test] fn generated_file_never_overwrites_a_user_choice(){
        let root=std::env::temp_dir().join(format!("cdda-settings-{}",std::process::id()));
        let _=fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        ensure(&root).unwrap();
        assert!(!read(&root).unwrap());
        let value="[Video]\nSkipIntro=1\n";
        fs::write(root.join(NAME),value).unwrap();
        ensure(&root).unwrap();
        assert!(read(&root).unwrap());
        assert_eq!(fs::read_to_string(root.join(NAME)).unwrap(),value);
        fs::remove_dir_all(root).unwrap();
    }
}
