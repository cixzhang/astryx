# Native infra: evidence and run commands

The infra rows of the spike ledger on our stack: Foundations, Layers,
Motion, Gestures, and Virtualizer. Crates live in `../crates/`; changes to
`packages/rust` are listed in `../RUST-CHANGES.md`; the per-file closure
disposition is in `CLOSURE.md`.

## Crates

| Crate | Row | What it gives components |
| --- | --- | --- |
| `astryx-spike-foundations` | Foundations | Brands and modes (`ThemeState`), extra tokens, i18n (`I18n`: 30 Astryx catalogs, ICU messages, plurals, ICU4X number, date and list formats, direction), focus models (`RovingFocus`, `Typeahead`, `GridFocus`, `VirtualFocus`), the size cascade, and `CxExt` to read them while building |
| `astryx-spike-layers` | Layers | `Layer` (popover, dialog, tooltip, menu at a point), Astryx placement (logical sides, flips, slide, viewport gutter), `Hover` (hover intent) |
| `astryx-spike-motion` | Motion | Astryx motion from the theme's motion tokens: layer enter and exit, the `stateChange` spring, collapse, color changes |
| `astryx-spike-gestures` | Gestures | The release velocity filter, pan, arena arbitration, momentum, rubber band, snap points, reorder; `Drag` and `SnapDrag` on the runtime |
| `astryx-spike-virtual` | Virtualizer | `VirtualList` (windowed rows, measured heights, scroll anchoring, row count and index for assistive technology, `ScrollMemory`) |
| `astryx-spike-infra-gallery`, `-web` | All | One screen per row on desktop and web |

## Done criteria (ledger) and evidence

| Row | Criterion | Status | Evidence |
| --- | --- | --- | --- |
| Foundations | All used tokens expressed | Done | `tools/token-coverage.py`: 181 of 181 tokens the core closure reads |
| Foundations | Runtime theme switch | Done | `ThemeState` (8 brands, light and dark); `theme` tests; `infra-theme` screen with `scheme` and `theme` parameters |
| Foundations | Plural and ICU messages correct in en and ar | Done | `i18n` tests: all 30 catalogs parse; Arabic uses all six plural categories; `infra-i18n-en`, `-ar`, `-fr` screens |
| Foundations | Focus ring and size cascade match | Code done; pixels by QA | Rings show only for keyboard focus (`:focus-visible`); size cascade tests in `cx.rs` |
| Layers | Escape closes popover then dialog; focus returns to each opener; outside click closes only the popover; page behind inert; one exit per close | Done | `astryx-spike-layers` tests (runtime-driven); DOM mirror shows the dialog `aria-modal` with the page left out; `infra-layers-popover-dialog` screen |
| Virtualizer | 10,000 rows with fewer than 100 rendered | Done | `astryx-spike-virtual` tests; `infra-virtual-10k` screen |
| Virtualizer | p95 frame of 16.7 ms or less on fling | Runtime side done | `fling` example: runtime frame p95 0.92 ms over a 246-frame touch fling (release build, without rasterizing); full frame by QA |
| Virtualizer | Scroll restore | Done | `the_offset_comes_back_when_the_list_returns` (anchored to the row at the top) |
| Virtualizer | "Row N of 10,000" announced | Exposed | AccessKit `row_count`/`row_index` and DOM `aria-rowcount`/`aria-rowindex` (checked in Chrome with `scripts/mirror.mjs`); screen reader listening by QA |
| Motion | Settle within 10% of Astryx's spring | Done | The runtime's stepped spring against Astryx's closed-form spring: within 10% of travel at every frame and of settle time |
| Motion | Retarget without jumps | Done | `a_toggle_retargets_from_where_it_is_without_a_jump` |
| Motion | Exit plays once | Done | One exit copy per node, replaced by a second exit, removed when it settles |
| Motion | Reduce Motion: no transform animation | Done | Travel lands at once; on touch screens opacity cross-fades |
| Gestures | Release velocity picks Astryx's snap point | Done | Sheet test: a flick carries to the next point; slow release returns; Astryx's `resolveSnapPoint` rules |
| Gestures | Inner list scrolls first | Done | Sheet over a list: the list scrolls back first, the sheet drags from the list top; mouse drags do not scroll |
| Gestures | Reorder by pointer and keyboard | Model done; component by UI lane | `Reorder` (grab, move, drop announcements; pointer drag to a slot) |
| Gestures | p95 of 16.7 ms or less | Runtime side done | `fling` example: sheet drag runtime frame p95 0.05 ms |

## Run commands

From `packages/rust-spike/native`:

```sh
# Tests of every infra crate.
cargo test -p astryx-spike-foundations -p astryx-spike-layers -p astryx-spike-motion \
  -p astryx-spike-gestures -p astryx-spike-virtual
# Desktop screens under xvfb: one case, settled, written to a PNG.
cargo build -p astryx-spike-infra-gallery --example infra_desktop
ASTRYX_CASE=infra-layers-popover-dialog ASTRYX_SCHEME=light ASTRYX_SIZE=640x560 \
  ASTRYX_SHOT=out.png ASTRYX_A11Y=out.json \
  xvfb-run -a -s "-screen 0 1280x800x24" target/debug/examples/infra_desktop
# Web screens: build, serve dist/, open infra.html?case=<id>&scheme=light|dark&dir=ltr|rtl&theme=<brand>.
crates/astryx-spike-infra-gallery-web/scripts/build.sh
# The web accessibility mirror of a case (needs playwright-core and Chrome).
node crates/astryx-spike-infra-gallery-web/scripts/mirror.mjs "http://localhost:8765/infra.html?case=infra-virtual-10k" "$CHROME"
# Frame cost of a 10,000-row fling and a sheet drag.
cargo run --release -p astryx-spike-infra-gallery --example fling
```

Cases: `infra-theme`, `infra-i18n-en`, `infra-i18n-ar`, `infra-i18n-fr`,
`infra-focus-menu`, `infra-layers-popover-dialog`, `infra-layers-tooltip`,
`infra-layers-context-menu`, `infra-motion-switch`,
`infra-motion-collapsible`, `infra-gestures-sheet`,
`infra-gestures-reorder`, `infra-virtual-10k`.

## Known gaps

- Software keyboard clearance, mobile keyboard model, and Android motion:
  no mobile shell is in scope.
- Calendar systems, collation, and date and time parsing: the date input
  family's own work.
- Menu press-and-drag across items, haptics, and edge autoscroll.
- Touch-specific layer press rules (a press that opens a layer does not
  also act inside it on touch).
- Dynamic type (text that scales with the OS setting).
