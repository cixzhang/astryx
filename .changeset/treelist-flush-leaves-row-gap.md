---
'@astryxdesign/core': patch
---

[feat] TreeList: two additive changes. (1) A fully flat tree — one with no expandable items at all — now renders its rows flush instead of reserving an empty chevron-alignment column that nothing lines up under; any tree that has at least one expandable item keeps the existing alignment for every row, so hierarchical trees render exactly as before. (2) Adds a themeable `--tree-list-row-gap` for the inter-row gap, defaulting to a subtle `2px` (`var(--spacing-0-5)`) separation between rows; set it on the `tree-list` target to widen or close it. The connector guides span the gap automatically and no longer overhang the last row. (#4540)
@freddymeta
