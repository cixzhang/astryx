// Spike fixture — PR #4930's CURRENT flat form: conditions as a root key.
import {defineTheme} from '@astryxdesign/core/theme';

export default defineTheme({
  name: 'acme',
  tokens: {
    '--color-accent': ['#0064E0', '#4599FF'],
    '--color-background-surface': ['#FFFFFF', '#101214'],
    '--radius-container': '12px',
    '--spacing-4': '16px',
  },
  components: {
    button: {
      base: {fontWeight: '600'},
      'variant:secondary': {backgroundColor: 'rgba(0,0,0,0.06)'},
    },
  },
  mobile: {
    tokens: {
      '--spacing-4': '12px',
      '--color-accent': ['#0050B3', '#7FB8FF'],
    },
    components: {
      button: {base: {paddingBlock: '12px'}},
    },
  },
});
