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
