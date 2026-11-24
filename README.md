# CyberGladiatorsFix

[![Windows x64 Build](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml/badge.svg?branch=main)](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/gameframe-glide.yml) [![Linux Wine / Proton compatible](https://img.shields.io/badge/Linux%20Wine%20%2F%20Proton-compatible-428b40?logo=linux&logoColor=white)](#installation)

**One-click installer and modern compatibility fix for CyberGladiators (1996).**

CyberGladiatorsFix installs and launches the original **US and French PC editions** on modern Windows, and on Linux through Wine/Proton. It is open source and **does not include the original game or CD audio**.

## Features

- **Automatic installation:** detects supported original CUE/BIN disc images, installs the game, verifies its files and launches it. The disc image is no longer needed after installation.
- **Modern graphics:** GameFrame Glide-to-OpenGL rendering, borderless fullscreen at desktop resolution, original **4:3 aspect ratio** and a 60 FPS limiter.
- **Original videos:** AVI playback with an option to skip the introduction.
- **CD music:** automatically extracts the original audio as **14 lossless FLAC tracks** for disc-free playback.
- **Windows and Wine/Proton:** compatibility fixes for video, focus and Alt+Tab. **From v2.0.1**, GE-Proton is detected automatically and uses a game-owned AVI window to avoid a black introduction; other Wine runners and native Windows keep their previous window policy.
- **Single executable:** the launcher includes the required compatibility components; no separate graphics wrapper installation is needed.

## Supported Game Editions

The original retail CD editions below are supported. Each SHA-256 identifies the **unmodified `CYBER/CYBER.EXE` file on the game CD**, not the CUE/BIN disc image or the executable patched by CyberGladiatorsFix.

| Edition | Original EXE size | Original `CYBER.EXE` SHA-256 |
| --- | ---: | --- |
| US retail (v1.0) | 351,776 bytes | `226a04a745addbfe77709aa7ec56bf5c17792c524b31882da7653e1973775624` |
| French retail | 352,288 bytes | `12db5e29120832ebc304695ecfa38f6c0d825afe2640433714d27f624da53cdb` |

The installer also verifies edition-specific files on the original CD; other executable or disc variants are not supported.

## Installation

1. Download and extract the [latest stable release](https://github.com/legluondunet/CyberGladiatorsFix/releases/latest) into a **new, empty, writable folder**.
2. Place `CyberGladiatorsFix.exe` and an image of your original game CD (**CUE/BIN**) in the same folder. For example:

   ```text
   CyberGladiatorsFix/
   ├── CyberGladiatorsFix.exe
   ├── CYBERG.CUE
   └── CYBERG.BIN
   ```

3. Run `CyberGladiatorsFix.exe` on Windows, or use Wine on Linux:

   ```sh
   wine CyberGladiatorsFix.exe
   ```

The installer handles the rest. For future launches, simply run `CyberGladiatorsFix.exe` again.

> **Upgrading:** v1.x requires a fresh installation; keep its old folder and saves separately. From v2.0.0 to v2.0.1, keep the existing `game/`, `music/` and `state/` directories and replace the launcher executable. The installer verifies the original game and safely updates only the runtime DLLs that it owns.
>
> **Disc formats:** original US/FR CUE/BIN images are supported for installation; a standalone ISO cannot be installed.

## Configuration

The optional `CyberGladiatorsFix.ini` file contains:

```ini
[Video]
SkipIntro=0

[Debug]
Log=0
```

Set `SkipIntro=1` to skip the opening video or `Log=1` to enable diagnostic logs. On GE-Proton v2.0.1+, no manual environment variable is needed; a diagnostic override `CYBER_AVI_GAME_OWNED_HOST=0` disables the automatic AVI-window policy, and `=1` forces it on the supported Wine video path.

Useful commands:

```text
CyberGladiatorsFix.exe                 Install, verify and launch
CyberGladiatorsFix.exe --install-only  Install without launching
CyberGladiatorsFix.exe --verify        Verify an existing installation
CyberGladiatorsFix.exe --help          Show all available options
```

## License

CyberGladiatorsFix and its compatibility bridge are licensed under **GPL-3.0-or-later**. The bundled [GameFrame](https://github.com/tanepiper/game-frame) renderer is **Apache-2.0**; `isomage` is **MIT**.

Original game files, textures, videos and CD audio are **not distributed**. You must supply your own supported original game media.

---

**Developed by [legluondunet](https://github.com/legluondunet), with assistance from ChatGPT (OpenAI).**
