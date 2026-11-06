# Build 85 — code audit and cleanup (2026-11-06)

**Baseline preserved:** GitHub Actions build #85, commit `612dd562c3760810bf1e30a9a36dc7a7dcadf6da`. The audit lives on `audit/build85-gameframe`; no release or merge is implied by a green CI run.

## Verified behavior (user-run tests, not CI)

| Runtime | Evidence | Outcome |
| --- | --- | --- |
| Wine-staging 11.17, 1920×1080 virtual desktop | `CyberGladiatorsFix(20261106-194305).log` | Intro fullscreen 4:3, two DirectDraw calls forwarded, game handoff, normal exit, original Glide restored |
| Wine-staging 11.17, no virtual desktop | User confirmation | Intro and game transition OK without a virtual desktop |
| Proton / Wine 11.0, 1920×1080 | `CyberGladiatorsFix(20261106-194442).log` | Complete ~40-second Cinepak intro, first OpenGL frame handoff, normal exit, original Glide restored |
| Windows 10/11 native | CI smoke loader only | Actual rendering and gameplay **not** tested |

The validated logs record two `IDirectDraw::SetDisplayMode(640,480,16)` requests forwarded as `SetDisplayMode(1920,1080,16)` through the **real Wine implementation**; `GetSystemMetrics` returned 1920×1080 before and after. The native video host uses a 1920×1080 window with a centered 1440×1080 4:3 surface.

**Important limitation:** unchanged desktop dimensions do not prove that display drivers never switched color depth, refresh rate, or physical display mode. The 16-bit request is deliberately still sent to DirectDraw. A direct physical-display-mode audit would need further runtime instrumentation.

## Changes in this audit

- Deleted the superseded `cdda/src/display_guard.rs` USER32 IAT hook, which was disabled in build 85.
- Removed the abandoned passive Wine stderr DirectDraw reader and its additional thread.
- Removed obsolete USER32 launch environment flags and the corresponding log entry.
- Kept the verified DirectDraw v1 IAT/COM technique and its **real** `SetDisplayMode` forwarding.
- Added a bounded per-process shadow-vtable allocation cap. The observed single-object path is unchanged; excess/unexpected COM instances retain their native vtables.
- Refuse to shadow an already-hooked vtable to avoid accidentally forwarding the hook into itself.
- Isolated display-mode translation in a pure function. Tests cover native resolutions and pass-through for nonmatching 16/32/8-bit modes.
- Added the known build-85 Glide SHA-256 to the interrupted-run recovery allowlist. Unknown or corrupt DLLs still fail closed.
- Reduced automatic CI triggers to current branches and extended PR-base checking, while retaining both 32-bit and 64-bit Windows Rust test runs, export checks, embedded DLL/hash verification and smoke tests.
- Updated the main README and the README embedded in CI artifacts.

## Security and compatibility notes

The DirectDraw interception is activated only when **all** of these conditions hold: the verified launcher sets its GameFrame and native-forwarding flags; Wine's `wine_get_version` is present; valid prelaunch desktop dimensions are available; the main executable has the expected base/PE32 shape and a by-name `ddraw.dll!DirectDrawCreate` import. Native Windows bypasses the hook. The launcher's independent SHA-256 checks restrict supported game editions. The patch does not write to game assets, the registry, DirectDraw's shared vtable, or Wine DLLs.

Only the exact legacy request 640×480×16 is translated. Unsupported or unexpected modes pass through unmodified. The original DirectDraw HRESULT is returned instead of manufacturing success (the latter caused the black-screen regression on build 83).

The GameFrame DLL is verified before installation, with hash-qualified backups and conservative restoration. The two x86 DLLs remain embedded in the single x64 launcher. The user receives one consolidated log alongside the executable. Third-party GameFrame is pinned and carries its Apache-2.0 license; the project uses GPL-3.0-or-later.

## Remaining work before release

1. Confirm no regressions from the audit artifact on Wine-staging 11.17 (with and without virtual desktop) and Proton with complete intro, transition and normal exit. This is a **new build**, not identical byte-for-byte to build 85.
2. Test first-time CUE/BIN extraction and upgrade recovery end-to-end on a clean installation; CI cannot use copyrighted game assets.
3. Validate native Windows 10/11 gameplay and input. The Wine-only hook should stay inactive on Windows.
4. If available, independently confirm physical display mode, refresh rate and color-depth behavior outside a Wine virtual desktop.
5. The older nGlide/DXVK diagnostic paths and historical experimental branches remain for traceability. Remove or archive them only in a separate review after the currently validated GameFrame path is merged.

**Do not merge or release based on CI alone.**
