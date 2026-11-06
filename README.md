# CyberGladiatorsFix

**Modern one-click installer, compatibility patch and launcher for Cyber Gladiators (1996).**

[![GameFrame Windows CI](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml/badge.svg)](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml)

CyberGladiatorsFix is a GPL-3.0-or-later Windows x64 launcher compatible with Wine/Proton. It bundles two Windows x86 DLLs: **CyberGladiatorsFix.dll** (our WinMM/Cinepak video and FLAC audio bridge) and **GameFrame glide.dll** (an open-source Apache-2.0 Glide-to-OpenGL wrapper). It does not distribute Cyber Gladiators, its copyrighted assets or proprietary nGlide.

**Status (build 85, 2026-11-06):** Fullscreen 4:3 intro and the first-real-OpenGL-frame handoff were validated under **Wine-staging 11.17** (with and without a virtual desktop) and **Proton reporting Wine 11.0** (full uninterrupted AVI playback). Both runs exited cleanly and restored the prior Glide DLL. The intended frame limit is **60 FPS**, not a guarantee of sustained performance. Native Windows 10/11 gameplay and first-time CUE/BIN installation still require end-to-end validation; CI cannot play the game.

## Quick start

1. Download the latest successful [GameFrame Windows x64 artifact](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml).
2. Put **CyberGladiatorsFix.exe** next to your own US retail CD **CUE** file and the **BIN** tracks it references in a writable directory.
3. **Double-click CyberGladiatorsFix.exe**. Under Wine, run **wine CyberGladiatorsFix.exe**.

**There is no need for --launch or an additional player.**

On first run the launcher searches for the supported CD image, validates the US game edition, extracts the game into a temporary staging folder, verifies the files, installs the compatibility patch and both embedded DLLs, then launches. On later runs it checks the existing installation first and launches directly; the CUE/BIN need not be present.

If an installation exists but does not pass its checks, the launcher refuses to run rather than deleting or silently replacing user files. Only one CUE image in the launcher's directory is accepted without an explicit path. **Standalone ISO input is not supported yet.** Data-only dumps cannot provide the original CD music.

## Installed layout

~~~text
CyberGladiators/
  CyberGladiatorsFix.exe
  CyberGladiatorsFix.log
  original-disc.cue
  original-disc*.bin
  CyberGladiatorsFix/
    cyber.exe
    CYBER.EXE.bak
    glide.dll                     (GameFrame, x86)
    CyberGladiatorsFix.dll       (WinMM/CDDA, x86)
    CyberGladiatorsFix.ini
    CyberGladiatorsFix.manifest
    cyber.ini
    music/
      Track02.flac
      Track03.flac
      ...
    iq/
    graphics/
    ...                          (original game resources)
~~~

The CD audio is extracted without resampling to **FLAC, 44.1 kHz, stereo, 16-bit**, in the **music/** directory. Existing verified root-level TrackNN.flac entries are migrated without re-encoding and their old manifest is backed up. Original IQ files are never modified by the launcher.

GameFrame renders the original **640×480** Glide game at the current display resolution, preserving the **4:3** aspect ratio with black side bars and a **60 FPS** frame limit. The original Cinepak AVI plays via the native WinMM/MCI player in a temporary fullscreen window. The handoff to the game is synchronized with the **first successful OpenGL SwapBuffers**, eliminating desktop flashes and the temporary 640×480 game window. A bounded timeout prevents a permanent black screen.

**Wine/Proton DirectDraw compatibility:** The 1996 game requests exclusive DirectDraw **640×480×16** twice before the AVI. On Wine only, the verified x86 bridge forwards these calls to the **real DirectDraw implementation**, substituting the captured desktop width and height while retaining the requested 16-bit color depth. This prevents Wine's desktop **dimensions** from collapsing to 640×480 and allows fullscreen 4:3 video without changing the game's internal rendering resolution. This is **not** a simulated successful DirectDraw call and is not enabled on native Windows. The observed display geometry remained 1920×1080 in the validated sessions; native depth or physical display-mode transitions have **not** been independently ruled out.

The one EXE embeds and validates both x86 DLLs before writing them. The GameFrame DLL remains installed after a fresh first run. If the player had a different valid glide.dll, it is backed up byte-for-byte and restored at the end of gameplay.

## Commands

~~~text
CyberGladiatorsFix.exe               Auto-install/verify and play
CyberGladiatorsFix.exe <disc.cue>    Use specified CUE/BIN image
CyberGladiatorsFix.exe --install-only   Install/upgrade without starting game
CyberGladiatorsFix.exe --verify      Strict integrity check
CyberGladiatorsFix.exe --max-graphics
CyberGladiatorsFix.exe --check-bundle
CyberGladiatorsFix.exe --upgrade-cdda
CyberGladiatorsFix.exe --version
CyberGladiatorsFix.exe --help
~~~

For normal use only a double-click is necessary. The legacy nGlide diagnostics remain internally available but are not part of the recommended experience.

## Configuration and integrity

The **CyberGladiatorsFix/CyberGladiatorsFix.ini** text file is distinct from the game's binary **cyber.ini**:

- **[Video] SkipIntro=0** — play the original introduction (default).
- **[Audio] EnableCDDA=1** — virtual CD audio using music/TrackNN.flac (default).
- **[Integrity] AllowModifiedIQ=0** — strict verification (default). Setting 1 allows only same-size differences in known original iq/iqNN.dat files during launch, without modifying them or trusting the altered hashes. The **--verify** command stays strict.

A single **CyberGladiatorsFix.log**, next to the EXE, records the system and display configuration, effective game options, integrity checks, video/Glide/FLAC behavior and normal or abnormal exit. No separate video player log is created.

### Compatibility and safety

Only the SHA-256-identified **US retail** game is supported. The renamed WinMM import name is checked by reversing just that name to the previously pinned executable hashes. Unknown executable revisions, unknown DLLs, unsafe symlinks, unexpected manifest entries and incomplete installs fail closed. CUE/BIN input is read-only, and the game installation uses a staging folder. A per-installation Windows mutex prevents concurrent launch or install.

Native Windows game rendering has **not yet been verified in the field**. GitHub Actions runs Rust tests (including DirectDraw mode-selection and backup-integrity checks), validates the x86 DLLs and Glide exports, and smoke-tests the native Windows loader, but does not play the game. The detailed build-85 audit and its remaining risks are recorded in [BUILD85_AUDIT.md](docs/BUILD85_AUDIT.md).

## Licenses

CyberGladiatorsFix code is **GPL-3.0-or-later** (LICENSE). The pinned [GameFrame](https://github.com/tanepiper/game-frame) component is **Apache-2.0** (LICENSE-APACHE in the downloadable artifact). Neither the game nor its copyright-protected disc tracks are bundled. For prior experiments see docs/DEVELOPMENT_HISTORY.md and [PR #2](https://github.com/legluondunet/CyberGladiatorsFix/pull/2).

*legluondunet assisted by ChatGPT (OpenAI)*
