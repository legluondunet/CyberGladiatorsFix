// SPDX-License-Identifier: GPL-3.0-or-later
//! Passive inspection of the SHA-256-verified French retail CYBER.EXE backup.
//! DirectDraw v1 SetDisplayMode is COM slot 21 (byte offset 0x54).
//! Scanning CODE instructions is only a candidate locator, NOT disassembly,
//! proof of control flow, or a substitute for inspecting original x86 bytes.
//! No process injection, DLL/VTable hooks, PE changes, or display-mode writes.

use crate::{diagnostics,fr_patch,patch};
use std::{fs,path::Path};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
struct CodeSection {
    raw:usize,
    size:usize,
    rva:u32,
    image_base:u32,
}

fn word(data:&[u8],off:usize)->Option<u16>{
    let slice=data.get(off..off.checked_add(2)?)?;
    Some(u16::from_le_bytes([slice[0],slice[1]]))
}
fn dword(data:&[u8],off:usize)->Option<u32>{
    let slice=data.get(off..off.checked_add(4)?)?;
    Some(u32::from_le_bytes(slice.try_into().ok()?))
}
fn executable_sections(data:&[u8])->Option<Vec<CodeSection>>{
    if data.get(0..2)?!=b"MZ" {return None;}
    let pe=dword(data,0x3c)? as usize;
    if data.get(pe..pe.checked_add(4)?)?!=b"PE\0\0" {return None;}
    let count=usize::from(word(data,pe.checked_add(6)?)?);
    if count==0 || count>96{return None;}
    let opt=pe.checked_add(24)?;
    if word(data,opt)?!=0x10b{return None;} // PE32, never PE32+.
    let image_base=dword(data,opt.checked_add(28)?)?;
    let opt_size=usize::from(word(data,pe.checked_add(20)?)?);
    if opt_size<96{return None;}
    let first=opt.checked_add(opt_size)?;
    let mut result=Vec::new();
    for i in 0..count{
        let sec=first.checked_add(i.checked_mul(40)?)?;
        let characteristics=dword(data,sec.checked_add(36)?)?;
        if characteristics&0x2000_0000==0{continue;} // IMAGE_SCN_MEM_EXECUTE.
        let rva=dword(data,sec.checked_add(12)?)?;
        let raw=usize::try_from(dword(data,sec.checked_add(20)?)?).ok()?;
        let raw_size=usize::try_from(dword(data,sec.checked_add(16)?)?).ok()?;
        let end=raw.checked_add(raw_size)?;
        if raw_size==0 || end>data.len(){return None;}
        result.push(CodeSection{raw,size:raw_size,rva,image_base});
    }
    (!result.is_empty()).then_some(result)
}

/// Match a possible CALL through an IDirectDraw v1 vtable at +0x54,
/// or MOV that fetches an indirect function pointer at +0x54.
/// Accept 8-bit or 32-bit displacement and account for an optional SIB byte.
/// These are *byte-pattern candidates*, including possible false positives.
fn possible_slot21(code:&[u8],at:usize)->Option<&'static str>{
    let op=*code.get(at)?;
    if op!=0xff && op!=0x8b{return None;}
    let modrm=*code.get(at.checked_add(1)?)?;
    if op==0xff && (modrm>>3)&7!=2{return None;} // FF /2 = near CALL.
    let mode=modrm>>6;
    if mode!=1 && mode!=2{return None;}
    let sib=usize::from(modrm&7==4);
    let disp=at.checked_add(2+ sib)?;
    match mode {
        1 if code.get(disp)==Some(&0x54)=>{},
        2 if code.get(disp..disp.checked_add(4)?)==Some(&[0x54,0,0,0][..])=>{},
        _=>return None,
    };
    if op==0xff{Some("call_vtable_slot21")}else{Some("mov_vtable_slot21")}
}
#[derive(Clone,Debug,PartialEq,Eq)]
struct Candidate {
    raw:usize,
    va:u32,
    kind:&'static str,
}
fn candidates(data:&[u8],sections:&[CodeSection])->Vec<Candidate>{
    let mut found=Vec::new();
    for sec in sections{
        let Some(code)=data.get(sec.raw..sec.raw+sec.size) else{continue;};
        for offset in 0..code.len(){
            let Some(kind)=possible_slot21(code,offset) else{continue;};
            let Some(va)=sec.image_base.checked_add(sec.rva)
                .and_then(|x|x.checked_add(offset as u32)) else{continue;};
            found.push(Candidate{raw:sec.raw+offset,va,kind});
        }
    }
    found
}
fn hex_window(data:&[u8],raw:usize)->String{
    // A bounded instruction-adjacent excerpt, not the game EXE itself.
    let start=raw.saturating_sub(24);
    let end=raw.saturating_add(12).min(data.len());
    data[start..end].iter().map(|b|format!("{b:02x}"))
        .collect::<Vec<_>>().join(" ")
}
fn report(root:&Path,message:&str){diagnostics::log(root,"FR_STATIC_DDRAW",message);}

/// Called once before spawning only the authenticated FR executable on native
/// Windows. Original backup remains unmodified and is never redistributed.
/// Malformed or missing backups are logged as a skipped diagnostic, not used.
pub fn inspect(root:&Path,game_exe:&Path){
    if !diagnostics::is_native_windows(){return;}
    let backup=patch::backup_path(game_exe);
    let meta=match fs::symlink_metadata(&backup){
        Ok(v)=>v,
        Err(_)=>{
            report(root,"status=skip reason=original_backup_unavailable read_only=1");
            return;
        }
    };
    if !meta.is_file()||meta.file_type().is_symlink(){
        report(root,"status=skip reason=not_regular_backup read_only=1");
        return;
    }
    let data=match fs::read(&backup){
        Ok(v)=>v,
        Err(_)=>{
            report(root,"status=skip reason=cannot_read_backup read_only=1");
            return;
        }
    };
    if data.len()!=fr_patch::LEN || patch::digest(&data)!=fr_patch::ORIGINAL{
        report(root,"status=skip reason=unverified_original_sha256 read_only=1");
        return;
    }
    let Some(sections)=executable_sections(&data) else{
        report(root,"status=skip reason=invalid_executable_sections read_only=1");
        return;
    };
    let sites=candidates(&data,&sections);
    report(root,&format!(
        "status=ok original_sha256_verified=1 executable_sections={} slot21_candidates={} interpretation=possible_x86_vtable_call_or_load read_only=1",
        sections.len(),sites.len()
    ));
    // Keep the unified log concise. Count unshown candidates explicitly.
    for candidate in sites.iter().take(24){
        report(root,&format!(
            "site={:#010x} raw={:#08x} opcode={} nearby_bytes={}",
            candidate.va,candidate.raw,candidate.kind,hex_window(&data,candidate.raw)
        ));
    }
    if sites.len()>24{
        report(root,&format!("additional_candidates_omitted={}",sites.len()-24));
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn slot21_finds_direct_call_and_mov_but_not_unrelated_vtable_calls(){
        assert_eq!(possible_slot21(&[0xff,0x50,0x54],0),Some("call_vtable_slot21"));
        assert_eq!(possible_slot21(&[0xff,0x93,0x54,0,0,0],0),Some("call_vtable_slot21"));
        assert_eq!(possible_slot21(&[0x8b,0x42,0x54],0),Some("mov_vtable_slot21"));
        assert_eq!(possible_slot21(&[0xff,0x54,0x24,0x54],0),Some("call_vtable_slot21"));
        assert_eq!(possible_slot21(&[0xff,0x50,0x50],0),None);
        assert_eq!(possible_slot21(&[0xff,0x10,0x54],0),None);
        assert_eq!(possible_slot21(&[0xff,0x90,0x54,0,0],0),None);
    }
    #[test]
    fn rejects_random_bytes_and_invalid_pe_headers(){
        assert!(executable_sections(&[0;512]).is_none());
        assert!(executable_sections(b"MZ").is_none());
    }
    #[test]
    fn pe32_sections_map_raw_offsets_to_virtual_addresses(){
        let mut data=vec![0u8;0x400];
        data[0..2].copy_from_slice(b"MZ");
        data[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        data[0x80..0x84].copy_from_slice(b"PE\0\0");
        data[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
        data[0x94..0x96].copy_from_slice(&0xe0u16.to_le_bytes());
        data[0x98..0x9a].copy_from_slice(&0x10bu16.to_le_bytes());
        data[0xb4..0xb8].copy_from_slice(&0x400000u32.to_le_bytes());
        let sec=0x178;
        data[sec..sec+5].copy_from_slice(b".text");
        data[sec+12..sec+16].copy_from_slice(&0x1000u32.to_le_bytes());
        data[sec+16..sec+20].copy_from_slice(&0x100u32.to_le_bytes());
        data[sec+20..sec+24].copy_from_slice(&0x200u32.to_le_bytes());
        data[sec+36..sec+40].copy_from_slice(&0x6000_0020u32.to_le_bytes());
        data[0x210..0x213].copy_from_slice(&[0xff,0x50,0x54]);
        let sections=executable_sections(&data).unwrap();
        assert_eq!(sections.len(),1);
        assert_eq!(candidates(&data,&sections),vec![Candidate{
            raw:0x210,va:0x401010,kind:"call_vtable_slot21"
        }]);
    }
}