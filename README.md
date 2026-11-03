# CyberGladiatorsFix

**v0.1.2 (lossless CD audio extraction).** Windows console CUE/BIN installer, FLAC audio extraction, speed fix, verifier and game launcher for CyberGladiators (1996).

[![Windows x64](https://img.shields.io/badge/Windows-x64-blue)](https://github.com/legluondunet)
[![Linux Wine / Proton compatible](https://img.shields.io/badge/Linux-Wine%20%2F%20Proton-blue)](https://www.winehq.org/)

By **legluondunet assisted by ChatGPT (OpenAI)**. Licensed GPL-3.0-or-later.

## Status: first development snapshot; not a confirmed playable release

This implementation follows the Windows-console, fail-closed, image-extraction approach of the author's **ScorchedPlanetFix**. It was adapted for CyberGladiators rather than copying game-specific Scorched Planet offsets or files.

**Known retail CYBER.EXE** (the user-provided executable):

- 351,776 bytes, SHA-256 `226a04a745addbfe77709aa7ec56bf5c17792c524b31882da7653e1973775624`.
- Patched SHA-256 `520d516a572f3287458337e8cef1231b5645f59e3952b3f568a2aab8f71af832`.
- `CYBER.EXE.bak` contains the unchanged original.
- At file offset `0x3116`: relative call displacement `c4 ff ff ff` -> `0e 30 04 00`.
- At file offset `0x3125`: `b5 ff ff ff` -> `ff 2f 04 00`.
- Both calls now target the existing `GetTickCount` import trampoline at virtual address `0x00446B28` rather than the old `QueryPerformanceCounter`-based routine.
- These offsets **must not** be used with unrecognized releases. Unknown hashes are refused and logged.

## CLI

```text
CyberGladiatorsFix.exe
CyberGladiatorsFix.exe <disc.cue>
CyberGladiatorsFix.exe --verify
CyberGladiatorsFix.exe --launch [--3dfx-logo]
CyberGladiatorsFix.exe --help
CyberGladiatorsFix.exe --version
```

Without arguments, looks for exactly one `.cue` in the current working directory (an `.iso` alongside it is ignored). The CUE references one or more `.bin` files holding the data and optional audio tracks. Standalone `.iso` and `.bin` inputs are deliberately rejected. Installs the **CYBER/** tree from the CD image to **CyberGladiatorsFix/** next to the patcher and, when a CUE is used, extracts available CD AUDIO tracks as `Track02.flac` through `Track15.flac` beside `cyber.exe`. Excludes CD-root installers and obsolete 3dfx hardware drivers. Patches the game and saves its original binary. Does not overwrite an existing installation. When run again with an existing CyberGladiatorsFix directory, it verifies or applies the exact speed patch and attempts to install the user's nGlide DLL.

`--verify` checks known patched executable identity, original `.bak` identity, installer-recorded SHA-256 values for extracted resources, and presence of a 32-bit `glide.dll`.

`--launch` verifies first, starts CYBER.EXE in the game directory and waits for exit. Configures nGlide locally for the game, without editing global registry. Optional `--3dfx-logo` requires a separately supplied `3DfxSpl.dll`.

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

## Known limitations / investigation

1. **CD check is not yet bypassed.** CyberGladiators may require the original CD mounted for game data; a previously documented game behavior looks at the first optical drive. Portable launch without a CD is NOT validated and should not be promised.
2. Game startup, nGlide behavior and animation speed have not been tested yet on Windows or Wine.
3. `--verify` audits extracted files against the **local installation manifest** created from the user's disc; this is an integrity check, not an independently authenticated retail-release database. Different executable editions are refused.
4. CD audio/multitrack music is absent from the supplied files, so real retail audio extraction and CDDA playback are not yet validated. Available audio tracks are converted to bit-exact 44.1 kHz 16-bit stereo FLAC (no resampling); pregaps before INDEX 01 are excluded. This is not a complete bit-perfect CD image/archive.
5. No executable, game assets, disc images or proprietary nGlide binaries are shipped in this source project.

## Build

Uses Rust `cargo`, Windows x64 target, Rust 2021, `isomage = 2.1.0`, `sha2 = 0.10`, `flacenc = 0.5.1` (pure Rust, Apache-2.0).

```powershell
cargo test --target x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
```

The source project includes a Windows GitHub Actions workflow. CI compilation, unit tests and CLI smoke tests have passed; real Windows/Wine gameplay and full retail audio extraction remain unverified.

## Credits and reference

- Project architecture based on [ScorchedPlanetFix](https://github.com/legluondunet/ScorchedPlanetFix) by the same author.
- **Original speed fix credit:** Aqrit ([Bitpatch.com — CyberGladiators Speed Fix](https://bitpatch.com/cybergladiators.html)). The original analysis and the two `CYBER.EXE` hexadecimal substitutions are by Aqrit; CyberGladiatorsFix incorporates them with SHA-256 executable verification and an automatic backup. The patch is also documented by [SierraVault](https://mirror.sierravault.net/1996-cybergladiators).
- nGlide: © Zeus Software; download separately at official site.