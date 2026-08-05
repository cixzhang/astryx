---
'@astryxdesign/core': patch
---

[feat] Avatar: the fallback surface (initials and default icon) is now a direct theme target via the stable `astryx-avatar-fallback` class. Theme its background through the `avatar-fallback` component key (e.g. `components: { 'avatar-fallback': { base: { backgroundColor: '...' } } }`) instead of the previous internal derived var.

@cixzhang
