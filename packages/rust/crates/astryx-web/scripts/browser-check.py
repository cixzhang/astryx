#!/usr/bin/env python3
"""Real-browser check for the astryx-web demo, using headless Chromium.

Serves a built dist directory, loads it, and checks that the canvas paints,
that the keyboard, gamepads, and assistive-tech style focus and clicks reach
the app, and that the accessibility tree carries roles, names, and states.
Writes screenshots, the AX tree, and a JSON summary to the proof directory.

    scripts/browser-check.py [--dist DIR] [--proof DIR] [--chrome PATH] [--timing N]

Only the Python standard library is used. Chromium speaks the DevTools
protocol over a pipe (--remote-debugging-pipe), so no WebSocket client is
needed.
"""

import argparse
import base64
import fcntl
import functools
import http.server
import json
import os
import shutil
import statistics
import struct
import subprocess
import sys
import tempfile
import threading
import time
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(HERE)

CHROME_CANDIDATES = [
    "headless_shell",
    "chromium-headless-shell",
    "/usr/lib64/chromium-browser/headless_shell",
    "chromium",
    "chromium-browser",
    "google-chrome",
]

GAMEPAD_STUB = """
(() => {
  const pad = { id: 'stub', index: 0, connected: true, mapping: 'standard', timestamp: 0,
    buttons: Array.from({length: 17}, () => ({pressed: false, touched: false, value: 0})),
    axes: [0, 0, 0, 0] };
  window.__stubPad = pad;
  window.__stubPadOn = false;
  navigator.getGamepads = () => [window.__stubPadOn ? pad : null, null, null, null];
})();
"""


class Cdp:
    def __init__(self, chrome, user_dir, width, height):
        to_chrome_r, self._w = os.pipe()
        self._r, from_chrome_w = os.pipe()

        def child_fds():
            # Copy both ends above fd 4 first, so neither dup2 clobbers the other.
            read_end = fcntl.fcntl(to_chrome_r, fcntl.F_DUPFD, 10)
            write_end = fcntl.fcntl(from_chrome_w, fcntl.F_DUPFD, 10)
            os.dup2(read_end, 3)
            os.dup2(write_end, 4)

        args = [
            chrome,
            "--headless",
            "--no-sandbox",
            "--remote-debugging-pipe",
            f"--user-data-dir={user_dir}",
            f"--window-size={width},{height}",
            "--hide-scrollbars",
            "--force-color-profile=srgb",
            "--use-angle=swiftshader",
            "--enable-unsafe-swiftshader",
            "--no-first-run",
            # Headless pages count as background pages; keep timers and frames running.
            "--disable-background-timer-throttling",
            "--disable-backgrounding-occluded-windows",
            "--disable-renderer-backgrounding",
            "about:blank",
        ]
        self.proc = subprocess.Popen(
            args,
            pass_fds=(3, 4, to_chrome_r, from_chrome_w),
            preexec_fn=child_fds,
            stdout=subprocess.DEVNULL,
            stderr=None if os.environ.get("CHROME_LOG") else subprocess.DEVNULL,
        )
        os.close(to_chrome_r)
        os.close(from_chrome_w)
        self._id = 0
        self._buf = b""
        self.events = []
        self.session = None

    def _read(self):
        while b"\0" not in self._buf:
            chunk = os.read(self._r, 1 << 20)
            if not chunk:
                raise RuntimeError("Chromium closed the pipe")
            self._buf += chunk
        raw, self._buf = self._buf.split(b"\0", 1)
        return json.loads(raw)

    def send(self, method, params=None, session=True):
        self._id += 1
        msg = {"id": self._id, "method": method, "params": params or {}}
        if session and self.session:
            msg["sessionId"] = self.session
        os.write(self._w, json.dumps(msg).encode() + b"\0")
        while True:
            reply = self._read()
            if reply.get("id") == self._id:
                if "error" in reply:
                    raise RuntimeError(f"{method}: {reply['error']}")
                return reply.get("result", {})
            self.events.append(reply)

    def eval(self, expression, await_promise=False):
        result = self.send(
            "Runtime.evaluate",
            {"expression": expression, "returnByValue": True, "awaitPromise": await_promise},
        )
        if "exceptionDetails" in result:
            raise RuntimeError(f"JS error in {expression!r}: {result['exceptionDetails']}")
        return result["result"].get("value")

    def close(self):
        try:
            self.send("Browser.close", session=False)
        except Exception:
            pass
        self.proc.wait(timeout=10)


def decode_png(data):
    """Decodes an 8-bit RGB or RGBA, non-interlaced PNG into rows of pixels."""
    assert data[:8] == b"\x89PNG\r\n\x1a\n"
    pos, idat = 8, b""
    while pos < len(data):
        length, kind = struct.unpack(">I4s", data[pos : pos + 8])
        body = data[pos + 8 : pos + 8 + length]
        if kind == b"IHDR":
            width, height, depth, color = struct.unpack(">IIBB", body[:10])
            assert depth == 8 and color in (2, 6), (depth, color)
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    bpp = 4 if color == 6 else 3
    raw = zlib.decompress(idat)
    stride = width * bpp
    rows, prev = [], bytearray(stride)
    for y in range(height):
        f = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1 : (y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - bpp] if i >= bpp else 0
            b = prev[i]
            c = prev[i - bpp] if i >= bpp else 0
            if f == 1:
                line[i] = (line[i] + a) & 255
            elif f == 2:
                line[i] = (line[i] + b) & 255
            elif f == 3:
                line[i] = (line[i] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pred = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                line[i] = (line[i] + pred) & 255
        rows.append(line)
        prev = line
    return width, height, bpp, rows


class Screen:
    def __init__(self, png, scale):
        self.png = png
        self.scale = scale
        self.width, self.height, self.bpp, self.rows = decode_png(png)

    def pixel(self, x, y):
        """The RGB color at a CSS-pixel position."""
        px, py = int(x * self.scale), int(y * self.scale)
        i = px * self.bpp
        return tuple(self.rows[py][i : i + 3])


def hex_rgb(value):
    return ((value >> 16) & 255, (value >> 8) & 255, value & 255)


def close_to(rgb, want, tolerance=6):
    return all(abs(a - b) <= tolerance for a, b in zip(rgb, hex_rgb(want)))


def serve(directory):
    handler = functools.partial(QuietHandler, directory=directory)
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


class QuietHandler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "text/javascript",
        ".ttf": "font/ttf",
    }

    def log_message(self, *args):
        pass


def find_chrome(explicit):
    for candidate in [explicit] if explicit else CHROME_CANDIDATES:
        path = shutil.which(candidate) or (candidate if os.path.isfile(candidate) else None)
        if path:
            return path
    sys.exit("no headless Chromium found; pass --chrome")


def ax_summary(nodes):
    """Non-ignored AX nodes as (role, name, properties) rows in tree order."""
    by_id = {n["nodeId"]: n for n in nodes}
    out = []

    def walk(node_id, depth):
        node = by_id.get(node_id)
        if not node:
            return
        role = node.get("role", {}).get("value")
        props = {}
        for p in node.get("properties", []):
            if p["name"] in ("focusable", "focused", "checked", "pressed", "disabled", "level",
                             "valuemin", "valuemax", "valuetext", "readonly", "settable", "live", "atomic"):
                props[p["name"]] = p["value"].get("value")
        # Generic containers matter only when they are live regions.
        keep = not node.get("ignored") and (role not in ("none", "generic", "InlineTextBox") or "live" in props)
        if keep:
            value = node.get("value", {}).get("value")
            if value not in (None, ""):
                props["value"] = value
            out.append({"depth": depth, "role": role, "name": node.get("name", {}).get("value", ""),
                        "props": props})
        for child in node.get("childIds", []):
            walk(child, depth + (1 if keep else 0))

    roots = [n for n in nodes if not n.get("parentId")]
    for root in roots:
        walk(root["nodeId"], 0)
    return out


def format_ax(rows):
    lines = []
    for r in rows:
        props = " ".join(f"{k}={v}" for k, v in r["props"].items())
        lines.append(f"{'  ' * r['depth']}{r['role']} \"{r['name']}\" {props}".rstrip())
    return "\n".join(lines)


class Check:
    def __init__(self):
        self.results = []

    def __call__(self, name, ok, detail=""):
        self.results.append({"check": name, "ok": bool(ok), "detail": detail})
        print(f"{'PASS' if ok else 'FAIL'}  {name}{'  (' + str(detail) + ')' if detail != '' else ''}")


def key(cdp, name, code=None, vk=0, text=None, shift=False):
    base = {"key": name, "code": code or name, "windowsVirtualKeyCode": vk,
            "modifiers": 8 if shift else 0}
    down = dict(base, type="keyDown" if text else "rawKeyDown")
    if text:
        down["text"] = text
    cdp.send("Input.dispatchKeyEvent", down)
    cdp.send("Input.dispatchKeyEvent", dict(base, type="keyUp"))


def settle(cdp, ms=120):
    cdp.eval(f"new Promise(r => setTimeout(r, {ms}))", await_promise=True)


ACTIVE_ID = "document.activeElement && document.activeElement.getAttribute('data-astryx-id')"
FRAMES = "Number(document.querySelector('.astryx-a11y').getAttribute('data-astryx-frames'))"


def tab_stop(cdp):
    return cdp.eval("(document.querySelector('.astryx-a11y [tabindex=\"0\"]')||{}).getAttribute"
                    "?.call(document.querySelector('.astryx-a11y [tabindex=\"0\"]'),'data-astryx-id')")


def rect_of(cdp, node_id):
    return cdp.eval(f"(() => {{ const r = document.querySelector('[data-astryx-id=\"{node_id}\"]')"
                    f".getBoundingClientRect(); return [r.x, r.y, r.width, r.height]; }})()")


def ax(cdp):
    return ax_summary(cdp.send("Accessibility.getFullAXTree")["nodes"])


def ax_find(rows, role, name):
    return next((r for r in rows if r["role"] == role and r["name"] == name), None)


def load(cdp, url, timeout=15):
    cdp.send("Page.navigate", {"url": url})
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            if cdp.eval("performance.getEntriesByName('astryx-first-frame').length") == 1:
                return cdp.eval(
                    "({first: performance.getEntriesByName('astryx-first-frame')[0].startTime,"
                    " start: performance.getEntriesByName('astryx-start')[0].startTime,"
                    " renderer: performance.getEntriesByName('astryx-renderer-ready')[0].startTime})"
                )
        except RuntimeError:
            pass
        time.sleep(0.02)
    raise RuntimeError("no first frame")


def open_page(chrome, user_dir, width, height, scale):
    cdp = Cdp(chrome, user_dir, width, height)
    target = cdp.send("Target.createTarget", {"url": "about:blank"}, session=False)["targetId"]
    cdp.session = cdp.send("Target.attachToTarget", {"targetId": target, "flatten": True},
                           session=False)["sessionId"]
    for domain in ("Page", "Runtime", "Network", "Accessibility"):
        cdp.send(f"{domain}.enable")
    cdp.send("Network.setCacheDisabled", {"cacheDisabled": True})
    cdp.send("Emulation.setDeviceMetricsOverride",
             {"width": width, "height": height, "deviceScaleFactor": scale, "mobile": False})
    return cdp


def timing_runs(chrome, url, runs):
    firsts, starts, renderers = [], [], []
    for _ in range(runs):
        with tempfile.TemporaryDirectory() as user_dir:
            cdp = open_page(chrome, user_dir, 800, 900, 2)
            t = load(cdp, url)
            firsts.append(t["first"])
            starts.append(t["start"])
            renderers.append(t["renderer"] - t["start"])
            cdp.close()
    median = lambda values: round(statistics.median(values), 1)
    return {"runs": runs,
            "navigation_to_first_frame_ms": median(firsts),
            "navigation_to_run_ms": median(starts),
            "run_to_renderer_ready_ms": median(renderers),
            "renderer_ready_to_first_frame_ms": median([f - s - r for f, s, r in zip(firsts, starts, renderers)]),
            "navigation_to_first_frame_all_ms": [round(v, 1) for v in firsts]}


def gallery_check(chrome, url, proof, check, summary):
    """The gallery on the real core runtime: paints, exposes a tree, takes keys."""
    with tempfile.TemporaryDirectory() as user_dir:
        cdp = open_page(chrome, user_dir, 960, 900, 2)
        summary["gallery_timing"] = load(cdp, url)
        settle(cdp, 300)
        shot = Screen(base64.b64decode(cdp.send("Page.captureScreenshot", {"format": "png"})["data"]), 2)
        with open(os.path.join(proof, "screenshot-gallery.png"), "wb") as f:
            f.write(shot.png)
        colors = {shot.pixel(x, y) for x in range(0, 960, 16) for y in range(0, 900, 16)}
        check("gallery: the runtime paints (many distinct colors)", len(colors) > 20, len(colors))
        rows = ax(cdp)
        with open(os.path.join(proof, "ax-tree-gallery.txt"), "w") as f:
            f.write(format_ax(rows) + "\n")
        roles = {}
        for r in rows:
            roles[r["role"]] = roles.get(r["role"], 0) + 1
        summary["gallery_roles"] = roles
        check("gallery: the home screen exposes headings, section buttons, and selectors",
              all(roles.get(role, 0) > 0 for role in ("heading", "button", "combobox")), roles)
        key(cdp, "Tab", vk=9)
        first = cdp.eval(ACTIVE_ID)
        key(cdp, "Tab", vk=9)
        settle(cdp)
        second = cdp.eval(ACTIVE_ID)
        check("gallery: Tab enters the app and moves focus with the DOM following",
              first is not None and second is not None and first != second, [first, second])

        errors = [e for e in cdp.events if e.get("method") == "Runtime.exceptionThrown"
                  or (e.get("method") == "Runtime.consoleAPICalled"
                      and e["params"].get("type") in ("error", "warning"))]
        check("gallery: no console errors or exceptions", not errors, errors[:3])
        cdp.close()

    # Mouse. A window tall enough for the whole gallery, since the shell has
    # no wheel scrolling yet.
    with tempfile.TemporaryDirectory() as user_dir:
        cdp = open_page(chrome, user_dir, 960, 8000, 1)
        load(cdp, url)
        settle(cdp, 300)

        def mouse(kind, x, y):
            cdp.send("Input.dispatchMouseEvent", {"type": kind, "x": x, "y": y, "button": "left",
                                                  "buttons": 1 if kind == "mousePressed" else 0, "clickCount": 1})

        def click(selector):
            x, y, w, h = cdp.eval(f"(() => {{ const r = document.querySelector('{selector}')"
                                  f".getBoundingClientRect(); return [r.x, r.y, r.width, r.height]; }})()")
            mouse("mouseMoved", x + w / 2, y + h / 2)
            cursor = cdp.eval("document.querySelector('#app').style.cursor")
            mouse("mousePressed", x + w / 2, y + h / 2)
            mouse("mouseReleased", x + w / 2, y + h / 2)
            settle(cdp, 200)
            return cursor

        def open_section(title):
            row = cdp.eval("[...document.querySelectorAll('.astryx-a11y button')].findIndex("
                           f"b => b.textContent === '{title}' || b.getAttribute('aria-label') === '{title}')")
            if row < 0:
                return None
            cdp.eval("[...document.querySelectorAll('.astryx-a11y button')].forEach((b, i) => "
                     f"b.toggleAttribute('data-check-target', i === {row}))")
            return click(".astryx-a11y [data-check-target]")

        cursor = open_section("Switch")
        switch = ".astryx-a11y [role=switch]:not([aria-disabled])"
        opened = cdp.eval(f"!!document.querySelector('{switch}')")
        check("gallery: a mouse click on a home row opens that section (pointer cursor on hover)",
              cursor == "pointer" and opened, [cursor, opened])
        if opened:
            before = cdp.eval(f"document.querySelector('{switch}').getAttribute('aria-checked')")
            switch_id = cdp.eval(f"document.querySelector('{switch}').getAttribute('data-astryx-id')")
            cursor = click(switch)
            after = cdp.eval(f"document.querySelector('[data-astryx-id=\"{switch_id}\"]').getAttribute('aria-checked')")
            check("gallery: a mouse click focuses and toggles a switch",
                  before != after and cdp.eval(ACTIVE_ID) == switch_id,
                  [cursor, before, after, cdp.eval(ACTIVE_ID), switch_id])
        key(cdp, "Escape", code="Escape", vk=27)
        settle(cdp, 200)
        open_section("TextInput")
        field = ".astryx-a11y input:not([readonly])"
        field_id = cdp.eval(f"document.querySelector('{field}')?.getAttribute('data-astryx-id')")
        cursor = click(field) if field_id else None
        check("gallery: clicking a text field focuses it (text cursor on hover)",
              cursor == "text" and cdp.eval(ACTIVE_ID) == field_id, [cursor, cdp.eval(ACTIVE_ID), field_id])
        summary["gallery_password_inputs"] = cdp.eval("document.querySelectorAll('.astryx-a11y input[type=password]').length")
        shot = Screen(base64.b64decode(cdp.send("Page.captureScreenshot", {"format": "png"})["data"]), 1)
        with open(os.path.join(proof, "screenshot-gallery-full.png"), "wb") as f:
            f.write(shot.png)
        cdp.close()


def fetched_fonts(cdp):
    return cdp.eval("performance.getEntriesByType('resource').filter(e => e.name.endsWith('.ttf'))"
                    ".map(e => e.name.split('/').pop())")


def pixel_check(chrome, base, dist, proof, check, summary):
    """The pixel theme: Latin draws with the bundled Galmuri alone; other scripts load
    the theme's pixel fallbacks, and only those."""
    with tempfile.TemporaryDirectory() as user_dir:
        cdp = open_page(chrome, user_dir, 960, 900, 2)
        load(cdp, base + "gallery.html#pixel")
        settle(cdp, 400)
        shot = cdp.send("Page.captureScreenshot", {"format": "png"})["data"]
        with open(os.path.join(proof, "screenshot-gallery-pixel.png"), "wb") as f:
            f.write(base64.b64decode(shot))
        fonts = fetched_fonts(cdp)
        theme = cdp.eval("[...document.querySelectorAll('.astryx-a11y [role=combobox]')]"
                         ".map(e => e.textContent || e.getAttribute('aria-label')).join(' | ')")
        check("pixel gallery: Latin needs no fallback font files",
              not any(n.startswith("galmuri") for n in fonts), [fonts, theme])
        cdp.close()

    if not os.path.exists(os.path.join(dist, "fonts", "galmuri7-kana-400.ttf")):
        print("SKIP  pixel fallbacks: built without PIXEL_FONTS_DIR")
        return
    with tempfile.TemporaryDirectory() as user_dir:
        cdp = open_page(chrome, user_dir, 960, 900, 2)
        load(cdp, base + "pixel.html")
        settle(cdp, 800)
        png = base64.b64decode(cdp.send("Page.captureScreenshot", {"format": "png"})["data"])
        with open(os.path.join(proof, "screenshot-pixel-scripts.png"), "wb") as f:
            f.write(png)
        shot = Screen(png, 2)
        fonts = sorted(fetched_fonts(cdp))
        summary["pixel_fonts"] = fonts
        wanted = ["galmuri7-han-400.ttf", "galmuri7-hangul-400.ttf", "galmuri7-kana-400.ttf",
                  "galmuri7-vietnamese-400.ttf", "galmuri9-cyrillic-400.ttf", "galmuri9-greek-400.ttf"]
        shared = [n for n in fonts if n.split("-")[0] in ("cyrillic", "greek", "vietnamese")]
        check("pixel text loads the theme's fallbacks for each script, not the shared subsets",
              all(n in fonts for n in wanted) and not shared and "galmuri7-latin-ext-400.ttf" not in fonts, fonts)
        rows = cdp.eval("[...document.querySelectorAll('.astryx-a11y [data-astryx-id]')]"
                        ".filter(e => /^(Latin|Vietnamese|Greek|Cyrillic|Kana|Hangul|Han):/.test(e.textContent))"
                        ".map(e => { const r = e.getBoundingClientRect(); return [e.textContent.split(':')[0], r.x, r.y, r.width, r.height]; })")
        inks = {}
        for name, x, y, w, h in rows:
            # The script's own text is the last third of each inked line.
            ink = [(px, py) for px in range(int(x), int(x + w)) for py in range(int(y), int(y + h))
                   if shot.pixel(px, py) != shot.pixel(4, 4)]
            if not ink:
                inks[name] = 0
                continue
            left, right = min(p[0] for p in ink), max(p[0] for p in ink)
            inks[name] = sum(1 for px, _ in ink if px >= right - (right - left) / 3)
        check("each script's row draws glyphs after its label", len(inks) == 7 and all(v > 20 for v in inks.values()),
              inks)
        cdp.close()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dist", default=os.path.join(CRATE, "dist"))
    parser.add_argument("--proof", default=os.path.join(CRATE, "proof"))
    parser.add_argument("--chrome")
    parser.add_argument("--timing", type=int, default=0, help="only measure first frame over N loads")
    args = parser.parse_args()

    chrome = find_chrome(args.chrome)
    server = serve(args.dist)
    url = f"http://127.0.0.1:{server.server_address[1]}/index.html"
    os.makedirs(args.proof, exist_ok=True)

    if args.timing:
        print(json.dumps(timing_runs(chrome, url, args.timing), indent=2))
        return

    check = Check()
    summary = {"chrome": chrome}
    with tempfile.TemporaryDirectory() as user_dir:
        cdp = open_page(chrome, user_dir, 800, 900, 2)
        summary["chrome_version"] = cdp.send("Browser.getVersion", session=False)["product"]
        cdp.send("Target.setDiscoverTargets", {"discover": True}, session=False)
        cdp.send("Page.addScriptToEvaluateOnNewDocument", {"source": GAMEPAD_STUB})
        summary["timing"] = load(cdp, url)
        settle(cdp, 300)

        # Canvas size follows devicePixelRatio.
        size = cdp.eval("(() => { const c = document.querySelector('#app');"
                        " return [c.width, c.height, c.clientWidth, c.clientHeight]; })()")
        check("canvas backing store is CSS size x devicePixelRatio 2", size == [1600, 1800, 800, 900], size)

        # Pixels: background, then the switch track drawn where the mirror says the switch is.
        shot = Screen(base64.b64decode(cdp.send("Page.captureScreenshot", {"format": "png"})["data"]), 2)
        with open(os.path.join(args.proof, "screenshot-light.png"), "wb") as f:
            f.write(shot.png)
        check("background pixel is the light theme background", close_to(shot.pixel(4, 4), 0xF5F6F8),
              shot.pixel(4, 4))
        check("card pixel is white", close_to(shot.pixel(250, 50), 0xFFFFFF), shot.pixel(250, 50))
        x, y, w, h = rect_of(cdp, 11)
        track = shot.pixel(x + w - 52 + 6, y + h / 2)
        check("switch track (on) is accent blue at the mirror's position", close_to(track, 0x0064E0), track)
        bx, by, bw, bh = rect_of(cdp, 13)
        save = shot.pixel(bx + 6, by + 6)
        check("Save button fill is accent blue at the mirror's position", close_to(save, 0x0064E0), save)

        rows = ax(cdp)
        with open(os.path.join(args.proof, "ax-tree-initial.txt"), "w") as f:
            f.write(format_ax(rows) + "\n")
        check("heading level 1 'Settings'", (ax_find(rows, "heading", "Settings") or {}).get("props", {})
              .get("level") == 1)
        wifi = ax_find(rows, "switch", "Wi-Fi") or {}
        check("switch 'Wi-Fi' checked=true", wifi.get("props", {}).get("checked") == "true", wifi)
        diag = ax_find(rows, "checkbox", "Send diagnostics") or {}
        check("checkbox 'Send diagnostics' checked=false", diag.get("props", {}).get("checked") == "false", diag)
        check("button 'Save' exists", ax_find(rows, "button", "Save") is not None)
        reset = ax_find(rows, "button", "Reset") or {}
        check("button 'Reset' disabled", reset.get("props", {}).get("disabled") is True, reset)
        bar = ax_find(rows, "progressbar", "Storage used") or {}
        check("progressbar 'Storage used' value 40", bar.get("props", {}).get("value") == 40, bar)
        check("list 'Recent files' with 8 list items", ax_find(rows, "list", "Recent files") is not None
              and sum(1 for r in rows if r["role"] == "listitem") == 8)
        check("image 'Astryx logo'", ax_find(rows, "image", "Astryx logo") is not None)
        check("browser find matches mirror text (find highlight lands over the canvas text)",
              cdp.eval("window.find('Design review notes')") is True)
        cdp.eval("getSelection().removeAllRanges()")
        status = next((r for r in rows if r["props"].get("live")), {})
        check("status text is a polite, atomic live region", status.get("props", {}).get("live") == "polite"
              and status.get("props", {}).get("atomic") is True, status)
        check("canvas is hidden from assistive tech", cdp.eval("document.querySelector('#app')"
              ".getAttribute('aria-hidden')") == "true")

        # Fonts: Latin subsets before the first frame, other scripts on demand.
        fonts = cdp.eval("performance.getEntriesByType('resource').filter(e => e.name.endsWith('.ttf'))"
                         ".map(e => [e.name.split('/').pop(), e.startTime, e.responseEnd])")
        first = summary["timing"]["first"]
        loaded = {name: (start, end) for name, start, end in fonts}
        summary["fonts"] = [[n, round(s), round(e)] for n, s, e in fonts]
        latin = [f"latin-{w}.ttf" for w in (400, 500, 700)]
        check("Latin subsets of all three faces load before the first frame",
              all(n in loaded and loaded[n][1] <= first for n in latin), {n: loaded.get(n) for n in latin})
        on_demand = [f"{s}-400.ttf" for s in ("cyrillic", "greek", "vietnamese", "hebrew", "arabic")]
        check("Cyrillic, Greek, Vietnamese, Hebrew, and Arabic load after the first frame, 400 only",
              all(n in loaded and loaded[n][0] >= first for n in on_demand) and len(loaded) == 8,
              sorted(loaded))
        def ink(node_id):
            x, y, w, h = rect_of(cdp, node_id)
            return sum(1 for px in range(int(x), int(x + w), 2) for py in range(int(y), int(y + h), 2)
                       if sum(shot.pixel(px, py)) < 300)
        rows_by_script = (("cyrillic", 103), ("greek", 104), ("vietnamese", 105), ("hebrew", 106), ("arabic", 107))
        inks = {name: ink(node_id) for name, node_id in rows_by_script}
        check("Cyrillic, Greek, Vietnamese, Hebrew, and Arabic rows draw text with the loaded faces",
              all(v > 15 for v in inks.values()), inks)

        def ink_side(node_id):
            x, y, w, h = rect_of(cdp, node_id)
            xs = [px for px in range(int(x), int(x + w), 2) for py in range(int(y), int(y + h), 2)
                  if sum(shot.pixel(px, py)) < 300]
            return (min(xs) - x, x + w - max(xs)) if xs else None
        # The renderer aligns TextAlign::Start to the left in either direction,
        # so where the ink sits is recorded, not checked.
        summary["rtl_ink_margins_left_right"] = {name: ink_side(node_id) for name, node_id in rows_by_script[3:]}
        dirs = cdp.eval("[106, 107].map(id => document.querySelector(`[data-astryx-id=\"${id}\"]`).dir)")
        check("Hebrew and Arabic rows have dir=rtl in the mirror", dirs == ["rtl", "rtl"], dirs)
        check("unused subsets (latin-ext, cyrillic-ext, greek-ext) never load",
              not any("-ext" in n for n in loaded), sorted(loaded))

        # Idle: no frames while nothing changes.
        f0 = cdp.eval(FRAMES)
        settle(cdp, 500)
        check("no frames while idle", cdp.eval(FRAMES) == f0, f"{f0} -> {cdp.eval(FRAMES)}")

        # Keyboard. Page content before and after the app shows Tab entering and leaving.
        cdp.eval("document.body.insertAdjacentHTML('afterbegin', '<button id=before>Before</button>');"
                 "document.body.insertAdjacentHTML('beforeend', '<button id=after>After</button>');"
                 "document.querySelector('#before').focus()")
        key(cdp, "Tab", vk=9)
        check("Tab from the page enters the app on the focused switch", cdp.eval(ACTIVE_ID) == "11",
              cdp.eval(ACTIVE_ID))
        f0 = cdp.eval(FRAMES)
        key(cdp, "ArrowDown", vk=40)
        settle(cdp, 400)
        check("ArrowDown moves app focus and DOM focus follows", cdp.eval(ACTIVE_ID) == "12",
              cdp.eval(ACTIVE_ID))
        animated = cdp.eval(FRAMES) - f0
        check("focus ring animates over several frames, then stops", animated > 3, f"{animated} frames")
        key(cdp, " ", code="Space", vk=32, text=" ")
        settle(cdp)
        diag = ax_find(ax(cdp), "checkbox", "Send diagnostics") or {}
        check("Space toggles the checkbox exactly once", diag.get("props", {}).get("checked") == "true", diag)
        key(cdp, "Tab", vk=9)
        check("Tab moves to the Name text field", cdp.eval(ACTIVE_ID) == "16", cdp.eval(ACTIVE_ID))

        # Typing reaches the app through the mirror field; the app owns the value.
        field_value = ("(() => { const n = document.querySelector('[data-astryx-id=\"16\"]');"
                       " return n.value; })()")
        for ch, code, vk in (("H", "KeyH", 72), ("i", "KeyI", 73), ("!", "Digit1", 49)):
            key(cdp, ch, code=code, vk=vk, text=ch)
        key(cdp, " ", code="Space", vk=32, text=" ")
        settle(cdp)
        textbox = ax_find(ax(cdp), "textbox", "Name") or {}
        diag = ax_find(ax(cdp), "checkbox", "Send diagnostics") or {}
        check("typed text and Space reach the app as text (Space does not activate)",
              textbox.get("props", {}).get("value") == "Hi! " and diag.get("props", {}).get("checked") == "true",
              textbox)
        key(cdp, "Backspace", vk=8)
        key(cdp, "Backspace", vk=8)
        settle(cdp)
        textbox = ax_find(ax(cdp), "textbox", "Name") or {}
        check("Backspace reaches the app as Input::Backspace", textbox.get("props", {}).get("value") == "Hi",
              textbox)
        def field_ink():
            png = Screen(base64.b64decode(cdp.send("Page.captureScreenshot", {"format": "png"})["data"]), 2)
            x, y, w, h = rect_of(cdp, 16)
            return sum(1 for px in range(int(x + 120), int(x + w), 1) for py in range(int(y + 8), int(y + h - 8), 2)
                       if sum(png.pixel(px, py)) < 300)
        ink_before = field_ink()
        cdp.send("Input.imeSetComposition", {"text": "ee", "selectionStart": 2, "selectionEnd": 2})
        settle(cdp)
        ink_composing = field_ink()
        check("IME preedit reaches the app through text_compose and draws", ink_composing > ink_before + 5,
              [ink_before, ink_composing])
        cdp.send("Input.imeSetComposition", {"text": "e", "selectionStart": 1, "selectionEnd": 1})
        composing = cdp.eval(field_value)
        cdp.send("Input.insertText", {"text": "\u1ebf"})
        settle(cdp)
        textbox = ax_find(ax(cdp), "textbox", "Name") or {}
        check("IME composition commits once through compositionend",
              composing == "Hie" and textbox.get("props", {}).get("value") == "Hi\u1ebf"
              and cdp.eval(field_value) == "Hi\u1ebf", [composing, textbox, cdp.eval(field_value)])
        cdp.eval("(() => { const n = document.querySelector('[data-astryx-id=\"16\"]');"
                 " const d = new DataTransfer(); d.setData('text/plain', ' web');"
                 " n.dispatchEvent(new InputEvent('beforeinput', {inputType: 'insertFromPaste',"
                 " dataTransfer: d, cancelable: true, bubbles: true})); })()")
        settle(cdp)
        textbox = ax_find(ax(cdp), "textbox", "Name") or {}
        check("paste reaches the app as text", textbox.get("props", {}).get("value") == "Hi\u1ebf web", textbox)

        field_caret = ("(() => { const n = document.querySelector('[data-astryx-id=\"16\"]');"
                       " return [n.selectionStart, n.selectionEnd]; })()")
        before_home = cdp.eval(field_caret)
        key(cdp, "Home", vk=36)
        settle(cdp)
        after_home = cdp.eval(field_caret)
        key(cdp, "X", code="KeyX", vk=88, text="X")
        settle(cdp)
        textbox = ax_find(ax(cdp), "textbox", "Name") or {}
        check("the hidden field's caret follows text_selection (Home, then typing at the caret)",
              before_home == [7, 7] and after_home == [0, 0] and cdp.eval(field_caret) == [1, 1]
              and textbox.get("props", {}).get("value") == "XHi\u1ebf web",
              [before_home, after_home, cdp.eval(field_caret), textbox.get("props", {}).get("value")])
        key(cdp, "End", vk=35)

        key(cdp, "Tab", vk=9)
        check("Tab moves to Save", cdp.eval(ACTIVE_ID) == "13", cdp.eval(ACTIVE_ID))
        key(cdp, "Enter", vk=13, text="\r")
        settle(cdp)
        rows = ax(cdp)
        check("Enter activates Save once (status text updates)",
              any(r["name"] == "Saved once" for r in rows), [r["name"] for r in rows if "Saved" in r["name"]])
        bar = ax_find(rows, "progressbar", "Storage used") or {}
        check("progressbar value updates to 50", bar.get("props", {}).get("value") == 50, bar)
        key(cdp, "Tab", vk=9)
        check("Tab skips the disabled Reset and moves to Help", cdp.eval(ACTIVE_ID) == "15", cdp.eval(ACTIVE_ID))
        key(cdp, "Enter", vk=13, text="\r")
        settle(cdp, 300)
        # A new tab starts blank and reports its URL in a later info change.
        opened = {e["params"]["targetInfo"]["targetId"]: e["params"]["targetInfo"] for e in cdp.events
                  if e.get("method") in ("Target.targetCreated", "Target.targetInfoChanged")
                  and e["params"]["targetInfo"].get("url", "").endswith("/help.html")}
        opened = list(opened.values())
        check("Command::OpenUrl opens a new tab without an opener", len(opened) == 1
              and opened[0].get("canAccessOpener") is not True, opened)
        # Full Chromium's --headless puts the new tab in front, which hides
        # the app's tab: no animation frames, so no gamepad polling or
        # resize handling, as in a real browser. Close it and come back.
        for tab in opened:
            cdp.send("Target.closeTarget", {"targetId": tab["targetId"]}, session=False)
        cdp.send("Page.bringToFront")
        settle(cdp, 200)
        check("the app's tab is in front again after the help tab closes",
              cdp.eval("document.visibilityState") == "visible", cdp.eval("document.visibilityState"))
        key(cdp, "Tab", vk=9)
        check("Tab past the last item leaves the app (no keyboard trap)",
              cdp.eval("document.activeElement.id") == "after", cdp.eval("document.activeElement.outerHTML"))
        key(cdp, "Tab", vk=9, shift=True)
        check("Shift+Tab from the page re-enters on the app's focused item", cdp.eval(ACTIVE_ID) == "11",
              cdp.eval(ACTIVE_ID))
        key(cdp, "Escape", code="Escape", vk=27)
        settle(cdp)

        # Assistive tech moves DOM focus and activates elements directly.
        cdp.eval("document.querySelector('[data-astryx-id=\"13\"]').focus()")
        settle(cdp)
        check("DOM focus from assistive tech moves app focus", tab_stop(cdp) == "13"
              and (ax_find(ax(cdp), "button", "Save") or {}).get("props", {}).get("focused") is True,
              tab_stop(cdp))
        cdp.eval("document.querySelector('[data-astryx-id=\"11\"]').click()")
        settle(cdp)
        rows = ax(cdp)
        wifi = ax_find(rows, "switch", "Wi-Fi") or {}
        check("click from assistive tech focuses and activates (switch turns off)",
              wifi.get("props", {}).get("checked") == "false" and tab_stop(cdp) == "11", wifi)
        cdp.eval("document.querySelector('[data-astryx-id=\"14\"]').click()")
        settle(cdp)
        check("click on a disabled button does nothing", tab_stop(cdp) == "11"
              and not any(r["name"] == "Saved 2 times" for r in ax(cdp)), tab_stop(cdp))

        # Gamepad through a stubbed Gamepad API.
        cdp.eval("window.__stubPadOn = true; window.dispatchEvent(new Event('gamepadconnected'))")
        settle(cdp, 100)
        cdp.eval("__stubPad.buttons[13].pressed = true")
        settle(cdp, 100)
        cdp.eval("__stubPad.buttons[13].pressed = false")
        settle(cdp, 100)
        check("gamepad d-pad down moves focus", tab_stop(cdp) == "12" and cdp.eval(ACTIVE_ID) == "12",
              tab_stop(cdp))
        cdp.eval("__stubPad.buttons[0].pressed = true")
        settle(cdp, 100)
        cdp.eval("__stubPad.buttons[0].pressed = false")
        settle(cdp, 100)
        settle(cdp, 100)
        diag = ax_find(ax(cdp), "checkbox", "Send diagnostics") or {}
        check("gamepad A toggles the checkbox", diag.get("props", {}).get("checked") == "false", diag)
        cdp.eval("__stubPad.axes[1] = 0.9")
        settle(cdp, 100)
        cdp.eval("__stubPad.axes[1] = 0")
        settle(cdp, 100)
        check("gamepad stick moves focus", tab_stop(cdp) == "16", tab_stop(cdp))
        # A connected pad keeps animation frames coming to poll it; with no
        # input they must not draw.
        settle(cdp, 300)
        f0 = cdp.eval(FRAMES)
        settle(cdp, 500)
        check("no frames while idle with a gamepad connected", cdp.eval(FRAMES) == f0,
              f"{f0} -> {cdp.eval(FRAMES)}")
        cdp.eval("window.__stubPadOn = false")
        settle(cdp, 100)

        # Preferences.
        cdp.send("Emulation.setEmulatedMedia", {"features": [{"name": "prefers-reduced-motion", "value": "reduce"}]})
        settle(cdp, 100)
        f0 = cdp.eval(FRAMES)
        key(cdp, "ArrowUp", vk=38)
        settle(cdp, 400)
        frames = cdp.eval(FRAMES) - f0
        check("reduced motion: focus change draws without animation", frames <= 2, f"{frames} frames")
        cdp.send("Emulation.setEmulatedMedia", {"features": [{"name": "prefers-color-scheme", "value": "dark"}]})
        settle(cdp, 200)
        dark = Screen(base64.b64decode(cdp.send("Page.captureScreenshot", {"format": "png"})["data"]), 2)
        with open(os.path.join(args.proof, "screenshot-dark.png"), "wb") as f:
            f.write(dark.png)
        check("prefers-color-scheme: dark reaches the app live", close_to(dark.pixel(4, 4), 0x18191A),
              dark.pixel(4, 4))
        cdp.send("Emulation.setEmulatedMedia", {"features": []})

        # Resize and devicePixelRatio change.
        cdp.send("Emulation.setDeviceMetricsOverride",
                 {"width": 500, "height": 700, "deviceScaleFactor": 3, "mobile": False})
        settle(cdp, 300)
        size = cdp.eval("(() => { const c = document.querySelector('#app');"
                        " return [c.width, c.height, c.clientWidth, c.clientHeight]; })()")
        check("resize to 500x700 at devicePixelRatio 3 resizes the backing store", size == [1500, 2100, 500, 700],
              size)
        rects = cdp.eval("[document.querySelector('.astryx-a11y'), document.querySelector('#app')].map(e => {"
                         " const r = e.getBoundingClientRect(); return [r.x, r.y, r.width, r.height]; })")
        check("mirror root covers the canvas after resize", rects[0] == rects[1], rects)
        small = Screen(base64.b64decode(cdp.send("Page.captureScreenshot", {"format": "png"})["data"]), 3)
        with open(os.path.join(args.proof, "screenshot-narrow-dpr3.png"), "wb") as f:
            f.write(small.png)
        x, y, w, h = rect_of(cdp, 13)
        save = small.pixel(x + 6, y + 6)
        check("after resize, pixels still match mirror positions", close_to(save, 0x0064E0), save)

        rows = ax(cdp)
        with open(os.path.join(args.proof, "ax-tree-final.txt"), "w") as f:
            f.write(format_ax(rows) + "\n")
        errors = [e for e in cdp.events if e.get("method") in ("Runtime.exceptionThrown",)
                  or (e.get("method") == "Runtime.consoleAPICalled"
                      and e["params"].get("type") in ("error", "warning"))]
        check("no console errors or exceptions", not errors, errors[:3])
        cdp.close()

    gallery_check(chrome, url.replace("index.html", "gallery.html"), args.proof, check, summary)
    if os.path.exists(os.path.join(args.dist, "pixel.html")):
        pixel_check(chrome, url.replace("index.html", ""), args.dist, args.proof, check, summary)

    summary["checks"] = check.results
    summary["passed"] = sum(r["ok"] for r in check.results)
    summary["failed"] = sum(not r["ok"] for r in check.results)
    with open(os.path.join(args.proof, "summary.json"), "w") as f:
        json.dump(summary, f, indent=2)
    print(f"\n{summary['passed']} passed, {summary['failed']} failed; proof in {args.proof}")
    server.shutdown()
    sys.exit(1 if summary["failed"] else 0)


if __name__ == "__main__":
    main()

