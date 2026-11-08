//! The one fixed palette of pixel and world art, from
//! `tokens/pixel-palette.json`: every color the Terrace theme paints and
//! every pixel of world art is one of its swatches.
//!
//! The rule covers authored colors. A lighting pass multiplies colors
//! (dusk, night, lights), so its output is not palette-exact, and that is
//! expected: night is a tint the light pass adds, not extra pixel colors.
//!
//! ```
//! use astryx_theme_pixel::PALETTE;
//! let leaf = PALETTE.get("leaf-3").unwrap();
//! assert!(PALETTE.contains(leaf));
//! assert!(!PALETTE.contains(astryx_core::Color::rgba(0.0, 0.0, 0.0, 1.0)));
//! ```

use astryx_core::world::SpriteSheet;
use astryx_core::{App, Color, ImageData, ImageId, Runtime};

/// One named color of the palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Swatch {
    pub name: &'static str,
    pub rgb: [u8; 3],
}

impl Swatch {
    /// The swatch as an opaque color.
    pub fn color(&self) -> Color {
        let [r, g, b] = self.rgb.map(|v| f32::from(v) / 255.0);
        Color::rgba(r, g, b, 1.0)
    }
}

/// An ordered list of named swatches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub name: &'static str,
    pub swatches: &'static [Swatch],
}

/// The palette of `tokens/pixel-palette.json`.
pub static PALETTE: Palette = Palette {
    name: crate::PALETTE_NAME,
    swatches: crate::SWATCHES,
};

impl Palette {
    /// The swatch named `name`, such as `"leaf-3"`, as an opaque color.
    pub fn get(&self, name: &str) -> Option<Color> {
        self.swatch(name).map(|s| s.color())
    }

    /// The swatch named `name`.
    pub fn swatch(&self, name: &str) -> Option<&'static Swatch> {
        self.swatches.iter().find(|s| s.name == name)
    }

    /// True when `color` is an opaque swatch.
    pub fn contains(&self, color: Color) -> bool {
        color.a >= 1.0 && self.swatch_of(color).is_some()
    }

    /// The swatch whose color `color` has, at any alpha: the swatch of a
    /// token such as an overlay that is a swatch at partial opacity.
    pub fn swatch_of(&self, color: Color) -> Option<&'static Swatch> {
        let rgb = to_u8(color);
        self.swatches.iter().find(|s| s.rgb == rgb)
    }

    /// The swatch nearest to `color` by eye (in Oklab).
    pub fn nearest(&self, color: Color) -> &'static Swatch {
        self.nearest_rgb(to_u8(color), &[])
    }

    /// The Oklab lightness of `color`, 0 to 1.
    pub fn lightness(&self, color: Color) -> f32 {
        oklab(to_u8(color))[0]
    }

    /// [`Self::nearest`], leaving out the swatches named in `except`.
    pub fn nearest_except(&self, color: Color, except: &[&str]) -> &'static Swatch {
        self.nearest_rgb(to_u8(color), except)
    }

    fn nearest_rgb(&self, rgb: [u8; 3], except: &[&str]) -> &'static Swatch {
        let want = oklab(rgb);
        self.swatches
            .iter()
            .filter(|s| !except.contains(&s.name))
            .min_by(|a, b| distance(oklab(a.rgb), want).total_cmp(&distance(oklab(b.rgb), want)))
            .expect("the palette has swatches")
    }

    /// The pixels of `image` that are not opaque swatches or fully clear.
    /// Edges blended to partial alpha are off-palette too.
    pub fn check(&self, image: &ImageData) -> Result<(), OffPalette> {
        let mut off = OffPalette::default();
        for (i, px) in image.rgba.as_chunks::<4>().0.iter().enumerate() {
            let ok = px[3] == 0 || (px[3] == 255 && self.swatches.iter().any(|s| s.rgb == px[..3]));
            if !ok {
                off.count += 1;
                if off.first.len() < OffPalette::LISTED {
                    let (x, y) = (i as u32 % image.width, i as u32 / image.width);
                    off.first.push((x, y, [px[0], px[1], px[2], px[3]]));
                }
            }
        }
        if off.count == 0 {
            Ok(())
        } else {
            Err(off)
        }
    }

    /// `image` with every pixel moved to the nearest swatch, and alpha to
    /// fully clear or fully opaque, for importing art made elsewhere.
    pub fn quantize(&self, image: &ImageData) -> ImageData {
        let mut rgba = image.rgba.to_vec();
        for px in rgba.as_chunks_mut::<4>().0 {
            if px[3] < 128 {
                px.copy_from_slice(&[0, 0, 0, 0]);
            } else {
                let s = self.nearest_rgb([px[0], px[1], px[2]], &[]);
                px.copy_from_slice(&[s.rgb[0], s.rgb[1], s.rgb[2], 255]);
            }
        }
        ImageData {
            width: image.width,
            height: image.height,
            rgba: rgba.into(),
        }
    }

    /// Art ready to register: unchanged under [`Import::Exact`], checked in
    /// debug builds (and so in tests), or moved onto the palette under
    /// [`Import::Quantize`].
    pub fn prepare(&self, image: &ImageData, import: Import) -> Result<ImageData, OffPalette> {
        match import {
            Import::Quantize => Ok(self.quantize(image)),
            Import::Exact => {
                if cfg!(debug_assertions) {
                    self.check(image)?;
                }
                Ok(image.clone())
            }
        }
    }

    /// Registers world art with `runtime` after [`Self::prepare`].
    pub fn register<A: App>(
        &self,
        runtime: &mut Runtime<A>,
        id: ImageId,
        image: &ImageData,
        import: Import,
    ) -> Result<(), OffPalette> {
        runtime.register_image(id, Some(self.prepare(image, import)?));
        Ok(())
    }

    /// A sprite sheet of `width` by `height` cells cut from `image` after
    /// [`Self::prepare`], so off-palette pixels are reported where the
    /// artist drew them.
    pub fn sprite_sheet(
        &self,
        id: ImageId,
        image: &ImageData,
        width: u32,
        height: u32,
        import: Import,
    ) -> Result<SpriteSheet, OffPalette> {
        Ok(SpriteSheet::grid(
            id,
            &self.prepare(image, import)?,
            width,
            height,
        ))
    }
}

/// How art made elsewhere joins the palette.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Import {
    /// Every pixel must already be a swatch; checked in debug builds.
    #[default]
    Exact,
    /// Pixels move to the nearest swatch.
    Quantize,
}

/// Off-palette pixels of an image: how many, and the first few as
/// `(x, y, rgba)`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OffPalette {
    pub count: usize,
    pub first: Vec<(u32, u32, [u8; 4])>,
}

impl OffPalette {
    /// Pixels listed in `first`.
    pub const LISTED: usize = 8;
}

impl std::fmt::Display for OffPalette {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} off-palette pixels", self.count)?;
        for (i, (x, y, [r, g, b, a])) in self.first.iter().enumerate() {
            let sep = if i == 0 { ": " } else { ", " };
            write!(f, "{sep}({x}, {y}) #{r:02x}{g:02x}{b:02x}")?;
            if *a != 255 {
                write!(f, " alpha {a}")?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for OffPalette {}

fn to_u8(c: Color) -> [u8; 3] {
    [c.r, c.g, c.b].map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// sRGB bytes in Oklab, where distance follows how different colors look.
fn oklab(rgb: [u8; 3]) -> [f32; 3] {
    let [r, g, b] = rgb.map(|v| {
        let c = f32::from(v) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    let l = (0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    [
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    ]
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|i| (a[i] - b[i]) * (a[i] - b[i])).sum()
}

/// WCAG contrast ratio of two opaque colors, 1 to 21.
pub fn contrast(a: Color, b: Color) -> f32 {
    let lum = |c: Color| {
        let [r, g, b] = [c.r, c.g, c.b].map(|v| {
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        });
        0.2126 * r + 0.7152 * g + 0.0722 * b
    };
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(pixels: &[[u8; 4]], width: u32) -> ImageData {
        ImageData {
            width,
            height: pixels.len() as u32 / width,
            rgba: pixels.concat().into(),
        }
    }

    #[test]
    fn the_palette_is_the_design_palette() {
        assert_eq!(PALETTE.swatches.len(), 32);
        assert_eq!(PALETTE.swatches[0].name, "ink");
        assert_eq!(PALETTE.swatch("leaf-3").unwrap().rgb, [0x6f, 0xae, 0x7a]);
        assert!(PALETTE.get("black").is_none());
        let names: std::collections::HashSet<_> = PALETTE.swatches.iter().map(|s| s.name).collect();
        assert_eq!(names.len(), PALETTE.swatches.len(), "names are unique");
    }

    #[test]
    fn contains_needs_an_opaque_swatch() {
        let ink = PALETTE.get("ink").unwrap();
        assert!(PALETTE.contains(ink));
        assert!(!PALETTE.contains(ink.with_alpha(0.5)));
        assert_eq!(PALETTE.swatch_of(ink.with_alpha(0.5)).unwrap().name, "ink");
        assert!(!PALETTE.contains(Color::rgba(0.0, 0.0, 0.0, 1.0)));
    }

    #[test]
    fn nearest_matches_by_eye() {
        for s in PALETTE.swatches {
            assert_eq!(PALETTE.nearest(s.color()).name, s.name);
        }
        assert_eq!(
            PALETTE.nearest(Color::rgba(1.0, 1.0, 1.0, 1.0)).name,
            "paper"
        );
    }

    #[test]
    fn check_reports_where_off_palette_pixels_are() {
        let ink = [0x12, 0x29, 0x53, 255];
        let clear = [9, 9, 9, 0];
        let black = [0, 0, 0, 255];
        let blended = [0x12, 0x29, 0x53, 128];
        assert_eq!(PALETTE.check(&image(&[ink, clear, ink, ink], 2)), Ok(()));
        let off = PALETTE
            .check(&image(&[ink, black, blended, ink], 2))
            .unwrap_err();
        assert_eq!(off.count, 2);
        assert_eq!(off.first, [(1, 0, black), (0, 1, blended)]);
        assert_eq!(
            off.to_string(),
            "2 off-palette pixels: (1, 0) #000000, (0, 1) #122953 alpha 128"
        );
    }

    #[test]
    fn quantize_moves_art_onto_the_palette() {
        let art = image(
            &[
                [0, 0, 0, 255],
                [250, 250, 250, 200],
                [1, 2, 3, 40],
                [0xd9, 0x1a, 0x1c, 255],
            ],
            2,
        );
        let fixed = PALETTE.quantize(&art);
        assert_eq!(PALETTE.check(&fixed), Ok(()));
        assert_eq!(&fixed.rgba[4..8], &[0xfe, 0xfe, 0xfe, 255]);
        assert_eq!(&fixed.rgba[8..12], &[0, 0, 0, 0]);
    }

    #[test]
    fn exact_imports_fail_in_debug_and_quantized_ones_pass() {
        let art = image(&[[0, 0, 0, 255]], 1);
        let checked = cfg!(debug_assertions);
        assert_eq!(PALETTE.prepare(&art, Import::Exact).is_err(), checked);
        assert!(PALETTE.prepare(&art, Import::Quantize).is_ok());
        let sheet = PALETTE
            .sprite_sheet(ImageId(7), &art, 1, 1, Import::Quantize)
            .unwrap();
        assert_eq!(sheet.len(), 1);
        let exact = PALETTE.sprite_sheet(ImageId(7), &art, 1, 1, Import::Exact);
        assert_eq!(exact.is_err(), checked);
    }

    #[test]
    fn contrast_follows_wcag() {
        let (ink, paper) = (PALETTE.get("ink").unwrap(), PALETTE.get("paper").unwrap());
        assert!((contrast(ink, paper) - 14.19).abs() < 0.01);
        assert_eq!(contrast(ink, ink), 1.0);
    }
}
