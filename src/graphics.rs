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
