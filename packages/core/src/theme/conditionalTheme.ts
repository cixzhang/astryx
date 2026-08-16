// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file conditionalTheme.ts
 * @input The conditions map — `defineTheme`'s second argument
 * @output Resolved conditional layers (media query + tokens + components)
 * @position Theme system utility; consumed by defineTheme and generateThemeRules
 *
 * A conditional theme layer is a set of theme values that apply only where a
 * condition matches. Conditions live in `defineTheme`'s **second argument**,
 * not in the theme object, so the theme object's key set stays closed while
 * the set of conditions a theme can express stays open:
 *
 *     defineTheme(mainTheme, {
 *       mobile: {...},                              // blessed alias
 *       '(min-width: 900px)': {...},                // raw media query
 *       '(prefers-reduced-motion: reduce)': {...},
 *     });
 *
 * A key is either a **blessed alias** from {@link CONDITION_ALIASES} or a raw
 * CSS media query. An alias is nothing but sugar: it resolves through the same
 * map to the same query string, so `mobile` and its expansion emit identical
 * CSS. Aliases exist to carry judgment a raw query cannot — `mobile` means
 * narrow AND touch, so a desktop user dragging their window narrow never gets
 * mobile treatment.
 *
 * Each value is **theme-shaped**: the same axes as the main theme input
 * (`typography`, `color`, `radius`, `motion`, `tokens`, `components`), so an
 * author learns one grammar. Values are resolved through the same helpers the
 * base theme uses, in the same precedence order.
 *
 * "Conditional" (not "media") is deliberate: in Astryx, *media* at the theme
 * layer already means content sitting on an image or video surface — see
 * `onMediaTokens.ts`, `MediaTheme`, `onDark`/`onLight`.
 *
 * Unset means nothing is emitted: no second argument, an empty map, or a
 * condition whose value is `null` produces no layer, so a theme that does not
 * opt in is byte-identical to one defined before conditions existed.
 */

import type {ComponentStyleMap, TokenName, TokenValue} from './defineTheme';
import type {TypographyConfig} from './types';
import type {TypeScaleConfig} from './expandTypeScale';
import {expandTypeScale, generateTypeScaleComponents} from './expandTypeScale';
import {expandMotionScale, type MotionScaleConfig} from './expandMotionScale';
import {expandRadiusScale, type RadiusScaleConfig} from './expandRadiusScale';
import {expandColorScale, type ColorScaleConfig} from './expandColorScale';
import {
  buildFontFamilyTokens,
  buildTypeScaleConfig,
  deepMergeComponents,
} from './themeAxes';

// =============================================================================
// Aliases
// =============================================================================

/**
 * The width below which the `mobile` alias can match, in px.
 *
 * To move it, use a raw query key instead of the alias — see
 * {@link mobileMediaQuery}. Width alone never makes `mobile` match; the
 * pointer must be coarse as well.
 */
export const DEFAULT_MOBILE_BREAKPOINT = 756;

/**
 * Build the media query the `mobile` alias expands to, optionally at a
 * different width. Use it as a computed key to keep the alias's judgment
 * (narrow AND touch) while moving the cutoff:
 *
 * ```ts
 * defineTheme(theme, {[mobileMediaQuery(640)]: {tokens: {...}}});
 * ```
 *
 * Both halves are load-bearing: the width bound keeps it off large touch
 * screens, and `pointer: coarse` keeps it off a narrowed desktop window.
 */
export function mobileMediaQuery(
  breakpoint: number = DEFAULT_MOBILE_BREAKPOINT,
): string {
  return `(max-width: ${breakpoint}px) and (pointer: coarse)`;
}

/**
 * Blessed condition aliases and the media query each expands to.
 *
 * An alias is pure sugar over its query — the emitted CSS is identical either
 * way. The list is deliberately short: an alias earns its place only when the
 * correct query is non-obvious enough that authors would get it wrong.
 */
export const CONDITION_ALIASES = {
  /** Narrow *and* touch — never a desktop window dragged narrow. */
  mobile: mobileMediaQuery(),
  /** Paged output. */
  print: 'print',
} as const;

/** Names of the blessed condition aliases. */
export type ConditionAlias = keyof typeof CONDITION_ALIASES;

/**
 * A condition key: a blessed alias, or any raw CSS media query.
 *
 * The `(string & {})` arm is what keeps both: it accepts an arbitrary query
 * without collapsing the union to `string`, so editors still offer `mobile`
 * and `print` as completions.
 */
export type ConditionKey = ConditionAlias | (string & {});

// =============================================================================
// Types
// =============================================================================

/**
 * Overrides that apply only where a condition matches — a partial theme with
 * the same shape as the main theme input.
 *
 * Every axis is independent: only the axes actually set here generate CSS.
 * Setting `tokens` alone emits token declarations and nothing else.
 */
export interface ConditionalThemeOverrides {
  /** Typography overrides — scale, families, weights. */
  typography?: TypographyConfig;
  /** Color scale overrides. */
  color?: ColorScaleConfig;
  /** Radius scale overrides. */
  radius?: RadiusScaleConfig;
  /** Motion scale overrides. */
  motion?: MotionScaleConfig;
  /** Explicit token overrides — highest precedence within the condition.
   *  Accepts `[light, dark]` tuples exactly as the main theme does. */
  tokens?: Partial<Record<TokenName, TokenValue>>;
  /** Component style overrides. */
  components?: ComponentStyleMap;
}

/**
 * `defineTheme`'s second argument: conditions mapped to the theme values that
 * apply under them.
 *
 * Keys are blessed aliases or raw media queries; `null` means "declared but
 * contributes nothing", which emits no CSS.
 *
 * **Ordering.** A media query adds no specificity, so where two conditions
 * both match, the one written **later in this object** wins — layers are
 * emitted in the map's own key order.
 */
export type ThemeConditions = {
  [K in ConditionKey]?: ConditionalThemeOverrides | null;
};

/**
 * A resolved conditional layer stored on DefinedTheme.
 * @internal
 */
export interface ResolvedConditionalTheme {
  /** The key as authored — an alias name or the raw query. */
  condition: string;
  /** The media query this condition compiles to, without the `@media` keyword. */
  query: string;
  /** Resolved token CSS values — only the tokens this layer sets. */
  tokens: Record<string, string>;
  /** Component style overrides for this layer, if any. */
  components?: ComponentStyleMap;
}

// =============================================================================
// Query validation
// =============================================================================

/**
 * Characters that would let a condition key escape the `@media` prelude it is
 * interpolated into and inject arbitrary rules into the generated stylesheet.
 *
 * Theme files are trusted code, so this is a guard against typos rather than
 * an attacker — but a stray brace silently corrupts every rule after it, and
 * the failure surfaces as unstyled UI rather than as a build error.
 */
const UNSAFE_QUERY_CHARS = /[{}<>;]|\/\*|\*\//;

/**
 * Validate a raw condition key before it is interpolated into `@media`.
 *
 * Only structural sabotage is rejected: a query that is merely wrong
 * (`(max-widht: 700px)`) is still emitted, because CSS already handles an
 * unrecognized feature by never matching, and a build-time allowlist of media
 * features would go stale as browsers add them.
 */
export function assertValidConditionQuery(
  key: string,
  query: string,
): asserts query is string {
  if (query.trim() === '') {
    throw new Error(
      `defineTheme: condition key ${JSON.stringify(key)} is empty. ` +
        `Use a blessed alias (${Object.keys(CONDITION_ALIASES).join(', ')}) or a CSS media query.`,
    );
  }
  if (UNSAFE_QUERY_CHARS.test(query) || /[\r\n]/.test(query)) {
    throw new Error(
      `defineTheme: condition key ${JSON.stringify(key)} is not a usable media query — ` +
        `it contains a character that would break out of the @media block ` +
        `({, }, ;, <, >, or a comment marker).`,
    );
  }
}

/** Expand a condition key to its media query. Aliases resolve; queries pass through. */
export function resolveConditionQuery(key: string): string {
  const alias = (CONDITION_ALIASES as Record<string, string | undefined>)[key];
  const query = alias ?? key;
  assertValidConditionQuery(key, query);
  return query;
}

// =============================================================================
// Resolution
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
 * Resolve one condition's partial theme into tokens + component overrides.
 *
 * Mirrors the base theme's axis precedence exactly: generated values first
 * (color, type scale, radius, motion, font families), explicit `tokens` last.
 */
function resolveOverrides(input: ConditionalThemeOverrides): {
  tokens: Record<string, string>;
  components?: ComponentStyleMap;
} {
  const tokens: Record<string, string> = {};

  const typo: TypographyConfig | undefined = input.typography;
  const typeScaleConfig: TypeScaleConfig | undefined = typo
    ? buildTypeScaleConfig(typo)
    : undefined;

  if (input.color) {
    Object.assign(tokens, expandColorScale(input.color));
  }
  if (typeScaleConfig) {
    Object.assign(tokens, expandTypeScale(typeScaleConfig));
  }
  if (input.radius) {
    Object.assign(tokens, expandRadiusScale(input.radius));
  }
  if (input.motion) {
    Object.assign(tokens, expandMotionScale(input.motion));
  }
  if (typo) {
    Object.assign(tokens, buildFontFamilyTokens(typo));
  }
  // Explicit tokens win over anything generated inside this condition.
  if (input.tokens) {
    for (const [key, value] of Object.entries(input.tokens)) {
      if (value !== undefined) {
        tokens[key] = resolveTokenValue(value);
      }
    }
  }

  let components = input.components;
  if (typeScaleConfig) {
    components = deepMergeComponents(
      generateTypeScaleComponents(typeScaleConfig),
      input.components,
    );
  }

  return {tokens, components};
}

/**
 * Resolve every condition in the map, in the map's own key order.
 *
 * Returns `undefined` when nothing resolves — the opt-in guarantee: a theme
 * with no second argument carries no conditional data and generates no
 * conditional CSS.
 */
export function resolveConditionalThemes(
  conditions?: ThemeConditions | null,
): ResolvedConditionalTheme[] | undefined {
  if (!conditions) {
    return undefined;
  }

  const layers: ResolvedConditionalTheme[] = [];
  for (const [key, overrides] of Object.entries(conditions)) {
    // Absent or null means "no layer" — the same opt-out at any key.
    if (overrides == null) {
      continue;
    }
    const {tokens, components} = resolveOverrides(overrides);
    layers.push({
      condition: key,
      query: resolveConditionQuery(key),
      tokens,
      components,
    });
  }

  return layers.length > 0 ? layers : undefined;
}
