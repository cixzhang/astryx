// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file check-doc-import-specifiers.test.mjs
 * Unit tests for the documented-import-specifier gate: every specifier the
 * docs teach must survive Node's exports-map resolution, because a reader
 * copies it verbatim.
 */

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {describe, it, expect, beforeAll, afterAll} from 'vitest';
import {
  findSpecifiers,
  isDocSurface,
  resolveSpecifiers,
} from './check-doc-import-specifiers.mjs';

const NAMES = ['@astryxdesign/core', '@astryxdesign/lab'];

describe('findSpecifiers', () => {
  it('finds a specifier in an import statement', () => {
    expect(
      findSpecifiers(`import {Button} from '@astryxdesign/core/Button';`, NAMES),
    ).toEqual([{specifier: '@astryxdesign/core/Button', line: 1}]);
  });

  it('finds a specifier inside a template literal, as a CodeBlock carries it', () => {
    const source = ['const code = `', `import {Theme} from '@astryxdesign/core/theme';`, '`;'].join('\n');
    expect(findSpecifiers(source, NAMES)).toEqual([
      {specifier: '@astryxdesign/core/theme', line: 2},
    ]);
  });

  it('finds a specifier quoted inside a quoted doc string', () => {
    // Doc prose escapes its quotes; the specifier is still copy-paste material.
    const source = `description: 'use IconButton from \\'@astryxdesign/core/IconButton\\'.'`;
    expect(findSpecifiers(source, NAMES)).toEqual([
      {specifier: '@astryxdesign/core/IconButton', line: 1},
    ]);
  });

  it('finds a backtick-quoted specifier in markdown prose', () => {
    expect(findSpecifiers('Import it from `@astryxdesign/lab`.', NAMES)).toEqual([
      {specifier: '@astryxdesign/lab', line: 1},
    ]);
  });

  it('ignores placeholder shapes', () => {
    const source = [
      `import {...} from '@astryxdesign/core/ComponentName';`,
      `'@astryxdesign/core/<Component>'`,
      `'@astryxdesign/core/{a,b}'`,
    ].join('\n');
    expect(findSpecifiers(source, NAMES)).toEqual([]);
  });

  it('ignores a computed specifier', () => {
    expect(findSpecifiers('`@astryxdesign/core/${name}`', NAMES)).toEqual([]);
  });

  it('ignores an unrelated package', () => {
    expect(findSpecifiers(`import x from '@acme/core/Button';`, NAMES)).toEqual([]);
  });
});

describe('isDocSurface', () => {
  it.each([
    'packages/core/src/Button/Button.doc.mjs',
    'apps/storybook/stories/Stepper.stories.tsx',
    'packages/cli/assets/templates/pages/x/page.tsx',
    'packages/core/README.md',
  ])('includes %s', file => {
    expect(isDocSurface(file)).toBe(true);
  });

  it.each([
    // A changelog records specifiers that were right at the time.
    'packages/core/CHANGELOG.md',
    // A migration test's subject is the stale specifier it rewrites.
    'packages/cli/authoring/doctypes/parse.test.mjs',
    'packages/cli/assets/codemods/transforms/v0.3.0/migrate-authoring-imports.mjs',
  ])('excludes %s', file => {
    expect(isDocSurface(file)).toBe(false);
  });
});

describe('resolveSpecifiers', () => {
  let pkgDir;

  beforeAll(() => {
    pkgDir = fs.mkdtempSync(path.join(os.tmpdir(), 'astryx-fixture-pkg-'));
    fs.writeFileSync(
      path.join(pkgDir, 'package.json'),
      JSON.stringify({
        name: '@fixture/ui',
        exports: {
          '.': './index.js',
          './Employee': './dist/Employee.js',
          './deep/*': './dist/deep/*.js',
        },
      }),
    );
  });

  afterAll(() => {
    fs.rmSync(pkgDir, {recursive: true, force: true});
  });

  const packages = () => new Map([['@fixture/ui', pkgDir]]);

  it('passes a specifier the exports map declares', () => {
    expect([...resolveSpecifiers(['@fixture/ui/Employee'], packages())]).toEqual([]);
  });

  it('passes the package root', () => {
    expect([...resolveSpecifiers(['@fixture/ui'], packages())]).toEqual([]);
  });

  it('passes a subpath matched by a wildcard pattern', () => {
    // Textual key lookup would miss this; Node's resolver does not.
    expect([...resolveSpecifiers(['@fixture/ui/deep/Gauge'], packages())]).toEqual([]);
  });

  it('fails a subpath the exports map does not declare', () => {
    expect(resolveSpecifiers(['@fixture/ui/EmployeeCard'], packages())).toEqual(
      new Map([['@fixture/ui/EmployeeCard', 'ERR_PACKAGE_PATH_NOT_EXPORTED']]),
    );
  });

  it('fails a deep path that reaches past the exports map', () => {
    expect(
      resolveSpecifiers(['@fixture/ui/dist/Employee.js'], packages()),
    ).toEqual(
      new Map([['@fixture/ui/dist/Employee.js', 'ERR_PACKAGE_PATH_NOT_EXPORTED']]),
    );
  });
});
