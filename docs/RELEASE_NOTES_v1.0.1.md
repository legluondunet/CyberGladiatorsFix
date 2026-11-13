# CyberGladiatorsFix v1.0.1 — maintenance release

CyberGladiatorsFix **1.0.1** is a maintenance update to the published v1.0.0
release for the original **Cyber Gladiators (1996)** PC game. Supported
original retail installation images remain **US and French CUE/BIN**.
The game and original copyrighted files are **not** included.

## Changes since v1.0.0

- **Clear, separate diagnostic logs:** `CyberGladiatorsFix_installer.log`
  covers installation, updates, checks and launcher failures.
  `CyberGladiatorsFix.log` covers in-game GameFrame Glide, WinMM/MCI,
  DirectDraw, original AVI and FLAC/CDDA events. Both share a session ID.
  `--verify` and `--install-only` do not erase the previous game log.
- **`--launch-no-verify`:** explicit direct launch of an existing
  `CyberGladiatorsFix/cyber.exe` without file-presence and SHA-256
  preflight checks, extraction, module replacement or installation repair.
  This is an opt-in troubleshooting shortcut, **not** a bypass for original
  disc authentication at installation. Missing executables or dependencies
  may still cause the OS or game to fail.
- **`--inspect <image.iso|image.cue> [track.bin|track.img]`:**
  read-only inventory for an unsupported CD image. Writes
  `CyberGladiatorsFix_inspect.log` beside the launcher, with filesystem
  inventory and CUE track information where available; supply this log
  when requesting support for a new edition. Inspection does **not**
  install a game, and **ISO installation is not supported**.
- **`--verbose`** can be combined with supported commands to include
  more installation/diagnostic context. **`-h`** is equivalent to `--help`.
- **Simpler command interface:** remove `--inspect-disc`,
  `--max-graphics`, `--check-bundle`, `--upgrade-cdda`,
  positional CUE launch, and experimental/legacy nGlide, DXVK and Vulkan
  launcher aliases. The original game graphics defaults are initialized
  automatically on first install. Normal upgrades automatically preserve
  prior DLLs in collision-safe, hash-qualified backups.
- **Source cleanup:** retire the unused proprietary nGlide installation
  fallback, historical renderer A/B experiment launch paths, unreachable
  installer code and obsolete CDDA consent logic. Retain the validated
  open-source GameFrame wrapper, shared DirectDraw/MCI compatibility engine
  and safety checks.

## Retained features

- Original US/FR CUE/BIN detection, staging and authenticated extraction;
  executable/immutable-resource SHA-256 validation. Game-writable `iq/`
  files require a strict initial installation hash and safe regular-file
  presence on later launches.
- One executable containing the x86 WinMM/CDDA/FLAC bridge and a pinned
  open-source GameFrame Glide wrapper. A differing `glide.dll` is preserved
  before replacement; previous backups are not overwritten.
- Native Cinepak/WinMM introduction, keyboard video skip, 4:3 image in
  desktop-resolution borderless fullscreen, focus and Alt+Tab adaptation
  for Windows/Wine, ~60 FPS cap and menu-only version/commit signature.
- FLAC music extracted losslessly from original CDDA tracks; user INI
  option `[Video] SkipIntro` remains supported.

## Commands

```text
CyberGladiatorsFix.exe
CyberGladiatorsFix.exe --install-only
CyberGladiatorsFix.exe --verify
CyberGladiatorsFix.exe --launch-no-verify
CyberGladiatorsFix.exe --inspect image.iso
CyberGladiatorsFix.exe --inspect image.cue [track.bin|track.img]
CyberGladiatorsFix.exe --verbose
CyberGladiatorsFix.exe --version
CyberGladiatorsFix.exe --help
CyberGladiatorsFix.exe -h
```

`--verbose` is a modifier, not a separate install/launch mode.

## Validation and scope

- The v1.0.1 preparation is based on code merged through PRs
  [#24](https://github.com/legluondunet/CyberGladiatorsFix/pull/24)
  and [#25](https://github.com/legluondunet/CyberGladiatorsFix/pull/25).
- On **13 November 2026**, a real original **US CUE/BIN fresh installation**
  under **Wine-staging 11.17** with build `1.0.0-23404f9f` extracted
  342 files and 14 FLAC tracks, verified 255 immutable assets by SHA-256
  and 100 IQ assets by presence, played native AVI into first OpenGL
  presentation, showed the menu signature, played CDDA and exited
  normally (code 0). The captured session used a video skip, so it does
  not prove an uninterrupted full introduction.
- The **v1.0.1 candidate `1.0.1-691f03b3`** was also tested on
  **13 November 2026** with an *existing* USA installation under Wine 11.17.
  Normal-launch integrity pre/post-checks passed (255 immutable SHA-256
  matches and 100 writable IQ files present); the installed GameFrame and
  CDDA modules were kept because their hashes matched the embedded build.
  The native 4:3 introduction started and was skipped with Enter, the
  transition to the first OpenGL frame and menu signature succeeded,
  the audio bridge decoded track 12 and started waveOut, and the game
  exited normally (code 0 after 6.18 seconds). The two logs shared a
  session ID. This was a **short smoke test**, not a new installation
  or proof of complete introduction, extended 60 FPS gameplay or
  Alt+Tab behavior; the explicit `--verify`, `--install-only` and
  `--launch-no-verify` paths remain to be field-tested.
- [Windows CI for the exact v1.0.1 test candidate](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/43476299506)
  succeeded (97 launcher tests, 54 WinMM/CDDA tests and GameFrame/WinMM
  build and loader checks). CI does not play the retail game.
- Earlier v1.0.0 project field tests covered US/FR on native Windows 11
  and Wine. **The final tagged 1.0.1 binary still requires a smoke test**
  on actual retail media; these results are not universal OS/GPU guarantees.

## Package, licensing and publication

Download `CyberGladiatorsFix-v1.0.1-Windows-x64.zip` and its separate
`.sha256` file from the **v1.0.1 GitHub Release** once it is published.
The ZIP contains exactly six files: `CyberGladiatorsFix.exe`,
`README.txt`, `LICENSE-GPL-3.0`, `LICENSE-APACHE`,
`LICENSE-MIT` and `SHA256SUMS.txt`. The publication workflow
checks the five listed file hashes, the archive hash and the source
commit suffix in `--version`.

CyberGladiatorsFix and its WinMM bridge: **GPL-3.0-or-later**.
GameFrame Glide wrapper: **Apache-2.0**.
isomage: **MIT**. Not affiliated with the game's original publisher.

The previous **v1.0.0** release remains available unchanged.

Source: https://github.com/legluondunet/CyberGladiatorsFix
