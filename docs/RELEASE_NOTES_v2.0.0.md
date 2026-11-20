# CyberGladiatorsFix v2.0.0 — major layout release

**Release candidate only. Nothing has been published.** CyberGladiatorsFix is a GPL-3.0-or-later single-file Windows x64 installer/launcher for the original *Cyber Gladiators* (1996) PC game, also usable with Wine/Proton. Only authenticated original **US and French CUE/BIN** media are supported for installation. Game data and copyrighted CD music are **not distributed**.

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

## Verified and not verified

- Merged [PR #27](https://github.com/legluondunet/CyberGladiatorsFix/pull/27) implemented the layout. Its [post-merge Windows CI](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/44752207790) passed compilation, DLL checks, automated tests and packaging. CI does **not** run the retail game.
- **French retail/Wine-staging 11.19**: new CUE/BIN install (343 original files, 14 FLAC), game launch, 4:3 AVI/GameFrame, music and normal exit. A second run with `Log=0` preserved logs; two consecutive `Log=1` runs created distinct, noncumulative logs.
- **USA retail/Wine-staging 11.19**: new CUE/BIN install (342 original files, 14 FLAC), 4:3 AVI/GameFrame, CDDA and normal exit; initial logs produced with `Log=0`. A USA second no-disc run has not been independently documented.
- **Native Windows gameplay using this new installation layout has not been field-tested.** Earlier Windows tests apply to older layouts. The Wine sessions used the PR #27 test build `1.0.1-53beda77` containing the new code, **not** a future tagged v2.0.0 binary.

This version bump requires its own CI. Release publication also requires an **explicit separate maintainer approval**, not merely a merge. The package includes exactly one EXE, README, GPL/Apache/MIT licenses and SHA256SUMS. No original game files or unpacked DLLs.

License: CyberGladiatorsFix and fix.dll GPL-3.0-or-later, GameFrame Apache-2.0, isomage MIT.

Source: https://github.com/legluondunet/CyberGladiatorsFix
