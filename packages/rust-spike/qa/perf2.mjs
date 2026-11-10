// Web performance per host on the same pages: first frame, transfer size and frame time.
//   node perf2.mjs > perf2.json
// First frame: the "qa:first-frame" mark when the page sets one, else first-contentful-paint.
// Transfer: every response the page loads, raw and as gzip -9 / brotli q11 would send it (the
// local servers do not compress, so production transfer is computed from the bodies).
// Frame time: requestAnimationFrame deltas while the switch is toggled by keyboard for 3 s, with
// vsync and the frame-rate limit off so a frame's length is its work; plus main-thread task time
// per toggle (CDP Performance TaskDuration).
import { gzipSync, brotliCompressSync, constants } from 'node:zlib';
const { chromium } = await import(process.env.PLAYWRIGHT_CORE ?? 'playwright-core');
const ours = process.env.OURS_PORT ?? 4201;
const hosts = {
  baseline: { gallery: 'http://127.0.0.1:6100/index.html?case=button-0001&scheme=light', interact: 'http://127.0.0.1:6006/iframe.html?id=core-switch--default&viewMode=story&globals=astryxTheme:neutral;colorMode:light' },
  exact: { gallery: 'http://127.0.0.1:4100/?case=button-0001&scheme=light&theme=neutral', interact: 'http://127.0.0.1:4100/?case=playground-0001&scheme=light&theme=neutral' },
  'ours-web': { gallery: `http://127.0.0.1:${ours}/gallery.html?case=button-0001&scheme=light`, interact: `http://127.0.0.1:${ours}/gallery.html?case=playground-0001&scheme=light` },
};
const browser = await chromium.launch({ executablePath: process.env.CHROME, headless: true, args: ['--disable-gpu-vsync', '--disable-frame-rate-limit'] });
const p = (xs, q) => { const s = [...xs].sort((a, b) => a - b); return s.length ? Math.round(s[Math.min(s.length - 1, Math.floor(s.length * q))] * 10) / 10 : null; };
const out = {};
for (const [host, urls] of Object.entries(hosts)) {
  const firsts = []; let transfer = null;
  for (let r = 0; r < 5; r++) {
    const ctx = await browser.newContext({ viewport: { width: 1280, height: 900 } }); const page = await ctx.newPage();
    const bodies = [];
    if (r === 0) page.on('response', async res => { try { const b = await res.body(); bodies.push({ url: res.url(), type: res.headers()['content-type'] ?? '', b }); } catch {} });
    await page.goto(urls.gallery, { waitUntil: 'networkidle', timeout: 60000 }); await page.waitForTimeout(800);
    firsts.push(await page.evaluate(() => { const m = performance.getEntriesByName('qa:first-frame')[0]; const f = performance.getEntriesByName('first-contentful-paint')[0]; return (m ?? f)?.startTime ?? null; }));
    if (r === 0) {
      const kb = n => Math.round(n / 102.4) / 10;
      const raw = bodies.reduce((a, x) => a + x.b.length, 0);
      const gz = bodies.reduce((a, x) => a + gzipSync(x.b, { level: 9 }).length, 0);
      const br = bodies.reduce((a, x) => a + brotliCompressSync(x.b, { params: { [constants.BROTLI_PARAM_QUALITY]: 11 } }).length, 0);
      transfer = { requests: bodies.length, rawKB: kb(raw), gzipKB: kb(gz), brotliKB: kb(br), largest: bodies.sort((a, b) => b.b.length - a.b.length).slice(0, 3).map(x => `${x.url.split('/').pop().split('?')[0]} ${kb(x.b.length)} KB`) };
    }
    await ctx.close();
  }
  // Frame time under interaction.
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 900 } }); const page = await ctx.newPage();
  await page.goto(urls.interact, { waitUntil: 'networkidle', timeout: 60000 }); await page.waitForTimeout(1200);
  for (let i = 0; i < 25; i++) { await page.keyboard.press('Tab'); const role = await page.evaluate(() => document.activeElement?.getAttribute('role') ?? document.activeElement?.type); if (role === 'switch' || role === 'checkbox') break; }
  const cdp = await ctx.newCDPSession(page); await cdp.send('Performance.enable');
  const task = async () => (await cdp.send('Performance.getMetrics')).metrics.find(m => m.name === 'TaskDuration').value;
  const task0 = await task(); let toggles = 0;
  await page.evaluate(() => { window.__f = []; let last = performance.now(); const tick = t => { window.__f.push(t - last); last = t; if (window.__f.length < 100000) requestAnimationFrame(tick); }; requestAnimationFrame(tick); });
  const t0 = Date.now(); while (Date.now() - t0 < 3000) { await page.keyboard.press('Space'); toggles++; await page.waitForTimeout(120); }
  const taskMs = (await task() - task0) * 1000;
  const frames = (await page.evaluate(() => window.__f)).slice(2);
  await ctx.close();
  out[host] = { firstFrameMs: { p50: p(firsts.filter(x => x != null), 0.5), runs: firsts.map(x => x && Math.round(x)) }, transfer, frameMs: { p50: p(frames, 0.5), p95: p(frames, 0.95), frames: frames.length }, mainThreadMsPerToggle: Math.round(taskMs / toggles * 10) / 10 };
}
await browser.close();
console.log(JSON.stringify(out, null, 1));
