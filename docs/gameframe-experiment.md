# GameFrame Glide / CyberGladiatorsFix — isolated experimental build

Only for Cyber Gladiators on Windows 10/11 and Wine/Proton.
Not a release. Does not replace the original stable nGlide code or PR #1.

UPSTREAM AND LICENSE
Source: https://github.com/tanepiper/game-frame
Pinned revision: f195a074a63cc150673c38becdec696163b0b3d1
GameFrame is Apache-2.0; the upstream LICENSE-APACHE is bundled.
The GPLv3 CyberGladiatorsFix launcher remains GPLv3.
No original CD-ROM game files or proprietary Glide binaries are distributed.

WHAT WE OBSERVED IN THE FIRST GAME TEST
The unmodified upstream GameFrame wrapper loads under Wine 11.0, renders
3D characters and textures, plays the game's music and exits normally.
However, it could create a separate drawing window while the original
CYBER.EXE input window stayed smaller, the picture was not fullscreen,
the keyboard focus was inconsistent, and the game ran too fast.

WHAT THIS EXPERIMENT CHANGES
The paired modified x86 glide.dll:
- Uses the original game-owned Win32 HWND (checked for the verified game
  image address and process owner) instead of the first visible window.
- Uses a desktop-sized borderless window on the selected monitor, WITHOUT
  changing the monitor's display mode or refresh rate.
- Fits original Glide pixels at their correct 4:3 aspect ratio, with black
  pillarboxes on a 16:9 display. The original game's framebuffer is unedited.
- Paces the game's grBufferSwap calls to 60 FPS instead of allowing the
  simulation to run too fast at the desktop's 120 Hz refresh.
- Logs to the existing single CyberGladiatorsFix.log beside the launcher.

The paired CyberGladiatorsFix.exe exposes an EXPLICIT new command:
  CyberGladiatorsFix.exe --launch --gameframe-test
It verifies the matching Glide DLL by SHA-256 and refuses another build,
keeps the original native MCI intro 4:3 on the full monitor with the
smooth-mode pre-video cover, and destroys that temporary video window
immediately after successful MCI close, BEFORE the GameFrame OpenGL
renderer starts. The nGlide-specific WineD3D WndProc replacement is OFF.
Other launch commands and the validated nGlide Direct3D9 path are unchanged.

GAMEFRAME AVI-IN-WINDOW COMPARISON (NEW EXPERIMENT)
GameFrame does not decode splash4.avi: the game's original Cinepak AVI
continues to be decoded by the Windows/Wine WinMM/MCI video driver.
However, the alternative command displays it inside the SAME original
GameFrame OpenGL game window, via a 4:3 centered native Win32 child:
  CyberGladiatorsFix.exe --launch --gameframe-test --avi-in-game-window

Unlike the existing --launch --gameframe-test (control), this option
does NOT create a separate, independent, topmost fullscreen MCI host.
Before MCI opens, the native game HWND is made desktop-sized, and the
movie's black host is a CHILD of that HWND. After MCI closes, ONLY that
child is destroyed; the parent game HWND and its OpenGL context survive.
The movie's sound, skip keys, original assets and 60 FPS GameFrame
limiter are unchanged. We preserve its native MCI decoder, not an
unverified new OpenGL AVI texture decoder.

This mode is intended to compare the visible AVI-to-3D transition and
focus/window behavior on Wine with our previous nGlide experiments.
The existing --launch --gameframe-test WITHOUT the extra flag stays
unchanged for A/B testing and rollback. If the new in-window path
renders video black, hides the picture, changes focus, or crashes,
re-run the standard GameFrame test command from the SAME matched
archive; no reinstall or rollback is necessary.

Build #20: fixes based on the first GameFrame in-window playback report.
The AVI child is correctly 1440x1080 but the game HWND can revert to a
640x480 client area DURING MCI playback. That clips the child, making only
the left section of the movie visible. The WinMM bridge now checks and
repairs the original parent window during black pre-roll, before MCI
reveals frames and periodically (every eighth native status poll) while
the video plays. It logs actual parent, host and video-surface client
geometry, which distinguishes parent clipping from a codec/scaling issue.
No change to the Wine MCI decoder, 4:3 ratio or video image contents.

After MCI close, the game also resets the original HWND to 648x513.
The modified GameFrame wrapper now performs ONE additional guarded
pre-presentation geometry correction before issuing the FIRST OpenGL
buffer swap (only in --avi-in-game-window). The older 30-swap
debounced recovery remains as a failsafe, without repeated
SetWindowPos requests. This keeps the 60 FPS cap and the stable
no-flicker mode.

Log markers:
  GAMEFRAME_VIDEO_IN_WINDOW enabled via launcher
  GAMEFRAME_HANDOFF_PREPARE stage=before_MCI_AVIVIDEO_open
  GAMEFRAME_IN_WINDOW_CREATED parent_is_game=1 independent_top_level=0
  GAMEFRAME_MOVIE_GEOMETRY parent_client=... host_client=... surface_client=...
  GAMEFRAME_FIRST_PRESENT_PREPARE before_GL_swap=1 (if 640x480 reset is found)
  GAMEFRAME_HANDOFF_PREPARE stage=after_MCI_close_before_host_destroy
  HOST_CLOSED reason=gameframe_in_window_movie_closed

This experiment is not true texture-based decoding in OpenGL. If it
still shows a compositor glitch, a separate decoded-frame upload to
GameFrame's GL framebuffer could be investigated later, subject to
proving the necessary codecs/decoding and frame timing are reliable
under both Windows and Wine.

HOW TO TEST (SAFE, REVERSIBLE)
1. Fully exit Cyber Gladiators and Wine processes for this game.
2. Back up the currently installed CyberGladiatorsFix.exe (the launcher).
3. Back up the installed glide.dll beside CYBER.EXE as glide.dll.nglide-backup.
   Do not change CYBER.EXE, the original IQ files or any CD image.
4. Put the CyberGladiatorsFix.exe supplied IN THIS SAME ARCHIVE beside the
   existing launcher (replacing the previous executable only).
5. Put the glide.dll from THIS SAME ARCHIVE in the CyberGladiatorsFix game
   directory beside CYBER.EXE. Never mix an old GameFrame glide.dll with
   the new launcher: SHA-256 validation intentionally rejects it.
6. Retain SkipIntro=0 in the existing CyberGladiatorsFix.ini.
7. From your working directory with the launcher, run:
     wine CyberGladiatorsFix.exe --launch --gameframe-test
   (In Lutris, use those two arguments; do not add --nglide-vulkan.)
8. Observe: intro fullscreen, menu fullscreen/centered, input and MangoHud,
   gameplay speed, music and Alt+Tab, and whether the game exits normally.
   The unified CyberGladiatorsFix.log contains the new GameFrame records.
9. If anything goes wrong, restore the original glide.dll from
   glide.dll.nglide-backup and the original CyberGladiatorsFix.exe.

The new branch is a prototype, not a final fix. Actual Windows 10/11
gameplay and all edge cases still need user testing. If either new DLL
or EXE cannot be verified, the game is NOT launched in this mode.
