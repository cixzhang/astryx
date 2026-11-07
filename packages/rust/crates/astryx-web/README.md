# astryx-web

Runs an Astryx Rust app in a browser `<canvas>` through WebAssembly. It draws
with `astryx-render-gl` on WebGL2 and mirrors each frame's accessibility tree
into real DOM elements, so screen readers, switch access, and magnifiers can
read and operate the app.

```rust,ignore
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let options = astryx_web::Options { fonts_url: Some("fonts/".into()), ..Default::default() };
    astryx_web::run_with("#app", my_app_surface, options)
}
```

Put the canvas in a positioned box (`position: relative`). The shell inserts
the mirror right after the canvas and places it over the canvas inside that
box. `web/index.html` is a minimal host page.

## Build, check, measure

```sh
scripts/build.sh                # dist/: demo.js, demo_bg.wasm, index.html, fonts/
scripts/size-report.sh          # raw, gzip -9, and brotli sizes
scripts/browser-check.py        # headless Chromium: pixels, keys, gamepad, AX tree
scripts/browser-check.py --timing 9   # time to first frame, median of 9 loads
```

`build.sh` builds with the `web-release` profile and needs `wasm-bindgen-cli`
at the same version as the `wasm-bindgen` crate and Python fontTools. It uses
`wasm-opt` when installed. `BASELINE=1` builds with the plain release profile
and skips `wasm-opt`. `browser-check.py` uses
only the Python standard library and any headless Chromium; it writes
screenshots, the AX tree, and `summary.json` to `proof/`.

The demo in `examples/demo.rs` is a fixed `Surface` (a settings panel) that
paints and reports its tree by hand. It tests the shell without the runtime.

## What the shell does

- **Canvas size.** The backing store is CSS size times `devicePixelRatio`,
  from `ResizeObserver` (`device-pixel-content-box` where supported). Zoom and
  moving between screens re-size it.
- **Frames on demand.** A frame draws after input, resize, or a preference
  change, and keeps drawing only while `Surface::wants_frames()` is true. While
  a gamepad is connected the shell polls it each animation frame but draws only
  when something changed.
- **Keyboard.** Arrows, Tab and Shift+Tab (`Next`, `Previous`), Enter and Space
  (`A`), Escape (`B`). Chords with Ctrl, Alt, or Meta stay with the browser.
- **Gamepad.** W3C standard mapping: A, B, X, Y, shoulders, triggers, Select,
  Start, d-pad, home (`Menu`), and the left stick with hysteresis. Held
  directions repeat after 400 ms, every 90 ms.
- **Preferences.** The shell reads `prefers-reduced-motion`,
  `prefers-color-scheme`, `prefers-contrast`, and `forced-colors`, and calls
  `Surface::set_preferences` before the first frame and on every change.
  `detect_preferences()` reads them directly.
- **Fonts.** `scripts/subset-fonts.py` splits each Roboto face into the script
  subsets in `fonts/ranges.txt` (CSS `unicode-range` style). With
  `Options::fonts_url` set, the first frame waits for the Latin subsets of all
  faces (the host page preloads them with the wasm); other subsets load when
  drawn text needs them, only for the faces that text uses, and the next frame
  re-lays out. Until astryx-raster takes font data at runtime, the renderer
  draws with its built-in fonts and the loaded data waits in the shell.

## Accessibility: the DOM mirror

Each `A11yNode` becomes a transparent element at the node's bounds:

| Role | Element |
|---|---|
| Group | `div role=group` |
| Text | `div` with the text |
| Heading(n) | `h1`..`h6` |
| Button | `button` (`aria-pressed` when `checked` is set) |
| Switch | `button role=switch aria-checked` |
| Checkbox | `button role=checkbox aria-checked` |
| ProgressBar | `div role=progressbar`, `aria-valuenow` 0..100 from "40", "40%", or "0.4"; otherwise `aria-valuetext` |
| TextInput | `input type=text readonly` with the value |
| List / ListItem | `div role=list` / `div role=listitem` |
| Image | `div role=img aria-label` |

`disabled` becomes `aria-disabled` (the element stays focusable, as in the
app). `focusable` nodes get a `tabindex`. `live` becomes `aria-live`
(`polite` or `assertive`) with `aria-atomic=true`. Elements are keyed by node id and updated in place, so a screen reader's
position survives frames. The canvas gets `aria-hidden`.

Focus is one tab stop with a roving `tabindex`:

- App to DOM: when the app moves focus and the page focus is in the app, DOM
  focus follows to the new element (`preventScroll`).
- DOM to app: DOM focus on a mirror element (Tab from the page, a screen
  reader, a script) sends `Surface::a11y_action(id, Focus)`.
- Activation from assistive tech arrives as a `click` and sends `Focus`, then
  `Activate`. The `click` a button fires after Enter or Space is ignored,
  because the key already reached the app as `A`.
- No keyboard trap: when Tab or Shift+Tab does not move app focus forward or
  backward (end of order, or the app wraps), the browser's own Tab moves focus
  out of the canvas. Tabbing back in lands on the app's focused element.
- With no focusable nodes, the mirror root takes the tab stop as
  `role=application` with `Options::label`, so keys still reach the app.

### What works and what does not

Checked in headless Chromium unless the note says otherwise.

| Capability | Status | Notes |
|---|---|---|
| Roles, names, values, states in the AX tree | Works | Verified with `Accessibility.getFullAXTree`. |
| Screen reader focus and activation | Works | Focus and click on mirror elements reach the app; not yet tried with NVDA, JAWS, VoiceOver, TalkBack. |
| Keyboard operation, no trap | Works | Tab enters, moves, and leaves the app. |
| Heading and list navigation | Works | Native `h1`..`h6`; list item counts come from the browser. |
| Positions for touch exploration and magnifiers | Partial | Bounds match the pixels (checked at DPR 2 and 3); not tried on a device. A layout shift that moves the canvas without resizing it is followed only if the canvas and mirror share a positioned box. |
| Reduced motion, color scheme, contrast, forced colors | Partial | Reported to the app; the app decides. The mirror stays transparent in forced-colors mode. |
| Browser find (Ctrl+F) | Partial | Finds mirror text; the highlight lands on the canvas text but the canvas does not scroll or highlight itself. |
| Status announcements (live regions) | Works | `aria-live` and `aria-atomic` are in the AX tree; not yet heard through a screen reader. |
| Text entry, IME, autofill, spellcheck | Missing | The contract has no text input; text fields are read-only. |
| Text selection and copy | Missing | Canvas text cannot be selected. |
| Page translation | Missing | Translation changes the mirror, not the pixels. |
| User font size, text-only zoom | Missing | Canvas text ignores them; page zoom works. |
| Descriptions, expanded, selected, ranges beyond 0..100, relations | Missing | Not in `A11yNode`. |
| Pointer and touch input | Missing | Not in the contract yet. A click on the canvas only gives the app keyboard focus. |
| Per-node language and text direction | Missing | Not in `A11yNode`; the mirror inherits the page. |
