// SPDX-License-Identifier: GPL-3.0-or-later
//! FLAC -> 44.1-kHz stereo PCM -> WinMM waveOut, driven by MCI CD commands.
//! Decoding and waveOut operations run on a private worker, never inside
//! the game thread or DllMain. CD track numbering includes data Track 01.
use claxon::FlacReader;
use std::ffi::c_void;
use std::fs::{self,OpenOptions};
use std::io::Write;
use std::path::{Path,PathBuf};
use std::sync::{mpsc,Arc};
use std::sync::atomic::{AtomicU32,Ordering};
use std::time::Duration;

const RATE:u32=44_100;
const CHANNELS:usize=2;
const WHDR_DONE:u32=1;
const WAVE_MAPPER:u32=0xffff_ffff;
const CHUNK_FRAMES:usize=RATE as usize / 2;
const MAX_QUEUED:usize=4;
const MAX_SAMPLES:usize=RATE as usize * CHANNELS * 60 * 12;
const MCI_MODE_STOP:u32=525;
const MCI_MODE_PLAY:u32=526;
const MCI_MODE_PAUSE:u32=529;

#[repr(C)]
struct WaveFormat {
    format_tag:u16, channels:u16, samples_per_sec:u32, avg_bytes_per_sec:u32,
    block_align:u16, bits_per_sample:u16, extra_size:u16,
}
#[repr(C)]
struct WaveHeader{
    data:*mut i8,buffer_bytes:u32,bytes_recorded:u32,user:usize,
    flags:u32,loops:u32,next:*mut WaveHeader,reserved:usize,
}
#[repr(C)]
struct MmTime {kind:u32,value:[u8;8]}

#[link(name="winmm")]
extern "system" {
    #[link_name="waveOutOpen"] fn wave_out_open(out:*mut *mut c_void,id:u32,format:*const WaveFormat,
       callback:usize,instance:usize,flags:u32)->u32;
    #[link_name="waveOutClose"] fn wave_out_close(handle:*mut c_void)->u32;
    #[link_name="waveOutPrepareHeader"] fn wave_out_prepare_header(handle:*mut c_void,
       header:*mut WaveHeader,bytes:u32)->u32;
    #[link_name="waveOutUnprepareHeader"] fn wave_out_unprepare_header(handle:*mut c_void,
       header:*mut WaveHeader,bytes:u32)->u32;
    #[link_name="waveOutWrite"] fn wave_out_write(handle:*mut c_void,
       header:*mut WaveHeader,bytes:u32)->u32;
    #[link_name="waveOutReset"] fn wave_out_reset(handle:*mut c_void)->u32;
    #[link_name="waveOutPause"] fn wave_out_pause(handle:*mut c_void)->u32;
    #[link_name="waveOutRestart"] fn wave_out_restart(handle:*mut c_void)->u32;
    #[link_name="waveOutGetPosition"] fn wave_out_get_position(handle:*mut c_void,
       time:*mut MmTime,size:u32)->u32;
}
fn log(root:&Path,s:&str){
    if let Ok(mut f)=OpenOptions::new().create(true).append(true)
        .open(root.join("CyberGladiatorsCDDA.log")){let _=writeln!(f,"{s}");}
}
pub struct Shared {
    pub mode:AtomicU32,
    pub track:AtomicU32,
    pub frames:AtomicU32,
}
pub enum Command{
    Play{track:u8,to:Option<u32>},Stop,Pause,Resume,Shutdown,
}

fn decode_track(path:&Path)->Result<Vec<i16>,String>{
    let mut flac=FlacReader::open(path).map_err(|e|format!("Open FLAC: {e}"))?;
    let info=flac.streaminfo();
    if info.sample_rate!=RATE||info.channels!=CHANNELS as u32||info.bits_per_sample!=16{
        return Err(format!("Unsupported FLAC: {} Hz, {} channels, {} bits",
            info.sample_rate,info.channels,info.bits_per_sample));
    }
    if info.samples.is_some_and(|count|count > (MAX_SAMPLES/CHANNELS) as u64){
        return Err("CDDA track exceeds 12-minute safety limit".into());
    }
    let mut pcm=Vec::new();
    for sample in flac.samples() {
        if pcm.len()>=MAX_SAMPLES {
            return Err("Decoded FLAC exceeds 12-minute safety limit".into());
        }
        let sample=sample.map_err(|e|format!("FLAC decode: {e}"))?;
        if !(-32768..=32767).contains(&sample) {
            return Err("FLAC samples not PCM 16-bit".into());
        }
        pcm.push(sample as i16);
    }
    if pcm.is_empty()||pcm.len()%CHANNELS!=0{return Err("Empty or truncated FLAC".into());}
    Ok(pcm)
}
struct Chunk{
    header:Box<WaveHeader>,
}
struct AudioTrack{
    device:*mut c_void,
    pcm:Vec<i16>,
    next_sample:usize,
    pending:Vec<Chunk>,
    paused:bool,
}
impl AudioTrack{
    fn open(path:&Path)->Result<Self,String>{
        let pcm=decode_track(path)?;
        let format=WaveFormat{
            format_tag:1,channels:2,samples_per_sec:RATE,
            avg_bytes_per_sec:RATE*CHANNELS as u32*2,
            block_align:4,bits_per_sample:16,extra_size:0,
        };
        let mut device=std::ptr::null_mut();
        let err=unsafe{wave_out_open(&mut device,WAVE_MAPPER,&format,0,0,0)};
        if err!=0||device.is_null(){
            return Err(format!("waveOutOpen failed (MMRESULT {err})"));
        }
        Ok(Self{device,pcm,next_sample:0,pending:Vec::new(),paused:false})
    }
    fn pump(&mut self)->Result<bool,String>{
        // WinMM sets WHDR_DONE asynchronously. Read the WinMM-owned field
        // with a volatile load, and unprepare only completed buffers.
        let mut i=0;
        while i<self.pending.len(){
            let hdr=self.pending[i].header.as_mut();
            let flags=unsafe{std::ptr::read_volatile(&hdr.flags)};
            if flags&WHDR_DONE!=0{
                let err=unsafe{wave_out_unprepare_header(self.device,hdr,
                    std::mem::size_of::<WaveHeader>() as u32)};
                if err!=0{return Err(format!("waveOutUnprepareHeader failed: {err}"));}
                self.pending.swap_remove(i);
            }else{i+=1;}
        }
        while !self.paused && self.pending.len()<MAX_QUEUED && self.next_sample<self.pcm.len(){
            let left=self.pcm.len()-self.next_sample;
            let len=left.min(CHUNK_FRAMES*CHANNELS);
            let data=unsafe{self.pcm.as_mut_ptr().add(self.next_sample)} as *mut i8;
            let mut chunk=Chunk{header:Box::new(WaveHeader{
                data,buffer_bytes:(len*2) as u32,bytes_recorded:0,user:0,
                flags:0,loops:0,next:std::ptr::null_mut(),reserved:0,
            })};
            let ptr=chunk.header.as_mut() as *mut WaveHeader;
            let hdrsize=std::mem::size_of::<WaveHeader>() as u32;
            let err=unsafe{wave_out_prepare_header(self.device,ptr,hdrsize)};
            if err!=0{return Err(format!("waveOutPrepareHeader failed: {err}"));}
            // Retain the prepared header until the driver has completed it.
            self.pending.push(chunk);
            let err=unsafe{wave_out_write(self.device,ptr,hdrsize)};
            if err!=0{return Err(format!("waveOutWrite failed: {err}"));}
            self.next_sample+=len;
        }
        Ok(self.next_sample==self.pcm.len() && self.pending.is_empty())
    }
    fn frames_played(&self)->u32{
        let mut t=MmTime{kind:2,value:[0;8]}; // TIME_SAMPLES
        let ok=unsafe{wave_out_get_position(self.device,&mut t,
            std::mem::size_of::<MmTime>() as u32)};
        if ok==0 && t.kind==2 {
            u32::from_le_bytes(t.value[..4].try_into().unwrap())
        }else{0}
    }
    fn pause(&mut self){if !self.paused{unsafe{wave_out_pause(self.device);}self.paused=true;}}
    fn resume(&mut self){if self.paused{unsafe{wave_out_restart(self.device);}self.paused=false;}}
}
impl Drop for AudioTrack{
    fn drop(&mut self){
        unsafe{
            wave_out_reset(self.device);
            let size=std::mem::size_of::<WaveHeader>() as u32;
            for chunk in &mut self.pending {
                let _=wave_out_unprepare_header(self.device,chunk.header.as_mut(),size);
            }
            self.pending.clear();
            let _=wave_out_close(self.device);
        }
    }
}
pub fn run(rx:mpsc::Receiver<Command>,shared:Arc<Shared>,root:PathBuf){
    let mut playing:Option<AudioTrack>=None;
    loop {
        match rx.recv_timeout(Duration::from_millis(25)){
            Ok(Command::Play{track,to})=>{
                playing=None; // calls waveOutReset and closes the old stream.
                let path=root.join(format!("Track{track:02}.flac"));
                match AudioTrack::open(&path){
                    Ok(mut audio)=>{
                        if let Err(err)=audio.pump(){
                            log(&root,&format!("Track{track:02}: playback initialization error: {err}"));
                            shared.mode.store(MCI_MODE_STOP,Ordering::Relaxed);
                        }else{
                            log(&root,&format!("Track{track:02}: decoded {} PCM frames, waveOut started, MCI_TO={to:?}",
                                audio.pcm.len()/CHANNELS));
                            shared.track.store(track as u32,Ordering::Relaxed);
                            shared.frames.store(0,Ordering::Relaxed);
                            shared.mode.store(MCI_MODE_PLAY,Ordering::Relaxed);
                            playing=Some(audio);
                        }
                    }
                    Err(err)=>{
                        log(&root,&format!("Track{track:02}: {err}"));
                        shared.mode.store(MCI_MODE_STOP,Ordering::Relaxed);
                    }
                }
            }
            Ok(Command::Stop)=>{
                playing=None;
                shared.mode.store(MCI_MODE_STOP,Ordering::Relaxed);
                shared.frames.store(0,Ordering::Relaxed);
            }
            Ok(Command::Pause)=>{
                if let Some(ref mut a)=playing {a.pause();}
                shared.mode.store(MCI_MODE_PAUSE,Ordering::Relaxed);
            }
            Ok(Command::Resume)=>{
                if let Some(ref mut a)=playing {a.resume();}
                shared.mode.store(if playing.is_some(){MCI_MODE_PLAY}else{MCI_MODE_STOP},Ordering::Relaxed);
            }
            Ok(Command::Shutdown)|Err(mpsc::RecvTimeoutError::Disconnected)=>{
                playing=None;
                log(&root,"MCI_CLOSE: CDDA player shut down");
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout)=>{}
        }
        if let Some(ref mut a)=playing{
            shared.frames.store(a.frames_played(),Ordering::Relaxed);
            match a.pump(){
                Ok(true)=>{
                    playing=None;
                    shared.mode.store(MCI_MODE_STOP,Ordering::Relaxed);
                    log(&root,"CDDA track finished");
                }
                Ok(false)=>{}
                Err(err)=>{
                    playing=None;
                    shared.mode.store(MCI_MODE_STOP,Ordering::Relaxed);
                    log(&root,&format!("waveOut stream error: {err}"));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn waveout_structs_match_win32_32bit_abi(){
        assert_eq!(std::mem::size_of::<WaveFormat>(),20);
        assert_eq!(std::mem::size_of::<WaveHeader>(),32);
        assert_eq!(std::mem::size_of::<MmTime>(),12);
    }
    #[test]
    fn pcm_timing_is_cd_quality(){
        assert_eq!(RATE,44100);
        assert_eq!(CHANNELS,2);
        assert_eq!(CHUNK_FRAMES,22050);
        assert_eq!(MAX_QUEUED,4);
    }
}
