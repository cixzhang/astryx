// Builds the 12-metric scorecard (HTML) from the harness outputs:
//   shots/<host>/<Family>/diff.json   visual parity (metric 1)
//   shots/<host>/<Family>/index.json  a11y snapshots vs the baseline (metric 3)
//   perf/<host>.json                  first frame, frame time, transfer (metrics 5, 6)
//   reports.json                      lane-reported values: behavior, text input, authoring,
//                                     theming, agent loop, ceiling tags, closure, Table API fit
//   node scorecard.mjs > scorecard.html
import { readFileSync, existsSync, readdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const here = dirname(fileURLToPath(import.meta.url));
const J = p => existsSync(p) ? JSON.parse(readFileSync(p, 'utf8')) : null;
const hosts = { exact: 'Exact web', 'exact-linux': 'Exact Linux', 'ours-web': 'Ours web', 'ours-linux': 'Ours Linux' };
const families = readdirSync(join(here, 'cases')).filter(f => f.endsWith('.json')).map(f => f.slice(0, -5)).sort();
const reports = J(join(here, 'reports.json')) ?? {};
const esc = s => String(s ?? '').replace(/[&<>]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' }[c]));
const norm = s => (s ?? '').replace(/"[^"]*"/g, m => m.toLowerCase()).replace(/\s+/g, ' ').trim();

const base = Object.fromEntries(families.map(f => [f, J(join(here, 'shots', 'storybook', f, 'index.json'))]));
const cell = v => v == null ? '<td class="na">—</td>' : `<td>${esc(v)}</td>`;
const visual = (h, f) => { const d = J(join(here, 'shots', h, f, 'diff.json')); return d ? `${d.summary.parityPercent}% (${d.summary.passing}/${d.summary.baselineShots})` : null; };
const a11y = (h, f) => {
  const a = J(join(here, 'shots', h, f, 'index.json')), b = base[f]; if (!a || !b) return null;
  const bm = new Map(b.cases.filter(r => r.ax).map(r => [`${r.id}.${r.scheme}.${r.width}`, norm(r.ax)]));
  let n = 0, ok = 0; for (const r of a.cases) { const k = `${r.id}.${r.scheme}.${r.width}`; if (!bm.has(k)) continue; n++; if (r.ax && norm(r.ax) === bm.get(k)) ok++; }
  return n ? `${(100 * ok / n).toFixed(1)}% (${ok}/${n})` : null;
};
const perf = h => { const p = J(join(here, 'perf', `${h}.json`)); return p?.length ? p[p.length - 1] : null; };
const rep = (metric, node, h) => reports?.[metric]?.[node]?.[h] ?? null;

const rows = [];
for (const f of families) rows.push(['1 Visual parity', f, Object.keys(hosts).map(h => visual(h, f))]);
for (const f of families) rows.push(['3 Accessibility (tree match)', f, Object.keys(hosts).map(h => a11y(h, f))]);
for (const [m, key] of [['2 Behavior parity', '2'], ['4 Text input', '4'], ['7 Authoring cost', '7'], ['8 Theming', '8'], ['9 Agent loop', '9'], ['10 Ceiling tags', '10'], ['12 Table API fit', '12']])
  for (const node of Object.keys(reports[key] ?? { '(not reported yet)': {} })) rows.push([m, node, Object.keys(hosts).map(h => rep(key, node, h))]);
rows.push(['5 First frame p50 (ms)', 'gallery', Object.keys(hosts).map(h => perf(h)?.firstFrameMsP50 ?? null)]);
rows.push(['5 Frame time p95 (ms)', 'interaction', Object.keys(hosts).map(h => perf(h)?.frameMsP95 ?? null)]);
rows.push(['6 Web transfer (KB, encoded)', 'gallery', Object.keys(hosts).map(h => perf(h)?.transferKB ?? null)]);
for (const node of Object.keys(reports['6'] ?? {})) rows.push(['6 Size', node, Object.keys(hosts).map(h => rep('6', node, h))]);
for (const node of Object.keys(reports['11'] ?? { 'locked closure 688 files / 67 families': {} })) rows.push(['11 Closure ported', node, Object.keys(hosts).map(h => rep('11', node, h))]);

const sb = perf('storybook');
const baseShots = families.reduce((s, f) => s + (base[f]?.cases.filter(r => r.file).length ?? 0), 0);
const totalCases = families.reduce((s, f) => s + J(join(here, 'cases', `${f}.json`)).components.reduce((a, c) => a + c.cases.length, 0), 0);
console.log(`<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Rust spike scorecard</title>
<style>:root{color-scheme:light dark;--fg:light-dark(#1c1e21,#e4e6eb);--mut:light-dark(#5d6168,#a8adb5);--bg:light-dark(#fff,#0e0f11);--card:light-dark(#f5f6f8,#1a1c1f);--line:light-dark(#dcdfe4,#30343a)}
body{font:14px/1.45 system-ui,sans-serif;color:var(--fg);background:var(--bg);margin:0}main{max-width:1180px;margin:0 auto;padding:20px}
h1{font-size:22px;margin:0 0 4px}.sub{color:var(--mut)}table{border-collapse:collapse;width:100%;margin-top:14px}th,td{border-bottom:1px solid var(--line);padding:6px 8px;text-align:left;vertical-align:top;overflow-wrap:anywhere}
th{background:var(--card);position:sticky;top:0}td.na{color:var(--mut)}.wrap{overflow-x:auto}</style></head><body><main>
<h1>Rust spike: 12-metric scorecard</h1>
<p class="sub">Updated ${new Date().toISOString().slice(0, 16).replace('T', ' ')} UTC. Baseline: Astryx Storybook at 957d934 (theme neutral), ${baseShots} of ${totalCases * 4} visual-matrix shots captured (${totalCases} cases × light/dark × 390/1280). Storybook first frame p50 ${sb?.firstFrameMsP50 ?? '—'} ms, transfer ${sb?.transferKB ?? '—'} KB. "—" = not built or not measured yet. Pass bars: visual ≤ 2% pixels differ per shot; a11y 100% on web; frame p95 ≤ 16.7 ms; first frame ≤ 100 ms.</p>
<div class="wrap"><table><tr><th>Metric</th><th>Node / row</th>${Object.values(hosts).map(h => `<th>${h}</th>`).join('')}</tr>
${rows.map(([m, n, vs]) => `<tr><td>${esc(m)}</td><td>${esc(n)}</td>${vs.map(cell).join('')}</tr>`).join('\n')}
</table></div></main></body></html>`);
