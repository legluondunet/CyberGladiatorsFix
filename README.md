# CyberGladiatorsFix

**v0.2.13 (read-only GUI-thread and DirectDraw state diagnosis after intro).** Windows CUE/BIN installer with lossless FLAC extraction, a 32-bit CD-audio MCI bridge, timing/no-CD/startup-video fixes, graphics preferences, verifier and game launcher for CyberGladiators (1996).

[![Windows x64](https://img.shields.io/badge/Windows-x64-blue)](https://github.com/legluondunet)
[![Linux Wine / Proton compatible](https://img.shields.io/badge/Linux-Wine%20%2F%20Proton-blue)](https://www.winehq.org/)

By **legluondunet assisted by ChatGPT (OpenAI)**. Licensed GPL-3.0-or-later.

## Status: first development snapshot; not a confirmed playable release

This implementation follows the Windows-console, fail-closed, image-extraction approach of the author's **ScorchedPlanetFix**. It was adapted for CyberGladiators rather than copying game-specific Scorched Planet offsets or files.

**Known retail CYBER.EXE** (the user-provided executable):

- 351,776 bytes, SHA-256 `226a04a745addbfe77709aa7ec56bf5c17792c524b31882da7653e1973775624`.
- Previous speed-only patched SHA-256 `520d516a572f3287458337e8cef1231b5645f59e3952b3f568a2aab8f71af832` (supported for upgrade).
- Previous speed + no-CD patched SHA-256 `d397053de4b1a8573bd851915e7aeb3773f3b7010a08b85162c28ea94bccdaf9` (supported for upgrade).
- Previous speed + no-CD + intro patched SHA-256 `d8329312a8f63f5efea1720b422429f530799c6887f31c18ba71f970390900b6` (supported for upgrade).
- Current speed + no-CD + intro + CDDA patched SHA-256 `71c8cb080a8d11001a91c370d2248dd9819cfd00b4b919cd7bbcb7699a96eedb` (intro enabled).
- Current skip-intro + CDDA SHA-256 `05215ec70462c0e34de7a5d3f9bf32adebb0a6a17b2946bcb768efb1db1b328f` (intro disabled). Both variants are verified.
- `CYBER.EXE.bak` contains the unchanged original.
- At file offset `0x3116`: relative call displacement `c4 ff ff ff` -> `0e 30 04 00`.
- At file offset `0x3125`: `b5 ff ff ff` -> `ff 2f 04 00`.
- Both calls now target the existing `GetTickCount` import trampoline at virtual address `0x00446B28` rather than the old `QueryPerformanceCounter`-based routine.
- **No-CD startup check:** at raw offset `0x4A16` (VA `0x00405416`), original `e8 4f ff ff ff` (CALL CD-volume check) becomes `b8 01 00 00 00` (`MOV EAX,1`). This skips the original first-CD-drive volume label test for `CYBER`. The separate CDDA bridge now emulates audio commands, not the physical CD filesystem. Only the verified USA executable may be patched.
- **Startup movie:** `0x63FB: 74 2F -> 90 90` bypasses the missing `[Config] CDROM` entry in the old `sierra.ini` when the original installer has not run. `0x640F: 7C 0C -> 90 90` chooses the existing 640×480 `SPLASH4.AVI` instead of the low-resolution `SPLASH.AVI`. Both files use **Cinepak**; Wine/Windows must still be able to decode Cinepak through Video for Windows / MCI.
- **CDDA import redirect:** original WINMM.dll import descriptor name field at file offset `0x4E434` changes from RVA `0x3ED40C` to `0x3EDB00`; the initially empty `.idata` padding at `0x4EF00` receives the name `CyberGladiatorsCDDA.dll`. All 15 imports remain satisfied. The bridge forwards 14 original WinMM functions unchanged and intercepts only `mciSendCommandA` for CD-audio devices; `mciSendStringA` continues to the true WinMM for AVI. The exact final patched executable SHA-256 is `71c8cb080a8d11001a91c370d2248dd9819cfd00b4b919cd7bbcb7699a96eedb`.
- **Optional intro skip:** `CyberGladiatorsFix.ini` (a new plain-text file, **not** the original 32-byte binary `cyber.ini`) contains `[Video] SkipIntro=0` by default. With `SkipIntro=1`, the patched `90 90` at file offset `0x63FB` becomes `EB 2F`, jumping to the introduction function's existing epilogue. All other videos and WinMM/CDDA calls remain unchanged. `--launch` reapplies the selected exact-hash patch before each game start.
- These offsets **must not** be used with unrecognized releases. Unknown hashes are refused and logged.

## CLI

```text
CyberGladiatorsFix.exe
CyberGladiatorsFix.exe <disc.cue>
CyberGladiatorsFix.exe --verify
CyberGladiatorsFix.exe --max-graphics
CyberGladiatorsFix.exe --launch [--3dfx-logo]
CyberGladiatorsFix.exe --help
CyberGladiatorsFix.exe --version
```

Without arguments, looks for exactly one `.cue` in the current working directory (an `.iso` alongside it is ignored). The CUE references one or more `.bin` files holding the data and optional audio tracks. Standalone `.iso` and `.bin` inputs are deliberately rejected. Installs the **CYBER/** tree from the CD image to **CyberGladiatorsFix/** next to the patcher, creates a 32-byte `cyber.ini` with the maximum 640×480 internal image size and 3/3 details, and extracts available CD AUDIO tracks as `Track02.flac` through `Track15.flac` beside `cyber.exe`. It also creates the separate editable `CyberGladiatorsFix.ini` containing the `SkipIntro` switch. Excludes CD-root installers and obsolete 3dfx hardware drivers. Patches the game and saves its original binary. Does not overwrite an existing installation. When run again with an existing CyberGladiatorsFix directory, it installs the 32-bit `CyberGladiatorsCDDA.dll` supplied beside the patcher and upgrades known prior speed/no-CD/intro patches if `cyber.exe.bak` remains intact. It also attempts to install the user's nGlide DLL. No game resources or FLAC tracks are re-extracted.

`--verify` checks known patched executable identity, original `.bak` identity, installer-recorded SHA-256 values for extracted resources, and presence of a 32-bit `glide.dll`.

**Fresh installs:** `cyber.ini` is created during extraction with maximum internal graphics settings. Both `CyberGladiatorsFix.exe` (Windows x64) and `CyberGladiatorsCDDA.dll` (Windows x86) must be extracted from the same GitHub Actions ZIP beside each other. The patcher copies the DLL into the game directory. **Existing installs:** put the two build files beside the existing patcher and rerun `CyberGladiatorsFix.exe` without arguments once to install the DLL and upgrade the original retail EXE; no CUE/BIN re-extraction is required. Use `--max-graphics` for older installations if desired; this backs up `cyber.ini` as `cyber.ini.bak`. The original `cyber.exe.bak` and all game `IQ/` data are preserved.

`--launch` verifies first, starts CYBER.EXE in the game directory and waits for exit. Configures nGlide locally for the game, without editing global registry. Optional `--3dfx-logo` requires a separately supplied `3DfxSpl.dll`.

## Safe CDDA bridge upgrades (v0.2.5)

Version 0.2.4 refused to enable CDDA on an existing installation when the game folder contained a **known v0.2.2** DLL, with SHA-256 `567e748733018c8ae91c1f31e6e90f30abebbf33cf838ea4a90e7aeeeeb055d0`. This fingerprint was verified against the original v0.2.2 GitHub Actions ZIP; it was not a user-modified library. The upgrade logic recognized only the two earliest broken DLLs and incorrectly classified all later project-owned versions as unknown.

The v0.2.5 patcher accepts the archived project-owned DLL hashes from versions v0.2.1 through v0.2.4 (plus the separately distributed development builds) for replacement, while continuing to **refuse unrecognized or modified DLLs**. Each replaced DLL is preserved in `CyberGladiatorsCDDA.dll.previous-<first12sha256>.bak` to avoid overwriting the previous `.previous.bak` saved by an earlier upgrade. Staging is verified before publication, with a rollback attempt if a rename fails.

**Test:** replace both launcher and x86 DLL from the same v0.2.5 ZIP; in `CyberGladiatorsFix/CyberGladiatorsFix.ini` retain `SkipIntro=1` and set `[Audio] EnableCDDA=1`. Then launch using `CyberGladiatorsFix.exe --launch --trace`. If the game crashes, attach `CyberGladiatorsFix-WineTrace.log` and `CyberGladiatorsCDDA.log` (if created). With `--launch --trace` and `EnableCDDA=1`, in-game CDDA music playback has now been confirmed under Wine on the USA edition (user test, v0.2.5). Pause/resume, individual track transitions, and native Windows still need separate validation.

## Capturing startup failure details (v0.2.4)

If `--launch` still exits with `0xC0000028` (STATUS_BAD_STACK), this status alone is insufficient to locate the underlying exception. Use:

```powershell
.\CyberGladiatorsFix.exe --launch --trace
```

This diagnostic launch still verifies the exact known game executable, keeps the original game assets and `IQ/` files untouched, and respects `[Video] SkipIntro` and `[Audio] EnableCDDA`. **Both normal and trace launches** now use `NGLIDE_BACKEND=0` (automatic) in v0.2.6. Previous builds forced Vulkan in normal mode whenever `vulkan-1.dll` loaded, even when Vulkan graphics initialization failed. The game is launched with `WINEDEBUG=+seh,+loaddll` and child stdout/stderr are written to `CyberGladiatorsFix-WineTrace.log` beside the patcher. The trace file is replaced on each diagnostic run. Other nGlide settings remain unchanged. This helps separate graphics-backend issues from the original Win32 stack/exception problem; it does **not** claim to fix the crash. If the crash persists, share the Wine trace for analysis. On native Windows the Wine debug variable is ignored, but the launcher's stdout/stderr capture still works.

**From v0.2.6**, normal `--launch` uses the same automatic nGlide backend as `--trace`, without setting `WINEDEBUG` or capturing diagnostics. Other nGlide settings are unchanged. The successful trace-mode comparison points to Vulkan backend selection as the likely cause of the startup issue; a no-trace v0.2.6 gameplay test is required for confirmation.

## Diagnosing startup without the experimental CDDA bridge (v0.2.3)

The `CyberGladiatorsFix.ini` file is **text** (never edit the game's 32-byte binary `cyber.ini`). On new installs, CDDA remains disabled by default while wider compatibility is being evaluated (basic playback has been confirmed under Wine on the USA edition):

```ini
[Video]
SkipIntro=1

[Audio]
EnableCDDA=0
```

`[Audio] EnableCDDA=0` selects the original **WINMM.dll** import, without loading `CyberGladiatorsCDDA.dll`; the CDDA bridge is not required. `EnableCDDA=1` opts in to the experimental FLAC bridge and automatically installs/upgrades the x86 DLL from beside the patcher on the next `--launch`. Older text INIs without `[Audio]` also default to disabled. This allows controlled back-to-back comparison without re-extracting the CD or touching any `IQ/` files. Both options can be changed between launches, and every one of the four combinations (intro on/off, CDDA on/off) is accepted only if its exact `CYBER.EXE` SHA-256 matches.

After editing `CyberGladiatorsFix.ini`, launch with `CyberGladiatorsFix.exe --launch`. Test first with `EnableCDDA=0`. If `CYBER.EXE` still exits with `0xC0000028`, the problem is **not exclusively** the CDDA import: this exit status was observed in earlier logs before the CDDA version was installed. Investigate nGlide, Wine, video initialization and collect a Wine backtrace. Re-enable only for testing after the base game starts.

## Skipping only the opening video

On a new installation the patcher creates two independent INI files beside `cyber.exe`:

- `cyber.ini` is the **binary 32-byte original game settings** (screen size and details); never add text keys to it.
- `CyberGladiatorsFix.ini` is **plain UTF-8 text** containing the patcher's options:

```ini
[Video]
SkipIntro=0
```

Change `SkipIntro=1` to skip the opening movie and go directly to the next original game step, or use `SkipIntro=0` to enable the opening movie again. The first time you use the updated patcher on an old installation, it creates the text INI without overwriting existing user files. After changing this setting, always start the game with `CyberGladiatorsFix.exe --launch` (or run the patcher with no arguments once before launching `cyber.exe` directly). The patcher switches between **two precisely SHA-256-verified EXE variants** before launching; this is not a global MCI-video-blocking hack. It leaves `cyber.ini`, FLACs and every `IQ/` file untouched.

## Intro video and graphics

The USA CD ships with both `CYBER/SPLASH.AVI` (320×240, Cinepak video, 22.05-kHz PCM audio) and `CYBER/SPLASH4.AVI` (640×480, same codecs). Both were inspected and decoded from the user-provided original data track. The original game normally skips the video if `[Config] CDROM` is absent from Sierra's Windows-directory `sierra.ini`. The patch bypasses only this legacy condition and selects `SPLASH4.AVI`. It does not install a Cinepak codec, modify the Windows directory or force a separate external video player. **Whether MCI/Cinepak playback works under Wine is still to be tested.**

CyberGladiators stores graphics preferences in a 32-byte binary `cyber.ini` in the game directory. Reverse engineering confirmed two four-byte little-endian fields at offsets `0x10` (internal screen-size index: `0` = 640×480 maximum) and `0x14` (detail index: `3` = maximum of 0–3). `--max-graphics` updates only those two fields, preserves the other 24 bytes and backs up the original once. The internal 640×480 image can be upscaled by nGlide; the option does not add native widescreen rendering.

On fresh installations, the 32-byte `cyber.ini` is created using the original USA game-generated file provided for analysis as the baseline; the six non-graphics DWORDs are preserved and are not all independently characterized. The graphics fields are set to `0` (640×480 maximum internal size) and `3` (detail 3/3). This generated file is a writable local game preference, excluded from the immutable CD-file manifest. The game may overwrite these settings when saving preferences. If you prefer adjusting them manually, the original manual specifies `+` / `-` for image size and `]` / `[` for detail.

## All three process windows disappear after AVI (v0.2.13)

The user's v0.2.12 Wine trace fully confirms `trace=1 passthru_noop=1`, `SkipIntro=0`, `EnableCDDA=1`, and matching installed/bundled DLL SHA-256 `3d816b5f213a328ccb5b4a3a278c8650c1f304d6d91df555901f975faf1ea90a`. The entire 40-second Cinepak AVI returns `mode=stopped`; `close movie` and `close avivideo` return success. The `grSstPassthruMode(1)` call is bypassed and the wrapper returns. Track12 starts decoding through the FLAC bridge, yet no game window appears.

The v0.2.12 read-only `EnumWindows` snapshots now show the exact window transition: immediately after MCI close the game PID owns **three top-level HWNDs** (main `0x200b0` visible, two hidden 1×1 windows); the main window changes from `(636,283)-(2556,1363)` (1920×1080) to `(636,283)-(1284,796)` (648×513) at 100 ms. At 500 ms the saved game HWND is invalid and **the process owns zero top-level windows**, remaining so at 1.5, 4 and 8 seconds. `GetForegroundWindow()` changes to `0x2007a`, an unrelated/unknown HWND. Thus the symptom is not merely a hidden window or a newly created replacement top-level window; the Windows GUI windows were destroyed while the DLL's monitoring thread remained operational for 8 seconds. Whether the game window-owner thread actually exits, blocks or handles window-destroy messages remains unknown. The 3dfx global continues to show `state_3dfx=1` and `state_vga=0`, which does not alone prove the rendering surface remains healthy.

The `CyberGladiatorsFix-WineTrace.log` provided contained only the launcher header and no Wine diagnostic lines. This is insufficient evidence for or against a Wine exception.

**New in v0.2.13:** after each video close, the x86 DLL records `GLIDE_RESTORE WINDOW_THREAD` status for the original game-window owner's **Win32 thread ID**. It uses `OpenThread`, `GetExitCodeThread` and a nonblocking `WaitForSingleObject` to distinguish an exited/signaled thread from a live thread even when the HWND is gone. It also samples the original game's three DirectDraw COM pointer globals `0x0044728C`, `0x00447290`, `0x00447294` and saved graphics mode `0x00528140` through **read-only** loads. No window is shown, recreated, moved or hooked, no game binary is changed on disk, and the A/B 3dfx NO-OP remains explicitly opt-in. The user's v0.2.12 DLL hash is on the safe upgrade list so this version does **not** require re-extraction of the game or FLAC audio tracks.

Repeat `CyberGladiatorsFix.exe --launch --trace --passthru-noop` using the matched v0.2.13 EXE + DLL with game INI `SkipIntro=0` and `EnableCDDA=1`. Rename the previous `CyberGladiatorsCDDA.log` and collect the fresh log plus any terminal Wine errors. Critical signals: `WINDOW_THREAD signaled=1` after 500 ms indicates that the original window-owning thread has exited; `signaled=0` indicates it remains alive without any window, requiring a different investigation. Compare the DD pointers at 100 and 500 ms to determine how far the restoration reached. No unvalidated workaround is enabled by default.

## After successful pass-through NO-OP: CDDA plays, game window missing (v0.2.12)

**User-confirmed regression from v0.2.11 Wine test:** The verified A/B launch used `SkipIntro=0`, `EnableCDDA=1`, identical installed/bundled first-party DLLs (SHA-256 `c1e9d05936d627c09e4e4d3eaa54d46f0818687f5f3d0affd67ca2e7da754c40`), and `CYBER_GLIDE_TRACE=1 CYBER_GLIDE_PASSTHRU_NOOP=1`. MCI `close movie` and `close avivideo` completed at 4 s; the trace then recorded `PASSTHRU BEGIN mode=1`, **`PASSTHRU BYPASSED mode=1`** and `PASSTHRU END mode=1`. Subsequently, the game's virtual CDDA successfully opened 14 FLAC tracks and started Track12 via `waveOut`. Nevertheless **the user heard music without any visible game window**. The original HWND `0x200b0` was valid/visible at 100 ms after MCI close but invalid from 500 ms onwards; the game process was still alive long enough to emit later read-only monitor observations. This excludes a non-returning grSstPassthruMode(1) call as the **only** explanation and does not establish whether the game recreates a window, hides it or loses/never initializes its 3dfx surface.

Reverse engineered exact USA original game `CYBER.EXE` SHA-256 `226a04a745addbfe77709aa7ec56bf5c17792c524b31882da7653e1973775624`: after AVI playback returns, wrapper `0x00406E27` calls restore routine `0x00403547`. That routine first calls the `grSstPassthruMode(1)` wrapper at `0x0040355C`, then `0x0040543A` to release graphics interface objects, manipulates the stored game HWND using Win32 APIs, and later calls `0x00404810` to rebuild rendering tables. The bypass only suppresses the 3dfx pass-through call; it does not skip these subsequent operations. The exact failure point is not yet known.

**New in v0.2.12:** read-only per-process top-level window inventory through `EnumWindows` after AVI close. Alongside the old HWND's validity, log the live game global HWND pointer, foreground HWND, 3dfx/VGA mode flags and for each process-owned top-level HWND: thread ID, `IsWindowVisible`, `IsIconic`, styles and screen rectangle. Observe at 0, 100, 500, 1500, 4000 and 8000 ms. The instrumentation does **not** create, destroy, show, activate or move any window; no game or Wine files modified by this observer. This helps distinguish a stale destroyed HWND, a newly created hidden or off-screen HWND, and a missing window. The DLL continues to use the exact same A/B bypass only when explicitly requested.

To test, upgrade **without re-extracting the CUE/BIN** (the v0.2.11 DLL hash is recognized for safe automatic replacement). Set `SkipIntro=0`, `EnableCDDA=1` in the game-folder `CyberGladiatorsFix.ini`. Launch `CyberGladiatorsFix.exe --launch --trace --passthru-noop`, observe whether the window appears, and upload the resulting `CyberGladiatorsCDDA.log` and especially `CyberGladiatorsFix-WineTrace.log` (the latter can reveal a Wine exception even if the game process remains active). **Do not enable this compatibility bypass by default:** it has not recovered the game window.

## Fix: previously unrecognized first-party DLLs (v0.2.11)

An in-place update of an existing installation failed with `Existing CyberGladiatorsCDDA.dll has an unrecognized SHA-256 (f7b26be8cf206e19b80c6418477804b6d25aaef4952e90b75bf7491506c54ab8)`. This DLL is **our own verified GitHub Actions build #87** (`CyberGladiatorsFix-Windows-x64`) from the v0.2.8 development series, not a third-party or modified file: we downloaded its archived DLL and recomputed its exact SHA-256. The existing strict allowlist inadvertently omitted intermediate CI artifacts even when we had provided them for testing.

Version 0.2.11 adds the exact SHA-256 of that binary plus 11 additional previously generated, separately downloaded and verified project-owned CI/DLL snapshots to the safe-upgrade allowlist. Upgrades continue to require the source x86 DLL next to the launcher, preserve a uniquely named backup of the old DLL, verify the installed copy and **refuse unknown hashes**. No CUE/BIN re-extraction or reinstallation of the game/FLAC tracks is required.

**In-place update:** close the game, replace `CyberGladiatorsFix.exe` and `CyberGladiatorsCDDA.dll` beside the patcher with the pair from one v0.2.11 Windows ZIP, then run `CyberGladiatorsFix.exe` (no arguments). If `EnableCDDA=1` in the game-folder `CyberGladiatorsFix.ini`, this safely upgrades the recognized old bridge. Once successful, run `--launch --trace --passthru-noop` with `SkipIntro=0` to repeat the graphics A/B experiment. Preserve past `CyberGladiatorsCDDA.log` files before tracing, as the CDDA logger is append-only. Never delete the game installation to solve this error.

## In-place diagnostic verification (v0.2.10)

The user's 2026-11-04 uploads show `CyberGladiatorsFix.ini` with `SkipIntro=0`, `EnableCDDA=1`, but a **cumulative** `CyberGladiatorsCDDA.log` containing two non-NOOP `PASSTHRU BEGIN mode=1` events without `BYPASSED` or `END`. The launcher log shows `--launch --trace --passthru-noop` being invoked after an intervening **fresh CUE extraction** that had reset the game's INI to `EnableCDDA=0`. These files do not establish whether the later launch loaded the intended CDDA DLL or whether the uploaded INI was from the game's folder. Do not conclude the diagnostic NOOP failed.

Starting with v0.2.10, `--launch --trace --passthru-noop` refuses to launch unless the **game-folder** `CyberGladiatorsFix.ini` contains `EnableCDDA=1`. After the normal in-place DLL upgrade and game verification, it checks that the DLL beside `CYBER.EXE` is **byte-for-byte identical** to the bundled x86 DLL and emits `PASS-THROUGH A/B PREREQUISITES` in the patcher's `CyberGladiatorsFix.log`, naming the actual settings file, effective settings and DLL SHA-256. The x86 bridge independently logs `GLIDE_RESTORE diagnostic environment present: trace=1 passthru_noop=1` at video startup, even before checking the in-memory hook. The v0.2.9 binary (`a95c5787f3a9b5dc101e673777111257fcf0482bf4dce8340cb5b91b00f5026a`) is explicitly recognized as a project-owned DLL for in-place upgrade, with backup. Unknown DLLs are still refused, and no disc extraction is needed.

**Repro:** close the game; extract **both** v0.2.10 EXE and DLL next to one another (not into the game folder); confirm effective `[Video] SkipIntro=0`, `[Audio] EnableCDDA=1` in `<patcher directory>/CyberGladiatorsFix/CyberGladiatorsFix.ini`; launch `CyberGladiatorsFix.exe --launch --trace --passthru-noop` without extracting the CUE again. Rename the old game `CyberGladiatorsCDDA.log` beforehand so it records only the new test. Supply the patcher log and game CDDA log, and report whether the menu appears. If the NOOP flag is truly active, look for `PASSTHRU BYPASSED mode=1` and `PASSTHRU END mode=1`. Do not declare the 3dfx workaround successful until the menu has been observed.

## Experimental 3dfx pass-through isolation (v0.2.9)

**Observed in user's v0.2.8 Wine trace:** `GLIDE_RESTORE pass-through call boundary tracing active` appears, proving that the in-memory pointer verification and instrumentation succeeded. After `close movie` and `close avivideo` return success, the game calls `_grSstPassthruMode@4(1)`: the log contains `GLIDE_RESTORE PASSTHRU BEGIN mode=1`, but **no `PASSTHRU END`**. The game HWND `0x200b0` is visible immediately after MCI close and 100 ms later, then is invalid at 500, 1500 and 4000 ms. This strongly implicates the vintage VGA→Voodoo pass-through call as a crash/hang boundary, though WinMM/Wine or observer interaction could still contribute; no Wine exception trace has been examined.

The new **explicit A/B test** keeps the normal `--launch` and standard `--launch --trace` behavior identical to v0.2.8. A third form, **`CyberGladiatorsFix.exe --launch --trace --passthru-noop`**, sets `CYBER_GLIDE_PASSTHRU_NOOP=1` only for this one game process. The verified in-memory `_grSstPassthruMode@4` observer then **bypasses ONLY the mode=1 call** and logs `GLIDE_RESTORE PASSTHRU BYPASSED mode=1` followed by `GLIDE_RESTORE PASSTHRU END mode=1`. Other mode values and all non-Glide MCI/API calls are delegated unchanged. There are no on-disk binary changes, no registry changes, and no changes to `IQ/`, FLACs or `glide.dll`. The old DLL is safely backed up before replacement.

**User test of 2026-11-04:** after a fresh CUE extraction, `CyberGladiatorsFix.ini` was recreated with `SkipIntro=0, EnableCDDA=0`. The launcher accepted two `--launch --trace --passthru-noop` invocations, but the only Glide log supplied came from an earlier launch (it showed `PASSTHRU BEGIN`, not `BYPASSED`). With CDDA disabled, the x86 bridge is not loaded, so this is **not a valid A/B test**. The SHA-256 `a95c5787f3a9b5dc101e673777111257fcf0482bf4dce8340cb5b91b00f5026a` seen in the cumulative upgrade-error log is confirmed against the published **v0.2.9** DLL ZIP; it should not be labeled a modified/unknown binary in historical diagnosis. A same-version source/target pair is already supported; a refusal can also indicate an older source ZIP and an attempted downgrade. New source commits now fail fast when NO-OP is requested with `EnableCDDA=0` rather than silently performing an uninstrumented run. Do not require the user to re-extract the original CUE/BIN.

Reproduce by keeping `[Video] SkipIntro=0` and `[Audio] EnableCDDA=1` and launching `--launch --trace --passthru-noop`. Let the AVI end (40s) and report **whether the menu becomes visible**, together with `CyberGladiatorsCDDA.log` and `CyberGladiatorsFix-WineTrace.log`. If the menu reappears this isolates the pass-through call as a necessary trigger in this setup and supports implementing a guarded compatibility workaround. If it remains black, inspect the logged window state and native Wine errors before changing rendering. For day-to-day gameplay, `SkipIntro=1` remains the proven workaround. **This is an experimental diagnostic, not a release-ready fix.**

## Tracing the VGA-to-3dfx switch after video (v0.2.8)

**Follow-up log review (2026-11-04):** the attached `CyberGladiatorsCDDA(2).log` records several AVI open/play/close cycles, including a full 40-second sequence and later 1-second/4-second interrupted sequences, but **zero `GLIDE_RESTORE` entries**. The source makes clear that `GLIDE_RESTORE` messages are only emitted when `CYBER_GLIDE_TRACE=1`, set for the child by `--launch --trace` in the v0.2.8 launcher. Even if the in-memory probe refuses the game signature/Glide export pointer, the v0.2.8 DLL emits `GLIDE_RESTORE pass-through tracing unavailable: ...` at the AVI opening. Hence this log alone cannot establish a Glide freeze: probable explanations are a launch without `--trace`, an earlier bridge DLL still installed, or a diagnostic-environment mismatch. Before any further code changes, confirm the v0.2.8 `CyberGladiatorsFix.exe --version`, use `CyberGladiatorsFix.exe --launch --trace` with `EnableCDDA=1` and `SkipIntro=0`, and collect **both** `CyberGladiatorsCDDA.log` and `CyberGladiatorsFix-WineTrace.log`. The latter must begin with `CYBER_GLIDE_TRACE=1`; if it does but no Glide entries appear, verify the loaded x86 bridge's hash/version. Do not infer the graphics call succeeded or failed from a log lacking observer markers.



**Confirmed under Wine:** a full, uninterrupted 40-second `SPLASH4.AVI` ends normally: `status movie mode` returns `stopped`, then `close movie` and `close avivideo` both return success. The screen is still black, and **Alt+Tab does not recover it**. An early input interruption is therefore not the only reproduction. The game menu and 14-track CDDA playback work when `SkipIntro=1`.

Reverse engineering of the exact USA `CYBER.EXE` reveals a Voodoo1-specific VGA/3dfx pass-through transition. `0x00403547` calls `0x0043519D` with `EAX=1`, which delegates to dynamically loaded `_grSstPassthruMode@4` from `glide.dll` (stored in the game's original `0x0044AB4C` pointer). The original 3dfx Glide 2.x programming guide explicitly recommends this function for introductory video clips: it switched physical display output between VGA video and Voodoo's separate 3D frame buffer. This *could* be incompatible with an emulated nGlide graphics surface under Wine, but the cause is not proven.

The v0.2.8 diagnostic DLL adds an **opt-in, temporary in-memory probe** only with `CyberGladiatorsFix.exe --launch --trace` and `[Audio] EnableCDDA=1`. When the real game's code and `glide.dll` export pointer match verified known addresses, the probe wraps that one function pointer to record `GLIDE_RESTORE PASSTHRU BEGIN mode=1` and `GLIDE_RESTORE PASSTHRU END mode=1` in `CyberGladiatorsCDDA.log`. It also observes the game's window style and visibility just after the video closes and at 100, 500, 1500 and 4000 ms. No existing graphics calls are skipped or altered; it does not change the on-disk EXE, Wine prefix, DLL on disk or any `IQ/` files. If a pointer or code signature does not match, it refuses the hook but continues the original video and MCI diagnostics.

To reproduce, set `SkipIntro=0`, `EnableCDDA=1`, run `CyberGladiatorsFix.exe --launch --trace`, let the AVI end naturally without pressing keys, and provide **`CyberGladiatorsCDDA.log`**. If `PASSTHRU BEGIN` has no corresponding `END`, the nGlide call has not returned; if both appear, the hang is later in graphics/window restoration. If neither appears, the game may be hung before that call or the opt-in probe refused to attach. The 1996 window-style changes are not assumed to be safe until tested. Leave `SkipIntro=1` for regular gameplay in the meantime.

An independent test worth trying after the screen goes black is **Alt+Enter**: unlike Alt+Tab, nGlide documents this as its full-screen/windowed mode toggle, so it can exercise the renderer's own display reinitialization. Avoid changing the system-wide nGlide configuration during this focused diagnosis.

## Investigating black screen after SPLASH4.AVI (v0.2.7)

User testing on Wine confirms **the game launches and is playable when `[Video] SkipIntro=1`**, but when `SkipIntro=0`, the 40-second Cinepak `SPLASH4.AVI` plays and the game freezes on a black screen instead of showing the character-selection/menu screen. The launcher log records the game starting but cannot by itself identify where execution hangs.

Static disassembly of the verified USA `CYBER.EXE` identifies the original transition:

- `0x004022C1`: starts `play movie window`, then calls `0x004021E6` repeatedly to query `status movie mode`.
- `0x00402286`: sends `close movie` and `close avivideo`.
- `0x00403547`: restores the graphics/window state after the video.
- Source `SPLASH4.AVI` is approximately 40 seconds long, 640×480 Cinepak at 15 fps.

**The cause is not yet isolated.** The game may repeatedly receive `playing` even after the clip ends, or MCI might close successfully while nGlide/Wine fails to restore the display. Do not change the game binary or disable graphics restoration without a trace.

**First Wine trace findings (2026-11-04):** MCI `open avivideo`, `open splash4.avi alias movie`, `play movie window`, `close movie`, and `close avivideo` all returned `0` (success). The last sampled `status movie mode` at four seconds still reported `playing`; both video close commands ran after approximately **five seconds**, even though the source `SPLASH4.AVI` duration is **40 seconds** (600 Cinepak frames at 15 fps). The verified USA executable's playback loop at VA `0x004022C1` can interrupt playback on keyboard or mouse input and then close the AVI. **Premature interruption is a strong possibility, not yet proven**: the trace does not record which input or event caused the exit. A new test should let the AVI reach its natural 40-second end without pressing keys or mouse buttons, and compare it to an intentionally interrupted playback. Successful MCI `close` calls do not prove the later renderer restoration at VA `0x00403547` succeeded; that function is not instrumented. Until the comparison, use `SkipIntro=1` for reliable gameplay.

**Second Wine trace findings (2026-11-04, uninterrupted playback):** `SPLASH4.AVI` reaches exactly 40 seconds. MCI `status movie mode` reports `playing` on 400-poll heartbeats through 37 seconds and switches to `stopped` at **poll 3942 / 40 s** (`result=0`). Both `close movie` and `close avivideo` return `result=0` at 40 s. This rules out an infinite MCI playback/polling loop **for that reproduction** and confirms that video cleanup completed through the WinMM API. It does **not** confirm that the 3dfx/DirectDraw renderer resumes. In the original verified USA executable, the intro caller invokes the display restore function at **VA `0x00403547`** immediately after `0x004022C1` returns. That function calls `0x0040543A` (graphics-object release), restores main-window style/position, recalculates rendering state and calls `0x00404E76`. The precise point of failure in that sequence remains unverified; do not disable or bypass these calls blindly. A practical non-invasive next discriminator is whether switching away from and back to the window with **Alt+Tab** makes the menu visible, or whether the game remains unresponsive. If needed, capture a Wine exception/rendering trace while reproducing. Preserve `SkipIntro=1` as the known-working option.

The v0.2.7 x86 `CyberGladiatorsCDDA.dll` now records `MCI_VIDEO BEGIN`, `MCI_VIDEO END`, and sampled `MCI_VIDEO STATUS` lines in **`CyberGladiatorsCDDA.log`**, located beside `CYBER.EXE`. Only movie/avivideo commands are logged. The status mode is recorded on transitions and every 400 polls to avoid excessive I/O. These calls are still forwarded unchanged to the real system WinMM, without modifying any result. The patcher can safely upgrade verified v0.2.5 and v0.2.6 DLLs by SHA-256 and backs up the old DLL.

To reproduce while keeping working CD music, set `[Video] SkipIntro=0` and `[Audio] EnableCDDA=1` in `CyberGladiatorsFix.ini`, then run `CyberGladiatorsFix.exe --launch --trace` (or ordinary `--launch` if it works). After the black screen appears, close the process and provide `CyberGladiatorsCDDA.log` **and** `CyberGladiatorsFix-WineTrace.log` (if present). If the final lines repeatedly report `playing`, investigate the MCI polling loop; if `close movie` and `close avivideo` complete, investigate graphics restoration. Meanwhile `SkipIntro=1` remains the validated workaround.

## Normal startup graphics compatibility (v0.2.6)

Under Wine, the user confirmed that `CyberGladiatorsFix.exe --launch --trace` starts Cyber Gladiators and plays CDDA music at maximum graphics, while `CyberGladiatorsFix.exe --launch` from v0.2.5 does not start. The principal process-setting difference was `NGLIDE_BACKEND`: normal launch forced Vulkan (`2`) whenever a Vulkan loader was available; trace used nGlide automatic backend (`0`). **This suggests a backend incompatibility, but the previous trace mode also enabled Wine debug logging, so Vulkan has not been isolated as the sole cause.**

From v0.2.6, both launch paths select `NGLIDE_BACKEND=0`. The ordinary `--launch` path still has **no** `WINEDEBUG` logging and does not redirect child output; `--launch --trace` remains available for deeper diagnostics. No game asset or binary patch changes are needed. Test `--launch` with `[Audio] EnableCDDA=1` to validate the no-trace startup behavior. ScorchedPlanetFix retains its own independent Vulkan selection.

## nGlide

The launcher applies process-local nGlide settings without editing the Windows/Wine registry: **`NGLIDE_BACKEND=0` (automatic, on normal and trace launches)**, `NGLIDE_RESOLUTION=1` (desktop), `NGLIDE_ASPECT=1` (correction), `NGLIDE_REFRESH=2` (60 Hz), `NGLIDE_VSYNC=1` (enabled), `NGLIDE_GAMMA=5` (gamma 1.0), and `NGLIDE_SPLASH=0` unless `--3dfx-logo` is explicitly requested. All settings other than the backend are unchanged from ScorchedPlanetFix. Forcing Vulkan based on `vulkan-1.dll` availability previously correlated with a Wine startup failure; the automatic renderer fixed the problem in `--trace` mode. Normal-mode success must still be confirmed with v0.2.6. These parameters affect Glide rendering only; AVI playback uses a separate MCI/Video-for-Windows path.

Install the **official nGlide** from [Zeus Software](https://www.zeus-software.com/downloads/nglide). The patcher can copy an already-installed 32-bit `glide.dll` from next to the patcher or from Windows `SysWOW64` into **CyberGladiatorsFix/**. It does not bundle, download or install proprietary DLLs. A historical CD `GLIDE.DLL` is explicitly excluded, so that it cannot replace the modern wrapper.

## Disc support

- Reads the ISO9660/Joliet/UDF filesystem inside the CUE/BIN **data track** through the same `isomage` parser used by ScorchedPlanetFix. This is internal conversion only, **not** support for ISO files as input.
- Accepts **CUE + one or multiple referenced BIN files**, or a **single shared BIN with multiple tracks**, with data track MODE1/2048, MODE1/2352 or MODE2/2352 (Form 1) and strict bounds checks. The first CUE track must contain data. CDDA tracks use 2352-byte sectors; if their BIN files exist, they are converted losslessly to FLAC.
- Rejects ambiguous input, unsafe paths, unexpected executable hashes, and preexisting output.
- The **complete CYBER directory** is kept conservatively until real disc inventories establish which individual resources are unnecessary. Installers outside CYBER are discarded.
- **USA edition identified using the provided CUE/BIN Track 01:** `CyberGladiators (USA).cue` + 133,217,280-byte `Track 01.bin`, MODE1/2352, volume `CYBER`, 56,640 sectors.
- The CD contains 760 files; its **CYBER/** tree contains **342 game files totaling 47,954,511 bytes (45.73 MiB)**. Install excludes 418 other CD files (setup/DirectX/demos and non-game content).
- Known USA profile requires the exact retail `CYBER.EXE`, `GRAPHICS/SHELL.FNT` and `SPLASH.AVI` hashes from this disc. The original binary SHA-256 matches the previously supplied `CYBER.EXE`.
- The supplied CUE declares 15 tracks: **Track 01 data** and **Tracks 02–15 CD audio**. The 14 audio BIN files were deliberately omitted in the supplied test fixture. When present, the installer extracts them losslessly as FLAC using a pure Rust encoder (no ffmpeg/flac executable required). Missing audio BIN files are reported and skipped without preventing extraction of the game data. No proprietary game audio is bundled. **The experimental x86 CDDA bridge now connects these FLACs to the game's MCI commands; in-game music playback was confirmed under Wine with v0.2.5.**
- All 56,640 raw MODE1 sectors were checked for sync and mode; all 342 game files were read and hashed from the data track; a local extraction and 8-byte binary patch simulation succeeded. **Rust CI builds and unit tests pass; the user has validated game launch under Wine, maximum graphics settings, and in-game CDDA playback with v0.2.5. Native Windows has not yet been validated.**

## CUE/BIN input layout

Both formats below are supported. The `.cue` sheet is always the only argument:

```text
disc.cue
Track 01.bin       # game data
Track 02.bin       # optional CD audio
Track 03.bin       # optional CD audio
...                 # further tracks named in CUE
```

Alternatively, a CUE may describe multiple sequential tracks within a **single shared BIN**.
The parser respects `INDEX 01` and boundaries at the next `INDEX 00`, when present.
No `track.bin` override CLI argument is needed or accepted. BIN files are resolved from the CUE; do not rename them without updating the CUE sheet.

## CD AUDIO (new in 0.1.2)

The CUE parser recognizes `TRACK NN AUDIO` and reads 2352-byte CDDA sectors as
44,100 Hz, signed 16-bit, interleaved stereo. `INDEX 01` marks the start of
music; the following track's `INDEX 00`, if present in a shared BIN, marks
its end. Audio before `INDEX 01` (stored pregap) is intentionally excluded.
The output is compressed losslessly into **TrackNN.flac** in the game directory,
without sample-rate conversion or volume processing. The local manifest records
SHA-256 and size for each generated FLAC; `--verify` checks those files as well.

On the provided US CUE, filenames will be `Track02.flac` .. `Track15.flac`.
Since the source audio BINs have not been provided, that conversion cannot
be tested against the retail audio yet. An absent audio BIN is clearly reported and
skipped; a present, damaged BIN is rejected. An ISO without a CUE sheet is not
accepted. A data BIN is always required.

**Experimental audio playback (v0.2.0):** the project builds `CyberGladiatorsCDDA.dll` as a separate Windows **x86** Rust DLL (while `CyberGladiatorsFix.exe` remains **x64**). The bridge intercepts only CD-audio `mciSendCommandA` messages: `MCI_OPEN` for type `cdaudio`, `MCI_SET` TMSF, `MCI_PLAY` (track numbers 02–15), `MCI_STOP`, `MCI_PAUSE`, `MCI_RESUME`, `MCI_CLOSE`, and basic status/seek. It decodes only locally extracted lossless FLACs into 44.1 kHz/16-bit/stereo PCM and plays through Windows `waveOut` from a worker thread. The other 14 WinMM imports, including `mciSendStringA` for AVI, are delegated to the original system WinMM module. The bridge writes `CyberGladiatorsCDDA.log` beside the game with track/decoder/device errors. **The user confirmed CDDA music plays during gameplay under Wine with v0.2.5 (USA edition, `--launch --trace`). Native Windows and pause/resume/track transitions have not been validated separately.**

**v0.2.2 fixes a startup hang in the first CDDA build.** The v0.2.1 x86 DLL accidentally exported 14 WinMM forwarding functions as 2-byte `EB FE` self-loops, including the `mciSendStringA` AVI function. The bridge now loads the real system-directory `winmm.dll` by its absolute Windows path, resolves non-CDDA APIs via `GetProcAddress` and forwards them with the original 32-bit calling convention. Windows x86 regression tests exercise `mmioSeek` and `mciSendStringA` through the real system DLL, detecting the previous infinite loop. A new installer run recognizes the two known broken DLL SHA-256 versions, backs up an existing broken DLL as `CyberGladiatorsCDDA.dll.previous.bak` and installs the newly bundled DLL. It refuses to overwrite unknown files. An unmodified game may still need compatibility work; playback under Wine/Windows must be confirmed.

**Upgrade existing installs:** extract **both** `CyberGladiatorsFix.exe` and `CyberGladiatorsCDDA.dll` from the **same latest ZIP** next to the patcher, replacing the older copies, then run the patcher **without arguments** once to upgrade the game folder's DLL. Do **not** re-extract the CD or delete `IQ/` files. After upgrading, use `CyberGladiatorsFix.exe --launch`, which reads `[Video] SkipIntro` from `CyberGladiatorsFix.ini`. The older bridge is rejected by `--verify` until upgraded.

The project-owned DLL is **not** a global `winmm.dll` replacement and must not be installed into the Windows/Wine system directories. It is supplied with the patcher by the Windows CI artifact and installed only beside the game. No original music files are bundled; only extracted tracks can play. Basic CDDA music playback under Wine is validated with v0.2.5; pause/resume, track transitions, and native Windows remain to be tested. The AVI-to-menu black-screen transition remains under investigation using the v0.2.7 MCI movie trace.

## Known limitations / investigation

1. A Wine report confirmed the game starts without asking for the CD and gameplay speed appears normal after v0.1.3. CDDA music playback in the running game is confirmed under Wine with v0.2.5; pause/resume, track transitions and native Windows still need validation.
2. The original startup AVI was absent and graphics settings were below maximum. The graphics maximum settings have been confirmed in-game under Wine. Intro AVI playback and its Cinepak/MCI compatibility remain separate outstanding questions.
3. `--verify` audits extracted files against the **local installation manifest** created from the user's disc; this is an integrity check, not an independently authenticated retail-release database. Different executable editions are refused.
4. The latest Wine install log reports conversion of all 14 tracks. The experimental CDDA player currently decodes whole individual tracks in a background thread (with a 12-minute bound) and streams PCM with a four-buffer waveOut queue. It does not yet implement physical CDDA emulation beyond the game's observed MCI call pattern or advanced notify behavior. Pregaps before INDEX 01 are excluded. This is not a complete bit-perfect CD image/archive.
5. No executable, game assets, disc images or proprietary nGlide binaries are shipped in this source project.

## Build

Uses Rust `cargo`: Windows x64 target for the installer and **Windows x86 target for the CDDA DLL**. Dependencies include `isomage = 2.1.0`, `sha2 = 0.10`, `flacenc = 0.5.1` and 32-bit `claxon = 0.4.3` (FLAC decoding in Rust).

```powershell
cargo test --target i686-pc-windows-msvc --manifest-path cdda/Cargo.toml
cargo build --release --target i686-pc-windows-msvc --manifest-path cdda/Cargo.toml
cargo test --target x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
```

The source project includes Windows GitHub Actions compilation and unit tests for **both** architectures, plus installer CLI smoke tests. The user confirmed the game launched without a CD at apparently normal speed, and all 14 FLAC tracks were extracted. CDDA music playback and maximum graphics settings have now been confirmed in-game under Wine; the AVI video transition, CDDA pause/resume/track changes, and native Windows behavior still require testing.

## Credits and reference

- Project architecture based on [ScorchedPlanetFix](https://github.com/legluondunet/ScorchedPlanetFix) by the same author.
- **Original speed fix credit:** Aqrit ([Bitpatch.com — CyberGladiators Speed Fix](https://bitpatch.com/cybergladiators.html)). The original analysis and the two `CYBER.EXE` hexadecimal substitutions are by Aqrit; CyberGladiatorsFix incorporates them with SHA-256 executable verification, an automatic backup and a separate no-CD startup-check patch. The patch is also documented by [SierraVault](https://mirror.sierravault.net/1996-cybergladiators).
- nGlide: © Zeus Software; download separately at official site.