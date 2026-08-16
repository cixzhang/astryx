// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file Conditional theme layers through `astryx theme build`.
 *
 * The conditional layer has to work in BOTH distribution modes. The runtime
 * side is covered by core's colocated `conditionalTheme.test.ts`; this suite
 * covers the built side — that `astryx theme build` emits the same `@media`
 * block, in the right layer, after the base rules, and that a theme without a
 * condition still builds with no conditional CSS at all.
 *
 * It also pins the resolution path: a theme file exporting a plain object
 * (rather than a `defineTheme()` result) has no second call argument to carry,
 * so it declares conditions under a `conditions` key — which the build's
 * "already resolved?" heuristic has to account for.
 *
 * Building requires a compiled @astryxdesign/core (there is no in-CLI fallback
 * generator), so the suite builds core once in beforeAll via the shared
 * ensureCoreBuilt() helper.
 */

import {describe, it, expect, beforeAll, beforeEach, afterEach} from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as os from 'node:os';
import {ensureCoreBuilt} from './ensure-core-built.mjs';
import {runCli} from '../../../test-utils/run-cli.mjs';

const MOBILE_QUERY = '@media (max-width: 756px) and (pointer: coarse)';

function writeTheme(dir, name, body) {
  fs.mkdirSync(dir, {recursive: true});
  // The CLI writes <basename>.css next to the source file, so use the
  // theme name as the filename for unambiguous fixtures.
  const file = path.join(dir, `${name}.mjs`);
  fs.writeFileSync(file, `export default ${body};\n`);
  return file;
}

async function buildTheme(tmpDir, name, body) {
  const project = path.join(tmpDir, 'project');
  const themesDir = path.join(project, 'themes');
  const themeFile = writeTheme(themesDir, name, body);

  const result = await runCli(
    ['theme', 'build', path.relative(project, themeFile)],
    project,
  );
  expect(result.code).toBe(0);

  const cssPath = path.join(themesDir, `${name}.css`);
  expect(fs.existsSync(cssPath)).toBe(true);
  return fs.readFileSync(cssPath, 'utf-8');
}

beforeAll(() => {
  ensureCoreBuilt();
}, 200_000);

let tmpDir;
beforeEach(() => {
  tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'astryx-build-theme-cond-'));
});
afterEach(() => {
  fs.rmSync(tmpDir, {recursive: true, force: true});
});

describe('theme build — conditional layer', () => {
  it('emits the mobile media block in @layer astryx-theme, after the base rules', async () => {
    const css = await buildTheme(
      tmpDir,
      'cond-mobile',
      `{
        name: 'cond-mobile',
        tokens: {'--spacing-4': '16px'},
        conditions: {mobile: {tokens: {'--spacing-4': '12px'}}},
      }`,
    );

    expect(css).toContain(MOBILE_QUERY);
    // Narrow AND touch — never a width-only query.
    expect(css).not.toMatch(/@media \(max-width: \d+px\)\s*\{/);
    // Scoped to the theme, like every other theme rule.
    expect(css).toContain('@scope ([data-astryx-theme="cond-mobile"])');

    // Precedence is source order: the conditional declaration has to land
    // after the base one, inside the same layer.
    const baseIndex = css.indexOf('--spacing-4: 16px;');
    const conditionalIndex = css.indexOf('--spacing-4: 12px;');
    expect(baseIndex).toBeGreaterThanOrEqual(0);
    expect(conditionalIndex).toBeGreaterThan(baseIndex);
    expect(css.lastIndexOf('@layer astryx-theme')).toBeLessThan(
      conditionalIndex,
    );
  }, 120_000);

  it('honors a raw media query key, including a custom breakpoint', async () => {
    const css = await buildTheme(
      tmpDir,
      'cond-bp',
      `{
        name: 'cond-bp',
        conditions: {
          '(max-width: 640px) and (pointer: coarse)': {
            tokens: {'--spacing-4': '12px'},
          },
          '(prefers-reduced-motion: reduce)': {
            tokens: {'--duration-fast': '0ms'},
          },
        },
      }`,
    );

    expect(css).toContain('@media (max-width: 640px) and (pointer: coarse)');
    expect(css).toContain('@media (prefers-reduced-motion: reduce)');
    expect(css).not.toContain('756px');
  }, 120_000);

  it('emits the color-scheme guard for light-dark() reached only through a condition', async () => {
    const css = await buildTheme(
      tmpDir,
      'cond-ld',
      `{
        name: 'cond-ld',
        tokens: {'--spacing-4': '16px'},
        conditions: {
          mobile: {tokens: {'--color-accent': ['#0050B3', '#7FB8FF']}},
        },
      }`,
    );

    expect(css).toContain('light-dark(#0050B3, #7FB8FF)');
    // Without the guard light-dark() resolves to its light arm forever.
    expect(css).toContain(':root { color-scheme: light dark; }');
  }, 120_000);

  it('emits no conditional CSS for a theme that declares no condition', async () => {
    const css = await buildTheme(
      tmpDir,
      'cond-none',
      `{name: 'cond-none', tokens: {'--spacing-4': '16px'}}`,
    );

    expect(css).not.toContain('pointer: coarse');
    expect(css).not.toContain('@media');
  }, 120_000);
});
