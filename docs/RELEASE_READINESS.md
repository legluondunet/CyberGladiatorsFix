# CyberGladiatorsFix — pre-release code audit

> **Status update — 2026-11-08 (PR #6, unmerged).** This document preserves the historical 2026-11-06 baseline audit below. Since then, user-supplied field logs and CI have validated the *opt-in shared owned-COM DirectDraw and MCI video path* for **FR/Windows, US/Windows and US/Wine 11.17**. Native FR completed its ~40-second AVI and Alt+Tab deduplication; US/Windows and US/Wine completed shorter ~14s and ~12s video/3D smoke tests with repeated Alt+Tab, FLAC and clean shutdown. The US `CYBER.cue`/`CYBER.bin` with an otherwise undeclared **150-sector zero audio pregap** is accepted without modifying original media; this has been field-validated, while exact retail SHA checks remain. Last CI for this correction: [#42496409905](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/42496409905), successful. **Default-mode change pending validation:** PR #6 now automatically activates the same owned-COM/MCI engine for both verified editions on Windows and Wine/Proton. The earlier successful Windows FR, Windows US and Wine US sessions used explicit test flags; the automatic default and newly enabled **FR/Wine** path require new real-machine acceptance tests before merging. No platform-specific launcher command is needed. Second-launch/`--verify` coverage remains useful; public release and merge are separate decisions. The older statements below that Windows gameplay or first-use CD extraction have never been tested are historical, not current status.


Audit date: **2026-11-06**. Baseline: `main` commit `fbe8fb9f5f8d0cc39f2d8bfed2c2d79a6161e35e`, produced by merged PRs #1, #2 and #3. All proposed changes belong to `audit/pre-release-cleanup` until independently reviewed and integrated.

## What has been verified

- **Existing installed game**: build 90 was used under Wine-staging 11.17 (without a virtual desktop) and Proton/Wine 11.0. Both logs showed the two real `IDirectDraw::SetDisplayMode` calls redirected from 640×480×16 to 1920×1080×16, a 1440×1080 centered 4:3 video surface, handoff to the first real OpenGL swap, ~59.5 FPS after stabilization, exit code 0, and original Glide DLL restoration.
- **Complete AVI**: a separate build-85 Proton test allowed approximately 40 seconds of Cinepak playback to complete naturally (`MCI_VIDEO mode="stopped"`). The build-90 tests skipped the video: they do not independently prove a second complete viewing.
- **Windows CI**: `main` GameFrame workflow #94 completed with 41 x86 and 86 x64 Rust tests passing, Glide export/PE checks, Windows LoadLibrary smoke, embedded-payload checks and rejection of a missing-CD launch. The retired legacy nGlide-only workflow was not a distributable build.
- **Known safeguards**: staged CUE/BIN extraction and post-copy manifest verification; SHA-256 gates on the retail EXE and embedded DLLs; symlink/unsafe-path rejection; per-installation mutex; fail-closed replacement; preservation of unknown `glide.dll`; single consolidated session log; CD audio FLAC without distributing copyrighted tracks.

These are **source inspection and observed tests**, not independent proof that every operating-system or display-driver scenario is correct.

## Implemented in this audit PR

1. **Interrupted-session recovery**: exact SHA-256 allowlist includes CI GameFrame artifacts #90–#94, plus earlier known builds. Tests assert each new entry. Unknown or altered DLLs are still rejected.
2. **One distributable CI pipeline**: retire `.github/workflows/windows.yml`, which built an x64 EXE **without** the required GameFrame wrapper. Move its unique CLI/log safety checks to the real bundled GameFrame workflow.
3. **Release-build traceability**: check out the PR's actual head commit and embed the checked-out commit's short hash in `--version` instead of a temporary PR merge SHA. Generate `SHA256SUMS.txt` for every packaged file.
4. **Accurate names and messages**: `launch_gameframe` replaces the former internal `launch_gameframe_test` name, with no changed launch behavior. Logs now distinguish a *requested* Wine hook from a *confirmed* DirectDraw call. Clarify default GameFrame versus legacy opt-in nGlide/DXVK diagnostics.
5. **Clear release information**: `docs/DISTRIBUTION.md` replaces the stale experimental artifact guide; `README.md` and the package describe the current `main` status and the remaining release gates. Artifacts are explicitly labeled **PreRelease**, not published releases.

This PR deliberately does **not** rewrite Glide/OpenGL, native WinMM/MCI sequencing, DirectDraw ABI, CD extraction, or the binary patch offsets that were validated in the field.

## Risk assessment

| Priority | Area | Finding and release decision |
| --- | --- | --- |
| **Blocking before public release** | Windows native gameplay | CI tests load the DLL, **not** the retail game; a real Windows 10/11 test must exercise intro, controls, 4:3 viewport, music, game exit and DLL restore |
| **Blocking before public release** | Clean CUE/BIN installation | Existing-installation logs do not validate first-use extraction/patching/FLAC on a clean machine; test an original supported retail image end-to-end |
| **Validate before release** | Physical display changes | Wine logs show identical 1920×1080 dimensions before and after the DirectDraw calls, but 16-bit depth is still forwarded. Neither physical display-mode/depth nor refresh-rate changes have been independently ruled out |
| **Follow-up hardening** | Dependency reproducibility | This repository does not track a Cargo.lock for the root and x86 bridge; CI uses stable Rust and downloads crates. For a release tag, consider committing tested lockfiles and applying `--locked`. GameFrame upstream is source-commit pinned |
| **Follow-up hardening** | Older GameFrame DLL inventory | Recovery must recognize only known exact hashes. Every newly distributed artifact needs its recorded digest, or a dedicated authenticated installation receipt design; never match arbitrary Glide DLLs by filename alone |
| **Keep isolated** | Legacy diagnostics | Old nGlide/DXVK/Vulkan experiments are still accessible by explicit developer flags and the corresponding opt-in diagnostic modules. Removing them wholesale would change untested behaviors; consider separate feature-gating/refactor after release |
| **Follow-up hardening** | Branch/release policy | Confirm GitHub branch rules, CI requirements and signed/tagged release workflow before publishing publicly. A green CI run is not game-play verification |

## Minimal grouped manual acceptance tests

**Test A — Native Windows 10/11 + clean install (release gate)**

Place an original verified USA CUE/BIN set next to the pre-release EXE in a **new, writable** folder. Run the EXE with no arguments. Confirm extraction finishes, verify the native fullscreen 4:3 Cinepak intro **without skipping**, ensure no undesirable physical resolution change, then check gameplay, input, FLAC soundtrack and the frame limit. Exit normally. Check that `CyberGladiatorsFix.log` reports successful installation, real game launch and clean exit, and that unknown/supplied Glide DLLs were preserved. Remove/relocate the CUE/BIN files and launch again: existing-install detection must work. Run `--verify` for strict integrity. On native Windows the `DDRAW_NATIVE_SUMMARY enabled` field should remain `0`; the Wine-only interception must never engage.

**Test B — Linux regression (after this cleanup PR)**

Run the pre-release binary under Wine-staging 11.17 without a virtual desktop and under Proton. Confirm full-screen video, transition, controls, FLAC, ~60 FPS steady-state and normal exit; inspect `DDRAW_NATIVE_SUMMARY` for the two successful native-size forward calls, zero shadow-limit events and the expected 4:3 geometry. For maximum confidence, allow one introduction to finish naturally rather than skipping.

**Test C — Recovery safety (if available)**

Simulate a terminated previous GameFrame session **only in a disposable copy** of the installation. Keep one correctly named and SHA-matching original Glide backup plus the byte-identical known prior project GameFrame DLL. The new EXE should recover and restore those bytes without altering unrelated files. An unknown or renamed DLL must fail closed instead of being silently overwritten. Do not tamper with the sole real installation.

## Release decision

A successful build/audit PR can merge to `main` after CI and basic Wine regression testing. **Do not publish the first public release** until the native Windows and first-time CUE/BIN tests are actually completed and reviewed. Preserve the last field-validated binary while evaluating any new artifact.

Signed: **legluondunet assisted by ChatGPT (OpenAI)**.
