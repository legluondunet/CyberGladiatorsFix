# CyberGladiatorsFix v1.0.1 — guarded maintenance release process

The previously published **v1.0.0** GitHub Release and its assets are immutable
historical artifacts. This procedure publishes **v1.0.1** separately, without
changing the earlier release. Merging or pushing code **never** publishes
automatically.

## 1. Approval and pre-publication gates

1. Merge the v1.0.1 preparation PR into `main` after a reviewed, green
   Windows CI: pinned GameFrame x86, embedded WinMM x86, launcher x64,
   unit tests, 32-bit DLL exports/loader and one-file packaging.
2. Confirm `Cargo.toml` contains `version = "1.0.1"`, that `main`
   has not advanced unexpectedly, that tag/release `v1.0.1` do not exist,
   and that v1.0.0 remains available.
3. Test original **US and FR** CUE/BIN media on native Windows 11 and Wine
   as possible. Check fresh install, repeated normal launch, `--install-only`,
   `--verify`, `--launch-no-verify`, video skip, 4:3 image/menus,
   signature, Alt+Tab, FLAC audio, normal exit, DLL backup retention and
   two correlated logs. `--inspect` must be read-only; `--verbose`
   must combine with commands.
4. Record any unavailable field-test combinations explicitly; a green CI
   run alone does not establish real-game compatibility.
5. Obtain explicit maintainer approval to publish this version. Do not
   trigger the release workflow in response to merely approving the PR.

## 2. Manually start the approved publisher

Open [Publish CyberGladiatorsFix v1.0.1](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/publish-v1.0.1.yml),
choose **Run workflow** on **main**, and enter exactly
`publish-v1.0.1` in the confirmation field.

This is the only publishing action needed: GitHub Actions builds, checks,
tags and uploads the resulting package. Never run the workflow before the
real-game acceptance decision.

## 3. Automatic publishing safeguards

The guarded `publish-v1.0.1.yml` workflow:

1. Runs only for the repository `legluondunet/CyberGladiatorsFix`,
   **main**, and the exact confirmation string. It reuses
   `gameframe-glide.yml` for a fresh Windows x64/x86 build and tests of
   the dispatch source commit.
2. Requires an unchanged checkout/source SHA and `main` commit, Cargo
   version 1.0.1, absent v1.0.1 release, and an existing v1.0.0 release.
   The previous release is never overwritten.
3. Downloads the freshly built artifact from this workflow execution.
   It must contain **exactly six flat files**: `CyberGladiatorsFix.exe`,
   `README.txt`, `LICENSE-GPL-3.0`, `LICENSE-APACHE`,
   `LICENSE-MIT` and `SHA256SUMS.txt`.
4. Checks five SHA-256 entries for the other files, verifies that
   `CyberGladiatorsFix.exe --version` prints
   `CyberGladiatorsFix 1.0.1-<first-eight-source-SHA>`, packages
   `CyberGladiatorsFix-v1.0.1-Windows-x64.zip`, extracts it for
   byte-for-byte validation and writes
   `CyberGladiatorsFix-v1.0.1-Windows-x64.zip.sha256`.
5. Appends final ZIP/EXE digests to the reviewed
   `docs/RELEASE_NOTES_v1.0.1.md` content. Rechecks `main`,
   creates an immutable `v1.0.1` tag at the exact verified commit
   (or rejects a different pre-existing tag), and publishes a new
   GitHub Release with ZIP and SHA-256. The workflow then inspects
   release status and both assets.

The normal tag-triggered CI run is an additional verification, **not**
the artifact that should replace the publisher's validated package.
Independent Windows hosted builds are not guaranteed bit-for-bit identical.

## 4. Post-publication acceptance

Open [CyberGladiatorsFix Releases](https://github.com/legluondunet/CyberGladiatorsFix/releases).
Verify `v1.0.1` is public/latest, both attached files download, the
separate ZIP checksum matches, and `--version` identifies the tag
commit. Confirm the original v1.0.0 ZIP and checksum are still accessible.

Finally test the **published ZIP**, not just an earlier CI artifact, on
actual game media. If a regression appears, raise a separate issue/fix
release rather than silently modifying the immutable published files.
