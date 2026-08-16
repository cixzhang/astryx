---
'@astryxdesign/core': patch
'@astryxdesign/cli': patch
---

[feat] `defineTheme` gains conditional theme layers, as a **second argument**: `defineTheme(theme, conditions)`. The theme object's key set stays closed; the conditions map's stays open. A key is either a blessed alias (`mobile`, `print`) or any raw CSS media query — `'(min-width: 900px)'`, `'(prefers-reduced-motion: reduce)'`, `'(forced-colors: active)'` — which is also how a custom breakpoint is expressed, so there is no separate `breakpoints` config. An alias is pure sugar for the query it expands to: `mobile` and `'(max-width: 756px) and (pointer: coarse)'` emit identical CSS, and `mobileMediaQuery(640)` gives you the same narrow-AND-touch shape at another width. Each value is a partial theme (`typography`, `color`, `radius`, `motion`, `tokens`, `components` — each axis independent, `[light, dark]` tuples included). Inside a matching condition the conditional value wins over the base theme; outside it the base theme is untouched. Where two conditions both match, the one written later wins — a media query adds no specificity, so layers are emitted in the map's key order. The feature is opt-in: a theme with no second argument emits no conditional CSS and its output is byte-identical to before. Works in both distribution modes: runtime `<Theme>` injection and `astryx theme build` (a theme file exporting a plain object declares conditions under a `conditions` key).

[fix] `astryx theme build` now emits the `color-scheme` guard when `light-dark()` is reachable only through a conditional layer. Previously the guard only inspected the base rules, so a theme whose only tuple value lived in a condition shipped `light-dark()` with no `color-scheme: light dark`, resolving to the light arm forever.

@cixzhang
