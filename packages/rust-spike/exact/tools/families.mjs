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
const FILL3 = ['Filler(label="One")', 'Filler(label="Two")', 'Filler(label="Three")'];
const avatars = ['Ada Lovelace', 'Grace Hopper', 'Alan Turing', 'Katherine Johnson', 'Edsger Dijkstra', 'Barbara Liskov'].map(name => ({ $shape: 'AvatarItem', name, src: '' }));

export const FAMILIES = {
  Foundations: {
    component: 'Swatches', uses: [], defaults: {},
    smoke: [{ id: 'foundations-0001', title: 'Token swatches', props: {} }],
  },
  Text: {
    component: 'Text', map: childrenAs('text'),
    defaults: { text: 'The quick brown fox jumps over the lazy dog', type: 'body', size: '', color: 'primary', weight: '', maxLines: 0, textWrap: '', justify: 'start', wordBreak: 'break-word', hasStrikethrough: false, hasTabularNumbers: false },
    smoke: sweep('Text', {}, { type: ['large', 'label', 'supporting', 'code', 'display-1', 'display-2', 'display-3'], size: ['xsm', 'sm', 'base', 'lg', 'xl', '2xl'], color: ['secondary', 'disabled', 'accent'], weight: ['medium', 'semibold', 'bold'], maxLines: [1], hasStrikethrough: [true], hasTabularNumbers: [true] }),
  },
  Heading: {
    component: 'Heading', map: childrenAs('text'),
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
    wrap: inner => ['view width=240 height=80 display="flex" align-items="center" justify-content="center"', ...inner.map(l => `  ${l}`)],
  },
  Link: {
    component: 'Link', map: childrenAs('text'),
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
    component: 'AvatarGroup',
    defaults: { items: avatars, size: 'md', max: 4 },
    uses: ['AvatarGroup', 'AvatarItem'], types: { items: 'AvatarItem[]' },
    smoke: sweep('AvatarGroup', {}, { size: ['sm', 'lg'], max: [2, 6] }),
  },
  Indicator: {
    component: 'CheckboxIndicator', qa: { file: 'Indicator', components: ['CheckboxIndicator'] }, uses: ['CheckboxIndicator'],
    defaults: { state: 'unchecked', size: 'md', isDisabled: false },
    smoke: sweep('Indicator', {}, { state: ['checked', 'indeterminate'], isDisabled: [true] }),
  },
  Stack: {
    component: 'Stack',
    defaults: { direction: 'vertical', gap: 0, padding: -1, paddingInline: -1, paddingInlineStart: -1, paddingInlineEnd: -1, paddingBlock: -1, paddingBlockStart: -1, paddingBlockEnd: -1, hAlign: '', vAlign: '', align: 'stretch', justify: 'start', wrap: 'nowrap', isScrollable: false },
    children: FILL3,
    smoke: sweep('Stack', {}, { direction: ['horizontal'], gap: [0, 1, 4, 8], padding: [2, 4], align: ['start', 'center', 'end'], justify: ['center', 'between'], wrap: ['wrap'] }),
    wrap: inner => ['view width=320 height=200 background-color="astryx.surface.muted"', ...inner.map(l => `  ${l}`)],
  },
  HStack: {
    component: 'HStack', qa: { file: 'Stack', components: ['HStack'] }, defaults: { gap: 0, padding: -1, paddingInline: -1, paddingInlineStart: -1, paddingInlineEnd: -1, paddingBlock: -1, paddingBlockStart: -1, paddingBlockEnd: -1, hAlign: '', vAlign: '', align: 'stretch', justify: 'start', wrap: 'nowrap', isScrollable: false }, children: FILL3,
    smoke: sweep('HStack', {}, { gap: [0, 4], align: ['center'], justify: ['between'] }),
    wrap: inner => ['view width=320 background-color="astryx.surface.muted"', ...inner.map(l => `  ${l}`)],
  },
  VStack: {
    component: 'VStack', qa: { file: 'Stack', components: ['VStack'] }, defaults: { gap: 0, padding: -1, paddingInline: -1, paddingInlineStart: -1, paddingInlineEnd: -1, paddingBlock: -1, paddingBlockStart: -1, paddingBlockEnd: -1, hAlign: '', vAlign: '', align: 'stretch', justify: 'start', wrap: 'nowrap', isScrollable: false }, children: FILL3,
    smoke: sweep('VStack', {}, { gap: [0, 4], align: ['center', 'end'] }),
    wrap: inner => ['view width=320 background-color="astryx.surface.muted"', ...inner.map(l => `  ${l}`)],
  },
  Grid: {
    component: 'Grid', defaults: { columns: 3, minWidth: 0, gap: 2, rowGap: -1, columnGap: -1, align: '', justify: '' },
    wrap: inner => ['view width=320', ...inner.map(l => `  ${l}`)],
    children: ['Filler(label="1")', 'Filler(label="2")', 'Filler(label="3")', 'Filler(label="4")', 'Filler(label="5")'],
    smoke: sweep('Grid', {}, { columns: [2, 4], minWidth: [90], gap: [0, 4] }),
  },
  Center: {
    component: 'Center', defaults: { axis: 'both', padding: -1, paddingInline: -1, paddingInlineStart: -1, paddingInlineEnd: -1, paddingBlock: -1, paddingBlockStart: -1, paddingBlockEnd: -1 }, children: ['Filler(label="Centred")'],
    smoke: sweep('Center', {}, { axis: ['horizontal', 'vertical'], padding: [4] }),
    wrap: inner => ['view width=240 height=120 background-color="astryx.surface.muted"', ...inner.map(l => `  ${l}`)],
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
    wrap: inner => ['view width=600 height=360 background-color="astryx.surface.surface"', ...inner.map(l => `  ${l}`)],
  },
  EmptyState: {
    component: 'EmptyState', defaults: { title: 'Title', description: 'Try a different search.', icon: 'search', headingLevel: 3, isCompact: false },
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
    defaults: { title: 'Shipping details', value: 'a', isOpen: false, isDisabled: false, chevronPosition: 'end', density: 'balanced', hasDivider: false, still: true },
    map: p => { const { defaultIsOpen, ...r } = p; return defaultIsOpen === undefined ? r : { ...r, isOpen: r.isOpen ?? defaultIsOpen }; },
    children: [TEXT('Ships in two business days.')],
    wrap: inner => ['view width=320', `  ${inner[0].replace(/\)$/, ', toggle=noop)')}`, ...inner.slice(1).map(l => `  ${l}`)],
    smoke: sweep('Collapsible', {}, { isOpen: [true], isDisabled: [true], chevronPosition: ['start'], density: ['compact', 'spacious'] }),
  },
  StackItem: {
    component: 'StackItem', qa: { file: 'Stack', components: ['StackItem'] }, uses: ['Stack', 'StackItem'],
    defaults: { size: 'static', isScrollable: false, crossAlignSelf: '' },
    children: ['Filler(label="Item")'],
    wrap: inner => ['view width=320 height=120 background-color="astryx.surface.muted"', `  Stack(direction="horizontal", ${STACK_CALL})`, '    Filler(label="Before")', ...inner.map(l => `    ${l}`), '    Filler(label="After")'],
    smoke: sweep('StackItem', {}, { size: ['fill'], crossAlignSelf: ['center', 'end', 'stretch'] }),
  },
  CheckIndicator: {
    component: 'CheckIndicator', qa: { file: 'Indicator', components: ['CheckIndicator'] },
    defaults: { state: 'unchecked', size: 'md', isDisabled: false },
    smoke: sweep('CheckIndicator', {}, { state: ['checked'] }),
  },
  RadioIndicator: {
    component: 'RadioIndicator', qa: { file: 'Indicator', components: ['RadioIndicator'] },
    defaults: { state: 'unchecked', size: 'md', isDisabled: false },
    smoke: sweep('RadioIndicator', {}, { state: ['checked'] }),
  },
  Tooltip: {
    component: 'Tooltip', uses: ['Tooltip'],
    defaults: { id: 'case-popover', content: 'Copy link', placement: 'above', alignment: 'center', delay: 0, isEnabled: true, focusTrigger: 'auto', still: true },
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
};
