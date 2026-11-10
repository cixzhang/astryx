// Captures the visual matrix for one family on one web host, one PNG per case,
// scheme and width, cropped to the rendered component. Also writes each case's
// accessibility subtree and timing.
//
//   node capture.mjs --host storybook --base http://127.0.0.1:6006 --family Button [--theme neutral]
//   node capture.mjs --host exact     --base http://127.0.0.1:4100 --family Button   (route ?case=<id>&scheme=<s>)
//   node capture.mjs --host ours-web  --base http://127.0.0.1:4200/gallery.html --family Button
// Output: shots/<host>/<Family>/<caseId>.<scheme>.<width>.png and shots/<host>/<Family>/index.json
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const opt = (n, d) => { const i = args.indexOf(n); return i < 0 ? d : args[i + 1]; };
const host = opt('--host'), base = opt('--base'), family = opt('--family');
const theme = opt('--theme', 'neutral');
const only = opt('--only'); // comma-separated case ids
const pw = opt('--playwright', process.env.PLAYWRIGHT_CORE ?? 'playwright-core');
const chrome = opt('--chrome', process.env.CHROME);
const out = resolve(opt('--out', join(here, 'shots')), host, family);
const doc = JSON.parse(readFileSync(join(here, 'cases', `${family}.json`), 'utf8'));
mkdirSync(out, { recursive: true });
const { chromium } = await import(pw);

// Storybook: the first story of each component whose id starts with core-<component>--.
let storyFor = {};
if (host === 'storybook') {
  const index = await (await fetch(`${base}/index.json`)).json();
  const entries = Object.values(index.entries).filter(e => e.type === 'story');
  for (const c of doc.components) {
    const slug = c.component.replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase();
    const flat = c.component.toLowerCase();
    const hit = entries.find(e => e.id.startsWith(`core-${slug}--`)) ?? entries.find(e => e.id.startsWith(`core-${flat}--`))
      ?? entries.find(e => e.id.replace(/-/g, '').startsWith(`core${flat}--`.replace(/-/g, '')));
    storyFor[c.component] = hit?.id ?? null;
  }
}
const argValue = v => typeof v === 'boolean' ? `!${v}` : typeof v === 'number' ? String(v) : String(v).replace(/[;:]/g, '');
const urlFor = (comp, c, scheme) => {
  if (host === 'storybook') {
    const a = Object.entries(c.props).map(([k, v]) => `${k}:${argValue(v)}`).join(';');
    return `${base}/iframe.html?id=${storyFor[comp]}&viewMode=story&args=${encodeURIComponent(a)}&globals=${encodeURIComponent(`astryxTheme:${theme};colorMode:${scheme}`)}`;
  }
  const sep = base.includes('?') ? '&' : '?';
  return `${base}${sep}case=${encodeURIComponent(c.id)}&scheme=${scheme}&theme=${theme}`;
};
// The element whose box is the case: Storybook's story content, or the lane's "case" box.
const caseSelector = host === 'storybook' ? '#storybook-root div[style*="padding"]' : '[data-testid="case"], #case, [aria-label="case"]';
// Crop: the union of the case box's children, grown by the 16 px case padding.
const cropOf = el => el.evaluate(e => {
  const rs = [...e.children].map(c => c.getBoundingClientRect()).filter(r => r.width > 0 && r.height > 0);
  const o = e.getBoundingClientRect();
  if (!rs.length) return { x: o.x, y: o.y, width: Math.max(1, o.width), height: Math.max(1, o.height) };
  // Not clipped to the case box: hosts differ in whether the box carries the padding.
  const W = document.documentElement.clientWidth;
  const x0 = Math.max(0, Math.min(...rs.map(r => r.left)) - 16), y0 = Math.max(0, Math.min(...rs.map(r => r.top)) - 16);
  const x1 = Math.min(W, Math.max(...rs.map(r => r.right)) + 16), y1 = Math.max(...rs.map(r => r.bottom)) + 16;
  return { x: Math.floor(x0), y: Math.floor(y0 + scrollY), width: Math.ceil(x1 - x0), height: Math.ceil(y1 - y0) };
});

const index = { host, family, theme, at: new Date().toISOString(), cases: [] };
const jobs = [];
for (const comp of doc.components) {
  if (host === 'storybook' && !storyFor[comp.component]) { index.cases.push({ component: comp.component, skipped: 'no story' }); continue; }
  for (const c of comp.cases) {
    if (only && !only.split(',').includes(c.id)) continue;
    for (const scheme of doc.schemes) for (const width of doc.widths) jobs.push({ comp, c, scheme, width });
  }
}
const ctxs = {};
const shoot = job => Promise.race([shootOnce(job), new Promise(res => setTimeout(() => { index.cases.push({ id: job.c.id, component: job.comp.component, scheme: job.scheme, width: job.width, error: 'timeout 90s' }); res(); }, 90000))]);
const shootOnce = async ({ comp, c, scheme, width }) => {
  const key = `${scheme}`;
  ctxs[key] ??= await browser.newContext({ colorScheme: scheme });
  const page = await ctxs[key].newPage(); await page.setViewportSize({ width, height: 900 });
  const errors = []; page.on('pageerror', e => errors.push(e.message.slice(0, 160)));
  const t0 = Date.now();
  let rec = { id: c.id, component: comp.component, scheme, width };
  try {
    await page.goto(urlFor(comp.component, c, scheme), { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(Number(opt('--settle', '250')));
    const el = await page.waitForSelector(caseSelector, { timeout: 15000 }).catch(() => null);
    if (!el) throw new Error('no case element');
    const box = await cropOf(el);
    const file = `${c.id}.${scheme}.${width}.png`;
    await page.screenshot({ path: join(out, file), clip: box, fullPage: true, animations: 'disabled', timeout: 20000 });
    const ax = await page.locator(caseSelector).first().ariaSnapshot({ timeout: 5000 }).catch(() => null);
    rec = { ...rec, file, box, ms: Date.now() - t0, ax, errors };
  } catch (e) { rec = { ...rec, error: e.message.slice(0, 200), errors }; }
  index.cases.push(rec); await page.close();
};
const browser = await chromium.launch({ executablePath: chrome, headless: true });
const N = Number(opt('--jobs', '12'));
let next = 0;
await Promise.all(Array.from({ length: N }, async () => { while (next < jobs.length) { const j = jobs[next++]; await shoot(j); } }));
// Retry failures with less concurrency: cold starts under load time out.
for (let pass = 0; pass < 2; pass++) {
  const failed = index.cases.filter(r => r.error && r.id);
  if (!failed.length) break;
  index.cases = index.cases.filter(r => !(r.error && r.id));
  const retry = failed.map(r => jobs.find(j => j.c.id === r.id && j.scheme === r.scheme && j.width === r.width)).filter(Boolean);
  let k = 0;
  await Promise.all(Array.from({ length: 4 }, async () => { while (k < retry.length) { const j = retry[k++]; await shoot(j); } }));
}
index.cases.sort((a, b) => (a.id ?? '').localeCompare(b.id ?? '') || (a.scheme ?? '').localeCompare(b.scheme ?? '') || (a.width ?? 0) - (b.width ?? 0));
{ const best = new Map(); for (const r of index.cases) { const k = `${r.id ?? r.component}.${r.scheme}.${r.width}`; const b = best.get(k); if (!b || (!b.file && r.file)) best.set(k, r); } index.cases = [...best.values()]; }
writeFileSync(join(out, 'index.json'), JSON.stringify(index, null, 1));
await browser.close();
const ok = index.cases.filter(r => r.file).length, bad = index.cases.filter(r => r.error || r.skipped).length;
console.log(`${host} ${family}: ${ok} shots, ${bad} failed/skipped -> ${out}`);
