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
  becoming optional), `Edit::effect` (and `Edit::on_change` becoming
  optional), and the runtime's state store, pruned with the node tree.
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
| astryx-core `a11y.rs`, `element.rs` | Relations resolved from id props; a labeled node covers its children only for roles named from content, unless `Semantics::one_stop` asks for the old behavior | Labeled containers (a group of avatars) hid their children | Behavior change for labeled container roles only. No `rust` code labels a container whose children are not focus stops (cards, HoldCards and rows use Radio, Checkbox, Button or ListItem, which still cover), so screen-reader output is unchanged: `examples/gallery/tests/a11y_golden.rs` checks the accessibility tree of all 20 gallery sections against goldens written from the tree before this change (`tests/a11y/*.txt`, including cards with children, HoldCard, SelectableCardGroup and Settings-style rows). `one_stop` restores the old reading for any container that needs it. |
| astryx-core `a11y.rs` | The page behind a modal layer is left out of the tree | Inert page behind dialogs | Additive (only with modal layers) |
| astryx-desktop `a11y.rs`, astryx-web `mirror.rs` | Bridge new roles and states; mirror elements get DOM ids `ax-<id>` | Platform accessibility | Additive |
| astryx-core `layer.rs` (new), `element.rs` | `LayerSpec`, `LayerAnchor`, `DismissReason`, `PlaceInput`, `HoverSpec`, `When`; `Node::{layer, hover, variants}`; `BuildCx::{layer, scope, in_scope, now_ms, last_rects, last_rect}`; `Node::variant` | Layers, hover intent, paint states | Additive |
| astryx-core `layout.rs` | `Layout::layers` offsets applied in `placed` | Placing layers after layout | Additive (empty unless layers exist) |
| astryx-core `runtime.rs` | Overlays built until none are queued (nested layers); layer placement, Escape to the topmost layer, outside-press dismissal, focus restore, hover tracking | Layers | Additive (inactive without layers or hover specs) |
| astryx-core `runtime.rs` | Focus stops come from the topmost trap only | A popover over a dialog takes focus | Behavior change only with two or more traps (none existed before) |
| astryx-core `contract.rs`, `runtime.rs`, astryx-desktop `lib.rs`, astryx-web `shell.rs` | `PointerEvent`, `PointerOutcome`, `Surface::pointer` (default no-op); shells send presses (down, up) before their existing `PressStart` mapping | A press outside any focus stop (light dismiss); later, drags for gestures. Hover stays on the existing `pointer_move`; hover intent (`HoverSpec`) reads that same pointer position | Additive |
| astryx-core `runtime.rs`, `paint.rs` | Focus visibility: every input a shell sends (keys, D-pad, face buttons, gamepad) and every key event makes focus visible; only a pointer press (`A11yAction::PressStart`) hides it until the next input; a screen reader moving focus keeps it visible. The default is visible, so a shell that never sends pointer events (the Brick) always shows the ring. `Runtime::hide_focus_until_keyboard` starts hidden, for pointer-first captures | `:focus-visible` parity on web and desktop | Behavior change only after a pointer press; `tests/focus_visible.rs` drives a gamepad-only input stream (Up, Down, A, B, Next, L1) and checks the ring on every frame, and checks a pointer press hides it until the next button |
| astryx-core `paint.rs` | Draw variants by paint state; modal backdrop | Hover tints and focus styles on any node | Additive |
| astryx-core `motion.rs` (new), `element.rs` | `Anim`, `Channel`, `Exit`, `Resize`; `Node::{anims, exit, resize, inert}`, `Node::animate` | Component motion on astryx-motion (Switch thumb, layer enter and exit, Collapsible) | Additive |
| astryx-core `runtime.rs`, `paint.rs`, `layout.rs`, `a11y.rs` | One `Animated` per node and channel kept across rebuilds; exit copies from the last paint, drawn above everything, one per node; height animation with a second layout pass while it moves; `inert` subtrees out of the a11y tree and focus order; `paint` takes a capture list | Motion row: retarget without jumps, exit plays once, layout animation, Reduce Motion | Additive (inactive without anims, exit, resize or inert) |
| astryx-core `element.rs`, `layer.rs`, `runtime.rs` | Component state kept by the runtime per node id and type (`LocalHandle`), changed from `Interact::effect`, `LayerSpec::dismiss_effect` and `HoverSpec::effect`; a change rebuilds; state is dropped with its node | Astryx's uncontrolled APIs (Tooltip, Popover, Collapsible `defaultIsOpen`, Table default sort) without an app message per bit of UI state | Additive; `HoverSpec::on_change` is now optional (introduced on this branch) |
| astryx-core `runtime.rs` | The build sees the frame's viewport (it saw (0, 0) on the first frame), and a viewport change rebuilds | Layers and sheets size from `BuildCx::viewport` | Bug fix (one more rebuild per resize) |
| astryx-core `gesture.rs` (new), `element.rs`, `runtime.rs`, `contract.rs`; astryx-desktop `lib.rs`; astryx-web `shell.rs` | `DragSpec` drag handlers; the runtime gives a drag to the innermost handler (or finger-scrolled container) that wants its direction after 10 points, cancels the press under it, scrolls containers under a finger with a coast at UIScrollView's deceleration; `PointerEvent::time_ms`; shells forward moves while pressed | Gestures row: sheets over scrolling lists, swipe rows, reorder, momentum | Additive (no drag handlers: only finger scrolling, which no shell sent before) |
| astryx-core `contract.rs`, `element.rs`, `runtime.rs`; astryx-desktop `lib.rs`, `input.rs`; astryx-web `keys.rs`, `shell.rs` | `Surface::key` with `KeyEvent` (logical key, modifiers, repeat), offered first to `on_key` handlers from the focused node out (Pass keeps today's mapping); `on_context` for a right click at a point; `FocusRequests` move focus to an element by id after a build | Menus, comboboxes, grids, tree lists, reorder, context menus (Shift+F10, right click), roving focus | Additive (no handlers: shells map keys as before) |
| astryx-core `element.rs`, `runtime.rs` | A scroll container can rebuild the view when its offset changes and read the offset while building; start at an initial offset when it appears; report offset changes; shift its offset once when content above the viewport changes height | Virtualizer: windowed rows, scroll restore, scroll anchoring | Additive |
| astryx-core `element.rs`, `runtime.rs`, `components/input.rs`, `components/keyboard.rs` | `Edit::on_change` is optional and `Edit::effect` runs first with a new value | Inputs that keep their own draft text (combobox query, number draft) | Struct change: `Edit` literals wrap `on_change` in `Some` and add `effect: None` (three sites updated) |
| astryx-core `gesture.rs`, `runtime.rs` | `DragSpec::on_drag` returns an optional message | Handlers that send only at the release (reorder, drag to dismiss) | Introduced on this branch |
| astryx-core `element.rs`, `runtime.rs`, `a11y.rs` | `Interact::focusable` (default true) and `Interact::pointer_only`: a pointer-only control is not a focus stop; a pointer press and release over it (through `Surface::pointer`) or assistive technology's activate runs it without moving focus | Combobox options, which the field points at through `aria-activedescendant` | Additive |

## `Edit` migration (group f)

`Edit::on_change` became `Option<Rc<dyn Fn(String) -> Msg>>` and
`Edit::effect: Option<Rc<dyn Fn(String)>>` was added. Every consumer:

| Consumer | Where | Migration |
| --- | --- | --- |
| TextInput | astryx-core `components/input.rs` | Builds `Edit { on_change: Some(on_change.clone()), effect: None, .. }`; behavior unchanged |
| OnScreenKeyboard compose | astryx-core `components/keyboard.rs` | Builds `Edit { on_change: Some(..KeyboardUpdate..), effect: None, .. }`; behavior unchanged |
| Runtime typing, Backspace and Delete | astryx-core `runtime.rs` `text_input` | Runs `effect` then `on_change` when present; sends no message when `on_change` is `None` |
| Runtime value and caret reads | astryx-core `runtime.rs` `field_value`, `caret`, `caret_paint`, `focused_edit` | Read `value`, `text_node`, `masked`, `caret` only; unchanged |
| Desktop IME commit | astryx-desktop `lib.rs` (`Ime::Commit`) | Calls `Surface::text_input`; no change |
| Web mirror input | astryx-web `shell.rs` (`beforeinput`, composition end) | Calls `Surface::text_input`; no change |
| Spike TextInput | spike `astryx-spike-ui` `controls/text_input.rs` | Same as core TextInput |

`Edit::multiline` (default `false` in every literal above) makes a field
multi-line: Shift+Enter, or Enter when the field sends nothing on
activate, types a line break at the caret; Up and Down move the caret
between wrapped lines and move focus on the first and last line; the
caret sits on its wrapped line. `A11yState::multiline` exposes it
(AccessKit `MultilineTextInput`, DOM `aria-multiline`). Tests:
`tests/multiline.rs` and `wrap_tests` in `runtime.rs`. Known gap: the
DOM mirror keeps an `<input>` for the field, so the browser's own editing
of a line break in the hidden field is not used; the runtime edits it.
| astryx-core `runtime.rs`, `paint.rs`, `contract.rs`, `a11y.rs`; astryx-web `mirror.rs` | Text selection in a focused field: Ctrl or Cmd+A selects all; Shift with Left, Right, Home and End extends; a plain arrow collapses a selection; typing, Backspace and Delete replace it; the selection paints behind the text and reaches assistive technology as `text_selection` (anchor, caret). `A11yState::editable` marks any node with an `Edit`, and the DOM mirror gives an editable combobox, search box or spin button a text `<input>` with its role, so browser text input reaches the app | Typeahead, Selector search, CommandPalette, PowerSearch and NumberInput on the web; select-all parity | Additive for fields (select all was not handled before); mirror change only for editable non-TextInput roles |
