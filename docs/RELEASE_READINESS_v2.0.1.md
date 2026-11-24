# CyberGladiatorsFix v2.0.1 — pre-publication checklist

**Status: release-preparation branch under review; v2.0.1 NOT published.** Published stable v2.0.0 remains available and unchanged. Merging source does **not** authorize a GitHub Release. This checklist is separate from historical [v2.0.0 readiness](RELEASE_READINESS_v2.0.0.md).

## Release scope

- **Change**: auto-select a game-owned native AVI popup on detected GE-Proton, fixing a black intro with working audio; keep native Windows and other Wine window policy intact ([merged PR #42](https://github.com/legluondunet/CyberGladiatorsFix/pull/42)).
- **No other runtime changes** besides the version/CLI descriptive help text in the release-preparation PR. Original US/FR retail installers, verified hash profiles, WinMM/DirectDraw bridge, CDDA, game graphics, save data and CUE processing are not deliberately modified.
- **Distribution**: Windows x64 one-file launcher containing x86 GameFrame and x86 WinMM bridge. No original game files, game music or CD images included.
- **Version**: `Cargo.toml` 2.0.1, executable and menu signature `CyberGladiatorsFix 2.0.1-<8-char-commit>`.
- **Upgrade**: v2.0.0 layout can be kept; replace the launcher and permit the normal hash-owned DLL update. No v1.x migration.

## Source and field evidence

| Gate | State | Evidence and qualification |
| --- | --- | --- |
| PR #42 code change | **Passed** | Merged into main as `436cbe1e`; game-owned MCI popup uses existing game's window as Win32 owner and is auto-selected on GE-Proton runner markers only. |
| GE-Proton / US | **Field-passed** | Maintainer visually confirmed the intro becomes visible without Alt+Tab with the manual override `ccf76046`, and without the override under `1df03404`. The latter log records `policy=auto-ge-proton` and MCI → GameFrame handoff, FLAC and exit 0. |
| Wine / FR without GE detection | **Runtime-passed** | `1df03404` logs `game_owned=0 independent=1 policy=auto-default`, AVI playback, reversible focus changes, handoff and exit 0. Exact Wine version and subjective visual quality are not in the log. |
| Native Windows / FR, merged post-fix source | **Field-passed** | `436cbe1e`, 24 November 2026: user said all OK. Log verifies `wine_virtual=0 game_owned=0 policy=auto-default`, movie, focus loss/return, first GL frame, audio and normal exit. |
| Native Windows / US | **Previous baseline passed** | Authentic USA pre-fix `9605dff4` movie/COM guard/Glide/FLAC/exit tested. No independent US/Windows `436cbe1e` retest provided. |
| Windows CI, merged fix `main` | **Passed** | [Run #45369032974](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/45369032974) on `436cbe1e`. |
| Normal video length / extended gameplay | **Partially covered** | Recent movie tests used skip keys; logs do not establish natural end-to-end playback or broad GPU/WM coverage. |

## Preparation and publication gates

- [ ] Review the v2.0.1 audit/notes/readme and release-publisher PR; **merge only with green PR CI**.
- [ ] Check successful Windows CI on the **exact final `main` SHA** (x64 launcher, x86 WinMM and pinned GameFrame, Rust unit tests, PE checks, smoke checks, six-file package and checksums).
- [ ] Ensure the supported `--inspect` continues to write its own diagnostic report without installing; `--inspect-disc` remains retired. The release-prep PR adds explicit CI coverage for the supported path.
- [ ] Confirm `Cargo.toml` version is **2.0.1** and that v2.0.0/v1.0.1/v1.0.0 public assets are untouched.
- [ ] **Obtain the maintainer's explicit approval to publish v2.0.1**, separately from consent to merge.
- [ ] Manually run [publish-v2.0.1](https://github.com/legluondunet/CyberGladiatorsFix/actions/workflows/publish-v2.0.1.yml) from **`main`**, confirmation `publish-v2.0.1`. Never dispatch automatically on push/merge/tag.
- [ ] Confirm tag `v2.0.1` points to the approved source SHA, the workflow succeeded, and the published ZIP has **exactly six flat files** plus separate `.zip.sha256`.
- [ ] Verify the checksum, CLI `--version` and game menu's source-commit suffix in the ZIP. Run a brief real-host smoke test of the **tag-built** ZIP; GitHub CI alone cannot confirm gameplay or window appearance.

## Audit qualifications

The [pre-publication source audit](CODE_AUDIT_v2.0.1.md) deliberately preserves the working game renderer, video host, installer and DLL update logic. Current CI and field evidence support a focused **maintenance** release, not a claim of universal GE-Proton, all GPUs, or full AVI runtime coverage. The source currently has no committed Cargo lockfile; its exact transitive dependency resolution is therefore not pinned solely by the manifest. The official workflow still verifies the built artifacts by checksum and embedded-DLL identity, but deterministic rebuilding across arbitrary future dependency registry states is not guaranteed. This can be hardened in a separately reviewed build-tooling PR.

See [v2.0.1 release notes](RELEASE_NOTES_v2.0.1.md), [manual release process](RELEASE_PROCESS.md) and [the previous v2.0.0 notes](RELEASE_NOTES_v2.0.0.md).
