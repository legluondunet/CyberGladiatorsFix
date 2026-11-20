CyberGladiatorsFix — Windows x64 release candidate and distribution guide
==========================================================

The distribution contains exactly one Windows x64 launcher, LICENSE-GPL-3.0,
LICENSE-APACHE, LICENSE-MIT (isomage 2.1.0), this README and SHA256SUMS.txt. The EXE embeds BOTH 32-bit Windows DLLs:
  1. fix.dll — our GPLv3 WinMM/Cinepak AVI and FLAC bridge
  2. glide.dll — Apache-2.0 GameFrame Glide-to-OpenGL wrapper

The original Cyber Gladiators (1996) game and its copyrighted CD audio
are NOT included. No nGlide, DXVK or standalone video player is shipped.

QUICK START
-----------
Place CyberGladiatorsFix.exe beside an authenticated original US or FR
retail CUE/BIN image and double-click the EXE. Under Wine/Proton use:

  wine CyberGladiatorsFix.exe

No --launch or additional option is required. The launcher searches
for an existing game/ installation first, verifies game
file hashes, then starts it. If no installation exists, it finds one
adjacent .cue file, extracts the original files from the referenced .bin
tracks into a temporary staging folder, verifies all extracted files,
installs the embedded DLLs and starts the game. A completed installation
does not require the CD image again. An absent or ambiguous CD or an
unverified existing installation triggers a clear error, never a
silent replacement of the player's files.

Supported retail images: exact-hash authenticated US and FR CUE/BIN,
including separate CD audio tracks. A verified two-second (150-sector)
all-zero pause before the first AUDIO INDEX 01 can be recognized when
INDEX 00 is omitted in a single raw 2352-byte BIN; source CUE/BIN are
never altered and strict CD-ROM sector/resource checks remain active.
Standalone ISO support is a future option, NOT part of this build.

Game installation — upcoming clean-layout branch (PR #27; not v1.0.1):
  CyberGladiatorsFix.exe
  CyberGladiatorsFix.ini
  CyberGladiatorsFix_installer.log
  CyberGladiatorsFix.log
  game/
    cyber.exe
    CYBER.EXE.bak     (original retail backup, SHA-256 verified)
    cyber.ini
    fix.dll
    glide.dll
    graphics/
    iq/
    ... original media, saves
  music/
    Track02.flac .. Track15.flac
  state/
    CyberGladiatorsFix.manifest
    CyberGladiatorsFix.sha256
    CyberGladiatorsFix.gameframe.sha256
    CyberGladiatorsFix.install-receipt.txt
  docs/
    README.md
    LICENSE-GPL-3.0
    LICENSE-APACHE
    LICENSE-MIT
    CyberGladiatorsFix.ini.example
    build-info.txt

The original CD audio is converted losslessly to sibling music/. These
tracks are verified through the root state/ manifest and loaded by a
child-scoped absolute path passed to fix.dll.

There is NO automatic migration from v1.0.1; fresh CUE/BIN installation
is required. Owner-managed DLLs may be upgraded without .bak files;
unknown DLLs are refused rather than overwritten.

SINGLE EXE, TWO DLLs
--------------------
The next branch bundles verified x86 fix.dll and GameFrame glide.dll in
one Windows x64 launcher. Both DLLs load beside game/cyber.exe and
are verified by exact hashes and installer ownership receipts.
Temporary replacement files are cleaned after activation, with no .bak.
The original retail CYBER.EXE.bak is retained independently.


The game is Glide 640x480, presented in 4:3 pillarboxed fullscreen at
the desktop resolution and limited to 60 FPS. The original Cinepak AVI
plays in native Wine/Windows MCI. Its temporary GDI video host stays
black after AVI close until the FIRST REAL successful GameFrame OpenGL
SwapBuffers, which releases the host on the original game UI thread.
A bounded timeout prevents indefinite black. In PR #6 the standard
launcher automatically enables one owned DirectDraw v1 COM and MCI
video implementation for authenticated US and FR retail editions on
both native Windows and Wine/Proton. The original logical 640x480x16
DirectDraw surfaces do not request a physical display-mode change.
Only the window-manager-specific Alt+Tab/focus adapter is selected
automatically. The former Wine native-size DirectDraw forwarding
experiment remains in diagnostic code, not in normal launch.

VALIDATION BEFORE PUBLIC RELEASE
--------------------------------
Start from your own CUE/BIN next to the EXE, not from an installed game.
Let the full intro AVI play, verify music, input, focus and 4:3 scene;
then exit normally. Start the EXE a second time without the CD files
to verify installed-game detection, and check that glide.dll persists.
Review both CyberGladiatorsFix_installer.log (launcher/install/verification)
and CyberGladiatorsFix.log (WinMM/CDDA and GameFrame gameplay) beside the EXE.
They carry the same session ID on a game launch; --verify leaves the game log
untouched. No special command, environment variable or INI option is needed for the shared
DirectDraw/MCI engine. All four US/FR × Windows 11/Wine-staging 11.17 combinations have now
been field-tested on normal, no-flag launch. FR/Wine was also checked
against a clean original CYBERG.cue install, repeat launch, video skip,
Alt+Tab and uninterrupted movie. Other systems are not guaranteed.

For the current v1.0.1 maintenance release, refer to [v1.0.1 release notes](RELEASE_NOTES_v1.0.1.md) and the [release process](RELEASE_PROCESS.md). The historical v1.0.0 package remains available unchanged.

Useful commands:
  CyberGladiatorsFix.exe                 auto-install/verify and play
  CyberGladiatorsFix.exe --install-only  install/upgrade but do not play
  CyberGladiatorsFix.exe --verify        strict non-IQ files; IQ presence
  CyberGladiatorsFix.exe --inspect image.iso
  CyberGladiatorsFix.exe --inspect image.cue [track.bin|track.img]
  CyberGladiatorsFix.exe --verbose       optional detailed logging, combinable
  CyberGladiatorsFix.exe --launch-no-verify  run installed game without file verification or repair
  CyberGladiatorsFix.exe --version
  CyberGladiatorsFix.exe --help

GitHub Actions artifacts are temporary test packages (retained 30 days), NOT public releases. Public distribution requires the reviewed tag-built six-file package attached to a GitHub Release. The CI workflow does not publish releases automatically.
The baseline implementation and audit were merged into main on 2026-11-06.
Merged PR #6 introduced the common owned-COM DirectDraw/MCI engine,
selected automatically on normal launch, plus the verified original US
CUE pregap compatibility. The default all-host selection has now been field-validated for both
retail editions on Windows 11 and Wine-staging 11.17, including FR/Wine
fresh install, first-key skip and complete movie playback. A public
release must not claim universal hardware or window-manager compatibility.
A green CI workflow checks compilation, PE exports and loading,
but DOES NOT verify actual video or gameplay on a player's machine.

PACKAGE AND RELEASE CHECKS
--------------------------
The package must contain exactly six files:
  CyberGladiatorsFix.exe
  LICENSE-GPL-3.0
  LICENSE-APACHE
  LICENSE-MIT
  README.txt
  SHA256SUMS.txt

SHA256SUMS.txt covers the first five files. Verify with
"sha256sum -c SHA256SUMS.txt" from the extracted package under Linux,
or Get-FileHash SHA256 under PowerShell. The executable version suffix
must match the source commit used for this distribution. Only the source
game's original authenticated CUE/BIN data is installed.

The build signature appears at the upper-right 4-pixel border of the
centered 4:3 fighter-selection menu and disappears during gameplay.

See docs/RELEASE_READINESS.md, docs/RELEASE_NOTES_v1.0.0.md and
docs/RELEASE_PROCESS.md in the source repository for validated platforms,
known limitations and the manual tag/release procedure. The latest
Windows FR test of the new archive behavior and strict INI configuration
passed on 2026-11-09. A future tag release still needs final validation
on the exact tagged build.

HISTORICAL TEST REFERENCES
--------------------------
Build #37, 2026-11-06, commit 63e0d7dd7490162bd2701f847a333f9f06d6d2f2:
The user verified a complete fullscreen 4:3 AVI, a seamless transition
with no desktop flashes, a stable 1920x1080 borderless game at ~59.6 FPS,
FLAC soundtrack and clean process exit under Wine. Build #42 proved the
one-file dual-DLL packaging and restored the previously installed DLL under its old policy; the upcoming unified-layout build **rejects unowned DLLs** and **replaces only installer-owned files without retained .bak backups**.
Subsequent builds removed failed/unused GDI-child experiments.

Third-party GameFrame pin:
  https://github.com/tanepiper/game-frame
  f195a074a63cc150673c38becdec696163b0b3d1
  Apache-2.0, full license text in LICENSE-APACHE

CyberGladiatorsFix license: GPL-3.0-or-later (LICENSE-GPL-3.0)
isomage 2.1.0: MIT (original copyright 2025 Jack Danger in LICENSE-MIT)
Authorship: legluondunet assisted by ChatGPT (OpenAI)
