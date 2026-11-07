#!/usr/bin/env python3
"""Checks a pixel palette from the design files and writes it to
tokens/pixel-palette.json, the one fixed palette every pixel theme and
sprite draws from.

    tools/pixel-tokens/export-palette.py <palette.json> [tokens/pixel-palette.json]

The input is an ordered object of swatch name to `#rrggbb`, or an object
with `name` and such `swatches`. Swatch names are lower-kebab-case. A
palette has 2 to 64 swatches, each name and color once. The design
session's palette is kept as `tools/pixel-tokens/palette.json`.
"""
import json
import pathlib
import re
import sys

NAME = re.compile(r"^[a-z][a-z0-9]*(-[a-z0-9]+)*$")
HEX = re.compile(r"^#[0-9a-fA-F]{6}$")


def fail(message):
    sys.exit(f"export-palette: {message}")


def main(argv):
    if len(argv) < 2:
        fail("usage: export-palette.py <palette.json> [out.json]")
    source = json.loads(pathlib.Path(argv[1]).read_text())
    if "swatches" not in source:
        source = {"name": "", "swatches": source}
    root = pathlib.Path(__file__).resolve().parents[2]
    out = pathlib.Path(argv[2]) if len(argv) > 2 else root / "tokens" / "pixel-palette.json"

    swatches = source.get("swatches")
    if not isinstance(swatches, dict) or not 2 <= len(swatches) <= 64:
        fail("`swatches` must be an object with 2 to 64 entries")
    seen = {}
    normalized = []
    for name, color in swatches.items():
        if not NAME.match(name):
            fail(f"swatch name {name!r} is not lower-kebab-case")
        if not isinstance(color, str) or not HEX.match(color):
            fail(f"swatch {name!r} is not #rrggbb: {color!r}")
        color = color.upper()
        if color in seen:
            fail(f"swatches {seen[color]!r} and {name!r} are both {color}")
        seen[color] = name
        normalized.append({"name": name, "rgb": color})

    result = {"name": source.get("name", ""), "swatches": normalized}
    out.write_text(json.dumps(result, indent=1) + "\n")
    print(f"export-palette: {len(normalized)} swatches to {out}")


if __name__ == "__main__":
    main(sys.argv)
