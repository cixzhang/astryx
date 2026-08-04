// Copyright (c) Meta Platforms, Inc. and affiliates.

/** @type {import('@astryxdesign/cli/authoring').ReferenceDoc} */

export const docs = {
  name: 'cli-integrations',
  title: 'CLI Integrations',
  category: 'guide',
  description:
    'Author an npm package that contributes components, templates, and upgrade codemods to Astryx.',

  sections: [
    {
      title: 'Overview',
      category: 'guide',
      content: [
        {
          type: 'prose',
          text: 'An integration is an npm package that contributes components, templates, and/or upgrade codemods to a consumer\'s design-system workflow. Consumers install the package and add it to their `astryx.config`; from then on the integration\'s contributions show up alongside core\'s in the same CLI commands.',
        },
        {
          type: 'prose',
          text: 'The system runs on two files. The consumer writes `astryx.config.{ts,mjs,js}` at their project root to list which packages to load. The author writes `astryx.integration.{ts,mjs,js}` at the package root to declare what the package contributes. This page is the author\'s guide. For the consumer side, run `npx astryx docs getting-started`.',
        },
        {
          type: 'prose',
          text: 'On the consumer side, adding your package is one line:',
        },
        {
          type: 'code',
          lang: 'typescript',
          code: "// astryx.config.ts\nexport default {\n  integrations: ['@acme/astryx-widgets'],\n};",
        },
        {
          type: 'prose',
          text: 'Your components and templates then appear next to core\'s:',
        },
        {
          type: 'code',
          lang: 'bash',
          code: 'astryx component --list --package @acme/astryx-widgets\nastryx component AcmeCarousel --props',
        },
      ],
    },
    {
      title: 'The Integration File',
      category: 'guide',
      content: [
        {
          type: 'prose',
          text: 'To register your package as an integration, add an `astryx.integration.{ts,mjs,js}` file as a sibling of your `package.json`. It tells the CLI where to find your components, templates, and codemods. Identity (name, version) comes from your `package.json`, not this file.',
        },
        {
          type: 'code',
          lang: 'typescript',
          code: "// astryx.integration.ts\nexport default {\n  components: './components',\n  templates: './templates',\n  codemods: './codemods',\n  templateTransform: './astryx/template-transform.ts',\n  issuesUrl: 'https://github.com/acme/widgets/issues',\n};",
        },
        {
          type: 'prose',
          text: 'Every field is optional. Declare only the contribution roots your package ships. There is no factory to call. Write a plain object, and for editor autocomplete and type-checking annotate it with the `AstryxIntegration` type exported from `@astryxdesign/cli/authoring`.',
        },
      ],
    },
    {
      title: 'Components',
      category: 'guide',
      content: [
        {
          type: 'prose',
          text: 'Export your components from your library however you like, and consumers still import them from your package. For each component the CLI should document, ship a `.doc.{ts,mjs,js}` file with the same stem, for example `AcmeCarousel.tsx` alongside `AcmeCarousel.doc.ts`.',
        },
        {
          type: 'code',
          lang: 'typescript',
          code: "// AcmeCarousel.doc.ts\nexport default {\n  type: 'component',\n  name: 'AcmeCarousel',\n  description: 'A carousel that cycles through slides.',\n  // props, usage, examples, ...\n};",
        },
      ],
    },
    {
      title: 'Templates',
      category: 'guide',
      content: [
        {
          type: 'prose',
          text: 'Templates are usually not exported from the package directly. Instead, consumers browse them through the CLI and materialize them into their app. Define a template as a plain object stamped with `type: \'page\'` (full pages) or `type: \'block\'` (smaller chunks) in a `.template.{ts,mjs,js}` file next to the source, for example `AcmeLandingPage.tsx` and `AcmeLandingPage.template.ts`.',
        },
        {
          type: 'code',
          lang: 'typescript',
          code: "// AcmeLandingPage.template.ts\nexport default {\n  type: 'page',\n  // name, description, preview, ...\n};",
        },
        {
          type: 'prose',
          text: 'The CLI needs the template source at consume time, so make sure it is included in your published package. This is typically done via the `exports` key in `package.json`. It also lets the docsite render template previews in the future.',
        },
        {
          type: 'code',
          lang: 'jsonc',
          code: '{\n  "exports": {\n    // ...\n    "./templates/*.tsx": "./templates/*.tsx"\n  }\n}',
        },
        {
          type: 'prose',
          text: 'To verify it resolves, try importing the template component with its `.tsx` extension. An extensionless specifier will not resolve under `moduleResolution: bundler`, and the extensionful export above is what lets this type-check without consumers enabling `allowImportingTsExtensions`.',
        },
        {
          type: 'code',
          lang: 'typescript',
          code: "import('@acme/astryx-widgets/templates/AcmeLandingPage.tsx');",
        },
      ],
    },
    {
      title: 'Template Transforms',
      category: 'guide',
      content: [
        {
          type: 'prose',
          text: 'A template transform reshapes the templates the CLI emits — including core (OSS) templates — as a pure output-layer. It never edits any template on disk; it only rewrites the source the CLI shows (`astryx template <id>`) or scaffolds (`astryx template <id> <path>`). Point the integration file\'s `templateTransform` field at a module that default-exports the transform. This is how a wrapper design system can make every template come out wrapped in its shell without the core templates knowing it exists.',
        },
        {
          type: 'prose',
          text: 'v1 exposes a single, footgun-resistant operation: `wrap`. You name the wrapper component and the module it comes from; the CLI wraps the page\'s returned JSX and adds the matching import as one unit (so a wrap can never emit an un-imported component), and it is idempotent (re-emitting never double-wraps).',
        },
        {
          type: 'code',
          lang: 'typescript',
          code: "// astryx/template-transform.ts\nimport type {AstryxTemplateTransform} from '@astryxdesign/cli/authoring';\nimport type {AppFrameProps} from '@acme/widgets';\n\n// Parameterize with the wrapper's props for a fully type-safe `wrap.props`.\nexport default {\n  description: 'Wraps pages in the Acme app shell so they inherit theming.',\n  wrap: {\n    component: 'AppFrame',\n    from: '@acme/widgets',\n    props: {surface: 'internal'},\n  },\n} satisfies AstryxTemplateTransform<AppFrameProps>;",
        },
        {
          type: 'prose',
          text: 'To wrap in more than one component, pass a stack listed OUTERMOST FIRST. Each wrapper auto-imports itself and can set its own props; the whole stack is applied idempotently.',
        },
        {
          type: 'code',
          lang: 'typescript',
          code: "// <Provider><AppFrame>…</AppFrame></Provider>\nexport default {\n  wrap: [\n    {component: 'Provider', from: '@acme/widgets'},\n    {component: 'AppFrame', from: '@acme/widgets', props: {surface: 'internal'}},\n  ],\n} satisfies AstryxTemplateTransform;",
        },
        {
          type: 'prose',
          text: 'Wrapper `props` may be primitives (`string`/`number`/`boolean`) or JSON-shaped objects/arrays — e.g. `props: {options: {analytics: true, tags: [\'a\', \'b\']}}` renders `options={{analytics: true, tags: [\'a\', \'b\']}}`. They must be statically serializable; functions, `ReactNode`, and references to imported values are intentionally out of scope (a future programmatic escape hatch is the place for those). When you parameterize with the wrapper\'s props type, only its statically-renderable props are offered and typos are compile errors.',
        },
        {
          type: 'prose',
          text: 'Scope with `appliesTo`: `types` (default `[\'page\']`; add `\'block\'` for blocks), `include` / `exclude` (template-id globs, where `*` is a wildcard, e.g. `include: [\'dashboard\', \'marketing/*\']` or `exclude: [\'blank\']`), and `packages` (only templates owned by these packages, e.g. `[\'@astryxdesign/core\']` to touch only the built-in OSS templates). A transform never rewrites its own package\'s templates.',
        },
        {
          type: 'prose',
          text: 'Alterations are surfaced loudly, not silently. When a transform changes a template, the CLI prints a prominent notice to stderr — naming the integration(s), what they wrapped, and your `description` — and points the user to `--no-transforms` to emit the ORIGINAL, unaltered template. The emitted source itself stays clean (no injected comments), and `--json` reports the applied packages via `transformedBy`. Always provide a `description` so that notice can explain what you change and why.',
        },
        {
          type: 'prose',
          text: 'Transforms are validated at the load boundary and dry-run by `astryx validate-integration`; at runtime a broken transform is skipped with a warning rather than breaking the command.',
        },
      ],
    },
    {
      title: 'Codemods',
      category: 'guide',
      content: [
        {
          type: 'prose',
          text: 'Ship codemods so `astryx upgrade` can migrate consumers across breaking changes in your package. Point the integration file\'s `codemods` field at your codemods root, and author each one as a plain object stamped with `type: \'code\'` (transforms source files) or `type: \'config\'` (rewrites the consumer\'s `astryx.config`).',
        },
        {
          type: 'code',
          lang: 'typescript',
          code: "// codemods/v2-rename-prop.ts\nexport default {\n  type: 'code',\n  // title, description, transform, ...\n};",
        },
        {
          type: 'prose',
          text: 'All authoring types are exported from `@astryxdesign/cli/authoring`: `ComponentDoc`, `HookDoc`, and `ReferenceDoc` for docs, `TemplateDoc` for templates, `AstryxTemplateTransform` for template transforms, and `AstryxConfig`, `AstryxIntegration`, and `AstryxCodemod` for the project files. Consumers can also run their own post-codemod hooks, such as a reinstall or rebuild, via `hooks.postCodemod` in their `astryx.config`.',
        },
      ],
    },
    {
      title: 'How It Works',
      category: 'guide',
      content: [
        {
          type: 'prose',
          text: 'Every CLI command loads the consumer\'s `astryx.config`, resolves each listed integration\'s manifest from `node_modules`, and discovers its contributions. Everything is validated against one strict schema at the load boundary: the CLI parses each file through `@astryxdesign/cli/authoring` when it loads it, not when you author it. There are no factories; you write a plain object and stamp its `type`.',
        },
        {
          type: 'prose',
          text: 'Discovery is resilient. A broken or misconfigured integration is skipped with a single non-blocking warning on stderr instead of crashing the CLI, and it never corrupts a `--json` stdout envelope. Everyday commands keep working with the remaining valid contributions.',
        },
        {
          type: 'prose',
          text: 'To inspect problems, run `astryx validate-integration <package>` for a detailed report on one package, or `astryx doctor` for an overall health check of the setup.',
        },
      ],
    },
  ],
};
