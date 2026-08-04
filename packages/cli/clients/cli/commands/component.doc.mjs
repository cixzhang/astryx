// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file CommandDoc for `astryx component`. The terminal binding of the
 * `component()` function (referenced via `fn`); its args/flags map to that
 * function's params so a converter can build Commander config + --help from one
 * source of truth.
 * @position packages/cli/clients/cli/commands — command documentation
 */

/** @type {import('@astryxdesign/cli/authoring').CommandDoc} */
export const doc = {
  type: 'command',
  name: 'component',
  displayName: 'astryx component',
  namespace: 'cli',
  summary: 'List components, or print a component doc.',
  description:
    'Resolves a component by name across core and integration packages and prints ' +
    'its authored doc, or lists the catalog grouped by category. Boolean flags narrow ' +
    'a single component to just its props, source, showcase, or example blocks.',
  fn: 'component',
  args: [{name: 'name', param: 'name', required: false}],
  options: [
    {flag: '--list', param: 'options.list'},
    {flag: '--category <category>', param: 'options.category'},
    {flag: '--props', param: 'options.props'},
    {flag: '--source', param: 'options.source'},
    {flag: '--showcase', param: 'options.showcase'},
    {flag: '--blocks', param: 'options.blocks'},
    {flag: '--package <name>', param: 'options.package'},
  ],
  examples: [
    {label: 'Browse the catalog', cli: 'astryx component --list'},
    {
      label: 'Props table as JSON',
      cli: 'astryx component XDSButton --props --json',
    },
  ],
  exitCodes: [
    {code: 0, when: 'success'},
    {
      code: 1,
      when: 'unknown component, category, or package — or @astryxdesign/core cannot be resolved',
    },
  ],
  related: ['search', 'hook', 'docs', 'template', 'swizzle'],
};
