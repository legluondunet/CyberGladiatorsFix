// SPDX-License-Identifier: GPL-3.0-or-later
//! Opt-in, process-isolated startup AVI playback. The helper exits before
//! CYBER.EXE creates its Glide/DXVK device, so the two renderers never share
//! the original game's DirectDraw or window state.
use crate::{bundle,cdda,diagnostics,install::DIR,settings,verify};
use std::fs::{self,File,OpenOptions};
use std::io::Read;
use std::path::Path;
use std::process::{Command,Stdio};
use std::thread;
use std::time::{Duration,Instant};

const PLAYER:&str="CyberGladiatorsIntro.exe";

fn validate_x86_exe(path:&Path)->Result<(),String>{
    let meta=fs::symlink_metadata(path).map_err(|e|format!("{PLAYER} missing: {e}"))?;
    if !meta.is_file()||meta.file_type().is_symlink()||!(4096..=16*1024*1024).contains(&meta.len()){
        return Err(format!("{PLAYER} must be a regular Windows x86 EXE (4 KiB..16 MiB), not a symlink"));
    }
    let mut bytes=[0u8;1024];
    File::open(path).and_then(|mut f|f.read_exact(&mut bytes))
        .map_err(|e|format!("Cannot read {PLAYER}: {e}"))?;
    if &bytes[..2]!=b"MZ"{return Err(format!("{PLAYER} has no MZ header"));}
    let pe=u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as usize;
    if pe.checked_add(26).is_none_or(|end|end>bytes.len())
        || &bytes[pe..pe+4]!=b"PE\0\0"
        || u16::from_le_bytes(bytes[pe+4..pe+6].try_into().unwrap())!=0x14c
        || u16::from_le_bytes(bytes[pe+22..pe+24].try_into().unwrap())&0x2000!=0
        || u16::from_le_bytes(bytes[pe+24..pe+26].try_into().unwrap())!=0x10b {
        return Err(format!("{PLAYER} must be a Windows x86 PE32 executable"));
    }
    Ok(())
}
/// A successful helper exit only demonstrates that the platform MCI path
/// reported completion. The user must still confirm that the AVI appeared.
pub fn play_before_game(root:&Path)->Result<(),String>{
    let game=root.join(DIR);
    settings::ensure(&game)?;
    if settings::read(&game)?{
        return Err("External intro requires [Video] SkipIntro=0 (the option plays it out-of-process and skips it only inside the game)".into());
    }
    // Fail fast on a runner configuration that cannot use the confirmed
    // D3D9/DXVK handoff, instead of forcing a 40-second unplayable prelude.
    if std::env::var("PROTON_USE_WINED3D").as_deref()==Ok("1"){
        return Err("Cannot use the isolated D3D9/DXVK intro handoff with PROTON_USE_WINED3D=1. Remove this in Lutris and restart Wine/Proton before launching".into());
    }
    // Installing/upgrading the optional embedded CDDA bridge before the
    // movie ensures a missing DLL is handled without playing 40 seconds
    // first, and without modifying the original CD game assets.
    if settings::read_cdda(&game)?{cdda::try_install(root,&game)?;}
    // Honor the same strict-by-default CD receipt and explicit IQ exception
    // as the ordinary launcher, before showing any video.
    verify::verify_launch_assets(root)?;
    // The x86 helper is embedded in the one-file release; stage it in a
    // uniquely named private temp directory and delete after child exit.
    let staged=bundle::TemporaryIntro::create()?;
    let helper=staged.path();
    validate_x86_exe(helper)?;
    // Append the helper's MCI diagnostics into the ONE session log beside
    // the patcher. Never truncate after the launcher wrote its config header.
    let log_path=diagnostics::location(root);
    let meta=fs::symlink_metadata(&log_path).map_err(|e|format!("Session log missing: {e}"))?;
    if !meta.is_file()||meta.file_type().is_symlink(){
        return Err("Refusing non-regular or symlink session log".into());
    }
    let output=OpenOptions::new().append(true).open(&log_path)
        .map_err(|e|format!("Cannot append intro trace: {e}"))?;
    let error_output=output.try_clone().map_err(|e|e.to_string())?;
    let mut child=Command::new(&helper)
        .current_dir(&game)
        .stdin(Stdio::null())
        .stdout(Stdio::from(output))
        .stderr(Stdio::from(error_output))
        .spawn().map_err(|e|format!("Cannot start isolated intro player: {e}"))?;
    crate::show(&format!("INTRO_EXTERNAL started PID={} helper={} log={}",child.id(),helper.display(),log_path.display()),root);
    let start=Instant::now();
    loop{
        match child.try_wait().map_err(|e|format!("Cannot observe external AVI player: {e}"))?{
            Some(status)=>{
                crate::show(&format!("INTRO_EXTERNAL exit_status={status} elapsed_ms={}",start.elapsed().as_millis()),root);
                if !status.success(){
                    return Err(format!("External AVI playback failed ({status}); see {}. Game not started, no game asset modified",log_path.display()));
                }
                return Ok(());
            }
            None if start.elapsed()>Duration::from_secs(100)=>{
                let _=child.kill();
                let _=child.wait();
                return Err(format!("External AVI playback timed out after 100 seconds; see {}. Game not started",log_path.display()));
            }
            None=>thread::sleep(Duration::from_millis(100)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_helper_is_rejected_without_spawn(){
        let candidate=std::env::temp_dir().join(format!("missing-cyber-intro-{}.exe",std::process::id()));
        assert!(validate_x86_exe(&candidate).is_err());
    }
    #[test]
    fn non_pe_helper_is_rejected_without_spawn(){
        let candidate=std::env::temp_dir().join(format!("invalid-cyber-intro-{}.exe",std::process::id()));
        let _=fs::remove_file(&candidate);
        fs::write(&candidate,vec![0u8;4096]).unwrap();
        assert!(validate_x86_exe(&candidate).is_err());
        fs::remove_file(candidate).unwrap();
    }
}
