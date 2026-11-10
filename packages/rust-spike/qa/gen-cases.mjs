// Generates the visual-matrix cases for one or more Astryx component families from
// their `{Name}.doc.mjs` files: one case per value of each visual prop (single-prop
// sweep, other props at their defaults) plus a pairwise covering set over all visual
// props. Case ids are stable for a given doc.
//
//   node gen-cases.mjs --astryx <astryx checkout> --out cases Button Switch TextInput
import { mkdirSync, writeFileSync, existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const args = process.argv.slice(2);
const opt = name => { const i = args.indexOf(name); if (i < 0) return undefined; const v = args[i + 1]; args.splice(i, 2); return v; };
const astryx = resolve(opt('--astryx') ?? '.');
const out = resolve(opt('--out') ?? 'cases');
const families = args;

// Props that change behavior or wiring but not pixels at rest.
const NON_VISUAL = /^(on[A-Z]|aria|data|id$|name$|type$|form|autoFocus|autoComplete|inputMode|enterKeyHint|tabIndex|ref$|key$|htmlFor|href$|target$|rel$|xstyle$|className$|style$|testId$|value$|defaultValue$|defaultChecked$|checked$|isChecked$|children$)/;
// Sample values for required free-form props.
const SAMPLE = { label: 'Label', placeholder: 'Placeholder', description: 'Description', title: 'Title', helperText: 'Helper text', errorMessage: 'Error message' };

const literals = type => {
  const t = String(type).trim();
  if (t === 'boolean') return [false, true];
  const parts = t.split('|').map(s => s.trim()).filter(p => p !== 'undefined');
  if (parts.length > 1 && parts.every(p => /^'[^']*'$/.test(p))) return parts.map(p => p.slice(1, -1));
  if (parts.length > 1 && parts.every(p => /^-?\d+(\.\d+)?$/.test(p))) return parts.map(Number);
  if (parts.length === 1 && parts[0] === 'boolean') return [false, true];
  return null;
};
const parseDefault = (d, values) => {
  if (d === undefined) return values[0];
  const s = String(d).replace(/^'|'$/g, '');
  if (s === 'true') return true; if (s === 'false') return false;
  if (values.includes(s)) return s;
  if (values.includes(Number(s))) return Number(s);
  return null; // unset: the component's own default
};

// Greedy pairwise covering array (AETG-style, deterministic).
function pairwise(params) {
  const names = Object.keys(params);
  if (names.length < 2) return [];
  const uncovered = new Set();
  for (let i = 0; i < names.length; i++) for (let j = i + 1; j < names.length; j++)
    for (const a of params[names[i]]) for (const b of params[names[j]]) uncovered.add(JSON.stringify([i, a, j, b]));
  const rows = [];
  while (uncovered.size) {
    let best = null, bestGain = -1;
    for (const seed of uncovered) {
      const [i, a, j, b] = JSON.parse(seed);
      const row = names.map(() => undefined); row[i] = a; row[j] = b;
      for (let k = 0; k < names.length; k++) {
        if (row[k] !== undefined) continue;
        let pick = params[names[k]][0], pickGain = -1;
        for (const v of params[names[k]]) {
          let g = 0;
          for (let m = 0; m < names.length; m++) if (row[m] !== undefined && m !== k) {
            const [x, y] = m < k ? [m, k] : [k, m], key = JSON.stringify(m < k ? [x, row[m], y, v] : [x, v, y, row[m]]);
            if (uncovered.has(key)) g++;
          }
          if (g > pickGain) { pick = v; pickGain = g; }
        }
        row[k] = pick;
      }
      let gain = 0;
      for (let x = 0; x < names.length; x++) for (let y = x + 1; y < names.length; y++) if (uncovered.has(JSON.stringify([x, row[x], y, row[y]]))) gain++;
      if (gain > bestGain) { best = row; bestGain = gain; }
      break; // first uncovered pair seeds the row: deterministic and fast enough
    }
    for (let x = 0; x < names.length; x++) for (let y = x + 1; y < names.length; y++) uncovered.delete(JSON.stringify([x, best[x], y, best[y]]));
    rows.push(Object.fromEntries(names.map((n, k) => [n, best[k]])));
  }
  return rows;
}

mkdirSync(out, { recursive: true });
const summary = {};
for (const arg of families) {
  // "Dir/Name" reads packages/core/src/Dir/Name.doc.mjs (a doc beside another component's).
  const [dir, family] = arg.includes('/') ? arg.split('/') : [arg, arg];
  const docPath = join(astryx, 'packages/core/src', dir, `${family}.doc.mjs`);
  if (!existsSync(docPath)) { console.error(`skip ${family}: no ${docPath}`); continue; }
  const { docs } = await import(pathToFileURL(docPath).href);
  const comps = (docs.components?.length ? docs.components : [{ name: family, props: [] }]).map((c, i) =>
    (i === 0 && !(c.props ?? []).length && (docs.props ?? []).length) ? { ...c, props: docs.props } : c);
  const docOut = { family, source: `packages/core/src/${dir}/${family}.doc.mjs`, schemes: ['light', 'dark'], widths: [390, 1280], components: [] };
  for (const comp of comps) {
  const base = {}, visual = {}, defaults = {};
  for (const p of comp.props ?? []) {
    const values = literals(p.type);
    if (values && !NON_VISUAL.test(p.name)) { visual[p.name] = values; defaults[p.name] = parseDefault(p.default, values); }
    else if (p.required && SAMPLE[p.name]) base[p.name] = SAMPLE[p.name];
    else if (p.required && /string/.test(String(p.type))) base[p.name] = p.name;
  }
  const seen = new Set(), cases = [];
  const add = (kind, props) => {
    const full = { ...base, ...props };
    const key = JSON.stringify(Object.keys(full).sort().map(k => [k, full[k]]));
    if (seen.has(key)) return; seen.add(key);
    cases.push({ id: `${comp.name.toLowerCase()}-${String(cases.length + 1).padStart(4, '0')}`, kind, props: full });
  };
  add('default', {});
  for (const [name, values] of Object.entries(visual)) for (const v of values) if (v !== defaults[name]) add('sweep', { [name]: v });
  for (const row of pairwise(visual)) {
    const diff = Object.fromEntries(Object.entries(row).filter(([k, v]) => v !== defaults[k]));
    add('pairwise', diff);
  }
  docOut.components.push({ component: comp.name, visualProps: visual, defaults, base, takesChildren: (comp.props ?? []).some(p => p.name === 'children'), cases });
  }
  const n = docOut.components.reduce((a, c) => a + c.cases.length, 0);
  writeFileSync(join(out, `${family}.json`), JSON.stringify(docOut, null, 1) + '\n');
  summary[family] = { components: docOut.components.length, cases: n, screenshots: n * 2 * 2 };
}
console.log(JSON.stringify(summary, null, 1));
