// Spike fixture — the same light-dark()-only-in-a-condition theme, written in
// PR #4930's flat form, to establish whether the guard gap predates the
// signature change.
import {defineTheme} from '@astryxdesign/core/theme';

export default defineTheme({
  name: 'csbug',
  tokens: {'--spacing-4': '16px'},
  mobile: {tokens: {'--color-accent': ['#0050B3', '#7FB8FF']}},
});
