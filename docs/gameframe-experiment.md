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

GAMEFRAME FULLSCREEN GDI VIDEO + FIRST OPENGL PRESENT HANDOFF (NEW EXPERIMENT)
The user's #30 no-upscale probe shows an entirely BLACK movie. The native
WinMM driver reports source 0,0 640x480 and requested destination
640,300 640x480 correctly; the decisive GDI device-context result is:
  GAMEFRAME_GDI_DC clip=(0,0,640,480)
The destination begins at x=640, OUTSIDE that clip. The AVI continues
playing; its decoded frames are invisible. In previous 1440x1080 attempts
the same 640x480 clip likely caused the partial picture.

A Wine MCI/GDI child window inside the OpenGL game HWND cannot be
assumed to retain a desktop-size GDI clip, even when GetClientRect is
1920x1080. Instead of additional MCI child-window geometry changes,
this NEW, separate option keeps the already proven FULL native AVI on
an independent, temporary GDI-hosting HWND, then hands over cleanly:

  CyberGladiatorsFix.exe --launch --gameframe-test --avi-first-present-handoff

At successful native MCI close, the video is hidden, the fullscreen GDI
host is painted black and marked ready. The actual GameFrame OpenGL
render thread reports successful SwapBuffers via a Win32 message to
that host's ORIGINAL owner UI thread, which then closes the host.
This avoids exposing CYBER.EXE's interim 648x513 window between phases.
The frame notification is sent AFTER real SwapBuffers, not on the
call to grBufferSwap or a timer that merely guesses frame readiness.
A bounded 3500 ms owner-thread fallback prevents a permanent black host
if a GL presentation notification cannot be delivered.

This option is scoped to the matched experimental EXE and GameFrame
glide.dll. Its normal in-window GDI AVI mode, unscaled crop diagnostic,
nGlide launch paths, CD music, 60 FPS, unmodified game assets and one log
remain untouched. This is NOT decoding video in GameFrame's OpenGL
texture; it is a temporary compatible GDI window, with renderer-driven
handoff. The temporary host is a single extra top-level window during
MCI playback, and is not represented as a true one-window implementation.
Native Windows 10/11 and Wine rendering must be validated in-game.

New markers:
  GAMEFRAME_FIRST_GL_HANDOFF armed=1 ...
  glide: GAMEFRAME_FIRST_GL_PRESENT posted=1 ...
  GAMEFRAME_FIRST_GL_HANDOFF notified=1 ...
  HOST_CLOSED reason=gameframe_first_GL_presented ...
  fallback marker GAMEFRAME_FIRST_GL_HANDOFF timed_out=1

GAMEFRAME MCI GDI CLIPPING DIAGNOSTIC (BUILD #25)
The #24 test confirms the user still sees a clipped movie, despite source
640x480 and measured Win32 game/host/MCI child dimensions remaining full
1920x1080 during playback. Moving the paint HWND to (0,0) and centering
the MCI destination did not solve it. Do not keep changing arbitrary
window offsets.

This build adds an opt-in control to isolate native GDI scaling:
  CyberGladiatorsFix.exe --launch --gameframe-test --avi-in-game-window --avi-unscaled-probe

This intentionally displays the original 640x480 Cinepak frame at native
size, centered on the 1920x1080 monitor at (640,300), WITHOUT the usual
1440x1080 destination StretchBlt scaling. It is a *diagnostic*, not the
desired user-facing resolution or final correction. The standard
--avi-in-game-window mode continues using 4:3 centered 1440x1080.

In BOTH in-window modes the WinMM bridge now reads the native MCI
driver's actual source/destination/destination_max/window rectangles
using read-only "where movie" queries after destination setup, after
MCI play, and at the first native status poll. It also samples the GDI
device context clip rectangle and logical/window viewport origins.
Look for GAMEFRAME_MCI_DRIVER_RECT and GAMEFRAME_GDI_DC markers in the
single CyberGladiatorsFix.log.

If the original 640x480 AVI is entirely visible in this diagnostic,
Wine's GDI StretchBlt inside a GameFrame OpenGL child is the likely
problem. We should then investigate converting decoded video frames to
a GameFrame OpenGL texture, rather than further window repositioning.
If it remains cropped unscaled, inspect native MCI source rectangle,
paint HDC clip and overlapping GL/GDI surface composition first.

This test does not change other GameFrame modes, the intact nGlide
launcher, game IQ data, audio playback, or the already validated
pre-first-present fullscreen recovery.

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

Build #23: correct partial AVI on Wine's native WinMM/GDI output.
The previous GameFrame mode centered the AVI by moving the MCI paint CHILD
to (240,0), then asked Wine to draw 1440x1080 from that window's origin.
On a 1920x1080 screenshot, only x=780..1679 contained decoded video:
the first 540 visible pixels were unexpectedly missing, despite ALL
window client geometries reporting the correct dimensions. This points
to a nested child HDC/destination-coordinate mismatch, not parent clipping.

For --avi-in-game-window ONLY, the MCI paint HWND and the initial black
cover now occupy the ENTIRE game client, starting at (0,0), 1920x1080.
The native WinMM/MCI destination is instead centered at
(240,0) with size 1440x1080. Avoid positioning and scaling the same
video at two separate layers. All other modes, the 60 FPS limiter,
the automatic 648x513 pre-first-present recovery and original game
files remain unchanged.

Diagnostic markers:
  GAMEFRAME_MCI_SURFACE_ORIGIN origin=(0,0) client=1920x1080
  MCI_REWRITE destination=240,0 1440x1080
  GAMEFRAME_MOVIE_GEOMETRY ... surface_client=1920x1080

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
