# astryx-theme-pixel

A retro pixel theme for Astryx Rust, ported from the `defineTheme` pixel
theme of the Astryx pixel gallery (`cixzhang/astryx-pixel-gallery`). Apps
switch to it like any other theme:

```rust
use astryx_theme_pixel::{theme, Palette};

// In `App::theme`:
ThemeChoice::system(Palette::GameBoy.theme()); // P = 2 points
ThemeChoice::new(theme(Palette::Nes, 3.0), Mode::Dark); // P = 3 points
```

## The one idea

Everything is a function of the pixel size `P`, in points.

- **Type.** Silkscreen (bundled, SIL OFL 1.1, `fonts/OFL.txt`) has 8 design
  pixels per em, so text sizes are 8P, 16P, and 24P, and line heights are
  whole pixels. Glyphs are sampled once per theme pixel, so every design
  pixel fills whole device pixels.
- **Geometry.** Spacing, element heights, the border (P), and the hard
  zero-blur shadows are whole multiples of P. Corners are staircases: a
  radius of 2P draws one step (small controls), 4P draws two (boxes), and a
  full radius draws a pixel circle or pill.
- **Icons.** 28 hand-drawn 12 by 12 icons from the source theme, plus
  `upload`, drawn for this port in the same style (`extra-icons.json`).
- **Color.** Five palettes, light and dark, resolved by Astryx's own theme
  resolver.

## Why P = 2 by default

At scale 3 (a 1024 by 768, 400 ppi handheld) one theme pixel is 6 device
pixels, and body text is 16 points, close to Astryx's 14 point body. P = 1
gives 8 point text, too small to read there. P = 2 stays a whole number of
device pixels at scales 1, 1.5, 2, and 3.

## Tokens

`tokens/pixel-themes.json` (same shape as `tokens/themes.json`, lengths in
pixel units) and `tokens/pixel-icons.json` are made by one script:

```sh
cd tools/pixel-tokens && npm install
node export.mjs <clone of astryx-pixel-gallery> ../../tokens
```

It resolves `createPixelTheme(3, palette)` with `resolveThemeTokens` from
the pinned `@astryxdesign/core`, as the native Swift generator does, and
divides lengths by 3, the pixel size the theme was designed at.

The Rust theme sets two things the CSS theme did per component:

- Radii: `inner` 2P, `element` and `container` 4P, `page` and `chat` 6P,
  so the staircases match the CSS clip-path corners.
- `border` takes the `borderEmphasized` color, as every CSS box ring does.

## Other scripts

Silkscreen covers Latin only. `SILKSCREEN.fallbacks` names pixel families
that the text engine tries next, when an app adds their files with
`add_fallback_font` (see `examples/fallback.rs`). None are bundled.

| Family | License | Size | Grid | Covers |
|---|---|---|---|---|
| Silkscreen (bundled) | OFL 1.1 | 32 + 31 KB | 8 px/em | Basic Latin, Latin-1 |
| Galmuri7 | OFL 1.1 | 3.7 MB | 8 px/em, aligned | Latin Extended, Vietnamese, Hangul, kana, 6,360 CJK |
| Fusion Pixel 8px Proportional | OFL 1.1 (parts: Misaki free license, Galmuri, Boutique Bitmap, Miseki Bitmap: OFL) | 2.5 MB per language file | 8 px/em, aligned | Latin Extended, Hangul, kana, 14,719 CJK |
| Galmuri9 | OFL 1.1 | 4.6 MB | 10 px/em, not aligned | Greek, Cyrillic, plus the above |

No open 8 px/em family with Greek and Cyrillic was found; Galmuri9 is
last in the list and draws them with uneven pixels.

## Leakage eliminated

Items of the source `LEAKAGE.md` that only existed because of CSS:

- **Fonts are named, never loaded.** The theme ships its font files
  (`ThemeFont`); the text engine loads them on first use.
- **`--radius-full` never scales; the Spinner keeps a true arc.** Circles,
  pills, and the spinner are drawn as staircases on the grid. The spinner
  turns in eight steps a second.
- **Inner corner radius and the missing corner block.** A border is drawn
  as the outer staircase minus the inner staircase, so it keeps one width
  around every corner.
- **Badge height.** Badges are never shorter than their label line, for any
  theme.
- **Dilation only reaches component targets.** All text goes through one
  text engine, so every run, in any component or app text, uses the theme
  font on the grid. No dilation is needed.
- **Page-level surfaces have no theme surface.** The runtime paints the
  body color behind the app.
- **The GL display pipeline is app code.** Renderers draw the grid
  natively (`DrawCmd::PushPixelGrid`), with no sampling shader.
- **Icons off the grid.** CSS draws the 12-row icons in 1em (2/3 P per icon
  pixel). Here an icon pixel is a whole number of theme pixels, or half a
  theme pixel for icons smaller than 12P.

## Remaining differences

1. **Not compared in a browser.** The goldens were checked against the
   theme source and its generated `pixel.css`, not against live pixels of
   the demo.
2. **Spacing scales with P.** Rust multiplies the P = 3 design by P. CSS
   snaps pixel spacing to P, so at P = 2 CSS spacing is about 1.5 times the
   Rust spacing. At P = 3 they match.
3. **Shapes from radii, not per-component clips.** Badges are pixel pills
   (CSS: two-step boxes), the Switch thumb is a pixel circle (CSS: square),
   and ProgressBar ends step (CSS: square).
4. **All borders are emphasized**, dividers included (CSS keeps dividers
   faint).
5. **Clips are square on the grid.** Content that overflows a stepped
   corner, such as a highlighted row at a menu's corner, shows a square
   corner.
6. **Small icons draw at half theme pixels.** Only icons of 12P and more
   have one icon pixel per theme pixel.
7. **Vector symbols without pixel art** (the Avatar person, the Thumbnail
   image placeholder) draw without antialiasing but are not pixel art.
8. **Radio and checkbox indicators are not ported**: Astryx Rust has no
   radio or checkbox yet. Chosen menu rows use the pixel check.
9. **Photos are not baked to the grid.** Images get stepped corners and
   nearest sampling on the grid, but they are not first reduced to one
   image pixel per theme pixel, as the source bakes its avatars. The runtime
   keeps no copy of image pixels, so baking would belong in the renderers.
10. **Status and categorical colors are not pinned** to the palettes, as
    in the source (`LEAKAGE.md` #7).
11. **Hierarchy compression** stays, by design: three text sizes.
12. **Fallbacks on the web** are not loaded yet: the web shell's font
    loader does not read `ThemeFont::fallbacks`.
13. **A smooth theme nested in a pixel theme** (`Element::themed`) still
    draws without antialiasing.
14. **Motion on the grid moves in whole pixels.** Press scale is left out
    (0.98 has no pixel form), and moving elements, such as the Switch thumb,
    step by whole theme pixels.
