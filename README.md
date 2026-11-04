# CyberGladiatorsFix

**v0.2.1 (optional startup video skip + experimental CDDA bridge).** Windows CUE/BIN installer with lossless FLAC extraction, a 32-bit CD-audio MCI bridge, timing/no-CD/startup-video fixes, graphics preferences, verifier and game launcher for CyberGladiators (1996).

[![Windows x64](https://img.shields.io/badge/Windows-x64-blue)](https://github.com/legluondunet)
[![Linux Wine / Proton compatible](https://img.shields.io/badge/Linux-Wine%20%2F%20Proton-blue)](https://www.winehq.org/)

By **legluondunet assisted by ChatGPT (OpenAI)**. Licensed GPL-3.0-or-later.

## Status: first development snapshot; not a confirmed playable release

This implementation follows the Windows-console, fail-closed, image-extraction approach of the author's **ScorchedPlanetFix**. It was adapted for CyberGladiators rather than copying game-specific Scorched Planet offsets or files.

**Known retail CYBER.EXE** (the user-provided executable):

- 351,776 bytes, SHA-256 `226a04a745addbfe77709aa7ec56bf5c17792c524b31882da7653e1973775624`.
- Previous speed-only patched SHA-256 `520d516a572f3287458337e8cef1231b5645f59e3952b3f568a2aab8f71af832` (supported for upgrade).
- Previous speed + no-CD patched SHA-256 `d397053de4b1a8573bd851915e7aeb3773f3b7010a08b85162c28ea94bccdaf9` (supported for upgrade).
- Previous speed + no-CD + intro patched SHA-256 `d8329312a8f63f5efea1720b422429f530799c6887f31c18ba71f970390900b6` (supported for upgrade).
- Current speed + no-CD + intro + CDDA patched SHA-256 `71c8cb080a8d11001a91c370d2248dd9819cfd00b4b919cd7bbcb7699a96eedb` (intro enabled).
- Current skip-intro + CDDA SHA-256 `05215ec70462c0e34de7a5d3f9bf32adebb0a6a17b2946bcb768efb1db1b328f` (intro disabled). Both variants are verified.
- `CYBER.EXE.bak` contains the unchanged original.
- At file offset `0x3116`: relative call displacement `c4 ff ff ff` -> `0e 30 04 00`.
- At file offset `0x3125`: `b5 ff ff ff` -> `ff 2f 04 00`.
- Both calls now target the existing `GetTickCount` import trampoline at virtual address `0x00446B28` rather than the old `QueryPerformanceCounter`-based routine.
- **No-CD startup check:** at raw offset `0x4A16` (VA `0x00405416`), original `e8 4f ff ff ff` (CALL CD-volume check) becomes `b8 01 00 00 00` (`MOV EAX,1`). This skips the original first-CD-drive volume label test for `CYBER`. The separate CDDA bridge now emulates audio commands, not the physical CD filesystem. Only the verified USA executable may be patched.
- **Startup movie:** `0x63FB: 74 2F -> 90 90` bypasses the missing `[Config] CDROM` entry in the old `sierra.ini` when the original installer has not run. `0x640F: 7C 0C -> 90 90` chooses the existing 640×480 `SPLASH4.AVI` instead of the low-resolution `SPLASH.AVI`. Both files use **Cinepak**; Wine/Windows must still be able to decode Cinepak through Video for Windows / MCI.
- **CDDA import redirect:** original WINMM.dll import descriptor name field at file offset `0x4E434` changes from RVA `0x3ED40C` to `0x3EDB00`; the initially empty `.idata` padding at `0x4EF00` receives the name `CyberGladiatorsCDDA.dll`. All 15 imports remain satisfied. The bridge forwards 14 original WinMM functions unchanged and intercepts only `mciSendCommandA` for CD-audio devices; `mciSendStringA` continues to the true WinMM for AVI. The exact final patched executable SHA-256 is `71c8cb080a8d11001a91c370d2248dd9819cfd00b4b919cd7bbcb7699a96eedb`.
- **Optional intro skip:** `CyberGladiatorsFix.ini` (a new plain-text file, **not** the original 32-byte binary `cyber.ini`) contains `[Video] SkipIntro=0` by default. With `SkipIntro=1`, the patched `90 90` at file offset `0x63FB` becomes `EB 2F`, jumping to the introduction function's existing epilogue. All other videos and WinMM/CDDA calls remain unchanged. `--launch` reapplies the selected exact-hash patch before each game start.
- These offsets **must not** be used with unrecognized releases. Unknown hashes are refused and logged.

## CLI

```text
CyberGladiatorsFix.exe
CyberGladiatorsFix.exe <disc.cue>
CyberGladiatorsFix.exe --verify
CyberGladiatorsFix.exe --max-graphics
CyberGladiatorsFix.exe --launch [--3dfx-logo]
CyberGladiatorsFix.exe --help
CyberGladiatorsFix.exe --version
```

Without arguments, looks for exactly one `.cue` in the current working directory (an `.iso` alongside it is ignored). The CUE references one or more `.bin` files holding the data and optional audio tracks. Standalone `.iso` and `.bin` inputs are deliberately rejected. Installs the **CYBER/** tree from the CD image to **CyberGladiatorsFix/** next to the patcher, creates a 32-byte `cyber.ini` with the maximum 640×480 internal image size and 3/3 details, and extracts available CD AUDIO tracks as `Track02.flac` through `Track15.flac` beside `cyber.exe`. It also creates the separate editable `CyberGladiatorsFix.ini` containing the `SkipIntro` switch. Excludes CD-root installers and obsolete 3dfx hardware drivers. Patches the game and saves its original binary. Does not overwrite an existing installation. When run again with an existing CyberGladiatorsFix directory, it installs the 32-bit `CyberGladiatorsCDDA.dll` supplied beside the patcher and upgrades known prior speed/no-CD/intro patches if `cyber.exe.bak` remains intact. It also attempts to install the user's nGlide DLL. No game resources or FLAC tracks are re-extracted.

`--verify` checks known patched executable identity, original `.bak` identity, installer-recorded SHA-256 values for extracted resources, and presence of a 32-bit `glide.dll`.

**Fresh installs:** `cyber.ini` is created during extraction with maximum internal graphics settings. Both `CyberGladiatorsFix.exe` (Windows x64) and `CyberGladiatorsCDDA.dll` (Windows x86) must be extracted from the same GitHub Actions ZIP beside each other. The patcher copies the DLL into the game directory. **Existing installs:** put the two build files beside the existing patcher and rerun `CyberGladiatorsFix.exe` without arguments once to install the DLL and upgrade the original retail EXE; no CUE/BIN re-extraction is required. Use `--max-graphics` for older installations if desired; this backs up `cyber.ini` as `cyber.ini.bak`. The original `cyber.exe.bak` and all game `IQ/` data are preserved.

`--launch` verifies first, starts CYBER.EXE in the game directory and waits for exit. Configures nGlide locally for the game, without editing global registry. Optional `--3dfx-logo` requires a separately supplied `3DfxSpl.dll`.

## Skipping only the opening video

On a new installation the patcher creates two independent INI files beside `cyber.exe`:

- `cyber.ini` is the **binary 32-byte original game settings** (screen size and details); never add text keys to it.
- `CyberGladiatorsFix.ini` is **plain UTF-8 text** containing the patcher's options:

```ini
[Video]
SkipIntro=0
```

Change `SkipIntro=1` to skip the opening movie and go directly to the next original game step, or use `SkipIntro=0` to enable the opening movie again. The first time you use the updated patcher on an old installation, it creates the text INI without overwriting existing user files. After changing this setting, always start the game with `CyberGladiatorsFix.exe --launch` (or run the patcher with no arguments once before launching `cyber.exe` directly). The patcher switches between **two precisely SHA-256-verified EXE variants** before launching; this is not a global MCI-video-blocking hack. It leaves `cyber.ini`, FLACs and every `IQ/` file untouched.

## Intro video and graphics

The USA CD ships with both `CYBER/SPLASH.AVI` (320×240, Cinepak video, 22.05-kHz PCM audio) and `CYBER/SPLASH4.AVI` (640×480, same codecs). Both were inspected and decoded from the user-provided original data track. The original game normally skips the video if `[Config] CDROM` is absent from Sierra's Windows-directory `sierra.ini`. The patch bypasses only this legacy condition and selects `SPLASH4.AVI`. It does not install a Cinepak codec, modify the Windows directory or force a separate external video player. **Whether MCI/Cinepak playback works under Wine is still to be tested.**

CyberGladiators stores graphics preferences in a 32-byte binary `cyber.ini` in the game directory. Reverse engineering confirmed two four-byte little-endian fields at offsets `0x10` (internal screen-size index: `0` = 640×480 maximum) and `0x14` (detail index: `3` = maximum of 0–3). `--max-graphics` updates only those two fields, preserves the other 24 bytes and backs up the original once. The internal 640×480 image can be upscaled by nGlide; the option does not add native widescreen rendering.

On fresh installations, the 32-byte `cyber.ini` is created using the original USA game-generated file provided for analysis as the baseline; the six non-graphics DWORDs are preserved and are not all independently characterized. The graphics fields are set to `0` (640×480 maximum internal size) and `3` (detail 3/3). This generated file is a writable local game preference, excluded from the immutable CD-file manifest. The game may overwrite these settings when saving preferences. If you prefer adjusting them manually, the original manual specifies `+` / `-` for image size and `]` / `[` for detail.

## nGlide

The launcher uses the same **process-local** nGlide settings as ScorchedPlanetFix, without editing the Windows/Wine registry: `NGLIDE_BACKEND=2` (Vulkan) if `vulkan-1.dll` can be loaded, otherwise `0` (automatic); `NGLIDE_RESOLUTION=1` (desktop), `NGLIDE_ASPECT=1` (correction), `NGLIDE_REFRESH=2` (60 Hz), `NGLIDE_VSYNC=1` (enabled), `NGLIDE_GAMMA=5` (gamma 1.0), and `NGLIDE_SPLASH=0` unless `--3dfx-logo` is explicitly requested. A loadable Vulkan DLL does not guarantee a working GPU driver. These parameters affect Glide rendering only; the game's AVI playback uses a separate MCI/Video-for-Windows path.

Install the **official nGlide** from [Zeus Software](https://www.zeus-software.com/downloads/nglide). The patcher can copy an already-installed 32-bit `glide.dll` from next to the patcher or from Windows `SysWOW64` into **CyberGladiatorsFix/**. It does not bundle, download or install proprietary DLLs. A historical CD `GLIDE.DLL` is explicitly excluded, so that it cannot replace the modern wrapper.

## Disc support

- Reads the ISO9660/Joliet/UDF filesystem inside the CUE/BIN **data track** through the same `isomage` parser used by ScorchedPlanetFix. This is internal conversion only, **not** support for ISO files as input.
- Accepts **CUE + one or multiple referenced BIN files**, or a **single shared BIN with multiple tracks**, with data track MODE1/2048, MODE1/2352 or MODE2/2352 (Form 1) and strict bounds checks. The first CUE track must contain data. CDDA tracks use 2352-byte sectors; if their BIN files exist, they are converted losslessly to FLAC.
- Rejects ambiguous input, unsafe paths, unexpected executable hashes, and preexisting output.
- The **complete CYBER directory** is kept conservatively until real disc inventories establish which individual resources are unnecessary. Installers outside CYBER are discarded.
- **USA edition identified using the provided CUE/BIN Track 01:** `CyberGladiators (USA).cue` + 133,217,280-byte `Track 01.bin`, MODE1/2352, volume `CYBER`, 56,640 sectors.
- The CD contains 760 files; its **CYBER/** tree contains **342 game files totaling 47,954,511 bytes (45.73 MiB)**. Install excludes 418 other CD files (setup/DirectX/demos and non-game content).
- Known USA profile requires the exact retail `CYBER.EXE`, `GRAPHICS/SHELL.FNT` and `SPLASH.AVI` hashes from this disc. The original binary SHA-256 matches the previously supplied `CYBER.EXE`.
- The supplied CUE declares 15 tracks: **Track 01 data** and **Tracks 02–15 CD audio**. The 14 audio BIN files were deliberately omitted in the supplied test fixture. When present, the installer extracts them losslessly as FLAC using a pure Rust encoder (no ffmpeg/flac executable required). Missing audio BIN files are reported and skipped without preventing extraction of the game data. No proprietary game audio is bundled. **The experimental x86 CDDA bridge now connects these FLACs to the game's MCI commands; gameplay listening tests are still required.**
- All 56,640 raw MODE1 sectors were checked for sync and mode; all 342 game files were read and hashed from the data track; a local extraction and 8-byte binary patch simulation succeeded. **Rust compilation and Windows/Wine gameplay are not yet validated.**

## CUE/BIN input layout

Both formats below are supported. The `.cue` sheet is always the only argument:

```text
disc.cue
Track 01.bin       # game data
Track 02.bin       # optional CD audio
Track 03.bin       # optional CD audio
...                 # further tracks named in CUE
```

Alternatively, a CUE may describe multiple sequential tracks within a **single shared BIN**.
The parser respects `INDEX 01` and boundaries at the next `INDEX 00`, when present.
No `track.bin` override CLI argument is needed or accepted. BIN files are resolved from the CUE; do not rename them without updating the CUE sheet.

## CD AUDIO (new in 0.1.2)

The CUE parser recognizes `TRACK NN AUDIO` and reads 2352-byte CDDA sectors as
44,100 Hz, signed 16-bit, interleaved stereo. `INDEX 01` marks the start of
music; the following track's `INDEX 00`, if present in a shared BIN, marks
its end. Audio before `INDEX 01` (stored pregap) is intentionally excluded.
The output is compressed losslessly into **TrackNN.flac** in the game directory,
without sample-rate conversion or volume processing. The local manifest records
SHA-256 and size for each generated FLAC; `--verify` checks those files as well.

On the provided US CUE, filenames will be `Track02.flac` .. `Track15.flac`.
Since the source audio BINs have not been provided, that conversion cannot
be tested against the retail audio yet. An absent audio BIN is clearly reported and
skipped; a present, damaged BIN is rejected. An ISO without a CUE sheet is not
accepted. A data BIN is always required.

**Experimental audio playback (v0.2.0):** the project builds `CyberGladiatorsCDDA.dll` as a separate Windows **x86** Rust DLL (while `CyberGladiatorsFix.exe` remains **x64**). The bridge intercepts only CD-audio `mciSendCommandA` messages: `MCI_OPEN` for type `cdaudio`, `MCI_SET` TMSF, `MCI_PLAY` (track numbers 02–15), `MCI_STOP`, `MCI_PAUSE`, `MCI_RESUME`, `MCI_CLOSE`, and basic status/seek. It decodes only locally extracted lossless FLACs into 44.1 kHz/16-bit/stereo PCM and plays through Windows `waveOut` from a worker thread. The other 14 WinMM imports, including `mciSendStringA` for AVI, are delegated to the original system WinMM module. The bridge writes `CyberGladiatorsCDDA.log` beside the game with track/decoder/device errors. **No actual gameplay playback has yet been validated on Wine or Windows.**

The project-owned DLL is **not** a global `winmm.dll` replacement and must not be installed into the Windows/Wine system directories. It is supplied with the patcher by the Windows CI artifact and installed only beside the game. No original music files are bundled; only extracted tracks can play. The CDDA implementation remains experimental pending playback, pause/resume and track-transition tests.

## Known limitations / investigation

1. A Wine report confirmed the game starts without asking for the CD and gameplay speed appears normal after v0.1.3. The CDDA bridge has not been tested in the running game yet; track playback, pause/resume and transitions still need validation.
2. The original startup AVI was absent and graphics settings were below maximum. The v0.1.4 video/graphics changes target both issues and **are awaiting gameplay confirmation under Wine**; Cinepak MCI/VfW codec support remains a separate dependency.
3. `--verify` audits extracted files against the **local installation manifest** created from the user's disc; this is an integrity check, not an independently authenticated retail-release database. Different executable editions are refused.
4. The latest Wine install log reports conversion of all 14 tracks. The experimental CDDA player currently decodes whole individual tracks in a background thread (with a 12-minute bound) and streams PCM with a four-buffer waveOut queue. It does not yet implement physical CDDA emulation beyond the game's observed MCI call pattern or advanced notify behavior. Pregaps before INDEX 01 are excluded. This is not a complete bit-perfect CD image/archive.
5. No executable, game assets, disc images or proprietary nGlide binaries are shipped in this source project.

## Build

Uses Rust `cargo`: Windows x64 target for the installer and **Windows x86 target for the CDDA DLL**. Dependencies include `isomage = 2.1.0`, `sha2 = 0.10`, `flacenc = 0.5.1` and 32-bit `claxon = 0.4.3` (FLAC decoding in Rust).

```powershell
cargo test --target i686-pc-windows-msvc --manifest-path cdda/Cargo.toml
cargo build --release --target i686-pc-windows-msvc --manifest-path cdda/Cargo.toml
cargo test --target x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
```

The source project includes Windows GitHub Actions compilation and unit tests for **both** architectures, plus installer CLI smoke tests. The user confirmed the game launched without a CD at apparently normal speed, and all 14 FLAC tracks were extracted. The new CDDA DLL, video transition and first-launch graphics settings still require end-to-end Wine/Windows validation.

## Credits and reference

- Project architecture based on [ScorchedPlanetFix](https://github.com/legluondunet/ScorchedPlanetFix) by the same author.
- **Original speed fix credit:** Aqrit ([Bitpatch.com — CyberGladiators Speed Fix](https://bitpatch.com/cybergladiators.html)). The original analysis and the two `CYBER.EXE` hexadecimal substitutions are by Aqrit; CyberGladiatorsFix incorporates them with SHA-256 executable verification, an automatic backup and a separate no-CD startup-check patch. The patch is also documented by [SierraVault](https://mirror.sierravault.net/1996-cybergladiators).
- nGlide: © Zeus Software; download separately at official site.