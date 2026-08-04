// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * Public type surface for an Astryx template transform — the module an
 * integration points its manifest `templateTransform` at (sibling-resolved from
 * `astryx.integration.{ts,mjs,js}`).
 *
 * A template transform is a PURE OUTPUT-LAYER over the templates the CLI emits.
 * It never edits the on-disk template source; it only reshapes the string the
 * CLI shows (`astryx template <id>`) or scaffolds (`astryx template <id>
 * <path>`). This lets an integration (e.g. an internal design-system wrapper)
 * reshape every OSS template with the core templates staying oblivious to it.
 *
 * The surface is intentionally TINY and declarative. v1 exposes a single
 * operation — `wrap` — which is footgun-resistant by construction: the CLI
 * builds the syntax and the import, so an author can't emit invalid TSX, forget
 * the wrapper's import, or double-wrap. The shape is additive; more operations
 * can be introduced later without breaking existing manifests.
 *
 * Authors write a plain object against {@link AstryxTemplateTransform} and
 * default-export it; the CLI validates it via `parseTemplateTransform` at the
 * load boundary.
 */

/**
 * A statically-declarable JSX attribute value for a wrapper prop. Primitives, or
 * JSON-shaped objects/arrays (rendered as object/array literals). Deliberately
 * excludes functions, `ReactNode`, and references to imported values — those are
 * not statically expressible and belong in the programmatic escape hatch.
 */
export type TemplateWrapPropValue =
  | string
  | number
  | boolean
  | null
  | TemplateWrapPropValue[]
  | {[key: string]: TemplateWrapPropValue};

/**
 * The subset of a component's props that can be expressed as a STATIC JSX
 * attribute literal — props whose type is assignable to {@link
 * TemplateWrapPropValue} (primitives + JSON objects/arrays). Non-serializable
 * props (functions, `ReactNode`, class instances) are excluded from the allowed
 * keys entirely, so a `wrap` can only set props it can actually render, and
 * typos / wrong value types are compile errors.
 */
export type StaticProps<P> = {
  [
    K in keyof P as NonNullable<P[K]> extends TemplateWrapPropValue ? K : never
  ]?: Extract<P[K], TemplateWrapPropValue>;
};

/**
 * Declarative wrap: wrap the template's default-export returned JSX in a
 * component. Idempotent — a template already wrapped in {@link component} is left
 * untouched. The wrapper's import is ALWAYS added automatically.
 *
 * Parameterize with the wrapper's props type for fully type-safe {@link props}:
 * `TemplateWrap<MetaAppFrameProps>`.
 *
 * @template WrapperProps the wrapper component's props (for typed `props`)
 */
export interface TemplateWrap<
  WrapperProps = Record<string, TemplateWrapPropValue>,
> {
  /** Component name to wrap the returned JSX with (e.g. `'AppFrame'`). */
  component: string;
  /**
   * Module specifier the wrapper is imported from (e.g. `'@xds/meta'`). Wrapping
   * ALWAYS adds the matching import automatically — `component` + `from` is a
   * single unit, so a wrap can never emit an un-imported component. The import
   * is merged/deduped if one already exists.
   */
  from: string;
  /** Import style for {@link component}. Default `'named'`. */
  importKind?: 'named' | 'default';
  /**
   * Static props to set on the wrapper. Strings render as string literals
   * (`surface="internal"`); numbers and booleans render as expressions
   * (`count={3}`, `flag={true}`); `true` renders as a bare attribute. Typed
   * against {@link WrapperProps} when provided.
   */
  props?: StaticProps<WrapperProps>;
}

/** Which templates a transform applies to. */
export interface TemplateTransformScope {
  /**
   * Template kinds the transform applies to. Default `['page']` — the headline
   * case is wrapping full-page templates. Include `'block'` to also transform
   * block templates.
   */
  types?: Array<'page' | 'block'>;
  /**
   * Only apply to templates whose id matches one of these globs (`*` is a
   * wildcard), e.g. `['dashboard', 'login-*', 'marketing/*']`. When set,
   * templates that match none are skipped. Combine with {@link exclude}.
   */
  include?: string[];
  /**
   * Never apply to templates whose id matches one of these globs. Takes
   * precedence over {@link include}.
   */
  exclude?: string[];
  /**
   * Only apply to templates owned by one of these packages, e.g.
   * `['@astryxdesign/core']` to transform ONLY the built-in OSS templates and
   * leave other integrations' templates alone.
   */
  packages?: string[];
}

/** Context describing the template currently being emitted (used for scoping). */
export interface AstryxTemplateContext {
  /** Template kind. */
  type: 'page' | 'block';
  /** Stable template id (e.g. `'dashboard'` or `'marketing/hero'`). */
  id: string;
  /** Owning package (`'@astryxdesign/core'` for built-in templates). */
  package: string;
}

/**
 * The definition an author writes for a template transform (default export).
 * Parameterize with the wrapper's props type for a fully type-safe `wrap.props`:
 * `satisfies AstryxTemplateTransform<MetaAppFrameProps>`.
 *
 * @template WrapperProps the `wrap` component's props (for typed `wrap.props`)
 */
export interface AstryxTemplateTransform<
  WrapperProps = Record<string, TemplateWrapPropValue>,
> {
  /**
   * A short human explanation of WHAT this transform does and WHY, shown in the
   * CLI's alteration notice (e.g. "Wraps pages in the Meta app shell + provider
   * so they inherit internal theming and analytics."). Strongly recommended:
   * it's what a consumer reads when the CLI tells them a template was altered.
   */
  description?: string;
  /** Which templates to apply to. Default: page templates only. */
  appliesTo?: TemplateTransformScope;
  /**
   * Wrap the default-export JSX in a component, or in a STACK of components
   * listed OUTERMOST FIRST. Every wrapper auto-imports itself and can set its
   * own props. Wrapping the whole stack is idempotent (guarded by the outermost
   * wrapper), so re-emitting never double-wraps.
   *
   * @example Single wrapper
   * ```
   * wrap: { component: 'AppFrame', from: '@xds/meta' }
   * ```
   * @example Provider + shell -> <MetaProvider><AppFrame>…</AppFrame></MetaProvider>
   * ```
   * wrap: [
   *   { component: 'MetaProvider', from: '@xds/meta' },
   *   { component: 'AppFrame', from: '@xds/meta', props: { surface: 'internal' } },
   * ]
   * ```
   */
  wrap: TemplateWrap<WrapperProps> | TemplateWrap[];
}
