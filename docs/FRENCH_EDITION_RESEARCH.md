# Cyber Gladiators — French edition identification (static research only)

**Status: research / NOT SUPPORTED OR PATCHED YET.** No French game asset or proprietary executable is checked into this repository. All findings below were obtained from a user-provided `cyber.exe` and must be verified against the complete original French CUE/BIN before enabling any edits or downloads.

Audit date: 2026-11-07. Branch: `feature/french-edition`. Preserve the verified US path, retail backup, and original IQ bytes.

## Original file identification

| Property | French candidate |
| --- | --- |
| File name | `cyber.exe` |
| SHA-256 | `12db5e29120832ebc304695ecfa38f6c0d825afe2640433714d27f624da53cdb` |
| File size | **352,288 bytes** (US original: **351,776** bytes; difference **+512**) |
| Format | PE32, Intel 80386, GUI, 6 sections, image base `0x00400000` |
| Section alignment | `0x1000`; raw alignment `0x200` |
| Language evidence | Embedded CP1252 text `VERSION FRANÇAISE`, `Veuillez insérer le CD-Rom de CyberGladiators dans le lecteur.` |
| Modification evidence | Original-looking calls, video gates, WINMM import and 24-byte vacant slot at every listed patch site; **not proof that the entire EXE is unmodified retail** |

The PE/COFF timestamp is unusual (interprets as 2037) and **cannot establish the file's provenance**. Check the complete French disc and independent retail reference before claiming a final supported edition.

## Patch site cross-check

Offsets are **raw file offsets**, not RVAs or virtual addresses. French original bytes were verified at the listed offsets. Any code refactor must check all original preimages before writing anything, identify the edition by the complete original SHA-256, and verify the full SHA-256 of the resulting patched file.

| Purpose | US raw offset | FR raw offset | FR original bytes | Candidate FR replacement / notes |
| --- | ---: | ---: | --- | --- |
| Speed / timing call A displacement | `0x3116` | `0x31D7` | `C4 FF FF FF` preceded by `E8` | `8D 2F 04 00` (derived call target `GetTickCount` thunk `0x00446B68`) |
| Speed / timing call B displacement | `0x3125` | `0x31E6` | `B5 FF FF FF` preceded by `E8` | `7E 2F 04 00` (same FR thunk) |
| CD-check CALL | `0x4A16` | `0x4AD5` | `E8 4C FF FF FF` | `B8 01 00 00 00`, **candidate only**; FR call targets `0x00405426`, drives and volume code seen in disassembly, caller tests EAX and loops on zero |
| Intro enabled gate | `0x63FB` | `0x63F5` | `74 2F` | `90 90` for enabled intro; `EB 2F` for SkipIntro=1, **candidate** |
| 640×480 intro gate | `0x640F` | `0x6409` | `7C 0C` | `90 90`, **candidate** |
| WINMM import descriptor Name RVA | `0x4E434` | `0x4E634` | `0C D4 3E 00` | Original RVA `0x003ED40C`; proposed proxy name RVA `0x003EDB00` |
| 24-byte import-name staging slot | `0x4EF00` | `0x4F100` | 24 zero bytes | `CyberGladiatorsFix.dll\0\0` (24 bytes), **candidate** |

**IMPORTANT:** The US timing displacement bytes `0E 30 04 00` / `FF 2F 04 00` **MUST NOT** be reused for FR. Their x86 relative-CALL targets are different. The FR imports include `GetTickCount` at a confirmed local import thunk `0x00446B68`; the table gives the two displacements recomputed relative to the FR CALL sites. The actual gameplay/timing effect still requires a runtime test.

## GameFrame / WinMM runtime address differences

| Meaning | US virtual address / RVA | FR virtual address / RVA | Evidence |
| --- | --- | --- | --- |
| Game HWND global | VA `0x00528018`, RVA `0x00128018` | **VA `0x00528118`, RVA `0x00128118`** | Multiple FR instructions read/write this pointer, including `CreateWindowExA` output at `0x004069E7` |
| Registered game WndProc | VA `0x00405F82`, RVA `0x00005F82` | **VA `0x0040604E`, RVA `0x0000604E`** | FR code at `0x0040714C` stores `0x0040604E` as the WNDCLASS procedure before `RegisterClassA` |
| Legacy 3dfx global | US RVA `0x0004727C` | **Still needs semantic verification** | FR references `0x0044727C`, but the intended flag semantics must be checked before use |

Hardcoded US addresses exist in at least:
- `tools/gameframe/patch_gameframe.py`
- `cdda/src/video_trace.rs`
- `cdda/src/glide_trace.rs` (historical diagnostics)
- `cdda/src/ddraw_native.rs` (hook eligibility / PE import checks)

Changing the executable SHA allowlist and offsets alone is **insufficient**. The GameFrame HWND access and native AVI host class verification would continue to use the US addresses and may refuse the FR HWND or fail to hand off the video. Keep the US code path unchanged and select strictly authenticated per-edition address profiles.

## Provisional resulting hashes, for subsequent tests only

Computed **in memory**, from the supplied file, by applying the candidate byte edits above. None has been played or validated against a complete French retail CD. Do **not** add these to executable acceptance allowlists until byte-level and field tests are complete.

| Candidate mode | SHA-256 (unvalidated) |
| --- | --- |
| No intro skip, original system WinMM | `b87ffbe50ebb1162f97fce0c203c288d0cfa8f74a0c1743c8121d04b2eddcf0e` |
| Skip intro, original system WinMM | `888d85ee215b2d902fce77fd67f18603db8598f207b358e64a911d7498fc0610` |
| No intro skip, project WinMM bridge | `8d70d1fb4b3be6e5a5e5cac132b3beafcc501482aaa9dd8217437fef72bb85f5` |
| Skip intro, project WinMM bridge | `66f25c59b314a916a02f215b1dde48b5e820bb8f2c130418258fd1e27ba1ac15` |

This table is a research trace only and does not authorize production patching.

## French CD CUE structure — supplied 2026-11-07

The user supplied `CYBERG.CUE`, a 966-byte ASCII/CRLF cue sheet with SHA-256 `5bfd895c9dc3b4d4d24ea2b42abee4f086e95b2c2d9bd415315fedea1fc285f0`.

- **One** `FILE "CYBERG.BIN" BINARY` declaration for all tracks. **No actual BIN supplied or verified yet.**
- **15 tracks**: `TRACK 01 MODE1/2352` at `INDEX 01 00:00:00`, then **14** `AUDIO` tracks numbered 02 through 15. The data-to-audio boundary is at sector **32,240** (`07:09:65`). The 2048-byte/sector normalized data ISO would therefore be **66,027,520 bytes**, assuming the BIN sectors validate.
- Track 02 has `PREGAP 00:02:00` and `INDEX 01 07:09:65`. Later audio tracks use separate `INDEX 00` (pregap) and `INDEX 01` (actual audio) offsets. The last track starts at sector **127,317** (`28:17:42`), and its duration cannot be determined without the BIN size.
- With sufficient valid BIN data, the current `cue::tracks`, `cue::normalize`, `cue::track_end` and `audio::extract` should be able to handle the layout and generate `music/Track02.flac` through `music/Track15.flac`. **This is parser compatibility, not proof of successful extraction.**
- The special `PREGAP` directive is accepted but ignored by our parser because it represents a virtual pregap not stored as data in `CYBERG.BIN`; for subsequent tracks the stored `INDEX 00` pregaps are intentionally excluded from FLAC audio by the `track_end` boundary calculation.
- The supplied `cyber.exe` cannot yet be cryptographically associated with `CYBERG.BIN`. Confirm its SHA-256 against `CYBER/CYBER.EXE` **extracted from the original data track**.

**Minimum needed to authenticate the FR edition:** the raw `CYBERG.BIN` in a controlled workspace, or a trustworthy extracted file manifest with SHA-256 digests for `CYBER/CYBER.EXE`, `CYBER/GRAPHICS/SHELL.FNT`, `CYBER/SPLASH.AVI` / `SPLASH4.AVI` if present. A simple CUE alone cannot prove the retail image or its media assets.

## Implementation plan, isolated from the US release

1. Obtain the **French CD `.cue`** plus the **disc asset manifest**. Determine track modes and whether the disc contains audio tracks; establish SHA-256 for `CYBER/CYBER.EXE`, `CYBER/GRAPHICS/SHELL.FNT`, `CYBER/SPLASH.AVI`, `CYBER/SPLASH4.AVI`, and any edition-specific resources. Never accept unverified files by language string alone.
2. Introduce per-edition `EditionProfile` data for original and patched hashes, patch preimages/offsets, import name slot, WndProc/HWND RVAs, and immutable resource fingerprints. Detect edition via **original EXE hash**, or verified original backup plus a known patched result hash, and propagate a validated edition ID to the x86 bridge.
3. Parameterize launcher patch/install/verify and the GameFrame/WinMM memory references; retain the current exact US profile as the default. Reject unknown editions without writing any files. Preserve French-only data, `IQ`, original backups and localized strings.
4. Unit-test both edition profiles, synthetic patch/reversal flows, malformed/unknown files, backup invariants and protected CUE paths. Keep any full retail binary and disc contents outside public Git.
5. Build a Windows pre-release candidate in GitHub Actions; test on an **isolated** French installation under Wine/Proton for full intro, 4:3 video, seamless OpenGL handoff, sound, speed, controls, no-CD, Alt+Tab, exit and backup restoration. Run US regression before merging.
6. Native Windows full-game and clean CUE/BIN acceptance tests still gate a public release.

**At this stage no FR runtime support has been enabled**, and `main` has not been changed.

legluondunet assisted by ChatGPT (OpenAI).
