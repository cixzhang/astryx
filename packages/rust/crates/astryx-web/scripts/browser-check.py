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
        check("list 'Recent files' with 6 list items", ax_find(rows, "list", "Recent files") is not None
              and sum(1 for r in rows if r["role"] == "listitem") == 6)
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
        latin = [f"Roboto-{w}.latin.ttf" for w in ("Regular", "Medium", "Bold")]
        check("Latin subsets of all three faces load before the first frame",
              all(n in loaded and loaded[n][1] <= first for n in latin), {n: loaded.get(n) for n in latin})
        on_demand = [f"Roboto-Regular.{s}.ttf" for s in ("cyrillic", "greek", "vietnamese")]
        check("Cyrillic, Greek, and Vietnamese load after the first frame, Regular only",
              all(n in loaded and loaded[n][0] >= first for n in on_demand) and len(loaded) == 6,
              sorted(loaded))
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
        check("Tab moves to Save", cdp.eval(ACTIVE_ID) == "13", cdp.eval(ACTIVE_ID))
        key(cdp, "Enter", vk=13, text="\r")
        settle(cdp)
        rows = ax(cdp)
        check("Enter activates Save once (status text updates)",
              any(r["name"] == "Saved once" for r in rows), [r["name"] for r in rows if "Saved" in r["name"]])
        bar = ax_find(rows, "progressbar", "Storage used") or {}
        check("progressbar value updates to 50", bar.get("props", {}).get("value") == 50, bar)
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
        check("gamepad stick moves focus", tab_stop(cdp) == "13", tab_stop(cdp))
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

