# PR [#5035](https://github.com/facebook/astryx/pull/5035) — LAYOUT / CSS evidence

`fix(SegmentedControl): clamp item labels to a single line` (ernestt) · head `52099d24123a253f6e93260735c8fca8b8580c8b`
BEFORE = `52099d2412^` (the PR's own parent) · AFTER = PR head · PROBE = PR head + `minWidth: 0` on `styles.fill`.
Real Chromium (Playwright, DPR 2), Storybook 10 dev server. Screenshots in `~/astryx/review-drafts/5035-matrix/`.

**Headline: the PR fixes the wrap-and-clip bug and replaces it with an overflow bug.** In `fill`
layout the label no longer wraps, but the segments now run out of the container and the last one
is lost. One line — `minWidth: 0` on `styles.fill` — turns it into the intended ellipsis.

---

## 1. Containment assumptions

| Assumption | This change relies on | Verified |
|---|---|---|
| Parent display | `[role=radiogroup]` is `inline-flex` (`hug`) / `flex; width:100%` (`fill`) | yes — observed |
| Which element shrinks | the `<button>` flex child must shrink; the change only constrains the inner `<span>` | **no — the button cannot shrink** |
| Min-size defaults | flex children default `min-width: auto`; `styles.fill` is `{flex: 1, justifyContent: 'center'}` with no `minWidth` | yes — `getComputedStyle(button).minWidth === "auto"` in every AFTER case |
| Width source | container-defined vs auto-sizing must agree | **no — they disagree** (case 6) |
| Ancestor constraints | `overflow`, grid tracks, flex ancestors above the component | **no — an `overflow:hidden` ancestor silently eats a whole segment** (case 7); a `flex:1` ancestor re-breaks even the probe (case 6c) |
| `hug` layout | unaddressed — `hug` has no shrink axis at all, so nothing clamps | n/a, and the probe does not help |

**Which box is being constrained:** the inner `<span>`. It is the wrong box. Its `min-width: 0`
only matters once it is inside a box that is itself allowed to be narrower than its text, and the
`<button>` never is.

## 2. Render matrix

| # | Case | Before | After | Probe | What actually renders |
|---|---|---|---|---|---|
| 1 | Baseline — short labels, 600px, `hug` | [`01-baseline.before.png`](01-baseline.before.png) | [`01-baseline.after.png`](01-baseline.after.png) | same as after | **Identical, byte-for-byte.** Grid / List / Table, no change. |
| 2 | Long label, `hug`, 600px (fits) | [`02…before.png`](02-long-hug-wide.before.png) | [`02…after.png`](02-long-hug-wide.after.png) | same as after | **Identical, byte-for-byte.** One line either way. |
| 3 | Long label, `hug`, 320px | [`03…before.png`](03-long-hug-320.before.png) | [`03…after.png`](03-long-hug-320.after.png) | [`03…probe.png`](03-long-hug-320.probe.png) | **Regression.** Before: label wraps to 2 lines, pill grows taller, all 3 segments fit in 320px. After: one line, group is 394px — "Table" is off the edge and "List" is half cut. Probe is byte-identical to after: `hug` is not fixed. |
| 4 | Long label, `fill`, 320px — the known break | [`04…before.png`](04-long-fill-320.before.png) | [`04…after.png`](04-long-fill-320.after.png) | [`04…probe.png`](04-long-fill-320.probe.png) | Before: text wraps to 4 lines and is **clipped top and bottom** by the fixed-height pill — "Grid view" and "collection" are sliced in half. After: one clean line, but the group is 392px in a 320px box; "Table" is entirely gone, "List" clipped. Probe: `Grid view …`, all three segments visible, 320px exactly. |
| 5 | Mixed — one long + two short, `fill`, 320px | [`05…before.png`](05-mixed-fill-320.before.png) | [`05…after.png`](05-mixed-fill-320.after.png) | [`05…probe.png`](05-mixed-fill-320.probe.png) | **Yes, the long one starves the others.** After: "Grid" survives, the long segment takes 273px and "Table" is pushed out of view. Probe: three equal 104px segments, long one ellipsed — siblings keep their share. |
| 6 | Width source: fixed div / auto block / flex `flex:1` / grid track | [`06…before.png`](06-width-sources.before.png) | [`06…after.png`](06-width-sources.after.png) | [`06…probe.png`](06-width-sources.probe.png) | **The grandparent question — they disagree.** See below. |
| 7 | Ancestor `overflow: visible` vs `hidden` | [`07…before.png`](07-ancestor-overflow.before.png) | [`07…after.png`](07-ancestor-overflow.after.png) | [`07…probe.png`](07-ancestor-overflow.probe.png) | After/visible: "Table" escapes and paints outside the parent. After/hidden: **"Table" is silently gone** — clipped, no ellipsis, no scrollbar, no affordance. Probe: both rows identical and contained. |
| 8 | RTL, `fill`, 320px | [`08…before.png`](08-rtl-fill-320.before.png) | [`08…after.png`](08-rtl-fill-320.after.png) | [`08…probe.png`](08-rtl-fill-320.probe.png) | Mirrors LTR exactly. After: overflow runs off the **left** edge, "جدول" gone. Probe: `عرض شبك…` with the ellipsis on the logical end (left). No RTL-specific defect. |
| 9 | Text zoom 200% (root 32px), `fill`, 320px | [`09…before.png`](09-zoom200-fill-320.before.png) | [`09…after.png`](09-zoom200-fill-320.after.png) | [`09…probe.png`](09-zoom200-fill-320.probe.png) | **Worst case both ways.** Before: only the middle line "items in" is visible, the rest clipped. After: group is 663px in 320px — only the first segment is even partly on screen. Probe: `Grid …` / List / Table all legible and contained. |
| 10 | Forced colors (`forcedColors: 'active'`) | [`10…before.png`](10-forced-colors-fill-320.before.png) | [`10…after.png`](10-forced-colors-fill-320.after.png) | [`10…probe.png`](10-forced-colors-fill-320.probe.png) | Same geometry as case 4 — forced colors adds nothing new. Selected pill is Highlight/HighlightText as authored; the ellipsis renders correctly in the probe. |
| 11 | Icon + label, and `isLabelHidden` | [`11…before.png`](11-icon-paths.before.png) | [`11…after.png`](11-icon-paths.after.png) | [`11…probe.png`](11-icon-paths.probe.png) | **Icon-only is untouched** — three 104px segments, pixel-identical in all three columns. Icon + short labels `hug`: untouched. Icon + long label `fill`: same break as case 4; in the probe the icon holds its size (`flexShrink: 0`) and only the text ellipses — `Grid vi…`. |

### Case 6 detail — the grandparent decides

| Grandparent | Before | After | Probe |
|---|---|---|---|
| (a) fixed `div` 320px | wraps, clipped, fits 320 | group 392 — overflows | `Grid view …`, fits 320 |
| (b) `width: auto` block (sizes to content) | wraps at 520 | 520, no overflow — **looks fine, hides the bug** | ellipses at 520 (`Grid view of all item…`) even though there is room |
| (c) flex parent `width:320`, child `flex: 1` | fits 320 | group 394 — overflows | **still 394 — still overflows.** The wrapper div is itself a flex child with `min-width: auto`, so the fix stops at the component boundary |
| (d) grid track `200px 1fr` | 203 in a 200 track — near fit | 392 in a 200 track — blows the track | 200 exactly, `Gri…` |

Two things fall out of this. (b) means an auto-sizing parent will never show a reviewer the bug —
whether the change looks correct depends entirely on who sets the width. (c) means even the probe
is only correct while every flex ancestor has `min-width: 0`; consumers will hit this.

## 3. Probe — the minimal delta

```diff
   fill: {
     flex: 1,
     justifyContent: 'center',
+    minWidth: 0,
   },
```
`packages/core/src/SegmentedControl/SegmentedControlItem.tsx`, one line.

With it, every `fill` case above (4, 5, 6a/b/d, 7, 8, 9, 10, 11) contains itself and ellipses;
`span.scrollWidth > span.clientWidth` becomes true for the first time anywhere in this matrix.
Measured: 254px of text into an 80px box in case 4. Without it, `ellipsed` is `false` in **every
single AFTER case** — the `text-overflow: ellipsis` the PR adds never fires once.

Not fixed by the probe:
- **`hug` layout** (case 3) — no shrink axis, so a long label still pushes siblings out of a
  narrow container. Arguably out of scope, but it is now *worse* than before the PR, where it
  wrapped and fitted.
- **flex ancestors** (case 6c) — needs `min-width: 0` on the consumer's wrapper too.

## 4. On the added test

`SegmentedControl.test.tsx` asserts `getAllInjectedCss()` contains `white-space: nowrap;` and
`text-overflow: ellipsis;`. That is a substring match against the whole injected stylesheet — it
does not assert the rules apply to this element, and jsdom has no layout, so it cannot observe
whether anything truncates. It passes on the current head, where **nothing truncates in any real
browser**. A layout assertion needs Chromium.

## 5. How to reproduce this yourself

Harness (not landed — scratch files, copies alongside these PNGs):

1. `apps/storybook/stories/Review5035Matrix.stories.tsx` → title `Review/5035`, one story per case,
   each wrapped in a `[data-shot]` div that fixes the container width. Copy:
   `~/astryx/review-drafts/5035-matrix/Review5035Matrix.stories.tsx`
2. `pnpm install && pnpm -F @astryxdesign/build build`, then `storybook dev -p 6035`.
   (Aside: `pnpm -F @astryxdesign/richtext build` fails typecheck on main — `RichTextView.tsx:148`
   `className`/`style` not on `RichTextViewProps`. Unrelated to this PR, but without a
   `packages/richtext/dist` the Storybook Vite dep scan bails and every page load takes minutes.)
3. `node shoot.mjs <before|after|probe>` — copy at
   `~/astryx/review-drafts/5035-matrix/shoot.mjs`. It clips each screenshot to `[data-shot]` and
   dumps `metrics.<phase>.json`: per-item width, `scrollWidth` vs `clientWidth`, whether the item
   escapes its radiogroup, and computed `min-width` / `white-space`.

Story ids: `review-5035--baseline`, `--long-hug-wide`, `--long-hug-320`, `--long-fill-320`,
`--mixed-fill-320`, `--width-sources`, `--ancestor-overflow`, `--rtl`, `--zoom-200`,
`--forced-colors`, `--icon-paths`. Zoom is `document.documentElement.style.fontSize = '32px'`
after load; forced colors is `page.emulateMedia({forcedColors: 'active'})`. No `updateStoryArgs`
needed — every case is its own story, since Storybook 10 ignores `iframe.html?…&args=`.

The one-line check without any of this:

```js
const s = document.querySelector('[role=radiogroup] button span:last-of-type');
s.scrollWidth > s.clientWidth   // false on head, true with minWidth:0 on styles.fill
```

## Not verified

- Only the neutral theme, light mode, `size="md"`. Sizes `sm`/`lg` and other themes not shot.
- No touch/mobile viewport, no actual browser zoom (root font-size only), no Windows HCM on real
  Windows — Chromium's `forcedColors` emulation only.
- No keyboard or interaction testing; this is a pure layout pass.
- Did not check whether a truncated label needs a `title`/tooltip so the full text stays
  reachable — worth deciding, since the probe makes truncation the normal outcome.
- Everything above is observed in Chromium. The only claim read off source is the content of
  `styles.fill`.
