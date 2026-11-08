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
    label:String,
    palette_sampled:bool,
    ink:[u8;3],
}
// Compact 7x9 squared, chamfered bitmap alphabet following the angular
// appearance and mixed-case proportions of the original bottom-menu UI.
// The retail game's actual proprietary font texture is not embedded.
fn glyph(ch:char)->[u8;9] {
    match ch {
        'A'=>[28,54,99,99,127,99,99,99,0],
        'B'=>[126,99,99,126,99,99,99,126,0],
        'C'=>[30,51,96,96,96,96,51,30,0],
        'D'=>[124,102,99,99,99,99,102,124,0],
        'E'=>[127,96,96,124,96,96,96,127,0],
        'F'=>[127,96,96,124,96,96,96,96,0],
        'G'=>[30,51,96,96,103,99,51,30,0],
        'H'=>[99,99,99,127,99,99,99,99,0],
        'I'=>[62,12,12,12,12,12,12,62,0],
        'J'=>[31,6,6,6,6,102,102,60,0],
        'K'=>[99,102,108,120,120,108,102,99,0],
        'L'=>[96,96,96,96,96,96,96,127,0],
        'M'=>[99,119,127,107,99,99,99,99,0],
        'N'=>[99,115,123,111,103,99,99,99,0],
        'O'=>[28,54,99,99,99,99,54,28,0],
        'P'=>[126,99,99,126,96,96,96,96,0],
        'Q'=>[28,54,99,99,99,107,54,29,0],
        'R'=>[126,99,99,126,108,102,99,99,0],
        'S'=>[30,99,96,60,6,3,99,62,0],
        'T'=>[127,12,12,12,12,12,12,12,0],
        'U'=>[99,99,99,99,99,99,54,28,0],
        'V'=>[99,99,99,99,54,54,28,8,0],
        'W'=>[99,99,99,107,107,127,119,99,0],
        'X'=>[99,99,54,28,28,54,99,99,0],
        'Y'=>[99,99,54,28,12,12,12,12,0],
        'Z'=>[127,3,6,12,24,48,96,127,0],
        'a'=>[0,0,60,6,62,102,102,62,0],
        'b'=>[96,96,124,102,99,99,102,124,0],
        'c'=>[0,0,62,99,96,96,99,62,0],
        'd'=>[3,3,31,51,99,99,51,31,0],
        'e'=>[0,0,60,102,126,96,102,60,0],
        'f'=>[14,24,24,124,24,24,24,24,0],
        'g'=>[0,0,62,99,99,63,3,126,0],
        'h'=>[96,96,124,102,99,99,99,99,0],
        'i'=>[12,0,28,12,12,12,12,30,0],
        'j'=>[6,0,14,6,6,6,102,60,0],
        'k'=>[96,96,102,108,120,120,108,102,0],
        'l'=>[28,12,12,12,12,12,12,30,0],
        'm'=>[0,0,118,107,107,107,107,107,0],
        'n'=>[0,0,124,102,99,99,99,99,0],
        'o'=>[0,0,60,102,99,99,102,60,0],
        'p'=>[0,0,124,102,99,126,96,96,0],
        'q'=>[0,0,31,51,99,63,3,3,0],
        'r'=>[0,0,110,112,96,96,96,96,0],
        's'=>[0,0,62,96,60,6,6,124,0],
        't'=>[24,24,126,24,24,24,24,14,0],
        'u'=>[0,0,99,99,99,99,51,31,0],
        'v'=>[0,0,99,99,99,54,54,28,0],
        'w'=>[0,0,99,99,107,107,127,54,0],
        'x'=>[0,0,99,54,28,28,54,99,0],
        'y'=>[0,0,99,99,63,3,6,124,0],
        'z'=>[0,0,127,6,12,24,48,127,0],
        '0'=>[28,54,99,99,107,99,54,28,0],
        '1'=>[12,28,12,12,12,12,12,62,0],
        '2'=>[60,102,6,12,24,48,96,126,0],
        '3'=>[124,6,6,60,6,6,6,124,0],
        '4'=>[6,14,30,54,102,127,6,6,0],
        '5'=>[127,96,96,124,6,6,102,60,0],
        '6'=>[30,48,96,124,102,99,102,60,0],
        '7'=>[127,3,6,12,24,24,24,24,0],
        '8'=>[60,102,102,60,102,102,102,60,0],
        '9'=>[60,102,99,63,3,6,12,120,0],
        '.'=>[0,0,0,0,0,0,12,12,0],
        '-'=>[0,0,0,0,62,0,0,0,0],
        ' '=>[0;9],
        _=>[0,0,0,28,28,0,28,0,0],
    }
}
/// (first lit x, last lit x) in the seven-column glyph raster.
/// Space remains a visible word gap while narrow lowercase letters keep
/// their original menu-like proportions.
fn span(glyph:&[u8;9])->(usize,usize) {
    let cols=(0..7).filter(|&x|
        glyph.iter().any(|row| row & (1u8 << (6-x))!=0)
    ).collect::<Vec<_>>();
    if cols.is_empty(){(0,3)}else{(*cols.first().unwrap(),*cols.last().unwrap())}
}
fn safe_label(text:&str)->bool{
    text.starts_with("CyberGladiatorsFix v")
        && (26..=56).contains(&text.len())
        && text.bytes().all(|v|v.is_ascii_alphanumeric()||b". -".contains(&v))
        && text.rsplit_once('-').is_some_and(|(_,suffix)|
            suffix.len()==8 && suffix.bytes().all(|v|v.is_ascii_hexdigit()))
}
// Muted lavender-grey of the original bottom UI text, used if GPU color
// sampling is unavailable. No opaque rectangle and no external font asset.
const MENU_PURPLE_FALLBACK:[u8;3]=[99,94,137];
fn make_sprite(text:&str,ink:[u8;3])->(Vec<u8>,i32,i32){
    let glyphs=text.chars().map(glyph).collect::<Vec<_>>();
    let w=(4+glyphs.iter().map(|g|{
        let (left,right)=span(g);
        right-left+2 // variable-width glyph plus one-pixel tracking
    }).sum::<usize>()) as i32;
    let h=12i32; // 9px original-style lettering + top/bottom/soft shadow
    let mut data=vec![0u8;(w*h*4) as usize];
    // The upstream present shader flips Y; supply bottom-up GL texels.
    let mut pixel=|x:i32,y:i32,color:[u8;4]|{
        let i=(((h-1-y)*w+x)*4) as usize;
        data[i..i+4].copy_from_slice(&color);
    };
    let mut cursor=1i32;
    for g in glyphs {
        let (first,last)=span(&g);
        for (y,row) in g.iter().copied().enumerate(){
            for x in first..=last {
                if row & (1u8 << (6-x)) != 0 {
                    let px=cursor+(x-first) as i32;
                    let py=1+y as i32;
                    // Matches the lightly shadowed, lavender footer
                    // lettering; the source menu remains visible underneath.
                    pixel(px+1,py+1,[9,9,23,130]);
                    pixel(px,py,[ink[0],ink[1],ink[2],242]);
                }
            }
        }
        cursor+=(last-first+2) as i32;
    }
    (data,w,h)
}
// Match the source game's violet-grey footer by sampling its actual
// lower-left "Ctrl + O pour Options" (or English counterpart) pixels.
// The logo/background are too dark; use bright muted purple candidates.
// A robust per-channel median avoids choosing one anti-aliased edge pixel.
fn purple_footer_palette(raw:&[u8])->Option<([u8;3],u32)>{
    let mut hist=[[0u32;256];3];
    let mut n=0u32;
    for p in raw.chunks_exact(4){
        let (r,g,b)=(p[0],p[1],p[2]);
        if r>=50 && g>=42 && b>=75 && r<180 && g<180 && b<205
            && b>=r.saturating_add(10) && b>=g.saturating_add(12)
            && r.abs_diff(g)<42 {
            hist[0][r as usize]+=1;
            hist[1][g as usize]+=1;
            hist[2][b as usize]+=1;
            n+=1;
        }
    }
    if n<32{return None;}
    let median=|bucket:&[u32;256]|->u8{
        let mut total=0;
        for (i,count) in bucket.iter().enumerate(){
            total+=count;
            if total>n/2{return i as u8;}
        }
        0
    };
    let ink=[
        median(&hist[0]).clamp(65,135),
        median(&hist[1]).clamp(60,133),
        median(&hist[2]).clamp(90,176),
    ];
    Some((ink,n))
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
        let (pixels,w,h)=make_sprite(&label,MENU_PURPLE_FALLBACK);
        let mut texture=0;
        unsafe{
            (gl.GenTextures)(1,&mut texture);
            if texture==0{return None;}
            (gl.ActiveTexture)(TEXTURE0);
            (gl.BindTexture)(TEXTURE_2D,texture);
            // Slight filtering resembles the original UI's softened
            // game-resolution glyph edges instead of ultra-sharp 5x7 text.
            (gl.TexParameteri)(TEXTURE_2D,TEXTURE_MIN_FILTER,LINEAR);
            (gl.TexParameteri)(TEXTURE_2D,TEXTURE_MAG_FILTER,LINEAR);
            (gl.TexParameteri)(TEXTURE_2D,TEXTURE_WRAP_S,CLAMP_TO_EDGE as i32);
            (gl.TexParameteri)(TEXTURE_2D,TEXTURE_WRAP_T,CLAMP_TO_EDGE as i32);
            (gl.PixelStorei)(UNPACK_ALIGNMENT,1);
            (gl.TexImage2D)(TEXTURE_2D,0,RGBA8 as i32,w,h,0,RGBA,
                UNSIGNED_BYTE,pixels.as_ptr() as *const c_void);
        }
        shim::log!("CYBER_MENU_SIGNATURE_READY text={label} detection=two_player_green_regions menu_only=1 GL_texture={texture} font=angular_mixed_case_7x9 color=menu_purple palette=footer_sample_or_fallback no_frame=1");
        Some(Self{texture,pixel_width:w,pixel_height:h,frames:0,hits:0,visible:false,polls:0,
            label,palette_sampled:false,ink:MENU_PURPLE_FALLBACK})
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
    /// Read only a narrow strip of the authenticated retail screen where
    /// its own violet "Ctrl + O pour Options" footer is drawn.
    /// The identical region works on the US/FR game's 4:3 framebuffer.
    unsafe fn sample_footer_ink(gl:&Gl,fbo:u32,w:u32,h:u32)
        ->Option<([u8;3],u32)> {
        if w<160||h<120{return None;}
        let left=(w as f32*0.12) as i32;
        let right=(w as f32*0.55) as i32;
        let bottom=(h as f32*0.015) as i32;
        let top=(h as f32*0.09) as i32;
        let (rw,rh)=(right-left,top-bottom);
        if rw<8||rh<4 {return None;}
        let mut raw=vec![0u8;(rw*rh*4) as usize];
        unsafe{
            (gl.BindFramebuffer)(READ_FRAMEBUFFER,fbo);
            (gl.PixelStorei)(PACK_ALIGNMENT,1);
            (gl.ReadPixels)(left,bottom,rw,rh,RGBA,UNSIGNED_BYTE,
                raw.as_mut_ptr() as *mut c_void);
            (gl.BindFramebuffer)(READ_FRAMEBUFFER,0);
        }
        purple_footer_palette(&raw)
    }
    unsafe fn calibrate_ink(&mut self,gl:&Gl,fbo:u32,w:u32,h:u32){
        if self.palette_sampled{return;}
        self.palette_sampled=true;
        let measured=unsafe{Self::sample_footer_ink(gl,fbo,w,h)};
        let (ink,samples,source)=match measured{
            Some((rgb,n))=>(rgb,n,"original_menu_footer"),
            None=>(MENU_PURPLE_FALLBACK,0,"fallback"),
        };
        self.ink=ink;
        let (pixels,tw,th)=make_sprite(&self.label,ink);
        if (tw,th)!=(self.pixel_width,self.pixel_height){return;}
        unsafe{
            (gl.ActiveTexture)(TEXTURE0);
            (gl.BindTexture)(TEXTURE_2D,self.texture);
            (gl.PixelStorei)(UNPACK_ALIGNMENT,1);
            (gl.TexSubImage2D)(TEXTURE_2D,0,0,0,tw,th,RGBA,
                UNSIGNED_BYTE,pixels.as_ptr() as *const c_void);
        }
        shim::log!("CYBER_MENU_SIGNATURE_STYLE font=angular_mixed_case_7x9 color_source={source} rgb={ink:?} matched_pixels={samples} no_frame=1 top_left=1");
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
        if !self.palette_sampled {
            unsafe{self.calibrate_ink(gl,fbo,ow,oh);}
        }
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
            // The present shader preserves the sprite's alpha. Purple-grey
            // menu-style lettering and a subtle edge shadow are composited;
            // transparent texels leave the original menu pixels untouched.
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
        let (pixels,w,h)=make_sprite("AB",MENU_PURPLE_FALLBACK);
        assert_eq!((w,h),(20,12));
        // Transparent space around text, not an opaque backing card.
        let at=|x:i32,y:i32| {
            let i=(((h-1-y)*w+x)*4) as usize;
            &pixels[i..i+4]
        };
        assert_eq!(at(0,0),&[0,0,0,0]);
        assert_eq!(at(w-1,h-1),&[0,0,0,0]);
        assert_eq!(at(3,1),&[99,94,137,242]); // A's top-center pixel
        assert!(pixels.chunks_exact(4).any(|p|p[3]==130));
    }
    #[test]
    fn mixed_case_angular_font_preserves_menu_like_proportions(){
        assert_ne!(glyph('A'),glyph('a'));
        assert_ne!(glyph('I'),glyph('i'));
        let a=span(&glyph('M'));
        let i=span(&glyph('i'));
        assert!(a.1-a.0>i.1-i.0);
    }
    #[test]
    fn violet_footer_palette_prefers_text_pixels_to_background(){
        let mut raw=Vec::new();
        for _ in 0..150{raw.extend_from_slice(&[10,13,44,255]);}
        for _ in 0..60{raw.extend_from_slice(&[94,89,127,255]);}
        assert_eq!(purple_footer_palette(&raw),Some(([94,89,127],60)));
        assert_eq!(purple_footer_palette(&raw[..150*4]),None);
    }
    #[test]
    fn menu_requires_green_titles_and_instructions_for_both_players(){
        assert!(is_control_screen([500,900,500,800]));
        assert!(!is_control_screen([500,0,500,0]));
        assert!(!is_control_screen([500,900,0,800]));
        assert_eq!(green_score(&[0,180,0,255,0,0,0,255]),5000);
    }
}
