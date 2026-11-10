"""Report which Astryx tokens a set of source files uses that the Rust
token set (packages/rust/tokens/themes.json) does not carry.

Usage: python3 tools/token-coverage.py <astryx-repo> <themes.json> [file-list.txt]

Without a file list, every non-test source file in packages/core/src counts.
"""
import json, pathlib, re, sys

repo = pathlib.Path(sys.argv[1]) / "packages/core/src"
themes = json.loads(pathlib.Path(sys.argv[2]).read_text())
files = (
    [repo / l.strip() for l in open(sys.argv[3]) if l.strip()]
    if len(sys.argv) > 3
    else [p for p in repo.rglob("*.ts*") if ".test." not in p.name and ".spec." not in p.name]
)
use = re.compile(r"(\w+Vars)\['--([a-z0-9-]+)'\]")
used = {}
for f in files:
    if f.exists():
        for group, name in use.findall(f.read_text()):
            used.setdefault(group, set()).add(name)

def norm(s):
    return re.sub(r"[^a-z0-9]", "", s.lower())

rust = set()
light = themes["astryx"]["light"]
for group, values in light.items():
    if isinstance(values, dict):
        for key, value in values.items():
            rust.add(norm(group + key))
            rust.add(norm(key))
            if isinstance(value, dict):
                for sub in value:
                    rust.add(norm(group + key + sub))
                    rust.add(norm(key + sub))

motion = json.loads((pathlib.Path(sys.argv[2]).parent / "motion.json").read_text())["themes"]["astryx"]
for group in ("duration", "ease"):
    for key in motion[group]:
        rust.add(norm(group + key))
# Tokens the Rust stack carries under other names or as foundations extras.
ALIASES = {
    "spacing-": "space-s",
    "font-size-": "font-size-s",
}
EXTRAS = {
    "focus-outline-width", "focus-outline-offset", "focus-outline-color", "focus-outline-style",
    "shadow-inset-error", "shadow-inset-success", "shadow-inset-warning",
    # A per-press runtime value, owned by the core press highlight, not a theme token.
    "astryx-press-alpha",
}

def expressed(name):
    if name in EXTRAS or norm(name) in rust or norm(name.split("-", 1)[-1]) in rust:
        return True
    for prefix, alias in ALIASES.items():
        if name.startswith(prefix) and norm(alias + name[len(prefix):]) in rust:
            return True
    return False

total = missing = 0
for group in sorted(used):
    names = sorted(used[group])
    gaps = [n for n in names if not expressed(n)]
    total += len(names)
    missing += len(gaps)
    print(f"{group}: {len(names)} used, {len(gaps)} missing{': ' + ', '.join(gaps) if gaps else ''}")
print(f"TOTAL {total} used, {total - missing} expressed ({100 * (total - missing) / max(total, 1):.1f}%)")
