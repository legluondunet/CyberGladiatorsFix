CyberGladiatorsFix — Windows x64 pre-release testing guide
==========================================================

The testing artifact contains the Windows x64 launcher, LICENSE-GPL-3.0,
LICENSE-APACHE and this README. The EXE embeds BOTH 32-bit Windows DLLs:
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
remain untouched; AllowModifiedIQ=1 tolerates only recognized same-size
IQ modifications for launch, never for --verify.

SINGLE EXE, TWO DLLs
--------------------
Both DLLs have SHA-256 and PE32 checks before installation. The first
clean install retains GameFrame glide.dll after gameplay. If the user
already has a different valid glide.dll, it is backed up byte-for-byte
and restored after the game process exits; known, unknown, ambiguous
and symlink cases are treated conservatively.

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

This is a pre-release testing artifact, NOT a public release.
The baseline implementation and audit were merged into main on 2026-11-06.
PR #6 (not yet merged) selects the common owned-COM DirectDraw/MCI engine
automatically on normal launch and supports the verified original US CUE
pregap. The default all-host selection has now been field-validated for both
retail editions on Windows 11 and Wine-staging 11.17, including FR/Wine
fresh install, first-key skip and complete movie playback. A public
release must not claim universal hardware or window-manager compatibility.
A green CI workflow checks compilation, PE exports and loading,
but DOES NOT verify actual video or gameplay on a player's machine.

HISTORICAL TEST REFERENCES
--------------------------
Build #37, 2026-11-06, commit 63e0d7dd7490162bd2701f847a333f9f06d6d2f2:
The user verified a complete fullscreen 4:3 AVI, a seamless transition
with no desktop flashes, a stable 1920x1080 borderless game at ~59.6 FPS,
FLAC soundtrack and clean process exit under Wine. Build #42 proved the
one-file dual-DLL packaging and restored the previously installed DLL.
Subsequent builds removed failed/unused GDI-child experiments.

Third-party GameFrame pin:
  https://github.com/tanepiper/game-frame
  f195a074a63cc150673c38becdec696163b0b3d1
  Apache-2.0, full license text in LICENSE-APACHE

CyberGladiatorsFix license: GPL-3.0-or-later (LICENSE-GPL-3.0)
Authorship: legluondunet assisted by ChatGPT (OpenAI)
