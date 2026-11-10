// Builds the 12-metric scorecard (HTML) from the harness outputs:
//   shots/<host>/<Family>/diff.json   visual parity per shot vs the Astryx React baseline (1)
//   shots/<host>/<Family>/index.json  a11y snapshots (3) and desktop first frame (5)
//   checks.json                       keyboard, behavior, text input, a11y spot checks (2, 3, 4)
//   perf2.json                        web first frame, frame time, transfer (5, 6)
//   authoring.json                    library and app source size per stack (7)
//   exact-unsupported.json            props the Exact lane marks as not expressible (1)
//   reports.json                      measured sizes and lane-reported values (6, 8-12)
//   node scorecard.mjs > scorecard.html
import { readFileSync, existsSync, readdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const here = dirname(fileURLToPath(import.meta.url));
const J = p => existsSync(p) ? JSON.parse(readFileSync(p, 'utf8')) : null;
const hosts = { exact: 'Exact web', 'exact-linux': 'Exact Linux', 'ours-web': 'Ours web', 'ours-linux': 'Ours Linux' };
const H = Object.keys(hosts);
const families = readdirSync(join(here, 'cases')).filter(f => f.endsWith('.json')).map(f => f.slice(0, -5)).sort();
const cases = Object.fromEntries(families.map(f => [f, J(join(here, 'cases', `${f}.json`))]));
const reports = J(join(here, 'reports.json')) ?? {}, checks = J(join(here, 'checks.json')) ?? {}, perf = J(join(here, 'perf2.json')) ?? {};
const authoring = J(join(here, 'authoring.json')) ?? {}, unsupported = J(join(here, 'exact-unsupported.json')) ?? {};
const esc = s => String(s ?? '').replace(/[&<>]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' }[c]));
const pct = (a, b) => b ? `${(100 * a / b).toFixed(1)}%` : '—';
const PASS = 2, CLOSE = 10;

// Case id -> { component, props }, and whether Exact marks one of its props as not expressible.
const caseInfo = new Map();
for (const f of families) for (const comp of cases[f].components) for (const c of comp.cases) caseInfo.set(c.id, { family: f, component: comp.component, props: c.props });
const notExpressible = id => { const c = caseInfo.get(id); const u = c && unsupported[c.component]; return !!u && Object.keys(c.props ?? {}).some(p => u.includes(p)); };

// Metric 1: per family and host, shots within 2% / within 10% of the baseline, mean diff.
const vis = {}; const tot = Object.fromEntries(H.map(h => [h, { base: 0, pass: 0, close: 0, compared: 0, notRendered: 0, nePass: 0, neBase: 0 }]));
for (const f of families) {
  vis[f] = {};
  const baseIdx = J(join(here, 'shots', 'baseline', f, 'index.json')); const baseShots = baseIdx?.cases.filter(r => r.file).length ?? 0;
  for (const h of H) {
    const d = J(join(here, 'shots', h, f, 'diff.json')); const t = tot[h]; t.base += baseShots;
    if (!d) { vis[f][h] = null; t.notRendered += baseShots; continue; }
    const rs = d.results.filter(r => r.pct !== undefined);
    const exp = rs.filter(r => !notExpressible(r.id)), ne = rs.filter(r => notExpressible(r.id));
    const v = { base: baseShots, compared: rs.length, pass: rs.filter(r => r.pct <= PASS).length, close: rs.filter(r => r.pct <= CLOSE).length, mean: d.summary.meanDiffPercent,
      expPass: exp.filter(r => r.pct <= PASS).length, exp: exp.length, ne: ne.length, nePass: ne.filter(r => r.pct <= PASS).length };
    vis[f][h] = v; t.pass += v.pass; t.close += v.close; t.compared += v.compared; t.notRendered += baseShots - v.compared; t.ne = (t.ne ?? 0) + v.ne; t.nePass += v.nePass;
  }
}

// Metric 3: a11y tree match, as the multiset of (role, name) pairs under the case box.
const SKIP = new Set(['generic', 'none', 'presentation', 'application', 'main', 'text']);
const pairsFromSnapshot = s => (s ?? '').split('\n').map(l => l.match(/^\s*-\s*([a-z]+)(?:\s+"([^"]*)")?/)).filter(Boolean).map(m => `${m[1]}:${(m[2] ?? '').toLowerCase()}`);
const pairsFromTree = n => { const out = []; const walk = x => { if (!x) return; out.push(`${String(x.role ?? '').toLowerCase()}:${String(x.label ?? '').toLowerCase()}`); (x.children ?? []).forEach(walk); }; walk(n); return out; };
const findCase = n => n?.label === 'case' ? n : (n?.children ?? []).map(findCase).find(Boolean);
const key = ps => ps.filter(p => !SKIP.has(p.split(':')[0]) && p !== 'group:case').sort().join('|');
const a11yOf = (h, f, r) => {
  if (r.ax) return key(pairsFromSnapshot(r.ax));
  if (r.a11yFile) { const t = J(join(here, 'shots', h, f, r.a11yFile)); const c = findCase(t); return c ? key((c.children ?? []).flatMap(pairsFromTree)) : null; }
  return null;
};
const a11y = {}; const a11yTot = Object.fromEntries(H.map(h => [h, { n: 0, ok: 0, avail: 0 }]));
for (const f of families) {
  a11y[f] = {}; const b = J(join(here, 'shots', 'baseline', f, 'index.json'));
  const bm = new Map((b?.cases ?? []).filter(r => r.ax && r.scheme === 'light').map(r => [`${r.id}.${r.width}`, a11yOf('baseline', f, r)]));
  for (const h of H) {
    const a = J(join(here, 'shots', h, f, 'index.json')); if (!a) { a11y[f][h] = null; continue; }
    let n = 0, ok = 0, avail = 0;
    for (const r of a.cases) { if (r.scheme !== 'light') continue; const k = `${r.id}.${r.width}`; if (!bm.has(k)) continue; n++; const v = a11yOf(h, f, r); if (v != null) avail++; if (v != null && v === bm.get(k)) ok++; }
    a11y[f][h] = n ? { n, ok, avail } : null; Object.assign(a11yTot[h], { n: a11yTot[h].n + n, ok: a11yTot[h].ok + ok, avail: a11yTot[h].avail + avail });
  }
}

// Metric 5: desktop first frame from the capture records.
const ff = h => { const xs = []; for (const f of families) { const a = J(join(here, 'shots', h, f, 'index.json')); for (const r of a?.cases ?? []) if (r.firstFrameMs) xs.push(+r.firstFrameMs); } xs.sort((a, b) => a - b); return xs.length ? { p50: xs[Math.floor(xs.length / 2)], p95: xs[Math.floor(xs.length * 0.95)], n: xs.length } : null; };
const ffLinux = ff('ours-linux');

const chk = (h, name) => { const r = checks[h]?.[Object.keys(checks[h] ?? {}).find(k => k.startsWith(name))]; return r ? `${r.pass ? 'pass' : 'FAIL'}${r.score ? ` (${r.score})` : ''}` : '—'; };
const visCell = v => v == null ? '<td class="na">not rendered</td>' : `<td class="${v.pass / Math.max(1, v.base) >= 0.9 ? 'ok' : v.close / Math.max(1, v.base) >= 0.5 ? 'mid' : 'bad'}">${pct(v.pass, v.base)} <span class="m">≤10%: ${pct(v.close, v.base)} · mean ${v.mean ?? '—'}%${v.ne ? ` · not expressible ${v.ne}` : ''}</span></td>`;
const a11yCell = v => v == null ? '<td class="na">—</td>' : v.avail === 0 ? '<td class="na">no tree</td>' : `<td>${pct(v.ok, v.n)} <span class="m">${v.ok}/${v.n}</span></td>`;
const row = (label, vals) => `<tr><td>${label}</td>${vals.map(v => `<td>${v ?? '<span class="na">—</span>'}</td>`).join('')}</tr>`;
const rep = (m, node) => reports?.[m]?.[node] ?? {};
const totalCases = families.reduce((s, f) => s + cases[f].components.reduce((a, c) => a + c.cases.length, 0), 0);
const baseShots = H.length ? tot[H[0]].base : 0;

const headline = [
  `Visual parity (shots within 2% of Astryx React, of ${baseShots}): ${H.map(h => `${hosts[h]} ${pct(tot[h].pass, tot[h].base)}`).join(' · ')}. Within 10%: ${H.map(h => `${hosts[h]} ${pct(tot[h].close, tot[h].base)}`).join(' · ')}.`,
  `Keyboard/behavior/text checks passed (of 5): React ${Object.values(checks.baseline ?? {}).filter(r => r.pass).length} · Exact ${Object.values(checks.exact ?? {}).filter(r => r.pass).length} · Ours web ${Object.values(checks['ours-web'] ?? {}).filter(r => r.pass).length}.`,
  `First frame p50 (web gallery): React ${perf.baseline?.firstFrameMs?.p50 ?? '—'} ms · Exact ${perf.exact?.firstFrameMs?.p50 ?? '—'} ms · Ours ${perf['ours-web']?.firstFrameMs?.p50 ?? '—'} ms; ours desktop ${ffLinux?.p50 ?? '—'} ms.`,
  `Web transfer, brotli (whole gallery): React ${perf.baseline?.transfer?.brotliKB ?? '—'} KB · Exact ${perf.exact?.transfer?.brotliKB ?? '—'} KB · Ours ${perf['ours-web']?.transfer?.brotliKB ?? '—'} KB.`,
  `Closure ported (files): Exact ${rep('11', 'headline').exact ?? '—'} · Ours ${rep('11', 'headline')['ours-web'] ?? '—'}.`,
];

console.log(`<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Rust spike scorecard</title>
<style>:root{color-scheme:light dark;--fg:light-dark(#1c1e21,#e4e6eb);--mut:light-dark(#5d6168,#a8adb5);--bg:light-dark(#fff,#0e0f11);--card:light-dark(#f5f6f8,#1a1c1f);--line:light-dark(#dcdfe4,#30343a);--ok:light-dark(#e6f4ea,#14301c);--mid:light-dark(#fff6dc,#33290f);--bad:light-dark(#fde8e8,#3a1717)}
body{font:14px/1.45 system-ui,sans-serif;color:var(--fg);background:var(--bg);margin:0}main{max-width:1180px;margin:0 auto;padding:20px}
h1{font-size:22px;margin:0 0 4px}h2{font-size:17px;margin:26px 0 6px}.sub,.m,.na{color:var(--mut)}.m{font-size:12px;display:block}
table{border-collapse:collapse;width:100%;margin-top:8px}th,td{border-bottom:1px solid var(--line);padding:6px 8px;text-align:left;vertical-align:top;overflow-wrap:anywhere}
th{background:var(--card);position:sticky;top:0}td.ok{background:var(--ok)}td.mid{background:var(--mid)}td.bad{background:var(--bad)}.wrap{overflow-x:auto}
ol.head{background:var(--card);padding:12px 12px 12px 32px;border-radius:8px}ol.head li{margin:3px 0}</style></head><body><main>
<h1>Rust spike: 12-metric scorecard</h1>
<p class="sub">Updated ${new Date().toISOString().slice(0, 16).replace('T', ' ')} UTC. Final lane tips: Exact de45fcb1; ours 65104fe1 for captures, 572506c1 for interactive checks. Baseline: Astryx React (@astryxdesign/core at 957d934, theme neutral) rendered by qa/baseline under the same case-box contract: ${families.length} families, ${totalCases} cases × light/dark × 390/1280 = ${baseShots} shots per stack. Shots are trimmed to painted content + 16 px before the pixel diff (channel tolerance 8). A shot passes at ≤ 2% differing pixels; "≤10%" is a looser near-match. Machine: a Linux x86_64 server, headless Chrome without a GPU (canvas renders through SwiftShader on the CPU).</p>
<h2>Headline</h2><ol class="head">${headline.map(l => `<li>${esc(l)}</li>`).join('')}</ol>

<h2>1 · Visual parity per family</h2>
<p class="sub">Exact columns also count cases that set a prop the Exact lane lists as not expressible ("not expressible n"); with those cases removed, Exact web passes ${pct(families.reduce((s, f) => s + (vis[f].exact?.expPass ?? 0), 0), families.reduce((s, f) => s + (vis[f].exact?.exp ?? 0), 0))} of the shots it can express, against ${pct(tot.exact.pass, tot.exact.compared)} of all shots it rendered. Shots not rendered: ${H.map(h => `${hosts[h]} ${tot[h].notRendered}`).join(', ')}.</p>
<div class="wrap"><table><tr><th>Family</th>${H.map(h => `<th>${hosts[h]}</th>`).join('')}</tr>
<tr><td><b>All families</b></td>${H.map(h => `<td><b>${pct(tot[h].pass, tot[h].base)}</b> <span class="m">≤10%: ${pct(tot[h].close, tot[h].base)} · ${tot[h].pass}/${tot[h].base}</span></td>`).join('')}</tr>
${families.map(f => `<tr><td>${f}</td>${H.map(h => visCell(vis[f][h])).join('')}</tr>`).join('\n')}</table></div>

<h2>2 · Behavior parity</h2>
<table><tr><th>Check</th><th>Astryx React</th><th>Exact web</th><th>Ours web</th></tr>
${['keyboard: Tab reaches the switch', 'keyboard: arrow keys', 'a11y + keyboard: table'].map(n => `<tr><td>${esc(Object.keys(checks.exact ?? {}).find(k => k.startsWith(n)) ?? n)}</td><td>${chk('baseline', n)}</td><td>${chk('exact', n)}</td><td>${chk('ours-web', n)}</td></tr>`).join('')}
${Object.entries(reports['2'] ?? {}).map(([node, v]) => `<tr><td>${esc(node)}</td><td class="na">—</td><td>${esc(v.exact)}</td><td>${esc(v['ours-web'])}</td></tr>`).join('')}</table>

<h2>3 · Accessibility</h2>
<p class="sub">Tree match: each light-scheme shot's (role, name) pairs under the case box equal the React page's. Exact Linux has no accessibility tree headless.</p>
<div class="wrap"><table><tr><th>Family</th>${H.map(h => `<th>${hosts[h]}</th>`).join('')}</tr>
<tr><td><b>All families</b></td>${H.map(h => `<td><b>${a11yTot[h].avail ? pct(a11yTot[h].ok, a11yTot[h].n) : 'no tree'}</b> <span class="m">${a11yTot[h].ok}/${a11yTot[h].n}</span></td>`).join('')}</tr>
<tr><td>Combobox names its highlighted option (aria-activedescendant)</td><td colspan="4">React ${chk('baseline', 'a11y: combobox')} · Exact ${chk('exact', 'a11y: combobox')} (attribute refused by Contract) · Ours web ${chk('ours-web', 'a11y: combobox')} </td></tr>
<tr><td>Table exposes aria-rowcount</td><td colspan="4">React ${checks.baseline?.[Object.keys(checks.baseline ?? {}).find(k => k.startsWith('a11y + keyboard'))]?.rowcountExposed ? 'yes' : 'no'} · Exact ${checks.exact?.[Object.keys(checks.exact ?? {}).find(k => k.startsWith('a11y + keyboard'))]?.rowcountExposed ? 'yes' : 'no (refused)'} · Ours web ${checks['ours-web']?.[Object.keys(checks['ours-web'] ?? {}).find(k => k.startsWith('a11y + keyboard'))]?.rowcountExposed ? 'yes' : 'no'}</td></tr>
${families.map(f => `<tr><td>${f}</td>${H.map(h => a11yCell(a11y[f][h])).join('')}</tr>`).join('\n')}</table></div>

<h2>4 · Text input</h2>
<table><tr><th>Check</th><th>Astryx React</th><th>Exact web</th><th>Ours web</th></tr>
<tr><td>Type, IME commit (に→日), paste-style insert, Ctrl+A replace</td><td>${chk('baseline', 'text input')}</td><td>${chk('exact', 'text input')}</td><td>${chk('ours-web', 'text input')}</td></tr>
${Object.entries(reports['4'] ?? {}).map(([node, v]) => `<tr><td>${esc(node)}</td><td class="na">—</td><td>${esc(v.exact)}; Linux: ${esc(v['exact-linux'])}</td><td>${esc(v['ours-web'])}; desktop: ${esc(v['ours-linux'])}</td></tr>`).join('')}</table>

<h2>5 · Performance</h2>
<table><tr><th>Measure</th><th>Astryx React</th><th>Exact web</th><th>Ours web</th><th>Ours desktop</th></tr>
${row('First frame p50, gallery case page (ms, 5 runs)', [perf.baseline?.firstFrameMs?.p50, perf.exact?.firstFrameMs?.p50, perf['ours-web']?.firstFrameMs?.p50, ffLinux ? `${ffLinux.p50} (p95 ${ffLinux.p95}, ${ffLinux.n} launches)` : null])}
${row('Frame time p50 / p95 while toggling a switch, frame-rate limit off (ms)', [perf.baseline && `${perf.baseline.frameMs.p50} / ${perf.baseline.frameMs.p95}`, perf.exact && `${perf.exact.frameMs.p50} / ${perf.exact.frameMs.p95}`, perf['ours-web'] && `${perf['ours-web'].frameMs.p50} / ${perf['ours-web'].frameMs.p95}`, null])}
</table><p class="sub">With vsync on, all three web stacks hold 16.7 ms p95. With the limit off, a DOM page's frame does no work when nothing changes; our canvas page repaints each frame on the CPU (no GPU here), which is what its p95 measures. Exact Linux: no first-frame mark from its headless host.</p>

<h2>6 · Size</h2>
<table><tr><th>Measure</th><th>Astryx React</th><th>Exact</th><th>Ours</th></tr>
${row('Web transfer, whole gallery: raw / gzip / brotli (KB)', ['baseline', 'exact', 'ours-web'].map(h => perf[h]?.transfer ? `${perf[h].transfer.rawKB} / ${perf[h].transfer.gzipKB} / ${perf[h].transfer.brotliKB}` : null))}
${row('Largest responses', ['baseline', 'exact', 'ours-web'].map(h => perf[h]?.transfer?.largest?.join('; ')))}
${Object.entries(reports['6'] ?? {}).map(([node, v]) => row(esc(node), [esc(v.baseline), esc(v.exact), esc(v['ours-web'])])).join('')}</table>

<h2>7 · Authoring cost</h2>
<table><tr><th>Measure</th><th>Exact (Contract + Tamber)</th><th>Ours (Rust)</th></tr>
${row('Component library: files / lines / KB', [authoring.exact && `${authoring.exact.total.files} / ${authoring.exact.total.lines} / ${authoring.exact.total.kb}`, authoring.ours && `${authoring.ours.total.files} / ${authoring.ours.total.lines} / ${authoring.ours.total.kb}`])}
${row('Lines per family (67)', [authoring.exact && Math.round(authoring.exact.total.lines / 67), authoring.ours && Math.round(authoring.ours.total.lines / 67)])}
${row('Gallery + templates: files / lines', [authoring.exact && `${authoring.exact.apps.files} / ${authoring.exact.apps.lines}`, authoring.ours && `${authoring.ours.apps.files} / ${authoring.ours.apps.lines}`])}
${row('Tests: files / lines', [authoring.exact && `${authoring.exact.tests.files} / ${authoring.exact.tests.lines}`, authoring.ours && `${authoring.ours.tests.files} / ${authoring.ours.tests.lines}`])}
${row('Changes to the shared base', ['none (Exact consumed by path, unmodified)', esc(authoring.ours?.packagesRustDelta)])}
${row('By area (lines)', [authoring.exact && Object.entries(authoring.exact.areas).map(([k, v]) => `${k} ${v.lines}`).join(', '), authoring.ours && Object.entries(authoring.ours.areas).map(([k, v]) => `${k} ${v.lines}`).join(', ')])}
</table>
${['8', '9', '10', '11', '12'].map(m => `<h2>${{ 8: '8 · Theming', 9: '9 · Agent loop', 10: '10 · Ceilings', 11: '11 · Closure ported', 12: '12 · Table API fit' }[m]}</h2><table><tr><th>Row</th>${H.map(h => `<th>${hosts[h]}</th>`).join('')}</tr>${Object.entries(reports[m] ?? {}).map(([node, v]) => row(esc(node), H.map(h => esc(v[h] ?? '')))).join('')}</table>`).join('\n')}
</main></body></html>`);
