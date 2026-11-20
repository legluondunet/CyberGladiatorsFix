# CyberGladiatorsFix v2.0.0 — major layout release

**Release candidate only. Nothing has been published.** CyberGladiatorsFix is a GPL-3.0-or-later single-file Windows x64 installer/launcher for the original *Cyber Gladiators* (1996) PC game, also usable with Wine/Proton. Only authenticated original **US and French CUE/BIN** media are supported for installation. Game data and copyrighted CD music are **not distributed**.

## Blocking Windows FR regression — investigation in progress

**Do not publish this candidate while [issue #30](https://github.com/legluondunet/CyberGladiatorsFix/issues/30) is unresolved.**
The native Windows French first-install smoke test completed successfully on
20 November 2026 (30.8 s, exit 0), with image, music and Alt+Tab return
confirmed by the maintainer. However, a subsequent trial using the **same
`2.0.0-2ae6cadb`** binary **crashed with an access violation
`0xc0000005`** after 6.638 seconds, after owned virtual DirectDraw
`SetCooperativeLevel` and before opening the AVI. Both installer logs said
`game installation not present yet`: the two attempts must not be presented
as proof of a successful repeat launch of an existing installation. The
faulting module/address is not yet known; proximity to a DirectDraw trace
does not establish its root cause. Native Windows FR launch stability
therefore remains **unresolved**, notwithstanding the successful first test.

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
- **USA retail/Wine-staging 11.19, v2.0.0 candidate `2.0.0-0f86b592` (20 November 2026)**: original CUE/BIN fresh installation, 342 original files, 14 FLAC tracks, source EXE backup and game manifest verified; 4:3 AVI, video focus loss/recovery, GameFrame/OpenGL, CDDA tracks 12 and 7, steady ~59.5 FPS, clean exit (53.6 seconds, code 0). Both first-install logs were generated with default `Log=0`. A second USA no-disc run has not independently been tested.
- **French retail / native Windows (20 November 2026, merged `main` build `2.0.0-2ae6cadb`)**: first-time authentic `CYBERG.CUE` installation succeeded (343 original game files, 14 lossless FLAC tracks, 356 CD manifest resources, 256 immutable SHA-256 checks and 100 writable IQ entries checked for presence). The game's verified original executable backup and installed runtime hash matched; AVI/MCI opened and played in 4:3, first GameFrame/OpenGL presentation completed, CDDA decoded tracks 12 and 7, and the game exited normally after 30.8 seconds (exit 0). Focus loss/return during the AVI is logged and the game's window was returned to Alt+Tab. The physical display mode stayed at 1920×1200@60 Hz throughout (0 sampled transitions), while the app used logical 1536×960 coordinates, consistent with 125% Windows display scaling; this is not a display-mode change. The initial launcher/game logs were created despite `[Debug] Log=0`. **The maintainer subsequently confirmed the image, music and return after Alt+Tab were all correct** on this native Windows FR test. This confirms visual/audio quality and focus recovery in addition to the log-based first-install/runtime checks. It does **not** demonstrate a later no-CD second launch. Native Windows USA with the new layout remains untested. Earlier Windows tests apply to older layouts. The earlier Wine sessions used the PR #27 test build `1.0.1-53beda77`. The new USA clean-install test used the version-bumped PR #28 build **`2.0.0-0f86b592`**, which also passed [Windows CI](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/44754612481). This is **not** a test of the final tagged/published v2.0.0 binary.

The version-bumped candidate and the final merged `main` commit passed Windows CI ([post-merge run #44760300708](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/44760300708)). Release publication also requires an **explicit separate maintainer approval**, not merely a merge. The package includes exactly one EXE, README, GPL/Apache/MIT licenses and SHA256SUMS. No original game files or unpacked DLLs.

License: CyberGladiatorsFix and fix.dll GPL-3.0-or-later, GameFrame Apache-2.0, isomage MIT.

Source: https://github.com/legluondunet/CyberGladiatorsFix
