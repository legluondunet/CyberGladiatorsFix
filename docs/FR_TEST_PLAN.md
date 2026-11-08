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

## Native Windows FR — reduce redundant window resizing during AVI handoff

This experiment is deliberately narrower than attempting to replace the
legacy DirectDraw `640×480×16` display mode. The failed 32-bit-native-mode
experiment has been removed. On native FR only, when the system is **still
640×480** at `close avivideo`, the WinMM bridge no longer expands the game
HWND prematurely to the *prelaunch* desktop dimensions: GameFrame was
immediately resizing the HWND a second time just before the first real
OpenGL `SwapBuffers`. The black MCI video host stays in place until
GameFrame's first-present guard performs the single necessary fullscreen
correction. Wine/Proton, US and restored-fullscreen-at-close paths are unchanged.

A single native FR test without Alt+Tab is enough:
- Compare the visible transitions from intro → menu with the earlier
  successful #140/#163 runs. Note any desktop flashes, large/small windows,
  or visible repeated window resizing.
- Expect `GAMEFRAME_HANDOFF_PREPARE ... deferred_to_first_GL_swap=1`
  after the AVI closes while `640×480` is active, and **no** early
  `SetWindowPos=1` `GAMEFRAME_HANDOFF_PREPARE` for that same stage.
- Expect exactly one `GAMEFRAME_FIRST_PRESENT_PREPARE ... SetWindowPos=1`
  when it is needed, before the first OpenGL swap; then
  `GAMEFRAME_FIRST_GL_HANDOFF notified=1`, 3D around 60 FPS, a clean exit,
  and full restoration of the original Windows display mode.
- An actual **Windows display-mode** switch to/from 640×480×16 is still
  expected; this change removes one redundant **window resize**, not the
  separate physical display-mode switch. Do not claim the latter is solved.

## Native Windows FR — DirectDraw virtualization triage (cnc-ddraw)

**Observed failure on 8 November 2026:** On a *disposable copy* of the
working #166 French install, injecting a third-party `ddraw.dll` from
cnc-ddraw leaves the physical desktop at `1920x1200x32` during intro
initialization, but `GetSystemMetrics` reports `640x480`. MCI opens
the movie successfully, then the unified log ends during the existing
one-second `PRE_VIDEO_BLACK_HOLD`, before `MCI_PLAY_RETURNED`.
**No crash code or failure location was captured.** It would be incorrect
to assume that the real `play` command or Cinepak decode caused the
failure: the process might have faulted asynchronously on another thread.

### Minimal A/B isolation — no new binary / graphics patch

Perform **one** second test only with the earlier official cnc-ddraw
`ddraw.dll` and `ddraw.ini`, in the same *disposable copy* used for
the failure. Leave the working #166 install unchanged.

1. Edit `CyberGladiatorsFix/CyberGladiatorsFix.ini` **inside the game
   subfolder**, not the game's binary `cyber.ini`.
2. Set `[Video]` / `SkipIntro=1`. This uses the **existing supported**
   intro-skip setting; the game's CDDA music and GameFrame renderer remain
   enabled. Leave `ddraw.dll` and `ddraw.ini` in this disposable copy
   only, with their previous test settings.
3. Start the launcher normally: `CyberGladiatorsFix.exe`. Do not
   Alt+Tab, change resolutions, or manually skip the already-disabled AVI.
   If the game hangs or crashes, stop; do **not** repeat it.
4. Record whether the menu/3D appears, whether Windows' physical mode stays
   `1920x1200x32`, and whether the exit is clean. Provide the unified log.
5. Remove both third-party files from the copy and restore `SkipIntro=0`
   afterwards. No file in the validated original installation is changed.

**Interpretation:**
- If the game reaches 3D with its native Windows mode unchanged, the
  virtualized 16-bpp DirectDraw state and GameFrame/OpenGL may coexist;
  investigate the MCI/video geometry, thread lifetime, or external
  wrapper's message loop next. This is **not yet proof** MCI scaling caused
  the earlier failure.
- If the game fails again even with AVI disabled, the conflict is in the
  broader DirectDraw/Glide/GameFrame startup or asynchronous thread state.
  Stop trying `cnc-ddraw` configurations and prioritize safer passive
  observation of game DirectDraw calls.
- If the virtual wrapper isn't loaded or the physical mode changes again,
  the test is inconclusive.

Do **not** integrate cnc-ddraw in a release, change the game's graphics
configuration, enable the earlier crashing native COM hook (#124), or
repeat the invalidated 32-bpp substitution experiment (#158).
Build #166 is the reference until a genuinely safe alternative is proven.

## Group D — unchanged US edition

Repeat a quick verified install/launch and video/FLAC/exit test with the known US retail edition in a separate directory. The US code path must continue to use its original file hashes, offsets, HWND and WndProc.

## Required before merging PR #5

- The experimental Windows CI build passes all unit, ABI, package and native DLL-loader checks.
- At least one complete French Wine-staging game run, including full AVI, no-CD second boot, FLAC, frame rate, Alt+Tab and shutdown.
- US regression test.
- Native Windows gameplay and clean CUE/BIN installation still gate the **first public release**.

**Never upload retail game data or original CD tracks to GitHub.** Logs and SHA-256 hashes are sufficient for review.

legluondunet assisted by ChatGPT (OpenAI).
