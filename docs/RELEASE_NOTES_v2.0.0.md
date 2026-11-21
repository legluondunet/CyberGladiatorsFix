# CyberGladiatorsFix v2.0.0 — major layout release

**Prepared release notes: the stable release has not yet been published.** CyberGladiatorsFix is a GPL-3.0-or-later single-file Windows x64 installer/launcher for the original *Cyber Gladiators* (1996) PC game, also usable with Wine/Proton. Only authenticated original **US and French CUE/BIN** media are supported for installation. Game data and copyrighted CD music are **not distributed**.

## Breaking change: new installation required

**v2.0.0 does not migrate v1.0.1 installations.** Put the new CyberGladiatorsFix.exe and original CUE/BIN in an empty writable directory. Do not replace a previous installation or assume old saves will be moved. Existing v1.x installations and the published v1.0.0/v1.0.1 downloads remain untouched.

## New architecture

```text
CyberGladiatorsFix.exe
CyberGladiatorsFix.ini
CyberGladiatorsFix_installer.log
CyberGladiatorsFix.log
game/    cyber.exe, CYBER.EXE.bak, cyber.ini, fix.dll, glide.dll, original assets
music/   Track02.flac ... Track15.flac
state/   installed resource manifest, DLL hashes, first-run/installation receipts
docs/    README, GPL/Apache/MIT licenses, example INI and build information
```

No extra `CyberGladiatorsFix/` folder. The renamed x86 WinMM/MCI/FLAC bridge **`fix.dll`** and GameFrame **`glide.dll`** live beside `game/cyber.exe`. Installer-owned DLL replacements do not retain permanent backups; unknown DLLs are never overwritten. The original retail `CYBER.EXE.bak` is deliberately retained and hash-checked. The installer retains strict immutable-asset hash checks and checks game-writable IQ resources by presence after initial hash verification.

## Default logging

```ini
[Video]
SkipIntro=0

[Debug]
Log=0
```

The **first installation and first successful game execution** create both separate logs beside the launcher. With `Log=0`, subsequent sessions leave the logs unmodified. With `Log=1`, both logs are regenerated each run, never cumulative; the launcher and runtime session IDs match. FLAC/CDDA stays enabled.

## Kept features

Verified CUE/BIN extraction, one-click installation and launch, native MCI/Cinepak 4:3 AVI with skip, borderless fullscreen desktop-resolution 4:3 GameFrame/OpenGL, ~60 FPS limit, lossless CDDA converted to 14 FLAC tracks, Windows/Wine focus and Alt+Tab support, and `--verify`, `--install-only`, `--launch-no-verify`, `--inspect`, `--verbose`, `--version` and `--help`. Standalone ISO may be inspected but not installed.

## DirectDraw startup and taskbar improvements

[Merged PR #31](https://github.com/legluondunet/CyberGladiatorsFix/pull/31) corrects an early game-owned DirectDraw cleanup that could dereference a null COM surface on native Windows. The authentic French and USA executables both have the vulnerable codepath. A **memory-only, edition-signature-checked guard** skips the cleanup only when the surface or DirectDraw global has not been initialized; the original verified retail executable files are not rewritten for this guard. The user-controlled `SkipIntro=1` option continues to bypass the movie without requiring the guard.

For Wine/XFCE, the original game HWND now receives small and large taskbar icons extracted **at runtime** from the locally installed retail `CYBER.EXE`; a system icon is used only if the original has none. No copyrighted icon is bundled and no extra rendering/overlay window is created. The maintainer confirmed the original icon appears in XFCE.

## Tested installations and compatibility (20–21 November 2026)

- **French retail / native Windows:** `2.0.0-cbad024d` visibly played the startup AVI, with Alt+Tab accepted, and separately validated `SkipIntro=1`. The early COM cleanup protection intercepted the known null-pointer path; multiple Windows FR sessions exited normally. This predates the final icon-only GameFrame change.
- **USA retail / native Windows:** `2.0.0-fdca004d` extracted an authentic US CUE/BIN in the new layout (341 original game files, 14 FLAC and 354 CD resources successfully checked). The USA minidumps established the same premature cleanup crash as FR; the shared guard then intercepted it in the live run. The maintainer visually confirmed the startup AVI and the log confirmed movie → OpenGL/menu, FLAC tracks 12 and 7, and normal exit after 19.88 seconds. A Space key press interrupted that movie, so this is not a full-length playback validation.
- **USA retail / Wine-staging 11.19 with XFCE:** latest field-tested `2.0.0-2b720877` played AVI through the native MCI bridge, recovered window focus during the film, reached GameFrame/menu, started FLAC playback and exited normally after 22.58 seconds. The maintainer **visually confirmed the taskbar icon** loaded from the original retail executable.
- **French retail / Wine-staging 11.19:** latest `2.0.0-2b720877` verified an existing no-disc installation (356 resources: 256 immutable SHA-256 checked and 100 IQ presence-only), started AVI, skipped it early with Space, returned to the menu, started FLAC track 12 and exited normally after 6.54 seconds. Earlier FR Wine runs also verified original disc installation, noncumulative logs and default `Log=0` behavior.
- **CI:** latest `2b720877` [Windows build #44838205814](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/44838205814) passed the x64 launcher, x86 GameFrame and WinMM modules, automated tests and packaging audits. PR #31 is merged at `main` commit `4ecd56d1`. CI is not a substitute for real-game testing.

**Not independently confirmed:** subjective music quality by ear; every Windows FR/USA combination of `SkipIntro`, repeated launches and Alt+Tab on the exact latest icon-build revision; uninterrupted full-duration playback during quick-skip sessions; untested graphics hardware/OS versions; or execution of the not-yet-built stable `v2.0.0` tag binary on a real host. Do not describe these as validated.

Publication is **manual and separately approved** from a source merge. The protected workflow will build the exact selected `main` commit, test the flat six-file distribution and its five SHA-256 hashes, create the `v2.0.0` tag and publish the ZIP plus a separate ZIP SHA-256 file. The package includes no copyrighted game EXE, original assets or extracted CD tracks.

License: CyberGladiatorsFix and fix.dll GPL-3.0-or-later, GameFrame Apache-2.0, isomage MIT.

Source: https://github.com/legluondunet/CyberGladiatorsFix
