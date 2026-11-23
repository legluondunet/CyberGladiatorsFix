CyberGladiatorsFix v2.0.0 — README for the one-file Windows package
=====================================================================

Open-source one-click installer and compatibility launcher for the original
CyberGladiators (1996) PC game. Windows x64 and Wine/Proton. GPL-3.0-or-later.

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
and saves separately. No proprietary game assets are bundled. Original source CUE/BIN media are
never modified; the installed CYBER.EXE is patched only after its verified
original is preserved as game/CYBER.EXE.bak.

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

The v2 layout, authenticated USA/FR media, AVI, GameFrame and FLAC were
tested under Wine-staging 11.19/XFCE. Earlier French Wine tests also
confirmed default Log=0 and noncumulative Log=1 session logging.

The same final engine 2.0.0-9605dff4 was exercised with both original
USA and French editions under native Windows on 22 November 2026.
USA: the live DirectDraw null-cleanup guard was invoked, original MCI AVI
played for about 28 seconds, GameFrame/OpenGL menu and FLAC playback
started, approximately 59 FPS stabilized, and the game exited with code 0.
FR: a fresh CYBERG.CUE/BIN installed 343 original game files and 14 FLAC
tracks; 356 resources passed integrity checks. The live FR DirectDraw
guard was invoked, MCI AVI played for about 36 seconds, the menu and
FLAC started, and the game exited with code 0. Both sessions logged video
focus loss and recovery; the FR physical desktop mode remained unchanged.

These final US/FR observations are log-confirmed. Both movie sessions
recorded a Space key press: full natural AVI completion is NOT proven.
Subjective video/audio quality and all current-build SkipIntro/repeat
permutations have not been separately confirmed. The exact tagged
release ZIP has not yet been run on a real host.

The tested 9605dff4 engine passed Windows CI #44845426448; the latest
documentation-only main commit 94370065 passed Windows CI #44989934680.
CI covers native build/tests, DLL checks and package validation, not
interactive retail gameplay. Check CI again on the final main SHA before
the separately approved manual publication.

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
