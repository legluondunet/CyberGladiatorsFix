# CyberGladiatorsFix

**Modern one-click installer, compatibility patch and launcher for Cyber Gladiators (1996).**

[![GameFrame Windows CI](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml/badge.svg)](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml)

CyberGladiatorsFix is a GPL-3.0-or-later Windows x64 launcher compatible with Wine/Proton. It bundles two Windows x86 DLLs: **CyberGladiatorsFix.dll** (our WinMM/Cinepak video and FLAC audio bridge) and **GameFrame glide.dll** (an open-source Apache-2.0 Glide-to-OpenGL wrapper). It does not distribute Cyber Gladiators, its copyrighted assets or proprietary nGlide.

**Pre-release field validation (2026-11-08):** Merged **[PR #6](https://github.com/legluondunet/CyberGladiatorsFix/pull/6)** introduced one project-owned DirectDraw v1 COM and MCI/WM_PAINT video implementation **automatically** for authenticated US and FR retail games under native Windows and Wine-staging 11.17, with small host-specific Alt+Tab/focus adapters. **All four edition/runtime combinations were tested successfully on the default launch:** US/Windows (complete ~40s AVI), FR/Windows (fresh original-CD installation, AVI skip, Alt+Tab), US/Wine (first-frame keyboard focus, XFCE fullscreen, Alt+Tab, AVI skip), and FR/Wine (fresh `CYBERG.cue` installation, second launch, complete ~39s AVI, Alt+Tab, keyboard skip, FLAC and clean exit). The original US `CYBER.cue` with an unindexed 150-sector zero pregap was also field-validated. This remains **field testing**, not a guarantee for all GPUs, Wine versions or window managers. For the current maintenance update, see the [v1.0.1 acceptance checklist](docs/RELEASE_READINESS_v1.0.1.md), [v1.0.1 notes](docs/RELEASE_NOTES_v1.0.1.md) and [manual release procedure](docs/RELEASE_PROCESS.md).

**2026-11-09 follow-up (merged PR #12):** The original US game may request `G:\cyber\splash4.avi` when a CD is mounted under Wine. CyberGladiatorsFix now routes that legacy CD-drive MCI request to the verified installed `splash4.avi`, independent of the mounted drive letter. A real Wine-staging 11.17 run of `v0.2.13-731aa136` completed the full ~39-second introduction, reversible Alt+Tab, GameFrame handoff, CDDA playback and normal exit with no visual regression reported. The subsequent merge to `main` passed the Windows CI workflow; a final release-tag build and smoke test are still required.

## Release manager: guarded v1.0.1 maintenance publication

The public [v1.0.0 release](https://github.com/legluondunet/CyberGladiatorsFix/releases/tag/v1.0.0) remains available and unchanged. After reviewing the source, green CI and real-game acceptance for US/FR on Windows/Wine, use the **manual** [Publish CyberGladiatorsFix v1.0.1](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/publish-v1.0.1.yml) workflow on **main**. Enter exactly `publish-v1.0.1` to authorize its fresh build, pinned-source checks, SHA-256/ZIP verification and publication of the new `v1.0.1` tag and assets. Nothing is published automatically when a PR is merged; see the [release procedure](docs/RELEASE_PROCESS.md). **Do not trigger the workflow before explicit release approval.**

## In-game version and commit signature

GameFrame draws `CyberGladiatorsFix v<version>-<8-character commit>` at the **upper-right edge** of the original 4:3 **fighter-selection / Control Screen**, with a four-pixel margin. It is **not** a Windows title-bar rename, topmost overlay, patch to the game EXE or a separate helper application. The launcher injects the actual build version as a child-only variable and the existing pinned x86 GameFrame OpenGL renderer draws unobtrusive pixel-font lettering **with a transparent background and no frame**, after the game's scene is composed, before the real frame swap.

Since the retail game has no exported public menu-state API, the signature uses a bounded **4:3 framebuffer detector**: the initial Control Screen is recognized by both players' green captions and instructions, while the subsequent **fighter-portrait selection screen** is recognized by the two original violet-grey bottom prompts (Options and Exit) that remain visible throughout selection. Green text is not required once portraits replace the instructions. Both footer regions must match to avoid keeping the signature into gameplay. Two consecutive positive probes are still required (every 20 frames); the label hides on the first negative probe and logs `CYBER_MENU_SIGNATURE_PROBE` with green and footer density scores. Probes read six small original-frame rectangles, with no Windows overlay, extra window or screenshot saved. The green Control Screen path was field-validated on French retail under Wine and Windows 11 in PR #8; the expanded fighter-selection detector awaits a new real-game acceptance run. As with any image-based heuristic, unrelated screens must be verified before release.

**Menu-matched typography (PR #9, accepted on both US and FR):** Instead of the old thin white uppercase 5x7 lettering, the signature uses a broader angular 7x9 mixed-case bitmap alphabet with variable character spacing and soft scaling, inspired by the violet-grey `Ctrl + O pour Options` / `Echap pour Quitter` instructions already on the fighter-selection screen. It samples the **actual lower-left menu footer's muted violet pixels once** from the original 4:3 game framebuffer to match the edition's original color (with a fixed violet-grey fallback when no reliable samples exist). Trace `CYBER_MENU_SIGNATURE_STYLE` records sampled RGB, sample count and whether fallback was used. The text stays transparent, right-aligned against the top border of the 4:3 game image, menu-only, and never touches the game EXE or resource files. The font resemblance remains approximate; no original font extraction is required. **The custom glyphs approximate the original game's UI font; they do not claim to be the exact proprietary game font**, which is not separately available to this build.

Acceptance test: skip the intro and verify the signature appears **both on the green Player 1/2 instructions and after selecting fighters while large character portraits are shown**, without flickering or vanishing during that transition. Open Options and start a fight: the signature must disappear outside selection. Return to character selection and verify it reappears. Check both US/FR when possible, 4:3 and 16:9 desktop geometry, Wine/XFCE Alt+Tab, keyboard, FLAC and clean exit. No special launcher command or INI setting is required.

## Quick start

1. For testing, download a successful [Windows x64 pre-release artifact](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml). The artifact includes `CyberGladiatorsFix.exe`, three open-source license notices (GPL, Apache-2.0 and MIT), `README.txt`, and `SHA256SUMS.txt` containing five hashes for the six distributed files. This is **not** a published release.
2. Put **CyberGladiatorsFix.exe** beside a supported original **US `CYBER.cue`/`CYBER.bin`** or **French `CYBERG.CUE`/`CYBERG.BIN`** image in a fresh writable folder. Both are authenticated by exact executable and media hashes; the verified pregap compatibility is included in the current pre-release build.
3. **Double-click CyberGladiatorsFix.exe**. Under Wine, run **wine CyberGladiatorsFix.exe**.

**There is no need for --launch or an additional player.**

On first run the launcher searches for the supported CD image, identifies the original US or French executable by SHA-256, verifies edition-specific media fingerprints, extracts into a temporary staging folder, verifies the files, installs the edition-specific compatibility patch and both embedded DLLs, then launches. On later runs it checks the existing installation first and launches directly; the CUE/BIN need not be present.

If an installation exists but does not pass its checks, the launcher refuses to run rather than deleting or silently replacing user files. Only one CUE image in the launcher's directory is accepted without an explicit path. **Standalone ISO input is not supported yet.** Data-only dumps cannot provide the original CD music.

**CUE/BIN compatibility:** Some single-BIN CD dumps (including the supplied CYBER.cue/CYBER.bin) store an actual two-second zero-filled pause before the first audio track but omit its INDEX 00. For a 2352-byte raw data track followed by AUDIO in the same BIN, CyberGladiatorsFix recognizes **exactly 150 consecutive all-zero sectors immediately preceding the first audio INDEX 01**. It excludes only that verified pause from the extracted ISO data; the original CUE and BIN are never modified, CDDA remains bounded by each declared INDEX 01, and every remaining data-sector sync/mode plus the authenticated retail EXE and resource hashes must still pass strict validation. Nonzero or malformed sectors are never silently skipped. For installation, the original CUE/BIN image is authenticated automatically before any game files are installed.

## Installed layout

~~~text
CyberGladiators/
  CyberGladiatorsFix.exe
  CyberGladiatorsFix_installer.log
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

**Windows/Wine DirectDraw compatibility:** The original game requests exclusive DirectDraw **640×480×16** before the AVI. The default verified-edition launcher uses one **owned COM DirectDraw v1 adapter** that preserves the logical surface/mode state without requesting a physical monitor-mode change. It is not a fake success return or a system-wide vtable patch. Previous Wine-only forwarding prototypes are not part of the supported launcher; only the shared validated DirectDraw adapter is selected. The same MCI/WM_PAINT video renderer serves both platforms; focus adaptations are chosen internally from the host runtime.

The one EXE embeds and validates both x86 DLLs before writing them. **At launch, any existing `glide.dll` that differs from the embedded verified GameFrame version (unknown, older or malformed) is preserved as `glide.dll.bak`**, and GameFrame is installed to launch the game. **After exit, the newer GameFrame `glide.dll` remains installed; the previous version is kept in its backup**, not automatically restored. If `glide.dll.bak` already exists, the launcher never overwrites it: it chooses `glide.dll.<SHA-256-prefix>.bak`, or a numbered `.bak` variant if necessary. All backup/install/retention operations and hashes appear in `CyberGladiatorsFix_installer.log`. An unknown file is archived regardless of its size or contents; symbolic links are moved aside without following them. Only a directory named `glide.dll` is rejected. The embedded GameFrame itself remains SHA-256/PE32 validated. Details: [Glide backup policy](docs/GLIDE_BACKUP_POLICY.md).

## Automatic common DirectDraw / MCI engine

**No platform- or edition-specific command is required.** Start `CyberGladiatorsFix.exe` normally on Windows, or `wine CyberGladiatorsFix.exe` on Linux. After verifying the retail edition, the launcher automatically selects one owned DirectDraw COM adapter, 640×480×16 logical surfaces, native MCI/WM_PAINT 4:3 AVI playback, WinMM/FLAC and first-real-OpenGL-present handoff. No physical display-mode switch is requested by the owned adapter; system DirectDraw vtables are untouched.

| Edition | Host | Common engine | Field status |
| --- | --- | --- | --- |
| FR | Native Windows | Enabled by default | **Validated on default launch**: clean `CYBERG.cue` install and common video, 4:3 menu signature, 14 FLAC tracks, no physical mode switch, clean exit; PR #11 validated unknown-Glide `.bak` collision and a second same-build launch without modifying `glide.dll` |
| US | Native Windows | Enabled by default | **Validated on default launch**: complete ~40s AVI, single Alt+Tab entry, OpenGL/FLAC, safe Glide backup/retention |
| US | Wine-staging 11.17 | Enabled by default | **Validated on default launch**: XFCE fullscreen, first-key video skip, Alt+Tab, GameFrame/FLAC and clean exit |
| FR | Wine-staging 11.17 | Enabled by default | **Validated**: clean `CYBERG.cue` install, re-launch, full ~39s AVI, keyboard skip, Alt+Tab, GameFrame/FLAC and clean exit |

The engine is selected automatically via an internal, child-scoped `CYBER_VIRTUAL_DDRAW=1` value after exact retail SHA-256 authentication. **Users do not set this variable.** The only host-specific code concerns the operating system's window-manager behavior: native Windows temporarily hides the black game HWND from Alt+Tab during the AVI (then restores its exact style), whereas Wine uses reversible host unmap and bounded minimize/restore. Both adaptations use the **same** video/DirectDraw/WinMM components and are not alternative player implementations.

**Pre-release cleanup:** the old nGlide/DXVK A/B command aliases and standalone proprietary nGlide install fallback have been removed. The supported game process always uses GameFrame. The diagnostic DLL still contains guarded troubleshooting hooks for video regressions, but ordinary launch never enables them. See the [pre-release audit](docs/PRE_RELEASE_AUDIT.md) and the [historical post-merge audit](docs/POST_MERGE_AUDIT.md).

## Commands

~~~text
CyberGladiatorsFix.exe                     Auto-install / verify / play
CyberGladiatorsFix.exe --install-only      Install or safely upgrade; do not play
CyberGladiatorsFix.exe --launch-no-verify  Launch directly, bypass checks/repairs
CyberGladiatorsFix.exe --verify            Read-only integrity checks
CyberGladiatorsFix.exe --inspect image.iso
CyberGladiatorsFix.exe --inspect image.cue [track.bin|track.img]
CyberGladiatorsFix.exe --verbose           Detailed diagnostics (combinable)
CyberGladiatorsFix.exe --version
CyberGladiatorsFix.exe --help              Also -h
~~~

For unknown retail discs, `--inspect` creates **CyberGladiatorsFix_inspect.log**
beside the launcher. Supply this report to help add support for new editions;
the inspection never installs or launches the game. Normal installation
automatically discovers a neighboring CUE/BIN; positional `<disc.cue>` launch
arguments have been removed. The optional `--verbose` modifier may be placed
before or after a command.

For normal use only a double-click is necessary; legacy nGlide/DXVK launcher switches are no longer accepted.

**Unchecked launch (opt-in):** `--launch-no-verify` starts the already-installed `CyberGladiatorsFix/cyber.exe` directly. It does **not** search for a disc, install files, validate the manifest, hash assets, check whether resources exist, patch the EXE or update/replace DLLs. The OS may still fail to start if the executable or working directory is absent; the game may fail later if dependencies are missing. For known US/FR executable sizes, it passes the same GameFrame/DirectDraw runtime options without validating their hashes; unknown sizes do not receive edition-specific memory profiles. Use normal launch or `--verify` to investigate installation problems.

**Automatic graphics:** New installations create the game's original 32-byte `cyber.ini` with maximum 640×480 internal resolution and detail 3/3. Existing user settings are not overwritten just to force a preference. There is no separate graphics CLI switch. Older or unknown WinMM/CDDA DLLs are preserved in SHA-qualified backups before the normal installer's automatic replacement; existing backups are never silently overwritten.

## Configuration and integrity

The **CyberGladiatorsFix/CyberGladiatorsFix.ini** text file is distinct from the game's binary **cyber.ini**:

- **[Video] SkipIntro=0** — play the original introduction (default).

**CDDA/FLAC playback is required** and enabled automatically. **All retail CD resources, including `iq/`, are strictly SHA-256 checked during the initial staged installation.** The game may subsequently rewrite `iq/` files: on later launches and with `--verify` every manifest-listed IQ file must exist as a regular file, but its size and hash are no longer compared. Other original assets, the EXE and backup, FLAC and embedded DLLs remain strictly checked **during normal launches and `--verify`**. Original IQ hashes remain in the manifest and the launcher never overwrites these files; the explicit `--launch-no-verify` shortcut skips preflight verification for an existing game, without adding an INI option. Legacy `[Audio]`/`[Integrity]` sections are removed automatically on startup while preserving `[Video] SkipIntro` and unrelated user text.

Two logs next to **CyberGladiatorsFix.exe** separate the installer/launcher from the running game:

- **`CyberGladiatorsFix_installer.log`**: refreshed for each installer, verification or launcher invocation; records system/display configuration, retail edition checks, SHA-256 verification, setup, game launch and exit status.
- **`CyberGladiatorsFix.log`**: refreshed **only when the game launches**; records WinMM/CDDA/FLAC, GameFrame Glide, DirectDraw and video diagnostics. A standalone `--verify`, `--install-only`, `--help` or `--version` must not clear this log.

The two files contain the same **session ID** for a given game launch. Both follow the same naming and lifecycle under native Windows and Wine/Proton. GameFrame and WinMM share the game log; no third video-player log is created.

### Compatibility and safety

The **US and French CYBERG retail** editions are identified by distinct exact SHA-256 profiles. All four US/FR × Windows/Wine combinations select the same owned-COM video implementation by default and have been tested on real Windows 11 / Wine-staging 11.17 hosts. Other systems require their own field verification. The US WinMM import-name migration is checked against previously pinned executable hashes; the FR implementation has its own checked preimages, import slot and known full-binary hashes. Unknown executable revisions, directory entries masquerading as `glide.dll`, unexpected manifest entries and incomplete game installations fail closed. Any different Glide file is archived with a unique `.bak` name before installing the verified bundled GameFrame; an unreadable old file can also be preserved without inspecting its contents. CUE/BIN input is read-only, and the game installation uses a staging folder. A per-installation Windows mutex prevents concurrent launch or install.

Native Windows **FR and US** GameFrame rendering and video handoff have now been field-tested on PR #6 builds; this does not establish compatibility with every GPU or Windows configuration. The sole distribution workflow runs Rust tests (including DirectDraw mode-selection and backup-integrity checks), verifies the x86 DLLs and Glide exports, checks the CLI, and smoke-tests the native Windows loader, but it does not play the game. Historical experimental branches and diagnostics remain available for regression research, not as supported user entrypoints. Read the [build-85 technical audit](docs/BUILD85_AUDIT.md) and the [pre-release checklist](docs/RELEASE_READINESS.md) for outstanding risks.

## French retail edition

The original `CYBERG.BIN` data track and `CYBER.EXE` are authenticated byte-for-byte, independently of the US media. The 15-track `CYBERG.CUE` has one MODE1/2352 data track and 14 CDDA tracks. Its game patches, verified original EXE backup and game HWND/WndProc addresses are distinct from the US ones.

On **native Windows**, the shared owned-COM mode was field-validated with full Cinepak playback, Alt+Tab, GameFrame handoff, FLAC, unchanged physical display mode and clean exit. The same mode also passed **FR/Wine-staging 11.17** on a clean `CYBERG.cue` installation, second launch, ~39-second intro, skip and Alt+Tab. The latest FR/Windows tests on 2026-11-09 confirmed the upper-right signature, new safe Glide archive collision behavior, and exact same-build `glide.dll` reuse. For historical diagnostic experiments see the [FR test plan](docs/FR_TEST_PLAN.md).

## Licenses

CyberGladiatorsFix code is **GPL-3.0-or-later** (LICENSE). The pinned [GameFrame](https://github.com/tanepiper/game-frame) component is **Apache-2.0**, and [isomage 2.1.0](https://github.com/JackDanger/isomage) is **MIT** (upstream copyright text shipped as `LICENSE-MIT`). The download includes GPL, Apache and MIT license notices. Neither the game nor its copyright-protected disc tracks are bundled. For prior experiments see [development history](docs/DEVELOPMENT_HISTORY.md) and the merged [GameFrame PR #2](https://github.com/legluondunet/CyberGladiatorsFix/pull/2). The test-artifact instructions are in [docs/DISTRIBUTION.md](docs/DISTRIBUTION.md).

*legluondunet assisted by ChatGPT (OpenAI)*
