#!/usr/bin/env python3
"""Summarize GameFrame WRAPPER_CALLS from one bounded Glide trace.

Standard library only. This inspects calls that the original game made;
it does not assert visual equivalence or change the renderer.
"""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
from pathlib import Path
import re

CALL = re.compile(r"^(gr[A-Za-z0-9_]+|gu[A-Za-z0-9_]+)\((.*)\)\tframe ([0-9]+)\s*$")
EFFECTS = {
    "Texture filtering and scaling": (
        "grTexFilterMode", "grTexClampMode", "grTexLodBiasValue",
        "grTexMipMapMode", "guTexAllocateMemory", "guTexChangeAttributes",
        "guTexSource", "grTexSource",
    ),
    "Texture combine and detail": (
        "grTexCombine", "grTexCombineFunction", "guTexCombineFunction",
        "grTexDetailControl",
    ),
    "Alpha, transparency and color": (
        "grAlphaBlendFunction", "grAlphaCombine", "guAlphaSource",
        "grAlphaControlsITRGBLighting", "grAlphaTestFunction",
        "grAlphaTestReferenceValue", "grChromakeyMode",
        "grChromakeyValue", "grColorCombine", "guColorCombineFunction",
        "grColorMask", "grConstantColorValue",
    ),
    "Fog and depth": (
        "grFogMode", "grFogColorValue", "grFogTable",
        "guFogGenerateExp", "guFogGenerateExp2", "guFogGenerateLinear",
        "grDepthBufferMode", "grDepthBufferFunction",
        "grDepthMask", "grDepthBiasLevel",
    ),
    "Dithering and 2D framebuffer": (
        "grDitherMode", "grLfbLock", "grLfbUnlock",
        "grLfbWriteMode", "grLfbGetWritePtr", "grLfbWriteRegion",
        "grLfbReadRegion", "grLfbConstantDepth",
    ),
}
TARGETS = {name for names in EFFECTS.values() for name in names}
CAUTIONS = {
    "grDitherMode": "GameFrame records the requested dithering state, but does not reproduce the original 16-bit dither pattern.",
    "grTexMipMapMode": "Mipmap-nearest-dither is approximated without dithering; check the observed mode.",
    "grTexDetailControl": "GameFrame ignores second-TMU detail-texture control.",
    "grTexCombine": "Some general texture-combiner modes are approximated; compare requested parameters.",
    "grAlphaControlsITRGBLighting": "GameFrame does not implement enabled iterated-alpha-controlled lighting.",
    "grAlphaCombine": "LOCAL_DEPTH alpha is approximated as iterated alpha when requested.",
    "grLfbWriteMode": "Some LFB writing modes are not handled.",
    "grLfbGetWritePtr": "Only the back-buffer pointer is handled.",
}
MAX_VARIANTS = 12


def read_trace(path: Path):
    totals: Counter[str] = Counter()
    modes: dict[str, Counter[str]] = defaultdict(Counter)
    frames: set[int] = set()
    matched = 0
    with path.open("r", encoding="utf-8", errors="replace") as source:
        for raw in source:
            result = CALL.match(raw.rstrip("\r\n"))
            if result is None:
                continue
            name, args, frame = result.groups()
            matched += 1
            totals[name] += 1
            frames.add(int(frame))
            if name in TARGETS:
                modes[name][args] += 1
    return matched, frames, totals, modes


def report(path: Path) -> int:
    matched, frames, totals, modes = read_trace(path)
    print("# CyberGladiatorsFix — recorded Glide effects")
    print(f"Source: {path.name}")
    if not matched:
        print("No GameFrame Glide calls found. Confirm WRAPPER_CALLS is enabled, "
              "the Wine path is writable, and the game reached the captured frame range.")
        return 2
    print(f"Recorded calls: {matched}; distinct functions: {len(totals)}; "
          f"recorded frames: {min(frames)}–{max(frames)} ({len(frames)} frames).")
    print("Scope: only the captured frame range; no claim about other levels or "
          "visual fidelity.")
    for heading, names in EFFECTS.items():
        print(f"\n## {heading}")
        found = False
        for name in names:
            if name not in totals:
                continue
            found = True
            print(f"- {name}: {totals[name]} call(s)")
            for args, count in modes[name].most_common(MAX_VARIANTS):
                compact = args if len(args) <= 190 else args[:187] + "..."
                print(f"    {count} x ({compact})")
            if len(modes[name]) > MAX_VARIANTS:
                print(f"    ... {len(modes[name]) - MAX_VARIANTS} more parameter variants")
            if name in CAUTIONS:
                print(f"    REVIEW: {CAUTIONS[name]}")
        if not found:
            print("- No calls observed in this frame range.")
    other = [name for name in totals if name not in TARGETS]
    print(f"\nOther recorded Glide/gu functions: {len(other)} "
          "(not treated as missing effects).")
    print("Limitations: state calls invoked internally by GameFrame's gu* "
          "helpers are not separately recorded; inspect the gu* arguments. "
          "The ordinary Glide log is still needed for 'not handled' warnings.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("trace", type=Path, help="WRAPPER_CALLS .txt file from one Wine game run")
    args = parser.parse_args()
    try:
        return report(args.trace)
    except (OSError, UnicodeError) as error:
        parser.error(str(error))
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
