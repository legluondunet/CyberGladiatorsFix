# French retail: non-exclusive adaptive borderless DirectDraw design

**Status: engineering design / static evidence; not an implemented display-mode
fix.** The default Windows FR build #168 still allows the original game to
change the real monitor to 640x480x16. Do not label the PR complete on the
basis of this document or the passive audit.

## User-visible invariant

The player's current monitor resolution, depth and refresh rate must remain
unchanged before, during, and after playing. The visible application is a
borderless, **non-exclusive** window filling the relevant monitor; the
dimensions must be discovered from the running system and honor DPI,
without 1920x1200 or other fixed desktop assumptions.

Both the MCI AVI and GameFrame/OpenGL content are presented without
stretching their 4:3 aspect ratio or changing Windows' real display mode.
The existing, functional first-present handoff, FLAC audio, input and
Alt+Tab must not regress. A restore *after* switching is not enough.

## Evidence in the original DirectDraw setup

The original FR executable is SHA-256 gated and inspected read-only by
`src/fr_ddraw_static.rs`. The x86 sites below were observed in the user's
Windows logs and are now checked as immutable instruction signatures
before reporting the dependency:

| x86 VA (FR original) | Operation | Consequence |
|---|---|---|
| `0x00405E7E` | `push 0x11`; v1 COM call at `0x00405E8E`, slot 20 / `+0x50` | `SetCooperativeLevel(DDSCL_FULLSCREEN|DDSCL_EXCLUSIVE)` |
| `0x004050AF` | ECX=16, EDX=480, EAX=640; call wrapper `0x00405EB7` | Requests `SetDisplayMode(640,480,16)` |
| `0x00406A68` | Second call through the same wrapper | May request same 640x480x16 mode |
| `0x00436411` | Pushes 16, 480, 640 and calls different COM object's `+0x54` | Additional candidate mode request; cannot ignore it |

The corresponding original US executable is independently available for
static comparison; its function around `0x00405E53` sets a
`DDSURFACEDESC` with:
`dwSize=0x6C`, `dwFlags=0x21`
(`DDSD_CAPS|DDSD_BACKBUFFERCOUNT`),
`dwBackBufferCount=1`, and
`ddsCaps.dwCaps=0x218`
(`DDSCAPS_PRIMARYSURFACE|DDSCAPS_FLIP|DDSCAPS_COMPLEX`),
then calls v1 `CreateSurface` at vtable `+0x18`.

This is an **exclusive-mode primary flipping chain** with a backbuffer,
not just a fullscreen window. The FR static analyzer now checks whether
the same complete descriptor and CreateSurface call occur in the verified
FR EXE, rather than assuming the US behavior is identical. The first
FR audit result showing `primary_flip_chain_candidates` will determine
whether that candidate is present.

The Windows DirectDraw contract makes the distinction important:
`DDSCL_NORMAL` cannot be combined with `DDSCL_EXCLUSIVE` or
`DDSCL_FULLSCREEN`. Native DirectDraw requires exclusive mode for
changing the physical display mode **or flipping the primary surface**.
Thus, changing only the `0x11` flags to `DDSCL_NORMAL` or returning
fake `DD_OK` from `SetDisplayMode` is **not sufficient**.

Microsoft reference:
- https://learn.microsoft.com/en-us/previous-versions/ms785063(v=vs.85)
- https://learn.microsoft.com/en-us/windows/win32/api/ddraw/ns-ddraw-ddscaps

## Required compatibility boundary

Design a project-owned, FR-specific **virtual DirectDraw v1 device**.
It must never let real system DirectDraw apply the game's exclusive
`SetCooperativeLevel` or `SetDisplayMode` calls:

1. Authenticate the exact French retail executable and the owned
   project component at startup. Keep the existing default implementation
   unchanged until the virtual device is fully testable.
2. Present a virtual v1 DirectDraw cooperative mode and
   `640x480x16` display mode to the legacy game. Native Windows remains
   in its prelaunch mode for the full session.
3. Intercept at least both SetDisplayMode paths, the cooperative-level
   request, `CreateSurface`, front/backbuffer attachment, and the
   surface Flip/Blt/Lock/Unlock/get-descriptor operations actually used.
   Implement only verified required methods, but **fail explicitly** for
   any unimplemented method; never return fake success.
4. Back the game's virtual 16-bpp primary + one backbuffer with
   compatible offscreen memory. Convert/present the completed frame to
   the GameFrame-controlled borderless OpenGL output while respecting
   4:3 scaling. The user's physical display stays 32-bpp (or whatever
   its original depth happens to be); 16 bpp is solely the game's
   logical surface format.
5. Keep all host/window metrics consistent. The virtual DirectDraw mode
   must not leak into the Win32 geometry used for MCI or GameFrame;
   distinguish `GetSystemMetrics` (potentially DPI virtualized) from
   `EnumDisplaySettings` (physical mode).
6. Use transactional setup and cleanup. Never replace a user's unknown
   `ddraw.dll`; restore verified original DLL state and game settings
   after normal or interrupted runs. Preserve US and Wine/Proton paths.
7. Create unit tests for virtual mode state, 16-bpp surface buffers and
   flip operations, plus a Windows-native opt-in trial with explicit
   diagnostics before enabling by default. The trial must fail closed
   without leaving Windows in an altered mode.

**Do not use** the old native vtable hook (#124), the native 32-bit
constant-substitution experiment (#158), returning `DD_OK` without
DirectDraw surface state, or external cnc-ddraw; all were unvalidated or
failed in real Windows FR testing.

## Measurement and acceptance

The Windows FR launcher now has an independent **passive**
`FR_MODE_WATCH` in the parent process, sampling
`EnumDisplaySettings(ENUM_CURRENT_SETTINGS)` every 50 ms and recording
transitions in the **one** session log, without writing any display
settings. This supplements the existing before/after snapshots; because
sampling is not continuous ETW capture, an extremely short change could
go unnoticed.

The completed fix requires:
- The physical current display mode remains identical before launch,
  throughout intro, gameplay, Alt+Tab and after exit.
- The actual game window is non-exclusive, undecorated and monitor-sized
  for multiple user resolutions/DPI scales.
- 4:3 AVI and gameplay plus CDDA work, no blank frame between them,
  around 60 FPS and clean closure.
- Windows FR, US and Wine/Proton regression coverage.

**Engineering decision**: until the virtual primary/flip-chain is
implemented and exercised, stop short of any automatic COM or mode
modification. The #168 graphical path remains the fallback.

legluondunet assisted by ChatGPT (OpenAI)
