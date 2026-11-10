// Each ported family for the gallery: its Contract component, Astryx's defaults for every prop
// (Contract has none), and smoke cases (single-prop sweeps) used until QA's case files land.
// `map` turns QA's Astryx-named props into the Contract component's where they differ.

const sweep = (family, base, axes) => {
  const out = [{ id: `${family.toLowerCase()}-0001`, props: { ...base } }];
  for (const [prop, values] of Object.entries(axes)) for (const v of values) out.push({ id: `${family.toLowerCase()}-${String(out.length + 1).padStart(4, '0')}`, props: { ...base, [prop]: v } });
  return out;
};

// Astryx's `children` (a text label) is `text` or `label` in the Contract component.
const childrenAs = key => p => {
  const { children, ...rest } = p;
  return children === undefined ? rest : { ...rest, [key]: String(children) };
};

const TEXT = t => `Text(text=${JSON.stringify(t)}, type="body", size="", color="primary", weight="", maxLines=0, textWrap="", justify="start", wordBreak="break-word", hasStrikethrough=false, hasTabularNumbers=false)`;
const STACK_CALL = 'gap=2, padding=-1, paddingInline=-1, paddingInlineStart=-1, paddingInlineEnd=-1, paddingBlock=-1, paddingBlockStart=-1, paddingBlockEnd=-1, hAlign="", vAlign="", align="stretch", justify="start", wrap="nowrap", isScrollable=false';
const CHOICES = [['free', 'Free', 'For one person'], ['pro', 'Pro', 'For small teams'], ['team', 'Team', 'For companies']].map(([value, label, description]) => ({ $shape: 'ChoiceItem', value, label, description, isDisabled: false }));
const ROWS = [['a', 'Write the brief'], ['b', 'Review the spec'], ['c', 'Ship the release'], ['d', 'Tell the team']].map(([key, label]) => ({ $shape: 'RowItem', key, label, description: '', icon: '', depth: 0, hasChildren: false, isExpanded: false }));
const TREE = [['src', 'src', 0, true, true], ['button', 'Button.tsx', 1, false, false], ['dialog', 'Dialog', 1, true, true], ['dialog-tsx', 'Dialog.tsx', 2, false, false], ['docs', 'docs', 0, true, false]].map(([key, label, depth, hasChildren, isExpanded]) => ({ $shape: 'RowItem', key, label, description: '', icon: '', depth, hasChildren, isExpanded }));
const MENU = [['edit', 'Edit', 'clock', '⌘E'], ['copy', 'Copy link', 'copy', '⌘C'], ['div', '', '', ''], ['delete', 'Delete', 'close', '']].map(([id, label, icon, shortcut]) => ({ $shape: 'MenuItem', id, label, icon, shortcut, isDestructive: id === 'delete', isDisabled: false, isChecked: false, isDivider: id === 'div' }));
const TCOLS = [['name', 'Name', '', 'start', true], ['role', 'Role', '', 'start', false], ['year', 'Born', '80px', 'end', true]].map(([key, label, width, align, isSortable]) => ({ $shape: 'TableColumn', key, label, width, align, isSortable }));
const TROWS = [['ada', 'Ada Lovelace', 'Analyst', '1815'], ['grace', 'Grace Hopper', 'Rear admiral', '1906'], ['alan', 'Alan Turing', 'Mathematician', '1912'], ['kj', 'Katherine Johnson', 'Mathematician', '1918']].map(([key, ...cells]) => ({ $shape: 'TableRow', key, cells, depth: 0, isExpanded: false, hasChildren: false }));
const FILL3 = ['Filler(label="One")', 'Filler(label="Two")', 'Filler(label="Three")'];
const avatars = ['Ada Lovelace', 'Grace Hopper', 'Alan Turing', 'Katherine Johnson', 'Edsger Dijkstra', 'Barbara Liskov'].map(name => ({ $shape: 'AvatarItem', name, src: '' }));

export const FAMILIES = {
  Playground2: {
    component: 'CompositesPlayground', uses: [], defaults: {},
    smoke: [{ id: 'playground-0002', title: 'Composites wired to state', props: {} }],
  },
  Playground3: {
    component: 'DataPlayground', uses: [], defaults: {},
    smoke: [{ id: 'playground-0003', title: 'Pickers and table wired to state', props: {} }],
  },
  Playground: {
    component: 'ControlsPlayground', uses: [], defaults: {},
    smoke: [{ id: 'playground-0001', title: 'Controls wired to state', props: {} }],
  },
  Foundations: {
    component: 'Swatches', uses: [], defaults: {},
    smoke: [{ id: 'foundations-0001', title: 'Token swatches', props: {} }],
  },
  Text: {
    component: 'Text', textProp: 'text', map: childrenAs('text'),
    defaults: { text: 'The quick brown fox jumps over the lazy dog', type: 'body', size: '', color: 'primary', weight: '', maxLines: 0, textWrap: '', justify: 'start', wordBreak: 'break-word', hasStrikethrough: false, hasTabularNumbers: false },
    smoke: sweep('Text', {}, { type: ['large', 'label', 'supporting', 'code', 'display-1', 'display-2', 'display-3'], size: ['xsm', 'sm', 'base', 'lg', 'xl', '2xl'], color: ['secondary', 'disabled', 'accent'], weight: ['medium', 'semibold', 'bold'], maxLines: [1], hasStrikethrough: [true], hasTabularNumbers: [true] }),
  },
  Heading: {
    component: 'Heading', textProp: 'text', map: childrenAs('text'),
    defaults: { text: 'Section heading', level: 1, accessibilityLevel: 0, weight: '', color: 'primary', textWrap: '', justify: 'start', wordBreak: 'break-word', hasStrikethrough: false },
    smoke: sweep('Heading', {}, { level: [1, 3, 4, 5, 6], color: ['secondary'] }),
  },
  Icon: {
    component: 'Icon',
    defaults: { icon: 'search', size: 'md', color: 'inherit', label: '' },
    smoke: sweep('Icon', {}, { icon: ['close', 'check', 'success', 'error', 'warning', 'info', 'calendar', 'clock', 'chevronDown', 'copy'], size: ['xsm', 'sm', 'lg'], color: ['primary', 'secondary', 'accent', 'success', 'error', 'warning'] }),
  },
  Spinner: {
    component: 'Spinner',
    defaults: { size: 'md', shade: 'default', label: '' },
    smoke: sweep('Spinner', {}, { size: ['sm', 'lg', 'xl'], shade: ['subtle', 'inherit'], label: ['Loading results'] }),
  },
  Kbd: {
    component: 'Kbd', defaults: { keys: 'Mod+K' },
    smoke: sweep('Kbd', {}, { keys: ['Esc', 'Shift+Enter', 'Ctrl+Alt+Delete'] }),
  },
  Divider: {
    component: 'Divider',
    defaults: { orientation: 'horizontal', label: '', variant: 'subtle', isFullBleed: false },
    smoke: sweep('Divider', {}, { variant: ['strong'], label: ['or'], orientation: ['vertical'] }),
  },
  Link: {
    component: 'Link', textProp: 'text', map: p => { const { label, ...r } = childrenAs('text')(p); return label === undefined ? r : { text: String(label), ...r }; },
    defaults: { text: 'Read the docs', href: '#', hasUnderline: false, isDisabled: false, isExternalLink: false, isStandalone: false, size: '', weight: '', color: 'accent' },
    smoke: sweep('Link', {}, { isExternalLink: [true], hasUnderline: [true], isDisabled: [true] }),
  },
  Badge: {
    component: 'Badge',
    defaults: { variant: 'neutral', label: 'Badge', icon: '' },
    smoke: sweep('Badge', {}, { variant: ['info', 'success', 'warning', 'error', 'blue', 'cyan', 'green', 'orange', 'pink', 'purple', 'red', 'teal', 'yellow', 'gray'], icon: ['check'] }),
  },
  Avatar: {
    component: 'Avatar', 
    defaults: { src: '', name: 'Ada Lovelace', size: 'md', px: 0, shape: 'circle', status: '', statusLabel: '' },
    map: p => (typeof p.size === 'number' ? { ...p, size: '', px: p.size } : p),
    smoke: sweep('Avatar', {}, { size: ['xsm', 'sm', 'lg', 'xl'], shape: ['rounded', 'square'], status: ['success', 'neutral', 'error'], name: ['Alice'] }),
  },
  AvatarGroup: {
    noSlot: true, component: 'AvatarGroup',
    defaults: { items: avatars, size: 'md', max: 4 },
    uses: ['AvatarGroup', 'AvatarItem'], types: { items: 'AvatarItem[]' },
    smoke: sweep('AvatarGroup', {}, { size: ['sm', 'lg'], max: [2, 6] }),
  },
  Indicator: {
    noSlot: true, component: 'CheckboxIndicator', qa: { file: 'Indicator', components: ['CheckboxIndicator'] }, uses: ['CheckboxIndicator'],
    defaults: { state: 'unchecked', size: 'md', isDisabled: false },
    smoke: sweep('Indicator', {}, { state: ['checked', 'indeterminate'], isDisabled: [true] }),
  },
  Stack: {
    component: 'Stack',
    defaults: { direction: 'vertical', gap: 0, padding: -1, paddingInline: -1, paddingInlineStart: -1, paddingInlineEnd: -1, paddingBlock: -1, paddingBlockStart: -1, paddingBlockEnd: -1, hAlign: '', vAlign: '', align: 'stretch', justify: 'start', wrap: 'nowrap', isScrollable: false },
    children: FILL3,
    smoke: sweep('Stack', {}, { direction: ['horizontal'], gap: [0, 1, 4, 8], padding: [2, 4], align: ['start', 'center', 'end'], justify: ['center', 'between'], wrap: ['wrap'] }),
    wrap: inner => ['view width=320 height=200', ...inner.map(l => `  ${l}`)],
  },
  HStack: {
    component: 'HStack', qa: { file: 'Stack', components: ['HStack'] }, defaults: { gap: 0, padding: -1, paddingInline: -1, paddingInlineStart: -1, paddingInlineEnd: -1, paddingBlock: -1, paddingBlockStart: -1, paddingBlockEnd: -1, hAlign: '', vAlign: '', align: 'stretch', justify: 'start', wrap: 'nowrap', isScrollable: false }, children: FILL3,
    smoke: sweep('HStack', {}, { gap: [0, 4], align: ['center'], justify: ['between'] }),
    wrap: inner => ['view width=320', ...inner.map(l => `  ${l}`)],
  },
  VStack: {
    component: 'VStack', qa: { file: 'Stack', components: ['VStack'] }, defaults: { gap: 0, padding: -1, paddingInline: -1, paddingInlineStart: -1, paddingInlineEnd: -1, paddingBlock: -1, paddingBlockStart: -1, paddingBlockEnd: -1, hAlign: '', vAlign: '', align: 'stretch', justify: 'start', wrap: 'nowrap', isScrollable: false }, children: FILL3,
    smoke: sweep('VStack', {}, { gap: [0, 4], align: ['center', 'end'] }),
    wrap: inner => ['view width=320', ...inner.map(l => `  ${l}`)],
  },
  Grid: {
    component: 'Grid', defaults: { columns: 3, minWidth: 0, gap: 2, rowGap: -1, columnGap: -1, align: '', justify: '' },
    children: ['Filler(label="1")', 'Filler(label="2")', 'Filler(label="3")', 'Filler(label="4")', 'Filler(label="5")'],
    smoke: sweep('Grid', {}, { columns: [2, 4], minWidth: [90], gap: [0, 4] }),
  },
  Center: {
    component: 'Center', defaults: { axis: 'both', padding: -1, paddingInline: -1, paddingInlineStart: -1, paddingInlineEnd: -1, paddingBlock: -1, paddingBlockStart: -1, paddingBlockEnd: -1 }, children: ['Filler(label="Centred")'],
    smoke: sweep('Center', {}, { axis: ['horizontal', 'vertical'], padding: [4] }),
  },
  AspectRatio: {
    component: 'AspectRatio', defaults: { ratio: 1.7778, shape: 'rectangle', fit: 'cover' }, children: ['view flex=1 background-color="astryx.surface.blue"'],
    smoke: sweep('AspectRatio', {}, { ratio: [1, 0.75], shape: ['ellipse'] }),
    wrap: inner => ['view width=240', ...inner.map(l => `  ${l}`)],
  },
  Section: {
    component: 'Section', defaults: { variant: 'section', padding: 4, paddingInline: -1, paddingInlineStart: -1, paddingInlineEnd: -1, paddingBlock: -1, paddingBlockStart: -1, paddingBlockEnd: -1, dividers: [], bleed: 0 }, types: { dividers: 'string[]' }, children: ['Text(text="Section content", type="body", size="", color="primary", weight="", maxLines=0, textWrap="", justify="start", wordBreak="break-word", hasStrikethrough=false, hasTabularNumbers=false)'],
    uses: ['Section', 'Text'],
    smoke: sweep('Section', {}, { variant: ['transparent', 'muted'], padding: [0, 2, 8], dividers: [['top', 'bottom']] }),
    wrap: inner => ['view width=320', ...inner.map(l => `  ${l}`)],
  },
  Card: {
    component: 'Card', defaults: { variant: 'default', padding: 4, elevation: 'none' },
    wrap: inner => ['view width=280', ...inner.map(l => `  ${l}`)],
    children: ['Text(text="Card content", type="body", size="", color="primary", weight="", maxLines=0, textWrap="", justify="start", wordBreak="break-word", hasStrikethrough=false, hasTabularNumbers=false)'], uses: ['Card', 'Text'],
    smoke: sweep('Card', {}, { variant: ['transparent', 'muted', 'blue', 'green', 'red'], padding: [0, 2, 8], elevation: ['low', 'med', 'high'] }),
  },
  Layout: {
    component: 'Layout', defaults: { height: 'fill', padding: 4, defaultHasDividers: false }, uses: ['Layout', 'LayoutHeader', 'LayoutPanel', 'LayoutContent', 'LayoutFooter', 'Text'],
    children: [
      'LayoutHeader(hasDivider=true, isFullBleed=false)', '  Filler(label="Header")',
      'LayoutPanel(side="start", width="120px", hasDivider=true)', '  Filler(label="Start")',
      'LayoutContent(contentWidth="", padding=4)', '  Filler(label="Content")',
      'LayoutPanel(side="end", width="100px", hasDivider=true)', '  Filler(label="End")',
      'LayoutFooter(hasDivider=true, isFullBleed=false)', '  Filler(label="Footer")',
    ],
    smoke: [{ id: 'layout-0001', props: {} }, { id: 'layout-0002', props: { height: 'auto' } }],
  },
  EmptyState: {
    component: 'EmptyState', defaults: { title: 'Title', description: '', icon: '', headingLevel: 3, isCompact: false },
    children: ['Filler(label="Action")'],
    smoke: sweep('EmptyState', {}, { isCompact: [true], icon: [''], description: [''] }),
  },
  Token: {
    component: 'Token', defaults: { label: 'Design', size: 'md', color: 'default', icon: '', isDisabled: false, removable: false },
    map: p => { const { onRemove, ...r } = p; return onRemove ? { ...r, removable: true } : r; },
    wrap: inner => [inner[0].replace(/\)$/, ', remove=noop)')],
    smoke: sweep('Token', {}, { size: ['sm', 'lg'], color: ['red', 'green', 'blue', 'purple'], icon: ['check'], isDisabled: [true], removable: [true] }),
  },
  Popover: {
    component: 'Popover', uses: ['Popover', 'Text'],
    defaults: { id: 'case-popover', label: 'Details', placement: 'below', alignment: 'center', padding: 3, role: 'dialog', hasCloseButton: false, closeButtonLabel: '', hasLightDismiss: true, still: true },
    opens: p => p.isOpen !== false && p.isEnabled !== false,
    children: [TEXT('Popover content')],
    wrap: inner => ['column align-items="center" width=360 height=220 justify-content="center"', '  button id="case-popover-trigger" popovertarget="case-popover" testId="popover-trigger" padding="8px 12px"', '    text "Open"', ...inner.map(l => `  ${l}`)],
    smoke: sweep('Popover', {}, { placement: ['above', 'start', 'end'], alignment: ['start', 'end'], padding: [0, 6], hasCloseButton: [true], isOpen: [false] }),
  },
  Collapsible: {
    component: 'Collapsible', uses: ['Collapsible', 'Text'],
    defaults: { title: 'Title', value: 'a', isOpen: false, isDisabled: false, chevronPosition: 'end', density: 'balanced', hasDivider: false, still: true },
    map: p => { const { defaultIsOpen, ...r } = p; return defaultIsOpen === undefined ? r : { ...r, isOpen: r.isOpen ?? defaultIsOpen }; },
    children: [TEXT('Ships in two business days.')],
    wrap: inner => ['view width=320', `  ${inner[0].replace(/\)$/, ', toggle=noop)')}`, ...inner.slice(1).map(l => `  ${l}`)],
    smoke: sweep('Collapsible', {}, { isOpen: [true], isDisabled: [true], chevronPosition: ['start'], density: ['compact', 'spacious'] }),
  },
  StackItem: {
    component: 'StackItem', qa: { file: 'Stack', components: ['StackItem'] }, uses: ['Stack', 'StackItem'],
    defaults: { size: 'static', isScrollable: false, crossAlignSelf: '' },
    children: ['Filler(label="Item")'],
    wrap: inner => ['view width=320 height=120', `  Stack(direction="horizontal", ${STACK_CALL})`, '    Filler(label="Before")', ...inner.map(l => `    ${l}`), '    Filler(label="After")'],
    smoke: sweep('StackItem', {}, { size: ['fill'], crossAlignSelf: ['center', 'end', 'stretch'] }),
  },
  CheckIndicator: {
    noSlot: true, component: 'CheckIndicator', qa: { file: 'Indicator', components: ['CheckIndicator'] },
    defaults: { state: 'unchecked', size: 'md', isDisabled: false },
    smoke: sweep('CheckIndicator', {}, { state: ['checked'] }),
  },
  RadioIndicator: {
    noSlot: true, component: 'RadioIndicator', qa: { file: 'Indicator', components: ['RadioIndicator'] },
    defaults: { state: 'unchecked', size: 'md', isDisabled: false },
    smoke: sweep('RadioIndicator', {}, { state: ['checked'] }),
  },
  Tooltip: {
    component: 'Tooltip', uses: ['Tooltip'],
    defaults: { id: 'case-popover', content: 'Label', placement: 'above', alignment: 'center', delay: 0, isEnabled: true, focusTrigger: 'auto', still: true },
    opens: p => p.isOpen !== false && p.isEnabled !== false,
    children: ['Filler(label="Hover me")'],
    wrap: inner => ['column align-items="center" justify-content="center" width=320 height=160', ...inner.map(l => `  ${l}`)],
    smoke: sweep('Tooltip', {}, { placement: ['below', 'start', 'end'], alignment: ['start', 'end'] }),
  },
  InfoTip: {
    component: 'InfoTip',
    defaults: { id: 'case-popover', content: 'Billing runs on the first of the month.', label: 'More information', size: 'sm' },
    opens: () => true,
    wrap: inner => ['row align-items="center" justify-content="center" width=320 height=160', ...inner.map(l => `  ${l}`)],
    smoke: sweep('InfoTip', {}, { size: ['xsm', 'md', 'lg'] }),
  },
  HoverCard: {
    component: 'HoverCard', uses: ['HoverCard'],
    defaults: { id: 'case-popover', title: 'Ada Lovelace', body: 'Wrote the first published algorithm.', label: '', placement: 'below', alignment: 'center', isEnabled: true, still: true },
    opens: p => p.isOpen !== false && p.isEnabled !== false,
    children: ['Filler(label="@ada")'],
    wrap: inner => ['column align-items="center" justify-content="center" width=360 height=200', ...inner.map(l => `  ${l}`)],
    smoke: sweep('HoverCard', {}, { placement: ['above', 'end'], alignment: ['start'] }),
  },
  Button: {
    noSlot: true, component: 'Button', actions: { press: 'noop' },
    defaults: { label: 'Label', variant: 'secondary', size: 'md', elevation: 'none', icon: '', isLoading: false, isDisabled: false, isIconOnly: false, groupEdge: '' },
    map: p => (p.isIconOnly && !p.icon ? { ...p, icon: 'search' } : p),
    smoke: sweep('Button', {}, { variant: ['primary', 'ghost', 'destructive'], size: ['sm', 'lg'], isLoading: [true], isDisabled: [true], isIconOnly: [true], icon: ['check'] }),
  },
  IconButton: {
    component: 'IconButton', actions: { press: 'noop' },
    defaults: { label: 'Search', icon: 'search', variant: 'ghost', size: 'md', isDisabled: false },
    smoke: sweep('IconButton', {}, { variant: ['primary', 'secondary'], size: ['sm', 'lg'], isDisabled: [true] }),
  },
  ButtonGroup: {
    component: 'ButtonGroup', uses: ['ButtonGroup', 'Button'],
    defaults: { label: 'Label', orientation: 'horizontal', elevation: 'none' },
    children: ['first', 'middle', 'last'].map((e, i) => `Button(label=${JSON.stringify(['Day', 'Week', 'Month'][i])}, variant="secondary", size="md", elevation="none", icon="", isLoading=false, isDisabled=false, isIconOnly=false, groupEdge=${JSON.stringify(e)}, press=noop)`),
    smoke: sweep('ButtonGroup', {}, { orientation: ['vertical'], elevation: ['low'] }),
  },
  Switch: {
    component: 'Switch', actions: { change: 'noopB' },
    defaults: { label: 'Label', isOn: false, size: 'md', isDisabled: false, isLoading: false, isLabelHidden: false, labelPosition: 'end', labelSpacing: 'hug', isRequired: false, isOptional: false, still: true },
    map: p => { const { value, isChecked, ...r } = p; return { ...r, ...(isChecked !== undefined ? { isOn: isChecked } : {}) }; },
    smoke: sweep('Switch', {}, { isOn: [true], size: ['sm'], isDisabled: [true], isLoading: [true], isLabelHidden: [true], labelPosition: ['start'], labelSpacing: ['spread'] }),
    wrap: inner => ['view width=280', ...inner.map(l => `  ${l}`)],
  },
  TextInput: {
    component: 'TextInput', actions: { input: 'noopS', clear: 'noop' },
    defaults: { id: 'case-input', label: 'Label', value: '', placeholder: 'Placeholder', description: '', size: 'md', isLabelHidden: false, isDisabled: false, isReadOnly: false, isRequired: false, isOptional: false, isLoading: false, hasClear: false, hasAutoFocus: false, autoCapitalize: '', statusMessage: '', statusVariant: 'error', statusPlacement: 'attached' },
    map: p => { const { statusVariant, ...r } = p; return statusVariant ? { ...r, statusPlacement: statusVariant, statusMessage: 'Enter a valid value.' } : r; },
    smoke: sweep('TextInput', {}, { value: ['value'], size: ['sm', 'lg'], isDisabled: [true], hasClear: [true], statusMessage: ['Enter a valid value.'] }),
    wrap: inner => ['view width=320', ...inner.map(l => `  ${l}`)],
  },
  NumberInput: {
    component: 'NumberInput', actions: { input: 'noopS', stepUp: 'noop', stepDown: 'noop' },
    defaults: { id: 'case-number', label: 'Quantity', value: '12', size: 'md', isDisabled: false },
    smoke: sweep('NumberInput', {}, { size: ['sm', 'lg'], isDisabled: [true] }),
    wrap: inner => ['view width=200', ...inner.map(l => `  ${l}`)],
  },
  CheckboxInput: {
    component: 'CheckboxInput', actions: { change: 'noopB' },
    defaults: { label: 'Email me updates', state: 'unchecked', description: '', isDisabled: false },
    smoke: sweep('CheckboxInput', {}, { state: ['checked', 'indeterminate'], description: ['Once a week.'], isDisabled: [true] }),
  },
  RadioList: {
    noSlot: true, component: 'RadioList', actions: { change: 'noopS' }, uses: ['RadioList', 'ChoiceItem'], types: { items: 'ChoiceItem[]' },
    defaults: { name: 'plan', label: 'Plan', items: CHOICES, value: 'pro', isDisabled: false },
    smoke: sweep('RadioList', {}, { value: ['free'], isDisabled: [true] }),
  },
  CheckboxList: {
    noSlot: true, component: 'CheckboxList', actions: { change: 'noopSB' }, uses: ['CheckboxList', 'ChoiceItem'], types: { items: 'ChoiceItem[]', values: 'string[]' },
    defaults: { label: 'Plans', items: CHOICES, values: ['pro'], isDisabled: false },
    smoke: sweep('CheckboxList', {}, { values: [['free', 'team']], isDisabled: [true] }),
  },
  Field: {
    component: 'Field', uses: ['Field', 'TextInput'],
    defaults: { label: 'Label', inputID: 'inputID', description: '', isLabelHidden: false, isDisabled: false, isRequired: false, isOptional: false, statusMessage: '', statusVariant: 'error', statusPlacement: 'attached' },
    map: p => { const { statusVariant, isGroupLabel, ...r } = p; return statusVariant ? { ...r, statusPlacement: statusVariant, statusMessage: 'Enter a valid value.' } : r; },
    children: ['TextInput(id="inputID", label="Label", value="", placeholder="Placeholder", description="", size="md", isLabelHidden=true, isDisabled=false, isReadOnly=false, isRequired=false, isOptional=false, isLoading=false, hasClear=false, hasAutoFocus=false, autoCapitalize="", statusMessage="", statusVariant="error", statusPlacement="attached", input=noopS, clear=noop)'],
    smoke: sweep('Field', {}, { description: ['Shown on your profile.'], isRequired: [true], statusMessage: ['Enter a valid value.'] }),
    wrap: inner => ['view width=320', ...inner.map(l => `  ${l}`)],
  },
  FieldStatus: {
    component: 'FieldStatus',
    defaults: { message: 'Enter a valid email address.', variant: 'error', placement: 'detached' },
    smoke: sweep('FieldStatus', {}, { variant: ['warning', 'success', 'info'], placement: ['attached'] }),
    wrap: inner => ['view width=320', ...inner.map(l => `  ${l}`)],
  },
  InputGroup: {
    component: 'InputGroup', uses: ['InputGroup', 'InputGroupText'],
    defaults: { label: 'Label', size: 'md', isDisabled: false },
    map: p => { const { isLabelHidden, isOptional, isRequired, ...r } = p; return r; },
    children: ['InputGroupText(text="https://")', 'input value="astryx.design" aria-label="Website" flex=1 min-width=0 border-width=0 background-color="transparent" font="astryx.type.body" color="astryx.text.primary"'],
    smoke: sweep('InputGroup', {}, { size: ['sm', 'lg'], isDisabled: [true] }),
    wrap: inner => ['view width=320', ...inner.map(l => `  ${l}`)],
  },
  SegmentedControl: {
    noSlot: true, component: 'SegmentedControl', actions: { change: 'noopS' }, uses: ['SegmentedControl', 'ChoiceItem'], types: { items: 'ChoiceItem[]' },
    defaults: { label: 'Label', items: CHOICES, value: 'pro', size: 'md', layout: 'hug', isDisabled: false },
    map: p => { const { value, onChange, ...r } = p; return r; },
    smoke: sweep('SegmentedControl', {}, { size: ['sm', 'lg'], layout: ['fill'], isDisabled: [true] }),
    wrap: inner => ['view width=360', ...inner.map(l => `  ${l}`)],
  },
  SelectableCard: {
    component: 'SelectableCard', actions: { press: 'noop' }, uses: ['SelectableCard', 'Text'],
    defaults: { label: 'Label', isSelected: false, isDisabled: false, padding: 4, variant: 'default', elevation: 'none' },
    children: [TEXT('Pro plan, billed yearly')],
    smoke: sweep('SelectableCard', {}, { isSelected: [true], isDisabled: [true], elevation: ['low'] }),
    wrap: inner => ['view width=280', ...inner.map(l => `  ${l}`)],
  },
  Item: {
    component: 'Item',
    defaults: { label: 'Ada Lovelace', description: 'Analyst', icon: 'calendar', endText: 'Today', density: 'balanced', isSelected: false, isHighlighted: false, isDisabled: false, labelLines: 0 },
    smoke: sweep('Item', {}, { density: ['compact', 'spacious'], isSelected: [true], isHighlighted: [true], isDisabled: [true], icon: [''], description: [''] }),
    wrap: inner => ['view width=320', ...inner.map(l => `  ${l}`)],
  },
  List: {
    component: 'List', uses: ['List', 'ListItem'],
    defaults: { label: 'Team', density: 'balanced', hasDividers: false, edge: 'padded', listStyle: 'none' },
    children: ['Ada Lovelace', 'Grace Hopper', 'Alan Turing'].map((n, i) => `ListItem(label=${JSON.stringify(n)}, description="", icon="", marker=(c.listStyle == "decimal" ? "${i + 1}." : c.listStyle == "disc" ? "•" : c.listStyle == "circle" ? "◦" : ""), density=c.density, hasDivider=${i > 0 ? 'c.hasDividers' : 'false'}, isSelected=false, isDisabled=false)`),
    smoke: sweep('List', {}, { density: ['compact', 'spacious'], hasDividers: [true], listStyle: ['decimal', 'disc'] }),
    wrap: inner => ['view width=320', ...inner.map(l => `  ${l}`)],
  },
  ReorderList: {
    component: 'ReorderList', actions: { reorder: 'noopSO' }, uses: ['ReorderList', 'RowItem'], types: { items: 'RowItem[]' },
    defaults: { id: 'case-reorder', label: 'Tasks', items: ROWS, density: 'balanced', hasDividers: true },
    smoke: sweep('ReorderList', {}, { density: ['compact'], hasDividers: [false] }),
    wrap: inner => ['view width=320 height=240', ...inner.map(l => `  ${l}`)],
  },
  TreeList: {
    component: 'TreeList', actions: { toggle: 'noopS', select: 'noopS' }, uses: ['TreeList', 'RowItem'], types: { items: 'RowItem[]' },
    defaults: { label: 'Files', items: TREE, selected: 'button', density: 'balanced', variant: 'lineGuides' },
    smoke: sweep('TreeList', {}, { density: ['compact', 'spacious'], variant: ['noGuides'] }),
    wrap: inner => ['view width=320', ...inner.map(l => `  ${l}`)],
  },
  Dialog: {
    component: 'Dialog', uses: ['Dialog', 'Text', 'Button'],
    defaults: { id: 'case-dialog', title: 'Title', purpose: 'form', variant: 'standard', padding: 6, hasCloseButton: true, still: true },
    opensModal: p => p.isOpen !== false && p.isInline !== true,
    map: p => { const { isOpen, isInline, ...r } = p; return r; },
    children: [TEXT('This removes the project and its history.'), 'row gap="astryx.space.s2" justify-content="flex-end"', '  Button(label="Cancel", variant="secondary", size="md", elevation="none", icon="", isLoading=false, isDisabled=false, isIconOnly=false, groupEdge="", press=noop)', '  Button(label="Delete", variant="destructive", size="md", elevation="none", icon="", isLoading=false, isDisabled=false, isIconOnly=false, groupEdge="", press=noop)'],
    smoke: sweep('Dialog', {}, { purpose: ['required', 'info'], variant: ['fullscreen'], padding: [2] }),
  },
  BottomSheet: {
    component: 'BottomSheet', uses: ['BottomSheet', 'Text'],
    defaults: { id: 'case-dialog', title: 'Share', still: true },
    opensModal: () => true,
    children: [TEXT('Send this page to someone.')],
    smoke: sweep('BottomSheet', {}, { title: ['Filters'] }),
  },
  DropdownMenu: {
    component: 'DropdownMenu', actions: { select: 'noopS' }, uses: ['DropdownMenu', 'MenuItem'], types: { items: 'MenuItem[]' },
    defaults: { id: 'case-popover', label: 'Actions', items: MENU, variant: 'secondary', size: 'md', hasChevron: true },
    opens: () => true,
    wrap: inner => ['column width=320 height=280', ...inner.map(l => `  ${l}`)],
    smoke: sweep('DropdownMenu', {}, { label: ['More'] }),
  },
  ContextMenu: {
    component: 'ContextMenu', actions: { select: 'noopS' }, uses: ['ContextMenu', 'MenuItem'], types: { items: 'MenuItem[]' },
    defaults: { id: 'case-popover', label: 'Row actions', items: MENU },
    children: ['Filler(label="Right-click or long-press me")'],
    wrap: inner => ['column width=320 height=280', ...inner.map(l => `  ${l}`)],
    smoke: sweep('ContextMenu', {}, {}),
  },
  Resizable: {
    component: 'Resizable', actions: { resize: 'noopN' }, uses: ['Resizable'],
    defaults: { label: 'Resize sidebar', width: 160, minWidth: 120, maxWidth: 280 },
    children: ['Filler(label="Sidebar")'],
    wrap: inner => ['view width=360 height=160', ...inner.map(l => `  ${l}`)],
    smoke: sweep('Resizable', {}, { width: [240] }),
  },
  Typeahead: {
    component: 'Typeahead', actions: { input: 'noopS', pick: 'noopS', clear: 'noop' }, uses: ['Typeahead', 'ChoiceItem'], types: { options: 'ChoiceItem[]' },
    defaults: { id: 'case-typeahead', label: 'Label', query: '', options: CHOICES, selected: '', placeholder: 'Placeholder', size: 'md', isDisabled: false, isLabelHidden: false, hasClear: false, hasEntriesOnFocus: false },
    map: p => { const { statusVariant, isRequired, isOptional, hasAutoFocus, ...r } = p; return r; },
    wrap: inner => ['view width=320', ...inner.map(l => `  ${l}`)],
    smoke: sweep('Typeahead', {}, { query: ['Pr'], size: ['sm', 'lg'], isDisabled: [true], hasClear: [true] }),
  },
  Tokenizer: {
    component: 'Tokenizer', actions: { input: 'noopS', add: 'noopS', remove: 'noopS' }, uses: ['Tokenizer', 'ChoiceItem'], types: { options: 'ChoiceItem[]', values: 'string[]' },
    defaults: { id: 'case-tokenizer', label: 'Label', query: '', options: CHOICES, values: ['free', 'pro'] },
    wrap: inner => ['view width=320', ...inner.map(l => `  ${l}`)],
    smoke: sweep('Tokenizer', {}, { values: [[]] }),
  },
  Selector: {
    component: 'Selector', actions: { change: 'noopS' }, uses: ['Selector', 'ChoiceItem'], types: { options: 'ChoiceItem[]', values: 'string[]' },
    defaults: { id: 'case-selector', label: 'Label', options: CHOICES, value: 'pro', placeholder: 'Choose', size: 'md', isDisabled: false, isMultiple: false, values: [] },
    wrap: inner => ['view width=280', ...inner.map(l => `  ${l}`)],
    smoke: sweep('Selector', {}, { value: [''], size: ['sm', 'lg'], isDisabled: [true] }),
  },
  MultiSelector: {
    component: 'Selector', actions: { change: 'noopS' }, uses: ['Selector', 'ChoiceItem'], types: { options: 'ChoiceItem[]', values: 'string[]' },
    defaults: { id: 'case-multi', label: 'Label', options: CHOICES, value: '', placeholder: 'Choose', size: 'md', isDisabled: false, isMultiple: true, values: ['free', 'team'] },
    wrap: inner => ['view width=280', ...inner.map(l => `  ${l}`)],
    smoke: sweep('MultiSelector', {}, { values: [[]] }),
  },
  Calendar: {
    component: 'Calendar', actions: { pick: 'noopS', shift: 'noopN' },
    defaults: { year: 2026, month: 11, selected: '2026-11-18', rangeStart: '', rangeEnd: '', today: '2026-11-10' },
    smoke: sweep('Calendar', {}, { selected: [''], month: [2], rangeStart: ['2026-11-04'] }).map(c => (c.props.rangeStart ? { ...c, props: { ...c.props, selected: '', rangeEnd: '2026-11-12' } } : c)),
  },
  DateInput: {
    component: 'DateInput', actions: { input: 'noopS', pick: 'noopS', shift: 'noopN' },
    defaults: { id: 'case-date', label: 'Label', value: '2026-11-18', endValue: '', isRange: false, year: 2026, month: 11, today: '2026-11-10', size: 'md', isDisabled: false },
    wrap: inner => ['view width=280', ...inner.map(l => `  ${l}`)],
    smoke: sweep('DateInput', {}, { value: [''], size: ['sm', 'lg'], isDisabled: [true] }),
  },
  DateRangeInput: {
    component: 'DateInput', actions: { input: 'noopS', pick: 'noopS', shift: 'noopN' },
    defaults: { id: 'case-range', label: 'Label', value: '2026-11-04', endValue: '2026-11-12', isRange: true, year: 2026, month: 11, today: '2026-11-10', size: 'md', isDisabled: false },
    wrap: inner => ['view width=280', ...inner.map(l => `  ${l}`)],
    smoke: sweep('DateRangeInput', {}, { value: [''] }),
  },
  TimeInput: {
    component: 'TimeInput', actions: { input: 'noopS' },
    defaults: { id: 'case-time', label: 'Label', value: '09:30', size: 'md', isDisabled: false },
    smoke: sweep('TimeInput', {}, { value: [''], size: ['sm', 'lg'], isDisabled: [true] }),
  },
  Pagination: {
    component: 'Pagination', actions: { go: 'noopN' },
    defaults: { page: 3, pageCount: 7, label: 'Pages' },
    smoke: sweep('Pagination', {}, { page: [1, 7], pageCount: [3] }),
  },
  Table: {
    component: 'Table', actions: { sort: 'noopS', toggleRow: 'noopSB2', toggleAll: 'noopB', toggleExpand: 'noopS' }, uses: ['Table', 'TableColumn', 'TableRow'], types: { columns: 'TableColumn[]', rows: 'TableRow[]', selected: 'string[]' },
    noSlot: true,
    defaults: { label: 'People', columns: TCOLS, rows: TROWS, density: 'balanced', dividers: 'rows', isStriped: false, hasHover: false, verticalAlign: 'middle', textOverflow: 'wrap', sortKey: '', sortDirection: 'asc', hasSelection: false, selected: [] },
    wrap: inner => ['view width=480', ...inner.map(l => `  ${l}`)],
    smoke: sweep('Table', {}, { density: ['compact', 'spacious'], dividers: ['columns', 'grid', 'none'], isStriped: [true], hasSelection: [true], sortKey: ['name'] }),
  },
};
