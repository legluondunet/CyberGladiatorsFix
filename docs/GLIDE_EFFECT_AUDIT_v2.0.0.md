# CyberGladiatorsFix v2.0.0 — Glide effects audit (before release)

**Status:** static source audit complete; **in-game effect-use inventory pending a bounded trace**. This is a diagnostic record, not permission to change the renderer or to publish a release.

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

## One bounded diagnostic run on Linux / Wine

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

Analyze the file using the standard-library-only tool, from a repository checkout:

```sh
python3 tools/gameframe/audit_glide_calls.py /path/to/CyberGladiatorsFix_glide_calls_YYYYMMDD-HHMMSS.txt
```

Supply both the resulting compact report, the original call trace, and the usual `CyberGladiatorsFix(...).log` for a full diagnosis. The report distinguishes actual observed calls and argument combinations; **an unobserved effect is not evidence that the game never uses it**. The original DLL log is required for `not handled`/stub summaries. The source script creates **no new runtime mode** and is not bundled into the public ZIP.

## Acceptance before any visual changes

1. Confirm presence or absence **within recorded frames** for bilinear/nearest, mipmaps and LOD, fog, alpha modes, chroma key, dithering, multi-TMU/detail and texture combiners.
2. For any mode approximated by GameFrame, check whether Cyber actually requests that exact value, and whether the visual result differs from native Voodoo/3dfx evidence.
3. If a real mismatch is confirmed, implement a narrowly scoped opt-in fix in a **separate PR** and compare screenshots/behavior on Windows and Wine. Do not disturb v2.0.0's validated AVI, audio, fullscreen, input or 60 FPS behavior.
4. The release-process PR #38 is separate. A diagnostic PR or green CI never authorizes a public v2.0.0 tag.
