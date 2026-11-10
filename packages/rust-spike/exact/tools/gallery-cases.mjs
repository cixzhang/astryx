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
function qaCases(family, def) {
  const file = resolve(QA, `${def.qa?.file ?? family}.json`);
  if (!existsSync(file)) return null;
  const doc = JSON.parse(readFileSync(file, 'utf8'));
  const names = def.qa?.components ?? [def.component, family];
  const picked = doc.components.filter(c => names.includes(c.component));
  if (!picked.length) return null;
  return picked.flatMap(c => c.cases.map(k => ({ id: k.id, title: `${c.component} ${k.kind} ${JSON.stringify(k.props)}`, props: { ...(c.base ?? {}), ...k.props } })));
}

const typeOf = (v, k, def) => def.types?.[k] ?? (typeof v === 'string' ? 'string' : typeof v === 'number' ? 'number' : typeof v === 'boolean' ? 'bool' : null);
const rows = [];
const families = [];
const dropped = {};
const opens = [];
for (const [family, def] of Object.entries(FAMILIES)) {
  const qa = qaCases(family, def);
  const cases = qa ?? def.smoke;
  const keys = Object.keys(def.defaults);
  const records = [];
  for (const c of cases) {
    const given = def.map ? def.map(c.props ?? {}) : c.props ?? {};
    for (const k of Object.keys(given)) if (!(k in def.defaults)) (dropped[family] ??= new Set()).add(k);
    const props = { ...def.defaults, ...given };
    if (def.opens && def.opens(given)) opens.push(c.id);
    rows.push({ id: c.id, family, title: c.title ?? (Object.entries(c.props ?? {}).map(([k, v]) => `${k}=${JSON.stringify(v)}`).join(' ') || 'defaults') });
    records.push(`${family}Case(${['caseId', ...keys].map(k => `${k}=${lit(k === 'caseId' ? c.id : props[k])}`).join(', ')})`);
  }
  const call = `${def.component}(${keys.map(k => `${k}=c.${k}`).join(', ')})`;
  const inner = def.children ? [call, ...def.children.map(l => `  ${l}`)] : [call];
  const body = def.wrap ? def.wrap(inner) : inner;
  families.push({
    family, fromQa: !!qa, uses: def.uses ?? [def.component],
    decl: [
      `shape ${family}Case`, '  caseId: string', ...keys.map(k => `  ${k}: ${typeOf(def.defaults[k], k, def)}`), '',
      `fn ${family[0].toLowerCase() + family.slice(1)}Cases(): ${family}Case[] = [${records.join(', ')}]`, '',
      `component ${family}Cases`, '  props', '    id: string', '  state presses = 0', '  action noop', '    presses = presses + 1', '  view',
      `    for c in ${family[0].toLowerCase() + family.slice(1)}Cases().filter(c => c.caseId == id) key=c.caseId`,
      ...body.map(l => `      ${l}`),
    ],
  });
}

const uses = [...new Set(families.flatMap(f => f.uses))].sort();
const LAYERS = new Set(['Popover', 'Collapsible', 'CollapsibleGroup', 'Tooltip', 'InfoTip', 'HoverCard', 'Dialog', 'DropdownMenu', 'ContextMenu', 'BottomSheet']);
const libUses = uses.filter(u => !LAYERS.has(u));
const layerUses = uses.filter(u => LAYERS.has(u));
const out = [
  '// The gallery\'s cases, written by tools/gallery-cases.mjs; do not edit.',
  ...(libUses.length ? [`use ${libUses.join(', ')} from "@astryx-spike/astryx"`] : []),
  ...(layerUses.length ? [`use ${layerUses.join(', ')} from "@astryx-spike/astryx-layers"`] : []),
  'use Swatches, Filler from "./swatches.contract"',
  '',
  'shape CaseRow',
  '  id: string',
  '  family: string',
  '  title: string',
  '',
  `fn caseOpens(id: string): bool = ${opens.length ? opens.map(i => `id == ${lit(i)}`).join(' or ') : 'false'}`,
  '',
  `fn caseRows(): CaseRow[] = [${rows.map(r => `CaseRow(id=${lit(r.id)}, family=${lit(r.family)}, title=${lit(r.title)})`).join(', ')}]`,
  '',
  'component CaseView',
  '  props',
  '    id: string',
  '  view',
  '    column',
];
for (const f of families) out.push(`      ${f.family}Cases(id=id)`);
for (const f of families) out.push('', ...f.decl);
writeFileSync(resolve(HERE, 'apps/astryx-gallery/cases.contract'), out.join('\n') + '\n');
writeFileSync(resolve(HERE, 'apps/astryx-gallery/cases-unsupported.json'), JSON.stringify(Object.fromEntries(Object.entries(dropped).map(([f, ks]) => [f, [...ks].sort()])), null, 1) + '\n');
console.log(`cases ${rows.length} in ${families.length} families (${families.filter(f => f.fromQa).map(f => f.family).join(', ')} from QA)`);
