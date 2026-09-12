# ChartArea Night Watch audit evidence

- Component: `lab/ChartArea`
- Rubric: `1.16.6`
- Repository base: `47ba5526fa93f2bbcb7c0c26ab0a49d86cf37825`
- Exact source head: `d72f0cb76f03ddf83ee445e1dd1c699bcd31f8c9`
- Before score: `57.5 / F`, 5 BLOCKs
- After score: `70.0 / C`, 3 BLOCKs
- Visual comparison: 8 base→head pairs and 14 previous-head→new-head pairs, 0 changed pairs, byte-identical PNGs
- Review fixes: typed `colors.categorical(1)[0]` examples and active-bound hidden-table headers (`month`, `upper95`, `mean`)

## Durable reports

- `scorecard.json` — post-fix score and retained findings
- `fr5-inventory.json` — closed 22-row public-surface evidence inventory
- `fr10-report.json` — exact-head machine-readable eligibility report
- `visual-diff.json` — before/after comparison summary
- `contrast-pairs.json` — rendered light/dark/custom/RTL/narrow/touch contrast matrix
- `a11y-audit.log` — three owned stories, zero axe violations
- `rtl-audit.log` — verified N-A, zero coverage gaps
- `typechecks-exports.log` — Core, Lab, Storybook, and package export checks
- `docsite.log` — generated docsite data and 498 passing tests

## Visual matrix

Before and after receipts cover neutral light/dark, Stone light, RTL, 320px,
coarse pointer, forced colors, and baseline fallback. The final owned-story
receipts additionally cover fill-only, edge-stroke, and baseline states in light
and dark.

Every matched before/after PNG is byte-identical. The audit therefore changes
consumer guidance and durable evidence, not ChartArea paint or runtime behavior.

## Retained findings

1. `AST-002/FR15`: invalid public bound-key combinations can render no useful band.
2. `AST-002/FR10,FR15`: missing or non-numeric row values are rendered as zero.
3. `WCAG 2.2 SC 1.4.11`: meaningful band/backdrop pairs measure 1.09:1–1.30:1, below 3:1.

These require API/default/visual owner decisions and were not changed by the audit.
