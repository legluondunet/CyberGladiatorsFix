// SPDX-License-Identifier: GPL-3.0-or-later
//! Minimal, conservative CD image installation built on ScorchedPlanetFix's
//! ISO parser (isomage) and strict known-executable fingerprint design.
use crate::{audio,cdda,cue,graphics,patch,settings};
use isomage::{cat_node,detect_and_parse_filesystem,TreeNode};
use sha2::{Digest,Sha256};
use std::collections::{BTreeSet, BTreeMap};
use std::fs::{self,File,OpenOptions};
use std::io::Write;
use std::path::{Path,PathBuf};

pub const DIR:&str="CyberGladiatorsFix";
pub const MANIFEST:&str="CyberGladiatorsFix.manifest";
const MAX_ENTRIES:usize=25_000;
const MAX_BYTES:u64=8*1024*1024*1024;
// Verified against the supplied USA Track 01 CD data: these immutable assets,
// together with the exact CYBER.EXE, identify the supported retail edition.
const USA_EDITION:&str="USA (Track 01, 1996)";
const USA_SHELL_FNT_SHA256:&str="d0a02e7aebd5104754debdca610bc7e94a2d88c7b97452c64a211fa3fbf477a7";
const USA_SPLASH_AVI_SHA256:&str="41773e2525a2b7e959e1bcb44ad3cd640df780551a13b110c26ce53b5fdbc936";
const FR_EDITION:&str="France (CYBERG, 1996)";
const FR_SHELL_FNT_SHA256:&str="45d2241819d32332d736f01f06b4cff694677edf1fba27588ffa8a9ff0a5d177";
const FR_SPLASH4_AVI_SHA256:&str="55bec4abb94c1a0a309eb566b8120ff0460ba69a9125f1c9c3e28c101ef05ff0";

#[derive(Clone)]
struct Asset{node:TreeNode,path:PathBuf,size:u64}

pub fn find_cue(cwd:&Path)->Result<PathBuf,String>{
    let mut matches=Vec::<PathBuf>::new();
    for entry in fs::read_dir(cwd).map_err(|e|e.to_string())? {
        let p=entry.map_err(|e|e.to_string())?.path();
        let m=fs::symlink_metadata(&p).map_err(|e|e.to_string())?;
        if m.is_file()&&!m.file_type().is_symlink()&&p.extension().is_some_and(|s|s.eq_ignore_ascii_case("cue")){
            matches.push(p);
        }
    }
    if matches.len()!=1{return Err(format!("Expected exactly one .cue file in {}, found {}. Supply an explicit path.",cwd.display(),matches.len()));}
    Ok(matches.remove(0))
}
fn safe_name(name:&str)->bool{
    if name.is_empty()||!name.is_ascii()||name=="."||name==".."||name.starts_with(' ')||name.ends_with([' ','.']){
        return false;
    }
    if name.chars().any(|c|c.is_control()||matches!(c,'/'|'\\'|':')){return false;}
    let base=name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if matches!(base.as_str(),"CON"|"NUL"|"PRN"|"AUX"|"COM1"|"COM2"|"LPT1"|"LPT2"|"LPT3"){
        return false;
    }
    true
}
// ISO entries use host-native PathBuf separators when preparing output files.
// Normalize only lookup / manifest keys so nested files have the same keys on
// Windows (\u005c\u005c) and Linux (/). Keep PathBuf for actual filesystem operations.
fn disc_key(path:&Path)->String {
    path.to_string_lossy().replace('\\',"/")
}
fn collect(dir:&TreeNode, parent:&Path, all:&mut Vec<Asset>, seen:&mut BTreeSet<String>)->Result<(),String>{
    for child in &dir.children {
        if all.len()+seen.len()>MAX_ENTRIES{return Err("Too many files on CD".into());}
        if !safe_name(&child.name){return Err(format!("Unsafe name in disc image: {}",child.name));}
        let name=child.name.to_ascii_lowercase();
        let relative=parent.join(&name);
        let key=disc_key(&relative);
        if !seen.insert(key.clone()){return Err(format!("Ambiguous case-colliding disc path: {key}"));}
        if child.is_directory{collect(child,&relative,all,seen)?;continue;}
        if child.size>MAX_BYTES||child.file_length!=Some(child.size)||child.file_location.is_none(){
            return Err(format!("Malformed ISO extent for {key}"));
        }
        if matches!(name.as_str(),"glide.dll"|"sst1init.dll"|"fxmemmap.vxd")&&parent.as_os_str().is_empty(){
            continue; // obsolete 3dfx hardware drivers; nGlide is supplied separately
        }
        if key==MANIFEST.to_ascii_lowercase()||key=="cyber.exe.bak"||key=="cyber.exe.tmp" {
            return Err(format!("Reserved name in disc image: {key}"));
        }
        all.push(Asset{node:child.clone(),path:relative,size:child.size});
    }
    Ok(())
}
fn bounds(node:&TreeNode,len:u64,count:&mut usize,parent:&Path)->Result<(),String>{
    *count+=1;
    if *count>MAX_ENTRIES{return Err("ISO filesystem has too many entries".into());}
    if !safe_name(&node.name)&&node.name!="/" {
        return Err(format!("Unsafe ISO name: {}",node.name));
    }
    let path=parent.join(&node.name);
    if !node.is_directory {
        let off=node.file_location.ok_or_else(||format!("ISO file has no extent: {}",path.display()))?;
        let size=node.file_length.ok_or_else(||format!("ISO file has no size: {}",path.display()))?;
        if node.size!=size {
            return Err(format!("ISO file has inconsistent lengths: {} (size {}, extent {})",
                path.display(),node.size,size));
        }
        // An empty ISO9660 file has no data to read; some retail discs leave
        // an invalid/sentinel LBA in that directory record. The USA disc
        // contains AUTORUN/AUTORUN.INF (0 bytes, LBA 0x69696969).
        // Retain strict bounds checking for every file with actual data.
        if size>0 && off.checked_add(size).is_none_or(|end|end>len) {
            return Err(format!("ISO file points outside the disc image: {} (offset {}, size {}, image {})",
                path.display(),off,size,len));
        }
    }
    for child in &node.children{bounds(child,len,count,&path)?;}
    Ok(())
}
fn hash_on_disc(iso:&mut File,asset:&Asset)->Result<String,String>{
    let mut data=Vec::with_capacity(asset.size.min(1_000_000) as usize);
    cat_node(iso,&asset.node,&mut data).map_err(|e|format!("Cannot read disc executable: {e}"))?;
    if data.len() as u64!=asset.size{return Err("Incomplete image file".into());}
    Ok(patch::digest(&data))
}
fn copy_asset(iso:&mut File,asset:&Asset,dst:&Path)->Result<String,String>{
    if let Some(p)=dst.parent(){fs::create_dir_all(p).map_err(|e|e.to_string())?;}
    let mut out=OpenOptions::new().write(true).create_new(true).open(dst).map_err(|e|e.to_string())?;
    struct Hashed<'a>{inner:&'a mut File,digest:Sha256,n:u64}
    impl Write for Hashed<'_>{
        fn write(&mut self,b:&[u8])->std::io::Result<usize>{
            let n=self.inner.write(b)?;self.digest.update(&b[..n]);self.n+=n as u64;Ok(n)
        }
        fn flush(&mut self)->std::io::Result<()>{self.inner.flush()}
    }
    let mut writer=Hashed{inner:&mut out,digest:Sha256::new(),n:0};
    cat_node(iso,&asset.node,&mut writer).map_err(|e|format!("Cannot extract {}: {e}",asset.path.display()))?;
    if writer.n!=asset.size{return Err(format!("Truncated {} on CD",asset.path.display()));}
    let hash=format!("{:x}",writer.digest.finalize());
    out.flush().map_err(|e|e.to_string())?;
    Ok(hash)
}
/// Independent re-hash of all files in the staged installation BEFORE
/// publication. This is intentionally distinct from the hashes captured
/// while extracting the CD, and must never trust an unverified source path.
fn verify_staged_manifest(staged:&Path,manifest:&str)->Result<(),String>{
    use std::io::Read;
    if manifest.len()>4*1024*1024||!manifest.starts_with("# CyberGladiatorsFix manifest v1\n"){
        return Err("Invalid staged manifest".into());
    }
    let mut paths=BTreeSet::new();
    let mut count=0usize;
    for line in manifest.lines().skip(1){
        let fields:Vec<_>=line.split('\t').collect();
        if fields.len()!=3{return Err("Malformed staged manifest row".into());}
        let name=fields[0];
        if name.is_empty()||name.contains(['\\',':','\r','\n'])
            ||!paths.insert(name.to_ascii_lowercase()){
            return Err(format!("Unsafe or duplicate staged manifest entry {name}"));
        }
        let size=fields[1].parse::<u64>().map_err(|e|e.to_string())?;
        if fields[2].len()!=64||!fields[2].bytes().all(|b|b.is_ascii_hexdigit()){
            return Err("Invalid staged manifest digest".into());
        }
        let mut path=staged.to_path_buf();
        for item in Path::new(name).components(){
            let std::path::Component::Normal(part)=item else{return Err("Unsafe staged relative path".into())};
            path.push(part);
            let m=fs::symlink_metadata(&path).map_err(|e|e.to_string())?;
            if m.file_type().is_symlink(){return Err("Staged manifest contains symlink".into());}
        }
        let m=fs::symlink_metadata(&path).map_err(|e|e.to_string())?;
        if !m.is_file()||m.len()!=size {
            return Err(format!("Staged file size mismatch: {name}"));
        }
        let mut input=File::open(&path).map_err(|e|e.to_string())?;
        let mut digest=Sha256::new();
        let mut buffer=[0u8;65536];
        loop {
            let n=input.read(&mut buffer).map_err(|e|e.to_string())?;
            if n==0{break;}digest.update(&buffer[..n]);
        }
        if format!("{:x}",digest.finalize())!=fields[2]{
            return Err(format!("Staged file SHA-256 mismatch: {name}"));
        }
        count+=1;
    }
    if count==0{return Err("Empty staged game manifest".into());}
    Ok(())
}
fn image_file(path:&Path)->Result<File,String>{
    let m=fs::symlink_metadata(path).map_err(|e|e.to_string())?;
    if !m.is_file()||m.file_type().is_symlink(){return Err("Image must be a regular file".into());}
    File::open(path).map_err(|e|e.to_string())
}

/// Stream a selected original ISO asset into a SHA-256 hasher without
/// extracting it or materializing the asset in memory.
fn inspect_hash(iso:&mut File,asset:&Asset)->Result<String,String>{
    struct DigestOnly{sha:Sha256,bytes:u64}
    impl Write for DigestOnly{
        fn write(&mut self,buf:&[u8])->std::io::Result<usize>{
            self.sha.update(buf);
            self.bytes+=buf.len() as u64;
            Ok(buf.len())
        }
        fn flush(&mut self)->std::io::Result<()>{Ok(())}
    }
    let mut sink=DigestOnly{sha:Sha256::new(),bytes:0};
    cat_node(iso,&asset.node,&mut sink)
        .map_err(|e|format!("Cannot hash original CD asset {}: {e}",asset.path.display()))?;
    if sink.bytes!=asset.size{
        return Err(format!("Truncated original CD asset {}",asset.path.display()));
    }
    Ok(format!("{:x}",sink.sha.finalize()))
}

/// Read-only edition inventory: never installs, patches or writes any retail
/// asset. The CUE parser creates one temporary normalized ISO view, which its
/// Drop guard removes. Results go exclusively to the unified launcher log.
///
/// Read-only inspection. The separate install function authorizes edition
/// patches only after verifying the exact original EXE and resource hashes.
pub fn inspect_disc(cue_path:&Path,root:&Path)->Result<(),String>{
    if !cue_path.extension().is_some_and(|s|s.eq_ignore_ascii_case("cue")){
        return Err("--inspect-disc requires a .cue sheet, not a BIN or ISO".into());
    }
    let tracks=cue::tracks(cue_path)?;
    let first=tracks.first().ok_or("CUE has no data track")?;
    let audio=tracks.iter().filter(|t|t.mode=="AUDIO").count();
    let track_files: BTreeSet<&str>=tracks.iter().map(|t|t.file.as_str()).collect();
    crate::show(&format!(
        "DISC_INSPECT CUE={} tracks={} audio_tracks={} bin_files={} first_mode={} first_index_sector={} read_only=1",
        cue_path.file_name().unwrap_or_default().to_string_lossy(),
        tracks.len(),audio,track_files.len(),first.mode,first.index01
    ),root);
    for track in &tracks{
        crate::show(&format!(
            "DISC_TRACK number={:02} mode={} index00_sector={} index01_sector={} bin={}",
            track.number,track.mode,
            track.index00.map_or_else(||"none".into(),|v|v.to_string()),
            track.index01,track.file
        ),root);
    }
    let iso_guard=cue::normalize(cue_path,root)?;
    let iso_path=iso_guard.0.as_path();
    let mut iso=image_file(iso_path)?;
    let len=iso.metadata().map_err(|e|e.to_string())?.len();
    if len==0||len>MAX_BYTES{return Err("Unsupported normalized disc data size".into());}
    let tree=detect_and_parse_filesystem(&mut iso,&iso_path.to_string_lossy())
        .map_err(|e|format!("Cannot inspect original data filesystem: {e}"))?;
    let mut count=0;
    for child in &tree.children{bounds(child,len,&mut count,Path::new(""))?;}
    let names=tree.children.iter().map(|n|n.name.as_str()).take(32).collect::<Vec<_>>();
    crate::show(&format!(
        "DISC_ISO normalized_bytes={len} entries={count} root_names={names:?}"
    ),root);
    let matches:Vec<_>=tree.children.iter()
        .filter(|n|n.is_directory&&n.name.eq_ignore_ascii_case("CYBER")).collect();
    let folder=match matches.as_slice(){
        [directory]=>directory,
        _=>return Err("French CD discovery requires one CYBER directory; see DISC_ISO root_names".into())
    };
    let mut seen=BTreeSet::new();
    let mut assets=Vec::new();
    collect(folder,Path::new(""),&mut assets,&mut seen)?;
    let files:BTreeMap<String,Asset>=assets.into_iter()
        .map(|a|(disc_key(&a.path),a)).collect();
    let bytes: u64=files.values().map(|a|a.size).sum();
    crate::show(&format!(
        "DISC_ASSETS files={} bytes={} all_filenames_checked=1",
        files.len(),bytes
    ),root);
    for name in ["cyber.exe","graphics/shell.fnt","splash.avi","splash4.avi"]{
        if let Some(asset)=files.get(name){
            let sha=inspect_hash(&mut iso,asset)?;
            crate::show(&format!(
                "DISC_ASSET path={name} size={} sha256={sha}",
                asset.size
            ),root);
            if name=="cyber.exe"{
                let edition=crate::edition::from_original_exe(asset.size,&sha);
                crate::show(&format!(
                    "DISC_EDITION original_exe={} install_supported={} fr_patch_enabled={}",
                    edition.label(),u8::from(edition.is_install_supported()),
                    u8::from(matches!(edition,crate::edition::OriginalEdition::FrenchRetail))
                ),root);
            }
        }else{
            crate::show(&format!("DISC_ASSET path={name} missing=1"),root);
            if name=="cyber.exe"{
                return Err("Original CD does not contain CYBER/CYBER.EXE".into());
            }
        }
    }
    crate::show(
        "DISC_INSPECT_COMPLETE disc_read_only=1 installed_game_unchanged=1 fr_runtime_field_validation_pending=1 normalized_iso_auto_removed=1",
        root
    );
    Ok(())
}

/// Install only CYBER/*, not Sierra's obsolete Windows 95 installer.
/// The installed-game path is verified and launched without requiring the
/// original disc image; the bundled WinMM bridge serves CD audio as FLAC.
pub fn install(cue_path:&Path,destination:&Path,root:&Path)->Result<String,String>{
    if destination.exists()||destination.is_symlink(){return Err(format!("Destination already exists: {}",destination.display()));}
    if !cue_path.extension().is_some_and(|s|s.eq_ignore_ascii_case("cue")) {
        return Err("Only a .cue sheet referencing one or more .bin files is supported".into());
    }
    // Normalize the first raw data track to an internal, short-lived
    // 2048-byte/sector filesystem view. Standalone ISO is not accepted.
    let temp_iso=cue::normalize(cue_path,root)?;
    let iso_path=temp_iso.0.as_path();
    let mut iso=image_file(iso_path)?;
    let len=iso.metadata().map_err(|e|e.to_string())?.len();
    if len==0||len>MAX_BYTES{return Err("Unsupported ISO file size".into());}
    let tree=detect_and_parse_filesystem(&mut iso,&iso_path.to_string_lossy())
        .map_err(|e|format!("Unsupported ISO/UDF image: {e}"))?;
    let mut count=0;
    for child in &tree.children{bounds(child,len,&mut count,Path::new(""))?;}
    let matches:Vec<_>=tree.children.iter().filter(|n|n.is_directory&&n.name.eq_ignore_ascii_case("CYBER")).collect();
    let folder=match matches.as_slice(){[dir]=>dir,_=>return Err("Disc not recognized: expected one CYBER directory".into())};
    let mut seen=BTreeSet::new();
    let mut assets=Vec::new();
    collect(folder,Path::new(""),&mut assets,&mut seen)?;
    let files:BTreeMap<String,Asset>=assets.into_iter().map(|a|(disc_key(&a.path),a)).collect();
    let exe=files.get("cyber.exe").ok_or("Disc not recognized: CYBER/CYBER.EXE missing")?;
    // First verify the complete original EXE hash, then edition-specific
    // immutable disc resources. Neither a translated string nor a filename
    // alone is sufficient for patch authorization.
    let hash=hash_on_disc(&mut iso,exe)?;
    let (edition_name,checks):(&str,&[(&str,&str)])=
        match crate::edition::from_original_exe(exe.size,&hash){
            crate::edition::OriginalEdition::UsaRetail=>(USA_EDITION,&[
                ("graphics/shell.fnt",USA_SHELL_FNT_SHA256),
                ("splash.avi",USA_SPLASH_AVI_SHA256),
            ]),
            crate::edition::OriginalEdition::FrenchRetail=>(FR_EDITION,&[
                ("graphics/shell.fnt",FR_SHELL_FNT_SHA256),
                ("splash.avi",USA_SPLASH_AVI_SHA256),
                ("splash4.avi",FR_SPLASH4_AVI_SHA256),
            ]),
            crate::edition::OriginalEdition::Unknown=>{
                return Err(format!(
                    "Unsupported CD edition: CYBER.EXE size={} SHA-256 {hash}; accepted only after exact US/FR retail identification",
                    exe.size
                ));
            }
        };
    for &(name,expected) in checks{
        let asset=files.get(name).ok_or_else(||format!("Unsupported {edition_name} CD: missing CYBER/{name}"))?;
        let actual=hash_on_disc(&mut iso,asset)?;
        if actual!=expected{
            return Err(format!("Unsupported {edition_name} CD: CYBER/{name} SHA-256 {actual} (expected {expected})"));
        }
    }
    let staged=destination.with_extension("staging");
    if staged.exists()||staged.is_symlink(){return Err(format!("Staging destination already exists: {}",staged.display()));}
    fs::create_dir(&staged).map_err(|e|e.to_string())?;
    let operation=(||->Result<String,String>{
        let mut manifest=String::from("# CyberGladiatorsFix manifest v1\n");
        for (i,(path,asset)) in files.iter().enumerate(){
            let output=staged.join(&asset.path);
            let hash=copy_asset(&mut iso,asset,&output)?;
            if path!="cyber.exe" {manifest.push_str(&format!("{path}\t{}\t{hash}\n",asset.size));}
            if (i+1)%50==0 {println!("Extracted {}/{} files",i+1,files.len());}
        }
        // Generate [Video] preferences before patching. CDDA and strict IQ
        // integrity are always-on internal behavior, not user settings.
        // Existing INIs are only migrated to remove obsolete sections.
        settings::ensure(&staged)?;
        // WinMM is always proxied to keep fullscreen AVI, the DXVK repair
        // and always-on FLAC playback in one x86 DLL. The child process
        // environment enables CDDA music without a user INI option.
        cdda::try_install(root,&staged)?;
        let exe=staged.join("cyber.exe");
        let outcome=patch::apply(&exe)?;
        patch::set_cdda(&exe,true)?;
        patch::set_skip_intro(&exe,settings::read(&staged)?)?;
        // Provide graphics preferences on the first boot. The original CD
        // does not contain CYBER.INI; preserve the valid 32-byte layout.
        graphics::create_defaults(&staged)?;
        let audio_report=audio::extract(cue_path,&staged)?;
        manifest.push_str(&audio_report.manifest_rows);
        fs::write(staged.join(MANIFEST),&manifest).map_err(|e|e.to_string())?;
        verify_staged_manifest(&staged,&manifest)?;
        patch::verify_exe(&staged.join("cyber.exe"))?;
        cdda::verify(&staged)?;
        // No incomplete or corrupted game installation can be published.
        // GameFrame is embedded and activated by autostart at first launch.
        // Do not copy proprietary nGlide into a fresh open-source install.
        fs::rename(&staged,destination).map_err(|e|format!("Cannot publish installation: {e}"))?;
        let audio_note=format!("{} lossless FLAC tracks saved in music/TrackNN.flac; {} tracks omitted (missing BIN files)",
            audio_report.extracted,audio_report.missing.len());
        Ok(format!("Detected edition: {edition_name}. {} game files extracted, {}, default CYBER.INI created (640x480, detail 3/3), CyberGladiatorsFix.ini created (SkipIntro=0 only; CDDA always enabled), native 4:3 AVI and embedded WinMM bridge selected. Renderer: bundled open-source GameFrame staged at first launch. Audio: {}. Installed to {}.",
            files.len(),outcome,audio_note,destination.display()))
    })();
    if operation.is_err(){let _=fs::remove_dir_all(&staged);}
    operation
}

#[cfg(test)]
mod iq_staging_policy_tests {
    use super::*;
    #[test]
    fn first_install_still_hashes_every_original_iq_file(){
        let dir=std::env::temp_dir().join(format!("CyberIQStaging-{}",
            std::process::id()));
        let _=fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("iq")).unwrap();
        let iq=dir.join("iq").join("iq40.dat");
        let original=b"original";
        fs::write(&iq,original).unwrap();
        let manifest=format!("# CyberGladiatorsFix manifest v1\\niq/iq40.dat\\t{}\\t{}\\n",
            original.len(),crate::patch::digest(original));
        assert!(verify_staged_manifest(&dir,&manifest).is_ok());
        fs::write(&iq,b"modified").unwrap();
        assert!(verify_staged_manifest(&dir,&manifest).unwrap_err()
            .contains("Staged file SHA-256 mismatch: iq/iq40.dat"));
        fs::remove_file(&iq).unwrap();
        assert!(verify_staged_manifest(&dir,&manifest).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }
}

#[cfg(test)]
mod disc_key_tests {
    use super::*;
    #[test]
    fn nested_reference_asset_uses_forward_slashes() {
        let path=Path::new("graphics").join("shell.fnt");
        assert_eq!(disc_key(&path),"graphics/shell.fnt");
        let files=BTreeMap::from([(disc_key(&path),42usize)]);
        assert_eq!(files.get("graphics/shell.fnt"),Some(&42));
    }
    #[test]
    fn deeply_nested_assets_use_forward_slashes() {
        let path=Path::new("fighters").join("plasma").join("head.dat");
        assert_eq!(disc_key(&path),"fighters/plasma/head.dat");
        assert_eq!(disc_key(Path::new("cyber.exe")),"cyber.exe");
    }
    #[test]
    fn backslashes_in_windows_style_keys_are_normalized() {
        assert_eq!(disc_key(Path::new(r"graphics\shell.fnt")),"graphics/shell.fnt");
    }
}
#[cfg(test)]
mod cue_selection_tests {
    use super::*;
    #[test]
    fn autodetect_ignores_iso_and_bin_but_accepts_cue() {
        let root=std::env::temp_dir().join(format!("cyber-cue-only-{}",std::process::id()));
        let _=fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        fs::write(root.join("disc.iso"),b"not accepted").unwrap();
        fs::write(root.join("disc.bin"),b"not accepted").unwrap();
        assert!(find_cue(&root).unwrap_err().contains("one .cue"));
        fs::write(root.join("disc.cue"),br#"FILE "disc.bin" BINARY"#).unwrap();
        assert_eq!(find_cue(&root).unwrap(),root.join("disc.cue"));
        fs::remove_dir_all(&root).unwrap();
    }
    #[test]
    fn explicit_iso_is_rejected_before_extraction() {
        let root=std::env::temp_dir().join(format!("cyber-iso-refused-{}",std::process::id()));
        let _=fs::remove_dir_all(&root);fs::create_dir(&root).unwrap();
        let result=install(&root.join("disc.iso"),&root.join("installed"),&root);
        assert!(result.unwrap_err().contains("Only a .cue"));
        fs::remove_dir_all(&root).unwrap();
    }
}
#[cfg(test)]
mod bounds_tests {
    use super::*;

    #[test]
    fn empty_file_with_garbage_lba_is_valid() {
        // Seen on the original USA CD: AUTORUN/AUTORUN.INF is 0 bytes
        // but its extent points beyond the end of the image.
        let mut autorun=TreeNode::new_directory("AUTORUN".into());
        autorun.add_child(TreeNode::new_file_with_location(
            "AUTORUN.INF".into(),0,0x69696969,0));
        let mut entries=0;
        assert!(bounds(&autorun,56_640*2_048,&mut entries,Path::new("")).is_ok());
        assert_eq!(entries,2);
    }

    #[test]
    fn nonempty_file_out_of_bounds_is_rejected_with_full_path() {
        let mut cyber=TreeNode::new_directory("CYBER".into());
        let mut graphics=TreeNode::new_directory("GRAPHICS".into());
        graphics.add_child(TreeNode::new_file_with_location(
            "SHELL.FNT".into(),4,0x69696969,4));
        cyber.add_child(graphics);
        let mut entries=0;
        let err=bounds(&cyber,56_640*2_048,&mut entries,Path::new(""))
            .expect_err("Nonempty out-of-range extent must be rejected");
        assert!(err.contains("CYBER\\GRAPHICS\\SHELL.FNT") ||
                err.contains("CYBER/GRAPHICS/SHELL.FNT"));
        assert!(err.contains("outside the disc image"));
    }

    #[test]
    fn nonempty_file_at_last_byte_is_accepted() {
        let file=TreeNode::new_file_with_location("DATA.BIN".into(),4,12,4);
        let mut entries=0;
        assert!(bounds(&file,16,&mut entries,Path::new("")).is_ok());
        let mut entries=0;
        assert!(bounds(&file,15,&mut entries,Path::new("")).is_err());
    }

    #[test]
    fn inconsistent_node_size_is_rejected_even_if_empty() {
        let file=TreeNode::new_file_with_location("MISMATCH.BIN".into(),1,0x69696969,0);
        let mut entries=0;
        assert!(bounds(&file,56_640*2_048,&mut entries,Path::new("")).is_err());
    }
}
