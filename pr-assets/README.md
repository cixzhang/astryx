# ChartArea Night Watch audit evidence

Rubric **1.16.6** · [PR #6282](https://github.com/facebook/astryx/pull/6282) · base `47ba5526fa93f2bbcb7c0c26ab0a49d86cf37825` · exact source head `f369c96a3a10620d3df8b4bcd59a8034747f04ac`

Independent review found two blockers on the prior reviewed head. Both are fixed; all GitHub Actions CI jobs pass on this exact head, while final `review-required` and `visual-acceptance` remain pending.

The copyable examples now use `colors.categorical(1)[0]`. The upper-plus-baseline receipt proves the hidden table headers are exactly `month`, `upper95`, and `mean`.

- [Post-fix scorecard](scorecard.json)
- [Before scorecard](before-scorecard.json)
- [Closed FR5 inventory](fr5-inventory.json)
- [FR10 eligibility report](fr10-report.json)
- [Visual diff report](visual-diff.json)
- [Rendered contrast pairs](contrast-pairs.json)

## Unchanged rendered output

The audit adds documentation, a draft observational contract, and an owned Storybook fixture. It does not change ChartArea runtime paint. All eight matched before/after PNG pairs are byte-identical.

| Before | Exact head |
|---|---|
| ![Before, neutral light](before/ChartArea__confidence-neutral-light.png) | ![After, neutral light](after/ChartArea__confidence-neutral-light.png) |
| ![Before, neutral dark](before/ChartArea__confidence-neutral-dark.png) | ![After, neutral dark](after/ChartArea__confidence-neutral-dark.png) |
| ![Before, narrow](before/ChartArea__confidence-neutral-narrow.png) | ![After, narrow](after/ChartArea__confidence-neutral-narrow.png) |

## Exact-head owned-story coverage

| Fill only | Edge stroke | Baseline fallback |
|---|---|---|
| ![Fill-only band](after/ChartArea__band__neutral-light.png) | ![Band with edge stroke](after/ChartArea__edge-stroke__neutral-light.png) | ![Upper bound against baseline](after/ChartArea__baseline__neutral-light.png) |

Canonical sensor receipts sit beside every PNG. Additional frames cover dark mode, Stone, RTL, coarse pointer, and forced colors.
