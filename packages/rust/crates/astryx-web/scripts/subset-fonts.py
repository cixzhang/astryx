#!/usr/bin/env python3
"""Splits each source family into the script subsets in fonts/ranges.txt.

    scripts/subset-fonts.py OUT_DIR

Writes OUT_DIR/<subset>-<weight>.ttf, for example cyrillic-700.ttf, for the
weights 400, 500, and 700. Roboto (Apache 2.0) comes from assets/fonts; Noto
Sans Arabic and Hebrew (SIL OFL 1.1) from crates/astryx-fonts/fonts.
Needs fontTools. Hinting is dropped because the renderer rasterizes without
it; all OpenType layout features stay so shaping matches the full font.

Each subset gets its own family name ("Roboto cyrillic"). The renderer picks
one face per family and weight, and falls back across families per
character, so subsets of one weight must not share a family.
"""

import os
import sys

from fontTools import subset
from fontTools.ttLib import TTFont

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(HERE)
ROOT = os.path.join(CRATE, "..", "..")
SOURCES = {
    "Roboto": os.path.join(ROOT, "assets", "fonts", "Roboto"),
    "NotoSansArabic": os.path.join(ROOT, "crates", "astryx-fonts", "fonts", "NotoSansArabic"),
    "NotoSansHebrew": os.path.join(ROOT, "crates", "astryx-fonts", "fonts", "NotoSansHebrew"),
}
WEIGHTS = {400: "Regular", 500: "Medium", 700: "Bold"}

# Spaces stay in the face of the run around them, so every subset carries
# them. They do not decide which subset loads.
SHARED = [0x20, 0xA0, 0xFEFF, *range(0x2000, 0x200C), 0x202F, 0x205F]


def read_ranges(path):
    subsets = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            name, source, ranges = line.split(" ", 2)
            codepoints = set()
            for part in ranges.split(","):
                part = part.strip().removeprefix("U+")
                lo, _, hi = part.partition("-")
                codepoints.update(range(int(lo, 16), int(hi or lo, 16) + 1))
            subsets.append((name, source, codepoints))
    return subsets


def main():
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)
    options = subset.Options()
    options.layout_features = ["*"]
    options.hinting = False
    options.desubroutinize = True
    options.notdef_outline = True
    options.name_IDs = ["*"]
    for name, source, codepoints in read_ranges(os.path.join(CRATE, "fonts", "ranges.txt")):
        for weight, style in WEIGHTS.items():
            font = TTFont(f"{SOURCES[source]}-{style}.ttf")
            subsetter = subset.Subsetter(options)
            subsetter.populate(unicodes=codepoints | set(SHARED))
            subsetter.subset(font)
            for record in font["name"].names:
                if record.nameID in (1, 16):
                    record.string = f"{source} {name}"
            font.save(os.path.join(out, f"{name}-{weight}.ttf"))

if __name__ == "__main__":
    main()
