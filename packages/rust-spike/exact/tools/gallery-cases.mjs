// The gallery's cases → apps/astryx-gallery/cases.contract. The gallery has no data module (the
// Linux host builds without one), so each case is a literal call in Contract, written here.
//
// A case is {id, props}. Cases come from QA's packages/rust-spike/qa/cases/<Family>.json when it
// exists, else from SMOKE below. Each family names its Contract component and Astryx's defaults:
// Contract props have no defaults, so every call passes every prop.
//
//   bun tools/gallery-cases.mjs
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { FAMILIES } from './families.mjs';

const HERE = resolve(dirname(new URL(import.meta.url).pathname), '..');
const QA = resolve(HERE, '../qa/cases');

const lit = v => {
  if (typeof v === 'string') return JSON.stringify(v);
  if (typeof v === 'number' || typeof v === 'boolean') return String(v);
  if (Array.isArray(v)) return `[${v.map(lit).join(', ')}]`;
  if (v && typeof v === 'object' && v.$shape) {
    const { $shape, ...fields } = v;
    return `${$shape}(${Object.entries(fields).map(([k, x]) => `${k}=${lit(x)}`).join(', ')})`;
  }
  throw new Error(`no Contract literal for ${JSON.stringify(v)}`);
};

// Each family's cases are data: a list of one shape, read by one call. Contract expands a
// component at each place it is used, so one call per case would repeat the component per case.
// QA's case file for a family: components[] each with cases[] {id, kind, props}. A family reads
// the components `def.qa` names (default: the one named like the family).
const qaFixture = {};
function qaCases(family, def) {
  const file = resolve(QA, `${def.qa?.file ?? family}.json`);
  if (!existsSync(file)) return null;
  const doc = JSON.parse(readFileSync(file, 'utf8'));
  const names = def.qa?.components ?? [def.component, family];
  const picked = doc.components.filter(c => names.includes(c.component));
  if (!picked.length) return null;
  qaFixture[family] = picked[0].fixture ?? null;
  return picked.flatMap(c => c.cases.map(k => ({ id: k.id, title: `${c.component} ${k.kind} ${JSON.stringify(k.props)}`, props: { ...(c.base ?? {}), ...k.props } })));
}

const TEXT_LABEL = 'Text(text="Label", type="body", size="", color="primary", weight="", maxLines=0, textWrap="", justify="start", wordBreak="break-word", hasStrikethrough=false, hasTabularNumbers=false)';
const typeOf = (v, k, def) => def.types?.[k] ?? (typeof v === 'string' ? 'string' : typeof v === 'number' ? 'number' : typeof v === 'boolean' ? 'bool' : null);
const rows = [];
const families = [];
const dropped = {};
const opens = [];
const modals = [];
for (const [family, def] of Object.entries(FAMILIES)) {
  const qa = qaCases(family, def);
  const cases = qa ?? def.smoke;
  const keys = Object.keys(def.defaults);
  const extraKeys = Object.keys(def.extra ?? {});
  const records = [];
  for (const c of cases) {
    const given0 = def.map ? def.map(c.props ?? {}) : c.props ?? {};
    const given = qa && qaFixture[family] === 'text' && def.textProp ? { [def.textProp]: 'Label', ...given0 } : given0;
    for (const k of Object.keys(given)) if (!(k in def.defaults)) (dropped[family] ??= new Set()).add(k);
    const props = { ...def.defaults, ...given };
    if (def.opens && def.opens(given)) opens.push(c.id);
    if (def.opensModal && def.opensModal(given)) modals.push(c.id);
    rows.push({ id: c.id, family, title: c.title ?? (Object.entries(c.props ?? {}).map(([k, v]) => `${k}=${JSON.stringify(v)}`).join(' ') || 'defaults') });
    const ex = { ...(def.extra ?? {}), ...Object.fromEntries(Object.entries(c.props ?? {}).filter(([k]) => k in (def.extra ?? {}))) };
    records.push(`${family}Case(${['caseId', ...keys, ...extraKeys].map(k => `${k}=${lit(k === 'caseId' ? c.id : k in ex && !(k in def.defaults) ? ex[k] : props[k])}`).join(', ')})`);
  }
  const acts = Object.entries(def.actions ?? {}).map(([k, a]) => `${k}=${a}`);
  const call = `${def.component}(${[...keys.map(k => `${k}=c.${k}`), ...acts].join(', ')})`;
  // QA's fixture decides the children when its file names one: three 40x24 boxes, the text
  // "Label" (a text prop, or a Text child), or none.
  let kids = def.children;
  if (qa && family in qaFixture) {
    const fx = qaFixture[family];
    kids = fx === 'boxes' ? ['FixtureBox()', 'FixtureBox()', 'FixtureBox()'] : fx === 'text' ? (def.textProp ? null : [TEXT_LABEL]) : null;
    if (def.keepChildren || def.noSlot) kids = def.noSlot ? null : def.children;
  }
  const inner = kids ? [call, ...kids.map(l => `  ${l}`)] : [call];
  const body = def.wrap ? def.wrap(inner) : inner;
  families.push({
    family, fromQa: !!qa, uses: def.uses ?? [def.component],
    decl: [
      `shape ${family}Case`, '  caseId: string', ...keys.map(k => `  ${k}: ${typeOf(def.defaults[k], k, def)}`), ...extraKeys.map(k => `  ${k}: ${typeOf(def.extra[k], k, def)}`), '',
      `fn ${family[0].toLowerCase() + family.slice(1)}Cases(): ${family}Case[] = [${records.join(', ')}]`, '',
      `component ${family}Cases`, '  props', '    id: string', '  state presses = 0', '  action noop', '    presses = presses + 1', '  action noopB(b: bool)', '    presses = presses + 1', '  action noopS(t: string)', '    presses = presses + 1', '  action noopSB(t: string, b: bool)', '    presses = presses + 1', '  action noopN(n: number)', '    presses = presses + 1', '  action noopSS(a: string, b: string)', '    presses = presses + 1', '  action noopSB2(t: string, b: bool)', '    presses = presses + 1', '  action noopSO(k: string, before: option<string>)', '    presses = presses + 1', '  view',
      '    column width="100%" align-items="flex-start"',
      `      for c in ${family[0].toLowerCase() + family.slice(1)}Cases().filter(c => c.caseId == id) key=c.caseId`,
      ...body.map(l => `        ${l}`),
    ],
  });
}

const uses = [...new Set(families.flatMap(f => f.uses))].sort();
const LAYERS = new Set(['Popover', 'Collapsible', 'CollapsibleGroup', 'Tooltip', 'InfoTip', 'HoverCard', 'Dialog', 'DropdownMenu', 'ContextMenu', 'BottomSheet', 'MenuItem', 'MenuRows', 'Resizable', 'Typeahead', 'Tokenizer', 'Selector', 'Calendar', 'DateInput', 'TimeInput', 'CommandPalette', 'Command', 'ChatMsg', 'ChatMessage', 'ChatComposer', 'ChatLayout', 'ChatMessageList', 'ChatToolCalls', 'PowerSearch', 'SearchField', 'SearchFilter', 'OverflowList']);
const libUses = [...new Set([...uses.filter(u => !LAYERS.has(u)), ...Object.values(FAMILIES).flatMap(d => d.libUses ?? [])])];
const layerUses = uses.filter(u => LAYERS.has(u));
const out = [
  '// The gallery\'s cases, written by tools/gallery-cases.mjs; do not edit.',
  ...(libUses.length ? [`use ${libUses.join(', ')} from "@astryx-spike/astryx"`] : []),
  ...(layerUses.length ? [`use ${layerUses.join(', ')} from "@astryx-spike/astryx-layers"`] : []),
  'use Swatches, Filler, FixtureBox from "./swatches.contract"',
  'use ControlsPlayground, CompositesPlayground, DataPlayground, ChatPlayground from "./playground.contract"',
  '',
  'shape CaseRow',
  '  id: string',
  '  family: string',
  '  title: string',
  '',
  `fn caseModal(id: string): bool = ${modals.length ? modals.map(i => `id == ${lit(i)}`).join(' or ') : 'false'}`,
  `fn caseOpens(id: string): bool = ${opens.length ? opens.map(i => `id == ${lit(i)}`).join(' or ') : 'false'}`,
  '',
  `fn caseRows(): CaseRow[] = [${rows.map(r => `CaseRow(id=${lit(r.id)}, family=${lit(r.family)}, title=${lit(r.title)})`).join(', ')}]`,
  '',
  'component CaseView',
  '  props',
  '    id: string',
  '  view',
  '    column width="100%" align-items="flex-start"',
];
for (const f of families) out.push(`      ${f.family}Cases(id=id)`);
for (const f of families) out.push('', ...f.decl);
writeFileSync(resolve(HERE, 'apps/astryx-gallery/cases.contract'), out.join('\n') + '\n');
writeFileSync(resolve(HERE, 'apps/astryx-gallery/cases-unsupported.json'), JSON.stringify(Object.fromEntries(Object.entries(dropped).map(([f, ks]) => [f, [...ks].sort()])), null, 1) + '\n');
console.log(`cases ${rows.length} in ${families.length} families (${families.filter(f => f.fromQa).map(f => f.family).join(', ')} from QA)`);
