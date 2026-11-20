# CyberGladiatorsFix v2.0.0 — release readiness

**Status: preparation only. No publication or tag is authorized.**

The version jump to **2.0.0** reflects a breaking directory layout introduced by merged [PR #27](https://github.com/legluondunet/CyberGladiatorsFix/pull/27) (main `189e98c1`). The installer intentionally **does not migrate old v1.0.1 installations**. Public v1.0.0/v1.0.1 releases remain immutable.

## Acceptance evidence, new layout, before version bump

| Scenario | FR / Wine-staging 11.19 | USA / Wine-staging 11.19 | Native Windows |
| --- | --- | --- | --- |
| Authentic CUE/BIN fresh install and layout | Validated: 343 files | Validated: 342 files | Not tested |
| 14 FLAC, 4:3 AVI, GameFrame and normal exit | Validated | Validated | Not tested |
| Second launch without source disc, Log=0 | Validated | Not tested | Not tested |
| Two noncumulative Log=1 sessions | Validated | Not tested | Not tested |

The user closed the test campaign. These sessions used pre-release `1.0.1-53beda77` with the new architecture; **they were not tests of a tagged v2.0.0 EXE**. The [Windows CI after PR #27 merge](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/44752207790) passed, but CI does not play the original retail game.

## Gates before public release

- [ ] Review v2.0.0 Cargo version, README, notes and guarded publisher
- [ ] Green version-bumped PR CI (x64 launcher, x86 DLLs, Rust and packaging checks)
- [ ] Merge prep PR and verify CI for final `main`
- [ ] Obtain explicit approval to **publish v2.0.0**, separate from merge
- [ ] Manually dispatch `publish-v2.0.0` on `main`; validate ZIP, SHA256, tag and release
- [ ] Check v1.0.0 and v1.0.1 remain published and unchanged

This checklist never silently marks the missing Windows v2-layout game tests or USA repeat launch as completed. If the maintainer publishes despite these gaps, disclose them.

The ZIP must have six flat files: `CyberGladiatorsFix.exe`, `README.txt`, `LICENSE-GPL-3.0`, `LICENSE-APACHE`, `LICENSE-MIT`, `SHA256SUMS.txt`. No original game files.
