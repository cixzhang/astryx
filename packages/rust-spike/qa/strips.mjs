// Side-by-side strip per family (Astryx React | Exact | ours) plus the two templates.
//   node strips.mjs <templates-dir> > strips.html
// Per family: one case of its root component, light, 390 wide: the case with the most props whose
// React shot is 40-400 px tall (else the first), with each shot's pixel difference from React.
import { readFileSync, existsSync, readdirSync, statSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const here = dirname(fileURLToPath(import.meta.url));
const tpl = process.argv[2];
const J = p => existsSync(p) ? JSON.parse(readFileSync(p, 'utf8')) : null;
const img = p => existsSync(p) && statSync(p).isFile() ? `<img loading="lazy" src="data:image/png;base64,${readFileSync(p).toString('base64')}">` : '<div class="none">not rendered</div>';
const families = readdirSync(join(here, 'cases')).filter(f => f.endsWith('.json')).map(f => f.slice(0, -5)).sort();
const cols = [['baseline', 'Astryx React'], ['exact', 'Exact (web)'], ['ours-web', 'Ours (web)'], ['exact-linux', 'Exact (Linux)'], ['ours-linux', 'Ours (Linux)']];
const rows = families.map(f => {
  const doc = J(join(here, 'cases', `${f}.json`)); const comp = doc.components[0]; if (!comp?.cases.length) return '';
  const bi = J(join(here, 'shots', 'baseline', f, 'index.json'));
  const hOf = id => bi?.cases.find(r => r.id === id && r.scheme === 'light' && r.width === 390 && r.box)?.box.height ?? 0;
  const fit = comp.cases.filter(x => { const h = hOf(x.id); return h >= 40 && h <= 400; }).sort((a, b) => Object.keys(b.props).length - Object.keys(a.props).length);
  const c = fit[0] ?? comp.cases[0];
  const file = `${c.id}.light.390.png`;
  const cells = cols.map(([h]) => {
    const d = J(join(here, 'shots', h, f, 'diff.json')); const r = d?.results.find(x => x.file === file);
    return `<td>${img(join(here, 'shots', h, f, file))}<div class="m">${h === 'baseline' ? 'reference' : r?.pct != null ? `${r.pct.toFixed(1)}% pixels differ` : ''}</div></td>`;
  }).join('');
  return `<tr><th>${f}<div class="m">${comp.component} · ${c.id}<br>${Object.keys(c.props).length ? Object.entries(c.props).map(([k, v]) => `${k}=${JSON.stringify(v)}`).join(' ').slice(0, 80) : 'defaults'}</div></th>${cells}</tr>`;
}).join('\n');
const T = (name, variants) => `<h2>Template: ${name}</h2><table><tr><th></th><th>Exact (web)</th><th>Exact (Linux)</th><th>Ours</th></tr>${variants.map(([label, ew, el, o]) => `<tr><th>${label}</th><td>${img(join(tpl, ew))}</td><td>${img(join(tpl, el))}</td><td>${o ? img(join(tpl, o)) : '<div class="none">—</div>'}</td></tr>`).join('')}</table>`;
console.log(`<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Rust spike: side by side</title>
<style>:root{color-scheme:light dark;--fg:light-dark(#1c1e21,#e4e6eb);--mut:light-dark(#5d6168,#a8adb5);--bg:light-dark(#fff,#0e0f11);--card:light-dark(#f5f6f8,#1a1c1f);--line:light-dark(#dcdfe4,#30343a)}
body{font:14px/1.45 system-ui,sans-serif;color:var(--fg);background:var(--bg);margin:0}main{max-width:1500px;margin:0 auto;padding:20px}
table{border-collapse:collapse;width:100%}th,td{border-bottom:1px solid var(--line);padding:6px;text-align:left;vertical-align:top}th{font-weight:600;width:150px}
thead th{background:var(--card);position:sticky;top:0}img{max-width:100%;max-height:320px;object-fit:contain;object-position:left top;border:1px solid var(--line);background:#fff}.m{color:var(--mut);font-size:12px;font-weight:400}.none{color:var(--mut);padding:12px}
h2{font-size:17px;margin:26px 0 6px}.wrap{overflow-x:auto}</style></head><body><main>
<h1>Rust spike: side by side</h1><p class="m">One case per family (the root component's case with the most props whose React shot is 40–400 px tall), light, 390 wide. The number under each image is the share of pixels that differ from Astryx React after trimming both to their painted content (channel tolerance 8; a shot passes at 2%).</p>
<div class="wrap"><table><thead><tr><th>Family</th>${cols.map(([, l]) => `<th>${l}</th>`).join('')}</tr></thead>${rows}</table></div>
${T('shell-nav', [['Desktop, light', 'exact-shell-nav-desktop-light.png', 'exact-linux-shell-nav-desktop-light.png', 'ours/shell-nav-desktop.png'], ['Desktop, dark', 'exact-shell-nav-desktop-dark.png', 'exact-linux-shell-nav-desktop-dark.png', 'ours/shell-nav-desktop-dark.png'], ['Command palette (⌘K)', 'exact-shell-nav-desktop-palette-filtered.png', '', 'ours/shell-nav-desktop-palette.png'], ['Mobile, light', 'exact-shell-nav-mobile-light.png', 'exact-linux-shell-nav-mobile-light.png', 'ours/shell-nav-phone.png'], ['Mobile, file tree sheet', 'exact-shell-nav-mobile-sheet.png', '', ''], ['Mobile, dark', 'exact-shell-nav-mobile-dark.png', 'exact-linux-shell-nav-mobile-dark.png', 'ours/shell-nav-phone-dark.png']])}
${T('settings-dialog', [['Desktop, light', 'exact-settings-dialog-desktop-light.png', 'exact-linux-settings-dialog-desktop-light.png', 'ours/settings-dialog-desktop.png'], ['Desktop, Appearance panel', 'exact-settings-dialog-desktop-appearance.png', '', 'ours/settings-dialog-desktop-appearance.png'], ['Desktop, dark', 'exact-settings-dialog-desktop-dark.png', 'exact-linux-settings-dialog-desktop-dark.png', 'ours/settings-dialog-desktop-dark.png'], ['Mobile, light', 'exact-settings-dialog-mobile-light.png', 'exact-linux-settings-dialog-mobile-light.png', 'ours/settings-dialog-phone.png'], ['Mobile, dark', 'exact-settings-dialog-mobile-dark.png', 'exact-linux-settings-dialog-mobile-dark.png', 'ours/settings-dialog-phone-dark.png']])}
</main></body></html>`);
