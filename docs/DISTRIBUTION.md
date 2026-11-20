CyberGladiatorsFix v2.0.0 — README for the one-file Windows package
=====================================================================

Open-source one-click installer and compatibility launcher for the original
Cyber Gladiators (1996) PC game. Windows x64 and Wine/Proton. GPL-3.0-or-later.

THIS ZIP DOES NOT CONTAIN THE ORIGINAL GAME OR COPYRIGHTED CD TRACKS.

BREAKING CHANGE FROM v1.0.1
----------------------------
Start from a **new empty writable folder**. Place CyberGladiatorsFix.exe
beside one supported original USA CYBER.cue/CYBER.bin or French
CYBERG.CUE/CYBERG.BIN disc image, then execute it. Under Wine:
  wine CyberGladiatorsFix.exe

The launcher authenticates the source image, installs the game and its
compatibility modules, extracts 14 FLAC CD audio tracks and starts the game.
On subsequent runs the CUE/BIN is not required. Standalone ISO **cannot be
installed**, although --inspect supports disc-image inventory.

There is NO migration from old v1.0.1 game folders. Keep old installations
and saves separately. No proprietary game assets are bundled or overwritten.

INSTALLED TREE
--------------
CyberGladiatorsFix.exe
CyberGladiatorsFix.ini
CyberGladiatorsFix_installer.log
CyberGladiatorsFix.log
game/
  cyber.exe
  CYBER.EXE.bak      original verified retail EXE
  cyber.ini
  fix.dll            verified embedded x86 WinMM/MCI/FLAC bridge
  glide.dll          verified embedded x86 GameFrame renderer
  graphics/, iq/, ... original resources
music/
  Track02.flac ... Track15.flac
state/
  CyberGladiatorsFix.manifest
  CyberGladiatorsFix.sha256
  CyberGladiatorsFix.gameframe.sha256
  CyberGladiatorsFix.install-receipt.txt
  CyberGladiatorsFix.first-launch-complete.txt (after first successful play)
docs/
  README.md
  LICENSE-GPL-3.0
  LICENSE-APACHE
  LICENSE-MIT
  CyberGladiatorsFix.ini.example
  build-info.txt

There is no additional CyberGladiatorsFix/ game folder. The original
CYBER.EXE.bak remains stored and SHA-256 verified. Installer-owned
fix.dll/glide.dll are replaced without permanent backup files but only
when ownership is confirmed; foreign DLLs are not overwritten.
Installed writable IQ data is checked by presence after first-install
hash verification; all other original immutable resources are SHA-256
verified.

CONFIGURATION
-------------
CyberGladiatorsFix.ini defaults:
  [Video]
  SkipIntro=0
  [Debug]
  Log=0

First installation and first successful game launch create both logs
regardless of default Log=0. Later Log=0 keeps those two logs untouched;
Log=1 recreates both logs on every session, without accumulating history.
The two logs share a session ID. SkipIntro=1 skips the original AVI; FLAC
music stays enabled automatically.

COMMANDS
--------
CyberGladiatorsFix.exe                     auto-install/verify/launch
CyberGladiatorsFix.exe --install-only      install/prepare without play
CyberGladiatorsFix.exe --verify            verify existing installation
CyberGladiatorsFix.exe --launch-no-verify  launch without integrity preflight
CyberGladiatorsFix.exe --inspect image.iso
CyberGladiatorsFix.exe --inspect image.cue [track.bin|track.img]
CyberGladiatorsFix.exe --verbose           optional diagnostics modifier
CyberGladiatorsFix.exe --version
CyberGladiatorsFix.exe --help              also -h

FEATURES / TESTING LIMITATIONS
------------------------------
The bundled GameFrame Glide-to-OpenGL wrapper presents borderless desktop-
resolution 4:3 graphics at approximately 60 FPS. The verified WinMM bridge
handles native Cinepak AVI, CDDA/FLAC and DirectDraw/MCI compatibility.
Focus/Alt+Tab adaptations exist for Windows and Wine.

After merged PR #27, original USA and French CUE/BIN installs, AVI,
GameFrame and FLAC playback were successfully tested under Wine-staging
11.19. The version-bumped USA candidate 2.0.0-0f86b592 also passed a
fresh retail CUE/BIN install and 53.6-second session with video focus
recovery, FLAC/CDDA and normal exit. FR additionally validated
subsequent Log=0 and consecutive Log=1 sessions.
Windows CI succeeded, including native DLL loader checks. A native
Windows FR test on the merged source build 2.0.0-2ae6cadb also passed
fresh retail CYBERG.CUE installation, 4:3 video/GameFrame, FLAC/CDDA,
focus return and normal exit after 30.8s. Its physical 1920x1200 mode
remained unchanged; logical screen metrics were 1536x960. Windows USA,
repeat launch remain untested in this layout; the maintainer confirmed
the FR image, music and Alt+Tab return were correct. A final tagged v2.0.0 release binary is not
yet field-tested.

KNOWN WINDOWS FR v2.0.0 STARTUP REGRESSION
-----------------------------------------
A later native Windows FR clean-install attempt using the exact same
2.0.0-2ae6cadb binary crashed before AVI open with exit code
0xc0000005, following the virtual DirectDraw cooperative-level trace.
Both observed attempts were logged as new installations, not a proven
repeat launch. The precise faulting module and instruction are unknown.
The earlier successful image, music and Alt+Tab test remains valid for
that one session only. **Do not publish while this release blocker
remains unresolved.**
https://github.com/legluondunet/CyberGladiatorsFix/issues/30

SIX-FILE DISTRIBUTION AND LICENSES
----------------------------------
CyberGladiatorsFix.exe, README.txt, LICENSE-GPL-3.0, LICENSE-APACHE,
LICENSE-MIT and SHA256SUMS.txt. The checksum manifest covers the first
five files. This ZIP has no standalone DLLs, game EXE, CUE/BIN or FLAC.

CyberGladiatorsFix and fix.dll: GPL-3.0-or-later
GameFrame: Apache-2.0
isomage: MIT

Source and release details:
https://github.com/legluondunet/CyberGladiatorsFix
docs/RELEASE_NOTES_v2.0.0.md and docs/RELEASE_READINESS_v2.0.0.md

Published v1.0.0/v1.0.1 packages remain unchanged. Public v2.0.0
publication requires a separate explicit maintainer approval.
