// Spike fixture — blessed alias, raw queries, and two conditions that overlap.
//
// `mobile` and the hand-written expansion of `print` sit side by side so the
// emitted CSS shows an alias is nothing but sugar. The last two conditions BOTH
// match a 1000px-wide window, and both set --spacing-4, so the emitted order is
// the whole precedence story.
import {defineTheme} from '@astryxdesign/core/theme';

export default defineTheme(
  {
    name: 'queries',
    tokens: {'--spacing-4': '16px', '--color-accent': '#0064E0'},
  },
  {
    mobile: {tokens: {'--spacing-4': '12px'}},
    '(max-width: 756px) and (pointer: coarse)': {
      tokens: {'--radius-container': '4px'},
    },
    print: {tokens: {'--color-accent': '#000000'}},
    '(prefers-reduced-motion: reduce)': {
      motion: {fast: 0, medium: 0, slow: 0, ratio: 1},
    },
    '(forced-colors: active)': {
      components: {button: {base: {borderWidth: '1px'}}},
    },
    // Both of these match at 1000px wide.
    '(min-width: 900px)': {tokens: {'--spacing-4': '20px'}},
    '(min-width: 600px)': {tokens: {'--spacing-4': '18px'}},
  },
);
