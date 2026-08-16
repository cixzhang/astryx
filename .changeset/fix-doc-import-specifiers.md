---
'@astryxdesign/cli': patch
---

[docs] Fix the theming snippet in the technical documentation page template: it taught `import { ThemeProvider } from '@astryxdesign/core/Theme'`, but the component is `Theme` and the subpath is `@astryxdesign/core/theme`, so the line a reader copied could not resolve.

@cixzhang
