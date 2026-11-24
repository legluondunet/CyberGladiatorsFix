# CyberGladiatorsFix — focused v2.0.1 source audit

**Date:** 24 November 2026. **Baseline:** merged `main` commit [`436cbe1e`](https://github.com/legluondunet/CyberGladiatorsFix/commit/436cbe1ef98dddd0af6968742ea6a6d822da0a55), after merged [PR #42](https://github.com/legluondunet/CyberGladiatorsFix/pull/42). Audit of release-sensitive components and distribution wiring, **not a formal, exhaustive security review** and not authorization to publish.

## Examined components and conclusions

| Component | Primary sources | Observation / decision |
| --- | --- | --- |
| GE-Proton intro and Win32 focus | `cdda/src/video_trace.rs`, `src/verify.rs` | The approved override/automatic policy is gated by the verified Wine movie profile and GE-Proton environment markers; native Windows retains an independent fullscreen AVI host. The working MCI decoder, `WM_PAINT`, timer, key filter and first real OpenGL-frame handoff remain unchanged. **No extra renderer or automatic Alt+Tab was added.** |
| Game media identification / install | `src/{cue,edition,install,patch,fr_patch,autostart,verify}.rs` | The normal path authenticates supported US/FR retail media, stages new installs, verifies game assets, keeps the original EXE backup, and refuses ambiguous or unsupported media. Later launches check mutable IQ resources for presence while hash-checking immutable resources. The named installation mutex prevents a second same-root launch from racing. |
| Runtime DLL updates | `src/{cdda,gameframe,layout,bundle}.rs` | Embedded `fix.dll` and `glide.dll` are checked for x86 PE32 shape and hashes; replacing a previously installed DLL requires a matching ownership receipt. Unknown, inconsistent or symlinked DLLs are not silently overwritten. Replacement stages and attempts rollback. No permanent runtime-DLL backups; original retail EXE backup is retained. |
| Runtime logs / configuration | `src/{settings,diagnostics,verify,main}.rs` | INI contains `[Video] SkipIntro` and `[Debug] Log`. First install/game session are logged, later `Log=0` preserves logs; `Log=1` regenerates them. Under normal game launches, the launcher sets edition and bundled renderer parameters from verified media rather than caller-provided identity. |
| CLI | `src/main.rs`, CI `gameframe-glide.yml` | The supported read-only `--inspect` command is **still implemented**. Only the obsolete `--inspect-disc` was retired. The release-prep PR adds a CI failure-path test for `--inspect` and updates CLI help describing GE-Proton. |
| Build + release packaging | `Cargo.toml`, `build.rs`, `.github/workflows/{gameframe-glide,publish-v2.0.0}.yml` | Pinned GameFrame upstream revision; CI builds/tests x86 WinMM, x86 Glide, x64 launcher, validates PE and ZIP candidates. Existing v2.0.0 publication is manual and guarded. The new **separate** `publish-v2.0.1.yml` verifies Cargo 2.0.1, final main SHA, prior stable v2.0.0, absent new tag/release, six flat files, original file hashes and ZIP SHA-256 before publishing. |
| Copyright and licensing | `LICENSE`, `LICENSE-MIT`, `third_party/game-frame/LICENSE-APACHE`, `src/layout.rs` | Runtime-supplied original game data and extracted CD music are not included in release archive. Code uses GPL-3.0-or-later, bundled GameFrame Apache-2.0 and isomage MIT; the archive includes license notices. |

## Findings addressed in the preparation PR

1. **Release version mismatch:** public `v2.0.0` already exists, but Cargo and publication documents still described it as an upcoming release. Set Cargo to `2.0.1` and prepare a **new manual v2.0.1 publication workflow**. Do not mutate the old v2.0.0 tag, workflow or assets.
2. **Outdated docs:** add notes and readiness for v2.0.1, update bundled `README.txt` source, installation/upgrade guidance, and mark v2.0.0 planning texts as historical where needed.
3. **Incomplete CLI smoke checks:** existing CI rejected old `--inspect-disc` but did not directly cover the still-supported `--inspect`. Add a missing-ISO failure test that checks the separate inspection report and confirms no installation or installer-log modification.
4. **Help text:** remove an outdated Wine-staging 11.17-only field-test reference and describe GE-Proton-specific automatic window ownership. This is documentation text inside the executable, not a runtime behavior change.

## Risk analysis / intentionally deferred

- **Runner detection:** `PROTONPATH`, `STEAM_COMPAT_TOOL_PATHS` and `STEAM_COMPAT_MOUNTS` are environment-provided hints, not a guarantee of compositor/driver behavior. Detection specifically checks for GE-Proton naming, and a diagnostic environment override is available. No universal guarantee across other Proton or GE versions.
- **Source size:** `cdda/src/video_trace.rs` is large and contains native Win32 window/message/paint/focus logic. Refactoring it immediately before a maintenance release risks regressions. Prefer a separately scoped PR and renewed game testing.
- **Dependencies:** `Cargo.lock` is not present in the reviewed repository tree. Source-pinned GameFrame, explicit direct dependencies and checksum validation reduce risks, but they do **not** guarantee identical transitive resolution across time. Consider a committed lockfile in a separately reviewed build/reproducibility task; do not silently change dependency versions in this hotfix.
- **Override and inspection:** `--launch-no-verify` is an intentional unsafe fast path: it skips preflight and should not be used as a repair mechanism. `--inspect` writes an inventory report with local input paths; do not post private path data publicly without checking it.
- **Field-test coverage:** latest merged `main` has direct native-Windows FR evidence and GE-Proton US user confirmation. Latest exact SHA has **no separate US/native Windows host run** or full-length natural AVI proof. CI cannot establish actual audio/video quality or every GPU/window-manager permutation.
- **Archive verification:** even with green Windows CI, do not claim a future public v2.0.1 ZIP passed a real-user gameplay test until that **tag-built artifact** is actually tested. Check its SHA-256 and version.

## Release acceptance

1. Review changes in the v2.0.1 preparation PR and inspect **green CI at its actual head SHA**.
2. Merge the PR only when documentation, packaging rules and CI pass. Record final `main` SHA and fresh `main` CI result.
3. Obtain **explicit authorization to publish v2.0.1**, separate from any PR merge approval.
4. Trigger the manually guarded v2.0.1 publisher from `main` with `publish-v2.0.1`. Inspect created tag, ZIP and checksums, then perform a tagged-binary game smoke test.

See [release readiness](RELEASE_READINESS_v2.0.1.md), [release notes](RELEASE_NOTES_v2.0.1.md), [publication procedure](RELEASE_PROCESS.md) and [the existing v2.0.0 Glide fidelity audit](GLIDE_EFFECT_AUDIT_v2.0.0.md). The Glide audit's visual-fidelity limitations are still applicable; this maintenance PR intentionally does not add filtering or alter rendering.
