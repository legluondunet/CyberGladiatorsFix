# CyberGladiatorsFix

**Modern one-click installer, compatibility patch and launcher for Cyber Gladiators (1996).**

[![GameFrame Windows CI](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml/badge.svg)](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml)

CyberGladiatorsFix is a GPL-3.0-or-later Windows x64 launcher compatible with Wine/Proton. It bundles two Windows x86 DLLs: **CyberGladiatorsFix.dll** (our WinMM/Cinepak video and FLAC audio bridge) and **GameFrame glide.dll** (an open-source Apache-2.0 Glide-to-OpenGL wrapper). It does not distribute Cyber Gladiators, its copyrighted assets or proprietary nGlide.

**Pre-release status (2026-11-08):** The previous installer and GameFrame implementation are on `main`. The still-open **[PR #6](https://github.com/legluondunet/CyberGladiatorsFix/pull/6)** unifies project-owned DirectDraw v1 COM and MCI/WM_PAINT virtual-video code for **verified FR/native Windows, US/native Windows and US/Wine-staging 11.17**, with native/Wine-specific focus handling. Those three experimental combinations have been field-tested, including single-window Alt+Tab; US/native Windows received a short (~14s) intro/gameplay smoke test. The original US `CYBER.cue` with an unindexed 150-sector zero pregap is now recognized and has been field-validated. CI is green, but PR #6 remains **unmerged**, this is **not a public release**, and FR/Wine using the owned-COM route is not yet enabled or validated. See [release readiness](docs/RELEASE_READINESS.md).

## Quick start

1. For testing, download a successful [Windows x64 pre-release artifact](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml). The artifact includes `CyberGladiatorsFix.exe`, both open-source license notices, `README.txt`, and `SHA256SUMS.txt` to verify the downloaded files. This is **not** a published release.
2. Put **CyberGladiatorsFix.exe** beside a supported original **US `CYBER.cue`/`CYBER.bin`** or **French `CYBERG.CUE`/`CYBERG.BIN`** image in a fresh writable folder. Both are authenticated by exact executable and media hashes; the new pregap tolerance is included in the PR #6 test build.
3. **Double-click CyberGladiatorsFix.exe**. Under Wine, run **wine CyberGladiatorsFix.exe**.

**There is no need for --launch or an additional player.**

On first run the launcher searches for the supported CD image, identifies the original US or French executable by SHA-256, verifies edition-specific media fingerprints, extracts into a temporary staging folder, verifies the files, installs the edition-specific compatibility patch and both embedded DLLs, then launches. On later runs it checks the existing installation first and launches directly; the CUE/BIN need not be present.

If an installation exists but does not pass its checks, the launcher refuses to run rather than deleting or silently replacing user files. Only one CUE image in the launcher's directory is accepted without an explicit path. **Standalone ISO input is not supported yet.** Data-only dumps cannot provide the original CD music.

**CUE/BIN compatibility:** Some single-BIN CD dumps (including the supplied CYBER.cue/CYBER.bin) store an actual two-second zero-filled pause before the first audio track but omit its INDEX 00. For a 2352-byte raw data track followed by AUDIO in the same BIN, CyberGladiatorsFix recognizes **exactly 150 consecutive all-zero sectors immediately preceding the first audio INDEX 01**. It excludes only that verified pause from the extracted ISO data; the original CUE and BIN are never modified, CDDA remains bounded by each declared INDEX 01, and every remaining data-sector sync/mode plus the authenticated retail EXE and resource hashes must still pass strict validation. Nonzero or malformed sectors are never silently skipped. Use --inspect-disc CYBER.cue to verify a disc read-only before installation.

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

## Shared project-owned DirectDraw / MCI test profiles (PR #6)

**These profiles are explicit opt-ins in the PR #6 test artifact, not enabled by normal double-click launch.** They share the same project-owned COM DirectDraw v1 adapter, 640×480×16 logical surfaces, 4:3 native MCI/WM_PAINT playback, FLAC WinMM bridge and first-real-OpenGL-frame handoff. No physical display mode is requested by the virtualizer; no system DirectDraw vtables are patched. An authenticated original retail executable determines the US/FR address profile, and the host determines the minimal Alt+Tab/focus policy.

| Verified retail edition | Host | Test command | Field result |
| --- | --- | --- | --- |
| FR | Windows native | `CyberGladiatorsFix.exe --launch --fr-borderless-test` | Full AVI (~40s), Alt+Tab single task-switcher window, GameFrame/FLAC and clean exit; no physical mode transition observed |
| US | Windows native | `CyberGladiatorsFix.exe --launch --us-virtual-ddraw-test` | AVI (~14s, deliberately stopped), repeated Alt+Tab, single task-switcher window, restored game HWND, GameFrame/FLAC and clean exit; same physical mode before/after |
| US | Wine-staging 11.17 | `wine CyberGladiatorsFix.exe --launch --us-virtual-ddraw-test` | AVI (~12s, deliberately stopped), single task-switcher window, repeated Alt+Tab with reversible host unmap/game minimization, GameFrame/FLAC and clean exit |
| FR | Wine/Proton | *Shared COM profile not enabled* | Requires separate runtime validation; existing normal launch is preserved |

The launcher selects the shared virtualizer via the **single child-scoped `CYBER_VIRTUAL_DDRAW`** flag. Previous FR/US/Windows/Wine flag names may remain as **diagnostic log labels**, but are no longer separate runtime switches. The Wine US profile uses a bounded hide/remap/minimize fallback; native Windows instead hides the black game HWND **from Alt+Tab only during the AVI** and restores its original extended window style at first real OpenGL present. Neither OS behavior is silently applied to the other.

The ordinary US/Wine GameFrame launch continues to support its established native DirectDraw mode-forwarding compatibility fallback; the experimental owned-COM mode explicitly disables that forwarding. Do not assume successful CI alone validates playback on untested GPUs or monitor setups.

## Commands

~~~text
CyberGladiatorsFix.exe               Auto-install/verify and play
CyberGladiatorsFix.exe <disc.cue>    Use specified CUE/BIN image
CyberGladiatorsFix.exe --install-only   Install/upgrade without starting game
CyberGladiatorsFix.exe --verify      Strict integrity check
CyberGladiatorsFix.exe --inspect-disc original.cue    Read-only disc inspection
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

The **US and French CYBERG retail** editions are identified by distinct exact SHA-256 profiles. Their verified native-Windows test paths share the same owned-COM video implementation. The FR/Wine **owned-COM** combination is not yet enabled or field-validated. The US WinMM import-name migration is checked against previously pinned executable hashes; the FR implementation has its own checked preimages, import slot and known full-binary hashes. Unknown executable revisions, unknown DLLs, unsafe symlinks, unexpected manifest entries and incomplete installs fail closed. CUE/BIN input is read-only, and the game installation uses a staging folder. A per-installation Windows mutex prevents concurrent launch or install.

Native Windows **FR and US** GameFrame rendering and video handoff have now been field-tested on PR #6 builds; this does not establish compatibility with every GPU or Windows configuration. The sole distribution workflow runs Rust tests (including DirectDraw mode-selection and backup-integrity checks), verifies the x86 DLLs and Glide exports, checks the CLI, and smoke-tests the native Windows loader, but it does not play the game. Historical experimental branches and diagnostics remain available for regression research, not as supported user entrypoints. Read the [build-85 technical audit](docs/BUILD85_AUDIT.md) and the [pre-release checklist](docs/RELEASE_READINESS.md) for outstanding risks.

## French retail edition

The original `CYBERG.BIN` data track and `CYBER.EXE` are authenticated byte-for-byte, independently of the US media. The 15-track `CYBERG.CUE` has one MODE1/2352 data track and 14 CDDA tracks. Its game patches, verified original EXE backup and game HWND/WndProc addresses are distinct from the US ones.

On **native Windows**, the explicit `--launch --fr-borderless-test` virtualized mode has been field-validated with a complete ~40-second movie, single Alt+Tab entry, GameFrame video handoff, FLAC, unchanged physical display mode and clean exit. The virtualized FR/Wine path has **not** been enabled: do not extrapolate Windows results to that path. For older diagnostic experiments, see the [historical FR test plan](docs/FR_TEST_PLAN.md).

## Licenses

CyberGladiatorsFix code is **GPL-3.0-or-later** (LICENSE). The pinned [GameFrame](https://github.com/tanepiper/game-frame) component is **Apache-2.0** (LICENSE-APACHE in the downloadable artifact). Neither the game nor its copyright-protected disc tracks are bundled. For prior experiments see [development history](docs/DEVELOPMENT_HISTORY.md) and the merged [GameFrame PR #2](https://github.com/legluondunet/CyberGladiatorsFix/pull/2). The test-artifact instructions are in [docs/DISTRIBUTION.md](docs/DISTRIBUTION.md).

*legluondunet assisted by ChatGPT (OpenAI)*
