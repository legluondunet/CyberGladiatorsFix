CyberGladiatorsFix — Windows x64 release candidate and distribution guide
==========================================================

The distribution contains exactly one Windows x64 launcher, LICENSE-GPL-3.0,
LICENSE-APACHE, LICENSE-MIT (isomage 2.1.0), this README and SHA256SUMS.txt. The EXE embeds BOTH 32-bit Windows DLLs:
  1. CyberGladiatorsFix.dll — our GPLv3 WinMM/Cinepak AVI and FLAC bridge
  2. glide.dll — Apache-2.0 GameFrame Glide-to-OpenGL wrapper

The original Cyber Gladiators (1996) game and its copyrighted CD audio
are NOT included. No nGlide, DXVK or standalone video player is shipped.

QUICK START
-----------
Place CyberGladiatorsFix.exe beside an authenticated original US or FR
retail CUE/BIN image and double-click the EXE. Under Wine/Proton use:

  wine CyberGladiatorsFix.exe

No --launch or additional option is required. The launcher searches
for an existing CyberGladiatorsFix/ installation first, verifies game
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

Game installation:
  CyberGladiatorsFix/
    cyber.exe
    CYBER.EXE.bak
    glide.dll
    CyberGladiatorsFix.dll
    CyberGladiatorsFix.ini
    CyberGladiatorsFix.manifest
    music/
      Track02.flac
      ...
    (original game IQ, graphics, AVI and other data)

Music is stored under music/TrackNN.flac, not the game root. An old
verified root-level FLAC installation migrates automatically without
lossy re-encoding. The old manifest is archived. Original IQ resources
remain untouched, and their SHA-256 verification is ALWAYS strict at
launch and with --verify. The WinMM FLAC bridge is always enabled.

CyberGladiatorsFix.ini contains only the user-selectable [Video]
SkipIntro option. Old [Audio] and [Integrity] sections are automatically
removed on startup, without changing the selected SkipIntro value.
CDDA and integrity protections are internal, mandatory behaviors.

SINGLE EXE, TWO DLLs
--------------------
Both DLLs have SHA-256 and PE32 checks before installation. The first
clean install retains GameFrame glide.dll after gameplay. If an old,
unknown or malformed regular-file glide.dll exists, the launcher first
renames it to glide.dll.bak without overwriting any existing backup,
then installs the embedded GameFrame DLL and launches the game.
If glide.dll.bak is already occupied, it archives the old DLL under
glide.dll.<16-character-SHA256>.bak, then a numbered .bak if necessary.
After the game exits, GameFrame stays installed and the backup remains
available for manual recovery; it is NOT automatically restored.
Operations, original and embedded SHA-256 digests, selected archive path
and any collision fallback are recorded in CyberGladiatorsFix.log.
No size limit or PE32-validity check is imposed on the old DLL: only the
bundled GameFrame is required to be a validated PE32/SHA-256 file.
Symlinks are archived as links, without following their targets;
directory entries cannot be used as glide.dll.

The WinMM import name in the patched US retail executable now refers
to CyberGladiatorsFix.dll rather than the old CyberGladiatorsCDDA.dll.
The rename is verified by reversing only the known import name bytes
and checking the SHA-256 of the previously pinned retail patch.
The original CYBER.EXE.bak is never changed.

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
Review the ONE CyberGladiatorsFix.log next to the launcher. No special
command, environment variable or INI option is needed for the shared
DirectDraw/MCI engine. All four US/FR × Windows 11/Wine-staging 11.17 combinations have now
been field-tested on normal, no-flag launch. FR/Wine was also checked
against a clean original CYBERG.cue install, repeat launch, video skip,
Alt+Tab and uninterrupted movie. Other systems are not guaranteed.

Useful commands:
  CyberGladiatorsFix.exe                 auto-install/verify and play
  CyberGladiatorsFix.exe --install-only  install/upgrade but do not play
  CyberGladiatorsFix.exe --verify        strict SHA-256 check
  CyberGladiatorsFix.exe --inspect-disc original.cue   read-only CD inspection
  CyberGladiatorsFix.exe --max-graphics  max game settings
  CyberGladiatorsFix.exe --check-bundle  embedded DLL checks
  CyberGladiatorsFix.exe --version
  CyberGladiatorsFix.exe --help

GitHub Actions artifacts are temporary test packages (currently retained 14 days), NOT public releases. Public distribution requires the reviewed tag-built six-file package attached to a GitHub Release. The CI workflow does not publish releases automatically.
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

See docs/RELEASE_READINESS.md, docs/RELEASE_NOTES_v0.2.13.md and
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
one-file dual-DLL packaging and restored the previously installed DLL under its old policy; current builds **archive** older Glide DLLs as non-overwriting `.bak` files and **retain** GameFrame after exit.
Subsequent builds removed failed/unused GDI-child experiments.

Third-party GameFrame pin:
  https://github.com/tanepiper/game-frame
  f195a074a63cc150673c38becdec696163b0b3d1
  Apache-2.0, full license text in LICENSE-APACHE

CyberGladiatorsFix license: GPL-3.0-or-later (LICENSE-GPL-3.0)
isomage 2.1.0: MIT (original copyright 2025 Jack Danger in LICENSE-MIT)
Authorship: legluondunet assisted by ChatGPT (OpenAI)
