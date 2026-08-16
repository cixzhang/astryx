// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * Tests for conditional theme layers — `defineTheme`'s second argument, whose
 * values apply only where their condition matches.
 *
 * The contract these pin, in order of how load-bearing it is:
 *   - no second argument emits NOTHING, so existing themes are byte-identical;
 *   - a key is a blessed alias or a raw media query, and an alias is nothing
 *     but sugar for the query it expands to;
 *   - `mobile` means narrow AND touch — never a width-only query;
 *   - each axis is independent — only what the author set generates CSS;
 *   - a condition value is theme-shaped, tuples included;
 *   - inside a matching condition the conditional value wins, and outside it
 *     the base theme is untouched;
 *   - between two matching conditions, the later key wins.
 */

import {describe, it, expect} from 'vitest';
import {
  defineTheme,
  generateThemeCSS,
  generateConditionalCSS,
} from './defineTheme';
import {
  CONDITION_ALIASES,
  DEFAULT_MOBILE_BREAKPOINT,
  mobileMediaQuery,
} from './conditionalTheme';

const MOBILE_QUERY = '@media (max-width: 756px) and (pointer: coarse)';

function allCSS(theme: Parameters<typeof generateThemeCSS>[0]): string {
  const {prose, component} = generateThemeCSS(theme);
  return [prose, component].filter(Boolean).join('\n\n');
}

describe('conditional theme — unset means nothing is emitted', () => {
  it('produces no conditional data when no condition is declared', () => {
    const theme = defineTheme({name: 'no-conditions', tokens: {}});
    expect(theme.__conditional).toBeUndefined();
    expect('__conditional' in theme).toBe(false);
  });

  it('produces no conditional data for an explicit null', () => {
    expect(
      defineTheme({name: 'null-arg'}, null).__conditional,
    ).toBeUndefined();
    expect(
      defineTheme({name: 'null-mobile'}, {mobile: null}).__conditional,
    ).toBeUndefined();
    expect(defineTheme({name: 'empty-map'}, {}).__conditional).toBeUndefined();
  });

  it('emits no media block, empty or otherwise, with no second argument', () => {
    const css = allCSS(
      defineTheme({
        name: 'plain',
        typography: {scale: {base: 14, ratio: 1.2}},
        tokens: {'--color-accent': '#0064E0'},
      }),
    );
    expect(css).not.toContain('@media');
    expect(css).not.toContain('pointer: coarse');
  });

  it('leaves a theme byte-identical to the same theme with no conditions', () => {
    const input = {
      name: 'identical',
      typography: {scale: {base: 14, ratio: 1.2}},
      tokens: {'--color-accent': '#0064E0'},
      components: {button: {base: {fontWeight: '600'}}},
    } as const;

    const withoutArg = allCSS(defineTheme({...input}));

    expect(allCSS(defineTheme({...input}, {}))).toBe(withoutArg);
    expect(allCSS(defineTheme({...input}, null))).toBe(withoutArg);
    expect(allCSS(defineTheme({...input}, {mobile: null}))).toBe(withoutArg);
    expect(allCSS(defineTheme({...input}, {mobile: undefined}))).toBe(
      withoutArg,
    );
  });

  it('emits nothing for a declared but empty condition', () => {
    const theme = defineTheme({name: 'empty-mobile'}, {mobile: {}});
    // The layer exists (the author asked for it) but contributes no rules.
    expect(theme.__conditional).toHaveLength(1);
    expect(generateConditionalCSS(theme)).toEqual({prose: '', component: ''});
    expect(allCSS(theme)).not.toContain('@media');
  });
});

describe('conditional theme — keys are aliases or raw queries', () => {
  it('compiles the mobile alias to narrow AND touch, never width alone', () => {
    const css = allCSS(
      defineTheme(
        {name: 'mobile-query'},
        {mobile: {tokens: {'--spacing-4': '12px'}}},
      ),
    );

    expect(css).toContain(MOBILE_QUERY);
    // The touch half is load-bearing: a desktop user dragging their window
    // narrow must not match.
    expect(css).toContain('(pointer: coarse)');
    expect(css).not.toMatch(/@media \(max-width: \d+px\)\s*\{/);
  });

  it('defaults the mobile breakpoint to 756px', () => {
    expect(DEFAULT_MOBILE_BREAKPOINT).toBe(756);
    expect(mobileMediaQuery()).toBe('(max-width: 756px) and (pointer: coarse)');
    expect(CONDITION_ALIASES.mobile).toBe(mobileMediaQuery());
  });

  it('treats an alias as pure sugar for its query', () => {
    const viaAlias = generateConditionalCSS(
      defineTheme({name: 'sugar'}, {mobile: {tokens: {'--spacing-4': '12px'}}}),
    );
    const viaQuery = generateConditionalCSS(
      defineTheme(
        {name: 'sugar'},
        {[CONDITION_ALIASES.mobile]: {tokens: {'--spacing-4': '12px'}}},
      ),
    );
    expect(viaAlias).toEqual(viaQuery);
  });

  it('accepts an arbitrary media query as a key', () => {
    const css = allCSS(
      defineTheme(
        {name: 'raw'},
        {
          '(min-width: 900px)': {tokens: {'--spacing-4': '20px'}},
          '(prefers-reduced-motion: reduce)': {
            tokens: {'--duration-fast': '0ms'},
          },
          '(forced-colors: active)': {
            components: {button: {base: {borderWidth: '1px'}}},
          },
        },
      ),
    );

    expect(css).toContain('@media (min-width: 900px)');
    expect(css).toContain('@media (prefers-reduced-motion: reduce)');
    expect(css).toContain('@media (forced-colors: active)');
  });

  it('reaches a custom breakpoint through a computed alias expansion', () => {
    const css = allCSS(
      defineTheme(
        {name: 'custom-bp'},
        {[mobileMediaQuery(640)]: {tokens: {'--spacing-4': '12px'}}},
      ),
    );
    expect(css).toContain('@media (max-width: 640px) and (pointer: coarse)');
    expect(css).not.toContain('756px');
  });

  it('rejects a key that would break out of the @media block', () => {
    expect(() =>
      defineTheme({name: 'inject'}, {'x} :root{color:red} @media all': {}}),
    ).toThrow(/not a usable media query/);
    expect(() => defineTheme({name: 'blank'}, {'  ': {}})).toThrow(/is empty/);
  });

  it('passes a merely wrong query through — CSS already never matches it', () => {
    const css = allCSS(
      defineTheme(
        {name: 'typo'},
        {'(max-widht: 700px)': {tokens: {'--spacing-4': '12px'}}},
      ),
    );
    expect(css).toContain('@media (max-widht: 700px)');
  });

  it('scopes conditional rules to the theme, like the base rules', () => {
    const {component} = generateConditionalCSS(
      defineTheme({name: 'scoped'}, {mobile: {tokens: {'--spacing-4': '12px'}}}),
    );
    expect(component).toContain('@media (max-width: 756px)');
    expect(component).toContain('@scope ([data-astryx-theme="scoped"])');
    expect(component).toContain('to ([data-astryx-theme])');
    expect(component).toContain('--spacing-4: 12px;');
  });
});

describe('conditional theme — axes are independent', () => {
  it('emits only token declarations when only tokens are set', () => {
    const {component} = generateConditionalCSS(
      defineTheme(
        {name: 'tokens-only', typography: {scale: {base: 14, ratio: 1.2}}},
        {mobile: {tokens: {'--spacing-4': '12px'}}},
      ),
    );

    expect(component).toContain('--spacing-4: 12px;');
    // No typography anywhere in the conditional block: not the generated
    // component rules, not the tokens, even though the base theme has a scale.
    expect(component).not.toContain('.astryx-text');
    expect(component).not.toContain('.astryx-heading');
    expect(component).not.toContain('--text-body-size');
  });

  it('emits typography tokens and component rules when typography is set', () => {
    const theme = defineTheme(
      {name: 'typo-mobile', typography: {scale: {base: 14, ratio: 1.2}}},
      {mobile: {typography: {scale: {base: 16, ratio: 1.2}}}},
    );
    const {component, prose} = generateConditionalCSS(theme);

    // The conditional scale's own tokens, not the base theme's.
    expect(theme.tokens['--font-size-base']).toBe('0.875rem');
    expect(component).toContain('--font-size-base: 1rem;');
    expect(component).toContain('.astryx-text.body');

    // Prose elements track the conditional scale too: line-heights are baked
    // into those rules as literals, so they have to be re-emitted.
    expect(prose).toContain(MOBILE_QUERY);
    expect(prose).toContain(':where(p)');
    expect(prose).toContain('line-height: 1.5;');
    expect(generateThemeCSS(theme).prose).toContain('line-height: 1.4286;');
  });

  it('emits no prose block when the condition changes no text tokens', () => {
    const {prose} = generateConditionalCSS(
      defineTheme(
        {name: 'no-prose-mobile'},
        {mobile: {tokens: {'--spacing-4': '12px'}}},
      ),
    );
    expect(prose).toBe('');
  });

  it('emits only component rules when only components are set', () => {
    const {component} = generateConditionalCSS(
      defineTheme(
        {name: 'components-only'},
        {mobile: {components: {button: {base: {paddingBlock: '12px'}}}}},
      ),
    );

    expect(component).toContain('.astryx-button');
    expect(component).toContain('padding-block: 12px;');
    expect(component).not.toContain(':scope {');
  });

  it('supports the radius, color and motion axes', () => {
    const {component} = generateConditionalCSS(
      defineTheme(
        {name: 'other-axes'},
        {
          mobile: {
            radius: {base: 4, multiplier: 2},
            motion: {fast: 100, medium: 250, ratio: 0.75},
            color: {accent: '#0064E0'},
          },
        },
      ),
    );

    expect(component).toContain('--radius-element');
    expect(component).toContain('--duration-fast');
    expect(component).toContain('--color-accent');
  });

  it('resolves a [light, dark] tuple inside a condition, as on the theme', () => {
    const theme = defineTheme(
      {name: 'tuple', tokens: {'--color-accent': ['#0064E0', '#4599FF']}},
      {mobile: {tokens: {'--color-accent': ['#0050B3', '#7FB8FF']}}},
    );

    expect(theme.tokens['--color-accent']).toBe(
      'light-dark(#0064E0, #4599FF)',
    );
    expect(theme.__conditional?.[0].tokens['--color-accent']).toBe(
      'light-dark(#0050B3, #7FB8FF)',
    );
  });

  it('does not police what an axis may override', () => {
    // Astryx guides, it does not prevent: a builder who asks for a mobile
    // radius or padding change gets it.
    const {component} = generateConditionalCSS(
      defineTheme(
        {name: 'unpoliced'},
        {
          mobile: {
            tokens: {'--radius-container': '0px', '--spacing-4': '2px'},
            components: {card: {base: {padding: '4px'}}},
          },
        },
      ),
    );
    expect(component).toContain('--radius-container: 0px;');
    expect(component).toContain('--spacing-4: 2px;');
    expect(component).toContain('.astryx-card');
  });
});

describe('conditional theme — precedence', () => {
  it('lets the conditional value win inside the query and leaves the base alone outside it', () => {
    const theme = defineTheme(
      {name: 'precedence', tokens: {'--spacing-4': '16px'}},
      {mobile: {tokens: {'--spacing-4': '12px'}}},
    );

    const {component} = generateThemeCSS(theme);

    // The base declaration still ships unchanged — desktop is untouched.
    const baseIndex = component.indexOf('--spacing-4: 16px;');
    const conditionalIndex = component.indexOf('--spacing-4: 12px;');
    expect(baseIndex).toBeGreaterThanOrEqual(0);
    expect(conditionalIndex).toBeGreaterThanOrEqual(0);

    // A media query adds no specificity, so the conditional block must come
    // LATER in source order for its value to win where it matches. This is
    // the documented precedence rule.
    expect(conditionalIndex).toBeGreaterThan(baseIndex);
    expect(component.indexOf(MOBILE_QUERY)).toBeGreaterThan(baseIndex);

    // The base theme's own token map is untouched by the conditional layer.
    expect(theme.tokens['--spacing-4']).toBe('16px');
  });

  it('emits conditions in the map\u2019s key order, so the later key wins', () => {
    // Both of these match a 1000px-wide window. Source order is the only
    // tiebreak — a media query adds no specificity — so the LATER key wins.
    const {component} = generateThemeCSS(
      defineTheme(
        {name: 'two-matching'},
        {
          '(min-width: 900px)': {tokens: {'--spacing-4': '20px'}},
          '(min-width: 600px)': {tokens: {'--spacing-4': '18px'}},
        },
      ),
    );

    expect(component.indexOf('--spacing-4: 18px;')).toBeGreaterThan(
      component.indexOf('--spacing-4: 20px;'),
    );
  });

  it('lets the same component rule be overridden inside the condition', () => {
    const {component} = generateThemeCSS(
      defineTheme(
        {
          name: 'component-precedence',
          components: {button: {base: {fontSize: '14px'}}},
        },
        {mobile: {components: {button: {base: {fontSize: '16px'}}}}},
      ),
    );

    expect(component.indexOf('font-size: 16px;')).toBeGreaterThan(
      component.indexOf('font-size: 14px;'),
    );
  });

  it('lets explicit conditional tokens beat the condition\u2019s own scale', () => {
    const theme = defineTheme(
      {name: 'within-condition'},
      {
        mobile: {
          typography: {scale: {base: 16, ratio: 1.2}},
          tokens: {'--text-body-size': '18px'},
        },
      },
    );

    expect(theme.__conditional?.[0].tokens['--text-body-size']).toBe('18px');
  });

  it('keeps desktop output unchanged when a mobile layer is added', () => {
    const base = {
      name: 'desktop-stable',
      typography: {scale: {base: 14, ratio: 1.2}},
      tokens: {'--spacing-4': '16px'},
      components: {button: {base: {fontWeight: '600'}}},
    } as const;

    const withoutMobile = generateThemeCSS(defineTheme({...base}));
    const withMobile = generateThemeCSS(
      defineTheme({...base}, {mobile: {tokens: {'--spacing-4': '12px'}}}),
    );

    // Everything the desktop cascade sees is the prefix before the @media
    // block — identical, character for character.
    const conditionalStart = withMobile.component.indexOf(MOBILE_QUERY);
    expect(conditionalStart).toBeGreaterThan(0);
    expect(withMobile.component.slice(0, conditionalStart).trimEnd()).toBe(
      withoutMobile.component.trimEnd(),
    );
    expect(withMobile.prose).toBe(withoutMobile.prose);
  });
});

describe('conditional theme — the deprecated flat form', () => {
  it('still resolves root mobile/breakpoints, and the 2nd argument wins', () => {
    const flat = defineTheme({
      name: 'flat',
      breakpoints: {mobile: 640},
      mobile: {tokens: {'--spacing-4': '12px'}},
    });
    expect(flat.__conditional?.[0].query).toBe(
      '(max-width: 640px) and (pointer: coarse)',
    );

    const both = defineTheme(
      {name: 'both', mobile: {tokens: {'--spacing-4': '12px'}}},
      {print: {tokens: {'--spacing-4': '0'}}},
    );
    expect(both.__conditional).toHaveLength(1);
    expect(both.__conditional?.[0].query).toBe('print');
  });
});
