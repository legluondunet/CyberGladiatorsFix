# CyberGladiatorsFix v2.0.1 — GE-Proton AVI compatibility

Maintenance update to the already published [v2.0.0](https://github.com/legluondunet/CyberGladiatorsFix/releases/tag/v2.0.0). **These are proposed notes for a future v2.0.1 publication, not evidence that v2.0.1 has been released.**

CyberGladiatorsFix remains a single Windows x64 installer/launcher for original authenticated **USA and French CyberGladiators CUE/BIN** discs. It runs on native Windows and under Wine/Proton. The archive contains the open-source compatibility tools, not game executables, original game assets, disc images or CD music.

## Fixed in v2.0.1

**GE-Proton introduction AVI initially black while audio played.** Under Lutris with GE-Proton, the native MCI movie could be playing and painting normally while its picture was hidden behind the still-black game window. Manually switching windows with Alt+Tab sometimes made the movie visible.

[Merged PR #42](https://github.com/legluondunet/CyberGladiatorsFix/pull/42) changes the temporary AVI-host **window ownership**, not the AVI decoder: on an identified GE-Proton launch the popup is now **owned by the real game window** and uses `WS_EX_TOOLWINDOW` while retaining non-activating, topmost behavior. That supplies an explicit owner/owned stacking relationship to the window manager. No synthetic Alt+Tab, alternate video renderer or new codec is needed.

- Detection is automatic when the runner exposes a GE-Proton path in `PROTONPATH`, `STEAM_COMPAT_TOOL_PATHS` or `STEAM_COMPAT_MOUNTS`; the path values are not written to the game log.
- Ordinary Wine/Wine-staging and native Windows keep the **independent AVI-host** behavior they used in v2.0.0.
- The developer-only environment override `CYBER_AVI_GAME_OWNED_HOST=1` forces the owned-host choice on supported Wine movie paths; `=0` forces the independent choice; unset selects the automatic policy. **Users do not need to configure this.** It is not a new INI setting.
- Only video-host window management and diagnostic reporting changed. Original DirectDraw virtualization, Cinepak/MCI updates, 4:3 presentation, GameFrame/Glide/OpenGL rendering, FLAC CD audio, intro skip, input and focus handoff are retained.

## Compatibility and real-host acceptance

The following are **distinct evidence types**. GitHub Actions compiles and tests the code but does not play the original retail game.

| Environment | Evidence | Limits |
| --- | --- | --- |
| **US / GE-Proton via Lutris** | Maintainer confirmed that the intro appeared without Alt+Tab using `2.0.0-ccf76046` with the experimental override, and again with automatic detection at `2.0.0-1df03404`, **without** the override. The latter run logs `game_owned=1 policy=auto-ge-proton ge_proton_source=PROTONPATH manual_override=unset`, MCI repaint, first real OpenGL-present handoff, audio start and exit 0. | The video was skipped with Enter in the automatic-detection test. No promise of all GE-Proton versions or all Linux desktops. |
| **FR / Wine (non-GE detected)** | `2.0.0-1df03404` runtime log: `game_owned=0 policy=auto-default`, MCI playback, focus loss/recovery, first OpenGL handoff and exit 0. | That runtime log does not identify the exact Wine runner or independently prove subjective image/audio quality. |
| **FR / native Windows** | **Merged `main` build `2.0.0-436cbe1e`**: real host log verifies `wine_virtual=0 game_owned=0 policy=auto-default`, native MCI playback with 169 paints, focus loss/return, successful OpenGL first-present handoff, menu, CDDA/FLAC and process exit 0. Maintainer reported everything working. | This is a reused game installation, not a fresh-disc test of the new build; movie was exited using Enter. |
| **US / native Windows** | Retail original US v2.0.0 field-tested on the pre-fix engine (`9605dff4`), including genuine COM cleanup guard, AVI, GameFrame, FLAC, focus transitions and clean exit. | No separate US native Windows run supplied on the post-merge `436cbe1e` commit; the PR #42 policy is designed to remain inactive under native Windows. |

Windows CI on the exact [merged `main` commit `436cbe1e`](https://github.com/legluondunet/CyberGladiatorsFix/commit/436cbe1ef98dddd0af6968742ea6a6d822da0a55) passed in [run #45369032974](https://github.com/legluondunet/CyberGladiatorsFix/actions/runs/45369032974). The future v2.0.1 source must also pass the release-candidate CI before publication.

## Installation and upgrading

- **New installation:** extract the official ZIP into an empty writable directory, place **one authenticated original** `CYBER.cue`/`CYBER.bin` (USA) **or** `CYBERG.CUE`/`CYBERG.BIN` (French) beside the launcher and run `CyberGladiatorsFix.exe`. The original disc image is not needed for later launches. A standalone ISO is not a supported installer input (but can be inspected with `--inspect`).
- **Upgrade from v2.0.0:** replace the launcher EXE in the existing v2.0.0 installation directory. Keep `game/`, `music/`, `state/` and the user's INI. The normal launcher validates the installed game and updates only project-owned compatibility DLLs, using hash receipts.
- **Upgrade from v1.x:** install in a **new directory**; v2 does not migrate v1 layouts or saved games.
- Defaults remain `[Video] SkipIntro=0` and `[Debug] Log=0`. Logs are separate launcher/runtime files beside the EXE; set `Log=1` to generate a fresh diagnostic session.

The same six-file ZIP structure remains: `CyberGladiatorsFix.exe`, `README.txt`, `LICENSE-GPL-3.0`, `LICENSE-APACHE`, `LICENSE-MIT`, `SHA256SUMS.txt`. The GitHub Release additionally supplies the ZIP SHA-256 as a separate asset.

## Scope and limits

This maintenance version **does not** introduce a new renderer, filtering or graphics options, installer-supported ISO images, a new save-game format or additional retail language editions. Full unskipped AVI completion, every GPU/WM/runner combination and extended gameplay sessions are not established by this fix's validation logs. The real-host observations above must not be substituted for an end-to-end smoke test of the **future tagged v2.0.1 ZIP**.

Source license: CyberGladiatorsFix and `fix.dll` **GPL-3.0-or-later**; bundled GameFrame **Apache-2.0**; `isomage` **MIT**. No original copyrighted game or disc audio is bundled.

Public publication is **manual and requires explicit maintainer approval**, independently of source review. See [v2.0.1 readiness](RELEASE_READINESS_v2.0.1.md) and the [publication procedure](RELEASE_PROCESS.md).
