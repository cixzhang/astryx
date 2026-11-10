// Pixel-diffs one host's captures against the Storybook baseline for a family, in
// headless Chrome (no image libraries). A shot passes when at most 2% of its pixels
// differ by more than the channel tolerance; size differences count as differing
// pixels over the padded area.
//
//   node diff.mjs --host exact --family Button [--tolerance 8] [--diffs 20]
// Output: shots/<host>/<Family>/diff.json and diff-<file> images for the worst cases.
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const opt = (n, d) => { const i = args.indexOf(n); return i < 0 ? d : args[i + 1]; };
const host = opt('--host'), family = opt('--family'), baseHost = opt('--base-host', 'storybook');
const tol = Number(opt('--tolerance', '8')), keep = Number(opt('--diffs', '20')), passPct = Number(opt('--pass', '2'));
const shots = resolve(opt('--shots', join(here, 'shots')));
const pw = opt('--playwright', process.env.PLAYWRIGHT_CORE ?? 'playwright-core');
const { chromium } = await import(pw);
const A = join(shots, host, family), B = join(shots, baseHost, family);
const ai = JSON.parse(readFileSync(join(A, 'index.json'), 'utf8')), bi = JSON.parse(readFileSync(join(B, 'index.json'), 'utf8'));
const baseFiles = new Map(bi.cases.filter(r => r.file).map(r => [r.file, r]));
const browser = await chromium.launch({ executablePath: process.env.CHROME, headless: true });
const page = await browser.newPage();
await page.setContent('<html><body></body></html>');
const results = [];
for (const r of ai.cases.filter(r => r.file)) {
  const b = baseFiles.get(r.file);
  if (!b) { results.push({ file: r.file, missingBaseline: true }); continue; }
  const a64 = readFileSync(join(A, r.file)).toString('base64'), b64 = readFileSync(join(B, r.file)).toString('base64');
  const res = await page.evaluate(async ({ a64, b64, tol, wantImage }) => {
    const load = async s => createImageBitmap(await (await fetch('data:image/png;base64,' + s)).blob());
    const [ia, ib] = await Promise.all([load(a64), load(b64)]);
    const W = Math.max(ia.width, ib.width), H = Math.max(ia.height, ib.height);
    const px = img => { const c = new OffscreenCanvas(W, H), x = c.getContext('2d'); x.drawImage(img, 0, 0); return x.getImageData(0, 0, W, H).data; };
    const da = px(ia), db = px(ib), out = new Uint8ClampedArray(W * H * 4);
    let diff = 0;
    for (let i = 0; i < W * H; i++) {
      const o = i * 4, inA = (i % W) < ia.width && Math.floor(i / W) < ia.height, inB = (i % W) < ib.width && Math.floor(i / W) < ib.height;
      const d = !inA || !inB || Math.abs(da[o] - db[o]) > tol || Math.abs(da[o + 1] - db[o + 1]) > tol || Math.abs(da[o + 2] - db[o + 2]) > tol || Math.abs(da[o + 3] - db[o + 3]) > tol;
      if (d) { diff++; out[o] = 255; out[o + 3] = 255; } else { const g = (db[o] + db[o + 1] + db[o + 2]) / 3; out[o] = out[o + 1] = out[o + 2] = 200 + g / 5; out[o + 3] = 255; }
    }
    let png = null;
    if (wantImage) { const c = new OffscreenCanvas(W, H); c.getContext('2d').putImageData(new ImageData(out, W, H), 0, 0); const blob = await c.convertToBlob({ type: 'image/png' }); { const u = new Uint8Array(await blob.arrayBuffer()); let str = ''; for (let i = 0; i < u.length; i += 32768) str += String.fromCharCode.apply(null, u.subarray(i, i + 32768)); png = btoa(str); } }
    return { W, H, aSize: [ia.width, ia.height], bSize: [ib.width, ib.height], diff, pct: 100 * diff / (W * H), png };
  }, { a64, b64, tol, wantImage: true });
  results.push({ file: r.file, id: r.id, scheme: r.scheme, width: r.width, ...res, png: undefined, _png: res.png });
}
results.sort((x, y) => (y.pct ?? -1) - (x.pct ?? -1));
results.forEach((x, i) => { if (x._png && i < keep) writeFileSync(join(A, `diff-${x.file}`), Buffer.from(x._png, 'base64')); delete x._png; });
const compared = results.filter(x => x.pct !== undefined);
const summary = {
  host, family, baseline: baseHost, tolerance: tol, passPercent: passPct,
  baselineShots: baseFiles.size, hostShots: ai.cases.filter(r => r.file).length, compared: compared.length,
  missingBaseline: results.filter(x => x.missingBaseline).length,
  passing: compared.filter(x => x.pct <= passPct).length,
  parityPercent: baseFiles.size ? +(100 * compared.filter(x => x.pct <= passPct).length / baseFiles.size).toFixed(1) : 0,
  meanDiffPercent: compared.length ? +(compared.reduce((s, x) => s + x.pct, 0) / compared.length).toFixed(2) : null,
};
writeFileSync(join(A, 'diff.json'), JSON.stringify({ summary, results }, null, 1));
console.log(JSON.stringify(summary));
await browser.close();
