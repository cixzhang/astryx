// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file Regression unit tests — one minimal, documented test per bug the chaos
 * suite (and the live demo) surfaced. These pin the exact behavior so a future
 * reimplementation of the engine can't silently reintroduce a known defect.
 *
 * Each `it` names the finding and the commit-worthy invariant it guards.
 */

import {describe, it, expect} from 'vitest';
import jscodeshift from 'jscodeshift';
import {fixDirectiveCorruption} from '../../../assets/codemods/runner.mjs';
import {ensureImport, wrapDefaultExportReturn} from './jsx.mjs';
import {applyTemplateTransforms} from './apply.mjs';
import {parseTemplateTransform} from '../../../authoring/template-transform/parse.mjs';

const CORE = '@astryxdesign/core';
const META = '@xds/meta';

/** Run a helper mutation and return the printed source (engine post-processing applied). */
function edit(src, mutate) {
  const j = jscodeshift.withParser('tsx');
  const root = j(src);
  mutate(j, root);
  return fixDirectiveCorruption(root.toSource({quote: 'single'}));
}

/** Run one wrap transform through the full engine. */
function wrap(src, extra = {}, opts = {}) {
  return applyTemplateTransforms(src, {
    filePath: '/t/page.tsx',
    template: {type: 'page', id: 'x', package: CORE},
    transforms: [{package: META, transform: {wrap: {component: 'W', from: META, ...extra}}}],
    jscodeshift,
    ...opts,
  });
}

const assertParses = src =>
  expect(() => jscodeshift.withParser('tsx')(src)).not.toThrow();
const count = (src, re) => (src.match(re) ?? []).length;

describe('regression: #1 return-statement parentheses leak into JSX children', () => {
  // `return (<X/>)` marked the argument parenthesized; recast reprinted the
  // parens inside the wrapper, where they render as literal `(` `)` text.
  it('emits no parentheses around the wrapped child', () => {
    const out = edit(
      `export default function Page() {\n  return (\n    <X />\n  );\n}\n`,
      (j, root) => wrapDefaultExportReturn(j, root, [{component: 'W'}]),
    );
    expect(out).not.toContain('>(');
    expect(out).not.toContain('/>)');
    expect(out).not.toContain('(<X');
    expect(out).toMatch(/<W>\s*<X \/>\s*<\/W>/);
    assertParses(out);
  });
});

describe("regression: #2 'use client' directive dropped / double-semicolon", () => {
  // Splicing program.body made recast drop the directive; the fix then exposed
  // recast's `'use client';;` double-print, cleaned by fixDirectiveCorruption.
  it('preserves a single directive and inserts the import after it', () => {
    const out = edit(`'use client';\nexport const y = 1;\n`, (j, root) =>
      ensureImport(j, root, {from: '@m', named: ['A']}),
    );
    expect(count(out, /'use client';/g)).toBe(1);
    expect(out).not.toContain("'use client';;");
    expect(out).toMatch(/^'use client';\s*\nimport \{\s*A\s*\} from '@m';/);
    assertParses(out);
  });
});

describe('regression: #3 only the last return was wrapped', () => {
  // The original loop wrapped only direct body returns and could touch nested
  // callback returns. Now: wrap every return owned by the component; never a
  // nested callback's.
  it('wraps all early returns of the component', () => {
    const {source} = wrap(
      `export default function Page({loading}) {\n  if (loading) return <Spinner />;\n  return <Main />;\n}\n`,
    );
    expect(count(source, /<W>/g)).toBe(2);
    assertParses(source);
  });

  it('never wraps a return inside a nested callback', () => {
    const {source} = wrap(
      `export default function Page({items}) {\n  return <ul>{items.map((i) => { return <li key={i} />; })}</ul>;\n}\n`,
    );
    expect(count(source, /<W>/g)).toBe(1);
    expect(source).not.toMatch(/<W>\s*<li/);
    assertParses(source);
  });
});

describe('regression: #4 wrapper import self-collision → duplicate binding', () => {
  // The wrapper was already imported (as a default) from the same module; a
  // naive add produced a duplicate `W` binding. Now the existing local is reused.
  it('reuses an existing default import instead of duplicating the binding', () => {
    const {source, transformedBy} = wrap(
      `import W from '@xds/meta';\nexport default function Page() { return <X />; }\n`,
      {importKind: 'named'},
    );
    expect(transformedBy).toEqual([META]);
    expect(source).toContain('<W>');
    expect(count(source, /from '@xds\/meta'/g)).toBe(1);
    assertParses(source);
  });
});

describe('regression: #6 namespace import cannot accept merged specifiers', () => {
  // `import * as Meta from 'm'` can't be combined with named/default specifiers;
  // merging into it produced invalid syntax. Now a separate import is emitted.
  it('emits a separate import rather than corrupting a namespace import', () => {
    const {source, transformedBy} = wrap(
      `import * as Meta from '@xds/meta';\nexport default function Page() { return <X />; }\n`,
    );
    expect(transformedBy).toEqual([META]);
    expect(source).toContain('import * as Meta from');
    expect(source).toMatch(/import \{\s*W\s*\} from '@xds\/meta'/);
    assertParses(source);
  });
});

describe('regression: #5 adversarial props', () => {
  // (a) A string with both quote kinds can't be a JSX attribute string; it must
  // go through an expression container. (b) An invalid attribute name silently
  // split into two attrs — now rejected at parse and skipped in the builder.
  it('renders a quote-containing string prop via an expression container', () => {
    const {source, transformedBy} = wrap(
      `export default function Page() { return <X />; }`,
      {props: {title: `a "b" 'c'`}},
    );
    expect(transformedBy).toEqual([META]);
    expect(source).toMatch(/title=\{/);
    assertParses(source);
  });

  it('rejects an invalid prop name at the load boundary', () => {
    expect(() =>
      parseTemplateTransform({wrap: {component: 'W', from: META, props: {'bad key': 'v'}}}),
    ).toThrow(/props/);
  });

  it('skips an invalid prop name in the builder without corrupting output', () => {
    const {source} = wrap(`export default function Page() { return <X />; }`, {
      props: {'bad key': 'x', good: 'y'},
    });
    expect(source).not.toContain('bad key');
    expect(source).not.toMatch(/\bbad\b/);
    expect(source).toMatch(/good='y'/);
    assertParses(source);
  });
});
