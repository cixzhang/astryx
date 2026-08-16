// Spike fixture — light-dark() reachable ONLY through a condition.
// Nothing on the main theme is a tuple, so the base CSS contains no
// light-dark(); the question is whether the color-scheme guard still ships.
import {defineTheme} from '@astryxdesign/core/theme';

export default defineTheme(
  {
    name: 'csbug',
    tokens: {'--spacing-4': '16px'},
  },
  {
    mobile: {tokens: {'--color-accent': ['#0050B3', '#7FB8FF']}},
  },
);
