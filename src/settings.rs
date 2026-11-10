// SPDX-License-Identifier: GPL-3.0-or-later
//! Only user-selectable settings belong in CyberGladiatorsFix.ini.
//! CDDA and immutable retail SHA-256 checks are mandatory. Game-writable
//! IQ files are strictly verified on first install, then checked for presence.
use std::fs::{self,OpenOptions};
use std::io::Write;
use std::path::Path;

pub const NAME:&str="CyberGladiatorsFix.ini";
pub const CDDA_ENABLED:bool=true;
const MAX_SIZE:u64=4096;
const DEFAULT:&str="; CyberGladiatorsFix preferences (NOT the game's binary cyber.ini)\r\n; 0 = play the opening AVI (default), 1 = skip only the opening AVI.\r\n[Video]\r\nSkipIntro=0\r\n";

fn parse(text:&str)->Result<bool,String>{
    let mut video=false;
    let mut seen_video=false;
    let mut choice:Option<bool>=None;
    for (line_no,raw) in text.trim_start_matches('\u{feff}').lines().enumerate(){
        let line=raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#'){continue;}
        if line.starts_with('['){
            if !line.ends_with(']'){return Err(format!("Malformed INI section on line {}",line_no+1));}
            video=line[1..line.len()-1].eq_ignore_ascii_case("Video");
            if video{
                if seen_video{return Err("Duplicate [Video] section in CyberGladiatorsFix.ini".into());}
                seen_video=true;
            }
            continue;
        }
        let (key,value)=line.split_once('=')
            .ok_or_else(||format!("Malformed INI setting on line {}",line_no+1))?;
        if !video{continue;}
        if !key.trim().eq_ignore_ascii_case("SkipIntro"){
            return Err(format!("Unknown [Video] option: {}",key.trim()));
        }
        if choice.is_some(){return Err("Duplicate SkipIntro in CyberGladiatorsFix.ini".into());}
        choice=Some(match value.trim().to_ascii_lowercase().as_str(){
            "0"|"false"|"off"|"no"=>false,
            "1"|"true"|"on"|"yes"=>true,
            _=>return Err("Invalid SkipIntro value: use 0 or 1".into()),
        });
    }
    if !seen_video{return Err("CyberGladiatorsFix.ini must contain [Video]".into());}
    choice.ok_or_else(||"CyberGladiatorsFix.ini is missing SkipIntro=0 or SkipIntro=1".into())
}

/// Remove only retired programmatic [Audio]/[Integrity] sections and their
/// exact old default comment lines. Preserve [Video], its user choice,
/// unrelated sections, casing, comments, BOM, and line endings byte-for-byte.
fn without_legacy_options(text:&str)->(String,bool){
    let mut output=String::with_capacity(text.len());
    let mut discard=false;
    let mut changed=false;
    for line in text.split_inclusive('\n'){
        let trimmed=line.trim();
        if trimmed.starts_with('[')&&trimmed.ends_with(']'){
            let section=trimmed[1..trimmed.len()-1].trim();
            discard=section.eq_ignore_ascii_case("Audio")
                ||section.eq_ignore_ascii_case("Integrity");
        }
        let old_generated_comment=matches!(trimmed,
            "; This option controls only CD music; the original native AVI remains enabled." |
            "; 0 = forward CD audio commands to system WinMM; 1 = virtual FLAC CD audio." |
            "; Disabled by default. Opt-in for games which update IQ data while running." |
            "; --verify remains strict even when this option is enabled."
        );
        if discard||old_generated_comment{
            changed=true;
        }else{
            output.push_str(line);
        }
    }
    (output,changed)
}
fn read_text(game:&Path)->Result<String,String>{
    let path=game.join(NAME);
    let m=fs::symlink_metadata(&path)
        .map_err(|e|format!("Cannot find {NAME}: {e}"))?;
    if !m.is_file()||m.file_type().is_symlink()||m.len()>MAX_SIZE{
        return Err(format!("{NAME} must be a regular text file of at most {MAX_SIZE} bytes"));
    }
    fs::read_to_string(&path).map_err(|e|format!("Cannot read {NAME}: {e}"))
}
/// Existing video preference is authoritative. Obsolete switches are ignored
/// immediately and deleted by a safe migration, without modifying game data.
/// New installs contain [Video] only.
pub fn ensure(game:&Path)->Result<(),String>{
    let path=game.join(NAME);
    match fs::symlink_metadata(&path){
        Ok(m)=>{
            if m.file_type().is_symlink()||!m.is_file()||m.len()>MAX_SIZE{
                return Err(format!("{NAME} must be a regular, non-symlinked text file up to {MAX_SIZE} bytes"));
            }
            let contents=read_text(game)?;
            parse(&contents)?;
            let (cleaned,changed)=without_legacy_options(&contents);
            if !changed{return Ok(());}
            parse(&cleaned)?; // Never replace a valid file with invalid settings.
            let staging=game.join("CyberGladiatorsFix.ini.cleanup.tmp");
            let mut file=OpenOptions::new().create_new(true).write(true).open(&staging)
                .map_err(|e|format!("Cannot stage obsolete INI cleanup: {e}"))?;
            if let Err(e)=file.write_all(cleaned.as_bytes()).and_then(|_|file.sync_all()){
                drop(file);
                let _=fs::remove_file(&staging);
                return Err(format!("Cannot prepare INI cleanup: {e}"));
            }
            drop(file);
            if let Err(e)=fs::rename(&staging,&path){
                let _=fs::remove_file(&staging);
                return Err(format!("Cannot finalize obsolete INI cleanup (old config retained): {e}"));
            }
            if let Some(root)=game.parent(){
                crate::show("INI_CLEANUP removed_legacy_sections=Audio,Integrity preserved_SkipIntro=1 cdda_fixed=1 iq_verify=presence_only_after_install",root);
            }
            Ok(())
        },
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>{
            let mut file=OpenOptions::new().write(true).create_new(true).open(&path)
                .map_err(|e|format!("Cannot create {}: {e}",path.display()))?;
            if let Err(e)=file.write_all(DEFAULT.as_bytes()).and_then(|_|file.sync_all()){
                drop(file);
                let _=fs::remove_file(&path);
                return Err(format!("Cannot initialize {NAME}: {e}"));
            }
            Ok(())
        },
        Err(e)=>Err(format!("Cannot inspect {NAME}: {e}")),
    }
}
pub fn read(game:&Path)->Result<bool,String>{
    parse(&read_text(game)?)
}
#[cfg(test)]
mod tests{
    use super::*;
    fn fixture(name:&str)->std::path::PathBuf{
        let folder=std::env::temp_dir().join(format!("CyberSettings-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .unwrap().as_nanos()));
        fs::create_dir_all(&folder).unwrap();
        folder
    }
    #[test]
    fn default_ini_exposes_video_only_and_cdda_is_internal(){
        assert!(!parse(DEFAULT).unwrap());
        assert!(CDDA_ENABLED);
        assert!(DEFAULT.contains("[Video]\r\nSkipIntro=0"));
        assert!(!DEFAULT.contains("[Audio]"));
        assert!(!DEFAULT.contains("[Integrity]"));
        assert!(!DEFAULT.contains("EnableCDDA"));
        assert!(!DEFAULT.contains("AllowModifiedIQ"));
    }
    #[test]
    fn accepts_skip_toggle_and_comments(){
        assert!(parse("[Video]\r\nSkipIntro=1\r\n").unwrap());
        assert!(parse("\u{feff}; comment\r\n[Video]\r\n skipintro = TRUE \r\n").unwrap());
        assert!(!parse("[Other]\nAny=Thing\n[Video]\nSkipIntro=off\n").unwrap());
    }
    #[test]
    fn fails_closed_on_ambiguous_video_settings(){
        for bad in [
            "[Video]\nSkipIntro=2",
            "[Video]\nSkipIntro=1\nSkipIntro=0",
            "[Video]\nExtra=1\nSkipIntro=0",
            "[Video]\n",
            "SkipIntro=1",
            "[Video]\nSkipIntro=0\n[Video]\nSkipIntro=1",
        ]{assert!(parse(bad).is_err(),"{bad}");}
    }
    #[test]
    fn obsolete_sections_are_dropped_while_other_settings_survive(){
        let old=concat!(
            "; Header\r\n[Video]\r\nSkipIntro=1\r\n\r\n",
            "; This option controls only CD music; the original native AVI remains enabled.\r\n",
            "; 0 = forward CD audio commands to system WinMM; 1 = virtual FLAC CD audio.\r\n",
            "[Audio]\r\nEnableCDDA=0\r\n\r\n",
            "; Disabled by default. Opt-in for games which update IQ data while running.\r\n",
            "; --verify remains strict even when this option is enabled.\r\n",
            "[Integrity]\r\nAllowModifiedIQ=1\r\n",
            "[Other]\r\nCustom=keep\r\n"
        );
        let (new,changed)=without_legacy_options(old);
        assert!(changed);
        assert!(!new.contains("[Audio]"));
        assert!(!new.contains("[Integrity]"));
        assert!(!new.contains("EnableCDDA"));
        assert!(!new.contains("AllowModifiedIQ"));
        assert!(new.contains("SkipIntro=1\r\n"));
        assert!(new.contains("[Other]\r\nCustom=keep\r\n"));
        assert!(parse(&new).unwrap());
        assert_eq!(without_legacy_options(&new),(new.clone(),false));
    }
    #[test]
    fn ensure_migrates_old_ini_and_keeps_video_setting(){
        let folder=fixture("migration");
        let old="[Video]\r\nSkipIntro=1\r\n[Audio]\r\nEnableCDDA=0\r\n[Integrity]\r\nAllowModifiedIQ=1\r\n";
        fs::write(folder.join(NAME),old).unwrap();
        ensure(&folder).unwrap();
        let updated=fs::read_to_string(folder.join(NAME)).unwrap();
        assert!(read(&folder).unwrap());
        assert!(!updated.contains("EnableCDDA"));
        assert!(!updated.contains("AllowModifiedIQ"));
        assert!(!updated.contains("[Audio]"));
        assert!(!updated.contains("[Integrity]"));
        assert!(!folder.join("CyberGladiatorsFix.ini.cleanup.tmp").exists());
        ensure(&folder).unwrap();
        assert_eq!(fs::read_to_string(folder.join(NAME)).unwrap(),updated);
        fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn invalid_ini_is_never_migrated_or_overwritten(){
        let folder=fixture("invalid");
        let old="[Video]\nSkipIntro=7\n[Audio]\nEnableCDDA=0\n";
        fs::write(folder.join(NAME),old).unwrap();
        assert!(ensure(&folder).is_err());
        assert_eq!(fs::read_to_string(folder.join(NAME)).unwrap(),old);
        fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn new_ini_and_existing_nonlegacy_ini_are_preserved(){
        let folder=fixture("new");
        ensure(&folder).unwrap();
        let created=fs::read_to_string(folder.join(NAME)).unwrap();
        assert_eq!(created,DEFAULT);
        assert!(!read(&folder).unwrap());
        let custom="[Video]\nSkipIntro=1\n[Custom]\nSetting=abc\n";
        fs::write(folder.join(NAME),custom).unwrap();
        ensure(&folder).unwrap();
        assert_eq!(fs::read_to_string(folder.join(NAME)).unwrap(),custom);
        assert!(read(&folder).unwrap());
        fs::remove_dir_all(folder).unwrap();
    }
}
