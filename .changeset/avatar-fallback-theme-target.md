---
'@astryxdesign/core': patch
---

[feat] Avatar: the fallback surface (initials and default icon) is now a direct theme target via the stable `astryx-avatar-fallback` class. Theme its background, text color, and font weight through the `avatar-fallback` component key (e.g. `components: { 'avatar-fallback': { base: { backgroundColor: '...' } } }`), replacing the previous internal derived vars. Per-size initials font size is still set on the `avatar` size tiers.

@cixzhang
