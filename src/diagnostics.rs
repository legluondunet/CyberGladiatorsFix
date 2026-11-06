// SPDX-License-Identifier: GPL-3.0-or-later
//! A single launcher-adjacent log for an invocation and its child processes.
//! Only whitelisted, troubleshooting-relevant system data is recorded:
//! never the machine name, username, MAC, registry identifiers or WINEPREFIX.
use crate::{graphics,install,settings};
use std::ffi::{c_char,c_void,CStr};
use std::fs::{self,OpenOptions};
use std::io::{self,Write};
use std::path::{Path,PathBuf};

pub const LOG_NAME:&str="CyberGladiatorsFix.log";

#[repr(C)]
#[derive(Default)]
struct SystemTime {
    year:u16,month:u16,day_of_week:u16,day:u16,
    hour:u16,minute:u16,second:u16,milliseconds:u16,
}
#[repr(C)]
struct MemoryStatus {
    length:u32,load:u32,total_phys:u64,avail_phys:u64,
    total_page_file:u64,avail_page_file:u64,
    total_virtual:u64,avail_virtual:u64,avail_extended_virtual:u64,
}
#[repr(C)]
struct DisplayDeviceW {
    cb:u32,
    device_name:[u16;32],
    device_string:[u16;128],
    state_flags:u32,
    device_id:[u16;128],
    device_key:[u16;128],
}
#[repr(C)]
struct DevModeW {
    name:[u16;32],
    spec_version:u16,driver_version:u16,size:u16,driver_extra:u16,
    fields:u32,
    position_x:i32,position_y:i32,orientation:u32,fixed_output:u32,
    color:u16,duplex:u16,y_resolution:u16,tt_option:u16,collate:u16,
    form_name:[u16;32],log_pixels:u16,
    bits_per_pel:u32,pels_width:u32,pels_height:u32,
    display_flags:u32,frequency:u32,
    icm_method:u32,icm_intent:u32,media_type:u32,dither_type:u32,
    reserved1:u32,reserved2:u32,panning_width:u32,panning_height:u32,
}
#[repr(C)]
struct OsVersion {
    size:u32,major:u32,minor:u32,build:u32,platform:u32,
    service_pack:[u16;128],
}
#[link(name="kernel32")]
unsafe extern "system" {
    fn GetLocalTime(time:*mut SystemTime);
    fn GlobalMemoryStatusEx(status:*mut MemoryStatus)->i32;
    fn GetModuleHandleA(name:*const c_char)->*mut c_void;
    fn GetProcAddress(module:*mut c_void,name:*const c_char)->*mut c_void;
}
#[link(name="user32")]
unsafe extern "system" {
    fn GetSystemMetrics(index:i32)->i32;
    fn EnumDisplayDevicesW(device:*const u16,index:u32,out:*mut DisplayDeviceW,flags:u32)->i32;
    fn EnumDisplaySettingsW(device:*const u16,mode:i32,out:*mut DevModeW)->i32;
}
#[link(name="ntdll")]
unsafe extern "system" {
    fn RtlGetVersion(version:*mut OsVersion)->i32;
}

fn timestamp()->String{
    let mut t=SystemTime::default();
    unsafe{GetLocalTime(&mut t)};
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        t.year,t.month,t.day,t.hour,t.minute,t.second)
}
fn clean(value:&str)->String {
    // Keep user-supplied driver/CPU strings from forging new log records.
    value.chars().map(|c|if c.is_control(){' '}else{c})
        .take(8192).collect::<String>().trim().to_owned()
}
fn utf16_name(value:&[u16])->String{
    let end=value.iter().position(|v|*v==0).unwrap_or(value.len());
    clean(&String::from_utf16_lossy(&value[..end]))
}
fn gpu_name()->String{
    let mut fallback=None;
    for index in 0..8{
        let mut dev:DisplayDeviceW=unsafe{std::mem::zeroed()};
        dev.cb=std::mem::size_of::<DisplayDeviceW>() as u32;
        if unsafe{EnumDisplayDevicesW(std::ptr::null(),index,&mut dev,0)}==0{break;}
        let name=utf16_name(&dev.device_string);
        if !name.is_empty(){
            if dev.state_flags&0x4!=0{return name;}
            if fallback.is_none(){fallback=Some(name);}
        }
    }
    fallback.unwrap_or_else(||"unavailable".into())
}
/// Primary desktop geometry captured before CYBER.EXE can request its old
/// 640x480 fullscreen mode. Only used for the opt-in GameFrame experiment.
pub fn primary_desktop_size()->Option<(u32,u32)>{
    let (w,h)=unsafe{(GetSystemMetrics(0),GetSystemMetrics(1))};
    (w>=640 && h>=480 && w<=16384 && h<=16384).then_some((w as u32,h as u32))
}
fn display_info()->String{
    let (w,h)=unsafe{(GetSystemMetrics(0),GetSystemMetrics(1))};
    let mut mode:DevModeW=unsafe{std::mem::zeroed()};
    mode.size=std::mem::size_of::<DevModeW>() as u16;
    let rate=if unsafe{EnumDisplaySettingsW(std::ptr::null(),-1,&mut mode)}!=0
        && mode.frequency>1 && mode.frequency<2000{
        format!("{}Hz (Win32 reported)",mode.frequency)
    }else{"unknown refresh".to_owned()};
    format!("{w}x{h}, {rate}")
}
fn windows_version()->String{
    let mut v=OsVersion{size:std::mem::size_of::<OsVersion>() as u32,
        major:0,minor:0,build:0,platform:0,service_pack:[0;128]};
    if unsafe{RtlGetVersion(&mut v)}==0 {
        format!("NT {}.{} build {} (Win32 reported)",v.major,v.minor,v.build)
    }else{"unavailable".into()}
}
fn wine_version()->Option<String>{
    let module=unsafe{GetModuleHandleA(c"ntdll.dll".as_ptr())};
    if module.is_null(){return None;}
    let symbol=unsafe{GetProcAddress(module,c"wine_get_version".as_ptr())};
    if symbol.is_null(){return None;}
    let call:unsafe extern "C" fn()->*const c_char=unsafe{std::mem::transmute(symbol)};
    let ptr=unsafe{call()};
    if ptr.is_null(){return None;}
    Some(clean(&unsafe{CStr::from_ptr(ptr)}.to_string_lossy()))
}
fn physical_memory()->String{
    let mut m=MemoryStatus{
        length:std::mem::size_of::<MemoryStatus>() as u32,
        load:0,total_phys:0,avail_phys:0,total_page_file:0,
        avail_page_file:0,total_virtual:0,avail_virtual:0,
        avail_extended_virtual:0,
    };
    if unsafe{GlobalMemoryStatusEx(&mut m)}!=0 {
        format!("{:.1} GiB (Win32 reported)",m.total_phys as f64/1024f64.powi(3))
    }else{"unavailable".into()}
}
fn safe_env_flag(key:&str)->String{
    // Report only the known boolean switch, not an arbitrary environment
    // value (which might contain private paths, tokens or user input).
    match std::env::var(key).ok().as_deref() {
        None=>"unset".into(),
        Some("0")=>"0".into(),
        Some("1")=>"1".into(),
        _=>"nonstandard value (redacted)".into(),
    }
}
fn command_label(args:&[std::ffi::OsString])->String {
    if args.is_empty(){return "install_or_update".into();}
    let known=args.iter().filter_map(|s|s.to_str())
        .filter(|s|s.starts_with("--"))
        .filter(|s|s.len()<=64 && s.chars().all(|c|c.is_ascii_alphanumeric()||"-_=".contains(c)))
        .collect::<Vec<_>>();
    if known.is_empty(){"install_from_disc".into()}else{known.join(" ")}
}
fn write_line(file:&mut impl Write,component:&str,msg:&str)->io::Result<()>{
    for line in msg.lines() {
        writeln!(file,"[{}] [{}] {}",timestamp(),component,clean(line))?;
    }
    Ok(())
}
pub fn log(root:&Path,component:&str,msg:&str){
    let path=root.join(LOG_NAME);
    if let Ok(mut f)=OpenOptions::new().create(true).append(true).open(path) {
        let _=write_line(&mut f,component,msg);
    }
}
pub fn location(root:&Path)->PathBuf{root.join(LOG_NAME)}
pub fn start_session(root:&Path,args:&[std::ffi::OsString])->Result<(),String>{
    let path=location(root);
    if let Ok(meta)=fs::symlink_metadata(&path) {
        if !meta.is_file()||meta.file_type().is_symlink(){
            return Err(format!("Refusing to overwrite non-regular or symlink log at {}",path.display()));
        }
    }
    // Exactly one log per invocation. A future-run log never contains
    // cumulative traces from a previous experiment.
    let mut file=OpenOptions::new().create(true).write(true).truncate(true)
        .open(&path).map_err(|e|format!("Cannot initialize {}: {e}",path.display()))?;
    write_line(&mut file,"SESSION",&format!(
        "BEGIN version={}-{} command={} pid={}",
        env!("CARGO_PKG_VERSION"),env!("CYBER_COMMIT"),
        command_label(args),std::process::id()
    )).map_err(|e|e.to_string())?;
    write_line(&mut file,"SYSTEM",&format!(
        "runtime={} Win32_OS={} CPU={} RAM={} GPU={} DISPLAY={}",
        wine_version().map(|v|format!("Wine {v}")).unwrap_or_else(||"native Windows (no Wine version detected)".into()),
        windows_version(),
        std::env::var("PROCESSOR_IDENTIFIER").ok().map(|v|clean(&v))
            .filter(|v|!v.is_empty()).unwrap_or_else(||"unknown".into()),
        physical_memory(),gpu_name(),display_info()
    )).map_err(|e|e.to_string())?;
    let host_os=std::env::var("CYBER_HOST_OS").ok()
        .map(|v|clean(&v)).filter(|v|!v.is_empty())
        .unwrap_or_else(||"not reported (use CyberGladiatorsFix-Wine.sh to include native Linux OS)".into());
    let host_kernel=std::env::var("CYBER_HOST_KERNEL").ok()
        .map(|v|clean(&v)).filter(|v|!v.is_empty())
        .unwrap_or_else(||"not reported".into());
    write_line(&mut file,"HOST",&format!(
        "Linux_distribution={} Linux_kernel={}",host_os,host_kernel
    )).map_err(|e|e.to_string())?;
    write_line(&mut file,"RUNTIME",&format!(
        "PROTON_USE_WINED3D={} WINEDEBUG={} WINEPREFIX=not logged (privacy)",
        safe_env_flag("PROTON_USE_WINED3D"),
        if std::env::var_os("WINEDEBUG").is_some(){"set (value omitted)"}else{"unset"}
    )).map_err(|e|e.to_string())?;
    let game=root.join(install::DIR);
    if game.is_dir() {
        // Read *effective* options through the same parsers as the launch
        // gate. Never copy the INI verbatim (future entries may be private).
        match (settings::read(&game),settings::read_cdda(&game),
            settings::read_allow_modified_iq(&game)) {
            (Ok(skip),Ok(cdda),Ok(iq))=>{
                write_line(&mut file,"CONFIG",&format!(
                    "SkipIntro={} EnableCDDA={} AllowModifiedIQ={} CDDA_FORMAT={}",
                    u8::from(skip),u8::from(cdda),u8::from(iq),cd_audio_backend(cdda)
                )).map_err(|e|e.to_string())?;
            }
            _=>write_line(&mut file,"CONFIG",
                "game settings unavailable or invalid; launcher will validate before modifying game")
                .map_err(|e|e.to_string())?,
        }
        if let Ok(summary)=graphics::describe_saved(&game){
            write_line(&mut file,"CONFIG",&summary).map_err(|e|e.to_string())?;
        }else{
            write_line(&mut file,"CONFIG","CYBER.INI graphics options unavailable or unrecognized")
                .map_err(|e|e.to_string())?;
        }
    }else{
        write_line(&mut file,"CONFIG","game installation not present yet")
            .map_err(|e|e.to_string())?;
    }
    file.sync_all().map_err(|e|format!("Cannot commit log header: {e}"))
}
// Record the *selected* CD audio backend, not a hardcoded FLAC claim when
// EnableCDDA=0 asks the same WinMM bridge to forward original CD audio calls.
fn cd_audio_backend(enabled:bool)->&'static str{
    if enabled{"FLAC/44100Hz/16bit/stereo"}else{"system_WinMM_native_CD_audio"}
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn config_log_distinguishes_music_from_video_bridge(){
        assert_eq!(cd_audio_backend(true),"FLAC/44100Hz/16bit/stereo");
        assert_eq!(cd_audio_backend(false),"system_WinMM_native_CD_audio");
    }
    #[test] fn only_known_cli_switches_enter_log(){
        assert_eq!(command_label(&[]),"install_or_update");
        assert_eq!(command_label(&["--launch".into(),"--trace".into()]),"--launch --trace");
        assert_eq!(command_label(&["C:\\Users\\private\\game.cue".into()]),"install_from_disc");
    }
    #[test] fn log_values_cannot_inject_newlines(){
        assert_eq!(clean("Adapter\\n[ERROR] forged"),"Adapter\\n[ERROR] forged");
        assert_eq!(clean("Adapter\n[ERROR] forged"),"Adapter [ERROR] forged");
    }
    #[test] fn win32_display_structs_have_stable_abi_sizes(){
        assert_eq!(std::mem::size_of::<DisplayDeviceW>(),840);
        assert_eq!(std::mem::size_of::<DevModeW>(),220);
    }
}
