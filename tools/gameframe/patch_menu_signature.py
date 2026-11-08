#!/usr/bin/env python3
"""Install a verified-game-only menu version label into pinned GameFrame.

Pure OpenGL overlay, with on-GPU frame pixel recognition; no new Win32
window, native focus change, game asset modification or bundled font file.
All text is minted by the x64 launcher and inherited by the child only.
The three anchored substitutions fail closed if the pinned upstream drifts.
"""
from pathlib import Path
import sys


def once(src: str, old: str, new: str, label: str) -> str:
    if src.count(old) != 1:
        raise RuntimeError(f"{label}: expected one pinned upstream anchor, got {src.count(old)}")
    return src.replace(old, new, 1)


def main(root: Path) -> None:
    render = root / "wrappers" / "glide" / "src" / "render"
    module_src = Path(__file__).with_name("menu_signature.rs").read_text(encoding="utf-8")

    # Assert the known unmodified module layout before creating any new file.
    module_file = render / "mod.rs"
    module = module_file.read_text(encoding="utf-8")
    module = once(module, "mod frame;\n", "mod frame;\nmod cyber_signature;\n", "declare video-only overlay")
    frame_file = render / "frame.rs"
    src = frame_file.read_text(encoding="utf-8")

    src = once(src, "use std::ptr::null;\n", "use std::ptr::null;\nuse std::cell::RefCell;\n", "frame RefCell import")
    src = once(
        src,
        "    small_depth: u32,\n}",
        "    small_depth: u32,\n"
        "    // Separate GL-only menu watermark. Never added to a Voodoo framebuffer.\n"
        "    cyber_signature: RefCell<Option<super::cyber_signature::MenuSignature>>,\n}",
        "frame signature state",
    )
    src = once(
        src,
        "let f = Frame { w, h, ow, oh, fbo, color, depth, back: 0, front: false, small_fbo, small_color, small_depth };",
        "let f = Frame { w, h, ow, oh, fbo, color, depth, back: 0, front: false, small_fbo, small_color, small_depth,"
        " cyber_signature: RefCell::new(super::cyber_signature::MenuSignature::new(gl)) };",
        "initialize signature once per GL context",
    )
    src = once(
        src,
        "        let _ = (self.depth, self.small_depth);",
        "        if let Some(s) = self.cyber_signature.borrow().as_ref() { unsafe { s.delete(gl); } }\n"
        "        let _ = (self.depth, self.small_depth);",
        "delete GL signature texture on teardown",
    )
    src = once(
        src,
        '            crate::gl::check(gl, "in the frame");',
        """            // Paint after the game's 4:3 scene reaches the window, before
            // SwapBuffers. No extra window, focus or input interception.
            if let Some(s) = self.cyber_signature.borrow_mut().as_mut() {
                s.draw(gl, p, self.fbo[which], self.ow, self.oh, cw, ch,
                    (x0, y0, vw, vh));
            }
            crate::gl::check(gl, "in the frame");""",
        "GL-only menu text before actual swap",
    )
    # Publish all files only after every pinned-source anchor matches.
    (render / "cyber_signature.rs").write_text(module_src, encoding="utf-8")
    frame_file.write_text(src, encoding="utf-8")
    module_file.write_text(module, encoding="utf-8")
    print("GameFrame CyberGladiators menu version signature installed (GL-only, conditional)")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: patch_menu_signature.py <pinned-upstream-gameframe-root>")
    main(Path(sys.argv[1]))
