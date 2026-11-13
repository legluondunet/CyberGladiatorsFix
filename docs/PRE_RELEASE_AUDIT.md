# Pre-release code audit — CyberGladiatorsFix

Status: cleanup merged in PR #25; this audit is not a substitute for real-game smoke tests. See [v1.0.1 readiness](RELEASE_READINESS_v1.0.1.md).

## Baseline and scope

This cleanup is stacked on the open CLI simplification PR #24, which introduced
`--inspect` for unsupported ISO/CUE media, `--verbose`, `-h`, and
`--launch-no-verify`. **Those commands must stay.** The older
`--inspect-disc` interface was removed in PR #24.

## Removed as obsolete

- Historical nGlide/DXVK renderer experiments, Vulkan A/B switches, video-delay
  probes and their associated Windows launcher dispatch code.
- Undocumented launch aliases, including bare `--launch` and the old
  `--launch --gameframe-test` / `--launch --trace` variations. Use no
  arguments for a verified normal launch, or `--launch-no-verify` to deliberately
  skip installation and asset validation.
- An unreachable installer branch following the exhaustive supported-CLI
  dispatch. `--install-only` and auto-install remain the supported paths.
- The separate proprietary nGlide installer and its bundled-DLL fallback.
  A supported release must embed GameFrame; `--verify` no longer approves a
  standalone nGlide-only distribution.
- Obsolete bridge inspection/NO-OP probe helpers that were only called from
  the retired renderer experiments, and an unused `--check-bundle` formatter.
- The former CDDA upgrade consent flag and allowlist. The normal installer
  already opted into automatic replacement. Preserve the complete existing
  SHA-qualified backup, receipt validation, staged update, rollback and
  collision protections.

## Intentionally preserved

- Verified US/FR retail disc recognition, CUE/BIN normalization, 150-sector
  pregap handling, temporary extraction, SHA-256 manifest and immutable assets
  checks; game-writable `iq/` resources remain presence-only after installation.
- Read-only `--inspect` ISO/CUE/BIN/IMG report in
  `CyberGladiatorsFix_inspect.log`.
- The source-pinned GameFrame Glide wrapper, one owned DirectDraw/MCI adapter,
  native AVI, 4:3 presentation, menu signature, Alt+Tab and desktop-focus logic.
- Embedded WinMM bridge, local FLAC/CDDA, safety checks, DLL backup and
  recovery routines, and the two launcher/game session logs.
- The native FR read-only display monitor and static DirectDraw scanner:
  they still run on the native-Windows FR game path. Removing them requires
  explicit field testing; they are not dead code.
- Guarded video forensic hooks inside the **game DLL**: although their former
  launcher command aliases are gone, removing low-level hooks without
  validating native AVI, CDDA, DirectDraw and first-present callsites would
  be unsafe this close to release. Revisit in a separate PR.
- Existing historical design/release documents as archival records; README
  and the active CLI help describe the new behavior.

## Acceptance gates before tagging the next version

1. CI Windows x64/MSVC: build source-pinned GameFrame, WinMM and launcher;
   PE32/export checks, tests, `--help`, `--inspect`, `--verify`,
   `--install-only`, and `--launch-no-verify` failure behavior on a clean host.
2. Windows 11 **and** Wine: US and FR original CUE/BIN clean install;
   second normal launch; `--install-only`; `--verify`; incomplete installation
   with `--launch-no-verify` without repair.
3. Full introduction, first-frame transition, native 4:3 menu/HUD and signature,
   video skip, FLAC music, input, Alt+Tab, borderless/fullscreen behavior and
   clean exit on both editions.
4. Verify no extra `glide.dll` backups are created unnecessarily, the old
   `.bak` is never overwritten, and the two logs are separate and correlated.
5. Publish only after a reviewed version bump, release workflow review and
   signed-off real-game acceptance. Green CI alone does **not** establish
   Windows/Wine gameplay compatibility.

No tag, release asset or currently distributed v1.0.0 executable is changed
by this audit PR.
