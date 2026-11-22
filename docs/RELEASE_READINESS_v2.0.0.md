# CyberGladiatorsFix v2.0.0 — release readiness

**Status: source merged and tested; public release not yet published.** The v1.0.0 and v1.0.1 releases remain unchanged. The [manual publication process](RELEASE_PROCESS.md) requires **explicit approval to publish**, separately from approving source changes.

The v2.0.0 layout ([merged PR #27](https://github.com/legluondunet/CyberGladiatorsFix/pull/27)) **does not migrate v1.x installations**. [Merged PR #31](https://github.com/legluondunet/CyberGladiatorsFix/pull/31) fixed early DirectDraw cleanup on French and USA retail executables, and restored the real EXE taskbar icon on Wine/XFCE. Its latest field-tested source was `2b720877`, merged to `main` as `4ecd56d1`.

## Real-host acceptance evidence (20–22 November 2026)

| Test | Native Windows | Wine-staging 11.19 |
| --- | --- | --- |
| USA original disc installation | Fresh US CUE/BIN on `fdca004d`: 341 original game files, 14 FLAC tracks, 354 verified resources | Earlier fresh retail US CUE/BIN installs; latest `2b720877` validated on an existing installation |
| French original disc installation | **Final engine `9605dff4` native Windows (22 Nov):** *fresh* original `CYBERG.CUE`/BIN extraction, 343 files and all 14 lossless FLAC tracks; 356 CD resources postchecked (256 immutable SHA-256, 100 mutable IQ presence-only); both runtime modules verified | Earlier fresh retail FR CUE/BIN; latest Wine `2b720877` verified an existing no-disc installation (356 resources, 256 SHA-256 checks and 100 IQ presence checks) |
| Original AVI, DirectDraw and menu | FR `cbad024d`: maintainer visually confirmed video and Alt+Tab, clean exit. USA `fdca004d`: visible AVI, null-cleanup guard invoked, successful first GL/menu/FLAC and clean exit. **Final `9605dff4` Windows US log (22 Nov)**: virtual DirectDraw, null-cleanup guard actually invoked, MCI AVI ~28s, focus loss/return, first OpenGL/menu/FLAC and clean exit (35.258s). **Final `9605dff4` Windows FR log (22 Nov)**: FR null-cleanup guard actually invoked, MCI AVI ~36s, focus loss/return, first OpenGL/menu/FLAC and clean exit (47.296s) | USA `2b720877`: AVI/focus recovery, GameFrame/menu, CDDA and clean exit after 22.58s. FR `2b720877`: AVI opened, quickly skipped using Space, first GL/menu and FLAC, exit 0 after 6.54s |
| Taskbar icon | FR `9605dff4`: installed from original `game_exe` on original HWND (log evidence; actual taskbar appearance not visually reconfirmed) | USA XFCE: **visually confirmed** original `CYBER.EXE` icon on `2b720877`, source=`game_exe` |
| SkipIntro=1 | FR validated on `cbad024d`; USA current-code retest not independently logged | Current `2b720877` skip-mode retest not independently logged |
| Repeated/no-disc starts and logs | Earlier FR successful repeated launches; latest icon-build native Windows repeat not independently confirmed | FR current build uses an existing no-disc installation; earlier Wine FR confirmed `Log=0` and noncumulative `Log=1` sessions |

These tests **supersede the earlier assertion that the v2-layout game had not been tested on native Windows**. They are real retail-game tests, unlike CI. **Both FR and US original editions have now been exercised under native Windows using the final engine commit `9605dff4` (22 November).** Both runtime logs confirm that the edition-specific early NULL DirectDraw cleanup guards intercepted one genuine uninitialized invocation, followed by sustained MCI AVI playback, OpenGL/menu handoff, FLAC startup and normal exit. The **FR run additionally validates fresh disc installation and 14 FLAC tracks**, while its display mode watcher records the physical desktop remaining at 1920×1200×32@60 Hz before and after, with no observed mode transitions. Both movie runs recorded a Space keydown, so **full natural AVI completion and subjective audio/video quality must not be inferred** from the logs.

## Windows CI

- [x] [Windows x64 launcher / x86 GameFrame and WinMM / tests / package CI #44838205814](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/44838205814) passed for `2b720877`.
- [x] PR #31 merged into `main` at `4ecd56d1`.
- [x] Final `9605dff4` native Windows US runtime smoke log (22 November 2026): COM guard intercepted the uninitialized pointers, AVI ~28s, video focus loss/recovery, 3D first real SwapBuffers, FLAC track 12 started and normal exit 0.
- [x] Final `9605dff4` native Windows **FR** fresh retail CUE/BIN install and runtime (22 November 2026): 343 original files, 14 FLAC, 356 resources verified, FR null-COM guard **actually exercised**, MCI AVI ~36s with focus loss/return, physical 1920×1200 display mode preserved, first real GameFrame/OpenGL SwapBuffers, FLAC Track12 waveOut started, stable ~58.9 FPS and exit code 0 after 47.296s. **Log evidence only; not a visual/audio quality sign-off.**
- [x] Final `9605dff4` [Windows CI #44845426448](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/44845426448) passed before this documentation update.
- [x] [Windows CI #44989204545](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/44989204545) passed after the prior US documentation merge (`4631d6c1`).
- [x] [Windows CI #44989934680](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/44989934680) passed on `94370065` after the FR documentation merge (PR #37).
- [ ] Verify Windows CI green on the exact final `main` SHA selected for manual release after the last documentation/help-text cleanup PR merges.
- [ ] Obtain explicit approval **to publish v2.0.0**, distinct from approving its merges.
- [ ] Manually dispatch [publish-v2.0.0](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/publish-v2.0.0.yml) on `main` with confirmation `publish-v2.0.0`.
- [ ] Confirm public `v2.0.0` tag points to the selected `main` commit, exact six-file ZIP and ZIP SHA-256, correct EXE commit suffix, unchanged v1.0.0/v1.0.1 assets, and perform a final host smoke test.

## Remaining field-test limitations

Audible music quality and visual quality of the exact latest US **and FR** Windows sessions have not been separately attested by the maintainer; FLAC decode and `waveOut` startup are logged. The 22 November `9605dff4` logs show native video focus loss/return but do **not** establish which user action caused it or prove full unskipped AVI completion (Space keydown in both). `SkipIntro=1` on the latest engine, USA fresh-install/repeat-no-disc scenarios on this exact build, extended gameplay endurance, and native Windows FR repeat launches remain unverified in these two sessions. The final **tag-built** executable has not been run on a real host, and other GPUs/OS/Wine versions are not comprehensively covered. Do not imply otherwise in release notes.

The release archive must contain exactly **six flat files**: `CyberGladiatorsFix.exe`, `README.txt`, `LICENSE-GPL-3.0`, `LICENSE-APACHE`, `LICENSE-MIT`, `SHA256SUMS.txt`, plus a separate downloadable ZIP `.sha256`. No original game data or CD tracks are distributed.
