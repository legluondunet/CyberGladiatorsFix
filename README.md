# CyberGladiatorsFix

**Modern one-click installer, compatibility patch and launcher for Cyber Gladiators (1996).**

[![GameFrame Windows CI](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml/badge.svg)](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml)

CyberGladiatorsFix is a GPL-3.0-or-later Windows x64 launcher compatible with Wine/Proton. It bundles two Windows x86 DLLs: **CyberGladiatorsFix.dll** (our WinMM/Cinepak video and FLAC audio bridge) and **GameFrame glide.dll** (an open-source Apache-2.0 Glide-to-OpenGL wrapper). It does not distribute Cyber Gladiators, its copyrighted assets or proprietary nGlide.

**Pre-release field validation (2026-11-08):** Merged **[PR #6](https://github.com/legluondunet/CyberGladiatorsFix/pull/6)** introduced one project-owned DirectDraw v1 COM and MCI/WM_PAINT video implementation **automatically** for authenticated US and FR retail games under native Windows and Wine-staging 11.17, with small host-specific Alt+Tab/focus adapters. **All four edition/runtime combinations were tested successfully on the default launch:** US/Windows (complete ~40s AVI), FR/Windows (fresh original-CD installation, AVI skip, Alt+Tab), US/Wine (first-frame keyboard focus, XFCE fullscreen, Alt+Tab, AVI skip), and FR/Wine (fresh `CYBERG.cue` installation, second launch, complete ~39s AVI, Alt+Tab, keyboard skip, FLAC and clean exit). The original US `CYBER.cue` with an unindexed 150-sector zero pregap was also field-validated. This remains **pre-release testing**, not a general promise of compatibility with every GPU, Wine version or window manager. See [release readiness](docs/RELEASE_READINESS.md).

## In-game version and commit signature

The pre-release GameFrame build draws `CyberGladiatorsFix v<version>-<8-character commit>` at the **upper-right edge** of the original 4:3 **fighter-selection / Control Screen**, with a four-pixel margin. It is **not** a Windows title-bar rename, topmost overlay, patch to the game EXE or a separate helper application. The launcher injects the actual build version as a child-only variable and the existing pinned x86 GameFrame OpenGL renderer draws unobtrusive pixel-font lettering **with a transparent background and no frame**, after the game's scene is composed, before the real frame swap.

Since the retail game has no exported public menu-state API, **this first build recognizes the Control Screen by the two green "Player 1" and "Player 2" captions and the green instructions beneath each**, sampled from four small rectangles of the game's actual 4:3 framebuffer. The detector requires two consecutive positive checks before displaying, immediately hides on a negative check, and logs `CYBER_MENU_SIGNATURE_PROBE` with green density scores (sampled every 20 frames). The overlay has no enclosing rectangle or opaque fill. The green-region menu detector was field-validated on French retail under Wine and native Windows 11 in PR #8. As with any image-based heuristic, other menu variants still need separate testing. No screen capture is saved and no game assets are modified.

**Menu-matched typography (experimental styling, PR #9):** Instead of the old thin white uppercase 5x7 lettering, the signature uses a broader angular 7x9 mixed-case bitmap alphabet with variable character spacing and soft scaling, inspired by the violet-grey `Ctrl + O pour Options` / `Echap pour Quitter` instructions already on the fighter-selection screen. It samples the **actual lower-left menu footer's muted violet pixels once** from the original 4:3 game framebuffer to match the edition's original color (with a fixed violet-grey fallback when no reliable samples exist). Trace `CYBER_MENU_SIGNATURE_STYLE` records sampled RGB, sample count and whether fallback was used. The text stays transparent, right-aligned against the top border of the 4:3 game image, menu-only, and never touches the game EXE or resource files. The font resemblance remains approximate; no original font extraction is required. **The custom glyphs approximate the original game's UI font; they do not claim to be the exact proprietary game font**, which is not separately available to this build.

Acceptance test: skip the intro, verify that the text appears at the fighter selection, open the Options screen if possible, start a fight and confirm the text disappears; leave/return to fighter selection and verify it reappears. Check 4:3 and 16:9 desktop geometry, Wine/XFCE Alt+Tab, keyboard, FLAC and normal exit. No special launcher command or INI setting is required.

## Quick start

1. For testing, download a successful [Windows x64 pre-release artifact](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml). The artifact includes `CyberGladiatorsFix.exe`, both open-source license notices, `README.txt`, and `SHA256SUMS.txt` to verify the downloaded files. This is **not** a published release.
2. Put **CyberGladiatorsFix.exe** beside a supported original **US `CYBER.cue`/`CYBER.bin`** or **French `CYBERG.CUE`/`CYBERG.BIN`** image in a fresh writable folder. Both are authenticated by exact executable and media hashes; the verified pregap compatibility is included in the current pre-release build.
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

**Windows/Wine DirectDraw compatibility:** The original game requests exclusive DirectDraw **640×480×16** before the AVI. The default verified-edition launcher uses one **owned COM DirectDraw v1 adapter** that preserves the logical surface/mode state without requesting a physical monitor-mode change. It is not a fake success return or a system-wide vtable patch. A previous Wine-only native-dimension forwarding mechanism remains in legacy diagnostics but is not selected during normal launch. The same MCI/WM_PAINT video renderer serves both platforms; focus adaptations are chosen internally from the host runtime.

The one EXE embeds and validates both x86 DLLs before writing them. **At launch, any existing `glide.dll` that differs from the embedded verified GameFrame version (unknown, older or malformed) is preserved as `glide.dll.bak`**, and GameFrame is installed to launch the game. **After exit, the newer GameFrame `glide.dll` remains installed; the previous version is kept in its backup**, not automatically restored. If `glide.dll.bak` already exists, the launcher never overwrites it: it chooses `glide.dll.<SHA-256-prefix>.bak`, or a numbered `.bak` variant if necessary. All backup/install/retention operations and hashes appear in the single log. An unknown file is archived regardless of its size or contents; symbolic links are moved aside without following them. Only a directory named `glide.dll` is rejected. The embedded GameFrame itself remains SHA-256/PE32 validated. Details: [Glide backup policy](docs/GLIDE_BACKUP_POLICY.md).

## Automatic common DirectDraw / MCI engine

**No platform- or edition-specific command is required.** Start `CyberGladiatorsFix.exe` normally on Windows, or `wine CyberGladiatorsFix.exe` on Linux. After verifying the retail edition, the launcher automatically selects one owned DirectDraw COM adapter, 640×480×16 logical surfaces, native MCI/WM_PAINT 4:3 AVI playback, WinMM/FLAC and first-real-OpenGL-present handoff. No physical display-mode switch is requested by the owned adapter; system DirectDraw vtables are untouched.

| Edition | Host | Common engine | Field status |
| --- | --- | --- | --- |
| FR | Native Windows | Enabled by default | **Validated on default launch**: clean `CYBERG.cue` install, single Alt+Tab entry, unchanged physical display mode, OpenGL/FLAC; full ~40s AVI on earlier test build |
| US | Native Windows | Enabled by default | **Validated on default launch**: complete ~40s AVI, single Alt+Tab entry, OpenGL/FLAC, safe Glide backup/retention |
| US | Wine-staging 11.17 | Enabled by default | **Validated on default launch**: XFCE fullscreen, first-key video skip, Alt+Tab, GameFrame/FLAC and clean exit |
| FR | Wine-staging 11.17 | Enabled by default | **Validated**: clean `CYBERG.cue` install, re-launch, full ~39s AVI, keyboard skip, Alt+Tab, GameFrame/FLAC and clean exit |

The engine is selected automatically via an internal, child-scoped `CYBER_VIRTUAL_DDRAW=1` value after exact retail SHA-256 authentication. **Users do not set this variable.** The only host-specific code concerns the operating system's window-manager behavior: native Windows temporarily hides the black game HWND from Alt+Tab during the AVI (then restores its exact style), whereas Wine uses reversible host unmap and bounded minimize/restore. Both adaptations use the **same** video/DirectDraw/WinMM components and are not alternative player implementations.

The historical A/B command aliases remain compatible with old test scripts, but they select the *same* common engine and are not required or documented as user commands. Legacy nGlide, DXVK and original Wine DirectDraw experiments remain isolated from normal gameplay. See the [post-merge cleanup audit](docs/POST_MERGE_AUDIT.md) for the retained diagnostics and deleted dead code.

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

The **US and French CYBERG retail** editions are identified by distinct exact SHA-256 profiles. All four US/FR × Windows/Wine combinations select the same owned-COM video implementation by default and have been tested on real Windows 11 / Wine-staging 11.17 hosts. Other systems require their own field verification. The US WinMM import-name migration is checked against previously pinned executable hashes; the FR implementation has its own checked preimages, import slot and known full-binary hashes. Unknown executable revisions, directory entries masquerading as `glide.dll`, unexpected manifest entries and incomplete game installations fail closed. Any different Glide file is archived with a unique `.bak` name before installing the verified bundled GameFrame; an unreadable old file can also be preserved without inspecting its contents. CUE/BIN input is read-only, and the game installation uses a staging folder. A per-installation Windows mutex prevents concurrent launch or install.

Native Windows **FR and US** GameFrame rendering and video handoff have now been field-tested on PR #6 builds; this does not establish compatibility with every GPU or Windows configuration. The sole distribution workflow runs Rust tests (including DirectDraw mode-selection and backup-integrity checks), verifies the x86 DLLs and Glide exports, checks the CLI, and smoke-tests the native Windows loader, but it does not play the game. Historical experimental branches and diagnostics remain available for regression research, not as supported user entrypoints. Read the [build-85 technical audit](docs/BUILD85_AUDIT.md) and the [pre-release checklist](docs/RELEASE_READINESS.md) for outstanding risks.

## French retail edition

The original `CYBERG.BIN` data track and `CYBER.EXE` are authenticated byte-for-byte, independently of the US media. The 15-track `CYBERG.CUE` has one MODE1/2352 data track and 14 CDDA tracks. Its game patches, verified original EXE backup and game HWND/WndProc addresses are distinct from the US ones.

On **native Windows**, the shared owned-COM mode has been field-validated with a complete ~40-second movie, single Alt+Tab entry, GameFrame handoff, FLAC, unchanged physical display mode and clean exit. It is now selected by default. The same code is automatically selected for FR/Wine as well, but **FR/Wine needs a dedicated real-machine acceptance test** before declaring that combination validated. For older diagnostic experiments, see the [historical FR test plan](docs/FR_TEST_PLAN.md).

## Licenses

CyberGladiatorsFix code is **GPL-3.0-or-later** (LICENSE). The pinned [GameFrame](https://github.com/tanepiper/game-frame) component is **Apache-2.0** (LICENSE-APACHE in the downloadable artifact). Neither the game nor its copyright-protected disc tracks are bundled. For prior experiments see [development history](docs/DEVELOPMENT_HISTORY.md) and the merged [GameFrame PR #2](https://github.com/legluondunet/CyberGladiatorsFix/pull/2). The test-artifact instructions are in [docs/DISTRIBUTION.md](docs/DISTRIBUTION.md).

*legluondunet assisted by ChatGPT (OpenAI)*
