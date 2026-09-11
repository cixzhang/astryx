# Chart FR5 public-surface inventory

Inventory closure: **25/25 rows accounted for** on `29efe4a12e97f9b7f7fb85a546ced3ff28cd2f7c`. Gaps are explicit; none are omitted or treated as policy.

| # | Public surface / reachable state | Source and standing evidence | Result |
|---:|---|---|---|
| 1 | `Chart` and `ChartProps` package export | `src/index.ts`, package typecheck/export checks, root regression test | Mapped; BaseProps/ref fixed. |
| 2 | Required `data`, `xKey`, `series` | Chart/marks tests, README, root story | Mapped. |
| 3 | Width measurement: zero-width placeholder → measured SVG | source, measurement test, browser geometry | Mapped; per-instance observer FIX retained. |
| 4 | `height` default and invalid-value clamp | source, placeholder geometry test, Storybook control | Mapped. |
| 5 | `margin` partial override | source and geometry test | Mapped; physical-name BLOCK retained. |
| 6 | `yBaseline`: `auto`, `zero`, `data` | layout source, mark tests/stories | Mapped. |
| 7 | Explicit `yDomain` precedence | layout source and streaming story | Mapped; root-level test gap recorded. |
| 8 | Numeric `xDomain`; categorical ignore | layout source and streaming story | Mapped. |
| 9 | `grid` slot before marks | render order, ChartGrid tests/stories | Mapped. |
| 10 | `axes` slot after marks | render order, ChartAxis tests/stories | Mapped. |
| 11 | `legend` off / true / config | root + legend tests; all-position receipts | Mapped. |
| 12 | `tooltip` off / true / config | root + tooltip tests; hover receipts | Mapped; tap/coarse-pointer BLOCK retained. |
| 13 | `interactions` slot | render order and interactive story | Mapped; stable root integration test gap recorded. |
| 14 | `children` SVG escape hatch | render order and consumer docs | Mapped. |
| 15 | Supplied `title` | accessible-name test and light/dark receipts | Mapped. |
| 16 | Supplied `subtitle` | description test and light/dark receipts | Mapped. |
| 17 | Generated accessible name | localized regression test and receipt | Mapped and fixed. |
| 18 | Small-data hidden table | table semantics tests and package axe | Mapped. |
| 19 | Empty data | finite fallback scales and empty-state receipts | Mapped; representation remains Needs Review. |
| 20 | More than 100 cells | cutoff test and large-data receipts | Mapped; equivalent-alternative BLOCK retained. |
| 21 | Mark clipping and render order | clipped-group/event-layer tests and visual matrix | Mapped. |
| 22 | Pointer move / nearest-x / leave | source, tooltip tests, hover receipts | Mapped; root integration-test FIX retained. |
| 23 | Auto palette / utility-mark exclusion | palette test, auto-palette receipts, contrast matrix | Mapped; public SeriesDef metadata BLOCK and token-system gap recorded. |
| 24 | Legend top/bottom/start/end | root fixture, legend tests, LTR/RTL receipts | Mapped. |
| 25 | RTL / 320 px / coarse pointer | canonical receipts and package-specific probes | Mapped; stock RTL routing gap and tap failure retained. |
