---
'@astryxdesign/core': patch
---

[fix] MetadataList: a numeric `columns` value is honored with stacked labels. `columns={3}` previously fell back to the responsive `repeat(auto-fill, minmax(280px, 1fr))` grid whenever labels were stacked (the default for multi-column lists), so the documented fixed column count only worked with `label={{position: 'start'}}`. The grid template now covers both label positions — `repeat(n, 1fr)` for stacked labels, `repeat(n, auto 1fr)` for side labels — and resolves through a StyleX dynamic style instead of an inline `style` object.
@cixzhang
