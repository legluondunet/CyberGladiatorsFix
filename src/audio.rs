// SPDX-License-Identifier: GPL-3.0-or-later
//! Lossless CDDA extraction (44,100 Hz, 16-bit little-endian stereo) into
//! TrackNN.flac. No resampling, gain change, normalization or external encoder.
//! Only INDEX 01 through the next track's INDEX 00 (or INDEX 01) is exported.
use crate::cue;
use flacenc::{bitsink::ByteSink,component::BitRepr,error::{SourceError,Verify},source::{Fill,Source}};
use sha2::{Digest,Sha256};
use std::fs::{File,OpenOptions};
#[cfg(test)] use std::fs;
use std::io::{Read,Seek,SeekFrom,Write};
use std::path::Path;

const SECTOR_BYTES:u64=2352;
const BYTES_PER_STEREO_SAMPLE:u64=4;
const SAMPLE_RATE:usize=44_100;
const MAX_BIN:u64=8*1024*1024*1024;

/// Stream samples directly from a bounded BIN range to flacenc. Audio source
/// memory is limited to a single encode block, not an entire CD track.
struct AudioSource {file:File,remaining_frames:u64}
impl Source for AudioSource {
    fn channels(&self)->usize{2}
    fn bits_per_sample(&self)->usize{16}
    fn sample_rate(&self)->usize{SAMPLE_RATE}
    fn len_hint(&self)->Option<usize>{usize::try_from(self.remaining_frames).ok()}
    fn read_samples<F:Fill>(&mut self,block_size:usize,dest:&mut F)->Result<usize,SourceError>{
        let frames=self.remaining_frames.min(block_size as u64) as usize;
        if frames==0 {return Ok(0);}
        let mut bytes=vec![0u8;frames*BYTES_PER_STEREO_SAMPLE as usize];
        self.file.read_exact(&mut bytes).map_err(SourceError::from_io_error)?;
        dest.fill_le_bytes(&bytes,2)?;
        self.remaining_frames-=frames as u64;
        Ok(frames)
    }
}

pub struct AudioReport {
    pub extracted:usize,
    pub missing:Vec<u8>,
    pub manifest_rows:String,
}
fn encode(file:File,sector_start:u64,sector_end:u64,destination:&Path)->Result<(u64,String),String>{
    if sector_end<=sector_start{return Err("Empty CD audio track".into());}
    let mut f=file;
    f.seek(SeekFrom::Start(sector_start*SECTOR_BYTES)).map_err(|e|e.to_string())?;
    let frames=(sector_end-sector_start)*SECTOR_BYTES/BYTES_PER_STEREO_SAMPLE;
    let src=AudioSource{file:f,remaining_frames:frames};
    let conf=flacenc::config::Encoder::default().into_verified()
        .map_err(|e|format!("FLAC encoder configuration: {e:?}"))?;
    let stream=flacenc::encode_with_fixed_block_size(&conf,src,conf.block_size)
        .map_err(|e|format!("Lossless FLAC encoding failed: {e}"))?;
    let mut sink=ByteSink::new();
    stream.write(&mut sink).map_err(|e|format!("Cannot serialize FLAC stream: {e}"))?;
    let mut output=OpenOptions::new().create_new(true).write(true).open(destination)
        .map_err(|e|format!("Cannot create {}: {e}",destination.display()))?;
    output.write_all(sink.as_slice()).map_err(|e|e.to_string())?;
    output.flush().map_err(|e|e.to_string())?;
    let hash=format!("{:x}",Sha256::digest(sink.as_slice()));
    Ok((sink.as_slice().len() as u64,hash))
}

/// Extract every available AUDIO track. A missing BIN is reported, not a
/// failure: users can own a data-only dump and still install the game.
/// Invalid or truncated BIN files are errors, not silently ignored.
pub fn extract(cue_path:&Path,staged:&Path)->Result<AudioReport,String>{
    let tracks=cue::tracks(cue_path)?;
    let mut report=AudioReport{extracted:0,missing:Vec::new(),manifest_rows:String::new()};
    for (position,t) in tracks.iter().enumerate(){
        if t.mode!="AUDIO"{continue;}
        let filename=format!("Track{:02}.flac",t.number);
        let output=staged.join(&filename);
        if output.exists()||output.is_symlink(){
            return Err(format!("Audio destination already exists: {}",output.display()));
        }
        let Some(bin)=cue::bin_path(cue_path,&t.file)? else {
            report.missing.push(t.number);
            eprintln!("WARNING: track {:02} omitted: missing {}",t.number,t.file);
            continue;
        };
        let f=File::open(&bin).map_err(|e|e.to_string())?;
        let len=f.metadata().map_err(|e|e.to_string())?.len();
        if len==0||len>MAX_BIN||len%SECTOR_BYTES!=0 {
            return Err(format!("Invalid AUDIO BIN sector alignment: {}",bin.display()));
        }
        let end=cue::track_end(&tracks,position,len/SECTOR_BYTES)?;
        let (size,hash)=encode(f,t.index01,end,&output)?;
        report.manifest_rows.push_str(&format!("{filename}\t{size}\t{hash}\n"));
        report.extracted+=1;
        println!("Extracted {filename} (FLAC lossless, {size} bytes)");
    }
    Ok(report)
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn omitted_tracks_are_reported_without_creating_empty_flac(){
        let base=std::env::temp_dir().join(format!("cyber-audio-missing-{}",std::process::id()));
        let _=fs::remove_dir_all(&base);
        fs::create_dir(&base).unwrap();
        fs::write(base.join("test.cue"),concat!(
            "FILE \"missing.bin\" BINARY\n",
            " TRACK 02 AUDIO\n",
            " INDEX 01 00:00:00\n",
        )).unwrap();
        let r=extract(&base.join("test.cue"),&base).unwrap();
        assert_eq!(r.extracted,0);assert_eq!(r.missing,vec![2]);
        assert!(!base.join("Track02.flac").exists());
        fs::remove_dir_all(&base).unwrap();
    }
    #[test]
    fn raw_pcm_round_trip_is_bit_exact(){
        // Synthetic 2352-byte Red Book sectors, including a one-sector pregap.
        let base=std::env::temp_dir().join(format!("cyber-audio-roundtrip-{}",std::process::id()));
        let _=fs::remove_dir_all(&base);fs::create_dir(&base).unwrap();
        let mut pcm=vec![0u8;2352*3];
        for i in 0..(pcm.len()/2){
            let value=((i as i32*997)%65536-32768) as i16;
            pcm[i*2..i*2+2].copy_from_slice(&value.to_le_bytes());
        }
        fs::write(base.join("audio.bin"),&pcm).unwrap();
        fs::write(base.join("test.cue"),concat!(
            "FILE \"audio.bin\" BINARY\n",
            " TRACK 02 AUDIO\n",
            " INDEX 00 00:00:00\n",
            " INDEX 01 00:00:01\n",
        )).unwrap();
        let r=extract(&base.join("test.cue"),&base).unwrap();
        assert_eq!(r.extracted,1);
        let output=File::open(base.join("Track02.flac")).unwrap();
        let mut reader=claxon::FlacReader::new(output).unwrap();
        assert_eq!(reader.streaminfo().sample_rate,44_100);
        assert_eq!(reader.streaminfo().channels,2);
        assert_eq!(reader.streaminfo().bits_per_sample,16);
        let restored:Vec<i32>=reader.samples().map(|s|s.unwrap()).collect();
        let expected:Vec<i32>=pcm[2352..].chunks_exact(2)
            .map(|s|i16::from_le_bytes([s[0],s[1]]) as i32).collect();
        assert_eq!(restored,expected);
        fs::remove_dir_all(&base).unwrap();
    }
    #[test]
    fn extract_two_audio_tracks_from_one_multitrack_bin() {
        let base=std::env::temp_dir().join(format!("cyber-audio-shared-{}",std::process::id()));
        let _=fs::remove_dir_all(&base);fs::create_dir(&base).unwrap();
        fs::write(base.join("disc.cue"),r#"FILE "all.bin" BINARY
 TRACK 01 MODE1/2352
 INDEX 01 00:00:00
 TRACK 02 AUDIO
 INDEX 00 00:00:02
 INDEX 01 00:00:03
 TRACK 03 AUDIO
 INDEX 00 00:00:05
 INDEX 01 00:00:06
"#).unwrap();
        let mut data=vec![0u8;2352*8];
        for sector in data[..2352*2].chunks_exact_mut(2352){
            sector[..12].copy_from_slice(&[0,255,255,255,255,255,255,255,255,255,255,0]);
            sector[15]=1;
        }
        // pregaps in sectors 2 and 5 intentionally carry arbitrary values;
        // tracks 02 and 03 contain distinctive constant PCM samples.
        data[2352*2..2352*3].fill(0x7d);
        data[2352*5..2352*6].fill(0x2d);
        for (start,end,sample) in [(3,5,1200i16),(6,8,-1700i16)] {
            for frame in data[start*2352..end*2352].chunks_exact_mut(2) {
                frame.copy_from_slice(&sample.to_le_bytes());
            }
        }
        fs::write(base.join("all.bin"),data).unwrap();
        // The CUE is the only input; each output must have exactly two
        // sectors (2 * 588 stereo frames), excluding pregaps.
        let report=extract(&base.join("disc.cue"),&base).unwrap();
        assert_eq!(report.extracted,2);
        assert!(report.missing.is_empty());
        for (track,sample) in [(2,1200i32),(3,-1700i32)] {
            let filename=format!("Track{track:02}.flac");
            assert!(report.manifest_rows.contains(&filename));
            let mut reader=claxon::FlacReader::open(base.join(filename)).unwrap();
            assert_eq!(reader.streaminfo().channels,2);
            assert_eq!(reader.streaminfo().sample_rate,44_100);
            let restored:Vec<i32>=reader.samples().map(|r|r.unwrap()).collect();
            assert_eq!(restored.len(),2*588*2);
            assert!(restored.iter().all(|v|*v==sample));
        }
        fs::remove_dir_all(&base).unwrap();
    }
    #[test]
    fn extract_two_audio_tracks_from_separate_bin_files() {
        let base=std::env::temp_dir().join(format!("cyber-audio-separate-{}",std::process::id()));
        let _=fs::remove_dir_all(&base);fs::create_dir(&base).unwrap();
        fs::write(base.join("disc.cue"),r#"FILE "data.bin" BINARY
 TRACK 01 MODE1/2352
 INDEX 01 00:00:00
FILE "a02.bin" BINARY
 TRACK 02 AUDIO
 INDEX 00 00:00:00
 INDEX 01 00:00:01
FILE "a03.bin" BINARY
 TRACK 03 AUDIO
 INDEX 01 00:00:00
"#).unwrap();
        fs::write(base.join("a02.bin"),vec![0u8;2352*3]).unwrap();
        fs::write(base.join("a03.bin"),vec![0u8;2352*2]).unwrap();
        let report=extract(&base.join("disc.cue"),&base).unwrap();
        assert_eq!(report.extracted,2);
        assert!(report.missing.is_empty());
        for track in [2,3] {
            let name=format!("Track{track:02}.flac");
            let file=base.join(name);
            assert!(file.exists());
            let mut reader=claxon::FlacReader::open(file).unwrap();
            assert_eq!(reader.samples().count(),2*588*2);
        }
        fs::remove_dir_all(&base).unwrap();
    }
    #[test]
    fn shared_file_track_boundary_excludes_next_pregap(){
        let t=vec![
            crate::cue::Track{number:2,file:"all.bin".into(),mode:"AUDIO".into(),index00:None,index01:0},
            crate::cue::Track{number:3,file:"all.bin".into(),mode:"AUDIO".into(),index00:Some(5),index01:7},
        ];
        assert_eq!(cue::track_end(&t,0,10).unwrap(),5);
    }
}
