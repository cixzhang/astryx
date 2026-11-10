# Rust spike measurement harness

Measures both stacks against the same Astryx baseline. Nothing here ships.

| Script | What it does |
| --- | --- |
| `baseline/` | The Astryx React gallery that every stack is diffed against. |
| `gen-cases.mjs` | Writes `cases/<Family>.json` from Astryx `{Name}.doc.mjs` files: one case per value of each visual prop (single-prop sweep) plus a pairwise covering set. Ids are stable for a given doc. |
| `capture.mjs` | Captures one PNG per case, scheme (light, dark) and width (390, 1280) from a web host: the Astryx Storybook baseline, the Exact web host, or our stack's web build. Crops to the rendered component plus 16 px. Records each case's accessibility snapshot. |
| `capture-desktop.mjs` | The same captures from our stack's Linux gallery binary, under a virtual X display. |
| `diff.mjs` | Diffs a host's captures against the baseline. A shot passes when at most 2% of its pixels differ by more than the channel tolerance. |
| `perf.mjs` | First frame, transfer size and frame times during a scripted drag or scroll. |
| `scorecard.mjs` | Builds the 12-metric scorecard from the outputs above and `reports.json`. |

## Conventions for galleries

Each stack's gallery renders exactly one case per page or run.

- Cases come from `cases/<Family>.json`. A case's `props` are the component's props. Each
  component block has a `fixture`: `"boxes"` means three children, each a 40 × 24 box with
  a 4 px radius on the theme's muted background; `"text"` means the child text `Label`;
  `null` means no children. Content props use fixed samples (`label: "Label"`,
  `name: "Ada Lovelace"`).
- The baseline is Astryx React itself: `baseline/` renders every case with
  `@astryxdesign/core` under this contract.
- Theme: Astryx neutral. The case sits in a box with 16 px padding on the theme's
  surface background (`--color-background-surface`).
- Web: `?case=<id>&scheme=light|dark&theme=neutral`. The case box has `data-testid="case"`
  (or `id="case"`). Set `performance.mark("qa:first-frame")` at the first painted frame.
- Desktop: the binary reads `ASTRYX_CASE`, `ASTRYX_SCHEME`, `ASTRYX_THEME`,
  `ASTRYX_SIZE` (`<w>x<h>`), `ASTRYX_SHOT` (PNG path) and `ASTRYX_A11Y` (JSON path for the
  accessibility tree), exits after the first settled frame, and prints
  `first_frame_ms=<n>`.

## Running

```sh
node gen-cases.mjs --astryx <astryx checkout> --out cases Button Switch TextInput
node capture.mjs --host storybook --base http://127.0.0.1:6006 --family Button --jobs 24
node capture.mjs --host ours-web --base http://127.0.0.1:4200/gallery.html --family Button
node diff.mjs --host ours-web --family Button
node scorecard.mjs > scorecard.html
```

`capture.mjs`, `diff.mjs` and `perf.mjs` need `playwright-core` (set `PLAYWRIGHT_CORE` to its
module path) and a Chromium binary in `CHROME`.
