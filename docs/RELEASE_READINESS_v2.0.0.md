# CyberGladiatorsFix v2.0.0 — release readiness

**Status: preparation only. No publication or tag is authorized.**

The version jump to **2.0.0** reflects a breaking directory layout introduced by merged [PR #27](https://github.com/legluondunet/CyberGladiatorsFix/pull/27) (main `189e98c1`). The installer intentionally **does not migrate old v1.0.1 installations**. Public v1.0.0/v1.0.1 releases remain immutable.

## Acceptance evidence, new layout and v2.0.0 candidate

| Scenario | FR / Wine-staging 11.19 | USA / Wine-staging 11.19 | Native Windows |
| --- | --- | --- | --- |
| Authentic CUE/BIN fresh install and layout | Validated: 343 files | Validated: 342 files, including v2.0.0 candidate | **FR validated** on `2.0.0-2ae6cadb` (343 files); USA not tested |
| 14 FLAC, 4:3 AVI, GameFrame and normal exit | Validated | Validated on `2.0.0-0f86b592` (53.6 s, exit 0) | **FR runtime validated** on `2.0.0-2ae6cadb` (30.8 s, exit 0); visual/audio subjective acceptance not independently recorded; USA not tested |
| Second launch without source disc, Log=0 | Validated | Not tested | Not tested |
| Two noncumulative Log=1 sessions | Validated | Not tested | Not tested |

**Native Windows FR (20 November 2026)**: original `CYBERG.CUE` detected, fresh installation of 343 game files and 14 FLAC tracks into the v2 layout, 356 manifest resources verified, original EXE backup retained and hash-checked, 4:3 native MCI video followed by GameFrame OpenGL, CDDA decoded and started waveOut, focus loss/return during AVI, exit 0 after 30.8 seconds. The current physical display mode was 1920×1200@60 Hz at start and end, with zero observed mode transitions; logical window metrics were 1536×960 (consistent with 125% display scaling). The initial logs were generated with `Log=0`. The logs alone do not establish subjective sound/video quality, a second no-CD launch or a completed native Windows USA test.

Earlier FR and USA field tests used pre-release `1.0.1-53beda77` with the new architecture. **On 20 November 2026 the version-bumped `2.0.0-0f86b592` candidate passed a fresh USA retail CUE/BIN installation** with 342 original files, 14 FLAC tracks, 4:3 AVI, focus loss/return during video, GameFrame, CDDA and normal exit after 53.6 seconds (code 0); both initial logs were generated with `Log=0`. This is **not a test of the final tagged/published v2.0.0 release**. [Windows CI after PR #27 merge](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/44752207790) and [Windows CI on v2.0.0 candidate](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/44754612481) passed, but CI does not play the original retail game.

## Gates before public release

- [x] v2.0.0 Cargo version, README, notes and guarded manual publisher prepared and reviewed on PR #28
- [x] Green version-bumped PR CI on `0f86b592` (x64 launcher, x86 DLLs, Rust and packaging checks); recheck CI after these documentation updates
- [x] Preparation PR #28 merged and [Windows CI passed on final `main` commit `2ae6cadb`](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/44760300708)
- [ ] Obtain explicit approval to **publish v2.0.0**, separate from merge
- [ ] Manually dispatch `publish-v2.0.0` on `main`; validate ZIP, SHA256, tag and release
- [ ] Check v1.0.0 and v1.0.1 remain published and unchanged

This checklist distinguishes the now-verified native Windows **FR first-install/runtime smoke test** from the still-unverified native Windows **USA**, Windows **second launch** and USA Wine **repeat launch** scenarios. If the maintainer publishes despite these gaps, disclose them.

The ZIP must have six flat files: `CyberGladiatorsFix.exe`, `README.txt`, `LICENSE-GPL-3.0`, `LICENSE-APACHE`, `LICENSE-MIT`, `SHA256SUMS.txt`. No original game files.
