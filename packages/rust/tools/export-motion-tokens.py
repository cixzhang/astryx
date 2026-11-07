"""Export Astryx motion tokens for the Rust experiment.

Usage: python3 tools/export-motion-tokens.py <astryx-repo> > tokens/motion.json

The Swift theme generator leaves motion out, so `export-tokens.py` cannot
see it. This script reads Astryx core's own sources instead:

- packages/core/src/theme/tokens.stylex.ts: `durationDefaults`, `easeDefaults`
  (the default theme's values, spec:AST-108 FR2)
- packages/core/src/theme/expandMotionScale.ts's formula, applied to each
  theme's `motion: {fast, medium, slow, ratio, easing?}` in
  packages/themes/<name>/src/<name>Theme.ts (spec:AST-108 FR5)
- packages/core/src/theme/motionKnobs.ts: `TEMPO_RANGE`
- packages/core/src/theme/themeMotion.ts: `CHARACTER_EASE`
- packages/core/src/motion/uikit/timing.ts: the Reduce Motion cross-fade and
  the press clocks
- packages/core/src/motion/uikit/longPress.ts: the long-press numbers
- docs/specs/AST-094-touch-presentations/motion-roles.md (generated from
  core's motion tables): the springs spec:AST-094 FR4 names
"""
import json
import math
import pathlib
import re
import sys

ROOT = pathlib.Path(sys.argv[1])
CORE = ROOT / "packages/core/src"
THEMES = ["astryx", "neutral", "stone", "butter", "chocolate", "gothic", "matcha", "y2k"]


def read(rel):
    return (ROOT / rel).read_text()


def block(text, name):
    """The `{...}` body of `export const <name> = {`."""
    m = re.search(r"export const " + re.escape(name) + r"\s*=\s*\{(.*?)\n\}", text, re.S)
    if not m:
        sys.exit(f"cannot find {name}")
    return m.group(1)


def bezier(text):
    m = re.fullmatch(r"cubic-bezier\(([^)]*)\)", text.strip())
    if not m:
        sys.exit(f"not a cubic-bezier: {text}")
    return [float(x) for x in m.group(1).split(",")]


def js_round(x):
    # JavaScript's Math.round: halves round up.
    return math.floor(x + 0.5)


def round_ms(ms):
    # expandMotionScale's roundMs: the nearest 5 ms.
    return js_round(ms / 5) * 5


tokens_ts = read("packages/core/src/theme/tokens.stylex.ts")
durations = dict(re.findall(r"'--duration-([a-z-]+)':\s*'([^']+)'", block(tokens_ts, "durationDefaults")))
eases = dict(re.findall(r"'--ease-([a-z-]+)':\s*'([^']+)'", block(tokens_ts, "easeDefaults")))

knobs_ts = read("packages/core/src/theme/motionKnobs.ts")
tempo = re.search(r"TEMPO_RANGE\s*=\s*\{min:\s*([\d.]+),\s*max:\s*([\d.]+)\}", knobs_ts)
theme_motion_ts = read("packages/core/src/theme/themeMotion.ts")
character = dict(re.findall(r"(crisp|expressive):\s*'([^']+)'", block(theme_motion_ts, "CHARACTER_EASE")))

timing_ts = read("packages/core/src/motion/uikit/timing.ts")
fade = block(timing_ts, "reducedMotionCrossFade")
press = block(timing_ts, "press")
long_press_ts = read("packages/core/src/motion/uikit/longPress.ts")


def number(text, name):
    m = re.search(name + r"\s*[:=]\s*([\d.]+)", text)
    if not m:
        sys.exit(f"cannot find {name}")
    return float(m.group(1))


def curve(text, name):
    m = re.search(name + r":\s*\[([^\]]*)\]", text)
    return [float(x) for x in m.group(1).split(",")]


def ms(value, table):
    if value.startswith("var(--duration-"):
        return ms(table[value[len("var(--duration-"):-1]], table)
    return float(value.removesuffix("ms"))


def theme_entry(name):
    table = dict(durations)
    config = {}
    if name != "astryx":
        src = read(f"packages/themes/{name}/src/{name}Theme.ts")
        m = re.search(r"\n\s*motion:\s*\{([^}]*)\}", src)
        if m:
            config = dict(re.findall(r"(\w+):\s*('[^']*'|[\d.]+)", m.group(1)))
    if {"fast", "medium", "ratio"} <= config.keys():
        fast, medium, ratio = (float(config[k]) for k in ("fast", "medium", "ratio"))
        table.update({
            "fast-min": f"{round_ms(fast * ratio)}ms",
            "fast": f"{round_ms(fast)}ms",
            "fast-max": f"{round_ms(fast / ratio)}ms",
            "medium-min": f"{round_ms(medium * ratio)}ms",
            "medium": f"{round_ms(medium)}ms",
            "medium-max": f"{round_ms(medium / ratio)}ms",
        })
        if "slow" in config:
            slow = float(config["slow"])
            table.update({
                "slow-min": f"{round_ms(slow * ratio)}ms",
                "slow": f"{round_ms(slow)}ms",
                "slow-max": f"{round_ms(slow / ratio)}ms",
            })
    if "tempo" in config or "character" in config:
        sys.exit(f"{name}: tempo/character knobs are not exported yet; extend this script")
    standard = config.get("easing", f"'{eases['standard']}'").strip("'")
    return {
        "knobs": {"tempo": 1.0, "character": "standard", "reducedMotion": "system"},
        # A MotionScaleConfig's durations win over a tempo knob (spec:AST-108 FR5).
        "durationScale": {"fast", "medium", "ratio"} <= config.keys(),
        "easingOverride": "easing" in config,
        "duration": {k: ms(v, table) for k, v in table.items()},
        "ease": {
            "standard": bezier(standard),
            # At 'standard' the entrance and move curves follow --ease-standard
            # unless the theme sets them; no published theme does.
            "entrance": bezier(standard if "easing" in config else eases["entrance"]),
            "move": bezier(standard if "easing" in config else eases["move"]),
            "exit": bezier(eases["exit"]),
        },
    }


def springs():
    roles = {}
    for line in read("docs/specs/AST-094-touch-presentations/motion-roles.md").splitlines():
        m = re.match(r"\|\s*`springs\.(\w+)`\s*\|\s*spring\s*\|\s*(.*?)\s*\|\s*`spec:AST-094` FR4\s*\|", line)
        if not m:
            continue
        name, default = m.groups()
        if r := re.fullmatch(r"response ([\d.]+) s, damping ratio ([\d.]+)", default):
            roles[name] = {"response": float(r.group(1)), "dampingRatio": float(r.group(2))}
        elif d := re.fullmatch(r"duration ([\d.]+) s, bounce ([\d.]+)", default):
            roles[name] = {"duration": float(d.group(1)), "bounce": float(d.group(2))}
        elif p := re.fullmatch(r"mass ([\d.]+), stiffness ([\d.]+), damping ([\d.]+)", default):
            roles[name] = {"mass": float(p.group(1)), "stiffness": float(p.group(2)), "damping": float(p.group(3))}
        else:
            sys.exit(f"unknown spring form: {line}")
    return roles


out = {
    "themes": {name: theme_entry(name) for name in THEMES},
    "knobs": {
        "tempoRange": [float(tempo.group(1)), float(tempo.group(2))],
        "characterEase": {k: bezier(v) for k, v in character.items()},
    },
    "reducedMotionCrossFade": {"ms": number(fade, "durationMs"), "curve": curve(fade, "curve")},
    "press": {
        "highlightInMs": number(press, "highlightInMs"),
        "flashMs": number(press, "flashMs"),
    },
    "longPress": {
        "ms": number(long_press_ts, "LONG_PRESS_DURATION_MS"),
        "allowableMovementPx": number(long_press_ts, "LONG_PRESS_ALLOWABLE_MOVEMENT_PX"),
    },
    "springs": springs(),
}
json.dump(out, sys.stdout, indent=1, sort_keys=False)
print()
