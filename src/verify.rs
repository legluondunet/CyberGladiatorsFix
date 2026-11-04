// SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only checks. Asset hashes are recorded during extraction and are
//! independently recomputed before launch, including the original backup.
use crate::{install::{DIR,MANIFEST},cdda,nglide,patch,settings};
use sha2::{Digest,Sha256};
use std::fs::{self,File};
use std::io::{Read,Write};
use std::path::{Path,Component};
use std::process::{Child,Command,Stdio};
fn file_hash(path:&Path)->Result<(u64,String),String>{
    let m=fs::symlink_metadata(path).map_err(|e|format!("Missing {}: {e}",path.display()))?;
    if !m.is_file()||m.file_type().is_symlink(){return Err(format!("Not a regular file: {}",path.display()));}
    let mut f=File::open(path).map_err(|e|e.to_string())?;
    let mut digest=Sha256::new();
    let mut n=0u64;
    let mut b=[0u8;64*1024];
    loop {let got=f.read(&mut b).map_err(|e|e.to_string())?;
        if got==0 {break;}digest.update(&b[..got]);n+=got as u64;
    }
    Ok((n,format!("{:x}",digest.finalize())))
}
fn safe_manifest_path(s:&str)->bool{
    let p=Path::new(s);
    if s.is_empty()||s.contains(['\t','\n','\r',':','\\'])||p.is_absolute(){return false;}
    p.components().all(|c|matches!(c,Component::Normal(_)))
}
pub fn verify(root:&Path)->Result<String,String>{
    let game=root.join(DIR);
    let meta=fs::symlink_metadata(&game).map_err(|e|format!("Missing game directory: {e}"))?;
    if !meta.is_dir()||meta.file_type().is_symlink(){return Err("Game folder is not a regular directory".into());}
    patch::verify_exe(&game.join("cyber.exe"))?;
    let manifest=fs::read_to_string(game.join(MANIFEST)).map_err(|e|format!("Manifest missing: {e}"))?;
    if manifest.len()>4*1024*1024||!manifest.starts_with("# CyberGladiatorsFix manifest v1\n"){
        return Err("Missing or invalid extraction manifest".into());
    }
    let mut count=0usize;let mut bytes=0u64;
    let mut seen=std::collections::BTreeSet::<String>::new();
    for line in manifest.lines().skip(1){
        let mut parts=line.split('\t');
        let name=parts.next().ok_or("Empty manifest row")?;
        let size=parts.next().ok_or("Missing manifest size")?.parse::<u64>().map_err(|e|e.to_string())?;
        let hash=parts.next().ok_or("Missing manifest SHA-256")?;
        if parts.next().is_some()||hash.len()!=64||!hash.bytes().all(|c|c.is_ascii_hexdigit())||
            !safe_manifest_path(name)||!seen.insert(name.to_ascii_lowercase()) {
            return Err(format!("Invalid manifest entry: {name}"));
        }
        let mut path=game.clone();
        // A local manifest must never cause verification to traverse a
        // symlinked directory outside the installation.
        for component in Path::new(name).components(){
            let Component::Normal(part)=component else{return Err(format!("Unsafe manifest path: {name}"))};
            path.push(part);
            let meta=fs::symlink_metadata(&path).map_err(|e|format!("Missing manifest path {}: {e}",path.display()))?;
            if meta.file_type().is_symlink(){return Err(format!("Symlink in manifest path: {}",path.display()));}
        }
        let (actual_size,actual_hash)=file_hash(&path)?;
        if actual_size!=size||actual_hash!=hash {return Err(format!("Game asset has changed: {name}"));}
        count+=1;bytes+=size;
    }
    nglide::verify(&game)?;
    let audio=if patch::cdda_active(&game.join("cyber.exe"))? {
        cdda::verify(&game)?;
        "CDDA bridge enabled and verified"
    }else{"native WinMM (CDDA bypass)"};
    Ok(format!("Verified CYBER.EXE patch + backup, {count} CD resources ({bytes} bytes), nGlide and {audio}"))
}
pub fn launch(root:&Path,logo:bool,trace:bool,passthru_noop:bool)->Result<Child,String>{
    if passthru_noop && (!trace || logo) {
        return Err("--passthru-noop is a trace-only diagnostic experiment".into());
    }
    let game=root.join(DIR);
    // For old installations, generate the absent *patcher* INI with
    // SkipIntro=0. Never touch the game's binary cyber.ini.
    settings::ensure(&game)?;
    let skip=settings::read(&game)?;
    let enable_cdda=settings::read_cdda(&game)?;
    let exe=game.join("cyber.exe");
    patch::verify_exe(&exe)?;
    // If the user explicitly opts into CDDA, copy/upgrade the project's
    // verified DLL first. Never switch the import descriptor to an absent DLL.
    if enable_cdda {cdda::try_install(root,&game)?;}
    // Prepare the exact import and intro variants *before* verification and
    // launch; disabled CDDA must not load or require the 32-bit DLL.
    patch::set_cdda(&exe,enable_cdda)?;
    patch::set_skip_intro(&exe,skip)?;
    verify(root)?;
    if logo{nglide::check_logo(&game)?;}
    let mut cmd=Command::new(game.join("cyber.exe"));
    cmd.current_dir(&game);
    if trace {
        // Diagnoses graphics/exception handling without modifying game bytes
        // or the Wine prefix. The trace is opt-in, stored beside the patcher,
        // and replaced by each new --launch --trace invocation.
        nglide::configure_diagnostic(&mut cmd,logo);
        cmd.env("WINEDEBUG","+seh,+loaddll");
        // Only the debug launch observes the native 3dfx-to-VGA transition.
        // The x86 CDDA DLL refuses unknown game/Glide pointers and leaves
        // all game files and normal --launch behavior untouched.
        cmd.env("CYBER_GLIDE_TRACE","1");
        if passthru_noop {
            // Explicit A/B experiment: the 32-bit bridge suppresses only
            // grSstPassthruMode(1), in memory, after the movie has closed.
            // The option is never enabled on ordinary --launch.
            cmd.env("CYBER_GLIDE_PASSTHRU_NOOP","1");
        }
        let log=root.join("CyberGladiatorsFix-WineTrace.log");
        if log.is_symlink(){return Err("Refusing symlink Wine trace log".into());}
        let mut output=fs::OpenOptions::new().create(true).write(true).truncate(true)
            .open(&log).map_err(|e|format!("Cannot create Wine trace log: {e}"))?;
        writeln!(output,"CyberGladiatorsFix --launch --trace")
            .and_then(|_|writeln!(output,"WINEDEBUG=+seh,+loaddll; NGLIDE_BACKEND=0 (auto); CYBER_GLIDE_TRACE=1; CYBER_GLIDE_PASSTHRU_NOOP={}; CDDA enabled={enable_cdda}; SkipIntro={skip}",if passthru_noop{1}else{0}))
            .and_then(|_|output.flush())
            .map_err(|e|format!("Cannot initialize Wine trace: {e}"))?;
        let errors=output.try_clone().map_err(|e|format!("Cannot capture Wine stderr: {e}"))?;
        cmd.stdout(Stdio::from(output));
        cmd.stderr(Stdio::from(errors));
    }else{
        nglide::configure(&mut cmd,logo);
    }
    cmd.spawn().map_err(|e|format!("Unable to start CYBER.EXE: {e}"))
}
