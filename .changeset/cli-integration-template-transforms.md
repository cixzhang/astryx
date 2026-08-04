---
'@astryxdesign/cli': patch
---

[feat] integration template transforms — an integration can reshape the templates the CLI emits as a pure output-layer, without editing any template on disk.

An integration manifest (`astryx.integration.*`) may now declare a `templateTransform` module. When configured, the CLI applies it to the source that `astryx template <id>` shows and `astryx template <id> <path>` scaffolds — including core (OSS) templates — so a wrapper design system can make every template come out wrapped in its shell without the core templates knowing it exists. The on-disk templates are never modified.

- New authoring surface `AstryxTemplateTransform` (exported from `@astryxdesign/cli/authoring`) with a single, footgun-resistant declarative operation for v1: `wrap` (`{component, from, importKind?, props?}`). Naming the wrapper and its module wraps the default-export's returned JSX and adds the matching import as one unit, so a wrap can never emit an un-imported component; wrapping is idempotent (never double-wraps).
- Type-safe props: `AstryxTemplateTransform<WrapperProps>` / `TemplateWrap<WrapperProps>` narrow `wrap.props` to the wrapper's statically-renderable props (`string`/`number`/`boolean`), so typos and non-serializable props are compile errors.
- Applies to page templates by default (`appliesTo: {types}` opts blocks in), never rewrites the transform owner's own templates, and composes across integrations in config order.
- Reuses the codemod engine: declarative steps compile to jscodeshift and run through the shared `validateOutput`/`fixDirectiveCorruption` safety net, so a broken transform can never emit unparseable source — it degrades to the untransformed input, warns on stderr (never in `--json`), and `--json` reports applied packages via `transformedBy`.
- `Project.templateTransforms()` loads/validates each transform under the existing skip+warn policy; `astryx validate-integration` validates the module (`missing_template_transform` / `invalid_template_transform`).

@josephfarina
