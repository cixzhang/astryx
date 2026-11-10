# Exact lane status

Astryx source: `957d9348bf8808373dcba68b9d32abb69ef12e15`. Exact: `94b8671f70fe2e25b307a40e7fce66dc27c9f802`.

- Dependency closure ported (core, by source files): 656 of 689 (95.2%).
- With the stretch rows: 671 of 720 (93.2%).
- Rows: 26 ported, 43 workaround, 0 blocked, 8 not started.

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
| BottomSheet | 11 | core, stretch, template | 6 | workaround | gallery bottomsheet-*; playground test opens it | A modal dialog at the bottom edge with a grabber; dragging down past 120 px closes it (pointer events). Snap points and detents not wired. |
| Button | 3 | core, stretch, template, weakness | 5 | ported | gallery button-*; playground test presses Save (web and Linux) | A native button drawn in Astryx's look. isInterruptible not wired. Press scale is a paint-only :active opacity (a state row cannot scale). |
| ButtonGroup | 3 | core, stretch, template, weakness | 5 | workaround | gallery button-group-* | A slot passes no context and Contract has no :first-child, so each Button takes `groupEdge` from the caller to square its inner corners. Group-wide size and isDisabled are each Button's props. |
| Calendar | 12 | core, stretch | 7 | workaround | gallery calendar-*; playground test moves a month and picks a day | Month grid from day arithmetic (Zeller's congruence) in Contract functions: no date library. Names are English (Contract formats dates en-US); other calendar systems and locales are not possible without a data module. Arrow-key grid navigation not wired. |
| Card | 2 | core, template | 2 | workaround | gallery card-* | A library cannot size a node on its slot's chain from a prop, so free sizes (width, height, maxWidth, minHeight) are the caller's wrapping view. Border inset compensation (padding minus border) dropped. |
| Center | 2 | core | 2 | workaround | gallery center-* | A library cannot size a node on its slot's chain from a prop, so free sizes (width, height, maxWidth, minHeight) are the caller's wrapping view. isInline dropped (no inline-flex). |
| Chat | 10 | weakness |  | not started |  |  |
| CheckboxInput | 3 | core, stretch, weakness | 5 | workaround | gallery checkboxinput-*; playground test checks it | aria-checked takes only a bool: an indeterminate box reads as unchecked. |
| CheckboxList | 4 | stretch | 5 | workaround | gallery checkboxlist-* | Options are data, as RadioList. |
| Collapsible | 5 | weakness | 3 | workaround | gallery collapsible-* | Trigger is a `title` (one slot, used by the content). Open state is the caller's (`isOpen`, `toggle`): a slot passes no context, so CollapsibleGroup cannot coordinate its children; single/multiple is the caller's state. Not a library component: it hides its slot. |
| CommandPalette | 10 | template |  | not started |  |  |
| ContextMenu | 2 | core | 6 | workaround | gallery contextmenu-*; playground test picks an item | contextPopover on the slot: right click and long press. |
| DateInput | 21 | core, stretch | 7 | workaround | gallery dateinput-*; playground test | ISO text field plus a calendar popover. Segmented date entry and locale formats not possible (en-US only). |
| DateRangeInput | 2 | stretch | 7 | workaround | gallery daterangeinput-* | DateInput with isRange; the calendar shades the range. |
| DateTimeInput | 2 | core, stretch | 7 | workaround | composed: DateInput beside TimeInput | No separate component: the two fields side by side. |
| Dialog | 5 | core, stretch, template | 6 | workaround | gallery dialog-*; playground test opens and closes | The platform's modal `dialog` (focus trap, inert page, Escape via closedby). Opened by showModal or commandfor, not by an isOpen prop: a dialog that state holds open is not available in Contract yet. isInline not possible. Lives in astryx-layers (hides its slot). |
| Divider | 2 | core, stretch, template | 1 | ported | gallery divider-* | isFullBleed needs the container's padding, which Contract cannot read (no runtime CSS variables): ignored. |
| DropdownMenu | 18 | core, stretch, template | 6 | workaround | gallery dropdownmenu-*; playground test picks an item | The platform menu (popover role=menu). Items are data (MenuItem[]); submenus, radio groups and type-ahead not wired. |
| EmptyState | 2 | core, stretch | 2 | ported | gallery emptystate-* | actions are the slot. |
| Field | 9 | core, stretch, template, weakness | 5 | ported | gallery field-* | The control is the slot and names itself by the label's id. |
| FieldStatus | 3 | core, stretch, template, weakness | 5 | ported | gallery fieldstatus-* |  |
| FormLayout | 1 | core, stretch, template, weakness | 2 | ported | used by settings-dialog (wave 8) |  |
| Grid | 3 | core | 2 | workaround | gallery grid-* | A library cannot size a node on its slot's chain from a prop, so free sizes (width, height, maxWidth, minHeight) are the caller's wrapping view. rowHeight not yet wired. |
| HStack | 2 | core, stretch, template | 2 | workaround | gallery h-stack-* | Contract forwards no slot into another component, so HStack repeats Stack's body instead of wrapping Stack. |
| Heading | 2 | core, stretch, template | 1 | ported | gallery heading-0001..0007 |  |
| HoverCard | 5 | weakness | 4 | workaround | gallery hovercard-* | Card content is a title and body text: the one slot is the trigger. Same delay limit as Tooltip. |
| Icon | 8 | core, stretch, template, weakness | 1 | ported | astryx/icons.contract (29 icons from defaultIcons.tsx as one path each); gallery icon-* | Contract expands a component at each use, so a branch per icon repeated all 29 icons at every Icon: one path chosen by a function keeps Icon small. Custom icon registry (registerIcons) has no Contract equivalent: icons are known at build. |
| IconButton | 2 | core, stretch | 5 | ported | gallery iconbutton-* |  |
| Indicator | 7 | core, stretch, template, weakness | 1 | ported | gallery checkbox-indicator-*, check-indicator-*, radio-indicator-* | Checkbox, radio and check indicators. |
| InfoTip | 1 | core, stretch, template, weakness | 4 | ported | gallery infotip-* | Opens on hover and on press (popovertarget). |
| InputGroup | 6 | core, stretch, template, weakness | 5 | ported | gallery input-group-* | Parts are the slot (InputGroupText, an input). |
| InteractiveRoleContext | 1 | core, stretch | 0 | workaround | - | No context through a slot: role is a prop. |
| Item | 9 | core, stretch, template, weakness | 6 | workaround | gallery item-* | Start and end content are an icon name and text; the slot is extra end content. Swipe actions not wired (Contract has row `delete=` and a scroll-snap long form). |
| Kbd | 2 | core, stretch, template | 1 | ported | gallery kbd-* | Mod shows as the Command sign on every platform for now. |
| Layout | 17 | core, stretch, template, weakness | 2 | workaround | gallery layout-0001, layout-0002 | Astryx takes header/start/end/footer as named props; Contract has one slot, so LayoutHeader, LayoutPanel, LayoutContent and LayoutFooter are children that place themselves on the frame's grid. Edge compensation through --container-padding-* custom properties has no equivalent. Panel width is a zero-height strut; content width is padding. No container queries. |
| Link | 3 | core, stretch, template, weakness | 1 | ported | gallery link-* | Hover underline thickness refused (a state row paints only); hover dims instead. LinkProvider (router swap) is Contract's own `link`/routes. |
| List | 6 | core, stretch, weakness | 6 | workaround | gallery list-* | Rows are the slot (ListItem); a slot passes no context, so each row takes the list's density, divider and marker. Reorderable lists are ReorderList (data rows). Edge compensation by custom properties dropped. |
| MobileNav | 2 | core |  | not started |  |  |
| MultiSelector | 3 | core | 7 | workaround | gallery multiselector-*; playground test chooses several | Selector with isMultiple. aria-multiselectable is refused. |
| NumberInput | 4 | core, stretch | 5 | ported | gallery numberinput-*; playground test steps it | Value is text; parse and clamp are the caller's (parseNumber). |
| OverflowList | 2 | core, stretch |  | not started |  |  |
| Pagination | 2 | core | 7 | ported | gallery pagination-*; playground test pages | Ellipsis for long ranges not wired. |
| Popover | 8 | core, stretch, template, weakness | 3 | workaround | gallery popover-*; web and Linux | Trigger is the caller's button (`popovertarget`). Placement maps to position-area; isModal, hasAutoFocus and presentation (bottom-sheet/adaptive) not yet wired (adaptive needs the viewport, which only the app root reads). Headless Linux `showPopover` is reported unsupported by Exact's docs, yet the anchored popover rendered in our headless run. |
| PowerSearch | 16 | stretch |  | not started |  |  |
| RadioList | 3 | stretch | 5 | workaround | gallery radiolist-*; playground test picks an option | Options are data (ChoiceItem[]), not RadioListItem children: one slot cannot tell a list which child is chosen. |
| Resizable | 6 | core, template | 6 | workaround | gallery resizable-*; playground test drags the handle 60 px | Width is the caller's state; drag by pointer events. Arrow-key resizing not wired. In astryx-layers: the library cannot size a slot-holding node from a prop. |
| Section | 2 | core, stretch, template | 2 | workaround | gallery section-* | Nested full-bleed (cancel the container's padding) needs the padding by prop (`bleed`): no runtime custom properties, no context through a slot. |
| SegmentedControl | 6 | template | 5 | workaround | gallery segmented-control-*; playground test picks a segment | A library control must be the native one, so each segment is a transparent radio input over its drawn segment. Items are data. |
| SelectableCard | 3 | template | 5 | ported | gallery selectable-card-* | A native button with aria-pressed. |
| Selector | 11 | core, stretch, template | 7 | workaround | gallery selector-*; playground test chooses | A trigger and a listbox popover. Contract refuses aria-activedescendant and aria-autocomplete, so the highlighted option is drawn, not announced; options are buttons in the tab order. Type-ahead in the list not wired. |
| SideNav | 2 | core |  | not started |  |  |
| SizeContext | 2 | core, stretch, template, weakness | 0 | workaround | - | No context through a slot: size is a prop each child takes. |
| Spinner | 4 | core, stretch, template, weakness | 1 | ported | gallery spinner-* | Turn is a CSS keyframe; reduced motion not yet wired. |
| Stack | 5 | core, stretch, template | 2 | workaround | gallery stack-*, stack-item-* | A library cannot size a node on its slot's chain from a prop, so free sizes (width, height, maxWidth, minHeight) are the caller's wrapping view. Padding is physical (left/right): Contract refuses logical CSS; the page mirrors under RTL. `as` (element type) dropped. StackItem ported. |
| Switch | 6 | core | 5 | ported | gallery switch-*; playground test toggles it (web and Linux) | A native checkbox (role switch) lies transparent over the drawn track and thumb. Drag-to-toggle (touch) not wired. |
| Table | 55 | core | 7 | workaround | gallery table-*; playground test sorts, selects all and one, pages | Columns and rows are data; a row is its cells in column order (Contract has no records keyed by a run-time string). Sorting, selection, pagination, tree rows are the caller's state; the table draws them. Not honoured: verticalAlign (cells take no vertical-align top/bottom) and hasHover=false (a state class chosen by a prop is refused). Not wired: column resize, sticky columns, virtualization (aria-rowcount/rowindex refused), filtering UI, column settings, grouped rows. |
| Text | 6 | core, stretch, template, weakness | 1 | ported | gallery text-0001..0024; web + Linux | `font` takes only literals, so the type role is a chain of literals. No inline-flex. `textWrap` keeps auto/balance plus nowrap; `pretty` is refused. Truncation tooltip waits for Tooltip (wave 4). |
| TextInput | 2 | core, stretch | 5 | ported | gallery text-input-*; playground test types and clears (web; Linux by the agent's type) | No :focus-within: the field's focus and blur set the ring through component state. hasAutoCorrect and hasSpellCheck have no Contract attribute. Real-keyboard editing on Linux not verified (the agent sets the value). |
| TimeInput | 5 | core, stretch | 7 | workaround | gallery timeinput-* | 24-hour HH:MM text field; no locale 12-hour format, no segment stepping. |
| Token | 4 | core, stretch | 2 | ported | gallery token-* | href/onClick variants and endContent not yet wired. |
| Tokenizer | 6 | core, stretch | 7 | workaround | gallery tokenizer-*; playground test adds and removes | Tokens and a typeahead in one field. Backspace-to-remove not wired. Contract refuses aria-activedescendant and aria-autocomplete, so the highlighted option is drawn, not announced; options are buttons in the tab order. |
| Toolbar | 2 | core | 2 | workaround | - | Arrow-key roving focus waits for wave 5 controls; overflow menu waits for OverflowList (wave 6). |
| Tooltip | 4 | core, stretch, template, weakness | 4 | workaround | gallery tooltip-*; test 'a tooltip case opens its tooltip' | Shows on hover and focus via the platform popover, anchored to the trigger. Timers run only in the app root's tasks, so `delay` is the fade's delay (the tooltip is open, transparent, for `delay` ms) and hideDelay is not possible. Content is text (one slot: the trigger). touchTrigger tap not wired. Hover is web and desktop only. |
| TopNav | 2 | core |  | not started |  |  |
| TreeList | 5 | stretch, template | 6 | workaround | gallery tree-list-*; playground test opens, closes and selects | Items are flattened rows with a depth (no recursion in a view). Open state is the caller's. Keyboard tree navigation (arrow keys) not wired. |
| Typeahead | 8 | core, stretch | 7 | workaround | gallery typeahead-*; playground test filters and picks | The Combobox root. A field plus a listbox popover; filtering in the view; Enter picks the highlighted option. Contract refuses aria-activedescendant and aria-autocomplete, so the highlighted option is drawn, not announced; options are buttons in the tab order. Arrow-key highlight not wired (an input takes no key event). Async sources are the caller's (no data module in a Linux build). |
| VStack | 2 | core, stretch, template | 2 | workaround | gallery v-stack-* | As HStack: no slot forwarding, so it repeats Stack's body. |
| VisuallyHidden | 2 | core, stretch, template, weakness | 0 | workaround | primitives.contract VisuallyHidden | Contract refuses clip-path inset(); a 1 px clipped box stands in. |
| infra:Foundations | 158 | core, stretch, template, weakness | 0 | workaround | astryx/astryx.tamber (251 tokens) and tokens/themes.tamber (8 themes, 1,066 per-theme values), imported by Exact's own `contract tamber import astryx`; gallery case foundations-0001 on web and Linux. | 279 of 286 resolved tokens map; 7 do not (duration min/max tiers, focus outline style). The 4 motion tokens cannot vary by theme (Tamber writes a duration or easing into its transition at build), so every theme uses the default theme's motion. A type role (`font`) varies by theme, so it must sit on each text, not on a container. Number and date formatting stay en-US in Contract (a data module would localise, but the Linux build carries none). |
| infra:Gestures | 68 | core, stretch, template, weakness | 6 | workaround | BottomSheet grabber drag, Resizable handle drag, ReorderList drag (tests pass on web) | Pointer events (pointerdown/move/up) with component state cover drags. No gesture arena: competing gestures (a sheet drag inside a scroll) are not arbitrated. Reorder is the host's (`reorderdrop` on a vertical virtualized list). Keyboard reorder (Space, arrows, Enter on the grip) did not move a row in our web run: the grip is exposed as a generic, unfocused element (gaps.test.contract keeps the failing test). |
| infra:Layers | 41 | core, stretch, template, weakness | 3 | workaround | astryx-layers/layers.contract; gallery popover-* (web and Linux) | The platform's top layer does the layer stack, focus, Escape and outside-press dismissal (`popover`, `dialog`). A popover anchors only to the control that opened it (position-area); no anchoring to another element, no collision flipping beyond the platform's, no portal. Exact holds a library to always showing its slot, so every component that hides its content until opened (Popover, Collapsible, Tooltip, Dialog, menus, sheets) lives in a second, plain package (@astryx-spike/astryx-layers) beside the library. |
| infra:Motion | 42 | core, stretch, template, weakness | 3 | workaround | layers.contract keyframes astryxEnter/Exit/FadeIn/Rise/SheetUp | Keyframes take literals, so presets carry the default theme's durations and easings. A library cannot read reduced motion (the viewport is the app's root resource): moving components take `still`. Exit animations do not run on Linux. Animating to an auto height is not possible (numeric heights only): Collapsible content rises in instead. |
