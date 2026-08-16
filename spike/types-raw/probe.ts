// Spike probe — what the type system says in each position.
// Every `@ts-expect-error` below is an assertion: tsc fails if the line
// compiles cleanly.
import {defineTheme, mobileMediaQuery} from '@astryxdesign/core/theme';

// 1. Valid: blessed alias, raw query, computed alias expansion, tuple values.
defineTheme(
  {
    name: 'ok',
    tokens: {'--color-accent': ['#0064E0', '#4599FF']},
  },
  {
    mobile: {tokens: {'--spacing-4': '12px'}},
    print: {tokens: {'--color-accent': '#000'}},
    '(min-width: 900px)': {tokens: {'--spacing-4': '20px'}},
    '(prefers-reduced-motion: reduce)': {motion: {fast: 0, medium: 0, slow: 0, ratio: 1}},
    [mobileMediaQuery(640)]: {tokens: {'--spacing-4': '10px'}},
    // A condition value takes a [light, dark] tuple exactly as the theme does.
    '(forced-colors: active)': {
      tokens: {'--color-accent': ['#0050B3', '#7FB8FF']},
    },
    'declared but empty': null,
  },
);

// 2. An unknown axis inside a condition value IS a type error.
defineTheme(
  {name: 'bad-axis'},
  {
    mobile: {

      spacing: {'--spacing-4': '12px'},
    },
  },
);

// 3. An unknown token inside a condition value IS a type error.
defineTheme(
  {name: 'bad-token'},
  {

    mobile: {tokens: {'--spacing-nope': '12px'}},
  },
);

// 4. A wrong value type inside a condition value IS a type error.
defineTheme(
  {name: 'bad-value'},
  {

    mobile: {tokens: {'--spacing-4': 12}},
  },
);

// 5. An unknown key on the THEME object is still a type error — the root key
//    set stayed closed, which is the point of moving conditions out.
defineTheme({
  name: 'closed-root',

  mobil: {tokens: {'--spacing-4': '12px'}},
});

// 6. A misspelled CONDITION key is NOT an error — it is a valid raw query as
//    far as the type system knows. This is the cost of arbitrary query keys.
defineTheme({name: 'typo-ok'}, {mobil: {tokens: {'--spacing-4': '12px'}}});
