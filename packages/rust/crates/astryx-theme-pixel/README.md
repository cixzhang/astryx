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

- **Type.** Galmuri (bundled Latin cuts, SIL OFL 1.1, `fonts/OFL.txt`).
  Galmuri7 has 8 design pixels per em and sets body text, labels, buttons,
  and badges at 8P. Galmuri9 has 10 and sets headings in steps of 10P:
  headings 2 to 6 at 10P, heading 1 and the smaller displays at 20P,
  display 1 at 30P, so every heading reads above its body text. Line
  heights are whole pixels, and glyphs are sampled once per theme pixel,
  so every design pixel fills whole device pixels.
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

The export names Galmuri7 for the body and code families and Galmuri9 for
headings; the source theme names Silkscreen. The Rust theme also sets
three things the CSS theme did per component or does not have:

- Radii: `inner` 2P, `element` and `container` 4P, `page` and `chat` 6P,
  so the staircases match the CSS clip-path corners.
- `border` takes the `borderEmphasized` color, as every CSS box ring does.
- The Galmuri9 heading steps above.

## Fonts and other scripts

| Family | License | Size | Grid | Covers |
|---|---|---|---|---|
| Galmuri7, Latin cut (bundled) | OFL 1.1 | 124 KB | 8 px/em | Latin, Latin Extended, Vietnamese |
| Galmuri9, Latin cut (bundled) | OFL 1.1 | 158 KB | 10 px/em | the above, Greek, Cyrillic |
| Galmuri7, full | OFL 1.1 | 3.7 MB | 8 px/em | adds Hangul, kana, 6,360 CJK |
| Fusion Pixel 8px Proportional | OFL 1.1 (parts: Misaki free license, Galmuri, Boutique Bitmap, Miseki Bitmap: OFL) | 2.5 MB per language file | 8 px/em | Latin, Hangul, kana, 14,719 CJK |

The Latin cuts are committed: OFL 1.1 allows a renamed or same-named
subset (Galmuri has no Reserved Font Name), and at 282 KB together they let
every build draw the theme with no download. Both are cut from the pinned
`galmuri` npm package by one script, which checks the package and both
outputs by SHA-256:

```sh
cd tools/pixel-tokens && npm install && cd ../..
tools/pixel-tokens/fetch-fonts.sh --subset   # full files into target/pixel-fonts, cuts into fonts/
export PIXEL_FONTS_DIR=$PWD/target/pixel-fonts
```

The full files stay a build input, never committed. The web build reads
`PIXEL_FONTS_DIR` and loads per-script subsets as text needs them; desktop
and handheld apps add the files with `add_fallback_font` (see
`examples/fallback.rs`). `FALLBACKS` sets the order: the full Galmuri7
(a family can hold its cut and its full file), Fusion Pixel 8px, then
Galmuri9 for Greek and Cyrillic. Fallback faces with another grid, such as
Galmuri9 in 8P text, draw at full resolution without antialiasing, so they
stay legible with slightly uneven pixels.

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

1. **Galmuri, not Silkscreen.** The port sets all text in Galmuri for
   readability. Text is mixed case, and Galmuri has no bold, so weight
   tokens draw regular; hierarchy comes from size steps and color.
2. **Not compared in a browser.** The goldens were checked against the
   theme source and its generated `pixel.css`, not against live pixels of
   the demo.
3. **Spacing scales with P.** Rust multiplies the P = 3 design by P. CSS
   snaps pixel spacing to P, so at P = 2 CSS spacing is about 1.5 times the
   Rust spacing. At P = 3 they match.
4. **Shapes from radii, not per-component clips.** Badges are pixel pills
   (CSS: two-step boxes), the Switch thumb is a pixel circle (CSS: square),
   and ProgressBar ends step (CSS: square).
5. **All borders are emphasized**, dividers included (CSS keeps dividers
   faint).
6. **Clips are square on the grid.** Content that overflows a stepped
   corner, such as a highlighted row at a menu's corner, shows a square
   corner.
7. **Small icons draw at half theme pixels.** Only icons of 12P and more
   have one icon pixel per theme pixel.
8. **Vector symbols without pixel art** (the Avatar person, the Thumbnail
   image placeholder) draw without antialiasing but are not pixel art.
9. **Radio and checkbox indicators are not ported**: Astryx Rust has no
   radio or checkbox yet. Chosen menu rows use the pixel check.
10. **Photos are not baked to the grid.** Images get stepped corners and
    nearest sampling on the grid, but they are not first reduced to one
    image pixel per theme pixel, as the source bakes its avatars. The
    runtime keeps no copy of image pixels, so baking would belong in the
    renderers.
11. **Status and categorical colors are not pinned** to the palettes, as
    in the source (`LEAKAGE.md` #7).
12. **Few text sizes**, by design: 8P body, 10P, 20P, and 30P headings.
13. **Uneven Greek and Cyrillic.** No open pixel font with Greek and
    Cyrillic on an 8 px/em grid was found, so Galmuri9 draws them on its
    own grid inside 8P text.
14. **A smooth theme nested in a pixel theme** (`Element::themed`) still
    draws without antialiasing.
15. **Motion on the grid moves in whole pixels.** Press scale is left out
    (0.98 has no pixel form), and moving elements, such as the Switch thumb,
    step by whole theme pixels.
