# CyberGladiatorsFix

**Modern compatibility patch, installer and launcher for Cyber Gladiators (1996), Windows x86.** Designed to work on recent Windows versions and Wine/Proton. Source under GPL-3.0-or-later.

> **Development build / test candidate:** The current implementation has been built and unit-tested with Windows CI, but must still be exercised end-to-end on an actual game installation under Wine and native Windows before a public release.

## What's included?

The Windows download contains **one file: `CyberGladiatorsFix.exe` (64-bit)**, embedding **one WinMM video/CDDA bridge DLL (32-bit)**. The x86 bridge displays the original Cinepak AVI natively in 4:3, performs the guarded WineD3D/DXVK window transition repair and *optionally* emulates CD audio with lossless FLAC. `EnableCDDA=0` still uses the same DLL for video but forwards CD-audio calls to the real system WinMM. There is **no second video player executable**, no additional terminal and no temporary player-extraction directory. The original game and nGlide are not bundled.

CyberGladiatorsFix can:

- Install a verified **US retail CUE/BIN** data track, preserving the original CD resources and creating a backed-up, validated `CYBER.EXE` with compatibility/no-CD patches.
- Extract CD-audio tracks as lossless `TrackNN.flac` files; the same 32-bit WinMM forwarding DLL optionally emulates legacy CD audio, or forwards CD commands to real WinMM when disabled.
- Start the original game's **640×480 Cinepak introduction in the game process** when [Video] SkipIntro=0, regardless of [Audio] EnableCDDA. Under Wine, a dedicated black fullscreen host centers the video in **4:3** without distortion. Only **Escape, Enter and Space** can skip the video; mouse button clicks and other keys do not. After the AVI closes, a guarded Win32 window-procedure repair prevents the previously observed WineD3D/DXVK stack overflow. The AVI presentation, native playback and mouse/keyboard policy have been validated in Wine 11.0; the standard launch path still needs a final regression check. The same native in-game AVI runs when FLAC CDDA music is disabled; the system WinMM handles only CD audio.
- Start the game with nGlide Direct3D9, which uses DXVK when correctly installed in Wine/Proton. On native Windows it uses the platform's Direct3D9 provider.
- Store **all launcher, video, CDDA, Wine, and collected DXVK diagnostics in one `CyberGladiatorsFix.log` beside the launcher**, with a fresh header describing the relevant effective user/hardware/game configuration.

## Installation

1. Obtain the latest successful **`CyberGladiatorsFix-Windows-x64-OneFile`** artifact from [GitHub Actions](https://github.com/legluondunet/CyberGladiatorsFix/actions). Extract `CyberGladiatorsFix.exe` to a **writable game/patcher directory**. This Windows EXE also runs under a configured Wine/Proton prefix.
2. Provide the original supported **US retail `.cue` and its `.bin` tracks** in or next to that directory. To install, run `CyberGladiatorsFix.exe` without parameters from the CUE directory, or pass the path of the CUE.
3. Install the third-party **nGlide** separately from [Zeus Software](https://www.zeus-software.com/downloads/nglide). Its valid 32-bit `glide.dll` must be in the game folder or available from the supported system directory.
4. Use the launcher's INI in `CyberGladiatorsFix/CyberGladiatorsFix.ini` to select video, audio and integrity behavior, then launch the game using the patcher. **Launch through `CyberGladiatorsFix.exe --launch`**, not `CYBER.EXE` directly, to apply the intended game-specific video, DXVK and CDDA configuration.

If the installation already exists, running the patcher without arguments performs a checked update; it does not re-extract or overwrite the original CD assets. The game backup `CYBER.EXE.bak` is preserved. An unknown CD edition or executable is rejected.

## Everyday usage

```text
CyberGladiatorsFix.exe               # install/update (from CUE directory)
CyberGladiatorsFix.exe --launch      # play the game
CyberGladiatorsFix.exe --verify      # strict verification of original assets
CyberGladiatorsFix.exe --trace       # play with extended Wine/DXVK diagnostics
CyberGladiatorsFix.exe --max-graphics
CyberGladiatorsFix.exe --upgrade-cdda
CyberGladiatorsFix.exe --check-bundle
CyberGladiatorsFix.exe --version
CyberGladiatorsFix.exe --help
```

**Video is automatic:** When `SkipIntro=0` and `EnableCDDA=1`, the game plays its original `splash4.avi` in a fullscreen 4:3 host, with black sidebars and skip limited to Escape/Enter/Space. The x86 bridge then attempts a one-shot, strictly validated WineD3D window-procedure repair after AVI close (no extra player). When `EnableCDDA=0`, the in-process AVI still plays, but CD-audio MCI commands are forwarded to system WinMM instead of decoded from FLAC. If `SkipIntro=1`, the intro is omitted. These paths preserve the user's INI and recognize only supported executable variants. `--dxvk-rootcause` remains an unmodified diagnostic control.

### Wine / Proton

```sh
WINEDEBUG=fixme-all wine CyberGladiatorsFix.exe --launch
WINEDEBUG=fixme-all wine CyberGladiatorsFix.exe --trace
```

A convenience `CyberGladiatorsFix-Wine.sh` script remains in the **source repository**, but is not needed in the one-file Windows download. If you use it next to the EXE, it records the native Linux distribution/kernel labels and suppresses Wine's noisy `fixme:` output during ordinary runs while respecting explicit `WINEDEBUG` values.

**For DXVK:** enable DXVK in Lutris/Proton and remove `PROTON_USE_WINED3D=1` from the runner environment. The presence of a D3D9 provider by itself does not prove that DXVK is loaded. If a Wine virtual desktop appears unexpectedly, inspect the same prefix's `winecfg → Graphics → Emulate a virtual desktop` option and Lutris runner settings; the patcher does not alter the prefix's desktop/registry settings.

## Configuration

The text configuration file **`CyberGladiatorsFix/CyberGladiatorsFix.ini`** is *not* the game's own 32-byte binary `cyber.ini`:

```ini
[Video]
SkipIntro=0

[Audio]
EnableCDDA=1

[Integrity]
AllowModifiedIQ=0
```

- `SkipIntro=0`: play the original fullscreen 4:3 intro, optionally skipping with Escape, Enter or Space only; `1`: omit the movie.
- `EnableCDDA=1` (**default for newly created INIs**): play extracted FLAC music through the verified WinMM bridge; `0`: forward CD-audio commands to native system WinMM. **Both settings use the same native in-game 4:3 AVI and the WineD3D/DXVK window repair.** Existing INI files are never overwritten; older INIs without an [Audio] section still select system CD audio.
- `AllowModifiedIQ=0`: require the CD's exact original resource hashes; `1`: explicitly allow **same-size** changes to `iq/iq00.dat…iq99.dat` during **launch only**. The `--verify` command **always remains strict**. The game itself may update some IQ records; never modify or overwrite these game files as part of the patcher refactor.

The internal video size/detail in the game's separate `cyber.ini` can be checked and adjusted using `--max-graphics`. nGlide normally scales 3D graphics to desktop resolution, preserving aspect ratio.

## CDDA upgrades and file safety

The original installer never silently overwrites an unrecognized 32-bit DLL, a modified executable, or an original CD asset. A previously recorded verified receipt or known first-party DLL hash permits a backed-up update. Otherwise use **`--upgrade-cdda`** explicitly, after checking the installed DLL. Existing SHA-qualified `.bak` files are reused only if their complete SHA-256 matches the version being preserved. Interrupted `.tmp` staging files are not deleted automatically. The embedded DLL is validated before it is written to the game folder.

**Do not remove or modify any `IQ/` original game data files** to resolve integrity errors. Preserve their current contents and inspect the mismatch in the log.

### Fullscreen native AVI in standard launches

The native game AVI and soundtrack are retained. When `SkipIntro=0`, `--launch` uses a temporary, in-process black fullscreen Win32 host and a dedicated centered 4:3 video surface. On a 1920×1080 monitor, the video occupies 1440×1080 pixels with black pillarboxes of 240 pixels at both sides. The host uses an explicit black brush and paints black on `WM_PAINT` and `WM_ERASEBKGND` to avoid white background under Wine. For the recognized `splash4.avi`, only the MCI window and destination commands are redirected, and the temporary host is destroyed after `close avivideo` **before** the WineD3D/DXVK WndProc repair.

The video input filters apply **only during playback** and are installed on the game's original UI thread, never globally. Escape, Enter and Space are the only permitted keys for skipping; letter keys and mouse buttons/wheel messages on that thread are suppressed. At video close both hooks are uninstalled, with counters recorded to the one `CyberGladiatorsFix.log`. The user confirmed visually correct black 4:3 pillarboxes, natural full-length video after mouse clicks, all three skip keys working, and other keys doing nothing under Wine 11.0. The old developer-only `--launch --avi-4x3-test` option has been retired now that this behavior is the standard launch. `--trace` uses the same 4:3 native video and input policy while recording additional diagnostics; the separate `--dxvk-rootcause` control deliberately bypasses presentation fixes for forensic comparisons.

For an unsupported window, the host refuses to activate; if the modified video-open fails, the original MCI open is retried. The executable and original `IQ/` assets are not changed by this presentation feature. Native Windows field tests, final ordinary-launch regressions and desktop-panel behavior are still outstanding.

### Native AVI integration status (development build)

On **2026-11-05**, a Wine 11.0 / GE-Proton DXVK test with `--dxvk-wndproc-repair-test` confirmed normal video completion, native D3D9 initialization, FLAC playback, gameplay and a clean process exit. On 2026-11-06, the user further confirmed native fullscreen 4:3 image, black sidebars, ignored mouse clicks and other keys, and working Escape/Enter/Space skipping. The fix identified `wined3d.dll+0xf4a20` as the active WndProc and restored the original game's `cyber.exe+0x5f82` before Direct3D9 installed its own WndProc. The ordinary `--launch` flow now prefers this native-video route **when CDDA is enabled**. That new default route still requires an ordinary-launch regression check, including video skip, gameplay, Alt+Tab, input, and exit; native Windows behavior is not field-validated. The second AVI player and its temporary extraction code have been removed. New installations generate `EnableCDDA=1`; existing INIs retain their audio preference. The one-process video path with `EnableCDDA=0` requires additional Wine and native Windows field validation before release.

### DXVK native-intro root-cause investigation (developer only)

The normal `--launch` path uses the in-process fullscreen AVI with the guarded WndProc repair regardless of the FLAC music setting. To investigate **why the original 1996 in-process AVI → nGlide Direct3D9/DXVK transition crashes**, use the explicit diagnostic command, not the external video workaround:

```sh
WINEDEBUG=fixme-all wine CyberGladiatorsFix.exe --dxvk-rootcause
```

This **deliberately exercises the known potentially crashing path**: `[Video] SkipIntro=0`, native game `splash4.avi`, forced nGlide DirectX9/DXVK, no Glide-pointer hook, no black curtain and no speculative DXVK configuration tweak. It refuses a skipped-video INI state instead of altering preferences; the CDDA music preference is independent. The launcher captures Wine exception messages, fresh DXVK D3D9 and any DirectDraw/D7VK logs, loaded D3D9/DirectDraw/MCIAVI provider fingerprints, exact first-chance fault module/RVA, and, if the original thread memory is still available, the instruction bytes and **heuristic** return-address candidates from the exhausted x86 stack. Those samples are not a validated call stack or proof of a DXVK bug. All collected text goes to **the same** `CyberGladiatorsFix.log` beside the launcher. The user's game and `IQ/` data are not changed by the instrumentation.

**2026-11-05 WineD3D/DXVK subclass-cycle hypothesis and guarded repair test:** The latest native-AVI forensic capture resolves `0x7b094a20` to `wined3d.dll+0xf4a20`. Its window-class ANSI WndProc is the original `CYBER.EXE+0x5f82`; DXVK's `D3D9WindowProc` is at `d3d9.dll+0xba1d0`. The sampled stack alternates the DXVK and WineD3D WndProc targets, consistent with a cyclic subclass chain (not a fully symbolized unwind). Wine's `wined3d_wndproc` and DXVK both forward to previously registered WndProcs. The **developer-only, experimental** `--dxvk-wndproc-repair-test` runs the native in-process AVI exactly as `--dxvk-rootcause` does, but after a successful `close avivideo` attempts to replace the game's *live instance* ANSI WndProc with its original verified window-class procedure. The in-process repair refuses unrecognized executable base, window owner/thread, class, Unicode mode or current provider (must resolve to WineD3D), and logs `WNDPROC_REPAIR APPLIED/SKIP/ERROR`. The normal `--launch` and `--trace` routes now use the verified guarded repair; `--dxvk-rootcause` intentionally preserves the unmodified crash-control route. This is **not yet a validated fix** and may reveal new video/rendering limitations; compare against `--dxvk-rootcause` using the same Wine prefix. It never changes `IQ/`, the Wine registry or system DLLs.

```sh
WINEDEBUG=fixme-all wine CyberGladiatorsFix.exe --dxvk-wndproc-repair-test
```

**2026-11-05 forensic breakthrough:** The user-provided native-AVI crash log consistently shows an x86 first-chance stack overflow at `d3d9.dll+0xBA1E8`, and a repeated `USER32.dll` return-address cycle in the EBP chain. The *actual bytes* at the faulting instruction contain `cmp ebx,0x83` and `cmp edi,1`. These precisely match DXVK's `D3D9WindowProc` special case for `WM_NCCALCSIZE` and `wParam==TRUE` in the GE-Proton DXVK base revision [f8637768, d3d9_window.cpp](https://github.com/doitsujin/dxvk/blob/f863776806824486d709fee16ea619c03e22def4/src/d3d9/d3d9_window.cpp#L34-L102). This strongly identifies a **Win32 window-procedure reentrancy/recursion problem** during the MCI video → nGlide D3D9 transition. It does *not* yet identify the active message, whether the stored previous procedure forms a cycle, or which component should be patched. The observer now captures the x86 EBX/EDI registers, matches the DXVK WndProc instruction fingerprint, attempts to inspect four stdcall arguments without trusting a damaged stack, and records the game's current WndProc pointer across the transition. All probes are read-only, do not edit nGlide/DXVK or game files, and only run under explicit diagnostic tracing. Do not blindly suppress `WM_NCCALCSIZE` messages or patch a third-party DLL based on this evidence alone.

The original observations are: with `SkipIntro=1`, D3D9/DXVK works; with the in-game AVI enabled, an exception `0xC00000FD` occurs about 200 ms after MCI `close movie` / `close avivideo` in the GE-Proton DXVK x86 `d3d9.dll` at RVA `0xBA1E8`. The issue persists with the optional Glide observer disabled and with desktop refresh-rate selection. It has **not** yet been established whether the recursive call originates in DXVK, nGlide, or Wine's video/window interaction. If the stack sampling is unavailable or ambiguous, obtain a true first-chance `bt` using WineDbg with the same runner, or analyze the exact loaded x86 `d3d9.dll` build rather than guessing a function from its RVA.

## One diagnostic log

Every invocation (except `--help` and `--version`) creates a fresh `CyberGladiatorsFix.log` **next to `CyberGladiatorsFix.exe`**. The header includes version/commit, timestamp, runtime (Windows or Wine), CPU, RAM, Win32 GPU/display size/refresh, the configured intro/CDDA/IQ policy, internal game graphics settings and effective nGlide options. Where supplied by the optional shell wrapper, it includes Linux distribution/kernel. During play, the log receives video, CDDA and exit messages; `--trace` additionally appends Wine information and collected DXVK D3D9 logs.

It does not intentionally record usernames, serial numbers, MAC addresses, hostnames or the full Wine prefix. Driver and crash traces may still contain private paths; review before sharing. Old log files from previous builds are **not** deleted; only the current main log is recreated.

If a launch fails, send **`CyberGladiatorsFix.log`**, the exact invocation and a short description of what appeared on screen. A build that passes Windows CI has not necessarily passed actual gameplay under Wine or native Windows.

## Architecture and development

- `src/`: x64 installer, strict verification, game launch orchestration, the embedded x86 bridge and unified diagnostics.
- `cdda/src/lib.rs`: one x86 WinMM forwarding DLL that hosts the original Cinepak AVI fullscreen, preserves the WineD3D/DXVK transition fix, and selects between FLAC CD audio emulation and native CD-audio forwarding.
- `cdda/src/video_trace.rs`: in-process 4:3 black video host and temporary keyboard/mouse input filtering.
- `cdda/src/player.rs`: lossless FLAC decoder and waveOut music when enabled.
- `.github/workflows/windows.yml`: builds the single x86 DLL first, embeds it in the x64 launcher, runs Windows tests and publishes **one EXE**.

The original Cinepak video no longer runs in a separate process. The previous stack-overflow crash is addressed by a guarded WndProc repair which was field-validated with CDDA enabled. The disabled-CDDA audio-forwarding path and native Windows behavior remain subject to final compatibility tests.

Detailed historical debugging and patch-offset research have been archived in [docs/DEVELOPMENT_HISTORY.md](docs/DEVELOPMENT_HISTORY.md).

**Credits:** legluondunet assisted by ChatGPT (OpenAI). License: [GPL-3.0-or-later](LICENSE).
