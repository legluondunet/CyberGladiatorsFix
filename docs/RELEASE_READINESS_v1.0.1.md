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

- [Windows CI for the v1.0.1 candidate](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/43476299506):
  **success** for commit `691f03b3`; 97 launcher and 54 WinMM/CDDA
  Rust tests passed, along with GameFrame/WinMM build, DLL exports,
  native loader and packaging checks. This is the exact candidate
  exercised in the latest US/Wine gameplay session. Rerun CI on final
  `main` after merging the release preparation PR.
- **US / Wine 11.17** real retail clean install (13 November 2026),
  build `1.0.0-23404f9f`: 342 files extracted, 14 FLAC tracks,
  255 immutable resource SHA-256 matches, 100 IQ resources present,
  virtual 640×480×16 DirectDraw, native 4:3 AVI/keyboard skip,
  first real OpenGL present, menu signature, audio, ~59.5 FPS
  and normal exit (44.2-second process, exit 0).
  Both separate session logs shared the same ID.
- **US / Wine 11.17 candidate test (13 November 2026)**, build
  `1.0.1-691f03b3`: existing installed USA edition recognized;
  255 immutable files SHA-256 validated, 100 writable IQ files checked
  for presence, patched executable and original backup verified;
  installed GameFrame and CDDA DLL digests matched embedded versions,
  without backup/replacement. The native introduction opened and was
  skipped with Enter; the transition to the first real OpenGL frame,
  1440x1080 centered 4:3 image on a 1920x1080 desktop and menu
  signature show/hide succeeded. Track 12 was decoded and waveOut
  started, and the game exited with code 0 after 6.183 seconds.
  The two separate logs carried the same session ID. This **short**
  run does **not** validate an uninterrupted intro, extended gameplay,
  steady 60 FPS, Alt+Tab, separate `--verify` invocation, `--install-only`
  or `--launch-no-verify`; it is not a fresh v1.0.1 install.
- Earlier v1.0.0 acceptance documented US/FR on native Windows 11 and
  Wine; those are earlier binaries and **not** field validation of
  the final v1.0.1 build.

## Required final acceptance matrix

| Test | USA / Wine | FR / Wine | USA / Windows 11 | FR / Windows 11 |
| --- | --- | --- | --- | --- |
| Fresh installation from authenticated original CUE/BIN | Earlier build passed | Pending on v1.0.1 | Pending on v1.0.1 | Pending on v1.0.1 |
| Second default launch and strict `--verify` | **Normal launch passed; inline integrity pre/post-check passed; separate `--verify` pending** | Pending | Pending | Pending |
| `--install-only` without game process | Pending | Pending | Pending | Pending |
| `--launch-no-verify` and two logs | Pending | Pending | Pending | Pending |
| Original AVI skip/full intro, 4:3, GameFrame menu and 60 FPS | **Intro skip, 4:3 and menu passed; complete intro and sustained 60 FPS pending** | Pending on v1.0.1 | Pending | Pending |
| FLAC, focus/Alt+Tab, exit code 0, glide DLL backup | **CDDA decoding/waveOut and exit 0 passed; Alt+Tab and backup replacement pending** | Pending on v1.0.1 | Pending | Pending |

**Read-only CLI checks** (can run on an otherwise empty folder):
`--help`, `-h`, `--version`, `--inspect image.iso`,
`--inspect image.cue [track.bin|track.img]`, and
`--verbose --inspect image.cue`. Verify `--inspect` doesn't create
an installation or alter the image. Unknown CLI switches and
old `--inspect-disc` variants must fail.

**Log checks:** `CyberGladiatorsFix_installer.log` and
`CyberGladiatorsFix.log` sit beside the launcher and share a session ID
for the same game run (**confirmed on US/Wine candidate**).
The separate-log preservation policy is also covered by automated
tests. A real standalone `--verify` test demonstrating that the prior
runtime log is not reset remains to be recorded.

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
