#!/usr/bin/env node
// Exports the retro pixel theme (cixzhang/astryx-pixel-gallery) to
// tokens/pixel-themes.json, in the shape of tokens/themes.json, and its
// bitmap icons to tokens/pixel-icons.json.
//
// Usage, with a clone of the pixel gallery:
//   cd tools/pixel-tokens && npm install
//   node export.mjs <pixel gallery clone> ../../tokens
//
// For each palette it loads `createPixelTheme(3, palette)` from the clone and
// resolves it with Astryx's own resolver (`resolveThemeTokens`, as the native
// Swift generator does), so HCT palettes and light-dark pairs come out as
// Astryx makes them. Lengths are then divided by 3: the JSON is in pixel
// units (P = 1), and the Rust theme multiplies by P at run time.

import {cpSync, mkdirSync, rmSync, writeFileSync} from 'node:fs';
import {dirname, join, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';

// The pixel size the theme was designed and committed at (src/studio/pixel.css).
const REFERENCE_P = 3;
const PALETTES = ['garden', 'gameboy', 'nes', 'c64', 'pico8'];
const here = dirname(fileURLToPath(import.meta.url));
const clone = resolve(process.argv[2] ?? '.');
const outDir = resolve(process.argv[3] ?? '.');
const work = join(here, '.pixel-src');

// Member names, in the order and shape of tokens/themes.json.
const COLOR = (
  'accent accentMuted backgroundBlue backgroundBody backgroundCard backgroundCyan ' +
  'backgroundErrorInverted backgroundGray backgroundGreen backgroundInverted backgroundMuted ' +
  'backgroundOrange backgroundPink backgroundPopover backgroundPurple backgroundRed ' +
  'backgroundSurface backgroundTeal backgroundYellow border borderBlue borderCyan ' +
  'borderEmphasized borderError borderGray borderGreen borderInfo borderOrange borderPink ' +
  'borderPurple borderRed borderSuccess borderTeal borderWarning borderYellow dataBlue1 ' +
  'dataBlue2 dataBlue3 dataBlue4 dataBlue5 dataCategoricalBlue dataCategoricalBrown ' +
  'dataCategoricalCyan dataCategoricalGreen dataCategoricalIndigo dataCategoricalOrange ' +
  'dataCategoricalPink dataCategoricalPurple dataCategoricalRed dataCategoricalTeal dataGray1 ' +
  'dataGray2 dataGray3 dataGray4 dataGray5 dataNeutral dataOrange1 dataOrange2 dataOrange3 ' +
  'dataOrange4 dataOrange5 dataPink1 dataPink2 dataPink3 dataPink4 dataPink5 dataPurple1 ' +
  'dataPurple2 dataPurple3 dataPurple4 dataPurple5 dataRed1 dataRed2 dataRed3 dataRed4 ' +
  'dataRed5 dataShamrock1 dataShamrock2 dataShamrock3 dataShamrock4 dataShamrock5 dataTeal1 ' +
  'dataTeal2 dataTeal3 dataTeal4 dataTeal5 dataYellow1 dataYellow2 dataYellow3 dataYellow4 ' +
  'dataYellow5 error errorMuted iconAccent iconBlue iconCyan iconDisabled iconError iconGray ' +
  'iconGreen iconInfo iconOrange iconPink iconPrimary iconPurple iconRed iconSecondary ' +
  'iconSuccess iconTeal iconWarning iconYellow info infoMuted neutral onAccent onDark onError ' +
  'onErrorMuted onInfo onInfoMuted onLight onSuccess onSuccessMuted onWarning onWarningMuted ' +
  'overlay overlayHover overlayPressed shadow skeleton success successMuted syntaxAdded ' +
  'syntaxAddedMarker syntaxAttribute syntaxBackground syntaxComment syntaxConstant ' +
  'syntaxDiffHeader syntaxFunction syntaxKeyword syntaxNumber syntaxOperator syntaxProperty ' +
  'syntaxPunctuation syntaxRemoved syntaxRemovedMarker syntaxString syntaxTag syntaxType ' +
  'syntaxVariable textAccent textBlue textCyan textDisabled textError textGray textGreen ' +
  'textInfo textOrange textPink textPrimary textPurple textRed textSecondary textSuccess ' +
  'textTeal textWarning textYellow tintHover track warning warningMuted'
).split(' ');
const SPACE = ['0', '0-5', '1', '1-5', '2', '3', '4', '5', '6', '7', '8', '9', '10', '11', '12'];
const RADIUS = ['none', 'inner', 'element', 'container', 'page', 'chat', 'full'];
const TYPES = {
  body: 'body', code: 'code', label: 'label', large: 'large', supporting: 'supporting',
  heading1: 'heading-1', heading2: 'heading-2', heading3: 'heading-3',
  heading4: 'heading-4', heading5: 'heading-5', heading6: 'heading-6',
  display1: 'display-1', display2: 'display-2', display3: 'display-3',
};
const FONT_SIZES = {
  base: 'base', lg: 'lg', s2xl: '2xl', s2xs: '2xs', s3xl: '3xl', s3xs: '3xs',
  s4xl: '4xl', s4xs: '4xs', s5xl: '5xl', sm: 'sm', xl: 'xl', xs: 'xs',
};
// Astryx's weight scale. The pixel theme does not override it, so the
// theme CSS does not repeat it.
const WEIGHTS = {normal: 400, medium: 500, semibold: 600, bold: 700};

const kebab = (name) =>
  name.replace(/([a-z])([A-Z0-9])/g, '$1-$2').replace(/([0-9])([A-Z])/g, '$1-$2').toLowerCase();

// Splits on commas that are not inside parentheses.
function args(text) {
  const out = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (c === '(') depth++;
    else if (c === ')') depth--;
    else if (c === ',' && depth === 0) {
      out.push(text.slice(start, i).trim());
      start = i + 1;
    }
  }
  out.push(text.slice(start).trim());
  return out;
}

// The text inside the call `name(...)` that starts at `index`.
function callBody(text, index, name) {
  let depth = 0;
  for (let i = index + name.length; i < text.length; i++) {
    if (text[i] === '(') depth++;
    else if (text[i] === ')' && --depth === 0) return [text.slice(index + name.length + 1, i), i + 1];
  }
  throw new Error(`unbalanced ${name}( in ${text}`);
}

// Replaces var() and light-dark() until only literals remain.
function substitute(value, vars, mode, seen = new Set()) {
  for (;;) {
    const v = value.indexOf('var(');
    const ld = value.indexOf('light-dark(');
    if (v < 0 && ld < 0) return value;
    if (ld >= 0 && (v < 0 || ld < v)) {
      const [body, end] = callBody(value, ld, 'light-dark');
      const [light, dark] = args(body);
      value = value.slice(0, ld) + (mode === 'light' ? light : dark) + value.slice(end);
    } else {
      const [body, end] = callBody(value, v, 'var');
      const [name, fallback] = args(body);
      if (seen.has(name)) throw new Error(`var cycle at ${name}`);
      const found = vars.get(name) ?? fallback;
      if (found === undefined) throw new Error(`undefined ${name}`);
      const inner = substitute(found, vars, mode, new Set([...seen, name]));
      value = value.slice(0, v) + inner + value.slice(end);
    }
  }
}

function parseColor(text) {
  text = text.trim();
  const named = {transparent: [0, 0, 0, 0], black: [0, 0, 0, 1], white: [1, 1, 1, 1]};
  if (named[text]) return named[text];
  let m = text.match(/^#([0-9a-f]+)$/i);
  if (m) {
    let h = m[1];
    if (h.length <= 4) h = [...h].map((c) => c + c).join('');
    const n = (i) => parseInt(h.slice(i, i + 2), 16) / 255;
    return [n(0), n(2), n(4), h.length === 8 ? n(6) : 1];
  }
  m = text.match(/^rgba?\((.*)\)$/);
  if (m) {
    const p = args(m[1].replace(/\s*\/\s*/, ',').replace(/\s+/g, m[1].includes(',') ? '' : ','));
    const ch = (s) => (s.endsWith('%') ? parseFloat(s) / 100 : parseFloat(s) / 255);
    const a = p[3] === undefined ? 1 : p[3].endsWith('%') ? parseFloat(p[3]) / 100 : parseFloat(p[3]);
    return [ch(p[0]), ch(p[1]), ch(p[2]), a];
  }
  m = text.match(/^color-mix\(in srgb,(.*)\)$/s);
  if (m) {
    const parts = args(m[1]).map((part) => {
      const pm = part.match(/^(.*?)\s+([\d.]+)%$/);
      return pm ? [parseColor(pm[1]), parseFloat(pm[2]) / 100] : [parseColor(part), null];
    });
    let [[c1, p1], [c2, p2]] = parts;
    if (p1 === null && p2 === null) p1 = p2 = 0.5;
    else if (p1 === null) p1 = 1 - p2;
    else if (p2 === null) p2 = 1 - p1;
    // CSS color-mix interpolates premultiplied colors.
    const a = c1[3] * p1 + c2[3] * p2;
    const rgb = [0, 1, 2].map((i) => (a === 0 ? 0 : (c1[i] * c1[3] * p1 + c2[i] * c2[3] * p2) / a));
    return [...rgb, a];
  }
  throw new Error(`unsupported color ${text}`);
}

const round = (v) => Math.round(v * 1e6) / 1e6;
const rgba = (c) => ({rgba: c.map(round)});
const units = (px) => round(parseFloat(px) / REFERENCE_P);

function weight(text) {
  return WEIGHTS[text] ?? Number(text);
}

function shadow(text) {
  if (text === 'none') return [];
  return args(text).map((layer) => {
    const inset = /\binset\b/.test(layer);
    const body = layer.replace(/\binset\b/, '').trim();
    const colorStart = body.search(/(#|rgba?\(|color-mix\()/);
    const lengths = body.slice(0, colorStart).trim().split(/\s+/).map(units);
    const [x, y, blur = 0, spread = 0] = lengths;
    const out = {x, y, blur, spread, color: rgba(parseColor(body.slice(colorStart)))};
    if (inset) out.isInset = true;
    return out;
  });
}

function tokens(vars, mode) {
  const get = (name) => {
    if (!vars.has(name)) throw new Error(`missing ${name}`);
    return substitute(vars.get(name), vars, mode);
  };
  const color = {};
  for (const m of COLOR) {
    const css = get(`--color-${kebab(m)}`);
    try {
      color[m] = rgba(parseColor(css));
    } catch (error) {
      throw new Error(`${mode} ${m}: ${error.message}`);
    }
  }
  const space = {};
  for (const s of SPACE) space[`s${s.replace('-', '_')}`] = units(get(`--spacing-${s}`));
  const radius = {};
  for (const r of RADIUS) radius[r] = r === 'full' ? 9999 : units(get(`--radius-${r}`));
  const size = {
    borderWidth: units(get('--border-width')),
    elementLg: units(get('--size-element-lg')),
    elementMd: units(get('--size-element-md')),
    elementSm: units(get('--size-element-sm')),
  };
  const font = {};
  for (const [member, css] of Object.entries(TYPES)) {
    font[member] = {
      size: units(get(`--text-${css}-size`)),
      weight: weight(get(`--text-${css}-weight`)),
      leading: round(Number(get(`--text-${css}-leading`))),
    };
  }
  // The source theme sets every family to Silkscreen. The Rust port draws
  // all text in Galmuri instead: Galmuri7 for body text and controls, and
  // the larger Galmuri9 for headings, both on the pixel grid.
  font.family = {
    body: ['Galmuri7', 'monospace'],
    code: ['Galmuri7', 'monospace'],
    heading: ['Galmuri9', 'monospace'],
  };
  font.size = {};
  for (const [member, css] of Object.entries(FONT_SIZES)) font.size[member] = units(get(`--font-size-${css}`));
  font.weight = {...WEIGHTS};
  return {
    color,
    space,
    radius,
    size,
    font,
    shadow: {low: shadow(get('--shadow-low')), med: shadow(get('--shadow-med')), high: shadow(get('--shadow-high'))},
  };
}

// Each icon is an <svg> of 1x1 <rect>s; this turns it back into rows of
// `#` (filled) and `.` (empty).
function iconRows(icons) {
  const out = {};
  for (const [name, svg] of Object.entries(icons)) {
    const [, , width, height] = svg.props.viewBox.split(' ').map(Number);
    const rows = Array.from({length: height}, () => Array(width).fill('.'));
    for (const rect of [svg.props.children].flat(Infinity)) {
      if (rect?.props) rows[rect.props.y][rect.props.x] = '#';
    }
    out[name] = rows.map((row) => row.join(''));
  }
  return out;
}

// The theme files sit next to this script's node_modules, so they import
// the pinned Astryx core.
mkdirSync(work, {recursive: true});
try {
  for (const file of ['pixel-theme.tsx', 'pixel-icons.tsx']) {
    cpSync(join(clone, 'src/studio', file), join(work, file));
  }
  const {runnerImport} = await import('vite');
  const load = async (file) =>
    (await runnerImport(file, {root: here, configFile: false, logLevel: 'error'})).module;
  const {createPixelTheme} = await load(join(work, 'pixel-theme.tsx'));
  const {resolveThemeTokens} = await import('@astryxdesign/core/theme');
  const out = {};
  for (const palette of PALETTES) {
    const theme = createPixelTheme(REFERENCE_P, palette);
    out[palette] = {};
    for (const mode of ['light', 'dark']) {
      const vars = new Map(
        Object.entries(resolveThemeTokens(theme, {mode})).filter(([, v]) => typeof v === 'string'),
      );
      out[palette][mode] = tokens(vars, mode);
    }
  }
  writeFileSync(join(outDir, 'pixel-themes.json'), JSON.stringify(out, null, 1) + '\n');
  const {pixelIcons} = await load(join(work, 'pixel-icons.tsx'));
  writeFileSync(join(outDir, 'pixel-icons.json'), JSON.stringify(iconRows(pixelIcons), null, 1) + '\n');
} finally {
  rmSync(work, {recursive: true, force: true});
}
