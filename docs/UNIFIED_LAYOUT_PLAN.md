# CyberGladiatorsFix — unified clean-install layout (approved architecture)

Status: approved architecture; implementation in progress on PR #27. This document describes the target **new-install-only** layout. It does not describe what the current v1.0.1 binary already produces.

```text
<folder containing CyberGladiatorsFix.exe>/
  CyberGladiatorsFix.exe
  CyberGladiatorsFix.ini
  CyberGladiatorsFix_installer.log
  CyberGladiatorsFix.log
  game/
    cyber.exe
    CYBER.EXE.bak
    cyber.ini
    fix.dll
    glide.dll
    graphics/
    iq/
    ... verified retail game files and saves
  music/
    Track02.flac
    ...
    Track15.flac
  state/
    CyberGladiatorsFix.manifest
    CyberGladiatorsFix.sha256
    CyberGladiatorsFix.install-receipt.txt
  docs/
    README.md
    LICENSE-GPL-3.0
    LICENSE-APACHE
    LICENSE-MIT
    CyberGladiatorsFix.ini.example
    build-info.txt
```

**No `CyberGladiatorsFix/` subdirectory is created.** The two embedded x86 runtime DLLs live next to `game/cyber.exe`, for reliable Windows/Wine DLL lookup. The game directory otherwise contains the original game files, its native `cyber.ini`, modified executable and required DLLs only.

## Ownership and replacement

- Rename our WinMM/CDDA bridge from `CyberGladiatorsFix.dll` to `fix.dll`; update verified game-import patches, PE exports and release bundle checks.
- `game/glide.dll` is the bundled open-source GameFrame runtime. Both `game/fix.dll` and `game/glide.dll`, when previously installed and owned by this installer, are replaced after staged hash/PE verification **without .bak backups**.
- Never silently overwrite unknown, user-supplied files or traverse unsafe symlinks. Refuse unexpected conflicting files instead of treating them as installer-owned.
- The retail original `CYBER.EXE.bak` remains in `game/`: this is a backup of the user's original retail executable, **not** of an installer-owned DLL.
- Preserve game-writable `iq/` files and user settings on ordinary subsequent runs.
- No older version installation migration or old-layout detector/repair path: the new architecture starts from an empty directory plus a supported original US/FR CUE/BIN. Do not transfer, rewrite or remove existing v1.0.1 installations. A non-empty incompatible destination is rejected.

## Install, verification and runtime

1. Keep one installer/launcher EXE, automatic verified CUE/BIN selection, installation, verification and launch. Input CUE/BIN stays outside the installed tree and is never modified.
2. Extract retail assets into `game/` in a private staging area; preserve edition authentication and source-EXE backup. Prepare `music/` separately from exact CDDA tracks; encode FLAC losslessly (44.1 kHz / 16-bit / stereo).
3. Stage and verify all expected game files, runtime DLLs and FLAC tracks **before** publishing new directories. On partial failure, clean only our new temporary/created paths and never destroy source images, previous game files or saves.
4. Put project manifests, hash receipt(s) and installation receipt in `state/`. Manifest entries are rooted explicitly in the intended `game/` or `music/` trees; reject escaping paths. `iq/` policy: strict checks on first install, presence-only on future verification.
5. Keep `CyberGladiatorsFix.ini` and both session-scoped (noncumulative) logs beside the launcher. Native `game/cyber.ini` stays independent.
6. Launch `game/cyber.exe` with `game/` as working directory. Inject an explicit **absolute** `music/` location into the WinMM/CDDA bridge (for example a child-only `CYBER_MUSIC_DIR` variable), so that `cdda/src/lib.rs` and `cdda/src/player.rs` never assume `game/music/`. Runtime and installer logs stay in the launcher directory.
7. Keep existing GameFrame 4:3, native AVI, CDDA, 60 FPS, focus/Alt-Tab, and US/FR SHA-256 policies untouched except for required path/import changes.

## Source changes

- Add a centralized `src/layout.rs` resolving launcher root, `game/`, `music/`, `state/`, `docs/`, and root-owned config/log paths.
- Refactor `src/install.rs`, `src/autostart.rs`, `src/verify.rs`, `src/audio.rs`, `src/cdda.rs`, `src/settings.rs`, `src/main.rs` to use those paths and the staged/published layout.
- Update `src/patch.rs` and embedded-bridge build setup for the shorter `fix.dll` name without weakening executable hash verification. Adjust game-bridge `cdda/src/lib.rs` and `cdda/src/player.rs` for the sibling `music/` path.
- Remove obsolete music/installation migration code when no longer called. Do not retain fallback code for the previous layout.
- Update readme, distribution notes, tests and packaging contract to match.

## Acceptance before merge

- GitHub Windows CI, Rust unit tests and DLL export checks; only the source-built and authenticated open-source DLLs may be bundled.
- Clean CUE/BIN installs from US and FR originals; `--install-only`, `--verify` (read-only), `--launch-no-verify`, normal launch and second launch without disc media.
- Validate `fix.dll` imports, GameFrame loading, FLAC playback, complete intro/skip, 4:3, Alt-Tab and clean exit on native Windows and Wine/Proton.
- Confirm no unwanted backups of `fix.dll` / `glide.dll`, original `CYBER.EXE.bak` retained, `iq/` not changed on relaunch, logs reset per session, and no empty `CyberGladiatorsFix/` directory.
- Keep this work on a separate **draft PR** until a test binary has been accepted. Never update the published v1.0.1 release artifacts.
