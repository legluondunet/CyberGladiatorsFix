// SPDX-License-Identifier: GPL-3.0-or-later
//! Pure in-memory model for a future French DirectDraw-v1 compatibility
//! layer. NOT connected to Win32 COM calls or enabled in gameplay yet.
//! It cannot change Windows display settings: this module has no Win32 API.
//!
//! The retail game requests DDSCL_EXCLUSIVE|DDSCL_FULLSCREEN, then
//! SetDisplayMode(640,480,16), and likely creates a 16-bpp primary
//! surface with a single flipping backbuffer. Native DDSCL_NORMAL alone
//! cannot provide that chain; it must be virtualized, not faked as DD_OK.
#![allow(dead_code)]

use std::mem;

const MAX_DIMENSION:u32=4096;
const MAX_PIXELS:u64=4096*4096;
const LEGACY_DDSCL_EXCLUSIVE_FULLSCREEN:u32=0x11;
const DDSD_CAPS_BACKBUFFER_COUNT:u32=0x21;
const DDSCAPS_PRIMARY_FLIP_COMPLEX:u32=0x218;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Mode{
    pub width:u32,
    pub height:u32,
    pub bpp:u32,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Rect{
    pub x:u32,
    pub y:u32,
    pub width:u32,
    pub height:u32,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Error{
    UnsupportedCooperativeMode,
    UnsupportedMode,
    InvalidSurface,
    PrimaryNotCreated,
    BufferOutOfBounds,
}
/// Game-facing and host-facing sizes are intentionally different.
/// The physical desktop is supplied by the host and never mutated here.
pub struct VirtualDevice{
    host_width:u32,
    host_height:u32,
    logical_mode:Option<Mode>,
    cooperative_requested:bool,
    front:Option<Vec<u16>>,
    back:Option<Vec<u16>>,
    flips:u64,
}
impl VirtualDevice{
    pub fn new(host_width:u32,host_height:u32)->Result<Self,Error>{
        if host_width==0||host_height==0||host_width>16384||host_height>16384{
            return Err(Error::UnsupportedMode);
        }
        Ok(Self{host_width,host_height,logical_mode:None,
            cooperative_requested:false,front:None,back:None,flips:0})
    }
    /// This *records* what the game asked for. A future COM adapter will
    /// expose success only when the equivalent virtual chain exists, while
    /// setting the *real* DirectDraw provider to non-exclusive cooperation.
    pub fn request_exclusive_cooperation(&mut self,flags:u32)->Result<(),Error>{
        if flags!=LEGACY_DDSCL_EXCLUSIVE_FULLSCREEN{
            return Err(Error::UnsupportedCooperativeMode);
        }
        self.cooperative_requested=true;
        Ok(())
    }
    pub fn request_mode(&mut self,mode:Mode)->Result<(),Error>{
        if !self.cooperative_requested || mode.bpp!=16
            ||mode.width==0||mode.height==0
            ||mode.width>MAX_DIMENSION||mode.height>MAX_DIMENSION
            ||u64::from(mode.width)*u64::from(mode.height)>MAX_PIXELS {
            return Err(Error::UnsupportedMode);
        }
        self.front=None;
        self.back=None;
        self.logical_mode=Some(mode);
        self.flips=0;
        Ok(())
    }
    pub fn create_primary_flip_chain(
        &mut self,desc_flags:u32,caps:u32,backbuffers:u32
    )->Result<(),Error>{
        if desc_flags!=DDSD_CAPS_BACKBUFFER_COUNT
            ||caps!=DDSCAPS_PRIMARY_FLIP_COMPLEX||backbuffers!=1 {
            return Err(Error::InvalidSurface);
        }
        let mode=self.logical_mode.ok_or(Error::UnsupportedMode)?;
        let count=(u64::from(mode.width)*u64::from(mode.height)) as usize;
        self.front=Some(vec![0;count]);
        self.back=Some(vec![0;count]);
        self.flips=0;
        Ok(())
    }
    pub fn mode(&self)->Option<Mode>{self.logical_mode}
    pub fn host_size(&self)->(u32,u32){(self.host_width,self.host_height)}
    pub fn flip_count(&self)->u64{self.flips}
    pub fn pitch_bytes(&self)->Result<u32,Error>{
        self.logical_mode.map(|m|m.width*2).ok_or(Error::UnsupportedMode)
    }
    /// Game writes 16-bit pixel *values* into an emulated surface; color masks
    /// remain untouched until a later presentation stage negotiates format.
    pub fn write_back(&mut self,x:u32,y:u32,value:u16)->Result<(),Error>{
        let mode=self.logical_mode.ok_or(Error::UnsupportedMode)?;
        if x>=mode.width||y>=mode.height{return Err(Error::BufferOutOfBounds);}
        let back=self.back.as_mut().ok_or(Error::PrimaryNotCreated)?;
        let idx=y as usize*mode.width as usize+x as usize;
        back[idx]=value;
        Ok(())
    }
    pub fn read_front(&self,x:u32,y:u32)->Result<u16,Error>{
        let mode=self.logical_mode.ok_or(Error::UnsupportedMode)?;
        if x>=mode.width||y>=mode.height{return Err(Error::BufferOutOfBounds);}
        let front=self.front.as_ref().ok_or(Error::PrimaryNotCreated)?;
        Ok(front[y as usize*mode.width as usize+x as usize])
    }
    /// Flip is an owned memory swap, not a hardware primary-surface flip.
    /// A later renderer will read the front buffer and present it through
    /// GameFrame/OpenGL, never through an exclusive DirectDraw primary.
    pub fn flip(&mut self)->Result<(),Error>{
        let front=self.front.as_mut().ok_or(Error::PrimaryNotCreated)?;
        let back=self.back.as_mut().ok_or(Error::PrimaryNotCreated)?;
        mem::swap(front,back);
        self.flips+=1;
        Ok(())
    }
    pub fn viewport_4_3(&self)->Rect{
        viewport_4_3(self.host_width,self.host_height)
    }
}
/// Exact integer 4:3 letterbox/pillarbox derived from the current display.
/// No physical resolution, DPI scale, refresh, or depth is hardcoded.
pub fn viewport_4_3(width:u32,height:u32)->Rect{
    if width==0||height==0{return Rect{x:0,y:0,width:0,height:0};}
    let (w,h)=if u64::from(width)*3>=u64::from(height)*4{
        (u64::from(height)*4/3,height)
    }else{
        (width,(u64::from(width)*3/4) as u32)
    };
    let w=w as u32;
    Rect{x:(width-w)/2,y:(height-h)/2,width:w,height:h}
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn logical_16bpp_and_physical_desktop_are_independent(){
        let mut device=VirtualDevice::new(2560,1440).unwrap();
        assert_eq!(device.host_size(),(2560,1440));
        assert_eq!(device.mode(),None);
        assert_eq!(device.request_mode(Mode{width:640,height:480,bpp:16}),
            Err(Error::UnsupportedMode));
        device.request_exclusive_cooperation(0x11).unwrap();
        device.request_mode(Mode{width:640,height:480,bpp:16}).unwrap();
        assert_eq!(device.mode(),Some(Mode{width:640,height:480,bpp:16}));
        assert_eq!(device.host_size(),(2560,1440));
        assert_eq!(device.pitch_bytes(),Ok(1280));
    }
    #[test]
    fn backbuffer_flip_does_not_modify_logical_or_host_mode(){
        let mut device=VirtualDevice::new(1920,1200).unwrap();
        device.request_exclusive_cooperation(0x11).unwrap();
        device.request_mode(Mode{width:640,height:480,bpp:16}).unwrap();
        device.create_primary_flip_chain(0x21,0x218,1).unwrap();
        assert_eq!(device.read_front(7,8),Ok(0));
        device.write_back(7,8,0xf81f).unwrap();
        device.flip().unwrap();
        assert_eq!(device.read_front(7,8),Ok(0xf81f));
        assert_eq!(device.flip_count(),1);
        assert_eq!(device.host_size(),(1920,1200));
        assert_eq!(device.mode().unwrap().bpp,16);
    }
    #[test]
    fn invalid_com_flags_or_surface_request_fail_closed(){
        let mut device=VirtualDevice::new(1920,1080).unwrap();
        assert_eq!(device.request_exclusive_cooperation(0x08),
            Err(Error::UnsupportedCooperativeMode));
        device.request_exclusive_cooperation(0x11).unwrap();
        assert_eq!(device.request_mode(Mode{width:640,height:480,bpp:32}),
            Err(Error::UnsupportedMode));
        device.request_mode(Mode{width:640,height:480,bpp:16}).unwrap();
        assert_eq!(device.create_primary_flip_chain(0x21,0x200,1),
            Err(Error::InvalidSurface));
        assert_eq!(device.create_primary_flip_chain(0x21,0x218,2),
            Err(Error::InvalidSurface));
        assert_eq!(device.flip(),Err(Error::PrimaryNotCreated));
    }
    #[test]
    fn viewport_tracks_monitor_resolutions_without_stretching(){
        assert_eq!(viewport_4_3(1920,1200),
            Rect{x:160,y:0,width:1600,height:1200});
        assert_eq!(viewport_4_3(1920,1080),
            Rect{x:240,y:0,width:1440,height:1080});
        assert_eq!(viewport_4_3(2560,1440),
            Rect{x:320,y:0,width:1920,height:1440});
        assert_eq!(viewport_4_3(1024,768),
            Rect{x:0,y:0,width:1024,height:768});
        assert_eq!(viewport_4_3(1200,1920),
            Rect{x:0,y:510,width:1200,height:900});
    }
    #[test]
    fn geometry_and_buffer_access_are_bounded(){
        assert!(matches!(VirtualDevice::new(0,1080),Err(Error::UnsupportedMode)));
        let mut device=VirtualDevice::new(1536,960).unwrap();
        device.request_exclusive_cooperation(0x11).unwrap();
        device.request_mode(Mode{width:640,height:480,bpp:16}).unwrap();
        device.create_primary_flip_chain(0x21,0x218,1).unwrap();
        assert_eq!(device.write_back(640,0,1),Err(Error::BufferOutOfBounds));
        assert_eq!(device.read_front(0,480),Err(Error::BufferOutOfBounds));
    }
}
