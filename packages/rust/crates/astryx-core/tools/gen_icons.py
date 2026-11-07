"""Turn Astryx core's fallback icons (packages/core/src/Icon/defaultIcons.tsx)
into Rust path data for astryx-core.

Usage: python3 gen_icons.py <astryx>/packages/core/src/Icon/defaultIcons.tsx > ../src/components/icon_paths.rs

Arcs, circles, and rounded rects become cubic Béziers. Filled icons use the
even-odd rule in SVG; the display list fills with the nonzero rule, so inner
contours are reversed to wind against the outer one, which fills the same.
"""
import math
import re
import sys

src = open(sys.argv[1]).read()
body = src[src.index('export const defaultIcons'):]
entries = re.findall(r"\n  '?([\w:]+)'?: \(\s*<svg \{\.\.\.(svgProps|solidSvgProps)\}([^>]*)>(.*?)</svg>", body, re.S)

def arc_to_cubics(x1, y1, rx, ry, phi, fa, fs, x2, y2):
    # SVG implementation notes F.6.5, then quarter-or-less cubic segments.
    if rx == 0 or ry == 0:
        return [((x1, y1), (x2, y2), (x2, y2))]
    phi = math.radians(phi)
    cp, sp = math.cos(phi), math.sin(phi)
    dx, dy = (x1 - x2) / 2, (y1 - y2) / 2
    x1p, y1p = cp * dx + sp * dy, -sp * dx + cp * dy
    rx, ry = abs(rx), abs(ry)
    lam = x1p**2 / rx**2 + y1p**2 / ry**2
    if lam > 1:
        rx, ry = rx * math.sqrt(lam), ry * math.sqrt(lam)
    num = rx**2 * ry**2 - rx**2 * y1p**2 - ry**2 * x1p**2
    den = rx**2 * y1p**2 + ry**2 * x1p**2
    co = math.sqrt(max(0, num / den)) * (-1 if fa == fs else 1)
    cxp, cyp = co * rx * y1p / ry, -co * ry * x1p / rx
    cx, cy = cp * cxp - sp * cyp + (x1 + x2) / 2, sp * cxp + cp * cyp + (y1 + y2) / 2
    def ang(ux, uy, vx, vy):
        a = math.atan2(ux * vy - uy * vx, ux * vx + uy * vy)
        return a
    t1 = ang(1, 0, (x1p - cxp) / rx, (y1p - cyp) / ry)
    dt = ang((x1p - cxp) / rx, (y1p - cyp) / ry, (-x1p - cxp) / rx, (-y1p - cyp) / ry)
    if not fs and dt > 0:
        dt -= 2 * math.pi
    elif fs and dt < 0:
        dt += 2 * math.pi
    n = max(1, math.ceil(abs(dt) / (math.pi / 2) - 1e-9))
    step = dt / n
    k = 4 / 3 * math.tan(step / 4)
    out = []
    def pt(t):
        x, y = rx * math.cos(t), ry * math.sin(t)
        return cx + cp * x - sp * y, cy + sp * x + cp * y
    def deriv(t):
        x, y = -rx * math.sin(t), ry * math.cos(t)
        return cp * x - sp * y, sp * x + cp * y
    for i in range(n):
        a, b = t1 + i * step, t1 + (i + 1) * step
        p0, p3 = pt(a), pt(b)
        d0, d3 = deriv(a), deriv(b)
        out.append(((p0[0] + k * d0[0], p0[1] + k * d0[1]), (p3[0] - k * d3[0], p3[1] - k * d3[1]), p3))
    return out

NUM = re.compile(r'\s*,?\s*(-?(?:\d+\.?\d*|\.\d+)(?:e-?\d+)?)')
FLAG = re.compile(r'\s*,?\s*([01])')
CMD = re.compile(r'\s*([MmLlHhVvCcSsQqTtAaZz])')

def parse(d):
    pos = 0
    cmd = None
    x = y = sx = sy = 0.0
    last_c = None
    subpaths, cur = [], []
    def num():
        nonlocal pos
        m = NUM.match(d, pos); pos = m.end(); return float(m.group(1))
    def flag():
        nonlocal pos
        m = FLAG.match(d, pos); pos = m.end(); return int(m.group(1))
    while d[pos:].strip():
        m = CMD.match(d, pos)
        if m:
            cmd = m.group(1); pos = m.end()
            if cmd in 'Zz':
                cur.append(('Z',)); subpaths.append(cur); cur = []
                x, y = sx, sy; last_c = None
                continue
        rel = cmd.islower()
        c = cmd.upper()
        ox, oy = (x, y) if rel else (0.0, 0.0)
        if c == 'M':
            if cur: subpaths.append(cur)
            x, y = ox + num(), oy + num(); sx, sy = x, y
            cur = [('M', (x, y))]
            cmd = 'l' if rel else 'L'; last_c = None
        elif c == 'L':
            x, y = ox + num(), oy + num(); cur.append(('L', (x, y))); last_c = None
        elif c == 'H':
            x = (x + num()) if rel else num(); cur.append(('L', (x, y))); last_c = None
        elif c == 'V':
            y = (y + num()) if rel else num(); cur.append(('L', (x, y))); last_c = None
        elif c == 'C':
            c1 = (ox + num(), oy + num()); c2 = (ox + num(), oy + num()); x, y = ox + num(), oy + num()
            cur.append(('C', c1, c2, (x, y))); last_c = c2
        elif c == 'S':
            c1 = (2 * x - last_c[0], 2 * y - last_c[1]) if last_c else (x, y)
            c2 = (ox + num(), oy + num()); x, y = ox + num(), oy + num()
            cur.append(('C', c1, c2, (x, y))); last_c = c2
        elif c == 'A':
            rx, ry, phi = num(), num(), num()
            fa, fs = flag(), flag()
            nx, ny = ox + num(), oy + num()
            for c1, c2, p in arc_to_cubics(x, y, rx, ry, phi, fa, fs, nx, ny):
                cur.append(('C', c1, c2, p))
            x, y = nx, ny; last_c = None
        else:
            raise SystemExit(f'unsupported path command {cmd}')
    if cur: subpaths.append(cur)
    return subpaths

def circle(cx, cy, r):
    return parse(f'M{cx + r} {cy}A{r} {r} 0 1 1 {cx - r} {cy}A{r} {r} 0 1 1 {cx + r} {cy}Z')

def rect(x, y, w, h, rx):
    if rx == 0:
        return parse(f'M{x} {y}h{w}v{h}h{-w}Z')
    return parse(f'M{x + rx} {y}H{x + w - rx}A{rx} {rx} 0 0 1 {x + w} {y + rx}V{y + h - rx}A{rx} {rx} 0 0 1 {x + w - rx} {y + h}'
                 f'H{x + rx}A{rx} {rx} 0 0 1 {x} {y + h - rx}V{y + rx}A{rx} {rx} 0 0 1 {x + rx} {y}Z')

def area(sp):
    pts = [seg[-1] for seg in sp if seg[0] != 'Z']
    return sum(pts[i][0] * pts[(i + 1) % len(pts)][1] - pts[(i + 1) % len(pts)][0] * pts[i][1] for i in range(len(pts))) / 2

def reverse(sp):
    closed = sp[-1][0] == 'Z'
    segs = [s for s in sp if s[0] != 'Z']
    start = segs[0][1]
    out, pos = [], start
    ends = [start] + [s[-1] for s in segs[1:]]
    rev = [('M', ends[-1])]
    for j in range(len(segs) - 1, 0, -1):
        s, to = segs[j], ends[j - 1]
        rev.append(('C', s[2], s[1], to) if s[0] == 'C' else ('L', to))
    if closed: rev.append(('Z',))
    return rev

def fmt(v):
    v = round(v, 3)
    return f'{v:.3f}'.rstrip('0').rstrip('.') + ('.0' if float(v).is_integer() else '')

def p(pt):
    return f'p({fmt(pt[0])}, {fmt(pt[1])})'

out = ['// Generated by tools/gen_icons.py from Astryx core\'s fallback icons. Do not edit.',
       '',
       'use crate::contract::{PathCmd, Point};',
       '',
       'const fn p(x: f32, y: f32) -> Point {',
       '    Point { x, y }',
       '}',
       '',
       'use PathCmd::{Close as Z, CubicTo as C, LineTo as L, MoveTo as M};',
       '']
table = []
for name, kind, extra, inner in entries:
    if ':' in name:
        continue
    filled = kind == 'solidSvgProps'
    width = 2.0 if 'strokeWidth={2}' in extra else 1.5
    subs = []
    for el in re.finditer(r'<(path|circle|rect)\b(.*?)/>', inner, re.S):
        tag, attrs = el.group(1), dict(re.findall(r'(\w+)="([^"]*)"', el.group(2)))
        if tag == 'path':
            subs += parse(attrs['d'])
        elif tag == 'circle':
            subs += circle(float(attrs['cx']), float(attrs['cy']), float(attrs['r']))
        else:
            subs += rect(float(attrs['x']), float(attrs['y']), float(attrs['width']), float(attrs['height']), float(attrs.get('rx', 0)))
    if filled and len(subs) > 1:
        outer = max(subs, key=lambda s: abs(area(s)))
        sign = area(outer) > 0
        subs = [s if s is outer or (area(s) > 0) != sign else reverse(s) for s in subs]
    cmds = []
    for sp in subs:
        for seg in sp:
            if seg[0] == 'M': cmds.append(f'M({p(seg[1])})')
            elif seg[0] == 'L': cmds.append(f'L({p(seg[1])})')
            elif seg[0] == 'C': cmds.append(f'C({p(seg[1])}, {p(seg[2])}, {p(seg[3])})')
            else: cmds.append('Z')
    const = re.sub(r'(?<!^)(?=[A-Z])', '_', name).upper()
    out.append(f'const {const}: &[PathCmd] = &[{", ".join(cmds)}];')
    table.append((name, const, filled, width))
out.append('')
out.append('/// The path, whether it is filled (else stroked), and its stroke width in')
out.append('/// the 24-point view box, for a built-in icon name.')
out.append("pub(crate) fn icon_path(name: &str) -> Option<(&'static [PathCmd], bool, f32)> {")
out.append('    Some(match name {')
for name, const, filled, width in table:
    out.append(f'        "{name}" => ({const}, {"true" if filled else "false"}, {width}),')
out.append('        _ => return None,')
out.append('    })')
out.append('}')
out.append('')
out.append('/// Every built-in icon name, in the order Astryx lists them.')
out.append(f'pub const BUILT_IN: [&str; {len(table)}] = [{", ".join(chr(34) + n + chr(34) for n, *_ in table)}];')
print('\n'.join(out))
