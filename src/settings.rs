// SPDX-License-Identifier: GPL-3.0-or-later
//! Text settings for CyberGladiatorsFix, intentionally separate from the
//! game's 32-byte binary cyber.ini preferences file.
use std::fs::{self,OpenOptions};
use std::io::Write;
use std::path::Path;

pub const NAME:&str="CyberGladiatorsFix.ini";
const MAX_SIZE:u64=4096;
const DEFAULT:&str="; CyberGladiatorsFix preferences (NOT the game's binary cyber.ini)\r\n; 0 = play the opening AVI (default), 1 = skip only the opening AVI.\r\n[Video]\r\nSkipIntro=0\r\n\r\n; CD audio bridge enabled for native video + lossless music by default.\r\n; 0 = system WinMM + legacy isolated AVI fallback; 1 = FLAC CDDA bridge.\r\n[Audio]\r\nEnableCDDA=1\r\n\r\n; Disabled by default. Opt-in for games which update IQ data while running.\r\n; --verify remains strict even when this option is enabled.\r\n[Integrity]\r\nAllowModifiedIQ=0\r\n";

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

/// New installs enable the tested WinMM bridge and in-process video. Legacy
/// INIs lacking [Audio] remain backward compatible and use system WinMM.
fn parse_cdda(text:&str)->Result<bool,String>{
    let mut audio=false;
    let mut seen_section=false;
    let mut choice:Option<bool>=None;
    for (line_no,raw) in text.trim_start_matches('\u{feff}').lines().enumerate(){
        let line=raw.trim();
        if line.is_empty()||line.starts_with(';')||line.starts_with('#'){continue;}
        if line.starts_with('['){
            if !line.ends_with(']'){return Err(format!("Malformed INI section on line {}",line_no+1));}
            audio=line[1..line.len()-1].eq_ignore_ascii_case("Audio");
            if audio{
                if seen_section{return Err("Duplicate [Audio] section".into());}
                seen_section=true;
            }
            continue;
        }
        let (key,value)=line.split_once('=')
            .ok_or_else(||format!("Malformed INI setting on line {}",line_no+1))?;
        if !audio{continue;}
        if !key.trim().eq_ignore_ascii_case("EnableCDDA"){
            return Err(format!("Unknown [Audio] option: {}",key.trim()));
        }
        if choice.is_some(){return Err("Duplicate EnableCDDA".into());}
        choice=Some(match value.trim().to_ascii_lowercase().as_str(){
            "0"|"false"|"no"|"off"=>false,
            "1"|"true"|"yes"|"on"=>true,
            _=>return Err("EnableCDDA must be 0 or 1".into()),
        });
    }
    if seen_section && choice.is_none(){
        return Err("[Audio] section requires EnableCDDA=0 or 1".into());
    }
    Ok(choice.unwrap_or(false))
}

/// Read the explicit, game-local opt-in to retain modified, same-size IQ
/// assets on launch. The strict --verify command ignores this setting.
/// Older INIs without an [Integrity] section keep the fail-closed default.
fn parse_iq_policy(text:&str)->Result<bool,String>{
    let mut integrity=false;
    let mut seen_section=false;
    let mut choice:Option<bool>=None;
    for (line_no,raw) in text.trim_start_matches('\u{feff}').lines().enumerate(){
        let line=raw.trim();
        if line.is_empty()||line.starts_with(';')||line.starts_with('#'){continue;}
        if line.starts_with('['){
            if !line.ends_with(']'){return Err(format!("Malformed INI section on line {}",line_no+1));}
            integrity=line[1..line.len()-1].eq_ignore_ascii_case("Integrity");
            if integrity {
                if seen_section{return Err("Duplicate [Integrity] section".into());}
                seen_section=true;
            }
            continue;
        }
        let (key,value)=line.split_once('=')
            .ok_or_else(||format!("Malformed INI setting on line {}",line_no+1))?;
        if !integrity{continue;}
        if !key.trim().eq_ignore_ascii_case("AllowModifiedIQ"){
            return Err(format!("Unknown [Integrity] option: {}",key.trim()));
        }
        if choice.is_some(){return Err("Duplicate AllowModifiedIQ".into());}
        choice=Some(match value.trim().to_ascii_lowercase().as_str(){
            "0"|"false"|"no"|"off"=>false,
            "1"|"true"|"yes"|"on"=>true,
            _=>return Err("AllowModifiedIQ must be 0 or 1".into()),
        });
    }
    if seen_section && choice.is_none(){
        return Err("[Integrity] section requires AllowModifiedIQ=0 or 1".into());
    }
    Ok(choice.unwrap_or(false))
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
            read(game)?;read_cdda(game)?;read_allow_modified_iq(game).map(|_|())
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

pub fn read_cdda(game:&Path)->Result<bool,String>{
    let path=game.join(NAME);
    let metadata=fs::symlink_metadata(&path)
        .map_err(|e|format!("Cannot find {}: {e}",path.display()))?;
    if !metadata.is_file()||metadata.file_type().is_symlink()||metadata.len()>MAX_SIZE{
        return Err(format!("{NAME} must be a regular text file of at most {MAX_SIZE} bytes"));
    }
    let contents=fs::read_to_string(&path).map_err(|e|format!("Cannot read {NAME}: {e}"))?;
    parse_cdda(&contents)
}

/// Whether the user explicitly permits same-size changes to IQ data during
/// launch. This reads only the tool's text configuration; no IQ files touched.
pub fn read_allow_modified_iq(game:&Path)->Result<bool,String>{
    let path=game.join(NAME);
    let metadata=fs::symlink_metadata(&path)
        .map_err(|e|format!("Cannot find {}: {e}",path.display()))?;
    if !metadata.is_file()||metadata.file_type().is_symlink()||metadata.len()>MAX_SIZE{
        return Err(format!("{NAME} must be a regular text file of at most {MAX_SIZE} bytes"));
    }
    let contents=fs::read_to_string(&path).map_err(|e|format!("Cannot read {NAME}: {e}"))?;
    parse_iq_policy(&contents)
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn modified_iq_opt_in_is_explicit_and_backward_compatible(){
        assert!(!parse_iq_policy(DEFAULT).unwrap());
        assert!(!parse_iq_policy("[Video]\nSkipIntro=0\n[Audio]\nEnableCDDA=1\n").unwrap());
        assert!(parse_iq_policy("[Video]\nSkipIntro=0\n[Integrity]\nAllowModifiedIQ=1\n").unwrap());
        for bad in [
            "[Integrity]\nAllowModifiedIQ=maybe\n",
            "[Integrity]\nAllowModifiedIQ=1\nAllowModifiedIQ=0\n",
            "[Integrity]\nUnknownKey=1\n",
            "[Integrity]\n",
            "[Integrity]\nAllowModifiedIQ=0\n[Integrity]\nAllowModifiedIQ=1\n",
        ] {assert!(parse_iq_policy(bad).is_err(),"{bad}");}
    }
    #[test] fn new_installs_enable_bridge_but_legacy_ini_without_audio_stays_off(){
        assert!(parse_cdda(DEFAULT).unwrap());
        assert!(!parse_cdda("[Video]\nSkipIntro=1\n").unwrap());
        assert!(parse_cdda("[Video]\nSkipIntro=1\n[Audio]\nEnableCDDA=1\n").unwrap());
        assert!(!parse_cdda("[Video]\nSkipIntro=1\n[Audio]\nEnableCDDA=off\n").unwrap());
    }
    #[test] fn rejects_invalid_audio_configuration(){
        for input in [
            "[Video]\nSkipIntro=0\n[Audio]\nEnableCDDA=2",
            "[Video]\nSkipIntro=0\n[Audio]\nEnableCDDA=0\nEnableCDDA=1",
            "[Video]\nSkipIntro=0\n[Audio]\nExtra=1",
            "[Video]\nSkipIntro=0\n[Audio]",
            "[Video]\nSkipIntro=0\n[Audio]\nEnableCDDA=0\n[Audio]\nEnableCDDA=1",
        ]{assert!(parse_cdda(input).is_err(),"{input}");}
    }
    #[test] fn defaults_play_video_and_are_parseable(){
        assert!(!parse(DEFAULT).unwrap());
        assert_eq!(NAME,"CyberGladiatorsFix.ini");
        assert!(parse_cdda(DEFAULT).unwrap());
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
        assert!(read_cdda(&root).unwrap(),"new installation should prefer native AVI + CDDA");
        // An existing user choice of system WinMM must never be rewritten
        // when the newer launcher introduces its different default.
        let value="[Video]\nSkipIntro=1\n[Audio]\nEnableCDDA=0\n";
        fs::write(root.join(NAME),value).unwrap();
        ensure(&root).unwrap();
        assert!(read(&root).unwrap());
        assert!(!read_cdda(&root).unwrap(),"existing CDDA opt-out must be preserved");
        assert_eq!(fs::read_to_string(root.join(NAME)).unwrap(),value);
        fs::remove_dir_all(root).unwrap();
    }
}
