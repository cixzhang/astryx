// STATUS.md from tools/status.json and tools/closure.json: each ledger row's status and the
// dependency closure ported %, counted in Astryx source files.
//
//   bun tools/status.mjs
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';

const HERE = resolve(dirname(new URL(import.meta.url).pathname), '..');
const closure = JSON.parse(readFileSync(resolve(HERE, 'tools/closure.json'), 'utf8'));
const status = JSON.parse(readFileSync(resolve(HERE, 'tools/status.json'), 'utf8'));
const fams = closure.families;
const core = Object.entries(fams).filter(([, f]) => !(f.groups.length === 1 && f.groups[0] === 'stretch'));
const sum = (list, pred) => list.filter(pred).reduce((n, [, f]) => n + f.files, 0);
const done = ([k]) => ['ported', 'workaround'].includes(status[k]?.status);
const coreFiles = sum(core, () => true), allFiles = sum(Object.entries(fams), () => true);
const pct = (a, b) => `${((100 * a) / b).toFixed(1)}%`;
const rows = Object.entries(fams).map(([k, f]) => {
  const s = status[k] ?? { status: 'not started', wave: '', evidence: '', notes: '' };
  return `| ${k} | ${f.files} | ${f.groups.join(', ')} | ${s.wave} | ${s.status} | ${s.evidence} | ${s.notes} |`;
});
const ceilings = JSON.parse(readFileSync(resolve(HERE, 'tools/ceilings.json'), 'utf8'));
const count = st => Object.keys(fams).filter(k => (status[k]?.status ?? 'not started') === st).length;
const md = [
  '# Exact lane status',
  '',
  `Astryx source: \`${closure.astryx}\`. Exact: \`94b8671f70fe2e25b307a40e7fce66dc27c9f802\`.`,
  '',
  `- Dependency closure ported (core, by source files): ${sum(core, done)} of ${coreFiles} (${pct(sum(core, done), coreFiles)}).`,
  `- With the stretch rows: ${sum(Object.entries(fams), done)} of ${allFiles} (${pct(sum(Object.entries(fams), done), allFiles)}).`,
  `- Rows: ${count('ported')} ported, ${count('workaround')} workaround, ${count('blocked')} blocked, ${count('not started')} not started.`,
  '',
  'A row counts as ported when its Contract port gives the Astryx behavior and look its cases test,',
  'workaround when it needs a different shape or an escape hatch (the note says which), and blocked',
  'when Contract cannot express it (the note gives the smallest failing snippet).',
  '',
  'Tests: `apps/astryx-gallery` (cases for every family, four interactive playgrounds, 23 tests),',
  '`apps/astryx-shell-nav` (6 tests) and `apps/astryx-settings-dialog` (5 tests) pass on the web host;',
  'all three build and respond on the Linux host. `gaps.test.contract` keeps the known failing test.',
  '',
  '## Ceilings found',
  '',
  '| Area | Finding | Evidence |',
  '| --- | --- | --- |',
  ...ceilings.map(c => `| ${c.area} | ${c.finding} | ${c.evidence} |`),
  '',
  '## Ledger rows',
  '',
  '| Row | Files | Groups | Wave | Status | Evidence | Notes |',
  '| --- | --- | --- | --- | --- | --- | --- |',
  ...rows,
  '',
].join('\n');
writeFileSync(resolve(HERE, 'STATUS.md'), md);
console.log(md.split('\n').slice(4, 7).join('\n'));
