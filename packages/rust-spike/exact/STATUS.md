# Exact lane status

Astryx source: `957d9348bf8808373dcba68b9d32abb69ef12e15`. Exact: `94b8671f70fe2e25b307a40e7fce66dc27c9f802`.

- Dependency closure ported (core, by source files): 341 of 689 (49.5%).
- With the stretch rows: 347 of 720 (48.2%).
- Rows: 17 ported, 15 workaround, 0 blocked, 45 not started.

A row counts as ported when its Contract port gives the Astryx behavior and look its cases test,
workaround when it needs a different shape or an escape hatch (the note says which), and blocked
when Contract cannot express it (the note gives the smallest failing snippet).

| Row | Files | Groups | Wave | Status | Evidence | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| AppShell | 5 | core, stretch, template |  | not started |  |  |
| AspectRatio | 2 | core | 2 | ported | gallery aspectratio-* | fit (cover/contain) applies to an image child, set by the caller. |
| Avatar | 5 | stretch | 1 | ported | gallery avatar-* | Initials use the first letter of the first and last words (no Unicode-aware search). Image fallbackSrc and load error not yet wired. Tooltip waits for wave 4. |
| AvatarGroup | 1 | stretch | 1 | ported | gallery avatargroup-* | Members are data (AvatarItem[]), not child elements: one slot cannot be counted. |
| Badge | 2 | core, weakness | 1 | ported | gallery badge-* (15 variants) | Forced-colours outline not expressible. |
| BottomSheet | 11 | core, stretch, template |  | not started |  |  |
| Button | 3 | core, stretch, template, weakness |  | not started |  |  |
| ButtonGroup | 3 | core, stretch, template, weakness |  | not started |  |  |
| Calendar | 12 | core, stretch |  | not started |  |  |
| Card | 2 | core, template | 2 | workaround | gallery card-* | A library cannot size a node on its slot's chain from a prop, so free sizes (width, height, maxWidth, minHeight) are the caller's wrapping view. Border inset compensation (padding minus border) dropped. |
| Center | 2 | core | 2 | workaround | gallery center-* | A library cannot size a node on its slot's chain from a prop, so free sizes (width, height, maxWidth, minHeight) are the caller's wrapping view. isInline dropped (no inline-flex). |
| Chat | 10 | weakness |  | not started |  |  |
| CheckboxInput | 3 | core, stretch, weakness |  | not started |  |  |
| CheckboxList | 4 | stretch |  | not started |  |  |
| Collapsible | 5 | weakness | 3 | workaround | gallery collapsible-* | Trigger is a `title` (one slot, used by the content). Open state is the caller's (`isOpen`, `toggle`): a slot passes no context, so CollapsibleGroup cannot coordinate its children; single/multiple is the caller's state. Not a library component: it hides its slot. |
| CommandPalette | 10 | template |  | not started |  |  |
| ContextMenu | 2 | core |  | not started |  |  |
| DateInput | 21 | core, stretch |  | not started |  |  |
| DateRangeInput | 2 | stretch |  | not started |  |  |
| DateTimeInput | 2 | core, stretch |  | not started |  |  |
| Dialog | 5 | core, stretch, template |  | not started |  |  |
| Divider | 2 | core, stretch, template | 1 | ported | gallery divider-* | isFullBleed needs the container's padding, which Contract cannot read (no runtime CSS variables): ignored. |
| DropdownMenu | 18 | core, stretch, template |  | not started |  |  |
| EmptyState | 2 | core, stretch | 2 | ported | gallery emptystate-* | actions are the slot. |
| Field | 9 | core, stretch, template, weakness |  | not started |  |  |
| FieldStatus | 3 | core, stretch, template, weakness |  | not started |  |  |
| FormLayout | 1 | core, stretch, template, weakness | 2 | ported | used by settings-dialog (wave 8) |  |
| Grid | 3 | core | 2 | workaround | gallery grid-* | A library cannot size a node on its slot's chain from a prop, so free sizes (width, height, maxWidth, minHeight) are the caller's wrapping view. rowHeight not yet wired. |
| HStack | 2 | core, stretch, template | 2 | ported | gallery hstack-* |  |
| Heading | 2 | core, stretch, template | 1 | ported | gallery heading-0001..0007 |  |
| HoverCard | 5 | weakness |  | not started |  |  |
| Icon | 8 | core, stretch, template, weakness | 1 | ported | astryx/icons.contract (29 icons from defaultIcons.tsx as one path each); gallery icon-* | Contract expands a component at each use, so a branch per icon repeated all 29 icons at every Icon: one path chosen by a function keeps Icon small. Custom icon registry (registerIcons) has no Contract equivalent: icons are known at build. |
| IconButton | 2 | core, stretch |  | not started |  |  |
| Indicator | 7 | core, stretch, template, weakness | 1 | ported | gallery indicator-* | Checkbox, radio and check indicators. |
| InfoTip | 1 | core, stretch, template, weakness |  | not started |  |  |
| InputGroup | 6 | core, stretch, template, weakness |  | not started |  |  |
| InteractiveRoleContext | 1 | core, stretch | 0 | workaround | - | No context through a slot: role is a prop. |
| Item | 9 | core, stretch, template, weakness |  | not started |  |  |
| Kbd | 2 | core, stretch, template | 1 | ported | gallery kbd-* | Mod shows as the Command sign on every platform for now. |
| Layout | 17 | core, stretch, template, weakness | 2 | workaround | gallery layout-0001, layout-0002 | Astryx takes header/start/end/footer as named props; Contract has one slot, so LayoutHeader, LayoutPanel, LayoutContent and LayoutFooter are children that place themselves on the frame's grid. Edge compensation through --container-padding-* custom properties has no equivalent. Panel width is a zero-height strut; content width is padding. No container queries. |
| Link | 3 | core, stretch, template, weakness | 1 | ported | gallery link-* | Hover underline thickness refused (a state row paints only); hover dims instead. LinkProvider (router swap) is Contract's own `link`/routes. |
| List | 6 | core, stretch, weakness |  | not started |  |  |
| MobileNav | 2 | core |  | not started |  |  |
| MultiSelector | 3 | core |  | not started |  |  |
| NumberInput | 4 | core, stretch |  | not started |  |  |
| OverflowList | 2 | core, stretch |  | not started |  |  |
| Pagination | 2 | core |  | not started |  |  |
| Popover | 8 | core, stretch, template, weakness | 3 | workaround | gallery popover-*; web and Linux | Trigger is the caller's button (`popovertarget`). Placement maps to position-area; isModal, hasAutoFocus and presentation (bottom-sheet/adaptive) not yet wired (adaptive needs the viewport, which only the app root reads). Headless Linux `showPopover` is reported unsupported by Exact's docs, yet the anchored popover rendered in our headless run. |
| PowerSearch | 16 | stretch |  | not started |  |  |
| RadioList | 3 | stretch |  | not started |  |  |
| Resizable | 6 | core, template |  | not started |  |  |
| Section | 2 | core, stretch, template | 2 | workaround | gallery section-* | Nested full-bleed (cancel the container's padding) needs the padding by prop (`bleed`): no runtime custom properties, no context through a slot. |
| SegmentedControl | 6 | template |  | not started |  |  |
| SelectableCard | 3 | template |  | not started |  |  |
| Selector | 11 | core, stretch, template |  | not started |  |  |
| SideNav | 2 | core |  | not started |  |  |
| SizeContext | 2 | core, stretch, template, weakness | 0 | workaround | - | No context through a slot: size is a prop each child takes. |
| Spinner | 4 | core, stretch, template, weakness | 1 | ported | gallery spinner-* | Turn is a CSS keyframe; reduced motion not yet wired. |
| Stack | 5 | core, stretch, template | 2 | workaround | gallery stack-* | A library cannot size a node on its slot's chain from a prop, so free sizes (width, height, maxWidth, minHeight) are the caller's wrapping view. Padding is physical (left/right): Contract refuses logical CSS; the page mirrors under RTL. `as` (element type) dropped. |
| Switch | 6 | core |  | not started |  |  |
| Table | 55 | core |  | not started |  |  |
| Text | 6 | core, stretch, template, weakness | 1 | ported | gallery text-0001..0024; web + Linux | `font` takes only literals, so the type role is a chain of literals. No inline-flex. `textWrap` keeps auto/balance plus nowrap; `pretty` is refused. Truncation tooltip waits for Tooltip (wave 4). |
| TextInput | 2 | core, stretch |  | not started |  |  |
| TimeInput | 5 | core, stretch |  | not started |  |  |
| Token | 4 | core, stretch | 2 | ported | gallery token-* | href/onClick variants and endContent not yet wired. |
| Tokenizer | 6 | core, stretch |  | not started |  |  |
| Toolbar | 2 | core | 2 | workaround | - | Arrow-key roving focus waits for wave 5 controls; overflow menu waits for OverflowList (wave 6). |
| Tooltip | 4 | core, stretch, template, weakness |  | not started |  |  |
| TopNav | 2 | core |  | not started |  |  |
| TreeList | 5 | stretch, template |  | not started |  |  |
| Typeahead | 8 | core, stretch |  | not started |  |  |
| VStack | 2 | core, stretch, template | 2 | ported | gallery vstack-* |  |
| VisuallyHidden | 2 | core, stretch, template, weakness | 0 | workaround | primitives.contract VisuallyHidden | Contract refuses clip-path inset(); a 1 px clipped box stands in. |
| infra:Foundations | 158 | core, stretch, template, weakness | 0 | workaround | astryx/astryx.tamber (251 tokens) and tokens/themes.tamber (8 themes, 1,066 per-theme values), imported by Exact's own `contract tamber import astryx`; gallery case foundations-0001 on web and Linux. | 279 of 286 resolved tokens map; 7 do not (duration min/max tiers, focus outline style). The 4 motion tokens cannot vary by theme (Tamber writes a duration or easing into its transition at build), so every theme uses the default theme's motion. A type role (`font`) varies by theme, so it must sit on each text, not on a container. Number and date formatting stay en-US in Contract (a data module would localise, but the Linux build carries none). |
| infra:Gestures | 68 | core, stretch, template, weakness |  | not started |  |  |
| infra:Layers | 41 | core, stretch, template, weakness | 3 | workaround | astryx-layers/layers.contract; gallery popover-* (web and Linux) | The platform's top layer does the layer stack, focus, Escape and outside-press dismissal (`popover`, `dialog`). A popover anchors only to the control that opened it (position-area); no anchoring to another element, no collision flipping beyond the platform's, no portal. Exact holds a library to always showing its slot, so every component that hides its content until opened (Popover, Collapsible, Tooltip, Dialog, menus, sheets) lives in a second, plain package (@astryx-spike/astryx-layers) beside the library. |
| infra:Motion | 42 | core, stretch, template, weakness | 3 | workaround | layers.contract keyframes astryxEnter/Exit/FadeIn/Rise/SheetUp | Keyframes take literals, so presets carry the default theme's durations and easings. A library cannot read reduced motion (the viewport is the app's root resource): moving components take `still`. Exit animations do not run on Linux. Animating to an auto height is not possible (numeric heights only): Collapsible content rises in instead. |
