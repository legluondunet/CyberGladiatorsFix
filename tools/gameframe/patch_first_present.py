#!/usr/bin/env python3
"""GameFrame native MCI AVI -> real OpenGL first-present handoff.

Apply strictly to the pinned experimental GameFrame sources. The native AVI
has an independent GDI paint host (no 640x480 GL-child clip); the temporary
black video host disappears AFTER the first actual GL SwapBuffers.
"""
from pathlib import Path
import sys


def once(src: str, before: str, after: str, note: str) -> str:
    count = src.count(before)
    if count != 1:
        raise ValueError(f"{note}: expected one anchor, got {count}")
    return src.replace(before, after, 1)


def main(root: Path) -> None:
    src_dir = root / "wrappers/glide/src"
    file = src_dir / "win32.rs"
    src = file.read_text(encoding="utf-8")
    src = once(src, "    pub fn GetWindowThreadProcessId(w: HWND, pid: *mut u32) -> u32;",
        """    pub fn GetWindowThreadProcessId(w: HWND, pid: *mut u32) -> u32;
    pub fn FindWindowA(class: *const u8, title: *const u8) -> HWND;
    pub fn GetPropA(w: HWND, name: *const u8) -> *mut c_void;
    pub fn PostMessageA(w: HWND, message: u32, wp: usize, lp: isize) -> BOOL;""",
        "Win32 AVI host post-present notification")
    file.write_text(src, encoding="utf-8")

    file = src_dir / "window.rs"
    src = file.read_text(encoding="utf-8")
    marker = "pub fn prepare(w: u32, h: u32) -> HWND {"
    addition = r"""/// For this verified game, an MCI paint CHILD of the OpenGL HWND may have
/// a GDI clipping box fixed at 640x480, despite its client being 1920x1080.
/// The separate native MCI host fixes the clip but needs a seamless handoff.
/// The WinMM bridge retains the host black until the *real* OpenGL render
/// thread has completed SwapBuffers, then removes it on its original UI
/// thread. Do not alter Glide or nGlide default behavior.
pub fn cyber_notify_real_first_gl_present() {
    if std::env::var("CYBER_GAMEFRAME_AVI_FIRST_PRESENT").as_deref() != Ok("1")
        || std::env::var("CYBER_GAMEFRAME_TEST").as_deref() != Ok("1") {
        return;
    }
    static SENT: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);
    if SENT.load(std::sync::atomic::Ordering::Acquire) { return; }
    unsafe {
        let host = FindWindowA(
            b"CyberGladiatorsFix_AVI_4x3_Host_v2\0".as_ptr(),
            std::ptr::null());
        if host.is_null() { return; }
        let mut pid = 0;
        if GetWindowThreadProcessId(host, &mut pid) == 0
            || pid != GetCurrentProcessId() { return; }
        // Native MCI sets this marker ONLY after the video closes
        // and the host becomes a black handoff curtain.
        if GetPropA(host,
            b"CyberGladiators_GameFrame_GlPresentedPending\0".as_ptr()).is_null() {
            return;
        }
        if PostMessageA(host, 0x8000+0x4c, 0, 0) != 0 {
            SENT.store(true, std::sync::atomic::Ordering::Release);
            log!("GAMEFRAME_FIRST_GL_PRESENT posted=1 after_real_SwapBuffers=1 host={host:?}");
        } else {
            log!("GAMEFRAME_FIRST_GL_PRESENT post_failed=1 may_retry=1");
        }
    }
}

"""
    src = once(src, marker, addition + marker, "insert post-swap Win32 notification")
    file.write_text(src, encoding="utf-8")

    file = src_dir / "render/frame.rs"
    src = file.read_text(encoding="utf-8")
    src = once(src, "            crate::win32::SwapBuffers(dc);",
        """            let presented = crate::win32::SwapBuffers(dc);
            if presented != 0 {
                // AFTER the real GL swap, not after enqueueing a render cmd.
                crate::window::cyber_notify_real_first_gl_present();
            }""",
        "real OpenGL first presentation")
    file.write_text(src, encoding="utf-8")
    print("GameFrame native AVI host handoff patched after real GL SwapBuffers")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: patch_first_present.py <pinned-upstream-gameframe-root>")
    main(Path(sys.argv[1]))
