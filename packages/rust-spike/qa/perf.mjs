// Web performance for one page per host: first frame, transfer size (encoded bytes),
// and frame times while a scripted interaction runs (pointer drag or wheel scroll).
//
//   node perf.mjs --host storybook --url '<url>' [--drag '<css selector>'] [--scroll '<css selector>'] [--runs 5]
// First frame: performance mark "qa:first-frame" when the page sets one, else first-contentful-paint.
// Appends one record to perf/<host>.json.
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const here = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const opt = (n, d) => { const i = args.indexOf(n); return i < 0 ? d : args[i + 1]; };
const host = opt('--host'), url = opt('--url'), label = opt('--label', url), drag = opt('--drag'), scroll = opt('--scroll');
const runs = Number(opt('--runs', '5'));
const { chromium } = await import(process.env.PLAYWRIGHT_CORE ?? 'playwright-core');
const browser = await chromium.launch({ executablePath: process.env.CHROME, headless: true, args: ['--enable-gpu-rasterization'] });
const p50 = xs => { const s = [...xs].sort((a, b) => a - b); return s[Math.floor(s.length / 2)]; };
const p95 = xs => { const s = [...xs].sort((a, b) => a - b); return s[Math.min(s.length - 1, Math.floor(s.length * 0.95))]; };
const firsts = [], transfers = [], frames = [];
for (let r = 0; r < runs; r++) {
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 900 } }); const page = await ctx.newPage();
  const cdp = await ctx.newCDPSession(page); await cdp.send('Network.enable'); await cdp.send('Network.setCacheDisabled', { cacheDisabled: true });
  let bytes = 0; cdp.on('Network.loadingFinished', e => { bytes += e.encodedDataLength; });
  await page.goto(url, { waitUntil: 'networkidle', timeout: 60000 });
  await page.waitForTimeout(500);
  const first = await page.evaluate(() => {
    const m = performance.getEntriesByName('qa:first-frame')[0]; if (m) return m.startTime;
    const p = performance.getEntriesByName('first-contentful-paint')[0]; return p ? p.startTime : null;
  });
  firsts.push(first); transfers.push(bytes);
  if (drag || scroll) {
    await page.evaluate(() => { window.__qaFrames = []; let last = performance.now(); const tick = t => { window.__qaFrames.push(t - last); last = t; if (window.__qaFrames.length < 600) requestAnimationFrame(tick); }; requestAnimationFrame(tick); });
    if (drag) { const b = await page.locator(drag).first().boundingBox(); if (b) { await page.mouse.move(b.x + 4, b.y + b.height / 2); await page.mouse.down(); for (let i = 0; i < 60; i++) await page.mouse.move(b.x + 4 + i, b.y + b.height / 2); await page.mouse.up(); } }
    if (scroll) { await page.locator(scroll).first().hover(); for (let i = 0; i < 60; i++) await page.mouse.wheel(0, 120); }
    await page.waitForTimeout(300);
    const f = await page.evaluate(() => window.__qaFrames.slice(1)); frames.push(...f);
  }
  await ctx.close();
}
await browser.close();
const rec = { host, label, url, at: new Date().toISOString(), runs, firstFrameMsP50: p50(firsts.filter(x => x != null)), transferKB: +(p50(transfers) / 1024).toFixed(1), frameMsP95: frames.length ? +p95(frames).toFixed(1) : null, frameMsP50: frames.length ? +p50(frames).toFixed(1) : null, frames: frames.length, note: 'headless Chrome on a Linux devserver, no GPU: frame times are an upper bound' };
mkdirSync(join(here, 'perf'), { recursive: true });
const f = join(here, 'perf', `${host}.json`); const all = existsSync(f) ? JSON.parse(readFileSync(f, 'utf8')) : [];
all.push(rec); writeFileSync(f, JSON.stringify(all, null, 1));
console.log(JSON.stringify(rec));
