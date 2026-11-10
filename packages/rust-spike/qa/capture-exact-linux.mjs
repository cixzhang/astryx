// Captures the visual matrix from the Exact gallery's Linux host through Exact's agent
// driver: one drive per case, scheme and width. Crops the window shot to the frame of
// the node with testId "case".
//
//   node capture-exact-linux.mjs --exact <exact checkout> --app <gallery dir> --bin <linux binary> --family Button [--jobs 16]
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawn } from 'node:child_process';
const here = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const opt = (n, d) => { const i = args.indexOf(n); return i < 0 ? d : args[i + 1]; };
const exact = resolve(opt('--exact')), app = resolve(opt('--app')), bin = resolve(opt('--bin')), family = opt('--family');
const host = opt('--host', 'exact-linux'), theme = opt('--theme', 'neutral'), jobsN = Number(opt('--jobs', '16')), height = 900;
const out = resolve(opt('--out', join(here, 'shots')), host, family); mkdirSync(out, { recursive: true });
const doc = JSON.parse(readFileSync(join(here, 'cases', `${family}.json`), 'utf8'));
const drive = (c, scheme, width, file) => new Promise(res => {
  const t0 = Date.now();
  const p = spawn('bun', ['scripts/agent.mjs', 'linux', '--url', `/?case=${c.id}&scheme=${scheme}&theme=${theme}`, '--size', `${width}x${height}`, '--storage', `qa-${process.pid}-${c.id}-${scheme}-${width}`,
    'clock data', 'clock +1', 'clock settle', 'layout case', `screenshot ${join(out, file)}`], { cwd: exact, env: { ...process.env, EXACT_APP_DIR: app, EXACT_LINUX_BIN: bin } });
  let so = '', se = ''; p.stdout.on('data', d => so += d); p.stderr.on('data', d => se += d);
  const kill = setTimeout(() => p.kill('SIGKILL'), 90000);
  p.on('close', code => { clearTimeout(kill); res({ code, ms: Date.now() - t0, so, se }); });
});
const jobs = [];
for (const comp of doc.components) for (const c of comp.cases) for (const scheme of doc.schemes) for (const width of doc.widths) jobs.push({ comp, c, scheme, width });
const index = { host, family, theme, at: new Date().toISOString(), cases: [] };
let next = 0;
await Promise.all(Array.from({ length: jobsN }, async () => { while (next < jobs.length) {
  const { comp, c, scheme, width } = jobs[next++];
  const file = `${c.id}.${scheme}.${width}.png`;
  const r = await drive(c, scheme, width, file);
  const m = r.so.match(/space viewport ([\d.]+),([\d.]+) ([\d.]+)×([\d.]+)/);
  const rec = { id: c.id, component: comp.component, scheme, width, ms: r.ms };
  if (r.code === 0 && existsSync(join(out, file)) && m) { rec.file = file; rec.box = { x: +m[1], y: +m[2], width: +m[3], height: +m[4] }; }
  else rec.error = `exit ${r.code}: ${(r.se + r.so).split('\n').filter(l => /error|refus|no view|op \d/i.test(l)).slice(-2).join(' | ').slice(0, 240)}`;
  index.cases.push(rec);
} }));
const { chromium } = await import(process.env.PLAYWRIGHT_CORE ?? 'playwright-core');
const browser = await chromium.launch({ executablePath: process.env.CHROME, headless: true }); const page = await browser.newPage();
for (const r of index.cases.filter(r => r.file)) {
  const b64 = await page.evaluate(async ({ png, b }) => {
    const img = await createImageBitmap(await (await fetch('data:image/png;base64,' + png)).blob());
    const c = new OffscreenCanvas(Math.max(1, Math.round(b.width)), Math.max(1, Math.round(b.height)));
    c.getContext('2d').drawImage(img, Math.round(b.x), Math.round(b.y), c.width, c.height, 0, 0, c.width, c.height);
    const u = new Uint8Array(await (await c.convertToBlob({ type: 'image/png' })).arrayBuffer()); let s = ''; for (let i = 0; i < u.length; i += 32768) s += String.fromCharCode.apply(null, u.subarray(i, i + 32768)); return btoa(s);
  }, { png: readFileSync(join(out, r.file)).toString('base64'), b: r.box });
  writeFileSync(join(out, r.file), Buffer.from(b64, 'base64'));
}
await browser.close();
index.cases.sort((a, b) => a.id.localeCompare(b.id) || a.scheme.localeCompare(b.scheme) || a.width - b.width);
writeFileSync(join(out, 'index.json'), JSON.stringify(index, null, 1));
console.log(`${host} ${family}: ${index.cases.filter(r => r.file).length} shots, ${index.cases.filter(r => r.error).length} failed -> ${out}`);
