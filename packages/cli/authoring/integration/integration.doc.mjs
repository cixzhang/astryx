// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file SchemaDoc for the `astryx.integration.*` manifest (AstryxIntegration).
 * Colocated with the schema (`type.ts` + `parse.mjs`) it documents.
 * @position packages/cli/authoring/integration — schema documentation
 */

/** @type {import('@astryxdesign/cli/authoring').SchemaDoc} */
export const doc = {
  type: 'schema',
  name: 'integration',
  displayName: 'Astryx Integration',
  namespace: 'cli',
  description:
    'The astryx.integration.* manifest that sits beside an integration ' +
    "package's package.json. Points the CLI at the package's components, " +
    'templates, codemods, and an optional template transform, and where to ' +
    'file issues. Every field is optional.',
  appliesTo: 'astryx.integration.{ts,mjs,js}',
  fields: [
    {
      name: 'components',
      type: 'string',
      description:
        'Relative path to the components/docs root (resolved to absolute).',
      example: "'./src/components'",
    },
    {
      name: 'templates',
      type: 'string',
      description:
        'Relative path to the templates root (resolved to absolute).',
      example: "'./src/templates'",
    },
    {
      name: 'codemods',
      type: 'string',
      description: 'Relative path to the codemods root (resolved to absolute).',
      example: "'./codemods'",
    },
    {
      name: 'templateTransform',
      type: 'string',
      description:
        'Relative path to a template-transform module (resolved to absolute). ' +
        'Its default export (an AstryxTemplateTransform) reshapes the templates ' +
        'the CLI emits — e.g. wrapping every page in the design system shell — ' +
        'as a pure output-layer that never edits the on-disk templates.',
      example: "'./astryx/template-transform.ts'",
    },
    {
      name: 'issuesUrl',
      type: 'string',
      description: 'Where to file issues/feedback for this integration.',
      example: "'https://github.com/acme/widgets/issues'",
    },
  ],
  examples: [
    {
      label: 'Typical',
      code: `export default {
  components: './src/components',
  templates: './src/templates',
  codemods: './codemods',
  templateTransform: './astryx/template-transform.ts',
  issuesUrl: 'https://github.com/acme/widgets/issues',
};`,
    },
  ],
  notes: [
    {
      type: 'prose',
      text:
        "Identity — the integration's name and version — comes from the " +
        "package's package.json, not from this manifest. The manifest only " +
        'declares where the CLI finds each kind of artifact.',
    },
    {
      type: 'prose',
      text:
        'Validate a manifest with `astryx validate-integration`. It is checked ' +
        'at the load boundary with a strict schema (parseIntegration): unknown ' +
        'keys are errors, and issuesUrl must be a valid URL.',
    },
  ],
};
