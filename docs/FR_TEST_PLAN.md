# CyberGladiatorsFix — French retail acceptance tests

**Experimental PR #5, NOT a public release.** Retain your last working US build. No French executable, disc data, music or IQ assets are distributed by this project.

## Test environment

- Use the Windows x64 **PreRelease** artifact produced by the latest successful GitHub Actions run for `feature/french-edition`.
- Create a **new, empty, writable directory** (not your working US game directory).
- Copy into it exactly `CyberGladiatorsFix.exe`, `CYBERG.CUE`, and the original matching `CYBERG.BIN`.
- Under Wine-staging 11.17, first test **without a virtual desktop**, with normal desktop resolution. Log hardware/configuration and any resolution switch observed.
- No nGlide, external `glide.dll`, old executable, game data or other CUE should be present in this test folder.

## Group A — full installation and native AVI

1. Run `WINEDEBUG=-all wine CyberGladiatorsFix.exe` in the fresh directory.
2. The log must show `Detected edition: France (CYBERG, 1996)`, original game-resource hashes verified, backed-up original FR `CYBER.EXE.bak`, and edition-specific patching (no US offsets).
3. Confirm the intro plays **completely without pressing Space**. It must be centered 4:3 with black pillarbox sides, no desktop flash or unintended display resolution change.
4. Verify GameFrame takes over on the first successful real OpenGL SwapBuffers. Look for `GAME_EDITION selected=FR`, `DDRAW_NATIVE_SUMMARY` (on Wine) and `GAMEFRAME_FIRST_GL_HANDOFF`.
5. Check main menu, controls, gameplay speed around 60 FPS, original French text and the game audio/music. The music directory should contain `Track02.flac` … `Track15.flac` if the entire disc was present.
6. Exit normally and check `GAME_EXIT exit code: 0`, `[SESSION] END success`, and the recorded Glide restoration/retention result. Upload the entire `CyberGladiatorsFix.log`.

## Group B — second launch without disc + integrity

1. Move (do not delete) `CYBERG.CUE` and `CYBERG.BIN` out of the folder.
2. Start `CyberGladiatorsFix.exe` again; it should detect the existing installation and run without requiring the physical CD.
3. Run `WINEDEBUG=-all wine CyberGladiatorsFix.exe --verify`; the original retail backup and 343 extracted CD files (minus intentionally excluded legacy drivers, if any) must verify without unexpected IQ modifications.
4. Confirm that a **non-matching EXE**, modified `SHELL.FNT` or altered `glide.dll` is rejected or preserved, not silently overwritten. Only perform corruption tests in a throwaway copy.

## Group C — Wine/Proton regression and Alt+Tab

- Exercise the intro skip once with Space and verify smooth transition without black screen or desktop preview.
- Alt+Tab to another window and back; check graphics, keyboard/mouse input and focus.
- Test under Proton as well as Wine-staging, and compare native desktop geometry before/after startup. Do not assume unchanged reported dimensions prove unchanged physical color depth.

## Native Windows FR — read-only resolution-switch diagnostic (PR #5)

Use the **latest successful** Windows build containing the passive display probes.
One complete launch is sufficient for this diagnostic; do **not** repeatedly
Alt+Tab to compensate for a display problem. Before starting, note whether
Windows itself visibly switches to 640x480 (screen blink, desktop scaling),
rather than only the video window changing size.

1. Start the game from the existing, verified FR installation with no manual
   display settings changes. Let `splash4.avi` play naturally.
2. During the intro, press Alt+Tab **once** to a different window and return.
   Check that the AVI remains visible and preserves its 4:3 aspect ratio.
3. After the full ~40-second intro, check the menu, a gameplay scene, 60 FPS,
   and one Alt+Tab during gameplay; quit normally.
4. Share only the unified `CyberGladiatorsFix.log`. Compare these markers:
   - `FR_DISPLAY_MODE_SNAPSHOT stage=before_game_spawn`: current versus
     registry Windows mode before the EXE runs (dimensions, bpp, refresh).
   - `FR_STATIC_DDRAW`: read-only static scan of the SHA-256-authenticated
     **original FR backup**, limited to executable PE32 sections. A candidate
     `call_vtable_slot21` or `mov_vtable_slot21` means an x86 instruction
     **might** access DirectDraw v1 slot 21 (+0x54), not proof that the instruction
     executes or that the pointer belongs to IDirectDraw. Starting with the
     static follow-up build, `slot21_calls` and `slot21_loads` are counted
     separately, **all indirect-call candidates** are listed (up to a limit
     of 64), and the known `0x00405EB7` wrapper is checked against its
     exact verified byte signature. The `wrapper_caller` lines record
     possible x86 `CALL rel32` instructions into the wrapper and bounded
     surrounding bytes that may reveal 640×480×16 setup parameters.
     These are byte-pattern findings, not a complete disassembly or a
     confirmed execution trace. No proprietary EXE is logged or exported.
     No executable modification, runtime hook, or display-mode call occurs.
   - `GAMEFRAME_FR_DISPLAY stage=before_grGlideInit`,
     `before_grSstOpen`, `after_grSstOpen`: screen geometry at the earliest
     intercepted Glide entry points.
   - `FR_DISPLAY_MODE stage=before_avivideo_open` and
     `stage=before_video_play`: system mode versus registry after game startup.
   - `FR_DISPLAY_MODE stage=movie_focus_transition` and
     `FR_NATIVE_VIDEO_RESIZED`: any changes triggered by Alt+Tab.
   - `DDRAW_NATIVE_SUMMARY ... enabled=0`: **native Windows DirectDraw
     interception must remain disabled** (the previous native COM hook crashed).
   - `GAMEFRAME_FIRST_GL_HANDOFF`, `GAME_EXIT exit code: 0` and
     `SESSION END success`: confirm the already-working transition and exit.
   - `FR_DISPLAY_MODE_SNAPSHOT stage=after_game_exit`: compare the native
     mode's **color depth** against `before_game_spawn`. Alt+Tab has been
     observed restoring 1920×1200 while leaving 16 bpp active, versus the
     32 bpp registry setting; only a post-exit reading can show whether the
     mismatch survives game shutdown. Do not forcibly restore modes from
     the diagnostic itself.

These read-only measurements do **not** prevent the original game's
resolution request. They narrow down when the mode changes so that a later
native-Windows-specific remedy can be designed and tested without disrupting
the validated Wine/Proton and US paths.

## Experimental Windows FR — avoid 640×480×16 requests (explicit opt-in)

**Never confuse this experimental build with the validated default launch.**
A fresh one-click launch still uses the unchanged original DirectDraw mode
sequence. The experimental command is intentionally separate:

```powershell
.\CyberGladiatorsFix.exe --fr-native-mode-test
```

Run it **only** against a previously SHA-256-verified, installed FR game on
native Windows. It refuses Wine/Proton, the US executable, missing game
installations, anything other than a currently active 32-bit desktop mode,
and unexpected game-memory signatures.

This experiment replaces the *immediate width/height/bpp arguments in
CYBER.EXE's loaded RAM only* at three original 640×480×16 setup sites
(`0x004050AF`, `0x00406A68`, `0x00436411`) with the original physical
Windows display width, height, and 32 bpp. It still calls the real
`IDirectDraw::SetDisplayMode`: no fake success result, IAT/COM vtable
interception, disabled graphics initialization, or retail-file replacement.
A normal launch and Wine/US modes do not activate the experiment.

**Compatibility is unproven.** Legacy DirectDraw may reject modern native
32-bit modes; the test can fail to display video or exit with an error.
If it fails, quit, restart normally **without the flag** and provide the
single `CyberGladiatorsFix.log` from the failed test. Process-memory edits
vanish when the game exits. Preserve the previous stable package until
validated.

Record `FR_NATIVE_MODE_TEST` in the launcher and
`glide: CYBER_FR_NATIVE_MODE_TEST status=applied` (or `refused`).
Then check whether `FR_DISPLAY_MODE stage=before_avivideo_open` stays at
the original physical resolution and 32 bpp, whether the full AVI keeps
its 4:3 ratio, whether gameplay runs near 60 FPS, and whether Windows
restores its exact original mode after exit. Do not merge or enable
by default until a native Windows A/B trial and Wine/US regressions
succeed.

## Group D — unchanged US edition

Repeat a quick verified install/launch and video/FLAC/exit test with the known US retail edition in a separate directory. The US code path must continue to use its original file hashes, offsets, HWND and WndProc.

## Required before merging PR #5

- The experimental Windows CI build passes all unit, ABI, package and native DLL-loader checks.
- At least one complete French Wine-staging game run, including full AVI, no-CD second boot, FLAC, frame rate, Alt+Tab and shutdown.
- US regression test.
- Native Windows gameplay and clean CUE/BIN installation still gate the **first public release**.

**Never upload retail game data or original CD tracks to GitHub.** Logs and SHA-256 hashes are sufficient for review.

legluondunet assisted by ChatGPT (OpenAI).
