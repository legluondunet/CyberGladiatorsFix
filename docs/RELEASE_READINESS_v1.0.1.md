# CyberGladiatorsFix v1.0.1 — release readiness

Status: **release preparation only**, not publication authorization.

## Scope and source

- New version: **1.0.1**. Public **v1.0.0** stays unchanged.
- Code baseline: merged PRs [#24](https://github.com/legluondunet/CyberGladiatorsFix/pull/24)
  and [#25](https://github.com/legluondunet/CyberGladiatorsFix/pull/25),
  `main` at `66090580` before this version/notes PR.
- Source changes: split logs, `--launch-no-verify`, read-only
  `--inspect`, `--verbose`, `-h`, retired user CLI switches and
  obsolete renderer experiments/installer fallbacks.
- Supported install media: authenticated original US/FR CUE/BIN only.
  Standalone ISO may be **inspected**, not **installed**.

## Evidence already available

- [Windows CI on merged cleanup](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/43472541529):
  **success** for commit `66090580`. Windows/MSVC build and existing
  automated tests validate the old version string `1.0.0`; rerun
  CI on this version-bump PR and on final `main` for v1.0.1.
- **US / Wine 11.17** real retail clean install (13 November 2026),
  build `1.0.0-23404f9f`: 342 files extracted, 14 FLAC tracks,
  255 immutable resource SHA-256 matches, 100 IQ resources present,
  virtual 640×480×16 DirectDraw, native 4:3 AVI/keyboard skip,
  first real OpenGL present, menu signature, audio, ~59.5 FPS
  and normal exit (44.2-second process, exit 0).
  Both separate session logs shared the same ID.
- Earlier v1.0.0 acceptance documented US/FR on native Windows 11 and
  Wine; those are earlier binaries and **not** field validation of
  the final v1.0.1 build.

## Required final acceptance matrix

| Test | USA / Wine | FR / Wine | USA / Windows 11 | FR / Windows 11 |
| --- | --- | --- | --- | --- |
| Fresh installation from authenticated original CUE/BIN | Earlier build passed | Pending on v1.0.1 | Pending on v1.0.1 | Pending on v1.0.1 |
| Second default launch and strict `--verify` | Pending on v1.0.1 | Pending | Pending | Pending |
| `--install-only` without game process | Pending | Pending | Pending | Pending |
| `--launch-no-verify` and two logs | Pending | Pending | Pending | Pending |
| Original AVI skip/full intro, 4:3, GameFrame menu and 60 FPS | Earlier build partly passed | Pending on v1.0.1 | Pending | Pending |
| FLAC, focus/Alt+Tab, exit code 0, glide DLL backup | Earlier build partly passed | Pending on v1.0.1 | Pending | Pending |

**Read-only CLI checks** (can run on an otherwise empty folder):
`--help`, `-h`, `--version`, `--inspect image.iso`,
`--inspect image.cue [track.bin|track.img]`, and
`--verbose --inspect image.cue`. Verify `--inspect` doesn't create
an installation or alter the image. Unknown CLI switches and
old `--inspect-disc` variants must fail.

**Log checks:** `CyberGladiatorsFix_installer.log` and
`CyberGladiatorsFix.log` sit beside the launcher, share a session ID
for the same game run, and only the corresponding session resets its log.
`--verify` must leave the prior game log untouched.

## Release and package checks

- Final v1.0.1 branch and merged `main`: green
  `gameframe-glide.yml`, correct `--version` commit suffix,
  six-file artifact with five original file hashes, no unpacked DLL
  or game asset in the ZIP.
- Dedicated publisher: `.github/workflows/publish-v1.0.1.yml`
  must accept only **main** and exact `publish-v1.0.1` confirmation.
  It must refuse a mismatched source SHA/version or a pre-existing
  v1.0.1 release, without changing v1.0.0.
- Publish only with explicit maintainer approval after field tests.
  See [RELEASE_PROCESS.md](RELEASE_PROCESS.md) and
  [RELEASE_NOTES_v1.0.1.md](RELEASE_NOTES_v1.0.1.md).

A passing CI run does **not** replace Windows/Wine gameplay smoke tests
with the final tagged executable.
