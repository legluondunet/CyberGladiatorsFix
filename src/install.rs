// SPDX-License-Identifier: GPL-3.0-or-later
//! Minimal, conservative CD image installation built on ScorchedPlanetFix's
//! ISO parser (isomage) and strict known-executable fingerprint design.
use crate::{audio,cue,nglide,patch};
use isomage::{cat_node,detect_and_parse_filesystem,TreeNode};
use sha2::{Digest,Sha256};
use std::collections::{BTreeSet, BTreeMap};
use std::fs::{self,File,OpenOptions};
use std::io::{Read,Write};
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

#[derive(Clone)]
struct Asset{node:TreeNode,path:PathBuf,size:u64}

pub fn find_image(cwd:&Path)->Result<PathBuf,String>{
    let mut matches=Vec::<PathBuf>::new();
    for entry in fs::read_dir(cwd).map_err(|e|e.to_string())? {
        let p=entry.map_err(|e|e.to_string())?.path();
        let m=fs::symlink_metadata(&p).map_err(|e|e.to_string())?;
        if m.is_file()&&!m.file_type().is_symlink()&&p.extension().is_some_and(|s|s.eq_ignore_ascii_case("iso")||s.eq_ignore_ascii_case("cue")){
            matches.push(p);
        }
    }
    if matches.len()!=1{return Err(format!("Expected exactly one ISO/CUE in {}, found {}. Supply an explicit path.",cwd.display(),matches.len()));}
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
fn collect(dir:&TreeNode, parent:&Path, all:&mut Vec<Asset>, seen:&mut BTreeSet<String>)->Result<(),String>{
    for child in &dir.children {
        if all.len()+seen.len()>MAX_ENTRIES{return Err("Too many files on CD".into());}
        if !safe_name(&child.name){return Err(format!("Unsafe name in disc image: {}",child.name));}
        let name=child.name.to_ascii_lowercase();
        let relative=parent.join(&name);
        let key=relative.to_string_lossy().replace('\\',"/");
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
fn bounds(node:&TreeNode,len:u64,count:&mut usize)->Result<(),String>{
    *count+=1;
    if *count>MAX_ENTRIES{return Err("ISO filesystem has too many entries".into());}
    if !safe_name(&node.name)&&node.name!="/" {return Err(format!("Unsafe ISO name: {}",node.name));}
    if !node.is_directory{
        let off=node.file_location.ok_or("ISO file has no extent")?;
        let size=node.file_length.ok_or("ISO file has no size")?;
        if off.checked_add(size).is_none_or(|end|end>len)||node.size!=size {
            return Err("ISO file points outside the disc image".into());
        }
    }
    for child in &node.children{bounds(child,len,count)?;}
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
fn image_file(path:&Path)->Result<File,String>{
    let m=fs::symlink_metadata(path).map_err(|e|e.to_string())?;
    if !m.is_file()||m.file_type().is_symlink(){return Err("Image must be a regular file".into());}
    File::open(path).map_err(|e|e.to_string())
}

/// Install only CYBER/*, not Sierra's obsolete Windows 95 installer.
/// A CD may still be required at runtime until the game's CD logic is analyzed.
pub fn install(image:&Path,bin:Option<&Path>,destination:&Path,root:&Path)->Result<String,String>{
    if destination.exists()||destination.is_symlink(){return Err(format!("Destination already exists: {}",destination.display()));}
    let ext=image.extension().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
    let temp_iso=if ext=="cue"{Some(cue::normalize(image,bin,root)?)}
        else if ext=="iso"&&bin.is_none(){None}
        else{return Err("Expected ISO or CUE [BIN]".into());};
    let iso_path=temp_iso.as_ref().map(|p|p.0.as_path()).unwrap_or(image);
    let mut iso=image_file(iso_path)?;
    let len=iso.metadata().map_err(|e|e.to_string())?.len();
    if len==0||len>MAX_BYTES{return Err("Unsupported ISO file size".into());}
    let tree=detect_and_parse_filesystem(&mut iso,&iso_path.to_string_lossy())
        .map_err(|e|format!("Unsupported ISO/UDF image: {e}"))?;
    let mut count=0;
    for child in &tree.children{bounds(child,len,&mut count)?;}
    let matches:Vec<_>=tree.children.iter().filter(|n|n.is_directory&&n.name.eq_ignore_ascii_case("CYBER")).collect();
    let folder=match matches.as_slice(){[dir]=>dir,_=>return Err("Disc not recognized: expected one CYBER directory".into())};
    let mut seen=BTreeSet::new();
    let mut assets=Vec::new();
    collect(folder,Path::new(""),&mut assets,&mut seen)?;
    let files:BTreeMap<String,Asset>=assets.into_iter().map(|a|(a.path.to_string_lossy().to_string(),a)).collect();
    let exe=files.get("cyber.exe").ok_or("Disc not recognized: CYBER/CYBER.EXE missing")?;
    if exe.size != patch::LEN as u64 {return Err(format!("Unsupported CYBER.EXE size: {}",exe.size));}
    let hash=hash_on_disc(&mut iso,exe)?;
    if hash!=patch::ORIGINAL {
        return Err(format!("Unsupported CD edition: CYBER.EXE SHA-256 {hash}; expected {}",patch::ORIGINAL));
    }
    for (name,expected) in [
        ("graphics/shell.fnt",USA_SHELL_FNT_SHA256),
        ("splash.avi",USA_SPLASH_AVI_SHA256),
    ]{
        let asset=files.get(name).ok_or_else(||format!("Unsupported USA CD: missing CYBER/{name}"))?;
        let actual=hash_on_disc(&mut iso,asset)?;
        if actual!=expected{
            return Err(format!("Unsupported USA CD: CYBER/{name} SHA-256 {actual} (expected {expected})"));
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
        let outcome=patch::apply(&staged.join("cyber.exe"))?;
        let audio_report=if ext=="cue"{Some(audio::extract(image,&staged)?)}else{None};
        if let Some(ref audio)=audio_report{manifest.push_str(&audio.manifest_rows);}
        fs::write(staged.join(MANIFEST),manifest).map_err(|e|e.to_string())?;
        let has_glide=nglide::try_install(root,&staged)?;
        fs::rename(&staged,destination).map_err(|e|format!("Cannot publish installation: {e}"))?;
        let audio_note=audio_report.map(|r|format!("{} lossless FLAC tracks saved as TrackNN.flac; {} tracks omitted (missing BIN files)",r.extracted,r.missing.len()))
            .unwrap_or_else(||"No CUE sheet provided; audio tracks cannot be extracted from this ISO".into());
        Ok(format!("Detected edition: {USA_EDITION}. {} game files extracted, {}, nGlide {}. Audio: {}. Installed to {}.",
            files.len(),outcome,if has_glide{"installed"}else{"missing (install separately)"},audio_note,destination.display()))
    })();
    if operation.is_err(){let _=fs::remove_dir_all(&staged);}
    operation
}
