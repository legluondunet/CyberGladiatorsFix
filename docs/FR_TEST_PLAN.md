# CyberGladiatorsFix — French retail acceptance tests

**Experimental PR #5, NOT a public release.** Retain your last working US build. No French executable, disc data, music or IQ assets are distributed by this project.

## Test environment

- Use the Windows x64 **PreRelease** artifact produced by the latest successful GitHub Actions run for `feature/french-edition`.
- Create a **new, empty, writable directory** (not your working US game directory).
- Copy into it exactly `CyberGladiatorsFix.exe`, `CYBERG.CUE`, and the original matching `CYBERG.BIN`.
- Under Wine-staging 11.17, first test **without a virtual desktop**, with normal desktop resolution. Log hardware/configuration and any resolution switch observed.
- No nGlide, external `glide.dll`, old executable, game data or other CUE should be present in this test folder.

## Group A — full installation and native AVI

1. Run `WINEDEBUG=-all wine CyberGladiatorsFix.exe` in the fresh directory.
2. The log must show `Detected edition: France (CYBERG, 1996)`, original game-resource hashes verified, backed-up original FR `CYBER.EXE.bak`, and edition-specific patching (no US offsets).
3. Confirm the intro plays **completely without pressing Space**. It must be centered 4:3 with black pillarbox sides, no desktop flash or unintended display resolution change.
4. Verify GameFrame takes over on the first successful real OpenGL SwapBuffers. Look for `GAME_EDITION selected=FR`, `DDRAW_NATIVE_SUMMARY` (on Wine) and `GAMEFRAME_FIRST_GL_HANDOFF`.
5. Check main menu, controls, gameplay speed around 60 FPS, original French text and the game audio/music. The music directory should contain `Track02.flac` … `Track15.flac` if the entire disc was present.
6. Exit normally and check `GAME_EXIT exit code: 0`, `[SESSION] END success`, and the recorded Glide restoration/retention result. Upload the entire `CyberGladiatorsFix.log`.

## Group B — second launch without disc + integrity

1. Move (do not delete) `CYBERG.CUE` and `CYBERG.BIN` out of the folder.
2. Start `CyberGladiatorsFix.exe` again; it should detect the existing installation and run without requiring the physical CD.
3. Run `WINEDEBUG=-all wine CyberGladiatorsFix.exe --verify`; the original retail backup and 343 extracted CD files (minus intentionally excluded legacy drivers, if any) must verify without unexpected IQ modifications.
4. Confirm that a **non-matching EXE**, modified `SHELL.FNT` or altered `glide.dll` is rejected or preserved, not silently overwritten. Only perform corruption tests in a throwaway copy.

## Group C — Wine/Proton regression and Alt+Tab

- Exercise the intro skip once with Space and verify smooth transition without black screen or desktop preview.
- Alt+Tab to another window and back; check graphics, keyboard/mouse input and focus.
- Test under Proton as well as Wine-staging, and compare native desktop geometry before/after startup. Do not assume unchanged reported dimensions prove unchanged physical color depth.

## Group D — unchanged US edition

Repeat a quick verified install/launch and video/FLAC/exit test with the known US retail edition in a separate directory. The US code path must continue to use its original file hashes, offsets, HWND and WndProc.

## Required before merging PR #5

- The experimental Windows CI build passes all unit, ABI, package and native DLL-loader checks.
- At least one complete French Wine-staging game run, including full AVI, no-CD second boot, FLAC, frame rate, Alt+Tab and shutdown.
- US regression test.
- Native Windows gameplay and clean CUE/BIN installation still gate the **first public release**.

**Never upload retail game data or original CD tracks to GitHub.** Logs and SHA-256 hashes are sufficient for review.

legluondunet assisted by ChatGPT (OpenAI).
