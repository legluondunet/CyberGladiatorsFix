# CyberGladiatorsFix

**Modern one-click installer, compatibility patch and launcher for Cyber Gladiators (1996).**

[![GameFrame Windows CI](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml/badge.svg)](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml)

CyberGladiatorsFix is a GPL-3.0-or-later Windows x64 launcher compatible with Wine/Proton. It bundles two Windows x86 DLLs: **CyberGladiatorsFix.dll** (our WinMM/Cinepak video and FLAC audio bridge) and **GameFrame glide.dll** (an open-source Apache-2.0 Glide-to-OpenGL wrapper). It does not distribute Cyber Gladiators, its copyrighted assets or proprietary nGlide.

**Pre-release status (2026-11-06):** The audited implementation is merged into `main`. Fullscreen 4:3 AVI, FLAC music, the first-real-OpenGL-frame handoff, and normal Glide restoration were verified under **Wine-staging 11.17** (with and without a virtual desktop) and **Proton reporting Wine 11.0** (builds 85 and 90). A complete uninterrupted ~40-second AVI was verified on build 85; build 90's Wine/Proton checks used the skip control and confirmed the transition. The frame limiter targets **60 FPS**, with ~59.5 FPS observed after stabilization. **This is not yet a public release:** native Windows gameplay and a first-time CUE/BIN install still require end-to-end field testing. See [Release readiness](docs/RELEASE_READINESS.md).

## Quick start

1. For testing, download a successful [Windows x64 pre-release artifact](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml). The artifact includes `CyberGladiatorsFix.exe`, both open-source license notices, `README.txt`, and `SHA256SUMS.txt` to verify the downloaded files. This is **not** a published release.
2. Put **CyberGladiatorsFix.exe** beside a supported **CUE/BIN** image in a fresh writable folder. The validated US edition is supported; the French **CYBERG.CUE/CYBERG.BIN** path is experimental in PR #5 until field testing is complete.
3. **Double-click CyberGladiatorsFix.exe**. Under Wine, run **wine CyberGladiatorsFix.exe**.

**There is no need for --launch or an additional player.**

On first run the launcher searches for the supported CD image, identifies the original US or French executable by SHA-256, verifies edition-specific media fingerprints, extracts into a temporary staging folder, verifies the files, installs the edition-specific compatibility patch and both embedded DLLs, then launches. On later runs it checks the existing installation first and launches directly; the CUE/BIN need not be present.

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

The validated **US retail** edition and an **experimental French CYBERG retail** edition are identified by distinct SHA-256 profiles. The French profile is awaiting real Wine/Proton gameplay tests and is **not yet a stable supported release**. The US WinMM import-name migration is checked against previously pinned executable hashes; the FR implementation has its own checked preimages, import slot and known full-binary hashes. Unknown executable revisions, unknown DLLs, unsafe symlinks, unexpected manifest entries and incomplete installs fail closed. CUE/BIN input is read-only, and the game installation uses a staging folder. A per-installation Windows mutex prevents concurrent launch or install.

Native Windows game rendering has **not yet been verified in the field**. The sole distribution workflow runs Rust tests (including DirectDraw mode-selection and backup-integrity checks), verifies the x86 DLLs and Glide exports, checks the CLI, and smoke-tests the native Windows loader, but it does not play the game. Historical experimental branches and diagnostics remain available for regression research, not as supported user entrypoints. Read the [build-85 technical audit](docs/BUILD85_AUDIT.md) and the [pre-release checklist](docs/RELEASE_READINESS.md) for outstanding risks.

## French retail edition (experimental PR #5)

The original French `CYBERG.BIN` data track and `cyber.exe` have been authenticated byte-for-byte. The 15-track `CYBERG.CUE` layout contains one MODE1/2352 data track and 14 CDDA tracks. French-specific `CYBER.EXE` patch offsets, backed-up executable SHA-256 hashes, and the game HWND / WndProc addresses are separate from the US ones.

For a first test, use **a separate, empty folder** containing the experimental `CyberGladiatorsFix.exe`, `CYBERG.CUE` and `CYBERG.BIN`. Do not put two CUEs beside the launcher. Test the native 4:3 introduction without skipping, menu, gameplay speed and input, FLAC tracks, Alt+Tab, clean exit, and a second boot without the CUE/BIN. Check `--verify` and share the single `CyberGladiatorsFix.log`. The original IQ data must not be edited by the launcher.

See [FR test procedure](docs/FR_TEST_PLAN.md) and [original-disc/patch research](docs/FRENCH_EDITION_RESEARCH.md). The French edition must pass Wine/Proton and a separate US non-regression test before this draft PR can merge.

## Licenses

CyberGladiatorsFix code is **GPL-3.0-or-later** (LICENSE). The pinned [GameFrame](https://github.com/tanepiper/game-frame) component is **Apache-2.0** (LICENSE-APACHE in the downloadable artifact). Neither the game nor its copyright-protected disc tracks are bundled. For prior experiments see [development history](docs/DEVELOPMENT_HISTORY.md) and the merged [GameFrame PR #2](https://github.com/legluondunet/CyberGladiatorsFix/pull/2). The test-artifact instructions are in [docs/DISTRIBUTION.md](docs/DISTRIBUTION.md).

*legluondunet assisted by ChatGPT (OpenAI)*
