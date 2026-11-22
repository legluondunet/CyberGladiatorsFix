# CyberGladiatorsFix v2.0.0 — release readiness

**Status: source merged and tested; public release not yet published.** The v1.0.0 and v1.0.1 releases remain unchanged. The [manual publication process](RELEASE_PROCESS.md) requires **explicit approval to publish**, separately from approving source changes.

The v2.0.0 layout ([merged PR #27](https://github.com/legluondunet/CyberGladiatorsFix/pull/27)) **does not migrate v1.x installations**. [Merged PR #31](https://github.com/legluondunet/CyberGladiatorsFix/pull/31) fixed early DirectDraw cleanup on French and USA retail executables, and restored the real EXE taskbar icon on Wine/XFCE. Its latest field-tested source was `2b720877`, merged to `main` as `4ecd56d1`.

## Real-host acceptance evidence (20–22 November 2026)

| Test | Native Windows | Wine-staging 11.19 |
| --- | --- | --- |
| USA original disc installation | Fresh US CUE/BIN on `fdca004d`: 341 original game files, 14 FLAC tracks, 354 verified resources | Earlier fresh retail US CUE/BIN installs; latest `2b720877` validated on an existing installation |
| French original disc installation | French v2-layout runtime on `cbad024d`, existing verified original assets | Earlier fresh retail FR CUE/BIN; latest `2b720877` verified an existing no-disc installation (356 resources, 256 SHA-256 checks and 100 IQ presence checks) |
| Original AVI, DirectDraw and menu | FR `cbad024d`: maintainer visually confirmed video and Alt+Tab, clean exit. USA `fdca004d`: visible AVI, null-cleanup guard invoked, successful first GL/menu/FLAC and clean exit. **Final `9605dff4` Windows US log (22 Nov)**: virtual DirectDraw, null-cleanup guard actually invoked, MCI AVI ~28s, focus loss/return, first OpenGL/menu/FLAC and clean exit (35.258s) | USA `2b720877`: AVI/focus recovery, GameFrame/menu, CDDA and clean exit after 22.58s. FR `2b720877`: AVI opened, quickly skipped using Space, first GL/menu and FLAC, exit 0 after 6.54s |
| Taskbar icon | Needs follow-up on final icon-build | USA XFCE: **visually confirmed** original `CYBER.EXE` icon on `2b720877`, source=`game_exe` |
| SkipIntro=1 | FR validated on `cbad024d`; USA current-code retest not independently logged | Current `2b720877` skip-mode retest not independently logged |
| Repeated/no-disc starts and logs | Earlier FR successful repeated launches; latest icon-build native Windows repeat not independently confirmed | FR current build uses an existing no-disc installation; earlier Wine FR confirmed `Log=0` and noncumulative `Log=1` sessions |

These tests **supersede the earlier assertion that the v2-layout game had not been tested on native Windows**. They are real retail-game tests, unlike CI. The **US runtime on the final source commit `9605dff4`** was now tested under native Windows on 22 November, independently confirming the key startup guard, sustained MCI playback and a clean OpenGL handoff; **FR native Windows validation still predates the final icon patch**. The log reports a Space keydown and 28s of MCI activity, so do not claim the AVI played to its natural end or that picture/sound quality was visually/audibly approved by the maintainer.

## Windows CI

- [x] [Windows x64 launcher / x86 GameFrame and WinMM / tests / package CI #44838205814](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/44838205814) passed for `2b720877`.
- [x] PR #31 merged into `main` at `4ecd56d1`.
- [x] Final `9605dff4` native Windows US runtime smoke log (22 November 2026): COM guard intercepted the uninitialized pointers, AVI 28s, video focus loss/recovery, 3D first real SwapBuffers, FLAC track 12 started and normal exit 0. **Log evidence only; not a visual/audio quality sign-off.**
- [x] Final `9605dff4` [Windows CI #44845426448](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/44845426448) passed before this documentation update.
- [ ] Re-run CI on the final source SHA after any documentation changes.
- [ ] Obtain explicit approval **to publish v2.0.0**, distinct from approving its merges.
- [ ] Manually dispatch [publish-v2.0.0](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/publish-v2.0.0.yml) on `main` with confirmation `publish-v2.0.0`.
- [ ] Confirm public `v2.0.0` tag points to the selected `main` commit, exact six-file ZIP and ZIP SHA-256, correct EXE commit suffix, unchanged v1.0.0/v1.0.1 assets, and perform a final host smoke test.

## Remaining field-test limitations

Audible music quality and visual quality of this exact Windows US session have not been explicitly attested by the user; FLAC decode and `waveOut` startup are logged. The 22 November US `9605dff4` log shows native video focus loss/return but does **not** establish which user action caused it, prove a full unskipped AVI, or establish `SkipIntro=1` or an install/second launch. Native Windows **FR** is still awaiting the final `9605dff4` field regression. The final **tag-built** executable has not been run on a real host, and other GPUs/OS/Wine versions are not comprehensively covered. Do not imply otherwise in release notes.

The release archive must contain exactly **six flat files**: `CyberGladiatorsFix.exe`, `README.txt`, `LICENSE-GPL-3.0`, `LICENSE-APACHE`, `LICENSE-MIT`, `SHA256SUMS.txt`, plus a separate downloadable ZIP `.sha256`. No original game data or CD tracks are distributed.
