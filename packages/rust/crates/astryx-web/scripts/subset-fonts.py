#!/usr/bin/env python3
"""Splits each Roboto face into the script subsets in fonts/ranges.txt.

    scripts/subset-fonts.py OUT_DIR

Writes OUT_DIR/<Face>.<subset>.ttf, for example Roboto-Bold.cyrillic.ttf.
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
FONTS = os.path.join(CRATE, "..", "..", "assets", "fonts")
FACES = ["Roboto-Regular", "Roboto-Medium", "Roboto-Bold"]

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
            name, ranges = line.split(" ", 1)
            codepoints = set()
            for part in ranges.split(","):
                part = part.strip().removeprefix("U+")
                lo, _, hi = part.partition("-")
                codepoints.update(range(int(lo, 16), int(hi or lo, 16) + 1))
            subsets.append((name, codepoints))
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
    for face in FACES:
        source = os.path.join(FONTS, f"{face}.ttf")
        for name, codepoints in read_ranges(os.path.join(CRATE, "fonts", "ranges.txt")):
            font = TTFont(source)
            subsetter = subset.Subsetter(options)
            subsetter.populate(unicodes=codepoints | set(SHARED))
            subsetter.subset(font)
            for record in font["name"].names:
                if record.nameID in (1, 16):
                    record.string = f"Roboto {name}"
            font.save(os.path.join(out, f"{face}.{name}.ttf"))


if __name__ == "__main__":
    main()
