---
'@astryxdesign/core': patch
---

[feat] Pagination: add an `input` variant — an editable page-number box (a `NumberInput`, so it clamps to `[1, totalPages]` with integer-only semantics) flanked by first/last («/») buttons, rendering `Page [ n ] / N`. Navigation is page-based via the existing `onChange`. The leading noun is set with an open `pageLabel` prop (defaults to the localized "Page"; pass `pageLabel="Row"` to relabel it). Also adds a `step` prop controlling how many pages the prev/next buttons advance per click (default 1, clamped to range); when greater than 1 the buttons' accessible names reflect the stride. Adds `chevronsLeft`/`chevronsRight` icons. The first/last/prev/next carets now also carry a hover tooltip (the same localized, step-aware label already used as their accessible name), so sighted users get the affordance the icon-only buttons previously exposed only to assistive tech. (#4248)

@freddymeta
