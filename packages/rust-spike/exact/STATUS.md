# Exact lane status

Astryx source: `957d9348bf8808373dcba68b9d32abb69ef12e15`. Exact: `94b8671f70fe2e25b307a40e7fce66dc27c9f802`.

- Dependency closure ported (core, by source files): 199 of 689 (28.9%).
- With the stretch rows: 205 of 720 (28.5%).
- Rows: 11 ported, 4 workaround, 0 blocked, 62 not started.

A row counts as ported when its Contract port gives the Astryx behavior and look its cases test,
workaround when it needs a different shape or an escape hatch (the note says which), and blocked
when Contract cannot express it (the note gives the smallest failing snippet).

| Row | Files | Groups | Wave | Status | Evidence | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| AppShell | 5 | core, stretch, template |  | not started |  |  |
| AspectRatio | 2 | core |  | not started |  |  |
| Avatar | 5 | stretch | 1 | ported | gallery avatar-* | Initials use the first letter of the first and last words (no Unicode-aware search). Image fallbackSrc and load error not yet wired. Tooltip waits for wave 4. |
| AvatarGroup | 1 | stretch | 1 | ported | gallery avatargroup-* | Members are data (AvatarItem[]), not child elements: one slot cannot be counted. |
| Badge | 2 | core, weakness | 1 | ported | gallery badge-* (15 variants) | Forced-colours outline not expressible. |
| BottomSheet | 11 | core, stretch, template |  | not started |  |  |
| Button | 3 | core, stretch, template, weakness |  | not started |  |  |
| ButtonGroup | 3 | core, stretch, template, weakness |  | not started |  |  |
| Calendar | 12 | core, stretch |  | not started |  |  |
| Card | 2 | core, template |  | not started |  |  |
| Center | 2 | core |  | not started |  |  |
| Chat | 10 | weakness |  | not started |  |  |
| CheckboxInput | 3 | core, stretch, weakness |  | not started |  |  |
| CheckboxList | 4 | stretch |  | not started |  |  |
| Collapsible | 5 | weakness |  | not started |  |  |
| CommandPalette | 10 | template |  | not started |  |  |
| ContextMenu | 2 | core |  | not started |  |  |
| DateInput | 21 | core, stretch |  | not started |  |  |
| DateRangeInput | 2 | stretch |  | not started |  |  |
| DateTimeInput | 2 | core, stretch |  | not started |  |  |
| Dialog | 5 | core, stretch, template |  | not started |  |  |
| Divider | 2 | core, stretch, template | 1 | ported | gallery divider-* | isFullBleed needs the container's padding, which Contract cannot read (no runtime CSS variables): ignored. |
| DropdownMenu | 18 | core, stretch, template |  | not started |  |  |
| EmptyState | 2 | core, stretch |  | not started |  |  |
| Field | 9 | core, stretch, template, weakness |  | not started |  |  |
| FieldStatus | 3 | core, stretch, template, weakness |  | not started |  |  |
| FormLayout | 1 | core, stretch, template, weakness |  | not started |  |  |
| Grid | 3 | core |  | not started |  |  |
| HStack | 2 | core, stretch, template |  | not started |  |  |
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
| Layout | 17 | core, stretch, template, weakness |  | not started |  |  |
| Link | 3 | core, stretch, template, weakness | 1 | ported | gallery link-* | Hover underline thickness refused (a state row paints only); hover dims instead. LinkProvider (router swap) is Contract's own `link`/routes. |
| List | 6 | core, stretch, weakness |  | not started |  |  |
| MobileNav | 2 | core |  | not started |  |  |
| MultiSelector | 3 | core |  | not started |  |  |
| NumberInput | 4 | core, stretch |  | not started |  |  |
| OverflowList | 2 | core, stretch |  | not started |  |  |
| Pagination | 2 | core |  | not started |  |  |
| Popover | 8 | core, stretch, template, weakness |  | not started |  |  |
| PowerSearch | 16 | stretch |  | not started |  |  |
| RadioList | 3 | stretch |  | not started |  |  |
| Resizable | 6 | core, template |  | not started |  |  |
| Section | 2 | core, stretch, template |  | not started |  |  |
| SegmentedControl | 6 | template |  | not started |  |  |
| SelectableCard | 3 | template |  | not started |  |  |
| Selector | 11 | core, stretch, template |  | not started |  |  |
| SideNav | 2 | core |  | not started |  |  |
| SizeContext | 2 | core, stretch, template, weakness | 0 | workaround | - | No context through a slot: size is a prop each child takes. |
| Spinner | 4 | core, stretch, template, weakness | 1 | ported | gallery spinner-* | Turn is a CSS keyframe; reduced motion not yet wired. |
| Stack | 5 | core, stretch, template |  | not started |  |  |
| Switch | 6 | core |  | not started |  |  |
| Table | 55 | core |  | not started |  |  |
| Text | 6 | core, stretch, template, weakness | 1 | ported | gallery text-0001..0024; web + Linux | `font` takes only literals, so the type role is a chain of literals. No inline-flex. `textWrap` keeps auto/balance plus nowrap; `pretty` is refused. Truncation tooltip waits for Tooltip (wave 4). |
| TextInput | 2 | core, stretch |  | not started |  |  |
| TimeInput | 5 | core, stretch |  | not started |  |  |
| Token | 4 | core, stretch |  | not started |  |  |
| Tokenizer | 6 | core, stretch |  | not started |  |  |
| Toolbar | 2 | core |  | not started |  |  |
| Tooltip | 4 | core, stretch, template, weakness |  | not started |  |  |
| TopNav | 2 | core |  | not started |  |  |
| TreeList | 5 | stretch, template |  | not started |  |  |
| Typeahead | 8 | core, stretch |  | not started |  |  |
| VStack | 2 | core, stretch, template |  | not started |  |  |
| VisuallyHidden | 2 | core, stretch, template, weakness | 0 | workaround | primitives.contract VisuallyHidden | Contract refuses clip-path inset(); a 1 px clipped box stands in. |
| infra:Foundations | 158 | core, stretch, template, weakness | 0 | workaround | astryx/astryx.tamber (251 tokens) and tokens/themes.tamber (8 themes, 1,066 per-theme values), imported by Exact's own `contract tamber import astryx`; gallery case foundations-0001 on web and Linux. | 279 of 286 resolved tokens map; 7 do not (duration min/max tiers, focus outline style). The 4 motion tokens cannot vary by theme (Tamber writes a duration or easing into its transition at build), so every theme uses the default theme's motion. A type role (`font`) varies by theme, so it must sit on each text, not on a container. Number and date formatting stay en-US in Contract (a data module would localise, but the Linux build carries none). |
| infra:Gestures | 68 | core, stretch, template, weakness |  | not started |  |  |
| infra:Layers | 41 | core, stretch, template, weakness |  | not started |  |  |
| infra:Motion | 42 | core, stretch, template, weakness |  | not started |  |  |
