// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file Colocated tests for `parseTemplateTransform` — the load-boundary
 * validator for an integration's `templateTransform` module. Zod is sealed
 * inside the parser; these exercise the public contract (validated value out /
 * readable error thrown).
 */

import {describe, it, expect} from 'vitest';
import {parseTemplateTransform} from './parse.mjs';

/** Run parseTemplateTransform and return the thrown message (asserting it throws). */
function reason(value, label = 'transform') {
  try {
    parseTemplateTransform(value, label);
  } catch (err) {
    return err instanceof Error ? err.message : String(err);
  }
  throw new Error('expected parseTemplateTransform to throw');
}

describe('parseTemplateTransform (load boundary)', () => {
  it('accepts a minimal wrap (normalized to a one-element stack)', () => {
    const parsed = parseTemplateTransform({
      wrap: {component: 'AppFrame', from: '@xds/meta'},
    });
    const wraps = Array.isArray(parsed.wrap) ? parsed.wrap : [parsed.wrap];
    expect(wraps[0].component).toBe('AppFrame');
  });

  it('accepts wrap with props, importKind, and a scope', () => {
    expect(() =>
      parseTemplateTransform({
        appliesTo: {types: ['page', 'block']},
        wrap: {
          component: 'Frame',
          from: '@xds/meta',
          importKind: 'default',
          props: {surface: 'internal', density: 2, compact: true},
        },
      }),
    ).not.toThrow();
  });

  it('requires a wrap', () => {
    expect(reason({})).toContain('wrap');
    expect(reason({appliesTo: {types: ['page']}})).toContain('wrap');
  });

  it('accepts a wrap stack (array, outermost first)', () => {
    const parsed = parseTemplateTransform({
      wrap: [
        {component: 'MetaProvider', from: '@xds/meta'},
        {component: 'AppFrame', from: '@xds/meta', props: {surface: 'internal'}},
      ],
    });
    expect(Array.isArray(parsed.wrap)).toBe(true);
    expect(parsed.wrap).toHaveLength(2);
  });

  it('rejects an empty wrap stack', () => {
    expect(reason({wrap: []})).toContain('wrap');
  });

  it('rejects an invalid component inside a wrap stack', () => {
    expect(
      reason({wrap: [{component: 'OK', from: '@m'}, {component: 'no.good', from: '@m'}]}),
    ).toContain('component');
  });

  it('rejects unknown keys (strict)', () => {
    expect(
      reason({wrap: {component: 'X', from: '@xds/meta'}, bogus: true}),
    ).toContain('Unrecognized key');
  });

  it('rejects a wrap missing its component', () => {
    expect(reason({wrap: {from: '@xds/meta'}})).toContain('component');
  });

  it('rejects a wrap missing its from (wrap always imports the wrapper)', () => {
    expect(reason({wrap: {component: 'MetaAppFrame'}})).toContain('from');
  });

  it('accepts a description', () => {
    const parsed = parseTemplateTransform({
      description: 'Wraps pages in the Meta shell.',
      wrap: {component: 'X', from: '@m'},
    });
    expect(parsed.description).toBe('Wraps pages in the Meta shell.');
  });

  it('accepts appliesTo include/exclude/packages scoping', () => {
    expect(() =>
      parseTemplateTransform({
        appliesTo: {
          types: ['page'],
          include: ['dashboard', 'marketing/*'],
          exclude: ['blank'],
          packages: ['@astryxdesign/core'],
        },
        wrap: {component: 'X', from: '@m'},
      }),
    ).not.toThrow();
  });

  it('rejects an unknown appliesTo type', () => {
    expect(
      reason({
        wrap: {component: 'X', from: '@xds/meta'},
        appliesTo: {types: ['widget']},
      }),
    ).toContain('appliesTo.types');
  });

  it('rejects a non-serializable value nested inside an object prop', () => {
    expect(
      reason({
        wrap: {component: 'X', from: '@m', props: {config: {onChange: () => {}}}},
      }),
    ).toContain('props');
  });

  it('rejects a non-identifier component name', () => {
    expect(reason({wrap: {component: 'Foo.Bar', from: '@m'}})).toContain(
      'component',
    );
    expect(reason({wrap: {component: 'has space', from: '@m'}})).toContain(
      'component',
    );
  });

  it('rejects an empty from', () => {
    expect(reason({wrap: {component: 'X', from: ''}})).toContain('from');
  });

  it('rejects an invalid prop name (would split into multiple attributes)', () => {
    expect(
      reason({wrap: {component: 'X', from: '@m', props: {'bad key': 'v'}}}),
    ).toContain('props');
  });

  it('accepts object and array prop values (JSON-shaped)', () => {
    expect(() =>
      parseTemplateTransform({
        wrap: {
          component: 'X',
          from: '@m',
          props: {
            config: {theme: 'dark', density: 3, nested: {a: [1, 2, {b: true}]}},
            tabs: ['a', 'b'],
            empty: {},
            nothing: null,
          },
        },
      }),
    ).not.toThrow();
  });

  it('rejects a non-JSON prop value (function)', () => {
    expect(
      reason({wrap: {component: 'X', from: '@m', props: {onReady: () => {}}}}),
    ).toContain('props');
  });

  it('accepts hyphenated prop names (data-*/aria-*)', () => {
    expect(() =>
      parseTemplateTransform({
        wrap: {
          component: 'X',
          from: '@m',
          props: {'data-testid': 'frame', 'aria-label': 'shell'},
        },
      }),
    ).not.toThrow();
  });
});
