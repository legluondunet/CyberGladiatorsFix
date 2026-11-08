// SPDX-License-Identifier: GPL-3.0-or-later
//! Opt-in-at-build, automatic main-menu watermark for the verified retail game.
//!
//! This is compiled into the pinned x86 GameFrame wrapper. No game resource,
//! system font, Win32 overlay window, focus ownership or MCI path is modified.
//! The signed version string is child-scoped by the authenticated x64 launcher.
//! A bounded GPU pixel probe recognizes the bright-green Player 1/Player 2
//! captions AND the lower instruction text on the fighter-selection screen.
//! Unrelated games or invalid labels cannot activate this overlay.
use crate::gl::*;
use crate::render::shaders::Program;
use std::ffi::c_void;

pub struct MenuSignature {
    texture:u32,
    pixel_width:i32,
    pixel_height:i32,
    frames:u64,
    hits:u8,
    visible:bool,
    polls:u64,
}
fn glyph(ch:char)->[u8;7] {
    match ch.to_ascii_uppercase() {
        'A'=>[14,17,17,31,17,17,17],
        'B'=>[30,17,17,30,17,17,30],
        'C'=>[14,17,16,16,16,17,14],
        'D'=>[30,17,17,17,17,17,30],
        'E'=>[31,16,16,30,16,16,31],
        'F'=>[31,16,16,30,16,16,16],
        'G'=>[14,17,16,23,17,17,14],
        'H'=>[17,17,17,31,17,17,17],
        'I'=>[31,4,4,4,4,4,31],
        'J'=>[7,2,2,2,18,18,12],
        'K'=>[17,18,20,24,20,18,17],
        'L'=>[16,16,16,16,16,16,31],
        'M'=>[17,27,21,21,17,17,17],
        'N'=>[17,25,21,19,17,17,17],
        'O'=>[14,17,17,17,17,17,14],
        'P'=>[30,17,17,30,16,16,16],
        'Q'=>[14,17,17,17,21,18,13],
        'R'=>[30,17,17,30,20,18,17],
        'S'=>[15,16,16,14,1,1,30],
        'T'=>[31,4,4,4,4,4,4],
        'U'=>[17,17,17,17,17,17,14],
        'V'=>[17,17,17,17,17,10,4],
        'W'=>[17,17,17,21,21,21,10],
        'X'=>[17,17,10,4,10,17,17],
        'Y'=>[17,17,10,4,4,4,4],
        'Z'=>[31,1,2,4,8,16,31],
        '0'=>[14,17,19,21,25,17,14],
        '1'=>[4,12,4,4,4,4,14],
        '2'=>[14,17,1,2,4,8,31],
        '3'=>[30,1,1,14,1,1,30],
        '4'=>[2,6,10,18,31,2,2],
        '5'=>[31,16,16,30,1,1,30],
        '6'=>[14,16,16,30,17,17,14],
        '7'=>[31,1,2,4,8,8,8],
        '8'=>[14,17,17,14,17,17,14],
        '9'=>[14,17,17,15,1,1,14],
        '.'=>[0,0,0,0,0,12,12],
        '-'=>[0,0,0,31,0,0,0],
        ' '=>[0;7],
        _=>[0,0,4,4,4,0,4],
    }
}
fn safe_label(text:&str)->bool{
    text.starts_with("CyberGladiatorsFix v")
        && (26..=56).contains(&text.len())
        && text.bytes().all(|v|v.is_ascii_alphanumeric()||b". -".contains(&v))
        && text.rsplit_once('-').is_some_and(|(_,suffix)|
            suffix.len()==8 && suffix.bytes().all(|v|v.is_ascii_hexdigit()))
}
fn make_sprite(text:&str)->(Vec<u8>,i32,i32){
    // Transparent, minimal 5x7 lettering with one-pixel dark drop-shadow.
    // No card/background/border: the game image remains visible behind it.
    let w=(text.len()*6+1) as i32;
    let h=9i32;
    let mut data=vec![0u8;(w*h*4) as usize];
    // The upstream present shader flips Y; supply bottom-up GL texels.
    let mut pixel=|x:i32,y:i32,color:[u8;4]|{
        let i=(((h-1-y)*w+x)*4) as usize;
        data[i..i+4].copy_from_slice(&color);
    };
    for (i,ch) in text.chars().enumerate(){
        let glyph=glyph(ch);
        for (y,row) in glyph.iter().enumerate(){
            for x in 0..5{
                if (row >> (4-x))&1!=0 {
                    let px=1+(i*6) as i32+x as i32;
                    let py=y as i32;
                    // Small shadow preserves contrast on the busy menu art.
                    pixel(px+1,py+1,[0,0,0,180]);
                    pixel(px,py,[216,232,220,255]);
                }
            }
        }
    }
    (data,w,h)
}
fn green_score(raw:&[u8])->u32{
    let mut matched=0usize;
    for p in raw.chunks_exact(4){
        let (r,g,b)=(p[0] as u16,p[1] as u16,p[2] as u16);
        if g>=90 && g>=r+45 && g>=b+40 {matched+=1;}
    }
    (matched*10_000/raw.len().saturating_div(4).max(1)) as u32
}
fn is_control_screen(scores:[u32;4])->bool {
    // Each player has BOTH an upper green title and lower green keyboard
    // instructions. Combat health bars may occupy the upper area, but not
    // the two multi-line instructional regions.
    scores.iter().all(|v|*v>=165)
}
impl MenuSignature {
    pub unsafe fn new(gl:&Gl)->Option<Self>{
        if std::env::var("CYBER_GAMEFRAME_TEST").as_deref()!=Ok("1") ||
           !matches!(std::env::var("CYBER_GAME_EDITION").as_deref(),Ok("US"|"FR")){
            return None;
        }
        let label=std::env::var("CYBER_GAMEFRAME_MENU_SIGNATURE").ok()?;
        if !safe_label(&label){return None;}
        let (pixels,w,h)=make_sprite(&label);
        let mut texture=0;
        unsafe{
            (gl.GenTextures)(1,&mut texture);
            if texture==0{return None;}
            (gl.ActiveTexture)(TEXTURE0);
            (gl.BindTexture)(TEXTURE_2D,texture);
            (gl.TexParameteri)(TEXTURE_2D,TEXTURE_MIN_FILTER,NEAREST);
            (gl.TexParameteri)(TEXTURE_2D,TEXTURE_MAG_FILTER,NEAREST);
            (gl.TexParameteri)(TEXTURE_2D,TEXTURE_WRAP_S,CLAMP_TO_EDGE as i32);
            (gl.TexParameteri)(TEXTURE_2D,TEXTURE_WRAP_T,CLAMP_TO_EDGE as i32);
            (gl.PixelStorei)(UNPACK_ALIGNMENT,1);
            (gl.TexImage2D)(TEXTURE_2D,0,RGBA8 as i32,w,h,0,RGBA,
                UNSIGNED_BYTE,pixels.as_ptr() as *const c_void);
        }
        shim::log!("CYBER_MENU_SIGNATURE_READY text={label} detection=two_player_green_regions menu_only=1 GL_texture={texture}");
        Some(Self{texture,pixel_width:w,pixel_height:h,frames:0,hits:0,visible:false,polls:0})
    }
    pub unsafe fn delete(&self,gl:&Gl){
        unsafe{(gl.DeleteTextures)(1,&self.texture);}
    }
    unsafe fn probe(&self,gl:&Gl,fbo:u32,w:u32,h:u32)->[u32;4]{
        // Coordinates are normalized against the original 4:3 game image,
        // NOT the physical letterboxed Windows/XFCE desktop.
        const ZONES:[(f32,f32,f32,f32);4]=[
            (0.19,0.39,0.11,0.21), // left Player 1 caption
            (0.16,0.44,0.22,0.40), // left keyboard instructions
            (0.60,0.83,0.11,0.21), // right Player 2 caption
            (0.56,0.86,0.22,0.40), // right keyboard instructions
        ];
        let mut scores=[0;4];
        unsafe{
            (gl.BindFramebuffer)(READ_FRAMEBUFFER,fbo);
            (gl.PixelStorei)(PACK_ALIGNMENT,1);
            for (i,(x0,x1,top0,top1)) in ZONES.iter().copied().enumerate(){
                let left=(x0*w as f32) as i32;
                let right=(x1*w as f32) as i32;
                let bottom=(h as f32*(1.0-top1)) as i32;
                let top=(h as f32*(1.0-top0)) as i32;
                let (rw,rh)=(right-left,top-bottom);
                if rw<8||rh<4 {continue;}
                let mut data=vec![0u8;(rw*rh*4) as usize];
                (gl.ReadPixels)(left,bottom,rw,rh,RGBA,UNSIGNED_BYTE,
                    data.as_mut_ptr() as *mut c_void);
                scores[i]=green_score(&data);
            }
            (gl.BindFramebuffer)(READ_FRAMEBUFFER,0);
        }
        scores
    }
    /// Draw only after the game's complete frame has been copied to the
    /// window framebuffer; never write into the Voodoo color/depth buffers.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn draw(&mut self,gl:&Gl,p:&Program,fbo:u32,ow:u32,oh:u32,
        cw:i32,ch:i32,viewport:(i32,i32,i32,i32)){
        if cw<320||ch<240 {return;}
        self.frames+=1;
        if self.frames%20==1 {
            let scores=unsafe{self.probe(gl,fbo,ow,oh)};
            self.polls+=1;
            let candidate=is_control_screen(scores);
            self.hits=if candidate{self.hits.saturating_add(1).min(3)}else{0};
            let was_visible=self.visible;
            self.visible=self.hits>=2;
            if self.polls<=6||self.polls%120==0||self.visible!=was_visible {
                shim::log!("CYBER_MENU_SIGNATURE_PROBE poll={} green_scores={scores:?} candidate={} visible={} transition={} gpu_readback=4_regions",
                    self.polls,u8::from(candidate),u8::from(self.visible),u8::from(self.visible!=was_visible));
            }
        }
        if !self.visible{return;}
        let (vx,vy,vw,vh)=viewport;
        let scale=(ch/480).clamp(1,3);
        let (w,h)=(self.pixel_width*scale,self.pixel_height*scale);
        if w+20>vw||h+20>vh{return;}
        // Anchored inside the original centered 4:3 game image, not to
        // the physical monitor edge or black pillarbox/letterbox bars.
        let left=vx+12;
        let top=vy+12;
        unsafe{
            (gl.BindFramebuffer)(FRAMEBUFFER,0);
            (gl.Viewport)(0,0,cw,ch);
            (gl.Disable)(DEPTH_TEST);
            (gl.Disable)(SCISSOR_TEST);
            // The present shader preserves the sprite's alpha. Only
            // lettering and its tiny shadow are composited; transparent
            // texels leave the original menu pixels untouched.
            (gl.Enable)(BLEND);
            (gl.BlendFuncSeparate)(0x0302,0x0303,1,0x0303); // SRC_ALPHA, ONE_MINUS_SRC_ALPHA
            (gl.ColorMask)(1,1,1,1);
            (gl.UseProgram)(p.id);
            (gl.ActiveTexture)(TEXTURE0);
            (gl.BindTexture)(TEXTURE_2D,self.texture);
            (gl.Uniform1i)(p.loc("uImg"),0);
            (gl.Uniform1f)(p.loc("uGamma"),1.0);
            (gl.Uniform2f)(p.loc("uScreen"),cw as f32,ch as f32);
            (gl.Uniform4f)(p.loc("uRect"),left as f32,top as f32,
                (left+w) as f32,(top+h) as f32);
            (gl.DrawArrays)(0x0005,0,4);
            (gl.Disable)(BLEND);
        }
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn only_launcher_signed_release_labels_are_accepted(){
        assert!(safe_label("CyberGladiatorsFix v0.2.13-1234abcd"));
        assert!(!safe_label("CyberGladiatorsFix v0.2.13-abcdef"));
        assert!(!safe_label("OtherFix v0.2.13-1234abcd"));
        assert!(!safe_label("CyberGladiatorsFix v0.2.13-1234abcd\nbad"));
    }
    #[test]
    fn signature_sprite_is_transparent_without_any_border_or_panel(){
        let (pixels,w,h)=make_sprite("AB");
        assert_eq!((w,h),(13,9));
        // Transparent space around text, not an opaque backing card.
        let at=|x:i32,y:i32| {
            let i=(((h-1-y)*w+x)*4) as usize;
            &pixels[i..i+4]
        };
        assert_eq!(at(0,0),&[0,0,0,0]);
        assert_eq!(at(w-1,h-1),&[0,0,0,0]);
        assert_eq!(at(1,0)[3],255);
        assert!(pixels.chunks_exact(4).any(|p|p[3]==180));
    }
    #[test]
    fn menu_requires_green_titles_and_instructions_for_both_players(){
        assert!(is_control_screen([500,900,500,800]));
        assert!(!is_control_screen([500,0,500,0]));
        assert!(!is_control_screen([500,900,0,800]));
        assert_eq!(green_score(&[0,180,0,255,0,0,0,255]),5000);
    }
}
