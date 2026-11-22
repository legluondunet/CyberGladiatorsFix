# GameFrame glide.dll ownership and replacement policy — v2.0.0

**Current policy:** CyberGladiatorsFix v2.0.0 uses the verified, embedded
GameFrame Glide wrapper and an installer-owned `fix.dll` WinMM/CDDA bridge
beside `game/cyber.exe`. Unlike the historical v1.x behavior, **v2 never
creates a persistent `glide.dll.bak` or `fix.dll.bak` when replacing an
installer-owned runtime DLL**. The older archival/rename policy remains
available in Git history; it is not a v2.0.0 installation instruction.

## Verified ownership

The runtime DLL must be a regular, non-symlinked x86 PE32 DLL, and its
actual SHA-256 must match the matching ownership receipt in `state/`:

- `state/CyberGladiatorsFix.gameframe.sha256` for `game/glide.dll`
- `state/CyberGladiatorsFix.sha256` for `game/fix.dll`

The launcher verifies its embedded DLL bytes and source-pinned GameFrame
hash independently. It **never trusts a receipt alone** to approve
unrecognized installed bytes.

## Installation and subsequent launches

| On-disk runtime DLL | Behavior |
| --- | --- |
| Absent, with no ownership receipt | Install verified embedded DLL and record its ownership SHA-256 |
| Present and matches both its receipt and the current embedded DLL | Reuse without changing it |
| Present and matches its receipt, but differs from the current verified embedded DLL | Stage, verify and replace the owned DLL; update receipt, with no permanent backup |
| Present but malformed, unknown, symlinked, or not matching its receipt | Refuse replacement rather than overwriting a foreign file |
| Absent but ownership receipt exists | Refuse silent repair; investigate inconsistent installation |
| DLL path is a directory or a staging/retirement path is occupied | Refuse unsafe replacement |

Replacement uses staging/retirement temporary files and attempts rollback
on failures. The temporary retired copy is removed after successful
activation; it is **not** a persistent `.bak` archive. Failed rollback
or cleanup can require manual inspection instead of blind retries.
Pre-existing legacy `glide.dll.bak` files are not consumed as proof of
ownership and are not silently migrated into the v2 layout.

The verified bundled GameFrame remains installed after the game exits.
`--verify` checks the runtime hashes/receipts without repairing them;
`--launch-no-verify` deliberately skips installation and integrity
preflight, and must not be used to repair an inconsistent game.

## Important exception: original retail executable

`game/CYBER.EXE.bak` is **not** a runtime DLL backup. It is the required
hash-verified copy of the owner's original retail game executable, kept
for authentication and safe patching. It is intentionally retained and
never treated as disposable installation staging.

See `src/gameframe.rs`, `src/cdda.rs`, `src/layout.rs` and the
[active v2.0.0 release notes](RELEASE_NOTES_v2.0.0.md) for the
implementation and compatibility scope.
