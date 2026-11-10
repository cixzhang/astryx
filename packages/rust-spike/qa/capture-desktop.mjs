// Captures the visual matrix from a desktop (Linux) gallery binary of our stack: one run
// per case, scheme and size, under a virtual X display. The binary reads
// ASTRYX_CASE, ASTRYX_SCHEME, ASTRYX_SIZE, ASTRYX_SHOT and ASTRYX_A11Y, writes the PNG
// and the accessibility tree, prints first_frame_ms=<n>, and exits.
//
//   node capture-desktop.mjs --bin <gallery binary> --family Button [--host ours-linux] [--jobs 8]
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawn } from 'node:child_process';
const here = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const opt = (n, d) => { const i = args.indexOf(n); return i < 0 ? d : args[i + 1]; };
const bin = resolve(opt('--bin')), family = opt('--family'), host = opt('--host', 'ours-linux'), theme = opt('--theme', 'neutral');
const jobsN = Number(opt('--jobs', '8')), height = Number(opt('--height', '900'));
const out = resolve(opt('--out', join(here, 'shots')), host, family); mkdirSync(out, { recursive: true });
const doc = JSON.parse(readFileSync(join(here, 'cases', `${family}.json`), 'utf8'));
const run = env => new Promise(res => {
  const t0 = Date.now();
  const p = spawn('xvfb-run', ['-a', '-s', `-screen 0 ${env.W}x${height}x24`, bin], { env: { ...process.env, ...env.vars } });
  let so = '', se = ''; p.stdout.on('data', d => so += d); p.stderr.on('data', d => se += d);
  const kill = setTimeout(() => p.kill('SIGKILL'), 60000);
  p.on('close', code => { clearTimeout(kill); res({ code, ms: Date.now() - t0, so, se }); });
});
const jobs = [];
for (const comp of doc.components) for (const c of comp.cases) for (const scheme of doc.schemes) for (const width of doc.widths) jobs.push({ comp, c, scheme, width });
const index = { host, family, theme, at: new Date().toISOString(), cases: [] };
let next = 0;
await Promise.all(Array.from({ length: jobsN }, async () => { while (next < jobs.length) {
  const { comp, c, scheme, width } = jobs[next++];
  const file = `${c.id}.${scheme}.${width}.png`, a11y = `${c.id}.${scheme}.${width}.a11y.json`;
  const r = await run({ W: width, vars: { ASTRYX_CASE: c.id, ASTRYX_SCHEME: scheme, ASTRYX_THEME: theme, ASTRYX_SIZE: `${width}x${height}`, ASTRYX_SHOT: join(out, file), ASTRYX_A11Y: join(out, a11y) } });
  const ff = (r.so.match(/first_frame_ms=([\d.]+)/) ?? [])[1];
  const rec = { id: c.id, component: comp.component, scheme, width, ms: r.ms, firstFrameMs: ff ? Number(ff) : null };
  if (r.code === 0 && existsSync(join(out, file))) { rec.file = file; if (existsSync(join(out, a11y))) rec.a11yFile = a11y; }
  else rec.error = `exit ${r.code}: ${r.se.slice(-200)}`;
  index.cases.push(rec);
} }));
index.cases.sort((a, b) => a.id.localeCompare(b.id) || a.scheme.localeCompare(b.scheme) || a.width - b.width);
writeFileSync(join(out, 'index.json'), JSON.stringify(index, null, 1));
console.log(`${host} ${family}: ${index.cases.filter(r => r.file).length} shots, ${index.cases.filter(r => r.error).length} failed -> ${out}`);
