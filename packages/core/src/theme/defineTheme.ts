// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * defineTheme — Create an Astryx theme from a flat token map.
 *
 * Two distribution modes:
 * - Unbuilt: Theme generates CSS and injects a <style> tag at runtime
 * - Built: `astryx theme build` pre-compiles to a CSS file; Theme just
 *   sets the data-astryx-theme attribute
 *
 * Token values can be:
 * - A string: used as-is for both light and dark modes
 * - A [light, dark] tuple: converted to light-dark(light, dark)
 *
 * @example
 * ```tsx
 * const oceanTheme = defineTheme({
 *   name: 'ocean',
 *   tokens: {
 *     '--color-accent': ['#0077B6', '#48CAE4'],    // [light, dark]
 *     '--color-background-surface': ['#F0F8FF', '#0A1628'],
 *     '--radius-container': '16px',                     // same in both modes
 *   },
 *   icons: oceanIcons,
 * });
 *
 * <Theme theme={oceanTheme}>
 *   <App />
 * </Theme>
 * ```
 *
 * SYNC: `DefineThemeInput` is the theme surface. Adding, removing, or renaming
 * a field means updating:
 * - /packages/cli/assets/theme.template.ts (documents every field; the
 *   drift guard is scripts/check-theme-template.test.mjs)
 * - /packages/cli/assets/docs/theme.doc.mjs (`astryx docs theme`)
 */

import type {IconRegistry} from '../Icon/globalIconRegistry';
import type {IndicatorRegistry} from '../Indicator/types';
import type {TypographyConfig} from './types';
import {
  resolveOnMedia,
  type OnMediaOverrides,
  type ResolvedOnMedia,
} from './onMediaTokens';
import {
  resolveConditionalThemes,
  mobileMediaQuery,
  type ConditionalThemeOverrides,
  type ResolvedConditionalTheme,
  type ThemeConditions,
} from './conditionalTheme';
import {buildFontFamilyTokens, buildTypeScaleConfig} from './themeAxes';
import {
  colorDefaults,
  spacingDefaults,
  sizeDefaults,
  borderDefaults,
  focusDefaults,
  radiusDefaults,
  shadowDefaults,
  durationDefaults,
  easeDefaults,
  typographyDefaults,
  textSizeDefaults,
  fontWeightDefaults,
  typeScaleDefaults,
} from './tokens.stylex';
import {
  expandTypeScale,
  generateTypeScaleComponents,
  type TypeScaleConfig,
} from './expandTypeScale';
import {expandMotionScale, type MotionScaleConfig} from './expandMotionScale';
import {expandRadiusScale, type RadiusScaleConfig} from './expandRadiusScale';
import {expandColorScale, type ColorScaleConfig} from './expandColorScale';

import type {DomainTokenName} from './domainTokens';
import {domainTokenDefaults} from './domainTokens';
import type {SyntaxThemeDefinition} from './syntax';
import {registerTheme} from './themeRegistry';
import {deepMergeComponents} from './mergeComponents';

// =============================================================================
// Types
// =============================================================================

/** All valid Astryx core token names */
export type CoreTokenName =
  | keyof typeof colorDefaults
  | keyof typeof spacingDefaults
  | keyof typeof sizeDefaults
  | keyof typeof borderDefaults
  | keyof typeof focusDefaults
  | keyof typeof radiusDefaults
  | keyof typeof shadowDefaults
  | keyof typeof durationDefaults
  | keyof typeof easeDefaults
  | keyof typeof typographyDefaults
  | keyof typeof textSizeDefaults
  | keyof typeof fontWeightDefaults
  | keyof typeof typeScaleDefaults;

/** All valid Astryx token names — core + domain tokens */
export type TokenName = CoreTokenName | DomainTokenName;

/**
 * Token value — either a single string or a [light, dark] tuple.
 * Tuples are converted to CSS light-dark() at theme creation time.
 */
export type TokenValue = string | [light: string, dark: string];

/**
 * CSS property values for a style rule.
 *
 * Keys are camelCase CSS properties with string values, OR pseudo-class
 * selectors (starting with `:`) mapping to nested property objects.
 *
 * Pseudo-class keys generate separate CSS rules with the pseudo appended
 * to the component selector. Supported pseudo-classes include `:hover`,
 * `:focus-visible`, `:active`, `:checked`, `:disabled`, etc.
 *
 * @example
 * ```ts
 * {
 *   borderColor: '#8F9296',
 *   ':hover': { borderColor: 'color-mix(in srgb, #8F9296, black 20%)' },
 *   ':focus-visible': { outline: '2px solid var(--color-accent)' },
 * }
 * ```
 */
export type StyleOverrides = Record<string, string | Record<string, string>>;

/**
 * Component style overrides.
 *
 * Each top-level key is a component name (lowercase). Values are objects
 * mapping style keys to CSS property overrides:
 * - `base` — styles applied to all instances of the component
 * - `prop:value` — styles when a visual prop matches (e.g. `variant:secondary`)
 * - `prop:value+prop:value` — intersection of multiple props
 *
 * The `base` key is optional — omit it to only override specific variants.
 *
 * Style values can include pseudo-class keys (`:hover`, `:focus-visible`, etc.)
 * to override interaction states without CSS custom property escape hatches.
 *
 * @example
 * ```tsx
 * components: {
 *   button: {
 *     base: { fontWeight: '600' },
 *     'variant:secondary': { backgroundColor: 'rgba(0,0,0,0.06)' },
 *     'variant:destructive+size:sm': { padding: '2px 6px' },
 *   },
 *   badge: {
 *     'variant:ghost': { border: '1px solid var(--color-border)' },
 *   },
 *   radio: {
 *     base: {
 *       borderColor: '#8F9296',
 *       ':hover': { borderColor: 'color-mix(in srgb, #8F9296, black 20%)' },
 *     },
 *   },
 * }
 * ```
 */
export type ComponentStyleMap = Record<string, Record<string, StyleOverrides>>;

/** Input to defineTheme */
export interface DefineThemeInput {
  /** Theme name — used for data-astryx-theme attribute and identification */
  name: string;

  /**
   * Base theme to extend. When provided, the new theme starts with everything
   * the base resolved to — tokens, component overrides, icons, indicators, and
   * its `onDark`/`onLight` surfaces — then applies this input on top. The base
   * theme's values have lowest precedence.
   *
   * The result is flat: an extended theme carries its inheritance in its own
   * resolved output, so `astryx theme build` emits one self-contained
   * stylesheet and the base's CSS does not need to be loaded alongside it.
   *
   * Use this to create variant themes that customize only a few aspects
   * (e.g. icons, accent color) without re-specifying the full theme.
   *
   * @example
   * ```tsx
   * import {neutralTheme} from '@astryxdesign/theme-neutral';
   *
   * const myTheme = defineTheme({
   *   name: 'my-brand',
   *   extends: neutralTheme,
   *   icons: myIcons,
   *   tokens: { '--color-accent': '#FF0000' },
   * });
   * ```
   */
  extends?: DefinedTheme;
  /**
   * Unified typography configuration — fonts, scale, and weights.
   *
   * Scale controls sizing; roles (body, heading, code) declare
   * fonts, fallbacks, and weights. Heading inherits from body if omitted.
   *
   * Font loading is the consumer's responsibility — add a <link> or
   * @import for your fonts before rendering the theme.
   *
   * @example
   * ```tsx
   * typography: {
   *   scale: { base: 14, ratio: 1.2 },
   *   body: { family: 'Geist', fallbacks: '-apple-system, sans-serif' },
   *   heading: { weight: 'semibold', weights: { 3: 'bold', 4: 'bold' } },
   *   code: { family: 'Geist Mono', fallbacks: '"SF Mono", monospace' },
   * }
   * ```
   */
  typography?: TypographyConfig;
  /**
   * Motion configuration. Computes duration min/max variants from
   * base values and a scaling ratio: min = base × ratio, max = base / ratio.
   *
   * Explicit `tokens` overrides take precedence over motion-generated values.
   *
   * @example
   * ```
   * motion: { fast: 175, medium: 410, slow: 975, ratio: 0.75 }
   *
   * // Suggested starting points:
   * //   Snappy:    { fast: 100, medium: 250, ratio: 0.75 }
   * //   Default:   { fast: 175, medium: 410, slow: 975, ratio: 0.75 }
   * //   Cinematic: { fast: 200, medium: 500, slow: 1200, ratio: 0.7 }
   * ```
   */
  motion?: MotionScaleConfig;
  /**
   * Radius configuration. Generates radius token overrides
   * from a base unit and multiplier.
   *
   * --radius-none and --radius-full are always fixed (never affected by multiplier).
   * --radius-inner through --radius-page = base * step * multiplier.
   *
   * When omitted, themes use the hardcoded defaults (base=4, multiplier=1).
   * Explicit `tokens` overrides take precedence over radius-generated values.
   *
   * @example
   * ```tsx
   * radius: { base: 4, multiplier: 1 }
   *
   * // Sharp/brutalist — all radii become 0
   * radius: { base: 4, multiplier: 0 }
   * ```
   */
  radius?: RadiusScaleConfig;
  /**
   * Color scale configuration. Generates color token overrides from a
   * single accent color using the HCT perceptual color model.
   *
   * Only generates tokens derivable from the accent — status colors,
   * categorical hues, and fixed tokens (on-dark/on-light) use defaults.
   * Explicit `tokens` entries always take precedence.
   *
   * `accent` is optional — omit it for a neutral-only theme, which keeps
   * the default accent tokens and only themes the neutrals.
   *
   * @example
   * ```tsx
   * color: { accent: '#0064E0', neutralStyle: 'cool', contrast: 'standard' }
   *
   * // Neutral-only — accent tokens stay at their defaults
   * color: { neutralStyle: 'warm' }
   * ```
   */
  color?: ColorScaleConfig;
  /** Token overrides — flat map of CSS custom property names to values.
   *  Values can be a string or [light, dark] tuple.
   *  Only include tokens you want to override; defaults fill the rest. */
  tokens?: Partial<Record<TokenName, TokenValue>>;
  /**
   * Component style overrides — keyed by component name (lowercase).
   * Each entry maps style keys to CSS property overrides, scoped under
   * the theme's data-astryx-theme attribute via @scope.
   *
   * Use `prop:value` keys to target specific visual props. New values
   * not in the base type are automatically detected by `astryx theme build`
   * and generate TypeScript module augmentations for type-safe extensibility.
   *
   * @example
   * ```tsx
   * components: {
   *   button: {
   *     base: { fontWeight: '600' },
   *     'variant:secondary': { backgroundColor: '...' },
   *     'variant:primary-muted': { backgroundColor: '#ECF5FF' }, // new — generates augmentation
   *   },
   *   banner: {
   *     'status:neutral': { backgroundColor: 'var(--color-background-muted)' }, // new status
   *   },
   * }
   * ```
   */
  components?: ComponentStyleMap;
  /** Icon registry — maps semantic icon names to React nodes */
  icons?: Partial<IconRegistry>;
  /**
   * Indicator overrides — replaces the components that draw stateful control
   * visuals with the theme's own, by name.
   *
   * Replacement is by indicator name, not per call site, so a single entry
   * reaches every component that draws that indicator: mapping `check` to
   * `RadioIndicator` gives radio visuals to every single-selection mark in the
   * app.
   *
   * Each entry is checked against its indicator's family, so a replacement
   * must accept the states that family passes.
   */
  indicators?: IndicatorRegistry;
  /**
   * Default syntax highlighting theme for code components.
   * Sets --color-syntax-* tokens at the theme root. Can be overridden
   * per-region (or per-instance) by wrapping in SyntaxTheme.
   *
   * @example
   * ```tsx
   * import {dracula} from '@astryxdesign/core/theme/syntax';
   * defineTheme({ name: 'my-theme', syntax: dracula, ... })
   * ```
   */
  syntax?: SyntaxThemeDefinition;
  /**
   * Overrides for content on a dark surface (e.g. inverted toast,
   * dark tooltip). Accepts token and component overrides — same shape
   * as the main theme. Token defaults are generated if omitted.
   *
   * Used by `<MediaTheme surface="dark">` to set semantic tokens
   * and component styles so children render correctly against a dark
   * background.
   *
   * @example
   * ```tsx
   * onDark: {
   *   tokens: { '--color-accent': '#90CAF9' },
   *   components: {
   *     button: { 'variant:ghost': { borderWidth: '1px' } },
   *   },
   * }
   * ```
   */
  onDark?: OnMediaOverrides;
  /**
   * Overrides for content on a light surface. Same shape as `onDark`
   * but for the inverse case (e.g. dark-mode page with a light popover).
   */
  onLight?: OnMediaOverrides;
  /**
   * @deprecated Conditions moved to `defineTheme`'s second argument:
   * `defineTheme(theme, {mobile: {...}})`. Kept only so themes written against
   * the earlier flat form keep building; the second argument always wins.
   */
  breakpoints?: {mobile?: number};
  /**
   * @deprecated Conditions moved to `defineTheme`'s second argument:
   * `defineTheme(theme, {mobile: {...}})`.
   */
  mobile?: ConditionalThemeOverrides | null;
}

/** A defined theme — ready to pass to <Theme> */
export interface DefinedTheme {
  /** Theme name */
  name: string;
  /** Token overrides — only the tokens the consumer specified */
  tokens: Record<string, string>;
  /** Component style overrides */
  components?: ComponentStyleMap;
  /** Icon registry */
  icons?: Partial<IconRegistry>;
  /** Indicator overrides for stateful control visuals, keyed by name */
  indicators?: IndicatorRegistry;
  /** Whether this theme has been pre-compiled by theme build CLI */
  __built?: true;
  /**
   * Raw input tokens preserved from defineTheme() input.
   * Keeps [light, dark] tuples intact for programmatic access
   * (e.g. data viz, canvas rendering) without parsing light-dark() strings.
   * @internal
   */
  __inputTokens?: Partial<Record<string, TokenValue>>;
  /**
   * Resolved on-media token overrides for dark surfaces.
   * Generated by defineTheme from defaults + user onDark overrides.
   * Used by MediaTheme and generateThemeRules.
   * @internal
   */
  __onDark?: ResolvedOnMedia;
  /**
   * Resolved on-media overrides for light surfaces.
   * @internal
   */
  __onLight?: ResolvedOnMedia;
  /**
   * Resolved conditional theme layers (currently `mobile`), each carrying the
   * media query it compiles to plus the tokens and component overrides that
   * apply inside it. Absent when the theme declares no conditions — that
   * absence is what keeps the feature opt-in.
   * @internal
   */
  __conditional?: ResolvedConditionalTheme[];
}

// =============================================================================
// All defaults merged into a single flat map
// =============================================================================

/** All Astryx token defaults as a flat map. Useful for resolving full token sets. */
export const tokenDefaults: Record<string, string> = {
  ...colorDefaults,
  ...spacingDefaults,
  ...sizeDefaults,
  ...borderDefaults,
  ...focusDefaults,
  ...radiusDefaults,
  ...shadowDefaults,
  ...durationDefaults,
  ...easeDefaults,
  ...typographyDefaults,
  ...textSizeDefaults,
  ...fontWeightDefaults,
  ...typeScaleDefaults,
  ...domainTokenDefaults,
};

// =============================================================================
// defineTheme
// =============================================================================

/**
 * Resolve a token value to a CSS string.
 * - String values pass through as-is
 * - [light, dark] tuples become light-dark(light, dark)
 */
function resolveTokenValue(value: TokenValue): string {
  if (Array.isArray(value)) {
    return `light-dark(${value[0]}, ${value[1]})`;
  }
  return value;
}

/**
 * Deep-merge helpers, font resolution, and type-scale config construction are
 * shared with the conditional theme layer — see ./themeAxes.ts.
 */

/**
 * Describe a rejected `extends` value for the error message — enough to tell a
 * missed import (`undefined`) from a module namespace or a plain object.
 */
function describeBadBase(value: unknown): string {
  if (value === undefined) {
    return 'undefined';
  }
  if (value === null) {
    return 'null';
  }
  if (typeof value !== 'object') {
    return typeof value;
  }
  const keys = Object.keys(value);
  return `an object with keys [${keys.slice(0, 4).join(', ')}${keys.length > 4 ? ', …' : ''}]`;
}

/**
 * Create an Astryx theme.
 *
 * Pass only the tokens you want to override — everything else
 * inherits from the Astryx defaults.
 *
 * When `typography.scale` is provided, it generates typography token overrides
 * that are merged into the token map. Explicit `tokens` entries take
 * precedence over generated values.
 *
 * @param input The theme itself. Its key set is closed — every axis it accepts
 *   is a property of the theme, not of a context the theme applies in.
 * @param conditions Optional map of conditions to the theme values that apply
 *   under them. Keys are blessed aliases (`mobile`, `print`) or raw CSS media
 *   queries; each value is theme-shaped. Where two conditions both match, the
 *   one written later wins.
 *
 * @example
 * ```ts
 * defineTheme(
 *   {name: 'acme', typography: {scale: {base: 14, ratio: 1.2}}},
 *   {
 *     mobile: {typography: {scale: {base: 16, ratio: 1.2}}},
 *     '(prefers-reduced-motion: reduce)': {motion: {fast: 0, medium: 0}},
 *   },
 * );
 * ```
 */
export function defineTheme(
  input: DefineThemeInput,
  conditions?: ThemeConditions | null,
): DefinedTheme {
  const tokens: Record<string, string> = {};

  // 0. Pre-seed from base theme when `extends` is provided (lowest precedence).
  // A base that is not a theme is refused rather than ignored: `extends` used
  // to inherit nothing when its value was undefined, which is what a named
  // import silently resolving to the wrong module hands over, and the theme
  // then built into a plausible-looking stylesheet missing everything it was
  // supposed to inherit.
  if ('extends' in input && !isDefinedTheme(input.extends)) {
    throw new Error(
      `defineTheme("${input.name}"): \`extends\` must be a theme from defineTheme(), got ${describeBadBase(input.extends)}. ` +
        `Check that the import naming your base theme resolves to its source and exports that name — ` +
        `a generated \`<theme>.js\` artifact sitting next to the source exports \`<name>Theme\`, not the source's own export.`,
    );
  }
  const base = input.extends;
  if (base) {
    for (const [key, value] of Object.entries(base.tokens)) {
      tokens[key] = value;
    }
  }

  // Build typeScale config from typography if present
  const typo = input.typography;
  const typeScaleConfig: TypeScaleConfig | undefined = typo
    ? buildTypeScaleConfig(typo)
    : undefined;

  // 1. Apply color-generated tokens (lowest precedence for colors)
  if (input.color) {
    const colorTokens = expandColorScale(input.color);
    for (const [key, value] of Object.entries(colorTokens)) {
      tokens[key] = value;
    }
  }

  // 1a. Apply typeScale-generated tokens (lowest precedence for type)
  if (typeScaleConfig) {
    const typeScaleTokens = expandTypeScale(typeScaleConfig);
    for (const [key, value] of Object.entries(typeScaleTokens)) {
      tokens[key] = value;
    }
  }

  // 1b. Apply radius-generated tokens (lowest precedence for radius)
  if (input.radius) {
    const radiusTokens = expandRadiusScale(input.radius);
    for (const [key, value] of Object.entries(radiusTokens)) {
      tokens[key] = value;
    }
  }

  // 1c. Apply motion-generated tokens (same precedence as typeScale)
  if (input.motion) {
    const motionTokens = expandMotionScale(input.motion);
    for (const [key, value] of Object.entries(motionTokens)) {
      tokens[key] = value;
    }
  }

  // 1d. Apply typography font family tokens
  if (typo) {
    Object.assign(tokens, buildFontFamilyTokens(typo));
  }

  // 1e. Apply syntax theme tokens (before explicit overrides)
  if (input.syntax) {
    const syntaxMap = input.syntax.tokens;
    const prefix = '--color-syntax-';
    for (const [key, value] of Object.entries(syntaxMap)) {
      tokens[prefix + key] = value;
    }
  }

  // 2. Apply explicit token overrides (highest precedence — overwrites generated tokens)
  if (input.tokens) {
    for (const [key, value] of Object.entries(input.tokens)) {
      if (value !== undefined) {
        tokens[key] = resolveTokenValue(value);
      }
    }
  }

  // 3. Generate component overrides: base (lowest) → typeScale → explicit (highest)
  let components = input.components;
  if (typeScaleConfig) {
    const generated = generateTypeScaleComponents(typeScaleConfig);
    components = deepMergeComponents(generated, input.components);
  }
  if (base?.components) {
    components = deepMergeComponents(base.components, components);
  }

  // 4. Resolve on-media token overrides (base's resolved surface, then
  // defaults, then this theme's own overrides)
  const __onDark = resolveOnMedia('dark', input.onDark, base?.__onDark);
  const __onLight = resolveOnMedia('light', input.onLight, base?.__onLight);

  // 4a. Resolve conditional layers. Undefined when none are declared, so a
  // theme that does not opt in carries no conditional data at all.
  //
  // The deprecated root keys are read only when no second argument is given,
  // so the flat form keeps building while it is being migrated away from.
  // Delete this fallback and the root keys together — nothing else reads them.
  const legacyConditions: ThemeConditions | undefined =
    input.mobile != null
      ? {[mobileMediaQuery(input.breakpoints?.mobile)]: input.mobile}
      : undefined;
  const __conditional = resolveConditionalThemes(
    conditions ?? legacyConditions,
  );

  // 5. Merge icons — input icons override base icons
  const icons =
    input.icons && base?.icons
      ? {...base.icons, ...input.icons}
      : (input.icons ?? base?.icons);

  // Indicator overrides merge by name, like icons: a child theme replacing one
  // indicator keeps the ones its base replaced.
  const indicators =
    input.indicators && base?.indicators
      ? {...base.indicators, ...input.indicators}
      : (input.indicators ?? base?.indicators);

  const theme: DefinedTheme = {
    name: input.name,
    tokens,
    components,
    icons,
    indicators,
    __inputTokens:
      base?.__inputTokens || input.tokens
        ? {...base?.__inputTokens, ...input.tokens}
        : undefined,
    __onDark,
    __onLight,
    // Spread rather than assign so a theme without conditions has no
    // `__conditional` key at all, not a key holding undefined.
    ...(__conditional ? {__conditional} : {}),
  };

  registerTheme(theme);
  return theme;
}

// =============================================================================
// CSS generation — re-exported from ./generateThemeRules.ts
// =============================================================================

export {
  generateThemeRules,
  generateThemeRulesSplit,
  generateOnMediaCSS,
  generateConditionalCSS,
  generateThemeCSS,
  type ThemeRulesSplit,
  type ThemeCSSOutput,
} from './generateThemeRules';

export {
  DEFAULT_MOBILE_BREAKPOINT,
  CONDITION_ALIASES,
  mobileMediaQuery,
  resolveConditionQuery,
  type ConditionAlias,
  type ConditionKey,
  type ConditionalThemeOverrides,
  type ResolvedConditionalTheme,
  type ThemeConditions,
} from './conditionalTheme';

// =============================================================================
// Type guard
// =============================================================================

/** Check if a theme object was created with defineTheme */
export function isDefinedTheme(theme: unknown): theme is DefinedTheme {
  return (
    typeof theme === 'object' &&
    theme !== null &&
    'name' in theme &&
    'tokens' in theme &&
    !('styles' in theme)
  );
}
