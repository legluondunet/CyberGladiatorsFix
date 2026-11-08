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
/// Locate direct x86 CALL rel32 instructions targeting the verified
/// function beginning at VA 0x00405EB7. Byte-pattern hits are *candidates*,
/// not a control-flow proof without a full x86 decoder.
fn rel32_callers(data:&[u8],sections:&[CodeSection],target:u32)->Vec<Candidate>{
    let mut found=Vec::new();
    for sec in sections{
        let Some(code)=data.get(sec.raw..sec.raw+sec.size) else{continue;};
        for off in 0..code.len().saturating_sub(4){
            if code[off]!=0xe8{continue;}
            let Some(bytes)=code.get(off+1..off+5) else{continue;};
            let delta=i32::from_le_bytes(bytes.try_into().unwrap()) as i64;
            let va=sec.image_base as i64+sec.rva as i64+off as i64;
            if va+5+delta!=target as i64{continue;}
            let Ok(va)=u32::try_from(va) else{continue;};
            found.push(Candidate{raw:sec.raw+off,va,kind:"call_rel32_to_mode_wrapper"});
        }
    }
    found
}
/// A specific x86 prologue and COM-style indirect call observed in the
/// SHA-256-verified original FR EXE. The apparent COM object type cannot
/// be proven solely from the bytes; the caller arguments offer more context.
fn likely_mode_wrapper(data:&[u8])->bool{
    const RAW:usize=0x54b7;
    const EXPECTED:&[u8]=&[
        0x55,0x8b,0xec,0x51,0x53,0x51,0x52,0x50,
        0xa1,0x8c,0x72,0x44,0x00,0x50,0x8b,0x10,
        0xff,0x52,0x54,
    ];
    data.get(RAW..RAW+EXPECTED.len())==Some(EXPECTED)
}
/// Exact instruction sequences observed in the authenticated French PE32.
/// These are *verified original-file* locations, not address patterns to
/// overwrite or hooks. Document all three aspects of the 16-bpp dependency:
/// fullscreen/exclusive cooperation, the primary mode wrapper, and the
/// independent mode call in a second graphics code path.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
struct DisplayPathEvidence{
    exclusive_cooperation:bool,
    primary_mode:bool,
    alternate_mode:bool,
    secondary_mode:bool,
}
fn exact_original_bytes(data:&[u8],raw:usize,expected:&[u8])->bool{
    data.get(raw..raw.saturating_add(expected.len()))==Some(expected)
}
fn display_path_evidence(data:&[u8])->DisplayPathEvidence{
    // 00405E7E: push 0x11 (= DDSCL_FULLSCREEN | DDSCL_EXCLUSIVE),
    // followed by IDirectDraw v1::SetCooperativeLevel, slot 20, +0x50.
    let exclusive_cooperation=exact_original_bytes(data,0x547e,&[
        0x6a,0x11,0xff,0x35,0x18,0x81,0x52,0x00,
        0xa1,0x8c,0x72,0x44,0x00,0x50,0x8b,0x10,
        0xff,0x52,0x50
    ]);
    // 004050AF: (ECX,EDX,EAX) = (16,480,640), then
    // call original 00405EB7 COM SetDisplayMode wrapper.
    let primary_mode=exact_original_bytes(data,0x46af,&[
        0xb9,0x10,0x00,0x00,0x00,
        0xba,0xe0,0x01,0x00,0x00,
        0xb8,0x80,0x02,0x00,0x00,
        0xe8,0xf4,0x0d,0x00,0x00
    ]);
    // 00406A68: a second caller that can request the identical mode.
    let alternate_mode=exact_original_bytes(data,0x6068,&[
        0xb9,0x10,0x00,0x00,0x00,
        0xba,0xe0,0x01,0x00,0x00,
        0xb8,0x80,0x02,0x00,0x00,
        0xe8,0x3b,0xf4,0xff,0xff
    ]);
    // 00436411: (16,480,640) pushed to a *different* COM object
    // global at 0044AF70; IDirectDraw v1::SetDisplayMode at +0x54.
    let secondary_mode=exact_original_bytes(data,0x35a11,&[
        0x6a,0x10,
        0x68,0xe0,0x01,0x00,0x00,
        0x68,0x80,0x02,0x00,0x00,
        0x8b,0x0d,0x70,0xaf,0x44,0x00,
        0x51,0x8b,0x01,0xff,0x50,0x54
    ]);
    DisplayPathEvidence{
        exclusive_cooperation,primary_mode,alternate_mode,secondary_mode
    }
}
/// Recognize the DirectDraw v1 primary flip-chain descriptor, not merely
/// SetDisplayMode. In retail x86 code, four consecutive MOV [abs],imm32
/// fill DDSURFACEDESC: dwSize=108, dwFlags=CAPS|BACKBUFFERCOUNT (0x21),
/// dwBackBufferCount=1, ddscaps.dwCaps=PRIMARY|FLIP|COMPLEX (0x218).
///
/// The addresses must point into a *single* descriptor: +0, +4, +20,
/// +104. Refuse partial matches, unrelated instructions and raw byte spans
/// crossing a PE executable section. A candidate remains static evidence;
/// this is not a proof that the call executed.
fn primary_flip_chain_candidates(data:&[u8],sections:&[CodeSection])->Vec<Candidate>{
    let mut found=Vec::new();
    for sec in sections{
        let Some(code)=data.get(sec.raw..sec.raw+sec.size)else{continue;};
        for off in 0..code.len().saturating_sub(39){
            let Some(run)=code.get(off..off+40)else{continue;};
            let Some(base)=dword(run,2)else{continue;};
            let expected=[
                (0u32,108u32),(4,0x21),(20,1),(104,0x218)
            ];
            let valid=expected.iter().enumerate().all(|(idx,(disp,value))|{
                let at=idx*10;
                run.get(at..at+2)==Some(&[0xc7,0x05][..])
                    &&dword(run,at+2)==base.checked_add(*disp)
                    &&dword(run,at+6)==Some(*value)
            });
            if !valid{continue;}
            // Only a verified call through COM slot 6 (+0x18) near the
            // descriptor writes can strengthen this candidate.
            let near_end=off.saturating_add(88).min(code.len());
            let Some(tail)=code.get(off+40..near_end)else{continue;};
            if !tail.windows(3).any(|w|w==[0xff,0x50,0x18]
                 ||w==[0xff,0x51,0x18]||w==[0xff,0x52,0x18]){
                continue;
            }
            let Some(va)=sec.image_base.checked_add(sec.rva)
                .and_then(|x|x.checked_add(off as u32))else{continue;};
            found.push(Candidate{raw:sec.raw+off,va,
                kind:"create_surface_primary_flip_chain"});
        }
    }
    found
}
fn hex_window(data:&[u8],raw:usize,before:usize,after:usize)->String{
    // Small, bounded instruction-adjacent excerpt. Never log the game EXE.
    let start=raw.saturating_sub(before);
    let end=raw.saturating_add(after).min(data.len());
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
    let evidence=display_path_evidence(&data);
    report(root,&format!(
        "display_dependency exclusive_cooperative_level={} exclusive_flags=0x11 (DDSCL_FULLSCREEN|DDSCL_EXCLUSIVE) set_mode_640x480x16_primary={} set_mode_640x480x16_alternate={} set_mode_640x480x16_secondary={} interpretation=exact_verified_file_instructions_not_runtime_proof read_only=1",
        u8::from(evidence.exclusive_cooperation),
        u8::from(evidence.primary_mode),
        u8::from(evidence.alternate_mode),
        u8::from(evidence.secondary_mode),
    ));
    if evidence.exclusive_cooperation&&evidence.primary_mode&&evidence.alternate_mode&&evidence.secondary_mode{
        report(root,
            "virtualization_requirements cooperative_level=nonexclusive logical_mode=640x480x16 physical_desktop_mode=preserved need_consistent_DirectDraw_surfaces=1 simple_bpp_or_mode_substitution=unsafe render_target=desktop_sized_borderless");
    }
    let primary_flip=primary_flip_chain_candidates(&data,&sections);
    report(root,&format!(
        "primary_flip_chain_candidates={} flags=0x21(DDSD_CAPS|DDSD_BACKBUFFERCOUNT) caps=0x218(DDSCAPS_PRIMARYSURFACE|DDSCAPS_FLIP|DDSCAPS_COMPLEX) backbuffers=1 surfaces_require_virtualization=1 static_scan=1 read_only=1",
        primary_flip.len()
    ));
    for candidate in primary_flip.iter().take(8){
        report(root,&format!(
            "primary_flip_chain_candidate={:#010x} raw={:#08x} COM_CreateSurface_slot=6 context_bytes={}",
            candidate.va,candidate.raw,hex_window(&data,candidate.raw,8,96)
        ));
    }
    let sites=candidates(&data,&sections);
    let calls:Vec<_>=sites.iter().filter(|site|site.kind=="call_vtable_slot21").collect();
    let loads:Vec<_>=sites.iter().filter(|site|site.kind=="mov_vtable_slot21").collect();
    report(root,&format!(
        "status=ok original_sha256_verified=1 executable_sections={} slot21_candidates={} slot21_calls={} slot21_loads={} interpretation=possible_x86_vtable_call_or_load read_only=1",
        sections.len(),sites.len(),calls.len(),loads.len()
    ));
    // Call sites are more informative than arbitrary reads of a struct's
    // +0x54 field. Report every call (up to 64), including any formerly
    // hidden by the original top-24 candidate cutoff.
    for site in calls.iter().take(64){
        report(root,&format!(
            "call_site={:#010x} raw={:#08x} opcode={} context_bytes={}",
            site.va,site.raw,site.kind,hex_window(&data,site.raw,96,48)
        ));
    }
    if calls.len()>64{
        report(root,&format!("additional_call_candidates_omitted={}",calls.len()-64));
    }
    if likely_mode_wrapper(&data){
        report(root,
            "mode_wrapper_candidate=0x00405eb7 indirect_call=0x00405ec7 object_global=0x0044728c pushed_argument_registers=EAX,EDX,ECX probable_api=IDirectDraw_v1_SetDisplayMode proof=not_yet_verified");
        let callers=rel32_callers(&data,&sections,0x00405eb7);
        report(root,&format!(
            "mode_wrapper_direct_caller_candidates={} scan=E8_rel32 read_only=1",
            callers.len()
        ));
        for site in callers.iter().take(48){
            report(root,&format!(
                "wrapper_caller={:#010x} raw={:#08x} nearby_bytes={}",
                site.va,site.raw,hex_window(&data,site.raw,80,24)
            ));
        }
        if callers.len()>48{
            report(root,&format!(
                "additional_wrapper_callers_omitted={}",callers.len()-48
            ));
        }
    }else{
        report(root,"mode_wrapper_candidate=unverified_signature no_action=1");
    }
    // Retain representative non-call sites as examples only.
    for site in loads.iter().take(6){
        report(root,&format!(
            "load_sample={:#010x} raw={:#08x} opcode={} nearby_bytes={}",
            site.va,site.raw,site.kind,hex_window(&data,site.raw,24,12)
        ));
    }
    if loads.len()>6{
        report(root,&format!("additional_load_samples_omitted={}",loads.len()-6));
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
    fn rel32_scan_finds_call_to_wrapper_but_no_other_call(){
        let mut image=vec![0u8;0x300];
        let sec=CodeSection{
            raw:0x100,size:0x80,rva:0x1000,image_base:0x400000
        };
        // VA 0x401020 -> 0x401010: E8 followed by signed -0x15.
        image[0x120..0x125].copy_from_slice(&[0xe8,0xeb,0xff,0xff,0xff]);
        image[0x130..0x135].copy_from_slice(&[0xe8,0,0,0,0]);
        let list=rel32_callers(&image,&[sec],0x401010);
        assert_eq!(list,vec![Candidate{
            raw:0x120,va:0x401020,kind:"call_rel32_to_mode_wrapper"
        }]);
    }
    #[test]
    fn mode_wrapper_requires_exact_verified_bytes(){
        let mut data=vec![0u8;0x54d0];
        assert!(!likely_mode_wrapper(&data));
        let sig:&[u8]=&[
            0x55,0x8b,0xec,0x51,0x53,0x51,0x52,0x50,
            0xa1,0x8c,0x72,0x44,0x00,0x50,0x8b,0x10,
            0xff,0x52,0x54
        ];
        data[0x54b7..0x54b7+sig.len()].copy_from_slice(sig);
        assert!(likely_mode_wrapper(&data));
        data[0x54c7]=0x90;
        assert!(!likely_mode_wrapper(&data));
    }
    #[test]
    fn recognizes_primary_backbuffer_flip_chain_but_rejects_partial_descriptors(){
        let mut data=vec![0u8;0x300];
        let sec=CodeSection{raw:0x100,size:0xc0,rva:0x1000,image_base:0x400000};
        let at=0x130;
        let base=0x0044_efa8u32;
        let fields=[(0u32,108u32),(4,0x21),(20,1),(104,0x218)];
        for (i,(offset,value)) in fields.iter().enumerate(){
            let pos=at+i*10;
            data[pos..pos+2].copy_from_slice(&[0xc7,0x05]);
            data[pos+2..pos+6].copy_from_slice(&(base+offset).to_le_bytes());
            data[pos+6..pos+10].copy_from_slice(&value.to_le_bytes());
        }
        // A 16-bpp primary *with flip-chain* is invalid in native
        // DirectDraw windowed cooperation without a virtualized surface.
        data[at+55..at+58].copy_from_slice(&[0xff,0x50,0x18]);
        let expected=vec![Candidate{raw:at,va:0x401030,
            kind:"create_surface_primary_flip_chain"}];
        assert_eq!(primary_flip_chain_candidates(&data,&[sec]),expected);
        data[at+36]=0x00; // corrupt DDSCAPS_PRIMARYSURFACE|FLIP|COMPLEX.
        assert!(primary_flip_chain_candidates(&data,&[sec]).is_empty());
        data[at+36]=0x18;
        data[at+55..at+58].copy_from_slice(&[0x90,0x90,0x90]);
        assert!(primary_flip_chain_candidates(&data,&[sec]).is_empty());
    }
    #[test]
    fn display_dependency_requires_exact_independent_original_sequences(){
        let mut data=vec![0u8;0x35a11+24];
        let coop=&[
            0x6a,0x11,0xff,0x35,0x18,0x81,0x52,0x00,
            0xa1,0x8c,0x72,0x44,0x00,0x50,0x8b,0x10,
            0xff,0x52,0x50
        ];
        let primary=&[
            0xb9,0x10,0x00,0x00,0x00,0xba,0xe0,0x01,0x00,0x00,
            0xb8,0x80,0x02,0x00,0x00,0xe8,0xf4,0x0d,0x00,0x00
        ];
        let alternate=&[
            0xb9,0x10,0x00,0x00,0x00,0xba,0xe0,0x01,0x00,0x00,
            0xb8,0x80,0x02,0x00,0x00,0xe8,0x3b,0xf4,0xff,0xff
        ];
        let secondary=&[
            0x6a,0x10,0x68,0xe0,0x01,0x00,0x00,
            0x68,0x80,0x02,0x00,0x00,
            0x8b,0x0d,0x70,0xaf,0x44,0x00,
            0x51,0x8b,0x01,0xff,0x50,0x54
        ];
        assert_eq!(display_path_evidence(&data),DisplayPathEvidence{
            exclusive_cooperation:false,primary_mode:false,
            alternate_mode:false,secondary_mode:false
        });
        data[0x547e..0x547e+coop.len()].copy_from_slice(coop);
        data[0x46af..0x46af+primary.len()].copy_from_slice(primary);
        data[0x6068..0x6068+alternate.len()].copy_from_slice(alternate);
        data[0x35a11..0x35a11+secondary.len()].copy_from_slice(secondary);
        assert_eq!(display_path_evidence(&data),DisplayPathEvidence{
            exclusive_cooperation:true,primary_mode:true,
            alternate_mode:true,secondary_mode:true
        });
        data[0x547f]=0x08; // DDSCL_NORMAL is *not* this original.
        assert!(!display_path_evidence(&data).exclusive_cooperation);
        data[0x547f]=0x11;
        data[0x35a11+1]=0x08;
        assert!(!display_path_evidence(&data).secondary_mode);
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