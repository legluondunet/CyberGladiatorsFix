# First public release: one-click GitHub Actions publication — v1.0.0

CyberGladiatorsFix follows the same **manually approved, automated publication** model as ScorchedPlanetFix. Merging a PR or pushing a tag **never publishes a GitHub Release**. The release manager explicitly initiates the dedicated publisher workflow after source/field acceptance.

## 1. Preconditions

- The final reviewed code is merged into `main`. The original retail images supported by this version remain **US and FR CUE/BIN**, not unverified DE/other editions.
- Confirm `Cargo.toml` has `version = "1.0.0"`; no known release-blocking issues remain.
- Prior authenticated retail field tests on Windows 11 and Wine-staging 11.17 validate installation, the original movie (including skip), full-screen 4:3, Alt+Tab, FLAC music, 60 FPS, menu signature and normal exit.
- The US/Wine test on the merged `bc55394e` build verified `--verify` and a normal game exit with **100 IQ files checked for safe presence** and **255 remaining resources strictly hashed**; original IQ data may be rewritten by the retail game.
- The release manager has explicitly approved public publication.

## 2. Start publication entirely in the GitHub web interface

1. Open [**Publish CyberGladiatorsFix v1.0.0**](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/publish-v1.0.0.yml).
2. Click **Run workflow**, select **main**.
3. Enter **`publish-v1.0.0`** in the `confirm` field, then click **Run workflow**.

**This is the only manual publishing action.** There is no need to create a tag, draft Release, upload a ZIP, copy SHA-256 hashes or run `git` locally. Do not run this workflow until the release is ready to go public.

The publisher refuses any branch other than `main`, a wrong confirmation phrase, a mismatched `Cargo.toml` version, an existing v1.0.0 release, an unexpected SHA or a branch that advances during publication. It uses `contents: write` for the designated release workflow only.

## 3. GitHub Actions performs all release checks

1. Invoke the same [Windows GameFrame build/CI workflow](../.github/workflows/gameframe-glide.yml) as a **reusable workflow**, building the verified source commit fresh in this release run. This compiles and tests the x86 open-source Glide DLL, bundled x86 WinMM, the Windows x64 launcher, native loader, CLI/no-CD checks and package structure.
2. Download the successful build's **six-file** artifact within the same workflow run.
3. Confirm the exact six allowed files: `CyberGladiatorsFix.exe`, `LICENSE-APACHE`, `LICENSE-GPL-3.0`, `LICENSE-MIT`, `README.txt` and `SHA256SUMS.txt`. All five listed SHA-256 entries must be correct, with no nested ZIP, sidecar DLL or copyrighted game content.
4. Ensure `CyberGladiatorsFix.exe --version` matches `1.0.0-<8-char source SHA>`.
5. Create `CyberGladiatorsFix-v1.0.0-Windows-x64.zip`, unpack and independently check all six files byte-for-byte. Generate `CyberGladiatorsFix-v1.0.0-Windows-x64.zip.sha256`.
6. Check that `main` still points to the build commit, create or verify the `v1.0.0` tag at that **exact same commit**, and publish the stable GitHub Release using [reviewed notes](RELEASE_NOTES_v1.0.0.md), the checked ZIP and ZIP checksum. The workflow then checks the Release's tag, public status and attachments.

The separate normal CI also runs automatically on the newly created `v1.0.0` tag; it checks that the tag version equals `Cargo.toml`. The manual publisher's fresh, fully tested build is the authoritative uploaded release artifact. The tag-triggered CI is an additional independent build, not a prerequisite for uploading or substituting the already verified artifact. The Rust build is **not guaranteed bit-for-bit reproducible** across independent runners; do not swap the release ZIP with an unrelated rebuild.

## 4. Final checks

Open [Releases](https://github.com/legluondunet/CyberGladiatorsFix/releases), verify the v1.0.0 ZIP and its `.sha256` are attached and downloadable, and ensure the tag matches the source SHA reported by `--version`.

As a final host acceptance check, download the *actual release ZIP*, run `--verify`, launch twice and check the original intro, menu signature, audio and graceful exit. Check at least one native Windows 11 host if possible. The GitHub Actions runner does **not** possess an authenticated retail CD and cannot reproduce real gameplay, so field acceptance remains necessary. Any post-publication regression should receive a focused follow-up issue/patch release rather than silent modification of the published assets.

Maintainer: **legluondunet assisted by ChatGPT (OpenAI)**.
