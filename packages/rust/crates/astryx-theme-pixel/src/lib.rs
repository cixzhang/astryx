//! A retro pixel theme for Astryx, ported from the `defineTheme` pixel theme
//! of the Astryx pixel gallery.
//!
//! Everything is a function of one number, the pixel size `P` in points:
//!
//! - Type is Galmuri: Galmuri7 (eight design pixels per em) at 8P for body
//!   text and controls, and Galmuri9 (ten per em) at 10P, 20P, and 30P for
//!   headings and display text. Each design pixel is a whole number of
//!   theme pixels, and every line height is a whole number of pixels.
//! - Spacing, borders (`P`), element heights, and hard zero-blur shadows
//!   are whole multiples of `P`.
//! - Corners and circles are staircases on the grid, and icons are 12 by
//!   12 bitmaps with one icon pixel per theme pixel.
//! - Terrace draws every color from one fixed palette, [`PALETTE`]. Five
//!   older retro palettes (garden, Game Boy, NES, C64, PICO-8), resolved by
//!   Astryx's own theme resolver, stay as they are.
//!
//! ```
//! use astryx_theme_pixel::PixelTheme;
//! let theme = PixelTheme::Terrace.theme();
//! assert_eq!(theme.grid, Some(2.0));
//! assert_eq!(theme.light.font.body.size, 16.0);
//! ```

use astryx_core::tokens::Rgba;
use astryx_core::tokens::{
    BitmapIcon, ColorTokens, FontFamilyTokens, FontSizeTokens, FontTokens, FontWeightTokens,
    RadiusTokens, ShadowLayer, ShadowTokens, SizeTokens, SpaceTokens, Theme, ThemeFont, Tokens,
    TypeStyle,
};
use std::sync::{Mutex, OnceLock};

mod palette;
mod terrace;

pub use palette::{contrast, Import, OffPalette, Palette, Swatch, PALETTE};

include!(concat!(env!("OUT_DIR"), "/pixel.rs"));

/// The default pixel size, in points. At scale 3 (a 400 ppi handheld) one
/// theme pixel is 6 device pixels, and body text is 16 points, close to
/// Astryx's 14 point body. It stays whole at scales 1, 1.5, 2, and 3.
pub const DEFAULT_PIXEL: f32 = 2.0;

/// Galmuri7, a pixel font with eight design pixels per em, for body text,
/// labels, and controls. A Latin, Latin Extended, and Vietnamese cut of the
/// `galmuri` package (SIL Open Font License 1.1, `fonts/OFL.txt`), made by
/// `tools/pixel-tokens/fetch-fonts.sh`.
pub static GALMURI7: ThemeFont = ThemeFont {
    family: "Galmuri7",
    files: &[include_bytes!("../fonts/Galmuri7-Latin.ttf")],
    em_pixels: 8,
    fallbacks: &FALLBACKS,
};

/// Galmuri9, ten design pixels per em, for headings. Its cut also keeps
/// Greek and Cyrillic, so it draws them for Galmuri7 text too.
pub static GALMURI9: ThemeFont = ThemeFont {
    family: "Galmuri9",
    files: &[include_bytes!("../fonts/Galmuri9-Latin.ttf")],
    em_pixels: 10,
    fallbacks: &FALLBACKS,
};

/// Families tried next, in this order, for characters a text's own family
/// lacks. The full Galmuri7 (Korean, kana, some CJK) and Fusion Pixel 8px
/// (CJK) join when an app adds their files to the renderer (see
/// `examples/fallback.rs`); both share Galmuri7's grid. Galmuri9, bundled,
/// draws Greek and Cyrillic on its own ten-pixel grid, so it comes last.
pub const FALLBACKS: [&str; 7] = [
    "Galmuri7",
    "Fusion Pixel 8px Prop latin",
    "Fusion Pixel 8px Prop zh_hans",
    "Fusion Pixel 8px Prop zh_hant",
    "Fusion Pixel 8px Prop ja",
    "Fusion Pixel 8px Prop ko",
    "Galmuri9",
];

/// The pixel themes: Terrace, drawn only from the fixed [`PALETTE`], and the
/// five retro palettes of the source theme, which predate it and are not
/// held to it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PixelTheme {
    #[default]
    Terrace,
    Garden,
    GameBoy,
    Nes,
    C64,
    Pico8,
}

impl PixelTheme {
    pub const ALL: [PixelTheme; 6] = [
        PixelTheme::Terrace,
        PixelTheme::Garden,
        PixelTheme::GameBoy,
        PixelTheme::Nes,
        PixelTheme::C64,
        PixelTheme::Pico8,
    ];

    /// The theme's name, such as `"gameboy"`: the palette's name in the
    /// source theme, short enough for a theme picker.
    pub fn name(self) -> &'static str {
        match self {
            PixelTheme::Terrace => "terrace",
            PixelTheme::Garden => "garden",
            PixelTheme::GameBoy => "gameboy",
            PixelTheme::Nes => "nes",
            PixelTheme::C64 => "c64",
            PixelTheme::Pico8 => "pico8",
        }
    }

    /// The theme with this name, such as `"nes"`.
    pub fn by_name(name: &str) -> Option<PixelTheme> {
        PixelTheme::ALL.into_iter().find(|p| p.name() == name)
    }

    /// The theme at [`DEFAULT_PIXEL`].
    pub fn theme(self) -> &'static Theme {
        theme(self, DEFAULT_PIXEL)
    }

    /// True when every color the theme paints is a [`PALETTE`] swatch, or
    /// a swatch at partial alpha.
    pub fn uses_palette(self) -> bool {
        self == PixelTheme::Terrace
    }

    /// The source theme's tokens this theme starts from: Terrace takes
    /// Garden's sizes and type and replaces every color.
    fn source(self) -> &'static [Tokens; 2] {
        match self {
            PixelTheme::Terrace | PixelTheme::Garden => &UNIT[0],
            other => &UNIT[other as usize - 1],
        }
    }
}

/// The pixel theme `which` with pixel size `pixel` points. Each theme and
/// size is made once and kept for the life of the program.
/// Pick a size that is a whole number of device pixels on the target
/// screen, such as 2 points at scale 3.
pub fn theme(which: PixelTheme, pixel: f32) -> &'static Theme {
    type Made = Vec<((PixelTheme, u32), &'static Theme)>;
    static CACHE: OnceLock<Mutex<Made>> = OnceLock::new();
    let pixel = if pixel > 0.0 { pixel } else { DEFAULT_PIXEL };
    let key = (which, pixel.to_bits());
    let mut cache = CACHE
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(&(_, theme)) = cache.iter().find(|(k, _)| *k == key) {
        return theme;
    }
    let [light, dark] = which.source();
    let (mut light, mut dark) = (tokens(light, pixel), tokens(dark, pixel));
    if which.uses_palette() {
        terrace::paint(&mut light, false);
        terrace::paint(&mut dark, true);
    }
    let theme: &'static Theme = Box::leak(Box::new(Theme {
        name: which.name(),
        light,
        dark,
        grid: Some(pixel),
        icons: ICONS,
        fonts: &FONTS,
    }));
    cache.push((key, theme));
    theme
}

/// The theme's fonts: body text first, then headings.
static FONTS: [ThemeFont; 2] = [GALMURI7, GALMURI9];

fn tokens(unit: &Tokens, pixel: f32) -> Tokens {
    let mut t = scale(unit, pixel);
    // The source theme cuts corners with clip-paths: two steps on boxes, one
    // on small controls. On the grid a radius of 4 pixels draws two steps
    // and 2 pixels draws one, so these radii give the same staircases.
    t.radius.inner = 2.0 * pixel;
    t.radius.element = 4.0 * pixel;
    t.radius.container = 4.0 * pixel;
    t.radius.page = 6.0 * pixel;
    t.radius.chat = 6.0 * pixel;
    // Every box of the source theme draws its frame in the emphasized
    // border color (an inset ring), so borders take that color here.
    t.color.border = t.color.border_emphasized;
    // Garden's hover tint (5%) does not show on paper-like surfaces at a
    // pixel's size, so hover is 12% and pressed, which must read stronger,
    // is 20%.
    t.color.overlay_hover[3] = 0.12;
    t.color.overlay_pressed[3] = 0.20;
    // Headings and display text are Galmuri9 at whole steps of its ten
    // design pixels: one step for headings 2 to 6, two for heading 1 and the
    // smaller displays, three for display 1. Body text is Galmuri7 at 8P, so
    // every heading reads above it.
    let f = &mut t.font;
    let steps = [
        (&mut f.heading2, 1.0),
        (&mut f.heading3, 1.0),
        (&mut f.heading4, 1.0),
        (&mut f.heading5, 1.0),
        (&mut f.heading6, 1.0),
        (&mut f.heading1, 2.0),
        (&mut f.display2, 2.0),
        (&mut f.display3, 2.0),
        (&mut f.display1, 3.0),
    ];
    for (style, n) in steps {
        style.size = n * 10.0 * pixel;
        style.leading = (style.size + 2.0 * pixel) / style.size;
    }
    t
}

fn scale_shadow(layers: &'static [ShadowLayer], p: f32) -> &'static [ShadowLayer] {
    let scaled: Vec<ShadowLayer> = layers
        .iter()
        .map(|l| ShadowLayer {
            x: l.x * p,
            y: l.y * p,
            blur: l.blur * p,
            spread: l.spread * p,
            ..*l
        })
        .collect();
    Box::leak(scaled.into_boxed_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_length_is_a_whole_number_of_pixels() {
        for which in PixelTheme::ALL {
            for p in [1.0, 2.0, 3.0] {
                let t = theme(which, p);
                assert_eq!(t.grid, Some(p));
                for tokens in [&t.light, &t.dark] {
                    let whole = |v: f32| (v / p).fract() == 0.0;
                    let s = &tokens.space;
                    for v in [s.s0_5, s.s1, s.s2, s.s3, s.s4, s.s6, s.s8, s.s10, s.s12] {
                        assert!(whole(v), "{} space {v} at P={p}", t.name);
                    }
                    let z = &tokens.size;
                    for v in [z.border_width, z.element_sm, z.element_md, z.element_lg] {
                        assert!(whole(v));
                    }
                    let f = &tokens.font;
                    let galmuri7 = [f.body, f.label, f.supporting, f.code].map(|s| (s, 8.0));
                    let galmuri9 = [f.heading1, f.heading4, f.display1].map(|s| (s, 10.0));
                    for (style, em) in galmuri7.into_iter().chain(galmuri9) {
                        assert_eq!((style.size / (em * p)).fract(), 0.0, "on the font grid");
                        let line = style.size * style.leading / p;
                        assert!((line - line.round()).abs() < 1e-3, "line height {line}");
                    }
                    for layer in tokens.shadow.low.iter().chain(tokens.shadow.high) {
                        assert_eq!(layer.blur, 0.0, "hard shadows");
                        assert!(whole(layer.x) && whole(layer.y));
                    }
                }
            }
        }
    }

    #[test]
    fn the_default_is_two_points_and_border_is_one_pixel() {
        let t = PixelTheme::Garden.theme();
        assert_eq!(t.grid, Some(2.0));
        assert_eq!(t.light.size.border_width, 2.0);
        assert_eq!(t.light.font.body.size, 16.0);
        assert_eq!(t.light.font.heading2.size, 20.0);
        assert_eq!(t.light.font.heading1.size, 40.0);
        assert_eq!(t.light.font.display1.size, 60.0);
        assert!(std::ptr::eq(t, theme(PixelTheme::Garden, 2.0)), "cached");
        assert_eq!(t.light.font.family.body, &["Galmuri7", "monospace"]);
        assert_eq!(t.light.font.family.heading, &["Galmuri9", "monospace"]);
    }

    #[test]
    fn palettes_pin_their_own_colors() {
        // DMG greens: body #9BBC0F light, #0F380F dark.
        let gb = PixelTheme::GameBoy.theme();
        let close = |a: [f32; 4], hex: u32| {
            let b = [(hex >> 16) & 255, (hex >> 8) & 255, hex & 255].map(|v| v as f32 / 255.0);
            (0..3).all(|i| (a[i] - b[i]).abs() < 0.002)
        };
        assert!(close(gb.light.color.background_body, 0x9BBC0F));
        assert!(close(gb.dark.color.background_body, 0x0F380F));
        assert!(close(gb.light.color.text_primary, 0x0F380F));
        // Accents are generated from the palette's seed by HCT, so only
        // their hue is checked.
        let a = PixelTheme::Pico8.theme().light.color.accent;
        assert!(a[0] > a[1] && a[0] > a[2]);
    }

    #[test]
    fn hover_shows_and_pressed_reads_stronger() {
        for which in PixelTheme::ALL {
            let t = which.theme();
            for tokens in [&t.light, &t.dark] {
                let c = &tokens.color;
                assert_eq!(c.overlay_hover[3], 0.12, "{}", t.name);
                assert!(c.overlay_pressed[3] > c.overlay_hover[3], "{}", t.name);
            }
        }
    }

    #[test]
    fn icons_are_twelve_by_twelve() {
        assert_eq!(ICONS.len(), 29, "the 28 source icons and upload");
        for icon in ICONS {
            assert_eq!(icon.rows.len(), 12, "{}", icon.name);
            assert!(icon.rows.iter().all(|r| r.len() == 12), "{}", icon.name);
        }
        for name in astryx_core::BUILT_IN_ICONS {
            assert!(
                PixelTheme::Nes.theme().icon(name).is_some(),
                "no pixel {name}"
            );
        }
        assert!(PixelTheme::ALL
            .iter()
            .all(|p| PixelTheme::by_name(p.name()) == Some(*p)));
    }
}
