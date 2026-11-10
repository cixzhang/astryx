// Astryx's default icons → icons.contract. Reads packages/core/src/Icon/defaultIcons.tsx at the
// spike's Astryx commit (ASTRYX_SRC names a checkout or an extracted tree) and writes one SVG per
// icon name, chosen by `icon`. Icons are 24-unit outlines (stroke 1.5, round caps) or solid fills.
//
//   ASTRYX_SRC=<astryx tree> bun tools/astryx-icons.mjs
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';

const HERE = resolve(dirname(new URL(import.meta.url).pathname), '..');
const SRC = resolve(process.env.ASTRYX_SRC ?? resolve(HERE, '../../..'), 'packages/core/src/Icon/defaultIcons.tsx');
const text = readFileSync(SRC, 'utf8');
const body = text.slice(text.indexOf('export const defaultIcons'));

const entries = [];
const rx = /\n\s{2}(?:'([^']+)'|([A-Za-z]\w*)):\s*\(\s*<svg \{\.\.\.(svgProps|solidSvgProps)\}>([\s\S]*?)<\/svg>\s*\)/g;
for (let m; (m = rx.exec(body)); ) entries.push({ name: m[1] ?? m[2], solid: m[3] === 'solidSvgProps', inner: m[4] });

// One path per icon: a shape becomes path data, so Icon is one `svg` whatever the name. Contract
// expands a component at each place it is used; a branch per icon would repeat every icon there.
const num = (attrs, k) => Number(attrs[k] ?? 0);
function pathData(inner) {
  const ds = [];
  let evenodd = false;
  const el = /<(path|rect|circle)\s+([\s\S]*?)\/>/g;
  for (let m; (m = el.exec(inner)); ) {
    const attrs = {};
    const at = /(\w+)=(?:"([^"]*)"|\{([^}]*)\})/g;
    for (let a; (a = at.exec(m[2])); ) attrs[a[1]] = a[2] ?? a[3];
    if (attrs.fillRule === 'evenodd') evenodd = true;
    if (m[1] === 'path') ds.push(attrs.d);
    else if (m[1] === 'circle') {
      const [cx, cy, r] = ['cx', 'cy', 'r'].map(k => num(attrs, k));
      ds.push(`M${cx - r} ${cy}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0`);
    } else {
      const [x, y, w, h] = ['x', 'y', 'width', 'height'].map(k => num(attrs, k));
      const rx = num(attrs, 'rx');
      ds.push(`M${x + rx} ${y}h${w - 2 * rx}a${rx} ${rx} 0 0 1 ${rx} ${rx}v${h - 2 * rx}a${rx} ${rx} 0 0 1 ${-rx} ${rx}h${-(w - 2 * rx)}a${rx} ${rx} 0 0 1 ${-rx} ${-rx}v${-(h - 2 * rx)}a${rx} ${rx} 0 0 1 ${rx} ${-rx}z`);
    }
  }
  return { d: ds.join(''), evenodd };
}

const chain = (pick, fallback) => entries.reduceRight((acc, e) => `icon == ${JSON.stringify(e.name)} ? ${JSON.stringify(pick(e))} : ${acc}`, JSON.stringify(fallback));
for (const e of entries) Object.assign(e, pathData(e.inner));

const lines = [
  '// Astryx\'s default icons, written by tools/astryx-icons.mjs from Icon/defaultIcons.tsx; do not edit.',
  '// An outline icon draws in its colour (stroke 1.5, round caps), a solid one fills with it.',
  '',
  `fn iconNames(): string[] = [${entries.map(e => JSON.stringify(e.name)).join(', ')}]`,
  `fn iconPath(icon: string): string = ${chain(e => e.d, '')}`,
  `fn iconSolid(icon: string): bool = ${entries.filter(e => e.solid).map(e => `icon == ${JSON.stringify(e.name)}`).join(' or ') || 'false'}`,
  '',
  'component IconGlyph',
  '  props',
  '    icon: string',
  '  view',
  '    svg viewBox="0 0 24 24" width="100%" height="100%" aria-hidden=true',
  '      fill=(iconSolid(icon) ? "currentColor" : "none") stroke=(iconSolid(icon) ? "none" : "currentColor")',
  '      stroke-width=1.5 stroke-linecap="round" stroke-linejoin="round"',
  '      path d=iconPath(icon) fill-rule=(iconSolid(icon) ? "evenodd" : "nonzero")',
];
writeFileSync(resolve(HERE, 'astryx', 'icons.contract'), lines.join('\n') + '\n');
console.log(`icons ${entries.length}`);
