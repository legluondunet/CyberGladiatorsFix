// SPDX-License-Identifier: GPL-3.0-or-later
//! During the one-click launch, prevent the launcher's private console from
//! covering the native AVI window while the launcher waits for cyber.exe.
//! Keep the Console subsystem: CLI stdout/stderr and --help/--verify must work.
//! Never hide a console shared with a shell/terminal.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};

static OWN_CONSOLE_HIDDEN: AtomicBool = AtomicBool::new(false);
type Hwnd = *mut c_void;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetConsoleWindow() -> Hwnd;
    fn GetConsoleProcessList(ids: *mut u32, count: u32) -> u32;
    fn GetCurrentProcessId() -> u32;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn IsWindowVisible(hwnd: Hwnd) -> i32;
    fn ShowWindow(hwnd: Hwnd, command: i32) -> i32;
    fn MessageBoxW(hwnd: Hwnd, message: *const u16, title: *const u16, flags: u32) -> i32;
}

const SW_HIDE: i32 = 0;
const MB_ICONERROR: u32 = 0x10;

fn is_private_console(attached: &[u32], own_pid: u32) -> bool {
    attached.len() == 1 && attached[0] == own_pid
}

/// Called only on the verified normal launch, immediately before spawning
/// the retail game. In particular this is NOT used by --help, --verify or
/// --install-only. Output remains attached even when its own HWND is hidden.
pub fn hide_private_console() -> &'static str {
    // Avoid headless automation dialogs even if a CI runner has a private HWND.
    if std::env::var_os("GITHUB_ACTIONS").is_some() {
        return "ci-keep";
    }
    // SAFETY: Win32 calls use valid non-null HWND and a fixed-size output buffer.
    unsafe {
        let hwnd = GetConsoleWindow();
        if hwnd.is_null() { return "no-console"; }
        let mut pids = [0u32; 2];
        let count = GetConsoleProcessList(pids.as_mut_ptr(), pids.len() as u32);
        if count != 1 || !is_private_console(&pids[..1], GetCurrentProcessId()) {
            return "shared-or-unknown-console";
        }
        if IsWindowVisible(hwnd) == 0 { return "already-hidden"; }
        // SW_HIDE does not steal focus from the MCI video window.
        ShowWindow(hwnd, SW_HIDE);
        if IsWindowVisible(hwnd) != 0 { return "hide-failed"; }
        OWN_CONSOLE_HIDDEN.store(true, Ordering::Relaxed);
        "hidden-private-console"
    }
}

/// If a launch failure happens after hiding our private console, the user
/// still needs an actionable visible error (and the usual log is preserved).
pub fn report_fatal_if_hidden(error: &str) {
    if !OWN_CONSOLE_HIDDEN.load(Ordering::Relaxed) { return; }
    let message = format!(
        "CyberGladiatorsFix could not launch or finish CyberGladiators:\n\n{error}\n\n\
         Check CyberGladiatorsFix_installer.log for diagnostics.\n\
         You can also enable [Debug] Log=1 in CyberGladiatorsFix.ini."
    );
    let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let title = wide("CyberGladiatorsFix — Error");
    let body = wide(&message);
    // SAFETY: UTF-16 strings are zero-terminated for the duration of the call.
    unsafe {
        MessageBoxW(std::ptr::null_mut(), body.as_ptr(), title.as_ptr(), MB_ICONERROR);
    }
}

#[cfg(test)]
mod tests {
    use super::is_private_console;

    #[test]
    fn only_a_single_self_owned_console_may_be_hidden() {
        assert!(is_private_console(&[42], 42));
        assert!(!is_private_console(&[], 42));
        assert!(!is_private_console(&[42, 101], 42));
        assert!(!is_private_console(&[101], 42));
    }
}
