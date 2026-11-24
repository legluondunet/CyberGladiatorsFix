CyberGladiatorsFix v2.0.1 — Windows x64 one-file package
====================================================

Compatibility installer and launcher for the original CyberGladiators (1996)
PC game. Runs on native Windows and through Wine/Proton on Linux.
Project and fix.dll license: GPL-3.0-or-later.

NO ORIGINAL GAME FILES, GAME DISC IMAGES OR COPYRIGHTED CD MUSIC ARE INCLUDED.

INSTALLATION (USA AND FRENCH ORIGINAL RETAIL DISCS ONLY)
---------------------------------------------------------
1. Put CyberGladiatorsFix.exe in a new, empty, writable directory.
2. Place ONE original CUE/BIN disc beside it:
   USA: CYBER.cue + CYBER.bin
   French: CYBERG.CUE + CYBERG.BIN
3. Launch CyberGladiatorsFix.exe. With Wine:
   wine CyberGladiatorsFix.exe

The launcher automatically recognizes, authenticates, installs, verifies
and starts the game; it extracts 14 lossless FLAC tracks from your own CD.
On later launches the CUE/BIN is not required. Standalone ISO is NOT a
supported installation format; --inspect can generate an ISO/CUE inventory
report without installing the game.

UPGRADING
---------
v2.0.0 -> v2.0.1: keep the existing v2 layout and replace only the launcher
EXE. Keep game/, music/, state/ and your CyberGladiatorsFix.ini.
On a normal launch, verified project-owned compatibility DLLs may be
replaced with the newly bundled versions using SHA-256 ownership receipts.

v1.x -> v2.x: requires a FRESH folder. There is NO automatic migration of
old v1.x directories or saves. Keep your previous installation separately.

INSTALLED DIRECTORY LAYOUT
--------------------------
CyberGladiatorsFix.exe
CyberGladiatorsFix.ini
CyberGladiatorsFix_installer.log
CyberGladiatorsFix.log
game/
  cyber.exe              verified and patched original retail executable
  CYBER.EXE.bak          preserved, SHA-256-checked original executable
  cyber.ini
  fix.dll                owned x86 WinMM/MCI/FLAC compatibility bridge
  glide.dll              owned x86 GameFrame Glide/OpenGL renderer
  graphics/, iq/, ...    original installed game assets
music/
  Track02.flac ... Track15.flac
state/
  CyberGladiatorsFix.manifest
  CyberGladiatorsFix.sha256
  CyberGladiatorsFix.gameframe.sha256
  CyberGladiatorsFix.install-receipt.txt
  CyberGladiatorsFix.first-launch-complete.txt (after successful first game run)
docs/
  README.md, license notices, INI example, build-info.txt

The original CUE/BIN is never modified. Immutable retail resources are
verified by SHA-256 on normal launches; game-writable IQ resources are
strictly hashed on initial installation and checked for safe regular-file
presence thereafter. Unknown, malformed, foreign or receipt-mismatched
runtime DLLs are NOT silently overwritten. Installer-owned DLL replacements
use staging and rollback but leave no permanent runtime DLL .bak files.
The original retail CYBER.EXE.bak is a separate required backup and stays.

CONFIGURATION
-------------
CyberGladiatorsFix.ini defaults to:
  [Video]
  SkipIntro=0

  [Debug]
  Log=0

SkipIntro=1 bypasses the opening AVI. FLAC/CDDA music is enabled by default
and is not exposed as a separate INI setting. The first installation and
first real game session produce separate logs beside the launcher. Later
Log=0 preserves the logs; Log=1 regenerates fresh, noncumulative logs.

COMMON COMMANDS
---------------
CyberGladiatorsFix.exe                     auto-install / verify / play
CyberGladiatorsFix.exe --install-only      install / prepare, do not play
CyberGladiatorsFix.exe --verify            verify existing installation
CyberGladiatorsFix.exe --launch-no-verify  launch WITHOUT asset preflight
CyberGladiatorsFix.exe --inspect image.iso
CyberGladiatorsFix.exe --inspect image.cue [track.bin|track.img]
CyberGladiatorsFix.exe --verbose           optional diagnostic modifier
CyberGladiatorsFix.exe --version
CyberGladiatorsFix.exe --help              also -h

--launch-no-verify is diagnostic and deliberately skips integrity/presence
checks, install and repair. It is NOT a recovery tool. --inspect logs an
inventory and local source paths to CyberGladiatorsFix_inspect.log; review
private path details before sharing it.

VIDEO / GE-PROTON FIX (NEW IN v2.0.1)
-------------------------------------
The native original AVI plays with the WinMM/MCI bridge; 4:3 presentation
and GameFrame 60 FPS borderless desktop-size rendering are preserved.

On GE-Proton, a separately managed AVI window could be hidden behind the
game's black startup window (audio still played); a manual Alt+Tab made the
picture appear. v2.0.1 automatically recognizes GE-Proton from known runner
metadata, creates a video popup owned by the real game window, and therefore
preserves the correct owner/owned stacking order. No replacement decoder or
synthetic Alt+Tab is used.

On native Windows and on other Wine runners, the previous independent video
window behavior is retained. Users do not need extra Lutris environment
variables. For diagnostics only, CYBER_AVI_GAME_OWNED_HOST=0 disables the
GE-Proton auto choice; =1 forces owned-host on supported Wine movie paths.

TESTING SCOPE AND LIMITATIONS
-----------------------------
- User visually confirmed automatic AVI display under Lutris GE-Proton US
  on build 2.0.0-1df03404 without a manual override.
- Native Windows FR post-merge build 2.0.0-436cbe1e: game owner remained
  independent, native MCI AVI, focus changes, GameFrame handoff, FLAC and
  clean exit 0 were logged; user reported all OK.
- Wine non-GE FR build 1df03404: independent-host path, movie, focus
  recovery, GameFrame handoff and clean exit 0 recorded.
- Earlier native Windows US v2.0.0 testing confirmed AVI and 3D/CDDA.
- Windows CI of merged 436cbe1e passed; release-preparation and final
  tagged v2.0.1 builds need their own CI verification.

These tests DO NOT prove unskipped complete AVI playback on every runner,
visual parity for all GPUs, native Windows US on the exact new merged SHA,
or a real-host smoke test of the future tagged v2.0.1 ZIP. The release is
manual and requires separate maintainer approval.

SIX-FILE DISTRIBUTION AND LICENSES
----------------------------------
The ZIP includes exactly:
  CyberGladiatorsFix.exe
  README.txt
  LICENSE-GPL-3.0
  LICENSE-APACHE
  LICENSE-MIT
  SHA256SUMS.txt

The release additionally provides a separate ZIP .sha256 checksum asset.
No standalone DLLs, game executables, CUE/BIN, original assets or FLAC are
distributed.

CyberGladiatorsFix / fix.dll: GPL-3.0-or-later
GameFrame: Apache-2.0
isomage: MIT

Source: https://github.com/legluondunet/CyberGladiatorsFix
Notes: docs/RELEASE_NOTES_v2.0.1.md
Readiness: docs/RELEASE_READINESS_v2.0.1.md
Release process: docs/RELEASE_PROCESS.md

Existing v1.0.0, v1.0.1 and v2.0.0 public packages are not modified.
