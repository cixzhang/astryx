#!/usr/bin/env node
// Cuts the pixel theme's bundled fonts from the full Galmuri files:
//   node subset-fonts.mjs <dir with Galmuri7.ttf and Galmuri9.ttf> <out dir>
// Galmuri7 keeps Latin, Latin Extended, and Vietnamese. Galmuri9 also keeps
// Greek and Cyrillic, so it is the built-in fallback for them. Run by
// fetch-fonts.sh; the output is committed in crates/astryx-theme-pixel/fonts.

import {readFileSync, writeFileSync} from 'node:fs';
import {join} from 'node:path';
import subsetFont from 'subset-font';

const LATIN = [
  [0x0020, 0x007e], [0x00a0, 0x024f], [0x02b0, 0x036f], [0x1e00, 0x1eff],
  [0x2000, 0x206f], [0x20a0, 0x20cf], [0x2100, 0x215f], [0x2190, 0x21ff],
  [0x2212, 0x2212], [0x25a0, 0x25ff], [0xfffd, 0xfffd],
];
const GREEK_CYRILLIC = [[0x0370, 0x03ff], [0x0400, 0x052f]];

const text = (ranges) =>
  ranges.flatMap(([lo, hi]) => Array.from({length: hi - lo + 1}, (_, i) => String.fromCodePoint(lo + i))).join('');

const [from, to] = process.argv.slice(2);
for (const [name, ranges] of [
  ['Galmuri7', LATIN],
  ['Galmuri9', [...LATIN, ...GREEK_CYRILLIC]],
]) {
  const out = await subsetFont(readFileSync(join(from, `${name}.ttf`)), text(ranges), {
    targetFormat: 'truetype',
  });
  writeFileSync(join(to, `${name}-Latin.ttf`), out);
  console.log(`${name}-Latin.ttf: ${out.length} bytes`);
}
