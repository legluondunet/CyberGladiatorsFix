# CyberGladiatorsFix

**Modern compatibility patch, installer and launcher for Cyber Gladiators (1996), Windows x86.** Designed to work on recent Windows versions and Wine/Proton. Source under GPL-3.0-or-later.

> **Development build / test candidate:** The current implementation has been built and unit-tested with Windows CI, but must still be exercised end-to-end on an actual game installation under Wine and native Windows before a public release.

## What's included?

The Windows download contains **one file: `CyberGladiatorsFix.exe` (64-bit)**. It embeds both project-owned 32-bit runtime components: the WinMM/CDDA FLAC bridge DLL and the isolated Cinepak AVI player. They run as 32-bit components because `CYBER.EXE` is 32-bit. During play, the video helper is extracted to a private temporary directory and removed after it exits. The DLL is installed beside the game only when `EnableCDDA=1`. The original game and nGlide are **not** bundled.

CyberGladiatorsFix can:

- Install a verified **US retail CUE/BIN** data track, preserving the original CD resources and creating a backed-up, validated `CYBER.EXE` with compatibility/no-CD patches.
- Extract CD-audio tracks as lossless `TrackNN.flac` files and emulate legacy CD-audio MCI via a 32-bit WinMM forwarding DLL (optional).
- Start the original game's **640×480 Cinepak introduction** in a separate, borderless fullscreen window. It plays at the screen's size while preserving **4:3 aspect ratio**. **Esc, Enter, Space or any ordinary key** skips the movie. The separate AVI player closes fully before the game starts with DXVK, avoiding the in-process AVI/Direct3D9 crash. The attempted seamless black-curtain handoff was removed after field feedback; investigating the original DXVK failure now takes priority.
- Start the game with nGlide Direct3D9, which uses DXVK when correctly installed in Wine/Proton. On native Windows it uses the platform's Direct3D9 provider.
- Store **all launcher, video, CDDA, Wine, and collected DXVK diagnostics in one `CyberGladiatorsFix.log` beside the launcher**, with a fresh header describing the relevant effective user/hardware/game configuration.

## Installation

1. Obtain the latest successful **`CyberGladiatorsFix-Windows-x64-OneFile`** artifact from [GitHub Actions](https://github.com/legluondunet/CyberGladiatorsFix/actions). Extract `CyberGladiatorsFix.exe` to a **writable game/patcher directory**. This Windows EXE also runs under a configured Wine/Proton prefix.
2. Provide the original supported **US retail `.cue` and its `.bin` tracks** in or next to that directory. To install, run `CyberGladiatorsFix.exe` without parameters from the CUE directory, or pass the path of the CUE.
3. Install the third-party **nGlide** separately from [Zeus Software](https://www.zeus-software.com/downloads/nglide). Its valid 32-bit `glide.dll` must be in the game folder or available from the supported system directory.
4. Use the launcher's INI in `CyberGladiatorsFix/CyberGladiatorsFix.ini` to select video, audio and integrity behavior, then launch the game using the patcher. **Do not run `CYBER.EXE` directly** if you require the out-of-process intro workaround.

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

**Video is automatic:** If `SkipIntro=0`, the launcher plays `splash4.avi` in an isolated full-screen player and then starts the game, with the in-game intro skipped *only for that launch*. If `SkipIntro=1`, the video is omitted. Both paths preserve the user's INI and use exact recognized executable patches. The former `--launch --trace --external-intro` option remains accepted as a developer diagnostic alias.

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

- `SkipIntro=0`: play and optionally skip the intro via keyboard; `1`: omit the movie.
- `EnableCDDA=1`: use the embedded CDDA bridge to play extracted FLAC music; `0`: use the original system WinMM behavior. The installed default currently keeps **0**; change to **1** after verifying audio support.
- `AllowModifiedIQ=0`: require the CD's exact original resource hashes; `1`: explicitly allow **same-size** changes to `iq/iq00.dat…iq99.dat` during **launch only**. The `--verify` command **always remains strict**. The game itself may update some IQ records; never modify or overwrite these game files as part of the patcher refactor.

The internal video size/detail in the game's separate `cyber.ini` can be checked and adjusted using `--max-graphics`. nGlide normally scales 3D graphics to desktop resolution, preserving aspect ratio.

## CDDA upgrades and file safety

The original installer never silently overwrites an unrecognized 32-bit DLL, a modified executable, or an original CD asset. A previously recorded verified receipt or known first-party DLL hash permits a backed-up update. Otherwise use **`--upgrade-cdda`** explicitly, after checking the installed DLL. Existing SHA-qualified `.bak` files are reused only if their complete SHA-256 matches the version being preserved. Interrupted `.tmp` staging files are not deleted automatically. The embedded DLL is validated before it is written to the game folder.

**Do not remove or modify any `IQ/` original game data files** to resolve integrity errors. Preserve their current contents and inspect the mismatch in the log.

### DXVK native-intro root-cause investigation (developer only)

The normal `--launch` path retains the isolated AVI playback workaround. To investigate **why the original 1996 in-process AVI → nGlide Direct3D9/DXVK transition crashes**, use the explicit diagnostic command, not the external video workaround:

```sh
WINEDEBUG=fixme-all wine CyberGladiatorsFix.exe --dxvk-rootcause
```

This **deliberately exercises the known potentially crashing path**: `[Video] SkipIntro=0`, `[Audio] EnableCDDA=1`, native game `splash4.avi`, forced nGlide DirectX9/DXVK, no Glide-pointer hook, no black curtain and no speculative DXVK configuration tweak. It refuses other INI states instead of altering user preferences. The launcher captures Wine exception messages, fresh DXVK D3D9 and any DirectDraw/D7VK logs, loaded D3D9/DirectDraw/MCIAVI provider fingerprints, exact first-chance fault module/RVA, and, if the original thread memory is still available, the instruction bytes and **heuristic** return-address candidates from the exhausted x86 stack. Those samples are not a validated call stack or proof of a DXVK bug. All collected text goes to **the same** `CyberGladiatorsFix.log` beside the launcher. The user's game and `IQ/` data are not changed by the instrumentation.

The original observations are: with `SkipIntro=1`, D3D9/DXVK works; with the in-game AVI enabled, an exception `0xC00000FD` occurs about 200 ms after MCI `close movie` / `close avivideo` in the GE-Proton DXVK x86 `d3d9.dll` at RVA `0xBA1E8`. The issue persists with the optional Glide observer disabled and with desktop refresh-rate selection. It has **not** yet been established whether the recursive call originates in DXVK, nGlide, or Wine's video/window interaction. If the stack sampling is unavailable or ambiguous, obtain a true first-chance `bt` using WineDbg with the same runner, or analyze the exact loaded x86 `d3d9.dll` build rather than guessing a function from its RVA.

## One diagnostic log

Every invocation (except `--help` and `--version`) creates a fresh `CyberGladiatorsFix.log` **next to `CyberGladiatorsFix.exe`**. The header includes version/commit, timestamp, runtime (Windows or Wine), CPU, RAM, Win32 GPU/display size/refresh, the configured intro/CDDA/IQ policy, internal game graphics settings and effective nGlide options. Where supplied by the optional shell wrapper, it includes Linux distribution/kernel. During play, the log receives video, CDDA and exit messages; `--trace` additionally appends Wine information and collected DXVK D3D9 logs.

It does not intentionally record usernames, serial numbers, MAC addresses, hostnames or the full Wine prefix. Driver and crash traces may still contain private paths; review before sharing. Old log files from previous builds are **not** deleted; only the current main log is recreated.

If a launch fails, send **`CyberGladiatorsFix.log`**, the exact invocation and a short description of what appeared on screen. A build that passes Windows CI has not necessarily passed actual gameplay under Wine or native Windows.

## Architecture and development

- `src/`: x64 installer, strict verification, launch orchestration, bundled component extraction and unified diagnostics.
- `cdda/src/lib.rs` + `cdda/src/player.rs`: WinMM MCI CDDA forwarding, FLAC decoder and waveOut output in the 32-bit game.
- `cdda/src/bin/CyberGladiatorsIntro.rs`: short-lived isolated Win32 MCI player for Cinepak.
- `.github/workflows/windows.yml`: builds the 32-bit components first, embeds them in the 64-bit launcher, tests both architectures, and publishes **one EXE**.

The helper remains an independent **process** even though its executable is hidden inside the downloadable launcher. This is intentional: loading the 1996 Cinepak MCI player into the same process as nGlide/DXVK reproduced a stack-overflow crash. Replacing the helper with a `rundll32` export from the CDDA DLL would require separate Wine compatibility validation before removing the proven-safe process-isolation method.

Detailed historical debugging and patch-offset research have been archived in [docs/DEVELOPMENT_HISTORY.md](docs/DEVELOPMENT_HISTORY.md).

**Credits:** legluondunet assisted by ChatGPT (OpenAI). License: [GPL-3.0-or-later](LICENSE).
