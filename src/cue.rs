// SPDX-License-Identifier: GPL-3.0-or-later
//! CUE/BIN track layout, read-only input validation and data track normalization.
//! CDDA audio is handled separately by audio.rs. Only CUE plus BIN inputs are supported.
use std::fs::{self,File,OpenOptions};
use std::io::{Read,Seek,SeekFrom,Write};
use std::path::{Path,PathBuf,Component};

pub struct TempIso(pub PathBuf);
impl Drop for TempIso { fn drop(&mut self){let _=fs::remove_file(&self.0);} }

#[derive(Clone,Debug)]
pub struct Track {
    pub number:u8,
    pub file:String,
    pub mode:String,
    pub index00:Option<u64>,
    pub index01:u64,
}
fn msf(s:&str)->Result<u64,String>{
    let v:Vec<_>=s.split(':').collect();
    if v.len()!=3||v.iter().any(|p|p.len()!=2||!p.bytes().all(|b|b.is_ascii_digit())) {
        return Err(format!("Invalid CUE timestamp {s}"));
    }
    let nums:Vec<u64>=v.iter().map(|n|n.parse::<u64>()).collect::<Result<_,_>>().map_err(|e|e.to_string())?;
    if nums[1]>=60||nums[2]>=75{return Err(format!("Invalid CUE timestamp {s}"));}
    Ok((nums[0]*60+nums[1])*75+nums[2])
}
fn file_record(line:&str)->Result<String,String>{
    let s=line.trim();
    if let Some(q)=s.strip_prefix('"'){
        let end=q.find('"').ok_or("Unclosed CUE FILE quote")?;
        if !q[end+1..].trim().eq_ignore_ascii_case("BINARY"){
            return Err("Only BINARY CUE files are supported".into());
        }
        return Ok(q[..end].to_string());
    }
    let mut p=s.split_whitespace();
    let name=p.next().ok_or("Empty CUE FILE")?;
    if !p.next().is_some_and(|t|t.eq_ignore_ascii_case("BINARY"))||p.next().is_some(){
        return Err("Unsupported CUE FILE syntax".into());
    }
    Ok(name.to_string())
}
fn valid_relative(name:&str)->bool{
    let p=Path::new(name);
    !p.as_os_str().is_empty() && p.components().all(|c|matches!(c,Component::Normal(_))) &&
        !name.contains([':', '\\', '\0']) && !name.starts_with('/') &&
        p.extension().is_some_and(|e|e.eq_ignore_ascii_case("bin"))
}
/// Resolve a CUE file reference inside its directory, refusing symlinks.
/// A completely absent audio file is allowed and reported separately by the caller.
pub fn bin_path(cue:&Path,name:&str)->Result<Option<PathBuf>,String>{
    if !valid_relative(name){return Err(format!("Unsafe CUE FILE path: {name}"));}
    let base=cue.parent().ok_or("Invalid CUE directory")?;
    let path=base.join(name);
    if !path.exists() {
        // Do not classify dangling symlinks as missing audio.
        if fs::symlink_metadata(&path).is_ok(){return Err(format!("Invalid BIN path: {}",path.display()));}
        return Ok(None);
    }
    let parent=base.canonicalize().map_err(|e|e.to_string())?;
    let actual=path.canonicalize().map_err(|e|e.to_string())?;
    if !actual.starts_with(&parent){return Err("CUE references a BIN outside its directory".into());}
    let mut current=base.to_path_buf();
    for component in Path::new(name).components(){
        let Component::Normal(part)=component else{return Err("Invalid CUE path".into())};
        current.push(part);
        let meta=fs::symlink_metadata(&current).map_err(|e|e.to_string())?;
        if meta.file_type().is_symlink(){return Err("CUE BIN path contains a symlink".into());}
    }
    let meta=fs::symlink_metadata(&path).map_err(|e|e.to_string())?;
    if !meta.is_file(){return Err(format!("CUE BIN is not a regular file: {}",path.display()));}
    Ok(Some(path))
}

pub fn tracks(cue:&Path)->Result<Vec<Track>,String>{
    let metadata=fs::symlink_metadata(cue).map_err(|e|e.to_string())?;
    if !metadata.is_file()||metadata.file_type().is_symlink()||metadata.len()>256*1024{
        return Err("Invalid or oversized CUE".into());
    }
    let contents=fs::read_to_string(cue).map_err(|e|e.to_string())?;
    let mut current_file=None::<String>;
    #[derive(Debug)] struct Pending {number:u8,file:String,mode:String,index00:Option<u64>,index01:Option<u64>}
    let mut items=Vec::<Pending>::new();
    for (line_number,line) in contents.trim_start_matches('\u{feff}').lines().enumerate(){
        let trimmed=line.trim();
        let mut p=trimmed.split_whitespace();
        let Some(op)=p.next() else{continue};
        match op.to_ascii_uppercase().as_str(){
            "FILE"=>{
                let name=file_record(trimmed[op.len()..].trim())?;
                if !valid_relative(&name){return Err(format!("Unsafe CUE FILE path: {name}"));}
                current_file=Some(name);
            },
            "TRACK"=>{
                let no=p.next().ok_or("Missing TRACK number")?.parse::<u8>().map_err(|_|"Invalid track number")?;
                let mode=p.next().ok_or("Missing TRACK mode")?.to_ascii_uppercase();
                if p.next().is_some()||no==0||no>99||items.len()>=99 ||
                   (items.last().is_some_and(|prev|prev.number>=no)){
                    return Err(format!("Invalid TRACK number/order at line {}",line_number+1));
                }
                let file=current_file.clone().ok_or("TRACK occurs before FILE")?;
                items.push(Pending{number:no,file,mode,index00:None,index01:None});
            },
            "INDEX"=>{
                let no=p.next().ok_or("INDEX without number")?;
                let t=p.next().ok_or("INDEX without timestamp")?;
                if p.next().is_some(){return Err("Unexpected INDEX arguments".into());}
                let track=items.last_mut().ok_or("INDEX without TRACK")?;
                match no {
                    "00"=>{if track.index00.replace(msf(t)?).is_some(){return Err("Repeated INDEX 00".into());}},
                    "01"=>{if track.index01.replace(msf(t)?).is_some(){return Err("Repeated INDEX 01".into());}},
                    _=>{},
                }
            },
            "REM"|"PREGAP"|"POSTGAP"|"FLAGS"|"TITLE"|"PERFORMER"|"SONGWRITER"|"CATALOG"|"ISRC"|"CDTEXTFILE"=>{},
            other=>return Err(format!("Unsupported CUE directive {other} at line {}",line_number+1)),
        }
    }
    if items.is_empty(){return Err("CUE has no tracks".into());}
    let mut out=Vec::new();
    for t in items {
        let i1=t.index01.ok_or(format!("TRACK {:02} missing INDEX 01",t.number))?;
        if t.index00.is_some_and(|i0|i0>i1){return Err(format!("TRACK {:02} INDEX 00 follows INDEX 01",t.number));}
        out.push(Track{number:t.number,file:t.file,mode:t.mode,index00:t.index00,index01:i1});
    }
    Ok(out)
}
/// End of a track: next track's INDEX 00 if present (pregap begins there),
/// otherwise INDEX 01. Uses the end of the file for separate-file tracks.
pub fn track_end(tracks:&[Track],which:usize,total_sectors:u64)->Result<u64,String>{
    let current=&tracks[which];
    let next=tracks.get(which+1).filter(|t|t.file.eq_ignore_ascii_case(&current.file));
    let end=next.map(|t|t.index00.unwrap_or(t.index01)).unwrap_or(total_sectors);
    if current.index01>=end||end>total_sectors {return Err(format!("Invalid bounds for track {:02}",current.number));}
    Ok(end)
}

/// Some valid single-BIN Red Book rips store an actual two-second pause as
/// 150 all-zero raw sectors before AUDIO track 02, but omit INDEX 00.
/// Recognize ONLY this exact, bounded anomaly: a first raw 2352-byte data
/// track, next AUDIO track in the same BIN with no INDEX 00, and precisely
/// 150 all-zero sectors immediately preceding the declared INDEX 01.
/// This never skips nonzero/invalid data or modifies the original media.
/// The prior data sectors and retail SHA-256 fingerprints remain mandatory.
const UNINDEXED_AUDIO_PREGAP_SECTORS:u64=150;
fn infer_unindexed_zero_audio_pregap(
    input:&mut File, tracks:&[Track], sector_size:usize, total_sectors:u64
)->Result<Option<u64>,String>{
    if sector_size!=2352 || tracks.len()<2{return Ok(None);}
    let data=&tracks[0];
    let next=&tracks[1];
    if next.mode!="AUDIO" || next.index00.is_some()
        || !next.file.eq_ignore_ascii_case(&data.file)
        || next.index01>total_sectors
        || next.index01<data.index01.saturating_add(UNINDEXED_AUDIO_PREGAP_SECTORS+1)
    {return Ok(None);}
    let start=next.index01-UNINDEXED_AUDIO_PREGAP_SECTORS;
    input.seek(SeekFrom::Start(start*2352)).map_err(|e|e.to_string())?;
    let mut sector=[0u8;2352];
    for _ in 0..UNINDEXED_AUDIO_PREGAP_SECTORS{
        input.read_exact(&mut sector).map_err(|e|format!("Truncated BIN pregap: {e}"))?;
        if sector.iter().any(|&byte|byte!=0){return Ok(None);}
    }
    Ok(Some(start))
}

pub fn normalize(cue:&Path, temp_dir:&Path)->Result<TempIso,String>{
    normalize_with_data(cue,temp_dir,None)
}
/// Read-only disc sources, optional explicitly selected BIN/IMG data track
/// for unsupported-CD diagnostics. No rewrite of the supplied CUE.
pub fn normalize_with_data(cue:&Path, temp_dir:&Path,
                           data_override:Option<&Path>)->Result<TempIso,String>{
    let tracks=tracks(cue)?;
    let data=&tracks[0];
    let (sector_size,start)=match data.mode.as_str(){
        "MODE1/2048"=>(2048usize,0usize),
        "MODE1/2352"=>(2352usize,16usize),
        "MODE2/2352"=>(2352usize,24usize),
        _=>return Err("First CUE track is not a supported CD-ROM data track".into()),
    };
    let bin=if let Some(path)=data_override {
        if !path.extension().is_some_and(|e|e.eq_ignore_ascii_case("bin") || e.eq_ignore_ascii_case("img")){
            return Err("Explicit data track must be BIN or IMG".into());
        }
        let m=fs::symlink_metadata(path).map_err(|e|e.to_string())?;
        if !m.is_file() || m.file_type().is_symlink() {
            return Err("Explicit BIN/IMG is not a regular file".into());
        }
        path.to_path_buf()
    }else{
        bin_path(cue,&data.file)?.ok_or_else(||format!("Data BIN missing: {}",data.file))?
    };
    let mut input=File::open(&bin).map_err(|e|e.to_string())?;
    let len=input.metadata().map_err(|e|e.to_string())?.len();
    if len==0||len>8*1024*1024*1024||len%sector_size as u64!=0{
        return Err("BIN size is invalid or does not align to sector size".into());
    }
    let total=len/sector_size as u64;
    let mut end=track_end(&tracks,0,total)?;
    if let Some(inferred)=infer_unindexed_zero_audio_pregap(&mut input,&tracks,sector_size,total)?{
        if inferred>=end || inferred<=data.index01 {
            return Err("Invalid inferred CUE data-track boundary".into());
        }
        end=inferred;
        crate::show(&format!(
            "CUE_UNINDEXED_PREGAP inferred=1 following_audio_track={:02} zero_sectors={} boundary_sector={} original_index01_sector={} source_bin_unchanged=1",
            tracks[1].number,UNINDEXED_AUDIO_PREGAP_SECTORS,end,tracks[1].index01
        ),temp_dir);
    }
    let output=temp_dir.join("CyberGladiatorsFix-disc.tmp.iso");
    let mut out=OpenOptions::new().create_new(true).write(true).open(&output)
        .map_err(|e|format!("Temporary ISO already exists or cannot be created ({}): {e}",output.display()))?;
    let result=(||->Result<(),String>{
        input.seek(SeekFrom::Start(data.index01*sector_size as u64)).map_err(|e|e.to_string())?;
        let mut sector=vec![0u8;sector_size];
        for _ in data.index01..end {
            input.read_exact(&mut sector).map_err(|e|format!("Truncated BIN: {e}"))?;
            if sector_size==2352 {
                if sector[..12]!=[0,255,255,255,255,255,255,255,255,255,255,0]{
                    return Err("Invalid raw CD sector sync pattern".into());
                }
                let mode=if start==16 {1}else{2};
                if sector[15]!=mode {return Err("Mixed/invalid CD sector mode".into());}
                if mode==2 && sector[18]&0x20!=0{return Err("MODE2 Form2 sectors cannot be read as ISO data".into());}
            }
            out.write_all(&sector[start..start+2048]).map_err(|e|e.to_string())?;
        }
        out.flush().map_err(|e|e.to_string())?;
        Ok(())
    })();
    if let Err(err)=result{drop(out);let _=fs::remove_file(&output);return Err(err);}
    Ok(TempIso(output))
}

#[cfg(test)]
mod tests{use super::*;
    #[test] fn timestamp(){assert_eq!(msf("00:02:00").unwrap(),150);assert!(msf("00:60:00").is_err());}
    /// Representative metadata from the supplied French 15-track CUE.
    /// The raw BIN/game files are not included in the repository.
    #[test]
    fn french_single_bin_15_track_layout_preserves_audio_boundaries(){
        let dir=std::env::temp_dir().join(format!(
            "cyber-fr-cue-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .unwrap().as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        let cue=dir.join("CYBERG.CUE");
        let layout=r#"FILE "CYBERG.BIN" BINARY
  TRACK 01 MODE1/2352
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    PREGAP 00:02:00
    INDEX 01 07:09:65
  TRACK 03 AUDIO
    INDEX 00 08:29:68
    INDEX 01 08:31:68
  TRACK 04 AUDIO
    INDEX 00 09:51:71
    INDEX 01 09:53:71
  TRACK 05 AUDIO
    INDEX 00 11:13:74
    INDEX 01 11:15:74
  TRACK 06 AUDIO
    INDEX 00 12:35:00
    INDEX 01 12:37:00
  TRACK 07 AUDIO
    INDEX 00 14:06:44
    INDEX 01 14:08:44
  TRACK 08 AUDIO
    INDEX 00 15:28:47
    INDEX 01 15:30:47
  TRACK 09 AUDIO
    INDEX 00 16:50:51
    INDEX 01 16:52:50
  TRACK 10 AUDIO
    INDEX 00 18:12:53
    INDEX 01 18:14:53
  TRACK 11 AUDIO
    INDEX 00 19:34:58
    INDEX 01 19:36:58
  TRACK 12 AUDIO
    INDEX 00 20:56:61
    INDEX 01 20:58:61
  TRACK 13 AUDIO
    INDEX 00 23:50:67
    INDEX 01 23:52:67
  TRACK 14 AUDIO
    INDEX 00 26:53:38
    INDEX 01 26:55:38
  TRACK 15 AUDIO
    INDEX 00 28:15:42
    INDEX 01 28:17:42
"#;
        fs::write(&cue,layout).unwrap();
        let tracks=tracks(&cue).unwrap();
        assert_eq!(tracks.len(),15);
        assert_eq!(tracks[0].number,1);
        assert_eq!(tracks[0].mode,"MODE1/2352");
        assert!(tracks.iter().all(|track|track.file=="CYBERG.BIN"));
        assert!(tracks[1..].iter().all(|track|track.mode=="AUDIO"));
        assert_eq!(tracks[1].index00,None); // virtual PREGAP is not part of BIN
        assert_eq!(track_end(&tracks,0,130_000).unwrap(),32_240);
        assert_eq!(tracks[2].index00,Some(38_243));
        assert_eq!(track_end(&tracks,1,130_000).unwrap(),38_243);
        assert_eq!(tracks[14].index01,127_317);
        assert_eq!(track_end(&tracks,14,130_000).unwrap(),130_000);
        // Parsing a cue sheet never needs to open or alter the proprietary BIN.
        assert!(bin_path(&cue,"CYBERG.BIN").unwrap().is_none());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test] fn cue_file(){assert_eq!(file_record("\"Disc 1.bin\" BINARY").unwrap(),"Disc 1.bin");}
    #[test]
    fn omitted_audio_tracks_do_not_block_first_data_track(){
        let root=std::env::temp_dir().join(format!("cyber-cue-no-audio-{}",std::process::id()));
        let _=fs::remove_dir_all(&root);fs::create_dir(&root).unwrap();
        let cue=root.join("disc.cue");
        fs::write(&cue,concat!(
            "FILE \"data.bin\" BINARY\n",
            "  TRACK 01 MODE1/2352\n",
            "    INDEX 01 00:00:00\n",
            "FILE \"missing-audio-track.bin\" BINARY\n",
            "  TRACK 02 AUDIO\n",
            "    INDEX 00 00:00:00\n",
            "    INDEX 01 00:02:00\n",
        )).unwrap();
        let mut sector=vec![0u8;2352];
        sector[..12].copy_from_slice(&[0,255,255,255,255,255,255,255,255,255,255,0]);
        sector[15]=1;
        sector[16..16+2048].fill(0x5a);
        fs::write(root.join("data.bin"),sector).unwrap();
        let output=normalize(&cue,&root).expect("Missing audio must not block data extraction");
        assert_eq!(fs::read(&output.0).unwrap(),vec![0x5a;2048]);
        drop(output);fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn cue_with_multiple_separate_binary_tracks() {
        let root=std::env::temp_dir().join(format!("cyber-multibin-{}",std::process::id()));
        let _=fs::remove_dir_all(&root);fs::create_dir(&root).unwrap();
        let cue=root.join("multi.cue");
        fs::write(&cue, r#"FILE "Track 01.bin" BINARY
 TRACK 01 MODE1/2352
 INDEX 01 00:00:00
FILE "Track 02.bin" BINARY
 TRACK 02 AUDIO
 INDEX 00 00:00:00
 INDEX 01 00:00:01
FILE "Track 03.bin" BINARY
 TRACK 03 AUDIO
 INDEX 01 00:00:00
"#).unwrap();
        let parsed=tracks(&cue).unwrap();
        assert_eq!(parsed.len(),3);
        assert_eq!(parsed[0].mode,"MODE1/2352");
        assert_eq!(parsed[1].mode,"AUDIO");
        assert_eq!(parsed[2].file,"Track 03.bin");
        assert!(bin_path(&cue,&parsed[1].file).unwrap().is_none());
        let mut data=vec![0u8;2352*2];
        for sector in data.chunks_exact_mut(2352) {
            sector[..12].copy_from_slice(&[0,255,255,255,255,255,255,255,255,255,255,0]);
            sector[15]=1;
        }
        data[16..16+2048].fill(0x5a);
        fs::write(root.join("Track 01.bin"),data).unwrap();
        let output=normalize(&cue,&root).unwrap();
        assert_eq!(fs::metadata(&output.0).unwrap().len(),4096);
        drop(output);
        fs::remove_dir_all(&root).unwrap();
    }
    #[test]
    fn rejects_non_bin_cue_file_reference() {
        assert!(!valid_relative("track.wav"));
        assert!(!valid_relative("disc.iso"));
        assert!(valid_relative("Track 02.BIN"));
    }
    #[test]
    fn unindexed_150_sector_zero_pregap_matches_explicit_index00(){
        let dir=std::env::temp_dir().join(format!(
            "cyber-unindexed-pregap-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .unwrap().as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        let data_sectors=3usize;
        let audio_start=data_sectors+UNINDEXED_AUDIO_PREGAP_SECTORS as usize;
        let mut bin=vec![0u8;(audio_start+2)*2352];
        for (i,sector) in bin.chunks_exact_mut(2352).take(data_sectors).enumerate(){
            sector[..12].copy_from_slice(&[0,255,255,255,255,255,255,255,255,255,255,0]);
            sector[15]=1;
            sector[16..2064].fill(0x41+i as u8);
        }
        bin[audio_start*2352..].fill(0x37);
        let bin_path=dir.join("CYBER.bin");
        fs::write(&bin_path,&bin).unwrap();
        let original_hash=crate::patch::digest(&bin);
        let cue=dir.join("CYBER.cue");
        let original_cue=concat!(
            "FILE \"CYBER.bin\" BINARY\n",
            " TRACK 01 MODE1/2352\n",
            " INDEX 01 00:00:00\n",
            " TRACK 02 AUDIO\n",
            " INDEX 01 00:02:03\n",
        );
        fs::write(&cue,original_cue).unwrap();
        let unindexed=normalize(&cue,&dir).unwrap();
        let normalized=fs::read(&unindexed.0).unwrap();
        assert_eq!(normalized.len(),data_sectors*2048);
        assert_eq!(normalized[0],0x41);
        assert_eq!(normalized[2048],0x42);
        drop(unindexed);
        let explicit_cue=original_cue.replace(
            " TRACK 02 AUDIO\n",
            " TRACK 02 AUDIO\n INDEX 00 00:00:03\n",
        );
        fs::write(&cue,&explicit_cue).unwrap();
        let indexed=normalize(&cue,&dir).unwrap();
        assert_eq!(fs::read(&indexed.0).unwrap(),normalized);
        drop(indexed);
        assert_eq!(crate::patch::digest(&fs::read(&bin_path).unwrap()),original_hash);
        fs::write(&cue,original_cue).unwrap();
        assert_eq!(fs::read_to_string(&cue).unwrap(),original_cue);
        // A nonzero byte in the presumed pause must NOT be skipped.
        bin[(data_sectors+65)*2352+26]=1;
        fs::write(&bin_path,&bin).unwrap();
        assert!(normalize(&cue,&dir).err().expect("malformed pause must be rejected").contains("Invalid raw CD sector sync pattern"));
        fs::remove_dir_all(&dir).unwrap();
    }
    #[test]
    fn unindexed_pregap_cannot_skip_valid_data_or_missing_audio(){
        let mut bytes=vec![0u8;153*2352];
        for sector in bytes.chunks_exact_mut(2352){
            sector[..12].copy_from_slice(&[0,255,255,255,255,255,255,255,255,255,255,0]);
            sector[15]=1;
        }
        let dir=std::env::temp_dir().join(format!("cyber-no-false-pregap-{}",std::process::id()));
        let _=fs::remove_dir_all(&dir);
        fs::create_dir(&dir).unwrap();
        let file=dir.join("data.bin");
        fs::write(&file,&bytes).unwrap();
        let mut input=File::open(&file).unwrap();
        let tracks=vec![
            Track{number:1,file:"data.bin".into(),mode:"MODE1/2352".into(),
                index00:None,index01:0},
            Track{number:2,file:"data.bin".into(),mode:"AUDIO".into(),
                index00:None,index01:153}
        ];
        assert_eq!(infer_unindexed_zero_audio_pregap(&mut input,&tracks,2352,153).unwrap(),None);
        assert_eq!(infer_unindexed_zero_audio_pregap(&mut input,&tracks,2048,153).unwrap(),None);
        let mut different_file=tracks.clone();
        different_file[1].file="audio.bin".into();
        assert_eq!(infer_unindexed_zero_audio_pregap(&mut input,&different_file,2352,153).unwrap(),None);
        let mut indexed=tracks.clone();
        indexed[1].index00=Some(3);
        assert_eq!(infer_unindexed_zero_audio_pregap(&mut input,&indexed,2352,153).unwrap(),None);
        drop(input);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn same_file_boundary_uses_next_index00(){
        let tracks=vec![
            Track{number:1,file:"cd.bin".into(),mode:"MODE1/2352".into(),index00:None,index01:0},
            Track{number:2,file:"cd.bin".into(),mode:"AUDIO".into(),index00:Some(100),index01:250},
        ];
        assert_eq!(track_end(&tracks,0,400).unwrap(),100);
        assert_eq!(track_end(&tracks,1,400).unwrap(),400);
    }
}
