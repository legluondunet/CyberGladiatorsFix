#!/usr/bin/env python3
"""Apply the minimal Cyber Gladiators-specific GameFrame integration.

Only to the pinned upstream GameFrame checkout in CI. Never touches the game
files or the default nGlide launch. Patches are strict and fail on drift.
"""
from pathlib import Path
import sys


def replace_one(src: str, before: str, after: str, name: str) -> str:
    n = src.count(before)
    if n != 1:
        raise RuntimeError(f"{name}: expected one anchor, found {n}: {before[:85]!r}")
    return src.replace(before, after, 1)


def main(root: Path) -> None:
    glide = root / "wrappers" / "glide" / "src"
    win = glide / "win32.rs"
    src = win.read_text(encoding="utf-8")
    src = replace_one(src, "pub struct WNDCLASSA {",
        """pub struct WNDCLASSA {""", "win32 WNDCLASSA anchor")
    src = replace_one(src, "pub const WS_VISIBLE: u32 = 0x1000_0000;",
        """pub const WS_VISIBLE: u32 = 0x1000_0000;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct MONITORINFO {
    pub cb_size: u32,
    pub monitor: RECT,
    pub work: RECT,
    pub flags: u32,
}""", "win32 monitor metadata")
    src = replace_one(src,
        "    pub fn IsWindowVisible(w: HWND) -> BOOL;",
        """    pub fn IsWindowVisible(w: HWND) -> BOOL;
    pub fn IsWindow(w: HWND) -> BOOL;
    pub fn MonitorFromWindow(w: HWND, flags: u32) -> *mut c_void;
    pub fn GetMonitorInfoA(monitor: *mut c_void, info: *mut MONITORINFO) -> BOOL;""",
        "win32 window and monitor functions")
    win.write_text(src, encoding="utf-8")

    path = glide / "window.rs"
    src = path.read_text(encoding="utf-8")
    marker = "/// Find (or make) the window and size it to `w × h` pixels with no frame."
    insert = """/// Cyber Gladiators' tested PE32 build keeps its real input HWND at this
/// game-owned global. Under the opt-in experiment, prefer THAT window rather
/// than the first visible top-level window (the MCI overlay or a HUD can be
/// enumerated first). This is never used outside the verified game opt-in.
fn cyber_game_window() -> Option<HWND> {
    if std::env::var("CYBER_GAMEFRAME_TEST").as_deref() != Ok("1") {
        return None;
    }
    unsafe {
        let base = GetModuleHandleA(std::ptr::null()) as usize;
        if base != 0x0040_0000 {
            log!("CyberGladiators HWND: unexpected game image base {base:#x}");
            return None;
        }
        // Exact per-edition EXE and original backup are validated by the
        // parent launcher before it sets this child-only environment value.
        let hwnd_rva = match std::env::var("CYBER_GAME_EDITION").as_deref() {
            Ok("US") => 0x0012_8018usize,
            Ok("FR") => 0x0012_8118usize,
            _ => {
                log!("CyberGladiators HWND: unknown game edition, refusing raw address");
                return None;
            }
        };
        let handle = std::ptr::read_volatile((base + hwnd_rva) as *const u32) as usize as HWND;
        let mut pid = 0;
        if handle.is_null()
            || IsWindow(handle) == 0
            || GetWindowThreadProcessId(handle, &mut pid) == 0
            || pid != GetCurrentProcessId()
        {
            log!("CyberGladiators HWND unavailable; falling back to original GameFrame enumeration");
            return None;
        }
        Some(handle)
    }
}

fn window_monitor(hwnd: HWND) -> Option<RECT> {
    unsafe {
        let monitor = MonitorFromWindow(hwnd, 2);
        if monitor.is_null() {
            return None;
        }
        let mut info = MONITORINFO {
            cb_size: std::mem::size_of::<MONITORINFO>() as u32,
            ..MONITORINFO::default()
        };
        if GetMonitorInfoA(monitor, &mut info) == 0 {
            return None;
        }
        Some(info.monitor)
    }
}

/// Width and height for preserving 4:3 within the actual game monitor.
pub fn cyber_viewport_size(w: u32, h: u32) -> Option<(u32, u32)> {
    let game = cyber_game_window()?;
    let rect = window_monitor(game)?;
    let mw = (rect.right - rect.left).max(1) as u32;
    let mh = (rect.bottom - rect.top).max(1) as u32;
    let ow = mw.min((mh as u64 * w as u64 / h.max(1) as u64) as u32);
    let oh = mh.min((mw as u64 * h as u64 / w.max(1) as u64) as u32);
    // One side is always equal to the monitor boundary, never stretch 4:3.
    if ow == mw { Some((ow, oh)) } else { Some((ow, mh)) }
}

"""
    src = replace_one(src, marker, insert + marker, "insert scoped HWND selection")
    src = replace_one(src,
        "    let hwnd = if found.1 {",
        """    let forced = cyber_game_window();
    let hwnd = if let Some(game) = forced {
        if !own.is_null() {
            unsafe { ShowWindow(own, 0) };
        }
        log!("CyberGladiators using original input HWND {game:?} (not a rendering proxy)");
        game
    } else if found.1 {""",
        "prefer actual game HWND")
    src = replace_one(src,
        """    unsafe {
        // WS_POPUP | WS_VISIBLE: a borderless window at (0, 0), like the full screen it was.""",
        """    if forced.is_some() {
        if let Some(monitor) = window_monitor(hwnd) {
            unsafe {
                // Borderless desktop window: never change display resolution.
                SetWindowLongA(hwnd, GWL_STYLE, (0x8000_0000u32 | WS_VISIBLE) as i32);
                SetWindowPos(hwnd, null_mut(),
                    monitor.left, monitor.top,
                    monitor.right - monitor.left, monitor.bottom - monitor.top,
                    0x20);
            }
            log!("CyberGladiators fullscreen monitor rect=({},{})-({},{}) game_hwnd={hwnd:?} input_window=render_window",
                monitor.left,monitor.top,monitor.right,monitor.bottom);
            return hwnd;
        }
    }
    unsafe {
        // WS_POPUP | WS_VISIBLE: a borderless window at (0, 0), like the full screen it was.""",
        "use borderless native monitor for verified game")
    path.write_text(src, encoding="utf-8")

    path = glide / "config.rs"
    src = path.read_text(encoding="utf-8")
    src = replace_one(src, "    pub fn output(&self, w: u32, h: u32) -> (u32, u32) {\n",
        """    pub fn output(&self, w: u32, h: u32) -> (u32, u32) {
        if std::env::var("CYBER_GAMEFRAME_TEST").as_deref() == Ok("1") {
            // GLIDE framebuffer remains 4:3, while the host window covers
            // the 16:9 monitor and the existing renderer letterboxes it.
            if let Some(size) = crate::window::cyber_viewport_size(w, h) {
                return size;
            }
        }
""", "preserve the game's 4:3 presentation")
    path.write_text(src, encoding="utf-8")

    path = glide / "api" / "sst.rs"
    src = path.read_text(encoding="utf-8")
    # Passive sampling at the FIRST Glide entry points, before the game's
    # AVI bridge is invoked. A few GetSystemMetrics reads only; no DirectDraw
    # hooks, display-mode modifications, or user-facing window operations.
    display_marker = '#[unsafe(no_mangle)]\\npub extern "system" fn grGlideInit() {'
    display_probe = """// Cyber Gladiators: Windows-native FR-only resolution timeline.
#[link(name = "user32")]
unsafe extern "system" {
    fn GetSystemMetrics(index: i32) -> i32;
}
fn cyber_fr_readonly_display_probe(stage: &str) {
    if std::env::var("CYBER_GAME_EDITION").as_deref() != Ok("FR")
        || std::env::var("CYBER_GAMEFRAME_ALT_TAB_NATIVE_FIX").as_deref() != Ok("1") {
        return;
    }
    unsafe {
        log!("GAMEFRAME_FR_DISPLAY stage={stage} metrics={}x{} read_only=1",
            GetSystemMetrics(0), GetSystemMetrics(1));
    }
}

"""
    src = replace_one(src, display_marker, display_probe + display_marker,
        "probe before first Glide initialization")
    src = replace_one(src, '    shim::call!("grGlideInit");',
        '    cyber_fr_readonly_display_probe("before_grGlideInit");\\n    shim::call!("grGlideInit");',
        "early Glide initialization probe")
    src = replace_one(src, '    shim::call!("grSstOpen", res, refresh, format, origin, smoothing, buffers);',
        '    cyber_fr_readonly_display_probe("before_grSstOpen");\\n    shim::call!("grSstOpen", res, refresh, format, origin, smoothing, buffers);',
        "before Glide resolution open")
    src = replace_one(src,
        """    opened(w, h, format, origin);
    flush();
    1
}""",
        """    opened(w, h, format, origin);
    flush();
    cyber_fr_readonly_display_probe("after_grSstOpen");
    1
}""",
        "after Glide resolution open")
    src = replace_one(src, '    log!("grSstPassthruMode {mode}");',
        '    log!("grSstPassthruMode {mode}");\\n    cyber_fr_readonly_display_probe("grSstPassthruMode");',
        "Glide video passthrough probe")

    marker = '#[unsafe(no_mangle)]\npub extern "system" fn grBufferSwap(_interval: u32) {'
    paced = """/// The original Voodoo's display synchronization kept this old game's
/// simulation at a reasonable speed. Modern OpenGL on a 120 Hz monitor
/// otherwise lets Cyber Gladiators run at twice its intended pace.
/// Enabled ONLY for the dedicated GameFrame A/B experiment.
fn cyber_frame_pace() {
    if std::env::var("CYBER_GAMEFRAME_TEST").as_deref() != Ok("1") {
        return;
    }
    static FPS: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    let fps = *FPS.get_or_init(|| {
        std::env::var("GLIDE_FPS_LIMIT").ok()
            .and_then(|s| s.parse::<u32>().ok())
            .filter(|n| (15..=120).contains(n))
            .unwrap_or(60)
    });
    thread_local! {
        static LAST: std::cell::RefCell<Option<std::time::Instant>> =
            const { std::cell::RefCell::new(None) };
    }
    let frame = std::time::Duration::from_nanos(1_000_000_000 / fps as u64);
    LAST.with(|cell| {
        let mut last = cell.borrow_mut();
        if let Some(previous) = *last {
            let deadline = previous + frame;
            let now = std::time::Instant::now();
            if now < deadline {
                std::thread::sleep(deadline - now);
            }
        }
        *last = Some(std::time::Instant::now());
    });
}

"""
    src = replace_one(src,marker,paced+marker,"insert paced swap")
    src = replace_one(src, """    super::movie::frame();
    shim::calls::end_frame();""",
        """    super::movie::frame();
    shim::calls::end_frame();
    cyber_frame_pace();""","invoke paced swap")
    path.write_text(src,encoding="utf-8")
    print("GameFrame patched for verified original HWND, centered 4:3 fullscreen and 60 fps pacing")


if __name__ == "__main__":
    if len(sys.argv)!=2:
        sys.exit("usage: patch_gameframe.py <upstream-repository-root>")
    main(Path(sys.argv[1]))
