# CyberGladiatorsFix v2.0.1 — guarded manual release procedure

**Do not publish automatically on push, merge or tag.** The existing public **v1.0.0, v1.0.1 and v2.0.0** tags/releases/assets are historical and must remain intact. A green source PR does not authorize publication.

## Preconditions

1. Review the [focused v2.0.1 code audit](CODE_AUDIT_v2.0.1.md), [v2.0.1 notes](RELEASE_NOTES_v2.0.1.md) and [v2.0.1 readiness](RELEASE_READINESS_v2.0.1.md). Confirm the scope: targeted GE-Proton AVI-host ownership, no change to native Windows/other Wine rendering, no copyrighted game data bundled.
2. Merge the reviewed v2.0.1 preparation PR **only after its Windows CI succeeds**. Check a new CI success on the exact merged `main` commit, with x64 launcher, x86 WinMM and source-pinned GameFrame, unit tests, PE/export checks, CLI smoke checks, package hashes and six-file ZIP candidate.
3. Check `Cargo.toml` says **`2.0.1`**; confirm published v2.0.0 is still present and **no v2.0.1 GitHub Release exists**. Do not reuse or overwrite a prior tag.
4. Obtain **explicit maintainer approval to publish v2.0.1**, separately from approval to merge changes.

## Publication (after explicit approval only)

1. Open [Publish CyberGladiatorsFix v2.0.1](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/publish-v2.0.1.yml).
2. Select **`main`** and type the exact confirmation **`publish-v2.0.1`**.
3. The manual workflow rebuilds and tests the selected immutable `main` SHA using the same Windows validation workflow. It refuses a changed `main`, wrong Cargo version, missing v2.0.0 predecessor or existing v2.0.1 public release.
4. The publisher checks the executable version `CyberGladiatorsFix 2.0.1-<first-8-source-sha>`, **exactly six flat files** (`CyberGladiatorsFix.exe`, `README.txt`, `LICENSE-GPL-3.0`, `LICENSE-APACHE`, `LICENSE-MIT`, `SHA256SUMS.txt`), **five** individual SHA-256 hashes, ZIP extraction roundtrip and separate ZIP SHA-256. Only after those validations does it create the `v2.0.1` tag and GitHub Release.
5. On [Releases](https://github.com/legluondunet/CyberGladiatorsFix/releases), verify new tag source SHA, public ZIP and its `.sha256`, `README.txt`, version suffix, accurate release notes and unchanged previous releases. A CI artifact is not itself a published release; builds from different runs need not be bit-identical.
6. Perform a real-host test of the **actual v2.0.1 published ZIP**. At minimum, confirm installation/upgrade launch, original AVI or skip, first menu/GameFrame, CDDA, focus/Alt+Tab and clean exit. CI cannot simulate proprietary retail gameplay.

## Rollback / blockers

If any guard or post-upload verification fails, **do not overwrite an existing release or delete an old stable tag to retry**. Inspect the failed run first. Do not silently reissue the same version after partially publishing it; require an explicit release-recovery decision. If the updated game binary does not run on the user's machine, keep the published v2.0.0 ZIP available as the prior stable build.

The old `publish-v2.0.0.yml` and its historical [notes](RELEASE_NOTES_v2.0.0.md) are retained for auditing but are **not** the procedure for future updates.
