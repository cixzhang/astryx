//! Astryx theme tokens as typed Rust values, generated at build time from
//! `tokens/themes.json`.
//!
//! Every published theme is a [`Theme`] with a `light` and a `dark`
//! [`Tokens`] value. Member names follow the Astryx token names in
//! snake_case: `backgroundBody` is `color.background_body`, spacing step 0.5
//! is `space.s0_5`.
//!
//! ```
//! use astryx_tokens::Theme;
//! let tokens = &Theme::ASTRYX.light;
//! assert_eq!(tokens.space.s4, 16.0);
//! assert_eq!(tokens.size.element_md, 32.0);
//! ```

#![no_std]

/// Straight (not premultiplied) sRGB color: red, green, blue, alpha, each 0..=1.
pub type Rgba = [f32; 4];

/// One type style: size in points, CSS weight, and line height as a multiple
/// of the size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypeStyle {
    pub size: f32,
    pub weight: u16,
    pub leading: f32,
}

/// One layer of a box shadow, as CSS `box-shadow` draws it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowLayer {
    pub x: f32,
    pub y: f32,
    pub blur: f32,
    pub spread: f32,
    pub is_inset: bool,
    pub color: Rgba,
}

/// Every token of one theme in one color mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tokens {
    pub color: ColorTokens,
    pub space: SpaceTokens,
    pub radius: RadiusTokens,
    pub size: SizeTokens,
    pub font: FontTokens,
    pub shadow: ShadowTokens,
}

/// A published Astryx theme: its name and its light and dark tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub name: &'static str,
    pub light: Tokens,
    pub dark: Tokens,
}

impl Theme {
    /// The published theme with this name, such as `"gothic"`.
    pub fn by_name(name: &str) -> Option<&'static Theme> {
        Self::ALL.iter().copied().find(|theme| theme.name == name)
    }
}

include!(concat!(env!("OUT_DIR"), "/tokens.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_published_theme_is_generated_default_first() {
        let names: [&str; 8] = core::array::from_fn(|i| Theme::ALL[i].name);
        assert_eq!(
            names,
            [
                "astryx",
                "butter",
                "chocolate",
                "gothic",
                "matcha",
                "neutral",
                "stone",
                "y2k"
            ]
        );
        assert_eq!(Theme::ALL.len(), 8);
        assert!(core::ptr::eq(
            Theme::by_name("gothic").unwrap(),
            Theme::GOTHIC
        ));
        assert!(Theme::by_name("missing").is_none());
    }

    #[test]
    fn scales_match_the_json() {
        let t = &Theme::ASTRYX.light;
        assert_eq!(
            (
                t.space.s0,
                t.space.s0_5,
                t.space.s1,
                t.space.s3,
                t.space.s10,
                t.space.s12
            ),
            (0.0, 2.0, 4.0, 12.0, 40.0, 48.0)
        );
        assert_eq!(
            (
                t.radius.inner,
                t.radius.element,
                t.radius.container,
                t.radius.full
            ),
            (4.0, 8.0, 12.0, 9999.0)
        );
        assert_eq!(
            (
                t.size.border_width,
                t.size.element_sm,
                t.size.element_md,
                t.size.element_lg
            ),
            (1.0, 28.0, 32.0, 36.0)
        );
    }

    #[test]
    fn type_styles_and_families() {
        let f = &Theme::ASTRYX.light.font;
        assert_eq!(
            f.body,
            TypeStyle {
                size: 14.0,
                weight: 400,
                leading: 1.4286
            }
        );
        assert_eq!(f.label.weight, 500);
        assert_eq!(f.heading1.size, 24.0);
        assert_eq!(f.heading1.weight, 600);
        assert_eq!(f.display1.size, 42.0);
        assert_eq!((f.size.base, f.size.s2xl, f.size.s4xs), (14.0, 24.0, 6.0));
        assert_eq!(
            (
                f.weight.normal,
                f.weight.medium,
                f.weight.semibold,
                f.weight.bold
            ),
            (400, 500, 600, 700)
        );
        assert_eq!(f.family.body.first(), Some(&"-apple-system"));
        assert_eq!(f.family.code.last(), Some(&"monospace"));
    }

    #[test]
    fn colors_are_unit_rgba_and_modes_differ() {
        for theme in Theme::ALL {
            for t in [&theme.light, &theme.dark] {
                for c in [
                    t.color.accent,
                    t.color.background_body,
                    t.color.text_primary,
                    t.color.overlay_pressed,
                ] {
                    assert!(
                        c.iter().all(|v| (0.0..=1.0).contains(v)),
                        "{} out of range: {c:?}",
                        theme.name
                    );
                }
            }
        }
        let a = Theme::ASTRYX;
        assert_ne!(a.light.color.background_body, a.dark.color.background_body);
        assert_eq!(a.light.color.background_card, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn shadows_are_layer_stacks() {
        let s = &Theme::ASTRYX.light.shadow;
        assert_eq!(s.low.len(), 2);
        assert_eq!((s.low[1].y, s.low[1].blur), (2.0, 8.0));
        assert_eq!((s.high[1].y, s.high[1].blur), (8.0, 24.0));
        assert!(!s.med[0].is_inset);
        assert_eq!(s.med[0].spread, 0.0);
    }
}
