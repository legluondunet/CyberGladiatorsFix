# CyberGladiatorsFix v2.0.0 — release readiness

**Status: source merged and tested; public release not yet published.** The v1.0.0 and v1.0.1 releases remain unchanged. The [manual publication process](RELEASE_PROCESS.md) requires **explicit approval to publish**, separately from approving source changes.

The v2.0.0 layout ([merged PR #27](https://github.com/legluondunet/CyberGladiatorsFix/pull/27)) **does not migrate v1.x installations**. [Merged PR #31](https://github.com/legluondunet/CyberGladiatorsFix/pull/31) fixed early DirectDraw cleanup on French and USA retail executables, and restored the real EXE taskbar icon on Wine/XFCE. Its latest field-tested source was `2b720877`, merged to `main` as `4ecd56d1`.

## Real-host acceptance evidence (20–21 November 2026)

| Test | Native Windows | Wine-staging 11.19 |
| --- | --- | --- |
| USA original disc installation | Fresh US CUE/BIN on `fdca004d`: 341 original game files, 14 FLAC tracks, 354 verified resources | Earlier fresh retail US CUE/BIN installs; latest `2b720877` validated on an existing installation |
| French original disc installation | French v2-layout runtime on `cbad024d`, existing verified original assets | Earlier fresh retail FR CUE/BIN; latest `2b720877` verified an existing no-disc installation (356 resources, 256 SHA-256 checks and 100 IQ presence checks) |
| Original AVI, DirectDraw and menu | FR `cbad024d`: maintainer visually confirmed video and Alt+Tab, clean exit. USA `fdca004d`: visible AVI, null-cleanup guard invoked, successful first GL/menu/FLAC and clean exit | USA `2b720877`: AVI/focus recovery, GameFrame/menu, CDDA and clean exit after 22.58s. FR `2b720877`: AVI opened, quickly skipped using Space, first GL/menu and FLAC, exit 0 after 6.54s |
| Taskbar icon | Needs follow-up on final icon-build | USA XFCE: **visually confirmed** original `CYBER.EXE` icon on `2b720877`, source=`game_exe` |
| SkipIntro=1 | FR validated on `cbad024d`; USA current-code retest not independently logged | Current `2b720877` skip-mode retest not independently logged |
| Repeated/no-disc starts and logs | Earlier FR successful repeated launches; latest icon-build native Windows repeat not independently confirmed | FR current build uses an existing no-disc installation; earlier Wine FR confirmed `Log=0` and noncumulative `Log=1` sessions |

These tests **supersede the earlier assertion that the v2-layout game had not been tested on native Windows**. They are real retail-game tests, unlike CI. However, the native Windows tests were on earlier revisions than the final `2b720877` icon patch. An early Space skip does not prove natural full-length playback.

## Windows CI

- [x] [Windows x64 launcher / x86 GameFrame and WinMM / tests / package CI #44838205814](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/44838205814) passed for `2b720877`.
- [x] PR #31 merged into `main` at `4ecd56d1`.
- [ ] Re-run CI on the final source SHA after any documentation changes.
- [ ] Obtain explicit approval **to publish v2.0.0**, distinct from approving its merges.
- [ ] Manually dispatch [publish-v2.0.0](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/publish-v2.0.0.yml) on `main` with confirmation `publish-v2.0.0`.
- [ ] Confirm public `v2.0.0` tag points to the selected `main` commit, exact six-file ZIP and ZIP SHA-256, correct EXE commit suffix, unchanged v1.0.0/v1.0.1 assets, and perform a final host smoke test.

## Remaining field-test limitations

Audible music quality is not explicitly attested by the user; FLAC decode and `waveOut` startup are logged. Not every Windows FR/USA `SkipIntro`, repeat-launch and Alt+Tab combination was repeated on `2b720877`. The final tag-built executable has not been run on a real host, and other GPUs/OS/Wine versions are not comprehensively covered. Do not imply otherwise in release notes.

The release archive must contain exactly **six flat files**: `CyberGladiatorsFix.exe`, `README.txt`, `LICENSE-GPL-3.0`, `LICENSE-APACHE`, `LICENSE-MIT`, `SHA256SUMS.txt`, plus a separate downloadable ZIP `.sha256`. No original game data or CD tracks are distributed.
