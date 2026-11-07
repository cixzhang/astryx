"""Turn @astryxdesign/native's generated Swift themes into JSON for the Rust experiment."""
import json, re, sys, pathlib
GEN = pathlib.Path(sys.argv[1])
COLOR = re.compile(r'Color\(\.sRGB, red: ([\d.e-]+), green: ([\d.e-]+), blue: ([\d.e-]+), opacity: ([\d.e-]+)\)')
def val(s):
    s = s.strip()
    out = COLOR.sub(lambda m: json.dumps({"rgba": [float(x) for x in m.groups()]}), s)
    out = re.sub(r'ShadowLayer\(', '{', out)
    out = re.sub(r'TypeStyleTokens\(|FontFamilyTokens\(|FontSizeTokens\(|FontWeightTokens\(', '{', out)
    out = re.sub(r'\)(?=\s*[,\]}]|$)', '}', out)
    out = re.sub(r'([{,]\s*)([A-Za-z_][A-Za-z0-9_]*)\s*:', r'\1"\2":', out)
    out = re.sub(r'"isInset":\s*(true|false)', r'"isInset": \1', out)
    return json.loads(out)
themes = {}
for f in sorted(GEN.glob('*Theme.swift')):
    text = f.read_text()
    name = re.search(r'theme "([^"]+)"', text).group(1)
    modes, mode, group = {}, None, None
    for line in text.splitlines():
        t = line.strip()
        if m := re.match(r'(light|dark): Tokens\(', t): mode = m.group(1); modes[mode] = {}; continue
        if m := re.match(r'(\w+): \w+Tokens\($', t): group = m.group(1); modes[mode][group] = {}; continue
        if mode and group and (m := re.match(r'`?(\w+)`?: (.+?),?$', t)):
            modes[mode][group][m.group(1)] = val(m.group(2))
        elif t.startswith(')'): group = None
    themes[name] = modes
json.dump(themes, sys.stdout, indent=1)
