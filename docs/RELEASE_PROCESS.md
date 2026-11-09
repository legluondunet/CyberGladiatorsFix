# First public release procedure — v1.0.0

**Maintainer checklist: not an automated publishing workflow.** Source is validated through PRs and GitHub Actions; a **GitHub Release must not be published automatically** by merging a PR or creating a tag. Release artifacts are built/tested from the exact intended source revision. The project currently has **no published Releases**.

## 1. Source acceptance and scope

- Merge the final audit-only PR after its green Windows CI run; confirm `main` points to the expected merge SHA.
- Confirm no runtime feature changes are pending and the supported retail set remains original **US and FR CUE/BIN** only.
- Confirm `Cargo.toml` reports `version = "1.0.0"`. The public tag must be precisely `v1.0.0`; do not publish a release under a tag that disagrees with the built launcher.
- Confirm field evidence for Windows 11 and Wine-staging 11.17. The upper-right signature is accepted on FR/native Windows and both editions in earlier Wine testing. **PR #11** additionally validated the `.bak` name collision and repeated same-build `glide.dll` reuse on FR/Windows. **PR #12** additionally received a complete US/Wine-staging 11.17 field run on its source commit (`731aa136`), confirming the mounted-CD local AVI rewrite, Alt+Tab, 4:3 menu, FLAC and clean exit, with user visual acceptance. Still check the **exact tagged package** after final audit (menu, 4:3, audio, exit); this prior PR artifact is not the tag-built binary.
- No tags, no GitHub Release, no GitHub Pages or distribution uploads until explicit maintainer approval.

## 2. Build provenance and tests

- Create `v1.0.0` on the **final reviewed `main` commit**, not on a feature branch or an ephemeral pull-request merge commit. Prefer an annotated tag with the source SHA, date and author. Do this only when the release is approved.
- The `.github/workflows/gameframe-glide.yml` workflow runs on `main`, PRs and version tags. The tag path verifies that `vX.Y.Z` equals the project's `Cargo.toml` version; tag activity **only builds and uploads a CI artifact**, never publishes a GitHub Release.
- Confirm successful Windows x86 Glide compilation and exported symbols; matching embedded x86 WinMM bridge; Windows x64 tests, embedded DLL SHA checks, strict no-game fail-closed smoke, PE32 loader smoke and package integrity check. GitHub Actions **cannot** run an authenticated original game.
- The launcher `--version` output must end in `-<8-char source SHA>`, matching the final tagged source commit.
- The binary is **not promised bit-for-bit reproducible**: Cargo lockfiles and complete Rust toolchain/runner pinning are future hardening. Recording the actual source commit, GameFrame upstream pin and per-asset SHA256SUMS is nevertheless mandatory.

## 3. Verify the distribution artifact

Expected files, **exactly six**:

~~~text
CyberGladiatorsFix.exe
LICENSE-APACHE
LICENSE-GPL-3.0
LICENSE-MIT
README.txt
SHA256SUMS.txt
~~~

- **One EXE** embeds both verified x86 DLLs. There must not be a separate `glide.dll`, `CyberGladiatorsFix.dll`, nGlide, DXVK, game `CYBER.EXE`, `.cue`, `.bin`, `.flac` or proprietary bitmap.
- `SHA256SUMS.txt` must contain exactly five valid SHA-256 entries, one per file except itself; re-hash the package **after download** and compare.
- Confirm `README.txt` documents automatic US/FR CUE/BIN install, supported runtime, command-line checks and limitations.
- GitHub Actions downloads expire (currently **14 days**). To distribute publicly, upload the reviewed files as a **durable GitHub Release asset** (normally a ZIP of the exact six files), and verify the Release's download link.

On Linux with the six files extracted, verify hashes without altering any file:

~~~bash
sha256sum -c SHA256SUMS.txt
~~~

On Windows PowerShell, compare `(Get-FileHash .\CyberGladiatorsFix.exe -Algorithm SHA256).Hash.ToLowerInvariant()` with the executable row in `SHA256SUMS.txt` (and repeat for the other four rows).

## 4. Minimal final game smoke test

Use the exact distribution build, not an older PR artifact:

1. Launch the verified original US or FR game via the one EXE, preferably once from a fresh writable folder with CUE/BIN and once after removing the source images.
2. Confirm original MCI intro (natural finish or space skip), desktop display mode unchanged, 4:3 game output, input and Alt+Tab.
3. Verify the unobtrusive `CyberGladiatorsFix v1.0.0-XXXXXXXX` signature is aligned to the **upper-right original 4:3 border** and is hidden during combat; check at least one native Windows 11 host and review the previously accepted US/FR Wine runs.
4. Confirm mandatory FLAC CD audio, ~60 FPS after stabilization, clean process exit, and **non-overwriting `.bak` archive** of any mismatched old `glide.dll`, with bundled verified GameFrame **retained** after exit. Re-launch the same EXE and confirm `action=keep_existing` without a new backup.
5. Check `CyberGladiatorsFix.log` for `GAMEFRAME_MENU_SIGNATURE placement=upper_right_edge_4px_transparent`, `CYBER_MENU_SIGNATURE_PROBE` visibility transition, `CYBER_MENU_SIGNATURE_STYLE`, `GAMEFRAME_GLIDE_POLICY action=keep_existing` (on identical second run), `GAME_EXIT exit_status=exit code: 0` and `SESSION END success`. The log proves instrumentation, not visual fidelity: visual acceptance remains a human check.
6. Run strict `--verify`. Confirm generated `CyberGladiatorsFix.ini` contains only `[Video] SkipIntro` and the log reports required FLAC and strict IQ integrity. Fail closed on unknown game assets. If problems emerge, **do not publish**; open a PR with the fix and repeat this checklist.

## 5. Publish only after sign-off

- Create a **draft** GitHub Release for `v1.0.0`, using reviewed [draft release notes](RELEASE_NOTES_v1.0.0.md).
- Upload exactly the tag-built artifact ZIP (or six files) along with the checksums; document the source SHA, successful Actions run URL and tests performed. Review all links and license notices.
- Mark it as a prerelease if broader host testing or known limitations warrant that designation. Decide this explicitly when publishing; the first release need not be called universal/stable.
- Publish only on maintainer's explicit **release approval**.
- After publishing, verify the public asset can be downloaded and that the published SHA256SUMS still matches; retain the tag and the release notes.

Maintainer signature: **legluondunet assisted by ChatGPT (OpenAI)**.
