# Review frames — toast render surface (#5428)

Captured in real Chromium against `storybook dev` at the PR head
`1fc9089471cf15fb5c5a544e38349bfba615e1e4`, and against the warm `main`
baseline for the before.

| frame | what it shows |
|---|---|
| `shots/main-default__shown.png` | default toast on `main` — the before |
| `shots/default__shown.png` | default toast at this head — identical |
| `shots/custom__shown.png` | a product surface drawn through `renderToast` |
| `shots/custom__empty.png` | the viewport with nothing in it |

`shots/stuck__no-dismiss.png` — an error toast under a renderer with no close, ten seconds after Escape and F6. `shots/control__builtin-card.png` — the same toast, Astryx's card.

The exit-transition comparison is a timeline, not a frame: a screenshot costs
longer than the 150ms transition, so any shot of it lands after it. Numbers are
in the review.
