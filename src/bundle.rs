// SPDX-License-Identifier: GPL-3.0-or-later
//! Self-contained x64 launcher with one privately built x86 WinMM bridge DLL.
//! The DLL handles the original fullscreen Cinepak AVI, optional CDDA music,
//! and WineD3D/DXVK WndProc repair. No separate intro process is distributed.
//! The Windows CI must build cdda/ x86 before the root x64 crate.
use sha2::{Digest,Sha256};

pub const CDDA_DLL:&[u8]=include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/cdda/target/i686-pc-windows-msvc/release/CyberGladiatorsCDDA.dll"
));
/// The Apache-2.0 GameFrame wrapper is bundled ONLY in the corresponding
/// Windows build. Proprietary nGlide is never included in any build.
#[cfg(gameframe_embedded)]
pub const GAMEFRAME_GLIDE_DLL:&[u8]=include_bytes!(env!("CYBER_GAMEFRAME_GLIDE_DLL_PATH"));
#[cfg(not(gameframe_embedded))]
pub const GAMEFRAME_GLIDE_DLL:&[u8]=&[];

pub fn gameframe_sha256()->Option<&'static str>{
    option_env!("CYBER_GAMEFRAME_GLIDE_SHA256")
}

/// Reject corrupt, foreign, or wrong-architecture embedded WinMM components.
/// This validates PE shape; the SHA of each staged/installed copy is checked
/// separately by cdda::try_install and its receipt/backup protections.
pub(crate) fn pe32_dll_valid(bytes:&[u8])->bool{
    if !(4096..=16*1024*1024).contains(&bytes.len()) || &bytes[..2]!=b"MZ" {return false;}
    let offset=u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as usize;
    let Some(end)=offset.checked_add(26) else{return false;};
    if end>bytes.len() || &bytes[offset..offset+4]!=b"PE\0\0" {return false;}
    let machine=u16::from_le_bytes(bytes[offset+4..offset+6].try_into().unwrap());
    let flags=u16::from_le_bytes(bytes[offset+22..offset+24].try_into().unwrap());
    let optional=u16::from_le_bytes(bytes[offset+24..offset+26].try_into().unwrap());
    machine==0x14c && optional==0x10b && flags&0x2000!=0
}
pub fn verify()->Result<(),String>{
    if !pe32_dll_valid(CDDA_DLL){
        return Err("Bundled WinMM bridge is not a valid Windows x86 PE32 DLL".into());
    }
    if !GAMEFRAME_GLIDE_DLL.is_empty(){
        if !pe32_dll_valid(GAMEFRAME_GLIDE_DLL){
            return Err("Bundled GameFrame Glide DLL is not Windows x86 PE32".into());
        }
        let expected=gameframe_sha256().ok_or("Missing embedded GameFrame SHA-256 pin")?;
        if hash(GAMEFRAME_GLIDE_DLL)!=expected{
            return Err("Bundled GameFrame Glide DLL SHA-256 mismatch".into());
        }
    }
    Ok(())
}
pub fn hash(bytes:&[u8])->String{format!("{:x}",Sha256::digest(bytes))}
pub fn describe()->Result<String,String>{
    verify()?;
    let mut summary=format!(
        "EMBEDDED_BUNDLE x86_WinMM_video_CDDA_DLL_bytes={} sha256={} separate_intro_exe=0",
        CDDA_DLL.len(),hash(CDDA_DLL)
    );
    if GAMEFRAME_GLIDE_DLL.is_empty(){
        summary.push_str(" open_source_glide_embedded=0 proprietary_nGlide_embedded=0");
    }else{
        summary.push_str(&format!(
            " open_source_glide_embedded=1 gameframe_glide_dll_bytes={} gameframe_sha256={} one_exe_distribution=1 proprietary_nGlide_embedded=0",
            GAMEFRAME_GLIDE_DLL.len(),hash(GAMEFRAME_GLIDE_DLL)
        ));
    }
    Ok(summary)
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn embedded_winmm_bridge_is_a_verified_x86_pe_dll(){
        verify().unwrap();
        assert_eq!(&CDDA_DLL[..2],b"MZ");
        assert_eq!(hash(CDDA_DLL).len(),64);
    }
    #[test]
    fn gameframe_bundle_matches_sha256_or_is_absent(){
        if GAMEFRAME_GLIDE_DLL.is_empty(){
            assert!(gameframe_sha256().is_none());
        }else{
            verify().unwrap();
            assert_eq!(hash(GAMEFRAME_GLIDE_DLL),gameframe_sha256().unwrap());
        }
    }
    #[test]
    fn rejecting_non_pe_and_exe_for_embedded_dll(){
        assert!(!pe32_dll_valid(&vec![0;4096]));
        let mut copy=CDDA_DLL.to_vec();
        let offset=u32::from_le_bytes(copy[0x3c..0x40].try_into().unwrap()) as usize;
        copy[offset+23]&=!0x20; // clear the IMAGE_FILE_DLL flag (0x2000)
        assert!(!pe32_dll_valid(&copy));
    }
}
