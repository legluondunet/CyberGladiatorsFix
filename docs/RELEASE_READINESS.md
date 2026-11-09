# Release readiness and final source audit — candidate v1.0.0

Source acceptance recorded after merged PR #12 (`dc8aa6a4`) and documentation PR #13 (`60263016`). This version-only preparation does not alter the previously field-tested video, audio or gameplay code. The intended first public version is **v1.0.0**; this document **does not authorize a tag, GitHub Release publication, or a claim of universal hardware compatibility**. The final release commit will change if this audit PR merges.

## Decision

**Release candidate technically prepared; public release not yet published.** Current source, Windows build/packaging checks and real-machine field evidence support a **limited-scope US/FR Windows 11 and Wine-staging 11.17 first release**. Before publication, run the final tag build from the exact merged source commit and inspect its attached package and SHA256SUMS. The upper-right menu signature is now also field-validated under **FR/native Windows** in PR #11, but the actual final tagged binary should still get a minimal end-to-end Windows smoke check. Do not substitute a GitHub Actions success for playing the retail game.

There are **no public GitHub Releases** as of this audit. The CI artifact currently expires after 14 days and must be uploaded to a GitHub Release for durable distribution.

## Verified source and acceptance evidence

| Topic | Evidence and limitations |
| --- | --- |
| Editions | Only authenticated original **USA** and **French** retail CUE/BIN images. Exact original EXE / disc-resource hash profiles; fail closed on unknown versions. No standalone ISO or unsupported localized editions. |
| First-time installation | FR/Windows: clean original `CYBERG.cue` install and gameplay. FR/Wine: clean `CYBERG.cue` install, second run, full AVI and FLAC. US/Wine: clean original USA CUE/BIN installation, 342 extracted files, 14 FLAC tracks and 355 authenticated CD resources verified in the 2026-11-08 23:55:56 log for build `0.2.13-cf9001d3`. US/Windows: complete intro/normal common-engine acceptance previously documented in PR #6. On **2026-11-09**, an additional **US/Wine-staging 11.17** run of PR #12 (`0.2.13-731aa136`) verified 355 original CD resources, a full ~39-second AVI, Alt+Tab recovery, unchanged physical resolution, first OpenGL present, CDDA/FLAC and normal exit; the user confirmed no visible issue. This was a repeat launch of an installed game, not a new installation. |
| Common runtime | Merged [PR #6](https://github.com/legluondunet/CyberGladiatorsFix/pull/6): one project-owned DirectDraw v1 COM/MCI/WinMM codepath for both editions and both platforms. Only focus/window-manager adaptation is host-specific; there is no duplicate default video engine. Merged [PR #7](https://github.com/legluondunet/CyberGladiatorsFix/pull/7) removed unused speculative and unreachable video/DirectDraw experiments. |
| Video / input / fullscreen | Real field testing of natural ~39–40-second Cinepak playback, keyboard skip, reversible Alt+Tab, first real OpenGL swap handoff and 4:3 aspect under Windows 11 and Wine/XFCE. Default owned COM wrapper avoids physically changing the desktop resolution, as verified in the observed logs; behavior on all window managers and GPUs is **not** guaranteed. |
| Audio / display | Native MCI AVI, mandatory FLAC CD audio, 640×480 logical Glide, centered 4:3 at desktop resolution, GameFrame/OpenGL ~60 FPS steady-state and normal exit. The archived old Glide DLL is **not restored** after exit: the SHA-256-verified bundled GameFrame stays installed, and a second same-build launch keeps it without creating a new archive. |
| Menu signature | [PR #8](https://github.com/legluondunet/CyberGladiatorsFix/pull/8) added conditional transparent text; [PR #9](https://github.com/legluondunet/CyberGladiatorsFix/pull/9) places **`CyberGladiatorsFix v<version>-<8-character commit>`** at the 4:3 menu's upper-right edge with **4-pixel margin**. The user validated this latest position **visually on FR and US editions**. US Wine log: `CYBER_MENU_SIGNATURE_PROBE poll=2 visible=1`, `poll=76 visible=0`, `CYBER_MENU_SIGNATURE_STYLE ... top_right=1 edge_margin_px=4`; the palette used `fallback rgb=[99,94,137]`. In-menu visibility is image-heuristic-based; fonts visually approximate original text, not extracted proprietary glyphs. |
| CI | [PR #12 Windows workflow #42677217583](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/42677217583) **success** on the tested commit `731aa136`; [merged `main` workflow #42681141750](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/42681141750) **success** on `dc8aa6a4` after merging PR #12. Earlier packaging audits validated the six-file distribution and five SHA-256 checksums. CI validates build, unit checks, DLL exports and package structure; it does **not** play the retail game. |

## Code and distribution scope

- **One download executable:** `CyberGladiatorsFix.exe` for Windows x64, also runnable under supported Wine/Proton. It embeds its x86 GameFrame/Glide DLL and the project WinMM/CDDA DLL. No proprietary original game data, copyrighted music, nGlide, DXVK or sidecar executable is redistributed.
- On first use, run beside one original CUE/BIN set. The game is staged, validated and installed; subsequent runs need no CUE/BIN. `--verify` and automatic launch **both** require strict retail resource/IQ SHA-256 integrity. CDDA/FLAC always runs through the bundled bridge; the only user INI setting is `[Video] SkipIntro`.
- `glide.dll` replacement compares to the bundled GameFrame SHA-256. An older/unknown file is moved to `glide.dll.bak` or a collision-free hashed/numbered `.bak` without overwriting existing backups, then GameFrame is installed and **retained after exit**. A matching GameFrame is reused. PR #11's FR/Windows logs validate both archive collision and identical second launch. Source: `src/gameframe.rs`.
- A single session log `CyberGladiatorsFix.log` is written next to the EXE; it contains version-commit, observed host information, edition, integrity events and result. The original source images and verified immutable CD assets remain untouched.
- **Licenses:** project GPL-3.0-or-later; GameFrame pinned to `tanepiper/game-frame@f195a074a63cc150673c38becdec696163b0b3d1`, Apache-2.0. The package contains `LICENSE-GPL-3.0` and `LICENSE-APACHE`. Distribution hardening also includes the original MIT notice (`LICENSE-MIT`) for runtime ISO parser `isomage 2.1.0` (copyright 2025 Jack Danger).
- **Distribution content:** executable, `README.txt`, `LICENSE-GPL-3.0`, `LICENSE-APACHE`, `LICENSE-MIT` and `SHA256SUMS.txt`. No game EXE, `.cue`, `.bin`, `.flac` or unpacked DLL.
- **Non-goals for this release:** add ISO, additional languages, replacement game assets, experimental nGlide/DXVK modes, a new SDL input subsystem or rewriting `cdda/src/video_trace.rs`.

## Remaining bounded risks — explicit release decisions

| Priority | Risk / decision |
| --- | --- |
| **Pre-publication gate** | Tag only after the release-documentation update is merged and CI passes at that exact head. **Build and check the version-matched `v1.0.0` tag**. Match the 8-character source commit in the EXE and checksum sheet; do not build from a temporary PR merge ref. |
| **Pre-publication gate** | **FR/native Windows** already confirms the final upper-right signature, fullscreen, FLAC and clean exit (9 Nov). Repeat a brief smoke test using the **exact tag-built binary**, not solely earlier PR artifacts. An additional US/Wine-staging 11.17 field log now covers the PR #12 build with the Glide archive/INI policy and the local AVI path rewrite, including visual acceptance. **This is not a test of a future tag-built executable**; the final tag still needs its own smoke check. |
| **Pre-publication gate** | Verify the final **tagged** CI package exactly has the six expected files, that five SHA-256 entries validate, that the `--version` source suffix matches the tag commit, then upload the immutable package to a draft GitHub Release and inspect published download links. Retain CI as a disposable 14-day build artifact only. |
| **Documented limitation** | `Cargo.lock` files for the root and x86 bridge are not tracked, `rust-toolchain@stable` / `windows-latest` move over time. The upstream wrapper source is pinned, but **fully reproducible byte-for-byte builds are not yet guaranteed**. Do not describe the package as reproducible. Follow up by introducing reviewed lockfiles and toolchain pins in a separate PR with CI and a final host recheck. |
| **Documented limitation** | Actual Windows 10, other hardware and Wine/Proton versions have not all been verified in this final tag cycle; make support claims by edition and tested platform, not universal. |
| **Deferred maintenance** | `cdda/src/video_trace.rs` is large. Refactor only after release, with field-verified behavior tests. Legacy diagnostic escapes remain strictly opt-in, not part of the advertised supported features. |

## Release procedure

Follow [RELEASE_PROCESS.md](RELEASE_PROCESS.md) and use [RELEASE_NOTES_v1.0.0.md](RELEASE_NOTES_v1.0.0.md). **Do not create tags or publish a release as part of an audit-only PR.** The release manager decides whether to mark this first release as prerelease after the final smoke check.

The historical 2026-11-06 risk matrix is preserved through Git history. Its earlier statements that FR/Wine, clean CUE/BIN installs or Windows gameplay were untested were superseded by the field evidence above.

Signed: **legluondunet assisted by ChatGPT (OpenAI)**.
