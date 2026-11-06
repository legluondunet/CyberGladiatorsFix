# GameFrame — validated Wine baseline and one-EXE packaging

**Validated baseline:** Git commit 63e0d7dd7490162bd2701f847a333f9f06d6d2f2,
GitHub Actions run 42221121327 (build #37).
Tested by user on 2026-11-06 with Wine 11.0 and a 1920x1080 120Hz desktop.

Confirmed: entire original AVI plays fullscreen in 4:3; the video-to-game
transition is seamless, with no desktop or temporary game window; the real
game input HWND and aspect ratio are correct; no window flicker; ~59.6 FPS
for normal gameplay; FLAC CDDA works; normal process exit code 0.
The exact validated invocation was:
  CyberGladiatorsFix.exe --launch --gameframe-test --avi-first-present-handoff

## One-file packaging experiment (not yet user-validated)
This repository now embeds the 32-bit GameFrame Glide DLL inside the Windows
x64 EXE, as it already does for the 32-bit WinMM bridge. The artifact contains
ONE Windows executable, plus license and documentation. At runtime the EXE
must still place the 32-bit DLLs beside the original CYBER.EXE; one EXE is a
distribution property, not a claim that the game runs without DLLs.

User-facing command:
  CyberGladiatorsFix.exe --launch --gameframe

This now chooses the proven first-real-OpenGL-frame handoff, formerly behind
the --avi-first-present-handoff diagnostic flag. The latter remains as a
compatibility alias. The existing separately supplied nGlide launch remains
unchanged. A temporary Glide switch preserves any existing validated x86
glide.dll to a hash-qualified backup and restores it on normal game exit.
Unknown files are not overwritten without a verified preserved copy.
Backups surviving crashes are checked on subsequent explicit GameFrame runs;
multiple or ambiguous backups abort instead of guessing. The sole session
log remains CyberGladiatorsFix.log beside the launcher.

## Rendering architecture
GameFrame source: https://github.com/tanepiper/game-frame
Pinned revision: f195a074a63cc150673c38becdec696163b0b3d1
License: Apache-2.0, upstream LICENSE-APACHE bundled.
Our CyberGladiatorsFix launcher and additions: GPL-3.0-or-later.
156 decorated Win32 Glide stdcall exports are verified in Windows CI.

Wine's MCIAVI/GDI paint DC is clipped at 640x480 inside the OpenGL game's
child window despite GetClientRect reporting 1920x1080. Attempting to draw a
centered larger AVI there produced a clipped or black video. Do NOT restore
those failed experimental modes as defaults. The validated implementation
uses a separate native MCI/GDI video host, retains its final black curtain
through startup, then releases it on the game UI thread only after the
GameFrame OpenGL render thread has completed a REAL successful SwapBuffers.
A 3500ms guarded fallback avoids indefinite black screens.
The GameFrame render/Glide HWND remains the actual game input window.
The DLL is open source, but there is no AVI decoding into an OpenGL texture.

## Pending validation
- Native Windows 10/11 gameplay/video/Alt+Tab/exit (CI cannot run the game).
- New automatic embedded DLL install/rollback under Wine and Windows.
- Complete package license review for all distributed components.
- No merge of PR #2 until further user approval.

## Rollback and emergency recovery
The stable tested build is #37 above. The new one-EXE build is experimental.
If the launcher fails after replacing the game DLL and cannot restore it,
exit the game completely, preserve any game/glide.dll and restore the
hash-qualified glide.dll.pre-gameframe-<hash>.bak from the game folder to
glide.dll manually. Do not touch CYBER.EXE, IQ files, CD images or INI.
