//! Terrace: the pixel theme drawn only from [`PALETTE`]. Garden's sizes,
//! type, and shapes, with every color a palette swatch and no blue: UI ink
//! is the dark brown `soil-4`, paper is `paper` and `paper-2`, the accent
//! is plum (`ink-plum` with `paper` text in light, `plum-1` with dark text
//! in dark), info is brown, success green, warning gold, and errors rose.
//! Dark mode sits on `soil-4` and `soil-3` with `paper` text. Blue is for
//! world art (water, sky) only.

use crate::palette::{Swatch, PALETTE};
use crate::{colors_mut, Rgba};
use astryx_core::tokens::{ShadowLayer, Tokens};

/// Color tokens Terrace sets to a named swatch, light and dark. Every other
/// color token takes the swatch nearest to Garden's value. Alpha always
/// stays as Garden has it, so overlays and muted fills are a swatch at
/// partial opacity.
pub(crate) const ROLES: &[(&str, &str, &str)] = &[
    ("backgroundBody", "paper-2", "soil-4"),
    ("backgroundSurface", "paper", "soil-3"),
    ("backgroundCard", "paper", "soil-3"),
    ("backgroundPopover", "paper", "soil-3"),
    ("backgroundMuted", "stone-1", "soil-1"),
    ("backgroundInverted", "soil-4", "paper"),
    ("backgroundGray", "soil-4", "paper"),
    ("backgroundErrorInverted", "rose-2", "rose-2"),
    ("textPrimary", "soil-4", "paper"),
    // Ordinary text is neutral: secondary text, labels, and captions take
    // the warm grays, and color is kept for attention and status.
    ("textSecondary", "wood-2", "stone-1"),
    ("textDisabled", "stone-2", "stone-2"),
    ("textAccent", "ink-plum", "plum-1"),
    // Rose text on paper is unreadable (1.9:1), so light error text and
    // icons are ink-plum; rose-2 stays the error fill and the dark text.
    ("textError", "ink-plum", "rose-2"),
    ("textSuccess", "green-2", "green-1"),
    ("textWarning", "soil-1", "straw-2"),
    ("textInfo", "wood-2", "wood-1"),
    ("iconPrimary", "soil-4", "paper"),
    ("iconSecondary", "wood-2", "stone-1"),
    ("iconDisabled", "stone-2", "stone-2"),
    ("iconAccent", "ink-plum", "plum-1"),
    ("iconError", "ink-plum", "rose-2"),
    ("iconSuccess", "green-2", "green-1"),
    ("iconWarning", "soil-1", "straw-2"),
    ("iconInfo", "wood-2", "wood-1"),
    // Ink-plum is too dark to stand out on the dark surfaces, so dark mode
    // takes the light plum-1 with dark text on it.
    ("accent", "ink-plum", "plum-1"),
    ("accentMuted", "ink-plum", "plum-1"),
    ("onAccent", "paper", "soil-4"),
    ("border", "soil-4", "stone-2"),
    ("borderEmphasized", "soil-4", "stone-2"),
    ("borderError", "ink-plum", "rose-2"),
    ("borderSuccess", "green-2", "green-1"),
    ("borderWarning", "soil-1", "straw-2"),
    ("borderInfo", "wood-2", "wood-1"),
    // Error fills (the Delete button, error badges and dots) are the earthy
    // rose-3 with dark text; rose-2 stays for hearts and message boxes.
    ("error", "rose-3", "rose-3"),
    ("errorMuted", "rose-2", "rose-2"),
    ("onError", "soil-4", "soil-4"),
    ("onErrorMuted", "ink-plum", "rose-1"),
    ("success", "green-2", "green-1"),
    ("successMuted", "green-2", "green-1"),
    ("onSuccess", "paper", "soil-4"),
    ("onSuccessMuted", "leaf-4", "mint"),
    ("warning", "straw-2", "straw-2"),
    ("warningMuted", "straw-2", "straw-2"),
    ("onWarning", "soil-4", "soil-4"),
    ("onWarningMuted", "soil-1", "straw-2"),
    // Info is brown: the theme keeps blue for world art (water, sky).
    ("info", "wood-2", "wood-1"),
    ("infoMuted", "wood-2", "wood-1"),
    ("onInfo", "paper", "soil-4"),
    ("onInfoMuted", "soil-4", "sand"),
    ("onDark", "paper", "paper"),
    ("onLight", "soil-4", "soil-4"),
    // Secondary buttons, neutral badges, and plates: an opaque warm surface
    // from the background's family, not a tint of ink.
    ("neutral", "stone-1", "soil-1"),
    ("overlay", "soil-4", "soil-4"),
    ("overlayHover", "soil-4", "paper"),
    ("overlayPressed", "soil-4", "paper"),
    ("tintHover", "soil-4", "paper"),
    ("shadow", "soil-4", "soil-4"),
    ("skeleton", "stone-1", "soil-1"),
    ("track", "stone-1", "soil-1"),
    ("syntaxBackground", "paper-2", "soil-3"),
    // Palette v1.3 replaced soil-2 with plum-1; gray ramp tokens that took
    // soil-2 take soil-1.
    ("dataGray5", "soil-1", "soil-1"),
    // Chart hues stay apart from each other without blue.
    ("dataCategoricalBlue", "soil-1", "sand"),
    ("dataCategoricalCyan", "leaf-3", "leaf-3"),
    ("dataCategoricalIndigo", "stone-2", "stone-2"),
    ("dataCategoricalPurple", "ink-plum", "rose-1"),
    ("dataCategoricalTeal", "leaf-4", "leaf-2"),
];

/// Tokens Terrace paints opaque, though Garden's value is a tint.
const OPAQUE: &[&str] = &["neutral"];

/// Swatches only the roles above may use, so nearest matches never pick
/// them: the dark accent stays the one plum-1 thing on screen.
const RESERVED: &[&str] = &["plum-1"];

/// Blue swatches, kept for world art. A token whose nearest swatch is one
/// of them takes the brown of the same lightness instead.
pub(crate) const BLUES: &[&str] = &["ink", "blue-3", "water-1", "water-2", "teal-1", "teal-2"];

/// The warm neutral ramp blue tokens move to, light to dark.
const BROWNS: &[&str] = &[
    "paper", "sand", "stone-1", "straw-3", "wood-1", "stone-2", "wood-2", "soil-1", "soil-3",
    "soil-4",
];

/// Repaints Garden's colors with palette swatches. Shadows are ink.
pub(crate) fn paint(tokens: &mut Tokens, dark: bool) {
    for (name, color) in colors_mut(&mut tokens.color) {
        let role = ROLES
            .iter()
            .find(|(member, ..)| *member == name)
            .map(|&(_, light, night)| if dark { night } else { light });
        *color = match role {
            Some(swatch) => with_swatch(*color, swatch),
            None => nearest(*color),
        };
        if OPAQUE.contains(&name) {
            color[3] = 1.0;
        }
    }
    let s = &mut tokens.shadow;
    for layers in [&mut s.low, &mut s.med, &mut s.high] {
        let painted: Vec<ShadowLayer> = layers
            .iter()
            .map(|l| ShadowLayer {
                color: with_swatch(l.color, "soil-4"),
                ..*l
            })
            .collect();
        *layers = Box::leak(painted.into_boxed_slice());
    }
}

fn with_swatch(color: Rgba, swatch: &str) -> Rgba {
    let s = PALETTE
        .swatch(swatch)
        .unwrap_or_else(|| panic!("Terrace names {swatch}, which the palette lacks"));
    let [r, g, b] = s.rgb.map(|v| f32::from(v) / 255.0);
    [r, g, b, color[3]]
}

fn nearest(color: Rgba) -> Rgba {
    let [r, g, b, a] = color;
    let opaque = astryx_core::Color::rgba(r, g, b, 1.0);
    let mut s = PALETTE.nearest_except(opaque, RESERVED);
    if BLUES.contains(&s.name) {
        let want = PALETTE.lightness(opaque);
        s = BROWNS
            .iter()
            .map(|n| PALETTE.swatch(n).expect("brown swatch"))
            .min_by(|x, y| {
                let d = |s: &Swatch| (PALETTE.lightness(s.color()) - want).abs();
                d(x).total_cmp(&d(y))
            })
            .expect("browns");
    }
    with_swatch([r, g, b, a], s.name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::contrast;
    use crate::{theme, PixelTheme};
    use astryx_core::Color;

    fn color(c: Rgba) -> Color {
        Color::rgba(c[0], c[1], c[2], c[3])
    }

    /// Every color token and shadow is a swatch, or a swatch at partial
    /// alpha, at every pixel size.
    #[test]
    fn every_terrace_color_is_on_the_palette() {
        let names: Vec<&str> = colors_mut(&mut PixelTheme::Terrace.theme().light.color.clone())
            .map(|(n, _)| n)
            .to_vec();
        for p in [1.0, 2.0, 3.0] {
            let t = theme(PixelTheme::Terrace, p);
            for (mode, tokens) in [("light", &t.light), ("dark", &t.dark)] {
                let mut colors = tokens.color;
                let mut off = Vec::new();
                for (name, c) in colors_mut(&mut colors) {
                    if PALETTE.swatch_of(color(*c)).is_none() {
                        off.push(name);
                    }
                }
                let s = &tokens.shadow;
                for layer in s.low.iter().chain(s.med).chain(s.high) {
                    if PALETTE.swatch_of(color(layer.color)).is_none() {
                        off.push("shadow layer");
                    }
                }
                assert!(
                    off.is_empty(),
                    "Terrace {mode} at P={p} is off the palette: {off:?}"
                );
            }
        }
        for (member, ..) in ROLES {
            assert!(
                names.contains(member),
                "Terrace sets {member}, which Astryx lacks"
            );
        }
    }

    /// Body, label, and caption text use the neutral text tokens; the plum
    /// accent and the blues are kept for attention and status.
    #[test]
    fn ordinary_text_is_neutral() {
        let colored = [
            "ink-plum", "plum-1", "blue-3", "water-1", "water-2", "teal-1", "teal-2",
        ];
        let t = PixelTheme::Terrace.theme();
        for (mode, tokens) in [("light", &t.light), ("dark", &t.dark)] {
            let c = &tokens.color;
            for (name, value) in [
                ("textPrimary", c.text_primary),
                ("textSecondary", c.text_secondary),
                ("iconPrimary", c.icon_primary),
                ("iconSecondary", c.icon_secondary),
            ] {
                let swatch = PALETTE.swatch_of(color(value)).unwrap().name;
                assert!(!colored.contains(&swatch), "{mode} {name} is {swatch}");
            }
        }
    }

    /// No Terrace token or shadow is blue: hue between 170 and 260 degrees
    /// with any real saturation. Blue stays in world art.
    #[test]
    fn no_ui_color_is_blue() {
        let blue = |c: Rgba| {
            let (r, g, b) = (c[0], c[1], c[2]);
            let (max, min) = (r.max(g).max(b), r.min(g).min(b));
            let l = (max + min) / 2.0;
            let d = max - min;
            if d < 1e-4 {
                return false;
            }
            let sat = d / (1.0 - (2.0 * l - 1.0).abs());
            let hue = if max == r {
                60.0 * ((g - b) / d).rem_euclid(6.0)
            } else if max == g {
                60.0 * ((b - r) / d + 2.0)
            } else {
                60.0 * ((r - g) / d + 4.0)
            };
            (170.0..=260.0).contains(&hue) && sat > 0.12
        };
        for p in [1.0, 2.0, 3.0] {
            let t = theme(PixelTheme::Terrace, p);
            for (mode, tokens) in [("light", &t.light), ("dark", &t.dark)] {
                let mut colors = tokens.color;
                let mut found = Vec::new();
                for (name, c) in colors_mut(&mut colors) {
                    if blue(*c) {
                        found.push(name);
                    }
                }
                let s = &tokens.shadow;
                if s.low
                    .iter()
                    .chain(s.med)
                    .chain(s.high)
                    .any(|l| blue(l.color))
                {
                    found.push("shadow");
                }
                assert!(
                    found.is_empty(),
                    "Terrace {mode} has blue tokens: {found:?}"
                );
            }
        }
        let rgba = |c: Color| [c.r, c.g, c.b, c.a];
        assert!(BLUES.iter().all(|b| blue(rgba(PALETTE.get(b).unwrap()))));
    }

    /// Reserved swatches appear only where a role names them.
    #[test]
    fn reserved_swatches_only_where_named() {
        let t = PixelTheme::Terrace.theme();
        for (dark, tokens) in [(false, t.light), (true, t.dark)] {
            let mut colors = tokens.color;
            for (name, c) in colors_mut(&mut colors) {
                let swatch = PALETTE.swatch_of(color(*c)).unwrap().name;
                if RESERVED.contains(&swatch) {
                    let named = ROLES
                        .iter()
                        .any(|&(m, l, d)| m == name && (if dark { d } else { l }) == swatch);
                    assert!(named, "{name} took {swatch} by nearest match");
                }
            }
        }
    }

    /// The older pixel themes predate the palette and keep their colors.
    #[test]
    fn older_themes_are_not_held_to_the_palette() {
        let gb = PixelTheme::GameBoy.theme();
        assert!(!PixelTheme::GameBoy.uses_palette());
        assert!(PALETTE
            .swatch_of(color(gb.light.color.background_body))
            .is_none());
    }

    /// Text and controls meet WCAG AA where the palette allows it. The
    /// pairs that cannot are listed with their ratios, so a palette change
    /// that fixes or breaks one shows up here.
    #[test]
    fn contrast_of_text_and_controls() {
        let t = PixelTheme::Terrace.theme();
        let c = |tokens: &Tokens, f: fn(&astryx_core::tokens::ColorTokens) -> Rgba| {
            color(f(&tokens.color)).with_alpha(1.0)
        };
        type Get = fn(&astryx_core::tokens::ColorTokens) -> Rgba;
        let pairs: [(&str, Get, Get, f32); 22] = [
            (
                "info text on body",
                |c| c.text_info,
                |c| c.background_body,
                4.5,
            ),
            (
                "info text on surface",
                |c| c.text_info,
                |c| c.background_surface,
                4.5,
            ),
            ("on-info text on info fill", |c| c.on_info, |c| c.info, 4.5),
            (
                "warning text on body",
                |c| c.text_warning,
                |c| c.background_body,
                4.5,
            ),
            (
                "success text on surface",
                |c| c.text_success,
                |c| c.background_surface,
                4.5,
            ),
            (
                "text on secondary button",
                |c| c.text_primary,
                |c| c.neutral,
                4.5,
            ),
            (
                "destructive focus ring on body",
                |c| c.border_error,
                |c| c.background_body,
                3.0,
            ),
            (
                "destructive focus ring on surface",
                |c| c.border_error,
                |c| c.background_surface,
                3.0,
            ),
            (
                "accent text on body",
                |c| c.text_accent,
                |c| c.background_body,
                4.5,
            ),
            (
                "accent text on surface",
                |c| c.text_accent,
                |c| c.background_surface,
                4.5,
            ),
            (
                "text on body",
                |c| c.text_primary,
                |c| c.background_body,
                4.5,
            ),
            (
                "text on surface",
                |c| c.text_primary,
                |c| c.background_surface,
                4.5,
            ),
            (
                "secondary text on body",
                |c| c.text_secondary,
                |c| c.background_body,
                4.5,
            ),
            (
                "secondary text on surface",
                |c| c.text_secondary,
                |c| c.background_surface,
                4.5,
            ),
            (
                "on-accent text on accent",
                |c| c.on_accent,
                |c| c.accent,
                4.5,
            ),
            (
                "error text on body",
                |c| c.text_error,
                |c| c.background_body,
                4.5,
            ),
            (
                "error text on surface",
                |c| c.text_error,
                |c| c.background_surface,
                4.5,
            ),
            ("on-error text on error", |c| c.on_error, |c| c.error, 4.5),
            (
                "on-warning text on warning",
                |c| c.on_warning,
                |c| c.warning,
                4.5,
            ),
            (
                "accent control on body",
                |c| c.accent,
                |c| c.background_body,
                3.0,
            ),
            (
                "accent control on surface",
                |c| c.accent,
                |c| c.background_surface,
                3.0,
            ),
            (
                "border on surface",
                |c| c.border_emphasized,
                |c| c.background_surface,
                3.0,
            ),
        ];
        // Pairs the palette cannot meet, with the ratio they reach. Button
        // rings Destructive in the error border color.
        let known: [(&str, &str, f32); 0] = [];
        // Status messages: text on a muted fill, a swatch at partial alpha
        // over the body or a surface.
        let muted: [(&str, Get, Get); 4] = [
            ("success", |c| c.on_success_muted, |c| c.success_muted),
            ("error", |c| c.on_error_muted, |c| c.error_muted),
            ("warning", |c| c.on_warning_muted, |c| c.warning_muted),
            ("info", |c| c.on_info_muted, |c| c.info_muted),
        ];
        let over = |fill: Rgba, ground: Rgba| {
            let a = fill[3];
            Color::rgba(
                fill[0] * a + ground[0] * (1.0 - a),
                fill[1] * a + ground[1] * (1.0 - a),
                fill[2] * a + ground[2] * (1.0 - a),
                1.0,
            )
        };
        let mut failing = Vec::new();
        for (mode, tokens) in [("light", &t.light), ("dark", &t.dark)] {
            for (name, fg, bg, need) in pairs {
                let ratio = contrast(c(tokens, fg), c(tokens, bg));
                if ratio < need {
                    failing.push((mode, name, (ratio * 100.0).round() / 100.0));
                }
            }
            let k = &tokens.color;
            for (name, fg, fill) in muted {
                for ground in [k.background_body, k.background_surface] {
                    let ratio = contrast(color(fg(k)), over(fill(k), ground));
                    if ratio < 4.5 {
                        failing.push((mode, name, (ratio * 100.0).round() / 100.0));
                    }
                }
            }
        }
        assert_eq!(failing, known);
    }
}
