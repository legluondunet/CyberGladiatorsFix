// SPDX-License-Identifier: GPL-3.0-or-later
//! Safely maximize the two saved graphics quality fields in CyberGladiators'
//! own legacy 32-byte binary CYBER.INI. Never overwrite other preferences.
use std::fs::{self,OpenOptions};
use std::io::Write;
use std::path::Path;

const SIZE:usize=32;
const RESOLUTION_OFFSET:usize=16;
const DETAIL_OFFSET:usize=20;

// Reference: 32-byte CYBER.INI written by the USA game and supplied for
// analysis. Other six DWORDs are retained from this known parseable sample;
// their exact meanings have not been independently characterized.
// At offsets 0x10 and 0x14: screen size index 0 and detail 3 are maximum.
const USA_INI_TEMPLATE:[u32;8]=[1,2,0,0,0,3,4,1];

fn default_bytes()->[u8;SIZE] {
    let mut data=[0u8;SIZE];
    for (index,value) in USA_INI_TEMPLATE.iter().enumerate() {
        data[index*4..index*4+4].copy_from_slice(&value.to_le_bytes());
    }
    data
}

/// Create a known-format 32-byte CYBER.INI inside the staged installation
/// while extracting a recognized USA CD. Never overwrite an existing file,
/// and let the caller remove its own staging directory on any error.
pub fn create_defaults(game:&Path)->Result<(),String> {
    let path=game.join("cyber.ini");
    let mut file=OpenOptions::new().write(true).create_new(true).open(&path)
        .map_err(|e|format!("Cannot create default CYBER.INI at {}: {e}",path.display()))?;
    let data=default_bytes();
    if let Err(e)=file.write_all(&data).and_then(|_|file.sync_all()){
        drop(file);
        let _=fs::remove_file(&path);
        return Err(format!("Cannot write default CYBER.INI: {e}"));
    }
    Ok(())
}

fn u32_at(data:&[u8],offset:usize)->u32{
    u32::from_le_bytes(data[offset..offset+4].try_into().unwrap())
}
fn maximize_bytes(data:&mut [u8])->Result<bool,String>{
    if data.len()!=SIZE{
        return Err(format!("Unknown CYBER.INI format ({} bytes; expected {SIZE}); unchanged",data.len()));
    }
    let resolution=u32_at(data,RESOLUTION_OFFSET);
    let detail=u32_at(data,DETAIL_OFFSET);
    // Verified in CYBER.EXE v1.0: resolution index 0 = 640x480
    // (indices 2-3 = 512x384, 4-5 = 384x288), and detail 3 is
    // the highest of 0..3. The other six fields are not modified.
    if resolution>5 || detail>3 {
        return Err(format!("Unrecognized CYBER.INI graphics values: screen {resolution}, detail {detail}; unchanged"));
    }
    if resolution==0 && detail==3{return Ok(false);}
    data[RESOLUTION_OFFSET..RESOLUTION_OFFSET+4].copy_from_slice(&0u32.to_le_bytes());
    data[DETAIL_OFFSET..DETAIL_OFFSET+4].copy_from_slice(&3u32.to_le_bytes());
    Ok(true)
}

/// Read only the two binary CYBER.INI fields whose meanings were verified.
/// Never interpret, log or change the other six unknown DWORDs.
pub fn describe_saved(game:&Path)->Result<String,String>{
    let path=game.join("cyber.ini");
    let m=fs::symlink_metadata(&path).map_err(|e|format!("CYBER.INI unavailable: {e}"))?;
    if !m.is_file()||m.file_type().is_symlink()||m.len()!=SIZE as u64 {
        return Err("CYBER.INI format is not the recognized 32-byte regular file".into());
    }
    let data=fs::read(&path).map_err(|e|format!("Cannot read CYBER.INI: {e}"))?;
    let screen=u32_at(&data,RESOLUTION_OFFSET);
    let detail=u32_at(&data,DETAIL_OFFSET);
    if screen>5||detail>3{
        return Err("CYBER.INI screen/detail fields are outside verified ranges".into());
    }
    let resolution=match screen {0|1=>"640x480",2|3=>"512x384",4|5=>"384x288",_=>unreachable!()};
    Ok(format!("CYBER.INI internal_resolution={} (index={screen}) graphics_detail={detail}/3",resolution))
}

/// Only touch a validated existing preferences file. The game generates
/// CYBER.INI when settings are saved on exit. Do not fabricate unrelated
/// control or input settings for users who have not launched it yet.
pub fn maximize_saved(game:&Path)->Result<String,String>{
    let path=game.join("cyber.ini");
    if !path.exists() {
        return Err("CYBER.INI not found. Start the game once, exit normally, then run --max-graphics again. In game, use + and ] to maximize screen size and detail.".into());
    }
    let meta=fs::symlink_metadata(&path).map_err(|e|e.to_string())?;
    if !meta.is_file()||meta.file_type().is_symlink(){
        return Err("CYBER.INI must be a regular, non-symlinked file".into());
    }
    let original=fs::read(&path).map_err(|e|e.to_string())?;
    let mut updated=original.clone();
    if !maximize_bytes(&mut updated)? {
        return Ok("Graphics already at maximum: 640x480 internal size, detail 3/3".into());
    }
    let backup=game.join("cyber.ini.bak");
    if backup.is_symlink() {
        return Err("Refusing symlink CYBER.INI backup".into());
    }
    if !backup.exists() {
        let mut out=OpenOptions::new().write(true).create_new(true).open(&backup)
            .map_err(|e|format!("Cannot back up CYBER.INI: {e}"))?;
        out.write_all(&original).and_then(|_|out.sync_all())
            .map_err(|e|format!("Cannot write CYBER.INI backup: {e}"))?;
    } else {
        let m=fs::symlink_metadata(&backup).map_err(|e|e.to_string())?;
        if !m.is_file(){return Err("Existing CYBER.INI backup is not a regular file".into());}
    }

    let tmp=game.join("cyber.ini.tmp");
    let retired=game.join("cyber.ini.old.tmp");
    if tmp.exists()||tmp.is_symlink()||retired.exists()||retired.is_symlink(){
        return Err("Refusing to overwrite temporary CYBER.INI files".into());
    }
    {
        let mut out=OpenOptions::new().write(true).create_new(true).open(&tmp)
            .map_err(|e|format!("Cannot create temporary CYBER.INI: {e}"))?;
        if let Err(e)=out.write_all(&updated).and_then(|_|out.sync_all()){
            drop(out);let _=fs::remove_file(&tmp);
            return Err(format!("Cannot write temporary CYBER.INI: {e}"));
        }
    }
    if let Err(e)=fs::rename(&path,&retired) {
        let _=fs::remove_file(&tmp);
        return Err(format!("Cannot prepare graphics settings update: {e}"));
    }
    if let Err(e)=fs::rename(&tmp,&path) {
        let restored=fs::rename(&retired,&path);
        return Err(format!("Cannot update CYBER.INI: {e}; restore: {restored:?}"));
    }
    let _=fs::remove_file(&retired);
    Ok("Graphics set to maximum in CYBER.INI (640x480 internal size, detail 3/3); original saved as cyber.ini.bak".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_only_verified_cyber_ini_fields(){
        let game=std::env::temp_dir().join(format!("cyber-display-settings-{}",std::process::id()));
        let _=fs::remove_dir_all(&game);
        fs::create_dir_all(&game).unwrap();
        create_defaults(&game).unwrap();
        let text=describe_saved(&game).unwrap();
        assert!(text.contains("640x480"));
        assert!(text.contains("graphics_detail=3/3"));
        assert!(!text.contains("private"));
        fs::remove_dir_all(game).unwrap();
    }
    #[test]
    fn generated_defaults_match_valid_usa_ini_layout(){
        let template=default_bytes();
        assert_eq!(template.len(),32);
        assert_eq!(
            (0..8).map(|i|u32_at(&template,i*4)).collect::<Vec<_>>(),
            vec![1,2,0,0,0,3,4,1]);
        assert_eq!(u32_at(&template,RESOLUTION_OFFSET),0);
        assert_eq!(u32_at(&template,DETAIL_OFFSET),3);
        let mut second=template;
        assert!(!maximize_bytes(&mut second).unwrap());
        assert_eq!(second,template);
    }
    #[test]
    fn creates_ini_at_extraction_and_refuses_overwrite(){
        let root=std::env::temp_dir().join(format!(
            "cyber-defaults-{}-{}",std::process::id(),"ini"));
        let _=fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        create_defaults(&root).unwrap();
        let ini=root.join("cyber.ini");
        assert_eq!(fs::read(&ini).unwrap(),default_bytes().to_vec());
        let err=create_defaults(&root).unwrap_err();
        assert!(err.contains("Cannot create default CYBER.INI"));
        assert_eq!(fs::read(&ini).unwrap(),default_bytes().to_vec());
        fs::remove_dir_all(&root).unwrap();
    }
    #[test]
    fn patches_only_graphics_dwords(){
        let mut original=[0x55u8;32];
        original[16..20].copy_from_slice(&5u32.to_le_bytes());
        original[20..24].copy_from_slice(&1u32.to_le_bytes());
        let mut output=original;
        assert!(maximize_bytes(&mut output).unwrap());
        assert_eq!(u32_at(&output,16),0);
        assert_eq!(u32_at(&output,20),3);
        assert_eq!(&original[..16],&output[..16]);
        assert_eq!(&original[24..],&output[24..]);
        assert!(!maximize_bytes(&mut output).unwrap());
    }
    #[test]
    fn refuses_unknown_file_layout_and_values(){
        assert!(maximize_bytes(&mut [0u8;28]).is_err());
        let mut data=[0u8;32];
        data[16..20].copy_from_slice(&6u32.to_le_bytes());
        assert!(maximize_bytes(&mut data).is_err());
        data[16..20].copy_from_slice(&0u32.to_le_bytes());
        data[20..24].copy_from_slice(&4u32.to_le_bytes());
        assert!(maximize_bytes(&mut data).is_err());
    }
    #[test]
    fn preserves_user_settings_and_original_backup(){
        let root=std::env::temp_dir().join(format!("cyber-graphics-{}",std::process::id()));
        let _=fs::remove_dir_all(&root);fs::create_dir(&root).unwrap();
        let file=root.join("cyber.ini");
        let mut data=vec![0u8;32];
        data[..4].copy_from_slice(&123u32.to_le_bytes());
        data[16..20].copy_from_slice(&2u32.to_le_bytes());
        data[20..24].copy_from_slice(&1u32.to_le_bytes());
        data[24..28].copy_from_slice(&7u32.to_le_bytes());
        fs::write(&file,&data).unwrap();
        maximize_saved(&root).unwrap();
        let saved=fs::read(&file).unwrap();
        assert_eq!(u32_at(&saved,16),0);
        assert_eq!(u32_at(&saved,20),3);
        assert_eq!(u32_at(&saved,24),7);
        assert_eq!(fs::read(root.join("cyber.ini.bak")).unwrap(),data);
        assert!(maximize_saved(&root).unwrap().contains("already"));
        fs::remove_dir_all(&root).unwrap();
    }
}
