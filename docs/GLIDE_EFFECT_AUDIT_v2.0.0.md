# CyberGladiatorsFix v2.0.0 — Glide effects audit (before release)

**Status:** static source audit and **one bounded French Wine in-game trace completed (23 November 2026)**; broader levels and visual reference comparisons remain untested. This is a diagnostic record, not permission to change the renderer or to publish a release.

## Evidence and exact code

- GameFrame source pinned by `.github/workflows/gameframe-glide.yml` to `tanepiper/game-frame@f195a074a63cc150673c38becdec696163b0b3d1`. It is modified only by the project-specific integration patches at build time. The inspected render-state and texture implementation is upstream `wrappers/glide/src/{api,render}`.
- **Wine-staging test, 23 November 2026**, CyberGladiatorsFix `2.0.0-2da9ee37` (French edition): Mesa 26.2.4 / AMD Radeon RX 6600M, OpenGL 4.6; desktop 1920×1080, logical Glide 640×480, 4:3 output 1440×1080. AVI handoff, FLAC music, ~59 FPS gameplay and exit 0 confirmed by the supplied runtime log. It also says `glide: every call this game made is implemented`.
- That last sentence comes from the **shim's missing-call / unsupported-case counter**. It does **not** prove that emulated effects look exactly like original hardware. Some simplifications are deliberate and do not count as a `shim::gap!`.

## Feature-level source review

| Effect | Upstream implementation | Fidelity qualification |
| --- | --- | --- |
| Texture minification/magnification | `grTexFilterMode` uses OpenGL nearest/linear | Supported for TMU 0; whether Cyber requests bilinear must be traced |
| Mipmaps, LOD bias, trilinear | `grTexMipMapMode`, `grTexLodBiasValue`, OpenGL mipmap filtering | Nearest-dither mipmaps render without dithering |
| Fog | `grFogMode`, fog table/color, shader | Implemented; actual usage unknown without trace |
| Alpha blending / test | `grAlphaBlendFunction`, `grAlphaTest*`, shader | Most common states implemented; some alpha local-depth / control states approximate |
| Chroma key | `grChromakey*`, shader discard | Implemented; actual usage unknown without trace |
| Z/W depth | `grDepthBuffer*`, render state/depth buffer | Implemented with documented precision/bias differences from Voodoo |
| Voodoo 16-bit dithering | `grDitherMode` | **Not reproduced**: render target has RGBA8 channels |
| Detail texture / secondary TMU | `grTexDetailControl`, TMU code | **Ignored / single-TMU assumption** |
| General texture combiner | `grTexCombine` | Some cases are mapped to simple decal; conditional gap warnings |
| 2D framebuffer scaling | `GLIDE_LFB_FILTER` in GameFrame `config.rs` | Default `nearest`; opt-in `linear` changes 2D sharpness, not the game's 3D texel filter |

None of the above establishes that Cyber Gladiators **asks for** the approximated modes in a representative level. Do not introduce new filtering, anisotropy, post-processing or dither hacks until a concrete difference is demonstrated.

## Recorded in-game effects — original FR edition under Wine, 23 November 2026

The user supplied the 15 MB `CyberGladiatorsFix_glide_calls_20261123-192121.txt.zip` archive (a 179 MB raw trace) and `CyberGladiatorsFix(20261123-182254).log`. **Only aggregate, non-proprietary observations are recorded here; the original archive and game data are not committed.**

The exact `2.0.0-2da9ee37` launcher logged **994,456 valid Glide calls**, **22 distinct exported functions**, **901 `grBufferSwap` calls / frames 0–900**, with menu and early gameplay in this sampled range. The complete ~83-second session continued beyond the trace; runtime reported approximately 59 FPS during active play and a clean exit (code 0). No unhandled or `shim::gap!` warning was recorded.

| Observed action or state | Calls | Parameters and interpretation |
| --- | ---: | --- |
| `guDrawTriangleWithClip` | 562,344 | Geometry drawn in Glide, including the gameplay sample |
| `guTexAllocateMemory` | 273 | **All**: 256×256, texture format `0xb`, `min_filter=1`, `mag_filter=1` → **bilinear/linear**; `mipmap_mode=0` → **no mipmapping**; `trilinear=0`; LOD bias `0.0` |
| `guTexSource` | 24,064 | Activates allocated textures; internally reapplies their filters, mipmap mode and LOD; the game need not call `grTexFilterMode` itself |
| `guTexDownloadMipMap` | 486 | Texture uploads through the historical named API; does **not** imply mipmapping is enabled |
| `grTexCombineFunction` | 868 | Only `(TMU=0, mode=1)`: simple **decal** combine |
| `guColorCombineFunction` | 1,735 | `0x6` (868 calls, texture × iterated RGB) and `0x2` (867 calls, iterated RGB alone) |
| `guAlphaSource` | 1,735 | `0x3` (868 calls, texture alpha × iterated alpha) and `0x1` (867 calls, iterated alpha) |
| `grAlphaBlendFunction` | 1 | `(0x1,0x5,0x4,0x0)`: RGB source alpha / one-minus-source-alpha, alpha ONE / ZERO |
| `grChromakeyMode` | 395,547 | **5,950** enable (`1`), **389,597** disable (`0`) |
| `grChromakeyValue` | 5,950 | Key colour `0` (black); implemented in GameFrame shader |
| `grGammaCorrectionValue` | 1 | **Gamma 1.6**, mapped by GameFrame's final presentation shader |

**Not requested in this 0–900 frame sample:** explicit `grFogMode`, `grDitherMode`, `grDepthBufferMode`, `grTexDetailControl`, general `grTexCombine`, `grTexMipMapMode`, LFB filtering or a secondary TMU. Explicit mipmap calls are also normally internal to `guTexSource`; its source allocation mode `0` confirms mipmapping is off for these sampled textures. GameFrame's default depth mode is disabled, but the absence of a state call is **not** a proof that every game scene is depth-free.

**Interpretation:** all explicitly requested filter/combine/alpha/chroma states in this trace are implemented by GameFrame's pinned code. The known approximations for Voodoo dithering, trilinear mipmaps and second-TMU detail textures **were not exercised by observed requests**. This is positive evidence for the sampled graphics path, not a screenshot-level certification of all original 3dfx effects in every level. The 256×256 source textures and original 640×480 rendering limit source detail; the 1440×1080 upscale cannot recreate absent texture detail.

**Current recommendation:** retain the game's requested bilinear filtering and the validated renderer unchanged for v2.0.0. Do **not** force anisotropic/trilinear processing, enable dithering or change `GLIDE_LFB_FILTER` by default on this evidence alone.

## Reproducing a bounded diagnostic run on Linux / Wine

GameFrame already offers `WRAPPER_CALLS` and `WRAPPER_CALLS_FRAMES`, implemented in pinned upstream `wrappers/shim/src/calls.rs`. The one-click launcher inherits these variables for its **game child**; `src/verify.rs` does not remove them. Its normal command-line options, integrity verification, renderer, INI and output logs stay unchanged.

In the directory containing **the installed `CyberGladiatorsFix.exe`**, run this under the **same Wine prefix as your tested installation**:

```sh
cd /path/to/CyberGladiatorsFix-installation
trace_file="$PWD/CyberGladiatorsFix_glide_calls_$(date +%Y%m%d-%H%M%S).txt"
WRAPPER_CALLS="$(winepath -w "$trace_file")" \
WRAPPER_CALLS_FRAMES=0-900 \
wine ./CyberGladiatorsFix.exe
```

Start a representative fight when the menu appears and exit normally. The first 900 frames are an initial, **bounded** sample chosen to include the menu and early play in the 23 November log; a different session may spend more time in the intro or menu. If very large, the text trace can be compressed before sharing. Tracing is intrusive and may lower FPS; **never use the trace run to benchmark speed**. The captured calls may include game asset sizes/hashes; review before publishing.

Analyze the file **or its ZIP directly** (streamed; no 179 MB extraction) using the standard-library-only tool, from a repository checkout:

```sh
python3 tools/gameframe/audit_glide_calls.py /path/to/CyberGladiatorsFix_glide_calls_YYYYMMDD-HHMMSS.txt.zip
```

Supply both the resulting compact report, the original call trace, and the usual `CyberGladiatorsFix(...).log` for a full diagnosis. The report distinguishes actual observed calls and argument combinations; **an unobserved effect is not evidence that the game never uses it**. The original DLL log is required for `not handled`/stub summaries. The source script creates **no new runtime mode** and is not bundled into the public ZIP.

## Acceptance before any visual changes

1. **Completed for FR frames 0–900:** actual bilinear, no requested mipmapping/trilinear/dither/fog/detail, alpha/combine/chroma and gamma states inventoried; no unsupported-call warning. Broader gameplay scenes not yet recorded.
2. For any mode approximated by GameFrame, check whether Cyber actually requests that exact value, and whether the visual result differs from native Voodoo/3dfx evidence.
3. If a real mismatch is confirmed, implement a narrowly scoped opt-in fix in a **separate PR** and compare screenshots/behavior on Windows and Wine. Do not disturb v2.0.0's validated AVI, audio, fullscreen, input or 60 FPS behavior.
4. The release-process PR #38 is separate. A diagnostic PR or green CI never authorizes a public v2.0.0 tag.
