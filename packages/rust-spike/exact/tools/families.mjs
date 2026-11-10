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

const avatars = ['Ada Lovelace', 'Grace Hopper', 'Alan Turing', 'Katherine Johnson', 'Edsger Dijkstra', 'Barbara Liskov'].map(name => ({ $shape: 'AvatarItem', name, src: '' }));

export const FAMILIES = {
  Foundations: {
    component: 'Swatches', uses: [], defaults: {},
    smoke: [{ id: 'foundations-0001', title: 'Token swatches', props: {} }],
  },
  Text: {
    component: 'Text', map: childrenAs('text'),
    defaults: { text: 'The quick brown fox jumps over the lazy dog', type: 'body', size: '', color: 'primary', weight: '', maxLines: 0, textWrap: '', justify: 'start', hasStrikethrough: false, hasTabularNumbers: false },
    smoke: sweep('Text', {}, { type: ['large', 'label', 'supporting', 'code', 'display-1', 'display-2', 'display-3'], size: ['xsm', 'sm', 'base', 'lg', 'xl', '2xl'], color: ['secondary', 'disabled', 'accent'], weight: ['medium', 'semibold', 'bold'], maxLines: [1], hasStrikethrough: [true], hasTabularNumbers: [true] }),
  },
  Heading: {
    component: 'Heading', map: childrenAs('text'),
    defaults: { text: 'Section heading', level: 2, color: 'primary' },
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
    wrap: call => ['view width=240 height=80 display="flex" align-items="center" justify-content="center"', `  ${call}`],
  },
  Link: {
    component: 'Link', map: childrenAs('text'),
    defaults: { text: 'Read the docs', href: '#', isExternal: false },
    smoke: sweep('Link', {}, { isExternal: [true] }),
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
    component: 'CheckboxIndicator', uses: ['CheckboxIndicator', 'RadioIndicator', 'CheckIndicator'],
    defaults: { checked: 'false', disabled: false },
    smoke: sweep('Indicator', {}, { checked: ['true', 'mixed'], disabled: [true] }),
  },
};
