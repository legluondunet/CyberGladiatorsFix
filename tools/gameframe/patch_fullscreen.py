#!/usr/bin/env python3
"""Cyber Gladiators-only fullscreen stabilization on top of patch_gameframe.py.

This is applied only to the pinned GameFrame source checkout by the opt-in CI.
The game may reset its window after grSstOpen, and its 640x480 display mode
can temporarily hide the monitor's real desktop size. Never change the
display mode, window focus or the original game/INI/IQ files.
"""
import sys
from pathlib import Path


def once(src: str, before: str, after: str, note: str) -> str:
    n = src.count(before)
    if n != 1:
        raise ValueError(f"{note}: expected one anchor, got {n}")
    return src.replace(before, after, 1)


def main(root: Path) -> None:
    src_dir = root / "wrappers/glide/src"

    file = src_dir / "win32.rs"
    src = file.read_text(encoding="utf-8")
    src = once(src, "    pub fn GetClientRect(w: HWND, r: *mut RECT) -> BOOL;",
        """    pub fn GetClientRect(w: HWND, r: *mut RECT) -> BOOL;
    pub fn GetWindowRect(w: HWND, r: *mut RECT) -> BOOL;
    pub fn IsIconic(w: HWND) -> BOOL;""", "read-only window geometry probes")
    src = once(src, "    pub fn GetCurrentProcessId() -> u32;",
        """    pub fn GetCurrentProcessId() -> u32;
    pub fn GetCurrentThreadId() -> u32;""", "owner-thread guard")
    file.write_text(src, encoding="utf-8")

    file = src_dir / "window.rs"
    src = file.read_text(encoding="utf-8")
    marker = "/// Width and height for preserving 4:3 within the actual game monitor."
    addition = """/// Wine may briefly report 640x480 when this 1996 game asks for a
/// physical Voodoo display. Preserve the launcher-captured desktop resolution,
/// not the transient game mode. The override only applies to the primary
/// monitor for the explicitly selected verified Cyber Gladiators build.
fn target_monitor(hwnd: HWND) -> Option<RECT> {
    let mut rect = window_monitor(hwnd)?;
    if rect.left != 0 || rect.top != 0 {
        return Some(rect);
    }
    if let Ok(size) = std::env::var("CYBER_GAMEFRAME_DESKTOP_SIZE") {
        if let Some((w, h)) = size.split_once(['x', 'X']) {
            if let (Ok(w), Ok(h)) = (w.parse::<i32>(), h.parse::<i32>()) {
                if (640..=16384).contains(&w) && (480..=16384).contains(&h) {
                    rect.right = rect.left + w;
                    rect.bottom = rect.top + h;
                }
            }
        }
    }
    Some(rect)
}

/// Verify the original input HWND's actual position and style periodically.
/// Cyber Gladiators may restore its 640x480 framed window AFTER grSstOpen,
/// so a one-time adjustment during open is insufficient. This runs in the
/// game's own thread and NEVER raises or activates the window.
pub fn cyber_maintain_fullscreen() {
    if std::env::var("CYBER_GAMEFRAME_TEST").as_deref() != Ok("1") {
        return;
    }
    static FRAME: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let frame = FRAME.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if frame % 30 != 0 { return; }
    let Some(hwnd) = cyber_game_window() else { return };
    unsafe {
        if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 { return; }
        let mut pid = 0;
        let tid = GetWindowThreadProcessId(hwnd, &mut pid);
        if tid == 0 || tid != GetCurrentThreadId() || pid != GetCurrentProcessId() {
            shim::log_once!("CyberGladiators fullscreen: swap is not on owner thread, skipped");
            return;
        }
        let Some(target) = target_monitor(hwnd) else { return };
        let style = GetWindowLongA(hwnd, GWL_STYLE) as u32;
        let desired_style = 0x8000_0000u32 | WS_VISIBLE;
        let mut actual = RECT::default();
        if GetWindowRect(hwnd, &mut actual) == 0 { return; }
        if style == desired_style
            && actual.left == target.left && actual.top == target.top
            && actual.right == target.right && actual.bottom == target.bottom {
            return;
        }
        if style != desired_style {
            SetWindowLongA(hwnd, GWL_STYLE, desired_style as i32);
        }
        // SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED; do not steal
        // focus or switch video mode, and only touch CYBER.EXE's own HWND.
        let ok = SetWindowPos(hwnd, null_mut(), target.left, target.top,
            target.right - target.left, target.bottom - target.top,
            SWP_NOZORDER | 0x10 | 0x20);
        static REPAIRS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = REPAIRS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        if n <= 12 || n % 60 == 0 {
            log!("CyberGladiators fullscreen recovery n={n} hwnd={hwnd:?} style={style:#010x} actual=({},{},{},{}) target=({},{},{},{}) SetWindowPos={ok} video_mode_unchanged=1",
                actual.left,actual.top,actual.right,actual.bottom,
                target.left,target.top,target.right,target.bottom);
        }
    }
}

"""
    src = once(src, marker, addition + marker, "add post-grSstOpen fullscreen stabilization")
    src = once(src, "    let rect = window_monitor(game)?;",
        "    let rect = target_monitor(game)?;",
        "use desktop dimensions for 4:3 output")
    src = once(src, "        if let Some(monitor) = window_monitor(hwnd) {",
        "        if let Some(monitor) = target_monitor(hwnd) {",
        "use desktop dimensions for borderless window")
    file.write_text(src, encoding="utf-8")

    file = src_dir / "api/sst.rs"
    src = file.read_text(encoding="utf-8")
    src = once(src, "    cyber_frame_pace();",
        """    cyber_frame_pace();
    crate::window::cyber_maintain_fullscreen();""",
        "poll actual original HWND after each 30 swaps")
    file.write_text(src, encoding="utf-8")
    print("Patched GameFrame: real pre-video desktop geometry + recurring game-window fullscreen correction (no display switch)")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: patch_fullscreen.py <pinned-gameframe-checkout>")
    main(Path(sys.argv[1]))
