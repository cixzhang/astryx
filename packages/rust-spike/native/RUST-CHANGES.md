# Changes to packages/rust made on this branch

The spike changes `packages/rust` only where an out-of-crate component or
infra crate cannot work otherwise. Each change is listed here for a later
proposal to the `rust` branch. "Additive" means new types, fields with
defaults, or methods; existing callers and the focus-first paths behave as
before. Every push runs the full `packages/rust` workspace tests,
including the pixel goldens.

## Proposal groups

The changes cut into separate atomic proposals. Commits on this branch
keep to one group each from here on; commit 60edfb1 mixes groups a to d
and splits by hunk along the rows below.

- **a. Layers**: `layer.rs`, `Node::{layer, hover}`, `BuildCx::{layer,
  scope, in_scope, now_ms, last_rects}`, `Layout::layers`, the runtime's
  layer placement, dismissal, focus restore and hover intent, the topmost
  trap, the modal-inert a11y tree, and `Surface::pointer` with the shells'
  press forwarding.
- **b. Focus-visible ring**: focus visibility in the runtime, rings only
  while visible, `Runtime::hide_focus_until_keyboard`.
- **c. Accessibility roles and states**: `Role` variants, `A11yState`,
  `Semantics::{state, refs}`, relation resolution, the covering rule, and
  the AccessKit and DOM-mirror bridges.
- **d. Draw variants and backdrop**: `Node::variants`, `When`, the paint
  state lookup, and the modal backdrop.
- **e. Motion**: `motion.rs` (`Anim`, `Channel`, `Exit`, `Resize`),
  `Node::{anims, exit, resize, inert, animate}`, `Layout::override_height`,
  the runtime's per-node animations (retargeting, entrance, Reduce Motion
  through the existing `MotionPolicy`), exit copies captured from the last
  paint, height animation with reflow, and the paint opacity channel.
- **f. Component state**: `Locals`, `LocalHandle`, `BuildCx::{locals,
  next_id, local_handle, local}`, `Interact::effect`,
  `LayerSpec::dismiss_effect`, `HoverSpec::effect` (and `on_change`
  becoming optional), and the runtime's state store, pruned with the
  node tree.
- **g. Drags**: `gesture.rs` (`DragSpec`, `DragEvent`, `DragPhase`),
  `Node::drag`, `PointerEvent::time_ms`, the runtime's drag sessions
  (innermost member that wants the direction takes the drag; finger
  scrolling with a coast), and the shells' move forwarding while a button
  is held.
- **h. Keys, context press, focus requests**: `Key`, `Modifiers`,
  `KeyEvent`, `KeyOutcome`, `Surface::key`; `Node::{on_key, on_context}`,
  `KeyResponse`; `BuildCx::focus_requests`, `FocusRequests`; the shells'
  key and right-click forwarding.
- **i. Scroll hooks for windowing**: `Node::{rebuild_on_scroll,
  initial_scroll, scroll_report, scroll_adjust}`,
  `BuildCx::{scroll_offsets, scroll_offset}`. Groups h and i share one
  commit; they split by the rows below.

## Changes

| File | Change | Why | Kind |
| --- | --- | --- | --- |
| astryx-core `lib.rs`, `element.rs`, `components/text.rs`, `light.rs`, `world.rs` | `pub mod authoring` re-exports the build types (Build, BuildCx, Node, Draw, …) | Components outside the crate | Additive (visibility) |
| astryx-core `element.rs` | `Draw::Paint(Painter)`, a custom painter | UI components with custom drawing | Additive |
| astryx-core `element.rs` | Typed context: `Element::provide`, `BuildCx::{provide, unprovide, context}` | Locale, size cascade, group context through slots | Additive |
| astryx-core `contract.rs` | 36 new `Role` variants; `A11yState` with ARIA states and relations; `HasPopup`, `Current`, `Sort`, `A11yOrientation` | Closure roles (dialog, menu, grid, tree, landmarks, …) and states (row count, activedescendant, …) | Additive (new enum variants: exhaustive matches outside the crate must add arms) |
| astryx-core `contract.rs` | `A11yNode::state` field | Carries the states | Additive (struct literals add `state: Default::default()`) |
| astryx-core `element.rs` | `Semantics::{state, refs}`, `A11yRefs`, `Semantics::{role, with_state, with_refs}` | States and relations by element id | Additive |
| astryx-core `a11y.rs` | Relations resolved from id props; a labeled node covers its children only for roles named from content | Labeled containers (a group of avatars) hid their children | Behavior change (a11y tree only; no existing test or golden changed) |
| astryx-core `a11y.rs` | The page behind a modal layer is left out of the tree | Inert page behind dialogs | Additive (only with modal layers) |
| astryx-desktop `a11y.rs`, astryx-web `mirror.rs` | Bridge new roles and states; mirror elements get DOM ids `ax-<id>` | Platform accessibility | Additive |
| astryx-core `layer.rs` (new), `element.rs` | `LayerSpec`, `LayerAnchor`, `DismissReason`, `PlaceInput`, `HoverSpec`, `When`; `Node::{layer, hover, variants}`; `BuildCx::{layer, scope, in_scope, now_ms, last_rects, last_rect}`; `Node::variant` | Layers, hover intent, paint states | Additive |
| astryx-core `layout.rs` | `Layout::layers` offsets applied in `placed` | Placing layers after layout | Additive (empty unless layers exist) |
| astryx-core `runtime.rs` | Overlays built until none are queued (nested layers); layer placement, Escape to the topmost layer, outside-press dismissal, focus restore, hover tracking | Layers | Additive (inactive without layers or hover specs) |
| astryx-core `runtime.rs` | Focus stops come from the topmost trap only | A popover over a dialog takes focus | Behavior change only with two or more traps (none existed before) |
| astryx-core `contract.rs`, `runtime.rs`, astryx-desktop `lib.rs`, astryx-web `shell.rs` | `PointerEvent`, `PointerOutcome`, `Surface::pointer` (default no-op); shells send presses (down, up) before their existing `PressStart` mapping | A press outside any focus stop (light dismiss); later, drags for gestures. Hover stays on the existing `pointer_move`; hover intent (`HoverSpec`) reads that same pointer position | Additive |
| astryx-core `runtime.rs`, `paint.rs` | Focus visibility: a pointer press makes focus not visible until the next arrow or Tab; rings paint only while visible; `Runtime::hide_focus_until_keyboard` | `:focus-visible` parity on web and desktop | Behavior change for pointer presses only; focus-first devices (no pointer) are unchanged |
| astryx-core `paint.rs` | Draw variants by paint state; modal backdrop | Hover tints and focus styles on any node | Additive |
| astryx-core `motion.rs` (new), `element.rs` | `Anim`, `Channel`, `Exit`, `Resize`; `Node::{anims, exit, resize, inert}`, `Node::animate` | Component motion on astryx-motion (Switch thumb, layer enter and exit, Collapsible) | Additive |
| astryx-core `runtime.rs`, `paint.rs`, `layout.rs`, `a11y.rs` | One `Animated` per node and channel kept across rebuilds; exit copies from the last paint, drawn above everything, one per node; height animation with a second layout pass while it moves; `inert` subtrees out of the a11y tree and focus order; `paint` takes a capture list | Motion row: retarget without jumps, exit plays once, layout animation, Reduce Motion | Additive (inactive without anims, exit, resize or inert) |
| astryx-core `element.rs`, `layer.rs`, `runtime.rs` | Component state kept by the runtime per node id and type (`LocalHandle`), changed from `Interact::effect`, `LayerSpec::dismiss_effect` and `HoverSpec::effect`; a change rebuilds; state is dropped with its node | Astryx's uncontrolled APIs (Tooltip, Popover, Collapsible `defaultIsOpen`, Table default sort) without an app message per bit of UI state | Additive; `HoverSpec::on_change` is now optional (introduced on this branch) |
| astryx-core `runtime.rs` | The build sees the frame's viewport (it saw (0, 0) on the first frame), and a viewport change rebuilds | Layers and sheets size from `BuildCx::viewport` | Bug fix (one more rebuild per resize) |
| astryx-core `gesture.rs` (new), `element.rs`, `runtime.rs`, `contract.rs`; astryx-desktop `lib.rs`; astryx-web `shell.rs` | `DragSpec` drag handlers; the runtime gives a drag to the innermost handler (or finger-scrolled container) that wants its direction after 10 points, cancels the press under it, scrolls containers under a finger with a coast at UIScrollView's deceleration; `PointerEvent::time_ms`; shells forward moves while pressed | Gestures row: sheets over scrolling lists, swipe rows, reorder, momentum | Additive (no drag handlers: only finger scrolling, which no shell sent before) |
| astryx-core `contract.rs`, `element.rs`, `runtime.rs`; astryx-desktop `lib.rs`, `input.rs`; astryx-web `keys.rs`, `shell.rs` | `Surface::key` with `KeyEvent` (logical key, modifiers, repeat), offered first to `on_key` handlers from the focused node out (Pass keeps today's mapping); `on_context` for a right click at a point; `FocusRequests` move focus to an element by id after a build | Menus, comboboxes, grids, tree lists, reorder, context menus (Shift+F10, right click), roving focus | Additive (no handlers: shells map keys as before) |
| astryx-core `element.rs`, `runtime.rs` | A scroll container can rebuild the view when its offset changes and read the offset while building; start at an initial offset when it appears; report offset changes; shift its offset once when content above the viewport changes height | Virtualizer: windowed rows, scroll restore, scroll anchoring | Additive |
