# CyberGladiatorsFix v1.0.0 — release notes

CyberGladiatorsFix 1.0.0 is the first public release for the original 1996 **Cyber Gladiators** PC game. It supports original authenticated US and FR CUE/BIN editions. The publisher workflow creates tag `v1.0.0` at the exact successfully built and verified source commit; the distributed executable displays that commit's eight-character suffix. The listed hardware field tests document accepted combinations, not universal platform guarantees.

## Overview

CyberGladiatorsFix provides a **one-click installer, compatibility patch and launcher** for authenticated **US and French** original CUE/BIN disc images, targeting modern Windows x64 and tested Wine setups. It runs the original retail game: **no original game files or CD music are distributed**.

The download is one Windows x64 executable, `CyberGladiatorsFix.exe`, embedding two individually verified x86 libraries: the GPLv3 WinMM/MCI/FLAC bridge and the Apache-2.0 open-source GameFrame Glide/OpenGL wrapper.

## Features

- **Automatic original-disc installation:** detect an adjacent supported US or FR CUE/BIN, identify the retail edition by cryptographic fingerprints, extract and verify game resources, apply the edition-specific speed/CD checks and set sensible original game settings. Original CUE/BIN files remain unchanged. Install once; next run launches directly without the CD image.
- **Original audio:** capture disc CDDA tracks losslessly as 44.1 kHz / 16-bit stereo FLAC, play them through the embedded WinMM bridge. Existing verified audio files migrate without lossy conversion.
- **Glide on modern displays:** built-in GameFrame, OpenGL output, 640×480 original logical resolution shown in a centered **4:3** viewport without stretching, fullscreen borderless at the desktop resolution, approximately **60 FPS** limit. At launch, reuse `glide.dll` if its SHA-256 matches the bundled GameFrame, otherwise rename the old file to a collision-free `.bak` (without overwriting an existing backup), install the verified embedded GameFrame and keep it after exit. This remains safe for unknown/old files; the original `.bak` stays available for manual recovery.
- **Original Cinepak introduction:** native WinMM/MCI AVI playback with smooth fullscreen handoff to the first successful OpenGL frame, black background during transitions, keyboard skip, and host-appropriate Alt+Tab and focus management.
- **Single shared DirectDraw video compatibility engine** for US/FR under both native Windows and Wine: virtualize the game's exclusive **640×480×16** logical DirectDraw surfaces without requesting a physical monitor-mode change; leave host-specific window-manager focus code isolated.
- **In-game build signature:** `CyberGladiatorsFix v<version>-<8-character commit>`, subtly shown without a frame at the **upper-right 4-pixel edge** of the fighter-selection menu's original 4:3 viewport; automatically hidden in gameplay.
- **Integrity & diagnostics:** exact edition/resource checks, original executable backup, fail-closed unknown file replacement, strict `--verify`, and **one session log** next to the launcher. **CDDA/FLAC and SHA-256 checks on immutable assets are mandatory** at launch and with `--verify`; all original `iq/` files are hashed on first staged installation, then checked for presence only since the game can rewrite them; the user configuration contains only `[Video] SkipIntro` and automatically removes obsolete `[Audio]`/`[Integrity]` sections on an installed copy.

## Verified combinations

| Retail edition | Runtime | Field evidence |
| --- | --- | --- |
| US | Native Windows 11 | Original AVI, GameFrame, audio, focus and exit tested on PR #6; latest `.bak` and INI policy from PR #11 has not had a separate US/Windows field log in this cycle. |
| FR | Native Windows 11 | Clean original CUE/BIN installation and common engine tested on PR #6; **2026-11-09 PR #11 field tests** confirmed the final upper-right signature, MCI/AVI and FLAC, safe `.bak` name collision, unchanged 1920×1200 desktop mode, successful exit and second same-build launch with `action=keep_existing`. |
| US | Wine-staging 11.17 / XFCE | Clean CUE/BIN install previously validated. A further **2026-11-09 PR #12** run (`0.2.13-731aa136`) verified that the game's `G:\cyber\splash4.avi` request is redirected to the authenticated local `splash4.avi` even with the CD mounted; full ~39-second AVI, Alt+Tab, first OpenGL frame, menu signature, FLAC playback, ~59.5 FPS and clean exit were observed. User reported no visual issues. |
| FR | Wine-staging 11.17 / XFCE | Clean install, second launch, full ~39s intro, video skip, Alt+Tab, audio and clean exit; user confirmed final upper-right signature. |

These are real-world acceptance results from multiple candidate commits, not a guarantee on every machine. **The merged v1.0.0 source commit `bc55394e` was additionally tested on US/Wine-staging 11.17**: `--verify` checked 255 immutable resources by SHA-256 and 100 game-writable IQ files for safe presence, then the game started, presented the 4:3 video and menu signature, played FLAC music, stabilized around 59.5 FPS and exited successfully. The official release workflow builds from the final source commit and independently validates its package; the downloaded release binary still merits a final native Windows smoke test. No universal Windows, Wine, Proton, GPU or window-manager compatibility is claimed.

## Install / use

1. Download the final release asset containing `CyberGladiatorsFix.exe`, three license notices (GPL, Apache-2.0 and MIT), `README.txt`, and `SHA256SUMS.txt`; verify the checksums.
2. Put `CyberGladiatorsFix.exe` in a new writable folder beside **one original US or FR `.cue`** and its referenced `.bin` file(s).
3. Double-click `CyberGladiatorsFix.exe` on Windows, or run `wine CyberGladiatorsFix.exe` under Wine. The launcher automatically installs, verifies and launches the game. Subsequent runs do not need the CD image.
4. Use `CyberGladiatorsFix.exe --verify` for strict verification of original EXE, immutable resources and CD audio plus safe presence of game-writable IQ files; use `--help` for available commands.

## Supported / unsupported

- **Supported retail images:** only authenticated US and French original CUE/BIN; some verified single-BIN discs with a 150-sector all-zero omitted INDEX 00 pregap are handled.
- **Not supported:** standalone ISO input, unverified executable revisions, other languages, arbitrary CD rips without audio tracks, or running without a legitimately obtained retail copy.
- The launcher is **Windows x64** and runs through supported Wine/Proton. OpenGL-capable hardware and working Wine/Windows MCI are required.
- The game and its proprietary assets, audio, nGlide, DXVK or any copyrighted font texture are **not bundled**. Legacy developer diagnostic paths are not supported end-user options.

## Licensing and acknowledgments

- **CyberGladiatorsFix:** GPL-3.0-or-later; its WinMM bridge is bundled inside the single launcher.
- **GameFrame** by its contributors: Apache-2.0, pinned to `tanepiper/game-frame@f195a074a63cc150673c38becdec696163b0b3d1`. Original project: https://github.com/tanepiper/game-frame.
- **isomage 2.1.0** by Jack Danger: MIT; original upstream copyright/permission terms in `LICENSE-MIT`.
- See `LICENSE-GPL-3.0`, `LICENSE-APACHE` and `LICENSE-MIT` inside the download. This is a fan-made compatibility tool; it is not affiliated with the original game's publisher.

### Source and technical references

- Main repository: https://github.com/legluondunet/CyberGladiatorsFix
- [Pre-release acceptance and source audit](RELEASE_READINESS.md)
- [Final tag/package/release procedure](RELEASE_PROCESS.md)
- [Merged common-engine PR #6](https://github.com/legluondunet/CyberGladiatorsFix/pull/6), [menu signature PRs #8](https://github.com/legluondunet/CyberGladiatorsFix/pull/8) and [#9](https://github.com/legluondunet/CyberGladiatorsFix/pull/9), [archive and internal-settings PR #11](https://github.com/legluondunet/CyberGladiatorsFix/pull/11), [local-AVI-path fix PR #12](https://github.com/legluondunet/CyberGladiatorsFix/pull/12)

**legluondunet assisted by ChatGPT (OpenAI)**
