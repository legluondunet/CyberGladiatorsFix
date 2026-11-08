# CyberGladiatorsFix v0.2.13 — release notes (draft)

**Status: prepared draft, not yet published.** First public-release candidate for the original 1996 **Cyber Gladiators** PC game. Planned source tag: \`v0.2.13\` (only after final audit PR merge and acceptance). The version string includes the *actual short source commit* for the distributed build.

## Overview

CyberGladiatorsFix provides a **one-click installer, compatibility patch and launcher** for authenticated **US and French** original CUE/BIN disc images, targeting modern Windows x64 and tested Wine setups. It runs the original retail game: **no original game files or CD music are distributed**.

The download is one Windows x64 executable, \`CyberGladiatorsFix.exe\`, embedding two individually verified x86 libraries: the GPLv3 WinMM/MCI/FLAC bridge and the Apache-2.0 open-source GameFrame Glide/OpenGL wrapper.

## Features

- **Automatic original-disc installation:** detect an adjacent supported US or FR CUE/BIN, identify the retail edition by cryptographic fingerprints, extract and verify game resources, apply the edition-specific speed/CD checks and set sensible original game settings. Original CUE/BIN files remain unchanged. Install once; next run launches directly without the CD image.
- **Original audio:** capture disc CDDA tracks losslessly as 44.1 kHz / 16-bit stereo FLAC, play them through the embedded WinMM bridge. Existing verified audio files migrate without lossy conversion.
- **Glide on modern displays:** built-in GameFrame, OpenGL output, 640×480 original logical resolution shown in a centered **4:3** viewport without stretching, fullscreen borderless at the desktop resolution, approximately **60 FPS** limit. Preserve user-installed \`glide.dll\` via SHA-qualified backup and restore.
- **Original Cinepak introduction:** native WinMM/MCI AVI playback with smooth fullscreen handoff to the first successful OpenGL frame, black background during transitions, keyboard skip, and host-appropriate Alt+Tab and focus management.
- **Single shared DirectDraw video compatibility engine** for US/FR under both native Windows and Wine: virtualize the game's exclusive **640×480×16** logical DirectDraw surfaces without requesting a physical monitor-mode change; leave host-specific window-manager focus code isolated.
- **In-game build signature:** \`CyberGladiatorsFix v<version>-<8-character commit>\`, subtly shown without a frame at the **upper-right 4-pixel edge** of the fighter-selection menu's original 4:3 viewport; automatically hidden in gameplay.
- **Integrity & diagnostics:** exact edition/resource checks, original executable backup, fail-closed unknown file replacement, strict \`--verify\`, and **one session log** next to the launcher. Optional \`AllowModifiedIQ=1\` relaxes only recognized same-sized IQ changes during *launch*, never strict \`--verify\`.

## Verified combinations

| Retail edition | Runtime | Field evidence |
| --- | --- | --- |
| US | Native Windows 11 | Normal startup, original AVI, GameFrame, audio, focus and exit tested on PR #6; the earlier signature was checked on native Windows. Final upper-right position requires last smoke check. |
| FR | Native Windows 11 | Clean original CUE/BIN installation, native AVI and shared engine, controls, audio and exit tested on PR #6; the earlier signature was checked on native Windows. Final upper-right position requires last smoke check. |
| US | Wine-staging 11.17 / XFCE | Clean CUE/BIN install, 14 FLAC tracks, 4:3 fullscreen, original MCI, menu signature **visible/hidden at correct times**, steady ~59.5 FPS and clean exit; user confirmed final upper-right position. |
| FR | Wine-staging 11.17 / XFCE | Clean install, second launch, full ~39s intro, video skip, Alt+Tab, audio and clean exit; user confirmed final upper-right signature. |

These are real-world acceptance results, **not a guarantee on all Windows, Wine, Proton, GPU and window-manager configurations**.

## Install / use

1. Download the final release asset containing \`CyberGladiatorsFix.exe\`, two open-source license notices, \`README.txt\`, and \`SHA256SUMS.txt\`; verify the checksums.
2. Put \`CyberGladiatorsFix.exe\` in a new writable folder beside **one original US or FR \`.cue\`** and its referenced \`.bin\` file(s).
3. Double-click \`CyberGladiatorsFix.exe\` on Windows, or run \`wine CyberGladiatorsFix.exe\` under Wine. The launcher automatically installs, verifies and launches the game. Subsequent runs do not need the CD image.
4. Use \`CyberGladiatorsFix.exe --verify\` for strict verification and \`--help\` for available commands.

## Supported / unsupported

- **Supported retail images:** only authenticated US and French original CUE/BIN; some verified single-BIN discs with a 150-sector all-zero omitted INDEX 00 pregap are handled.
- **Not supported:** standalone ISO input, unverified executable revisions, other languages, arbitrary CD rips without audio tracks, or running without a legitimately obtained retail copy.
- The launcher is **Windows x64** and runs through supported Wine/Proton. OpenGL-capable hardware and working Wine/Windows MCI are required.
- The game and its proprietary assets, audio, nGlide, DXVK or any copyrighted font texture are **not bundled**. Legacy developer diagnostic paths are not supported end-user options.

## Licensing and acknowledgments

- **CyberGladiatorsFix:** GPL-3.0-or-later.
- **GameFrame** by its contributors: Apache-2.0, pinned to \`tanepiper/game-frame@f195a074a63cc150673c38becdec696163b0b3d1\`. Original project: https://github.com/tanepiper/game-frame.
- See \`LICENSE-GPL-3.0\` and \`LICENSE-APACHE\` inside the download. This is a fan-made compatibility tool; it is not affiliated with the original game's publisher.

### Source and technical references

- Main repository: https://github.com/legluondunet/CyberGladiatorsFix
- [Pre-release acceptance and source audit](RELEASE_READINESS.md)
- [Final tag/package/release procedure](RELEASE_PROCESS.md)
- [Merged Windows/Wine common-engine PR #6](https://github.com/legluondunet/CyberGladiatorsFix/pull/6), [menu signature PR #8](https://github.com/legluondunet/CyberGladiatorsFix/pull/8), [final position PR #9](https://github.com/legluondunet/CyberGladiatorsFix/pull/9)

**legluondunet assisted by ChatGPT (OpenAI)**
