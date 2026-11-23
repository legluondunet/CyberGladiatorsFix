# CyberGladiatorsFix

**One-click installer and modern compatibility fix for CyberGladiators (1996).**

[**Download CyberGladiatorsFix v2.0.0**](https://github.com/legluondunet/CyberGladiatorsFix/releases/tag/v2.0.0)

CyberGladiatorsFix installs and launches the original **US and French PC editions** on modern Windows, and on Linux through Wine/Proton. It is open source and **does not include the original game or CD audio**.

## Features

- **Automatic installation:** detects supported original CUE/BIN disc images, installs the game, verifies its files and launches it. The disc image is no longer needed after installation.
- **Modern graphics:** GameFrame Glide-to-OpenGL rendering, borderless fullscreen at desktop resolution, original **4:3 aspect ratio** and a 60 FPS limiter.
- **Original videos:** AVI playback with an option to skip the introduction.
- **CD music:** automatically extracts the original audio as **14 lossless FLAC tracks** for disc-free playback.
- **Windows and Wine/Proton:** compatibility fixes for video, focus and Alt+Tab.
- **Single executable:** the launcher includes the required compatibility components; no separate graphics wrapper installation is needed.

## Installation

1. Download and extract the [latest release](https://github.com/legluondunet/CyberGladiatorsFix/releases/tag/v2.0.0) into a **new, empty, writable folder**.
2. Put your original disc image beside `CyberGladiatorsFix.exe`:
   - **US:** `CYBER.cue` and `CYBER.bin`
   - **French:** `CYBERG.CUE` and `CYBERG.BIN`
3. Run `CyberGladiatorsFix.exe` on Windows, or use Wine on Linux:

   ```sh
   wine CyberGladiatorsFix.exe
   ```

The installer handles the rest. For future launches, simply run `CyberGladiatorsFix.exe` again.

> **Upgrading from v1.x:** v2.0.0 requires a fresh installation and does not migrate existing installations or saved games. Keep your previous folder and saves separately.
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

Set `SkipIntro=1` to skip the opening video or `Log=1` to enable diagnostic logs.

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
