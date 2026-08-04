---
'@astryxdesign/cli': patch
---

[feat] Add a self-documenting layer to the CLI: typed, colocated `.doc.mjs` for every command, every `@astryxdesign/cli/api` function, and every authored schema (config, integration, codemod, the response envelope, and the doc-types themselves). Adds the `FunctionDoc`, `SchemaDoc`, `CommandDoc`, and `EnumDoc` authoring types with sealed parsers, a `defineCommand` converter, and a drift harness that keeps the docs in sync with the live CLI.
@josephfarina
