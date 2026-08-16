// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file themeAxes.ts
 * @input Theme axis configs from defineTheme input (typography, components)
 * @output Type-scale config, font-family tokens, merged component style maps
 * @position Theme system utility; consumed by defineTheme and conditionalTheme
 *
 * Shared helpers for turning the declarative axes of a theme input into the
 * lower-level shapes the expanders and the CSS generator consume. Extracted
 * from defineTheme so the conditional theme layer (`mobile`) resolves its
 * partial theme through exactly the same code as the base theme — one
 * implementation, no drift between the two.
 */

import type {TypographyConfig, FontWeight} from './types';
import type {TypeScaleConfig} from './expandTypeScale';

/**
 * Resolve a FontWeight name to a var() reference.
 * Named weights map to var(--font-weight-*); raw values pass through.
 */
export function resolveFontWeight(weight: FontWeight): string {
  const named: Record<string, string> = {
    normal: 'var(--font-weight-normal)',
    medium: 'var(--font-weight-medium)',
    semibold: 'var(--font-weight-semibold)',
    bold: 'var(--font-weight-bold)',
  };
  return named[weight] ?? weight;
}

/**
 * Build the full CSS font-family value from family + fallbacks.
 * Quotes the family name if it contains spaces.
 */
export function buildFontFamily(
  family?: string,
  fallbacks?: string,
): string | undefined {
  if (!family) {
    return undefined;
  }
  const quoted = family.includes(' ') ? `"${family}"` : family;
  if (fallbacks) {
    return `${quoted}, ${fallbacks}`;
  }
  return quoted;
}

/**
 * Build the type-scale config from a typography config.
 *
 * Returns undefined when the typography config has no `scale` — font families
 * and weights alone do not produce a scale.
 */
export function buildTypeScaleConfig(
  typo: TypographyConfig,
): TypeScaleConfig | undefined {
  if (!typo.scale) {
    return undefined;
  }

  // Collect weight overrides from typography roles
  const headingWeights: Partial<Record<1 | 2 | 3 | 4 | 5 | 6, string>> = {};
  const headingRole = typo.heading;
  if (headingRole?.weights) {
    for (const [level, w] of Object.entries(headingRole.weights)) {
      if (w) {
        headingWeights[Number(level) as 1 | 2 | 3 | 4 | 5 | 6] =
          resolveFontWeight(w);
      }
    }
  }
  // Default heading weight from role
  const defaultHeadingWeight = headingRole?.weight
    ? resolveFontWeight(headingRole.weight)
    : undefined;
  if (defaultHeadingWeight) {
    for (let i = 1; i <= 6; i++) {
      if (!(i in headingWeights)) {
        headingWeights[i as 1 | 2 | 3 | 4 | 5 | 6] = defaultHeadingWeight;
      }
    }
  }

  // Text weight overrides from roles
  const textWeights: Partial<Record<string, string>> = {};
  if (typo.body?.weight) {
    textWeights.body = resolveFontWeight(typo.body.weight);
  }
  if (typo.code?.weight) {
    textWeights.code = resolveFontWeight(typo.code.weight);
  }

  return {
    base: typo.scale.base,
    ratio: typo.scale.ratio,
    weights: {
      ...(Object.keys(headingWeights).length > 0
        ? {heading: headingWeights}
        : {}),
      ...(Object.keys(textWeights).length > 0 ? {text: textWeights} : {}),
    },
  };
}

/**
 * Build the `--font-family-*` token overrides implied by a typography config.
 * Heading inherits from body when it declares no family of its own.
 */
export function buildFontFamilyTokens(
  typo: TypographyConfig,
): Record<string, string> {
  const tokens: Record<string, string> = {};

  const bodyFamily = buildFontFamily(typo.body?.family, typo.body?.fallbacks);
  const headingFamily =
    buildFontFamily(typo.heading?.family, typo.heading?.fallbacks) ??
    bodyFamily;
  const codeFamily = buildFontFamily(typo.code?.family, typo.code?.fallbacks);

  if (bodyFamily) {
    tokens['--font-family-body'] = bodyFamily;
  }
  if (headingFamily) {
    tokens['--font-family-heading'] = headingFamily;
  }
  if (codeFamily) {
    tokens['--font-family-code'] = codeFamily;
  }

  return tokens;
}
