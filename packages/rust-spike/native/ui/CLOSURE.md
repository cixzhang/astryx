# UI closure disposition

Every UI component family in the locked closure (Astryx core at 957d934, the 17 locked roots of the spike ledger), with where the native UI lane covers it on our Rust stack at the tip that carries this file. Infra rows (Foundations, Gestures, Layers, Motion; 308 files) are in `../infra/CLOSURE.md`.

File counts are the ledger's per-family closure counts. `files ported` counts the family's modules whose behavior the Rust stack carries; a `workaround` family lists what it lacks in its note. Evidence names the QA case-id prefix the gallery renders and the `astryx-spike-ui` test suite (`crates/astryx-spike-ui/tests/`).

## Totals

- **Locked UI families**: 67 families, 380 files: 309 ported (81%). 45 families ported, 22 workaround, 0 blocked, 0 not started.
- **Locked closure with infra**: 380 UI + 308 infra = 688 files (the ledger's 688).
- **Stretch-only families** (PowerSearch closure): 6 families, 31 files: 23 ported.

## Families (locked)

| Family | Files | Files ported | Status | Evidence | Note |
| --- | --- | --- | --- | --- | --- |
| AppShell | 5 | 4 | workaround | app-shell-*; wave8_nav app_shell_* | Slots, variants, mobile drawer below 768 pt. No skip link. |
| AspectRatio | 2 | 2 | ported | aspect-ratio-*; wave2_layout | Rectangle/ellipse, fit modes. |
| Badge | 2 | 2 | ported | badge-*; wave1_primitives | Core component, re-exported. |
| BottomSheet | 11 | 7 | workaround | bottom-sheet-*; wave4_overlays bottom_sheet_* | Modal, purposes, heights, drag to dismiss. No multi snap points, sheet switcher or edge tint. |
| Button | 3 | 3 | ported | button-*; wave5_controls | Variants, sizes, loading, icon-only, real Tooltip. |
| ButtonGroup | 3 | 3 | ported | button-group-*; wave5_controls button_group_* | Size, disabled and corners reach children through typed context. |
| Calendar | 12 | 11 | workaround | calendar-*; wave7_dates calendar_* | Single/range, 1-2 months, keyboard grid, week numbers. Gregorian only; other calendar systems render gregorian. |
| Card | 2 | 2 | ported | card-*; wave2_layout | 13 variants, elevation, padding. |
| Center | 2 | 2 | ported | center-*; wave2_layout |  |
| Chat | 10 | 4 | workaround | chat-composer-* (gallery sweep); wave9_chat | ChatComposer, ChatComposerInput, ChatSendButton, ChatComposerDrawer. Multi-line editing at the caret. No paste-as-token, @-trigger menu or dictation. |
| CheckboxInput | 3 | 3 | ported | checkbox-input-*; wave5_controls |  |
| Collapsible | 5 | 5 | ported | collapsible-*; wave6_composites collapsible_* | Uncontrolled or controlled, animated height, inert when closed, CollapsibleGroup. |
| CommandPalette | 10 | 6 | workaround | command-palette-*; wave7_search command_palette_* | Modal or inline, search, arrows/Enter, activedescendant. No result groups or custom footer. |
| ContextMenu | 2 | 2 | ported | context-menu-*; wave6_composites context_menu_*, uncontrolled_context_menu_* | Right click at the pointer, Shift+F10 and the Menu key at the target. |
| DateInput | 21 | 4 | workaround | date-input-*; wave7_dates date_input_* | Popover calendar, formats, clear. No native picker, touch wheels or typed date segments. |
| DateTimeInput | 2 | 2 | ported | date-time-input-*; wave7_dates | Date and time side by side. |
| Dialog | 5 | 4 | workaround | dialog-*, dialog-header-*; wave6_composites dialog_* | Modal or inline, standard/fullscreen, purposes, DialogHeader. No useImperativeDialog. |
| Divider | 2 | 2 | ported | divider-*; wave1_primitives | Core component. |
| DropdownMenu | 18 | 10 | workaround | dropdown-menu-*; wave6_composites *_menu_* | Items, checkbox/radio rows, groups, dividers, destructive, close on act. No submenus, drill-in, bottom-sheet presentation or menu typeahead. |
| EmptyState | 2 | 2 | ported | empty-state-*; wave2_layout |  |
| Field | 9 | 8 | workaround | field-*; wave5_controls field_* | Label, description, required/optional, statuses. No PanelSearchInput. |
| FieldStatus | 3 | 3 | ported | field-status-*; wave5_controls | Attached and detached. |
| FormLayout | 1 | 1 | ported | form-layout-* |  |
| Grid | 3 | 3 | ported | grid-*; wave2_layout | Count and auto-fill/fit columns. |
| HStack | 2 | 2 | ported | h-stack-*; wave2_layout |  |
| Heading | 2 | 2 | ported | heading-* | Core component. |
| HoverCard | 5 | 4 | workaround | hover-card-*; wave4_overlays | Hover/focus, delays, own state. No touch trigger. |
| Icon | 8 | 8 | ported | icon-*; wave1_primitives | 20 colors, sizes, SVG path glyphs. |
| IconButton | 2 | 2 | ported | icon-button-* |  |
| Indicator | 7 | 7 | ported | checkbox-indicator-*, check-indicator-*, radio-indicator-* |  |
| InfoTip | 1 | 1 | ported | info-tip-*; wave4_overlays |  |
| InputGroup | 6 | 6 | ported | input-group-*; wave5_controls |  |
| InteractiveRoleContext | 1 | 1 | ported | (via Semantics roles) | Roles carried by Semantics, no context needed. |
| Item | 9 | 7 | workaround | list-item-* ; core Item reuse | Core Item/ListItem. No swipe actions. |
| Kbd | 2 | 2 | ported | kbd-* | Core component. |
| Layout | 17 | 17 | ported | layout-*; wave2_layout | Header/footer/content/panels, dividers, padding cascade. |
| Link | 3 | 3 | ported | link-*; wave1_primitives |  |
| List | 6 | 6 | ported | list-*; wave6_composites list_* | Density, dividers, edge, markers; reorder by keyboard (announced) and pointer. |
| MobileNav | 2 | 2 | ported | mobile-nav-*; wave8_nav app_shell_moves_side_nav_* | Modal drawer from either edge; MobileNavToggle. |
| MultiSelector | 3 | 2 | workaround | multi-selector-*; wave7_search multi_selector_* | Toggle, select all, count/labels/badges. No search, create or bottom sheet. |
| NumberInput | 4 | 3 | workaround | number-input-*; wave7_dates number_input_* | Draft text, arrows, steppers, units, clamping. Commits on Enter/arrows/steppers; no wheel. |
| OverflowList | 2 | 1 | workaround | overflow-list-* | Shows up to maxVisibleItems plus a +N token; no measure-to-fit. |
| Pagination | 2 | 2 | ported | pagination-*; wave6_composites pagination_* | pages/count/compact/dots/input/none. |
| Popover | 8 | 5 | workaround | popover-*; wave4_overlays, wave6_composites uncontrolled_popover_* | Anchored, own state, modal trap, dismissal. No bottom-sheet presentation or touch rules. |
| Resizable | 6 | 5 | workaround | resize-handle-*; wave6_composites resizable_* | Drag handle, limits, collapse, snaps, ResizeHandle. No autoSaveId persistence. |
| Section | 2 | 2 | ported | section-*; wave2_layout | Variants, logical padding, dividers. |
| SegmentedControl | 6 | 4 | workaround | segmented-control-*; wave5_controls | Hug/fill, items, icons. No sliding selection lens or touch tracking. |
| SelectableCard | 3 | 3 | ported | selectable-card-*; wave5_controls |  |
| Selector | 11 | 7 | workaround | selector-*; wave7_search selector_* | Popup listbox, keyboard, variants, activedescendant. No search field or bottom-sheet presentation. |
| SideNav | 2 | 2 | ported | side-nav-*; wave8_nav side_nav_* | Sections, items, nested items, rail collapse, resizable edge. |
| SizeContext | 2 | 2 | ported | (ButtonGroup/Toolbar size tests) | Size cascade via typed context. |
| Spinner | 4 | 4 | ported | spinner-*; wave1_primitives |  |
| Stack | 5 | 5 | ported | stack-*, stack-item-*; wave2_layout | Stack, StackItem. |
| Switch | 6 | 6 | ported | switch-*; wave5_controls |  |
| Table | 55 | 50 | workaround | table-*; wave7_table (7 tests) | Typed columns; plugin trait; sorting, selection, filtering, pagination, row index/status, grouping, tree, expansion, column settings/resize widths, scroll-windowed virtualization. Sticky columns marked but not pinned; no table context menu, column-resize drag handle or gliding divider. |
| Text | 6 | 6 | ported | text-* | Core component. |
| TextInput | 2 | 2 | ported | text-input-*; wave5_controls |  |
| TimeInput | 5 | 2 | workaround | time-input-*; wave7_dates time_input_* | Time list in a selector. No typing, wheels or touch field. |
| Token | 4 | 4 | ported | token-* |  |
| Tokenizer | 6 | 3 | workaround | tokenizer-*; wave7_search tokenizer_* | Tokens, typeahead add, remove. No create, overflow behaviors or bottom sheet. |
| Toolbar | 2 | 2 | ported | toolbar-*; wave6_composites |  |
| Tooltip | 4 | 4 | ported | tooltip-*; wave4_overlays | Hover/focus with Astryx timing, own state. |
| TopNav | 2 | 2 | ported | top-nav-*; wave8_nav |  |
| TreeList | 5 | 5 | ported | tree-list-*; wave8_nav tree_list_* | Levels, guides, density, one tab stop, Left/Right. |
| Typeahead | 8 | 7 | workaround | typeahead-*; wave7_search typeahead_* | Suggest, arrows/Enter/Escape, activedescendant, pointer-only options. No busy indicator lane. |
| VStack | 2 | 2 | ported | v-stack-*; wave2_layout |  |
| VisuallyHidden | 2 | 2 | ported | visually-hidden-* |  |

## Families (stretch only)

| Family | Files | Files ported | Status | Evidence | Note |
| --- | --- | --- | --- | --- | --- |
| Avatar | 5 | 5 | ported | avatar-*; wave1_primitives | Core component (stretch closure). |
| AvatarGroup | 1 | 1 | ported | avatar-group-*; wave1_primitives | Stretch closure. |
| CheckboxList | 4 | 4 | ported | checkbox-list-*; wave5_controls | Stretch closure. |
| DateRangeInput | 2 | 2 | ported | date-range-input-*; wave7_dates | Stretch closure. |
| PowerSearch | 16 | 8 | workaround | power-search-*; wave9_power_search (3 tests) | Stretch. Field/operator/value flow, enum/string/number/date values, token removal. No edit popover, bottom sheet, date zones, list/entity/custom operators. |
| RadioList | 3 | 3 | ported | radio-list-*; wave5_controls | Stretch closure. |

## Also built

- Templates `shell-nav` and `settings-dialog`: crate `astryx-spike-templates` (tests `templates.rs`, example `shots`).
- Gallery `astryx-spike-gallery` renders every family in `qa/cases` except `Chat` (core exports no `Chat` component; the gallery sweeps `ChatComposer`).
