# GameFrame glide.dll backup policy

The standard one-click launcher (US/FR, Windows/Wine) always uses the
**SHA-256-verified, embedded GameFrame Glide wrapper**. It does not load or
silently run an unknown older `glide.dll` from the installed game folder.

## Launch cases

| Installed `glide.dll` | Launcher action | After game exits |
| --- | --- | --- |
| Missing | Install bundled GameFrame | GameFrame remains installed |
| Identical to bundled GameFrame | Reuse without modification | GameFrame remains installed |
| Older, unknown, malformed, oversized or unreadable file | Rename original to an unused `.bak` without requiring a valid DLL structure; install bundled GameFrame | GameFrame remains installed; backup stays intact |
| Symbolic link named `glide.dll` | Move the **link itself** to `.bak`, without following its target; install GameFrame | Link backup and its original target stay untouched |
| Directory named `glide.dll` | Refuse, since a directory is not a DLL file | Existing directory unchanged |

The first archive is **`glide.dll.bak`**. If this already exists,
the launcher preserves it and selects
`glide.dll.<first 16 hexadecimal digits of SHA-256>.bak`.
If that also exists, it tries `glide.dll.<hash16>.1.bak`, then
`.2.bak`, etc., within a bounded number of attempts. If the old file
cannot be read or is a symbolic link, `unknown` replaces `<hash16>` in
the alternative backup name. Existing archives
are **never overwritten**. Legacy backup names from older builds remain
untouched; no old backup is implicitly considered the active DLL.

The launcher stages the bundled DLL in a separate file, validates its
Windows x86 PE32 format and SHA-256, checks the installed file again
without enforcing a file-size or old-PE32 restriction, and performs
non-replacing moves (Windows
`MoveFileW`, also supported under Wine). If staging/archiving/activation
fails, it attempts to roll back the original while keeping it recoverable.
If the rollback itself fails, the preserved original remains in the
named backup for manual recovery.

**Log events:**
- `GAMEFRAME_GLIDE_BACKUP`: archive path, original SHA-256 when readable
  (otherwise `unreadable_or_link`), collision
  fallback decision, and confirmation that existing backups were not touched
- `GAMEFRAME_GLIDE_POLICY`: action
  `replace_unknown_or_old`, `install_missing`, or `keep_existing`;
  embedded SHA-256 and keep-on-exit policy
- `GAMEFRAME_GLIDE_FINISH`: embedded wrapper remains active, backup is
  retained, and no automatic restoration takes place
- `GAMEFRAME_GLIDE_FINISH_ERROR`: if the active DLL changed during gameplay,
  leave both original and new files untouched and request inspection

**Manual recovery:** exit the game and launcher, then move the current
`glide.dll` somewhere safe and rename the desired archived `.bak`
back to `glide.dll`. Do not delete an unknown existing `.bak` and
do not copy files over it. On the next one-click launch,
CyberGladiatorsFix may archive this restored DLL again because the
embedded GameFrame renderer is mandatory for the supported launch path.

This policy covers renderer files only; the retail game's authenticated
`CYBER.EXE.bak`, immutable game data, CUE/BIN and CDDA audio are untouched.
The only supported and tested default renderer is the bundled GameFrame;
legacy nGlide/DXVK experiment commands are separate and opt-in.

legluondunet assisted by ChatGPT (OpenAI)
