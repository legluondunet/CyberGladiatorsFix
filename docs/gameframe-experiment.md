# GameFrame Glide — isolated experimental proof of concept
#
# Only for Cyber Gladiators on Windows 10/11 or Linux Wine/Proton.
# NOT a release and NOT a replacement for the validated nGlide path.

Source: https://github.com/tanepiper/game-frame
Pinned upstream revision: f195a074a63cc150673c38becdec696163b0b3d1
GameFrame licensing: Apache-2.0 (LICENSE-APACHE enclosed).
Prototype build: Windows x86 PE32 glide2x.dll from upstream Rust source, renamed
to glide.dll. This directory contains NO original game binaries or assets.

This is a deliberately standalone Glide experiment, not yet an embedded
CyberGladiatorsFix component. The original game imports glide.dll.

WHAT WAS CHECKED
- Source revision and original decorated stdcall export table
- 32-bit Windows DLL format
- 156 upstream exported functions (see build logs)
- Presence of Glide 2.11 functions such as grSstOpen

WHAT IS NOT YET VERIFIED
- Every CYBER.EXE import (its PE import table is not bundled in this repo)
- Actual menu/gameplay rendering, texture correctness, video handoff
- Windows 10/11 gameplay or Wine/Proton gameplay
- Dependency license attribution beyond upstream Apache-2.0
- Frame pacing, input and CD music with this wrapper

SAFE FIRST TEST PROCEDURE (opt-in only)
1. Stop Cyber Gladiators, CyberGladiatorsFix and related Wine processes.
2. Find the installed game directory containing CYBER.EXE, normally
   <CyberGladiatorsFix.exe folder>/CyberGladiatorsFix/.
3. COPY the currently installed glide.dll (nGlide) to a safe external
   backup named glide.dll.nglide-backup; do not modify CYBER.EXE or IQ data.
4. Copy this experimental glide.dll into that game directory.
5. Launch CyberGladiatorsFix.exe using your existing installation. For a
   direct baseline start with the normal --launch; if it works, compare the
   normal --launch --smooth-video-test experiment with SkipIntro=0. The
   validated smooth-video path was built for nGlide and may require new
   GameFrame-specific handoff logic. Do not assume it works unchanged.
6. Note whether the game reaches its menu and whether the picture is correct.
   Log files from this experimental wrapper may be separate from the
   CyberGladiatorsFix log: the full one-log integration is a later step.
7. Stop the game. Restore the ORIGINAL glide.dll from the backup. Keep this
   experimental DLL only for A/B testing.

If you see missing exports, visual defects, a black screen, a crash or a
nonresponsive game, restore nGlide. The prototype DLL does not need nGlide
configuration values (NGLIDE_BACKEND etc.) and ignores them.

NO GAME DATA ALTERATIONS ARE PART OF THIS EXPERIMENT.
Neither the CI build nor the export audit needs a copy of Cyber Gladiators.
