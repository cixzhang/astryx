# Chart audit evidence

- Pull request: [facebook/astryx#6247](https://github.com/facebook/astryx/pull/6247)
- Rubric: v1.16.2
- Base: `41ab6db2a5d193739759ad2ba14b6be21509a2d7`
- Exact head: `29efe4a12e97f9b7f7fb85a546ced3ff28cd2f7c`
- Score: **58.1 / F → 78.0 / C**
- Retained BLOCK findings: **4**

## Visual evidence

- [Before/after contact sheet](./Chart__before-after__contact-sheet.png)
- [`matrix/`](./matrix/) — exact-head screenshots and sanitized canonical sensor receipts
- [Visual pixel diff](./reports/visual-diff.json) — 24 matched pairs, 0 changed pixels
- [State-visual conformance matrix](./reports/state-visual-matrix.json)
- [Rendered contrast matrix](./reports/contrast-matrix.json)

## Accessibility, RTL, and checks

- [Package-specific axe report](./reports/charts-chart-a11y.json) — 34 stories, 0 violations, 0 page errors
- [Package-specific RTL report](./reports/charts-chart-rtl.json)
- [Stock a11y report](./reports/standard-a11y-chart.json)
- [Stock RTL report](./reports/standard-rtl-chart.json) — records the current `lab/Chart` routing gap
- Exact-head GitHub CI: all terminal; `pr-rtl` is the sole failure and reproduces retained I17/V11 (Chart + ChartLegend resolve to `lab/chart` plus one uncovered component). All other checks passed.
- [Local and GitHub check summary](./reports/checks.json)
- [Scorecard](./reports/scorecard.json)
- [Closed FR5 inventory](./inventory.md)
- [FR10 exact-head eligibility report](./reports/fr10-report.json)

The evidence branch is asset-only and is not merged into the repository.
