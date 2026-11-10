// Astryx themes → Tamber. Reads resolved theme JSON (tokens/<theme>.json, written by Exact's
// scripts/astryx-tokens.mjs) through `contract tamber import astryx`, then writes:
//   astryx/astryx.tamber      the package's tokens: Astryx's default theme
//   tokens/themes.tamber      the app-side `theme` axis: one `when theme=<name>` line per token a
//                             theme changes (an app pastes it into its app.tamber)
//   tokens/import-report.json what the importer mapped and what it reported unmapped, per theme
//
//   EXACT_SDK=<exact checkout> bun tools/astryx-tamber.mjs
import { spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync, readdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';

const HERE = resolve(dirname(new URL(import.meta.url).pathname), '..');
const SDK = resolve(process.env.EXACT_SDK ?? resolve(HERE, '.exact-sdk'));
const contract = resolve(SDK, 'target/debug/contract');
const THEMES = ['default', 'neutral', 'stone', 'butter', 'chocolate', 'gothic', 'matcha', 'y2k'];

function importTheme(name) {
  const r = spawnSync(contract, ['tamber', 'import', 'astryx', resolve(HERE, 'tokens', `${name}.json`), '--prefix', 'astryx'], { encoding: 'utf8', maxBuffer: 64 << 20 });
  const text = (r.stdout || '') + (r.stderr || '');
  const start = text.indexOf('{');
  if (start < 0) throw new Error(`import ${name}: no JSON (${text.slice(0, 300)})`);
  return JSON.parse(text.slice(start));
}

const runs = Object.fromEntries(THEMES.map(t => [t, importTheme(t)]));
const tokensOf = run => {
  const m = new Map();
  for (const op of run.ops) if (op.op === 'add-token') m.set(op.name, { kind: op.kind, value: op.value });
  return m;
};
const base = tokensOf(runs.default);

// A type token names its family: Astryx's type scale is size/line-height and weight; the family is
// the body or heading family token.
const typeFamily = name => (/heading|display|large/.test(name) ? 'astryx.family.heading' : name.endsWith('.code') ? 'astryx.family.code' : 'astryx.family.body');
const valueOf = (name, t) => (t.kind === 'type' ? `${t.value} ${typeFamily(name)}` : t.value);
const line = (name, t) => `  ${name}${t.kind ? `: ${t.kind}` : ''} = ${valueOf(name, t)}`;

const groups = new Map();
for (const [name, t] of base) {
  const g = name.split('.').slice(0, 2).join('.');
  if (!groups.has(g)) groups.set(g, []);
  groups.get(g).push(line(name, t));
}
const pkg = [
  '// Astryx tokens for Exact: the default theme, resolved from @astryxdesign/core and imported by',
  '// `contract tamber import astryx` (tools/astryx-tamber.mjs writes this file; do not edit it).',
  '// An app sets any token by its full name, or a whole theme through the `theme` axis in',
  '// tokens/themes.tamber.',
  'tamber astryx',
  '',
  'theme',
  ...[...groups.values()].flat(),
  '',
].join('\n');
writeFileSync(resolve(HERE, 'astryx', 'astryx.tamber'), pkg);

const axis = [
  '// The app side of Astryx theming: one axis whose values are Astryx themes. Each `when` line is a',
  '// token that theme sets to another value than the default theme. Written by',
  '// tools/astryx-tamber.mjs; an app copies this block into its app.tamber.',
  'axes',
  `  theme = ${THEMES.join(' | ')}`,
  '',
  'theme',
];
let overrides = 0;
const fixedMotion = [];
for (const [name, t] of base) {
  const whens = [];
  for (const th of THEMES.slice(1)) {
    const o = tokensOf(runs[th]).get(name);
    if (o && o.value !== t.value) whens.push(`    when theme=${th}: ${valueOf(name, o)}`);
  }
  if (whens.length && (t.kind === 'duration' || t.kind === 'easing')) {
    // Tamber writes a duration or easing into its transition at build: one value, no `when`.
    fixedMotion.push({ name, themes: whens.length });
  } else if (whens.length) {
    axis.push(line(name, t), ...whens);
    overrides += whens.length;
  }
}
writeFileSync(resolve(HERE, 'tokens', 'themes.tamber'), axis.join('\n') + '\n');

const report0 = Object.fromEntries(THEMES.map(t => [t, { counted: runs[t].counted, unmapped: runs[t].unmapped.map(u => ({ name: u.name, why: u.why })), offers: runs[t].offers.map(o => ({ role: o.role, token: o.token })) }]));
const report = { themes: report0, motionTokensFixedAcrossThemes: fixedMotion };
writeFileSync(resolve(HERE, 'tokens', 'import-report.json'), JSON.stringify(report, null, 1) + '\n');
console.log(`tokens ${base.size}, theme overrides ${overrides}, motion tokens held to the default ${fixedMotion.length}, unmapped ${runs.default.unmapped.length}`);
