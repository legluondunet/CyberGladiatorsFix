# CyberGladiatorsFix

**v0.1.5 (game-writable IQ files).** Windows CUE/BIN installer, lossless FLAC extraction, timing + no-CD + startup-video fixes, safe graphics settings tool, verifier and game launcher for CyberGladiators (1996).

[![Windows x64](https://img.shields.io/badge/Windows-x64-blue)](https://github.com/legluondunet)
[![Linux Wine / Proton compatible](https://img.shields.io/badge/Linux-Wine%20%2F%20Proton-blue)](https://www.winehq.org/)

By **legluondunet assisted by ChatGPT (OpenAI)**. Licensed GPL-3.0-or-later.

## Status: first development snapshot; not a confirmed playable release

This implementation follows the Windows-console, fail-closed, image-extraction approach of the author's **ScorchedPlanetFix**. It was adapted for CyberGladiators rather than copying game-specific Scorched Planet offsets or files.

**Known retail CYBER.EXE** (the user-provided executable):

- 351,776 bytes, SHA-256 `226a04a745addbfe77709aa7ec56bf5c17792c524b31882da7653e1973775624`.
- Previous speed-only patched SHA-256 `520d516a572f3287458337e8cef1231b5645f59e3952b3f568a2aab8f71af832` (supported for upgrade).
- Previous speed + no-CD patched SHA-256 `d397053de4b1a8573bd851915e7aeb3773f3b7010a08b85162c28ea94bccdaf9` (supported for upgrade).
- Current speed + no-CD + intro patched SHA-256 `d8329312a8f63f5efea1720b422429f530799c6887f31c18ba71f970390900b6`.
- `CYBER.EXE.bak` contains the unchanged original.
- At file offset `0x3116`: relative call displacement `c4 ff ff ff` -> `0e 30 04 00`.
- At file offset `0x3125`: `b5 ff ff ff` -> `ff 2f 04 00`.
- Both calls now target the existing `GetTickCount` import trampoline at virtual address `0x00446B28` rather than the old `QueryPerformanceCounter`-based routine.
- **No-CD startup check:** at raw offset `0x4A16` (VA `0x00405416`), original `e8 4f ff ff ff` (CALL CD-volume check) becomes `b8 01 00 00 00` (`MOV EAX,1`). This skips the original first-CD-drive volume label test for `CYBER`; it does **not** provide CDDA emulation or replace other potentially CD-dependent code. Only the verified USA executable may be patched.
- **Startup movie:** `0x63FB: 74 2F -> 90 90` bypasses the missing `[Config] CDROM` entry in the old `sierra.ini` when the original installer has not run. `0x640F: 7C 0C -> 90 90` chooses the existing 640×480 `SPLASH4.AVI` instead of the low-resolution `SPLASH.AVI`. Both files use **Cinepak**; Wine/Windows must still be able to decode Cinepak through Video for Windows / MCI.
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

Without arguments, looks for exactly one `.cue` in the current working directory (an `.iso` alongside it is ignored). The CUE references one or more `.bin` files holding the data and optional audio tracks. Standalone `.iso` and `.bin` inputs are deliberately rejected. Installs the **CYBER/** tree from the CD image to **CyberGladiatorsFix/** next to the patcher and, when a CUE is used, extracts available CD AUDIO tracks as `Track02.flac` through `Track15.flac` beside `cyber.exe`. Excludes CD-root installers and obsolete 3dfx hardware drivers. Patches the game and saves its original binary. Does not overwrite an existing installation. When run again with an existing CyberGladiatorsFix directory, it upgrades known prior speed/no-CD patches (including v0.1.3) with the intro fix, provided `cyber.exe.bak` is intact, and attempts to install the user's nGlide DLL. No game resources or FLAC tracks are re-extracted.

`--verify` checks the supported patched executable and its unmodified original `.bak`, SHA-256 hashes of **static** extracted resources (including FLAC tracks), and a 32-bit `glide.dll`. The retail `IQ/iq00.dat`–`IQ/iq99.dat` files are game-writable: their original **3,696-byte size, presence and regular-file status** remain checked, but content hash changes are allowed. The original hashes stay in the manifest for reference.

**Existing installs:** first run `CyberGladiatorsFix.exe` **without arguments** once to upgrade the current executable (speed + no-CD) with the intro fix. Keep `cyber.exe.bak` untouched. To maximize the saved graphics settings, first **exit the game normally**, then run `CyberGladiatorsFix.exe --max-graphics`, followed by `--launch`. This command only edits the two validated fields in an existing 32-byte binary `cyber.ini` and preserves `cyber.ini.bak`. It refuses missing/unknown formats rather than inventing settings for an unplayed game.

`--launch` verifies first, starts CYBER.EXE in the game directory and waits for exit. Configures nGlide locally for the game, without editing global registry. Optional `--3dfx-logo` requires a separately supplied `3DfxSpl.dll`.

## Intro video and graphics

The USA CD ships with both `CYBER/SPLASH.AVI` (320×240, Cinepak video, 22.05-kHz PCM audio) and `CYBER/SPLASH4.AVI` (640×480, same codecs). Both were inspected and decoded from the user-provided original data track. The original game normally skips the video if `[Config] CDROM` is absent from Sierra's Windows-directory `sierra.ini`. The patch bypasses only this legacy condition and selects `SPLASH4.AVI`. It does not install a Cinepak codec, modify the Windows directory or force a separate external video player. **Whether MCI/Cinepak playback works under Wine is still to be tested.**

CyberGladiators stores graphics preferences in a 32-byte binary `cyber.ini` in the game directory. Reverse engineering confirmed two four-byte little-endian fields at offsets `0x10` (internal screen-size index: `0` = 640×480 maximum) and `0x14` (detail index: `3` = maximum of 0–3). `--max-graphics` updates only those two fields, preserves the other 24 bytes and backs up the original once. The internal 640×480 image can be upscaled by nGlide; the option does not add native widescreen rendering.

If `cyber.ini` does not yet exist, launch and exit the game normally so it can write its preferences, then run `--max-graphics`. Alternatively, in the game the original manual specifies `+` / `-` for image size and `]` / `[` for detail.

## nGlide

Install the **official nGlide** from [Zeus Software](https://www.zeus-software.com/downloads/nglide). The patcher can copy an already-installed 32-bit `glide.dll` from next to the patcher or from Windows `SysWOW64` into **CyberGladiatorsFix/**. It does not bundle, download or install proprietary DLLs. A historical CD `GLIDE.DLL` is explicitly excluded, so that it cannot replace the modern wrapper.

## Disc support

- Reads the ISO9660/Joliet/UDF filesystem inside the CUE/BIN **data track** through the same `isomage` parser used by ScorchedPlanetFix. This is internal conversion only, **not** support for ISO files as input.
- Accepts **CUE + one or multiple referenced BIN files**, or a **single shared BIN with multiple tracks**, with data track MODE1/2048, MODE1/2352 or MODE2/2352 (Form 1) and strict bounds checks. The first CUE track must contain data. CDDA tracks use 2352-byte sectors; if their BIN files exist, they are converted losslessly to FLAC.
- Rejects ambiguous input, unsafe paths, unexpected executable hashes, and preexisting output.
- The **complete CYBER directory** is kept conservatively until real disc inventories establish which individual resources are unnecessary. Installers outside CYBER are discarded.
- **USA edition identified using the provided CUE/BIN Track 01:** `CyberGladiators (USA).cue` + 133,217,280-byte `Track 01.bin`, MODE1/2352, volume `CYBER`, 56,640 sectors.
- The CD contains 760 files; its **CYBER/** tree contains **342 game files totaling 47,954,511 bytes (45.73 MiB)**. Install excludes 418 other CD files (setup/DirectX/demos and non-game content).
- Known USA profile requires the exact retail `CYBER.EXE`, `GRAPHICS/SHELL.FNT` and `SPLASH.AVI` hashes from this disc. The original binary SHA-256 matches the previously supplied `CYBER.EXE`.
- The supplied CUE declares 15 tracks: **Track 01 data** and **Tracks 02–15 CD audio**. The 14 audio BIN files were deliberately omitted in the supplied test fixture. When present, the installer extracts them losslessly as FLAC using a pure Rust encoder (no ffmpeg/flac executable required). Missing audio BIN files are reported and skipped without preventing extraction of the game data. No proprietary game audio is bundled. **The game does not automatically play FLAC instead of CDDA; integrating playback is a separate task.**
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

**Important:** Extracted FLACs are an archive of the music, not yet a CDDA
emulation layer. `CYBER.EXE` may continue to request physical CD audio; nGlide
is a graphics wrapper and does not automatically supply FLAC playback.

## Verifier behavior after the first game run

The original disc contains exactly 100 `IQ/iq00.dat` through `IQ/iq99.dat` files, each **3,696 bytes**. The game references these files by name and can write them during play. The initial verifier incorrectly required their original CD SHA-256 values forever, which blocked subsequent launches after `iq/iq41.dat` changed. The new verifier still requires the files to exist with their original fixed sizes, rejects symlinks, and reports their number, but permits changes in their content. All other game resources (including the 14 generated FLACs), the patched `CYBER.EXE` and untouched `CYBER.EXE.bak` retain strict SHA-256 validation. Existing installations do not need reinstalling or manifest rewrites.

## Known limitations / investigation

1. A Wine report confirmed the game starts without asking for the CD and gameplay speed appears normal after v0.1.3. The no-CD behavior is promising but other CD-dependent features may remain, especially CDDA music.
2. The video/graphics changes introduced in v0.1.4 remain **unconfirmed under Wine**: the user's next attempt was stopped by a false-positive integrity alert for `iq/iq41.dat`. v0.1.5 adjusts the verifier to account for game-writable IQ files; Cinepak MCI/VfW codec support remains a possible dependency.
3. `--verify` checks static extracted files against the local manifest and keeps executable identity checks strict. `IQ/iqNN.dat` changes are allowed only for exactly 100 known names and original sizes, because the game modifies this directory during gameplay. The manifest is local, not an independently authenticated retail database. Unsupported executable editions are refused.
4. The latest Wine install log reports conversion of 14 audio tracks to FLAC. Their playback in the game is **not** integrated: 44.1 kHz 16-bit stereo PCM is converted losslessly, pregaps before INDEX 01 are excluded, and native CDDA requests may still need handling. This is not a complete bit-perfect CD image/archive.
5. No executable, game assets, disc images or proprietary nGlide binaries are shipped in this source project.

## Build

Uses Rust `cargo`, Windows x64 target, Rust 2021, `isomage = 2.1.0`, `sha2 = 0.10`, `flacenc = 0.5.1` (pure Rust, Apache-2.0).

```powershell
cargo test --target x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
```

The source project includes Windows GitHub Actions compilation, unit tests and CLI smoke tests. The user confirmed game launch without a CD, normal apparent speed, extraction of 342 resources, and FLAC conversion of all 14 audio tracks. The new intro and graphics settings adjustments are not yet verified in the game.

## Credits and reference

- Project architecture based on [ScorchedPlanetFix](https://github.com/legluondunet/ScorchedPlanetFix) by the same author.
- **Original speed fix credit:** Aqrit ([Bitpatch.com — CyberGladiators Speed Fix](https://bitpatch.com/cybergladiators.html)). The original analysis and the two `CYBER.EXE` hexadecimal substitutions are by Aqrit; CyberGladiatorsFix incorporates them with SHA-256 executable verification, an automatic backup and a separate no-CD startup-check patch. The patch is also documented by [SierraVault](https://mirror.sierravault.net/1996-cybergladiators).
- nGlide: © Zeus Software; download separately at official site.