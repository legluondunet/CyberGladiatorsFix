# CyberGladiatorsFix

**One-click installer, compatibility patch and launcher for Cyber Gladiators (1996).**

[![Windows CI](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml/badge.svg)](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml)

CyberGladiatorsFix is a GPL-3.0-or-later Windows x64 launcher for Windows and Wine/Proton. It bundles our verified x86 **`fix.dll`** (WinMM/MCI/FLAC/CDDA and DirectDraw video support) and the verified x86 **GameFrame `glide.dll`** wrapper (Glide-to-OpenGL, Apache-2.0). **No copyrighted game data, original CD music or proprietary nGlide are distributed.**

## Quick start

1. Obtain the EXE from a successful [Windows x64 test build](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml) or a published release.
2. In a **new empty writable folder**, place `CyberGladiatorsFix.exe` next to **one original USA `CYBER.cue`/`CYBER.bin` or French `CYBERG.CUE`/`CYBERG.BIN`** disc image.
3. Double-click the EXE on Windows or run `wine CyberGladiatorsFix.exe` on Linux. The installer authenticates, extracts and checks the retail files, installs both embedded DLLs, extracts 14 FLAC music tracks and launches the game. Subsequent launches do not need the source CUE/BIN.

**Important v2.0.0 breaking change:** the old v1.0.1 `CyberGladiatorsFix/` installation is **not migrated**. Preserve old installations and saved games separately; never expect an in-place upgrade to work. Standalone ISO may be inspected with `--inspect` but **cannot be installed**.

## Installation structure (v2.0.0 source)

```text
CyberGladiatorsFix.exe
CyberGladiatorsFix.ini
CyberGladiatorsFix_installer.log
CyberGladiatorsFix.log
game/
  cyber.exe
  CYBER.EXE.bak     # original verified retail executable
  cyber.ini
  fix.dll           # our embedded WinMM/MCI/FLAC bridge
  glide.dll         # verified embedded GameFrame
  graphics/, iq/, ... original resources
music/
  Track02.flac ... Track15.flac
state/
  CyberGladiatorsFix.manifest
  CyberGladiatorsFix.sha256
  CyberGladiatorsFix.gameframe.sha256
  CyberGladiatorsFix.install-receipt.txt
  CyberGladiatorsFix.first-launch-complete.txt # created after first successful run
docs/
  README.md, LICENSE-GPL-3.0, LICENSE-APACHE, LICENSE-MIT
  CyberGladiatorsFix.ini.example, build-info.txt
```

There is no nested `CyberGladiatorsFix/` directory. `CYBER.EXE.bak` is **intentionally retained** for verification. Installer-owned `fix.dll` and `glide.dll` can be replaced only after ownership checks, **without permanent DLL backups**; unknown DLL files are never silently overwritten. On initial install, all original assets including writable `iq/` are hashed; later, changed IQ files must exist as regular files but are no longer hash-compared, while immutable assets remain SHA-256-checked.

## INI and logging

```ini
[Video]
SkipIntro=0

[Debug]
Log=0
```

The **first installation and first successful gameplay session** generate both root logs even with `Log=0`. Afterward `Log=0` leaves both files untouched. Set `Log=1` to regenerate two noncumulative diagnostic logs each session; launcher and game logs share a session ID. `SkipIntro=1` skips the opening AVI; original FLAC/CDDA music remains automatic.

## Commands

```text
CyberGladiatorsFix.exe                     auto install / verify / launch
CyberGladiatorsFix.exe --install-only      install without launching
CyberGladiatorsFix.exe --verify            verify existing installation
CyberGladiatorsFix.exe --launch-no-verify  direct launch without checks/repairs
CyberGladiatorsFix.exe --inspect image.iso
CyberGladiatorsFix.exe --inspect image.cue [track.bin|track.img]
CyberGladiatorsFix.exe --verbose           diagnostic modifier (combinable)
CyberGladiatorsFix.exe --version
CyberGladiatorsFix.exe --help              also -h
```

Normal launch selects a shared DirectDraw/MCI/AVI path for both exact retail editions, with original native video, optional skip, a centered **4:3** image in desktop-resolution borderless fullscreen, GameFrame/OpenGL ~60 FPS limiting, FLAC music, focus/Alt+Tab adaptations for Windows/Wine and a menu-only version/commit signature. Neither a special renderer option nor a separate player is required.

## Validation and limitations

The v2 architecture was merged in [PR #27](https://github.com/legluondunet/CyberGladiatorsFix/pull/27). [PR #31](https://github.com/legluondunet/CyberGladiatorsFix/pull/31) adds a signature-verified memory-only guard for **early NULL DirectDraw cleanup** on both original French and USA executables, plus an XFCE taskbar icon loaded directly from the installed retail EXE. The resulting source was merged to `main` at `4ecd56d1`. The latest field-tested `2.0.0-2b720877` build passed [Windows CI #44838205814](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/44838205814), including 32-bit runtime and 64-bit launcher compilation, automated tests and packaging checks. CI does **not** run the retail game.

Under **native Windows**, the French edition on `cbad024d` visibly played its AVI, passed Alt+Tab and `SkipIntro=1`, exercised the guarded early cleanup and exited normally. The American edition on `fdca004d` passed a fresh retail CUE/BIN install, visibly played its AVI, exercised the US guarded cleanup, reached the menu/GameFrame/FLAC and exited normally. Both are v2-layout builds, but earlier than the icon-only `2b720877` change.

Under **Wine-staging 11.19 / XFCE**, the final `2b720877` candidate ran both FR and US original editions with native MCI AVI, GameFrame/OpenGL, FLAC and normal exit. The **original game icon was visually confirmed** in the XFCE taskbar on USA. The French session reused an existing verified installation and skipped the AVI early with Space. Prior Wine FR runs verified `Log=0` file preservation and noncumulative `Log=1` sessions.

**Scope limits:** do not infer full unskipped AVI playback from an early Space bypass; the final tagged binary, every latest-build Windows `SkipIntro`/repeat permutation, audible quality and all other host combinations have not been independently validated. See [v2 readiness](docs/RELEASE_READINESS_v2.0.0.md).

## Releases

- **Latest published:** [v1.0.1](https://github.com/legluondunet/CyberGladiatorsFix/releases/tag/v1.0.1) (unchanged)
- **Prepared:** v2.0.0 — pending guarded manual publication; **not yet published**
- [v2.0.0 release notes](docs/RELEASE_NOTES_v2.0.0.md)
- [v2.0.0 release readiness](docs/RELEASE_READINESS_v2.0.0.md)
- [Guarded manual release process](docs/RELEASE_PROCESS.md)
- [Detailed installation-layout record](docs/UNIFIED_LAYOUT_PLAN.md)

The release ZIP contains the one EXE, README.txt, GPL/Apache/MIT licenses and SHA256SUMS (six flat files). Public release creation requires **separate, explicit maintainer approval**: merging a PR never publishes a release. Existing v1.0.0 and v1.0.1 assets remain untouched.

CyberGladiatorsFix and fix.dll: GPL-3.0-or-later; GameFrame: Apache-2.0; isomage: MIT.
