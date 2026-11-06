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
    pub fn IsIconic(w: HWND) -> BOOL;
    pub fn GetForegroundWindow() -> HWND;""", "read-only window geometry probes")
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

/// The in-game-window AVI has closed, but CYBER.EXE changes its HWND to
/// 648x513 again BEFORE the Glide game renders its first menu frame.
/// Prepare the correct borderless parent BEFORE the first OpenGL Swap,
/// rather than 30+ swaps afterwards. This may adjust at most ONCE, is
/// limited to the first 120 swaps, and keeps the previous 30-frame
/// debounced recovery as a fallback. No GL context recreation.
pub fn cyber_prepare_before_first_present() {
    let in_window = std::env::var("CYBER_GAMEFRAME_AVI_IN_WINDOW").as_deref() == Ok("1");
    let first_present = std::env::var("CYBER_GAMEFRAME_AVI_FIRST_PRESENT").as_deref() == Ok("1");
    if !(in_window || first_present)
        || std::env::var("CYBER_GAMEFRAME_TEST").as_deref() != Ok("1") {
        return;
    }
    static CHECKS:std::sync::atomic::AtomicU32=std::sync::atomic::AtomicU32::new(0);
    static REPAIRED:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
    if REPAIRED.load(std::sync::atomic::Ordering::Relaxed)
        || CHECKS.fetch_add(1,std::sync::atomic::Ordering::Relaxed)>=120 {
        return;
    }
    let Some(hwnd)=cyber_game_window() else {return};
    unsafe {
        if IsWindowVisible(hwnd)==0 || IsIconic(hwnd)!=0 || GetForegroundWindow()!=hwnd {
            return;
        }
        let mut pid=0;
        let tid=GetWindowThreadProcessId(hwnd,&mut pid);
        if tid==0 || tid!=GetCurrentThreadId() || pid!=GetCurrentProcessId() {
            return;
        }
        let Some(target)=target_monitor(hwnd) else {return};
        let mut rect=RECT::default();
        if GetWindowRect(hwnd,&mut rect)==0 {return;}
        let exact=rect.left==target.left && rect.top==target.top
            && rect.right==target.right && rect.bottom==target.bottom;
        if exact {return;}
        if REPAIRED.compare_exchange(false,true,
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed).is_err() {return;}
        let style=GetWindowLongA(hwnd,GWL_STYLE) as u32;
        let clean=(style & !0x00c4_0000u32)|0x8000_0000u32|WS_VISIBLE;
        let style_changed=style!=clean;
        if style_changed {
            SetWindowLongA(hwnd,GWL_STYLE,clean as i32);
        }
        let flags=SWP_NOZORDER|0x10|
            if style_changed {0x20} else {0}; // NOACTIVATE, conditional FRAMECHANGED
        let ok=SetWindowPos(hwnd,null_mut(),target.left,target.top,
            target.right-target.left,target.bottom-target.top,flags);
        let mut after=RECT::default();
        let readback=GetWindowRect(hwnd,&mut after);
        log!("GAMEFRAME_FIRST_PRESENT_PREPARE before_GL_swap=1 hwnd={hwnd:?} old_rect=({},{},{},{}) target=({},{},{},{}) new_rect=({},{},{},{}) old_style={style:#010x} new_style={clean:#010x} SetWindowPos={ok} readback_ok={readback} mode_change=none focus_change=none",
            rect.left,rect.top,rect.right,rect.bottom,
            target.left,target.top,target.right,target.bottom,
            after.left,after.top,after.right,after.bottom);
    }
}

/// The original game can restore its 640x480 window after grSstOpen.
/// Check the REAL game HWND, but never resize a correctly positioned
/// fullscreen window merely because its harmless Win32 style flags differ.
/// Previous builds compared the entire style DWORD and could trigger
/// SWP_FRAMECHANGED on every 30th frame, leading to visible flicker.
pub fn cyber_maintain_fullscreen() {
    if std::env::var("CYBER_GAMEFRAME_TEST").as_deref() != Ok("1") {
        return;
    }
    static FRAME: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    static BAD_SAMPLES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let frame = FRAME.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if frame % 30 != 0 { return; }
    let Some(hwnd) = cyber_game_window() else { return };
    unsafe {
        // Do not interfere with Alt+Tab or the intro's temporary video host.
        if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0
            || GetForegroundWindow() != hwnd {
            BAD_SAMPLES.store(0, std::sync::atomic::Ordering::Relaxed);
            return;
        }
        let mut pid = 0;
        let tid = GetWindowThreadProcessId(hwnd, &mut pid);
        if tid == 0 || tid != GetCurrentThreadId() || pid != GetCurrentProcessId() {
            shim::log_once!("CyberGladiators fullscreen: swap is not on owner thread, skipped");
            return;
        }
        let Some(target) = target_monitor(hwnd) else { return };
        let style = GetWindowLongA(hwnd, GWL_STYLE) as u32;
        let mut actual = RECT::default();
        if GetWindowRect(hwnd, &mut actual) == 0 { return; }
        // Allow a 2-pixel window-manager rounding error. Critically, ignore
        // irrelevant style bits such as WS_CLIPCHILDREN and WS_CLIPSIBLINGS.
        let delta_ok = |a: i32, b: i32| a.abs_diff(b) <= 2;
        let geometry_ok =
            delta_ok(actual.left, target.left) &&
            delta_ok(actual.top, target.top) &&
            delta_ok(actual.right, target.right) &&
            delta_ok(actual.bottom, target.bottom);
        if frame % 600 == 0 {
            log!("GAMEFRAME_WINDOW_CHECK fullscreen={} hwnd={hwnd:?} style={style:#010x} rect=({},{},{},{}) target=({},{},{},{}) mode_change=none",
                u8::from(geometry_ok),
                actual.left, actual.top, actual.right, actual.bottom,
                target.left, target.top, target.right, target.bottom);
        }
        if geometry_ok {
            BAD_SAMPLES.store(0, std::sync::atomic::Ordering::Relaxed);
            return; // NO SetWindowLongA, NO SetWindowPos, NO redraw
        }

        // Require two consecutive bad geometry samples: do not fight a
        // one-frame size transition during the game's own window messages.
        let bad = BAD_SAMPLES.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        if bad < 2 { return; }

        // Prevent a recurring window-manager tug-of-war. We would rather
        // temporarily retain an imperfect window than flicker indefinitely.
        static LAST: std::sync::OnceLock<std::sync::Mutex<Option<std::time::Instant>>> =
            std::sync::OnceLock::new();
        let lock = LAST.get_or_init(|| std::sync::Mutex::new(None));
        let mut last = lock.lock().unwrap_or_else(|e| e.into_inner());
        if last.is_some_and(|previous| previous.elapsed() < std::time::Duration::from_secs(3)) {
            return;
        }
        *last = Some(std::time::Instant::now());
        drop(last);
        BAD_SAMPLES.store(0, std::sync::atomic::Ordering::Relaxed);

        // Preserve harmless, game-specific bits and remove ONLY the visible
        // frame: WS_CAPTION (0x00C00000) + WS_THICKFRAME (0x00040000).
        // Set the popup flag, without activating or raising this HWND.
        let clean_style = (style & !0x00c4_0000u32) | 0x8000_0000 | WS_VISIBLE;
        let change_style = style != clean_style;
        if change_style {
            SetWindowLongA(hwnd, GWL_STYLE, clean_style as i32);
        }
        let flags = SWP_NOZORDER | 0x10 |
            if change_style { 0x20 } else { 0 }; // SWP_NOACTIVATE, FRAMECHANGED only if needed
        let ok = SetWindowPos(hwnd, null_mut(), target.left, target.top,
            target.right - target.left, target.bottom - target.top, flags);
        let mut after = RECT::default();
        let readback = GetWindowRect(hwnd, &mut after);
        static REPAIRS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = REPAIRS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        if n <= 12 || n % 20 == 0 {
            log!("GAMEFRAME_FULLSCREEN_RECOVERY attempt={n} hwnd={hwnd:?} old_style={style:#010x} new_style={clean_style:#010x} style_changed={} old_rect=({},{},{},{}) target=({},{},{},{}) result={} readback_ok={} readback=({},{},{},{}) retry_cooldown_ms=3000 display_mode_unchanged=1",
                u8::from(change_style),
                actual.left, actual.top, actual.right, actual.bottom,
                target.left, target.top, target.right, target.bottom, ok, readback,
                after.left, after.top, after.right, after.bottom);
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
    src = once(src, "    super::lfb::finish_write();\n    push(Cmd::Swap);",
        """    super::lfb::finish_write();
    // The child AVI and OpenGL share one original game HWND. Repair the
    // game's post-video 648x513 reset BEFORE the very first GL present.
    crate::window::cyber_prepare_before_first_present();
    push(Cmd::Swap);""",
        "repair original HWND before first GL present (AVI-in-window only)")
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
