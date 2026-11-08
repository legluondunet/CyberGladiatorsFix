# Release readiness and final source audit — candidate v0.2.13

Audit updated **2026-11-09** against \`main\` at **\`43fb30c27fa7353e923a69aa0352c907bd024698\`** (merged PR #9). The intended first public version is **v0.2.13**; this document **does not authorize a tag, GitHub Release publication, or a claim of universal hardware compatibility**. The final release commit will change if this audit PR merges.

## Decision

**Release candidate technically prepared; public release not yet published.** Current source, Windows build/packaging checks and real-machine field evidence support a **limited-scope US/FR Windows 11 and Wine-staging 11.17 first release**. Before publication, run the final tag build from the exact merged source commit, inspect its attached package and SHA256SUMS, and perform a short native-Windows smoke check of the final **upper-right menu signature** (last position change was accepted on both editions under Wine; earlier Windows signature and shared engine had been validated). Do not substitute a GitHub Actions success for playing the retail game.

There are **no public GitHub Releases** as of this audit. The CI artifact currently expires after 14 days and must be uploaded to a GitHub Release for durable distribution.

## Verified source and acceptance evidence

| Topic | Evidence and limitations |
| --- | --- |
| Editions | Only authenticated original **USA** and **French** retail CUE/BIN images. Exact original EXE / disc-resource hash profiles; fail closed on unknown versions. No standalone ISO or unsupported localized editions. |
| First-time installation | FR/Windows: clean original \`CYBERG.cue\` install and gameplay. FR/Wine: clean \`CYBERG.cue\` install, second run, full AVI and FLAC. US/Wine: clean original USA CUE/BIN installation, 342 extracted files, 14 FLAC tracks and 355 authenticated CD resources verified in the 2026-11-08 23:55:56 log for build \`0.2.13-cf9001d3\`. US/Windows: complete intro/normal common-engine acceptance previously documented in PR #6. |
| Common runtime | Merged [PR #6](https://github.com/legluondunet/CyberGladiatorsFix/pull/6): one project-owned DirectDraw v1 COM/MCI/WinMM codepath for both editions and both platforms. Only focus/window-manager adaptation is host-specific; there is no duplicate default video engine. Merged [PR #7](https://github.com/legluondunet/CyberGladiatorsFix/pull/7) removed unused speculative and unreachable video/DirectDraw experiments. |
| Video / input / fullscreen | Real field testing of natural ~39–40-second Cinepak playback, keyboard skip, reversible Alt+Tab, first real OpenGL swap handoff and 4:3 aspect under Windows 11 and Wine/XFCE. Default owned COM wrapper avoids physically changing the desktop resolution, as verified in the observed logs; behavior on all window managers and GPUs is **not** guaranteed. |
| Audio / display | Native MCI AVI, decoded FLAC CD audio, 640×480 logical Glide, centered 4:3 presentation at desktop resolution, GameFrame/OpenGL ~60 FPS steady-state, normal exit and original Glide DLL backup/restore safeguards. |
| Menu signature | [PR #8](https://github.com/legluondunet/CyberGladiatorsFix/pull/8) added conditional transparent text; [PR #9](https://github.com/legluondunet/CyberGladiatorsFix/pull/9) places **\`CyberGladiatorsFix v<version>-<8-character commit>\`** at the 4:3 menu's upper-right edge with **4-pixel margin**. The user validated this latest position **visually on FR and US editions**. US Wine log: \`CYBER_MENU_SIGNATURE_PROBE poll=2 visible=1\`, \`poll=76 visible=0\`, \`CYBER_MENU_SIGNATURE_STYLE ... top_right=1 edge_margin_px=4\`; the palette used \`fallback rgb=[99,94,137]\`. In-menu visibility is image-heuristic-based; fonts visually approximate original text, not extracted proprietary glyphs. |
| CI | [Main run #42533077838](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/42533077838) **success**, based on the merged PR #9 head: x86 Glide PE/exports, x86 WinMM bridge, x64 launcher tests, embedded DLL integrity, CLI safety tests, one-file package plus license notices and SHA256SUMS, and x86 DLL loader smoke. **No CI test plays the original game.** |

## Code and distribution scope

- **One download executable:** \`CyberGladiatorsFix.exe\` for Windows x64, also runnable under supported Wine/Proton. It embeds its x86 GameFrame/Glide DLL and the project WinMM/CDDA DLL. No proprietary original game data, copyrighted music, nGlide, DXVK or sidecar executable is redistributed.
- On first use, run beside one original CUE/BIN set. The game is staged, validated and installed; subsequent runs need no CUE/BIN. \`--verify\` performs strict asset verification; \`AllowModifiedIQ=1\` is an explicit **launch-only** exception for original-sized IQ files, not an exception to \`--verify\`.
- \`glide.dll\` replacement uses SHA-256-bound backup paths, recovery checks and restore-on-exit for a previously installed user DLL; existing unknown/mismatched assets are not silently overwritten. Verified project GameFrame is retained when there was no original DLL. Source: \`src/gameframe.rs\`.
- A single session log \`CyberGladiatorsFix.log\` is written next to the EXE; it contains version-commit, observed host information, edition, integrity events and result. The original source images and verified immutable CD assets remain untouched.
- **Licenses:** project GPL-3.0-or-later; GameFrame pinned to \`tanepiper/game-frame@f195a074a63cc150673c38becdec696163b0b3d1\`, Apache-2.0. The package contains both license texts.
- **Distribution content:** executable, \`README.txt\`, \`LICENSE-GPL-3.0\`, \`LICENSE-APACHE\` and \`SHA256SUMS.txt\`. No game EXE, \`.cue\`, \`.bin\`, \`.flac\` or unpacked DLL.
- **Non-goals for this release:** add ISO, additional languages, replacement game assets, experimental nGlide/DXVK modes, a new SDL input subsystem or rewriting \`cdda/src/video_trace.rs\`.

## Remaining bounded risks — explicit release decisions

| Priority | Risk / decision |
| --- | --- |
| **Pre-publication gate** | Tag only after this audit PR is merged and CI passes at that exact head. **Build and check the version-matched \`v0.2.13\` tag**. Match the 8-character source commit in the EXE and checksum sheet; do not build from a temporary PR merge ref. |
| **Pre-publication gate** | Final **native Windows 11** sanity run of the latest upper-right signature, fullscreen/menus, FLAC and clean exit. Shared engine and earlier menu placement had native Windows acceptance, but the final 4-pixel top-right positioning was field-confirmed here via user US/FR edition feedback and a Wine log. |
| **Pre-publication gate** | Verify the final **tagged** CI package exactly has the five expected files, that four SHA-256 entries validate, that the \`--version\` source suffix matches the tag commit, then upload the immutable package to a draft GitHub Release and inspect published download links. Retain CI as a disposable 14-day build artifact only. |
| **Documented limitation** | \`Cargo.lock\` files for the root and x86 bridge are not tracked, \`rust-toolchain@stable\` / \`windows-latest\` move over time. The upstream wrapper source is pinned, but **fully reproducible byte-for-byte builds are not yet guaranteed**. Do not describe the package as reproducible. Follow up by introducing reviewed lockfiles and toolchain pins in a separate PR with CI and a final host recheck. |
| **Documented limitation** | Actual Windows 10, other hardware and Wine/Proton versions have not all been verified in this final tag cycle; make support claims by edition and tested platform, not universal. |
| **Deferred maintenance** | \`cdda/src/video_trace.rs\` is large. Refactor only after release, with field-verified behavior tests. Legacy diagnostic escapes remain strictly opt-in, not part of the advertised supported features. |

## Release procedure

Follow [RELEASE_PROCESS.md](RELEASE_PROCESS.md) and use [RELEASE_NOTES_v0.2.13.md](RELEASE_NOTES_v0.2.13.md). **Do not create tags or publish a release as part of an audit-only PR.** The release manager decides whether to mark this first release as prerelease after the final smoke check.

The historical 2026-11-06 risk matrix is preserved through Git history. Its earlier statements that FR/Wine, clean CUE/BIN installs or Windows gameplay were untested were superseded by the field evidence above.

Signed: **legluondunet assisted by ChatGPT (OpenAI)**.
