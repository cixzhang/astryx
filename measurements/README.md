# Raw measurement output for #5408

Chromium, driven with Playwright against a Storybook dev server rendering the
real component, canvas and SVG in the same page.

| file | what |
|---|---|
| `verify.json` | computed-style reads, style recalcs at mount (N=1/8/20/38), 3 s rotation deltas, phase spread, colour-after-mount, compositing layers |
| `pin-batching.json` | style recalcs with the timeline pin applied one ring at a time vs batched into one frame |
| `phase-refcallback.json` · `phase-raf.json` · `phase-waapi.json` · `phase-batched.json` | max phase separation of four spinners mounted 260 ms apart, at 1x / 4x / 10x CPU throttling, three runs each — one file per mechanism tried |
| `matrix-parity.json` | canvas vs SVG on the real component: 32 cells (4 sizes x 4 shades x DPR 1 and 2), ink pixels, ink mass, arc centroid angle, outer radius, box size |
| `spike-parity.json` | the same comparison off-Storybook at DPR 1 / 1.25 / 1.5 / 1.75 / 2, plus 24-angle rotation stability, plus the rejected two-node variant |
| `gcs-attribution-*.txt` | every `getComputedStyle` call landing on a spinner, with the JS stack that made it |
| `css-geometry.txt` | whether an SVG ring can take a themed diameter and rail width from CSS with no JS (context for #5214) |
