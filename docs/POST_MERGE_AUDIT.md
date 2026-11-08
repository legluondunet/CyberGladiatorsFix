# Post-merge source audit — 2026-11-08

Baseline: [merged PR #6](https://github.com/legluondunet/CyberGladiatorsFix/pull/6), commit `c12d1eff`. This audit applies to the source after the **shared default** DirectDraw v1 COM / WinMM MCI / GameFrame path was field-tested for US and FR retail editions under both Windows 11 and Wine-staging 11.17. The change set is **maintenance only**, not a new renderer or a public release.

## Runtime ownership

| Component | Production implementation | Platform differences |
| --- | --- | --- |
| Retail CUE/BIN install and SHA-256 gate | `src/{cue,install,patch,fr_patch,verify}.rs` | None in its core; edition offsets and original SHA-256 fingerprints necessarily differ |
| Owned DirectDraw v1 wrapper | `cdda/src/ddraw_native.rs` and `cdda/native/fr_virtual_ddraw.cpp` | Same COM wrapper on Windows and Wine; runtime identity only informs logging |
| Native MCI AVI/4:3, initial cover and input | `cdda/src/video_trace.rs` + `cdda/src/lib.rs` | Same decoder, Win32 message pumping, WinMM exports and first-GL-frame handoff |
| Focus, fullscreen/window-manager integration | `cdda/src/video_trace.rs` and `tools/gameframe/patch_fullscreen.py` | Windows temporarily hides the black game entry from Alt+Tab; Wine uses reversible host unmap/minimize, one guarded initial keyboard-focus transfer and monitor-sized Win32 popup hints |
| FLAC / virtual CD audio | `cdda/src/player.rs`, `cdda/src/lib.rs` | Common |
| Glide/OpenGL 60 FPS and restoration | pinned GameFrame wrapper and `src/gameframe.rs` | Common; monitor/window geometry adapter handles Wine/XFCE |
| Host report | `src/diagnostics.rs`, optional `CyberGladiatorsFix-Wine.sh` | Windows DEVMODE watcher when supported; Linux wrapper adds OS/kernel context only |

## Safe dead-code removal

1. **Deleted `cdda/src/virtual_ddraw.rs` (9,064 bytes)** and its private module declaration. It described a speculative in-memory French DirectDraw *model*, explicitly stated it was **never connected to Win32 COM** and was not called from any production function. The real, verified COM bridge is the separate C++/Rust implementation above. Its five model-only tests were not runtime integration tests; keeping them as if they validated the production wrapper was misleading.
2. **Removed the unused native `MCI_UPDATE` status-poll refresh experiment** from `cdda/src/video_trace.rs`: `VirtualMciRefresh`, its state, `mci_refresh_is_due`, `virtual_fr_movie_repaint_target`, `virtual_fr_movie_repaint_result` and the isolated throttle unit test. None of its production entry points was called by the WinMM bridge or any other source; it was superseded by the real MCI `WM_PAINT` / 50ms timer and bounded UI-message pump used by all editions.
3. **Clarified shared-runtime names and comments:** the common AVI focus updater is no longer named `refresh_experimental_movie_focus`; historical A/B command aliases are grouped without changing accepted arguments or which renderer they launch. Corrected the stale CLI help and module comments that described verified default Wine paths as untested experiments.
4. **Reconciled documentation after PR #6 merge.** The prior FR/Wine/Windows acceptance matrix remains cited; dated historical design reports remain as archival references, not current instructions.

## Explicitly retained

- The Windows/Wine **focus and fullscreen adapters**. They correct field-confirmed different window-manager behavior; removing them would break XFCE taskbar layering or keyboard/Alt+Tab.
- Hash-pinned US/FR edition patch profiles, safe CUE/BIN normalization, the 150-sector zero CD audio pregap exception, and original Glide restoration.
- Existing **nGlide / WineD3D / DXVK / Vulkan diagnostic probes** and a disabled Wine native DirectDraw forwarding fallback. None is selected in normal one-click operation, but they may be useful in new GPU/Wine compatibility investigations. Wholesale removal should be separately reviewed and field-tested.
- Legacy CLI test aliases as compatibility entry points only: they select the same common default engine. They are not listed as user-facing launch commands.

## Residual maintenance risks

- `cdda/src/video_trace.rs` remains a large Win32 video/focus/handoff module. Splitting it into smaller files would be a more invasive source-ownership refactor: defer until after a stable release or a separate test-backed PR. It is **not** evidence of duplicate Windows/Wine rendering engines.
- The launcher has a diagnostic renderer launcher separate from the normal GameFrame path. Keep it explicitly opt-in; do not let diagnostic environment variables be inherited into the validated game child.
- Current GitHub Actions builds/tests verify Rust paths, x86 WinMM exports, native PE loading, bundle hashes and package contents, **not interactive video playback on the GitHub runner**. The four-way host/edition acceptance was performed on real user machines before PR #6; source-only cleanup does not substitute for renewed field testing if behavior changes later.
- Pinned upstream GameFrame source helps reproducibility. The historical release-readiness audit also notes Cargo dependency lockfile policy as a possible release-hardening step; this cleanup does not silently change dependency versions or CI packaging.

## Exit criteria for this maintenance PR

- No modification to installed game bytes, retail signatures, CUE parsing, video window APIs, focus transitions, COM wrapper, FLAC playback, Glide display logic or physical monitor-mode calls.
- Run the Windows one-file GitHub Actions CI at the **cleanup commit** and inspect the test/PE export/package status.
- Merge this PR only after green CI; no user test required for the removed unreachable code and comment-only changes. This is not authorization for a public release/tag.

legluondunet assisted by ChatGPT (OpenAI)
