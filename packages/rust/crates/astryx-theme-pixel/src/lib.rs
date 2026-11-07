//! A retro pixel theme for Astryx, ported from the `defineTheme` pixel theme
//! of the Astryx pixel gallery.
//!
//! Everything is a function of one number, the pixel size `P` in points:
//!
//! - Type is Silkscreen, whose design pixel is one eighth of its size, so
//!   every text size is a multiple of `8P` and every line height a whole
//!   number of pixels. The theme draws on a grid of `P`, so each design
//!   pixel lands on whole device pixels.
//! - Spacing, borders (`P`), element heights, and hard zero-blur shadows
//!   are whole multiples of `P`.
//! - Corners and circles are staircases on the grid, and icons are 12 by
//!   12 bitmaps with one icon pixel per theme pixel.
//! - Five palettes (garden, Game Boy, NES, C64, PICO-8) in light and dark,
//!   resolved by Astryx's own theme resolver.
//!
//! ```
//! use astryx_theme_pixel::Palette;
//! let theme = Palette::GameBoy.theme();
//! assert_eq!(theme.grid, Some(2.0));
//! assert_eq!(theme.light.font.body.size, 16.0);
//! ```

use astryx_core::tokens::{
    BitmapIcon, ColorTokens, FontFamilyTokens, FontSizeTokens, FontTokens, FontWeightTokens,
    RadiusTokens, ShadowLayer, ShadowTokens, SizeTokens, SpaceTokens, Theme, ThemeFont, Tokens,
    TypeStyle,
};
use std::sync::{Mutex, OnceLock};

include!(concat!(env!("OUT_DIR"), "/pixel.rs"));

/// The default pixel size, in points. At scale 3 (a 400 ppi handheld) one
/// theme pixel is 6 device pixels, and body text is 16 points, close to
/// Astryx's 14 point body. It stays whole at scales 1, 1.5, 2, and 3.
pub const DEFAULT_PIXEL: f32 = 2.0;

/// Silkscreen Regular and Bold, under the SIL Open Font License 1.1
/// (`fonts/OFL.txt`). Latin only.
pub static SILKSCREEN: ThemeFont = ThemeFont {
    family: "Silkscreen",
    files: &[
        include_bytes!("../fonts/Silkscreen-Regular.ttf"),
        include_bytes!("../fonts/Silkscreen-Bold.ttf"),
    ],
    em_pixels: 8,
    fallbacks: &FALLBACKS,
};

/// Pixel families for scripts Silkscreen lacks, tried in this order when
/// an app adds their files to the renderer (see `examples/fallback.rs`).
/// Galmuri7 and Fusion Pixel 8px share Silkscreen's grid of eight pixels
/// per em; Galmuri9 (Greek, Cyrillic) does not, so it comes last.
pub const FALLBACKS: [&str; 7] = [
    "Galmuri7",
    "Fusion Pixel 8px Prop latin",
    "Fusion Pixel 8px Prop zh_hans",
    "Fusion Pixel 8px Prop zh_hant",
    "Fusion Pixel 8px Prop ja",
    "Fusion Pixel 8px Prop ko",
    "Galmuri9",
];

/// The five retro palettes of the pixel theme.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Palette {
    #[default]
    Garden,
    GameBoy,
    Nes,
    C64,
    Pico8,
}

impl Palette {
    pub const ALL: [Palette; 5] = [
        Palette::Garden,
        Palette::GameBoy,
        Palette::Nes,
        Palette::C64,
        Palette::Pico8,
    ];

    /// The palette's name in the source theme, such as `"gameboy"`.
    pub fn name(self) -> &'static str {
        match self {
            Palette::Garden => "garden",
            Palette::GameBoy => "gameboy",
            Palette::Nes => "nes",
            Palette::C64 => "c64",
            Palette::Pico8 => "pico8",
        }
    }

    /// The theme's name, such as `"pixel-gameboy"`.
    pub fn theme_name(self) -> &'static str {
        match self {
            Palette::Garden => "pixel-garden",
            Palette::GameBoy => "pixel-gameboy",
            Palette::Nes => "pixel-nes",
            Palette::C64 => "pixel-c64",
            Palette::Pico8 => "pixel-pico8",
        }
    }

    /// The palette with this name, such as `"nes"`.
    pub fn by_name(name: &str) -> Option<Palette> {
        Palette::ALL
            .into_iter()
            .find(|p| p.name() == name || p.theme_name() == name)
    }

    /// The pixel theme in this palette at [`DEFAULT_PIXEL`].
    pub fn theme(self) -> &'static Theme {
        theme(self, DEFAULT_PIXEL)
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// The pixel theme in `palette` with pixel size `pixel` points. Each
/// palette and size is made once and kept for the life of the program.
/// Pick a size that is a whole number of device pixels on the target
/// screen, such as 2 points at scale 3.
pub fn theme(palette: Palette, pixel: f32) -> &'static Theme {
    type Made = Vec<((Palette, u32), &'static Theme)>;
    static CACHE: OnceLock<Mutex<Made>> = OnceLock::new();
    let pixel = if pixel > 0.0 { pixel } else { DEFAULT_PIXEL };
    let key = (palette, pixel.to_bits());
    let mut cache = CACHE
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(&(_, theme)) = cache.iter().find(|(k, _)| *k == key) {
        return theme;
    }
    let [light, dark] = &UNIT[palette.index()];
    let theme: &'static Theme = Box::leak(Box::new(Theme {
        name: palette.theme_name(),
        light: tokens(light, pixel),
        dark: tokens(dark, pixel),
        grid: Some(pixel),
        icons: ICONS,
        fonts: std::slice::from_ref(&SILKSCREEN),
    }));
    cache.push((key, theme));
    theme
}

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
        for palette in Palette::ALL {
            for p in [1.0, 2.0, 3.0] {
                let t = theme(palette, p);
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
                    for style in [f.body, f.label, f.supporting, f.heading1, f.display1] {
                        assert_eq!((style.size / (8.0 * p)).fract(), 0.0, "on the font grid");
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
        let t = Palette::Garden.theme();
        assert_eq!(t.grid, Some(2.0));
        assert_eq!(t.light.size.border_width, 2.0);
        assert_eq!(t.light.font.body.size, 16.0);
        assert_eq!(t.light.font.display1.size, 48.0);
        assert!(std::ptr::eq(t, theme(Palette::Garden, 2.0)), "cached");
        assert_eq!(t.light.font.family.body, &["Silkscreen", "monospace"]);
    }

    #[test]
    fn palettes_pin_their_own_colors() {
        // DMG greens: body #9BBC0F light, #0F380F dark.
        let gb = Palette::GameBoy.theme();
        let close = |a: [f32; 4], hex: u32| {
            let b = [(hex >> 16) & 255, (hex >> 8) & 255, hex & 255].map(|v| v as f32 / 255.0);
            (0..3).all(|i| (a[i] - b[i]).abs() < 0.002)
        };
        assert!(close(gb.light.color.background_body, 0x9BBC0F));
        assert!(close(gb.dark.color.background_body, 0x0F380F));
        assert!(close(gb.light.color.text_primary, 0x0F380F));
        // Accents are generated from the palette's seed by HCT, so only
        // their hue is checked.
        let a = Palette::Pico8.theme().light.color.accent;
        assert!(a[0] > a[1] && a[0] > a[2]);
    }

    #[test]
    fn icons_are_twelve_by_twelve() {
        assert_eq!(ICONS.len(), 29, "the 28 source icons and upload");
        for icon in ICONS {
            assert_eq!(icon.rows.len(), 12, "{}", icon.name);
            assert!(icon.rows.iter().all(|r| r.len() == 12), "{}", icon.name);
        }
        for name in astryx_core::BUILT_IN_ICONS {
            assert!(Palette::Nes.theme().icon(name).is_some(), "no pixel {name}");
        }
        assert!(Palette::ALL
            .iter()
            .all(|p| Palette::by_name(p.name()) == Some(*p)));
    }
}
