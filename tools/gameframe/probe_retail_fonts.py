#!/usr/bin/env python3
"""Read-only Cyber Gladiators original font/VAM format inventory.

No modifications are made to retail files. Writes a bounded textual report
only to stdout (redirect it outside the game directory if desired).
This is a *format discovery* probe, not a decoder: no unrelated .FNT spec is
assumed. It can optionally make preview PNGs for .VAM images if Pillow exists.
"""
from __future__ import annotations
import argparse
import hashlib
import io
import re
import struct
from collections import Counter
from pathlib import Path

NAMES = ("shell.fnt", "font.fnt", "font2.fnt",
         "font.vam", "font2.vam",
         "shadowg.vam", "shadow.vam", "haira.vam",
         "hair2a.vam", "hair2.vam", "hair.vam")
RETAIL_SHA = {
    "fr": "45d2241819d32332d736f01f06b4cff694677edf1fba27588ffa8a9ff0a5d177",
    "us": "d0a02e7aebd5104754debdca610bc7e94a2d88c7b97452c64a211fa3fbf477a7",
}

def hx(data: bytes, offset: int = 0) -> str:
    lines=[]
    for i in range(0,len(data),16):
        chunk=data[i:i+16]
        lines.append(f"{offset+i:08x}  {chunk.hex(' ',1):47s}  "
                     + "".join(chr(c) if 32<=c<127 else "." for c in chunk))
    return "\n".join(lines)

def magic(data: bytes) -> str:
    lead=data[:16]
    return " ".join(f"{b:02x}" for b in lead)

def bounded_ascii(data: bytes, limit=12) -> str:
    parts=[]
    for m in re.finditer(rb"[\x20-\x7e]{5,}",data):
        v=m.group().decode("ascii","replace")
        parts.append(f"{m.start():08x}: {v[:96]!r}")
        if len(parts)>=limit:break
    return "; ".join(parts) or "(none)"

def bmp(data: bytes, name: str) -> None:
    if len(data)<54 or data[:2]!=b"BM":
        print("BMP_INFO absent (not a standard BMP header)")
        return
    try:
        declared,off=struct.unpack_from("<IxxxxI",data,2)
        dib=struct.unpack_from("<I",data,14)[0]
        if dib>=40 and len(data)>=54:
            width,height,planes,bpp,compression,size=struct.unpack_from("<iiHHII",data,18)
            print("BMP_INFO",f"size={declared} raster_off={off} dib={dib}",
                  f"width={width} height={height} planes={planes} bits={bpp}",
                  f"compression={compression} bitmap_size={size}")
        else:
            print("BMP_INFO",f"size={declared} raster_off={off} dib={dib}")
    except struct.error as e:
        print("BMP_INFO parse-error:",str(e))

def basic_probe(name:str,data:bytes,full_hex:bool) -> None:
    print(f"\n===== {name} =====")
    print("BYTES",len(data),"SHA256",hashlib.sha256(data).hexdigest())
    if name.lower()=="shell.fnt":
        sha=hashlib.sha256(data).hexdigest()
        print("KNOWN_SHELL_SHA",next((edition for edition,digest in RETAIL_SHA.items()
                if digest==sha),"unknown; no format assumed"))
    print("HEADER",magic(data))
    print("FIRST_256_HEX:\n"+hx(data[:256]))
    print("LAST_64_HEX:\n"+hx(data[-64:],max(0,len(data)-64)))
    print("FREQUENT_BYTES",Counter(data).most_common(12))
    print("PRINTABLE_ASCII",bounded_ascii(data))
    if len(data)>=128:
        print("FIRST_32_U16_LE",list(struct.unpack_from("<32H",data)))
        print("FIRST_16_U32_LE",list(struct.unpack_from("<16I",data)))
    if name.lower().endswith(".vam"):bmp(data,name)
    if full_hex and name.lower() in ("font.fnt","font2.fnt") and len(data)<8192:
        print("COMPLETE_SMALL_FONT_HEX_BEGIN")
        print(hx(data))
        print("COMPLETE_SMALL_FONT_HEX_END")
    if name.lower()=="shell.fnt":
        for sig in (b"BM",b"RIFF",b"FONT",b"font",b"FNT",b"fnt"):
            found=[m.start() for m in re.finditer(re.escape(sig),data)]
            print(f"SHELL_MARKER_{sig!r}",found[:25],f"total={len(found)}")

def locate(folder:Path|None)->Path:
    if folder is not None:
        if not folder.is_dir():
            raise ValueError("Specified directory does not exist")
        return folder
    for p in (
        Path.cwd()/"CyberGladiatorsFix"/"CYBER"/"GRAPHICS",
        Path.cwd()/"CYBER"/"GRAPHICS",
        Path.cwd()/"GRAPHICS",
        Path.cwd(),
    ):
        if (p/"shell.fnt").exists() or (p/"font.fnt").exists():
            return p
    raise ValueError("Cannot find GRAPHICS folder; pass --graphics /path/to/CYBER/GRAPHICS")

def main()->None:
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--graphics",type=Path,help="Original game's CYBER/GRAPHICS directory")
    parser.add_argument("--brief",action="store_true",help="Omit small FNT complete hex dumps")
    parser.add_argument("--preview-dir",type=Path,
        help="Optional directory OUTSIDE original game tree for PNG previews of VAM files (requires Pillow)")
    args=parser.parse_args()
    base=locate(args.graphics)
    print("CYBER_RETAIL_FONT_PROBE_V1 read_only=1 game_files_modified=0")
    print("FOUND_NAMES",", ".join(x.name for x in base.iterdir() if x.name.lower() in NAMES))
    names={x.name.lower():x for x in base.iterdir() if x.is_file()}
    for name in NAMES:
        p=names.get(name)
        if p is None:
            print(f"\n===== {name}: NOT PRESENT =====")
            continue
        data=p.read_bytes()
        basic_probe(name,data,not args.brief)
        if args.preview_dir and name.endswith(".vam"):
            try:
                from PIL import Image
                with Image.open(io.BytesIO(data)) as image:
                    image.load()
                    if image.width>4096 or image.height>4096:
                        raise ValueError("image dimensions too large")
                    dest=args.preview_dir
                    if dest.resolve()==base.resolve() or base.resolve() in dest.resolve().parents:
                        raise ValueError("preview path must be outside the original game directory")
                    dest.mkdir(parents=True,exist_ok=True)
                    target=dest / (name+".png")
                    image.convert("RGBA").save(target)
                    print("PREVIEW",target.name,f"pixels={image.width}x{image.height}")
            except (ImportError,ValueError,OSError) as e:
                print("PREVIEW_ERROR",type(e).__name__,str(e))
    print("\nPROBE_COMPLETE read_only=1 retail_files_changed=0")

if __name__=="__main__":
    try:
        main()
    except (OSError,ValueError) as exc:
        raise SystemExit(f"Font probe failed: {exc}")
