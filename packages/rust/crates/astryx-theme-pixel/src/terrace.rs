//! Terrace: the pixel theme drawn only from [`PALETTE`]. Garden's sizes,
//! type, and shapes, with every color a palette swatch: UI ink is `ink`,
//! UI paper is `paper` and `paper-2`, the UI accent is plum (`ink-plum`
//! with `paper` text in light, `plum-1` with `ink` text in dark), errors
//! are `rose-2`, and dark mode sits on `soil-4`, `ink`, and `blue-3` with
//! `paper` text. Shadows and dark lines are `ink`, never black.

use crate::palette::PALETTE;
use crate::{colors_mut, Rgba};
use astryx_core::tokens::{ShadowLayer, Tokens};

/// Color tokens Terrace sets to a named swatch, light and dark. Every other
/// color token takes the swatch nearest to Garden's value. Alpha always
/// stays as Garden has it, so overlays and muted fills are a swatch at
/// partial opacity.
pub(crate) const ROLES: &[(&str, &str, &str)] = &[
    ("backgroundBody", "paper-2", "soil-4"),
    ("backgroundSurface", "paper", "ink"),
    ("backgroundCard", "paper", "ink"),
    ("backgroundPopover", "paper", "ink"),
    ("backgroundMuted", "stone-1", "blue-3"),
    ("backgroundInverted", "ink", "paper"),
    ("backgroundGray", "ink", "paper"),
    ("backgroundErrorInverted", "rose-2", "rose-2"),
    ("textPrimary", "ink", "paper"),
    // Ordinary text is neutral: secondary text, labels, and captions take
    // the warm grays, and color is kept for attention and status.
    ("textSecondary", "wood-2", "stone-1"),
    ("textDisabled", "stone-2", "stone-2"),
    ("textAccent", "ink-plum", "plum-1"),
    // Rose text on paper is unreadable (1.9:1), so light error text and
    // icons are ink-plum; rose-2 stays the error fill and the dark text.
    ("textError", "ink-plum", "rose-2"),
    ("textSuccess", "green-2", "green-1"),
    ("textWarning", "wood-2", "straw-2"),
    ("textInfo", "blue-3", "water-2"),
    ("iconPrimary", "ink", "paper"),
    ("iconSecondary", "wood-2", "stone-1"),
    ("iconDisabled", "stone-2", "stone-2"),
    ("iconAccent", "ink-plum", "plum-1"),
    ("iconError", "ink-plum", "rose-2"),
    ("iconSuccess", "green-2", "green-1"),
    ("iconWarning", "wood-2", "straw-2"),
    ("iconInfo", "blue-3", "water-2"),
    // Ink-plum is too dark to stand out on the dark surfaces, so dark mode
    // takes the light plum-1 with ink text on it.
    ("accent", "ink-plum", "plum-1"),
    ("accentMuted", "ink-plum", "plum-1"),
    ("onAccent", "paper", "ink"),
    ("border", "ink", "stone-2"),
    ("borderEmphasized", "ink", "stone-2"),
    ("borderError", "ink-plum", "rose-2"),
    ("borderSuccess", "green-2", "green-1"),
    ("borderWarning", "wood-2", "straw-2"),
    ("borderInfo", "blue-3", "water-2"),
    // Error fills (the Delete button, error badges and dots) are the earthy
    // rose-3 with ink text; rose-2 stays for hearts and message boxes.
    ("error", "rose-3", "rose-3"),
    ("errorMuted", "rose-2", "rose-2"),
    ("onError", "ink", "ink"),
    ("onErrorMuted", "ink-plum", "rose-1"),
    ("success", "green-2", "green-1"),
    ("successMuted", "green-2", "green-1"),
    ("onSuccess", "paper", "ink"),
    ("onSuccessMuted", "leaf-4", "mint"),
    ("warning", "straw-2", "straw-2"),
    ("warningMuted", "straw-2", "straw-2"),
    ("onWarning", "ink", "ink"),
    ("onWarningMuted", "wood-2", "straw-2"),
    ("info", "blue-3", "teal-1"),
    ("infoMuted", "blue-3", "teal-1"),
    ("onInfo", "paper", "ink"),
    ("onInfoMuted", "ink", "water-2"),
    ("onDark", "paper", "paper"),
    ("onLight", "ink", "ink"),
    // Secondary buttons, neutral badges, and plates: an opaque warm surface
    // from the background's family, not a tint of ink.
    ("neutral", "stone-1", "soil-1"),
    ("overlay", "ink", "ink"),
    ("overlayHover", "ink", "paper"),
    ("overlayPressed", "ink", "paper"),
    ("tintHover", "ink", "paper"),
    ("shadow", "ink", "ink"),
    ("skeleton", "stone-1", "blue-3"),
    ("track", "stone-1", "blue-3"),
    ("syntaxBackground", "paper-2", "ink"),
    // Palette v1.3 replaced soil-2 with plum-1; gray ramp tokens that took
    // soil-2 take soil-1.
    ("dataGray5", "soil-1", "soil-1"),
];

/// Tokens Terrace paints opaque, though Garden's value is a tint.
const OPAQUE: &[&str] = &["neutral"];

/// Swatches only the roles above may use, so nearest matches never pick
/// them: the dark accent stays the one plum-1 thing on screen.
const RESERVED: &[&str] = &["plum-1"];

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
                color: with_swatch(l.color, "ink"),
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
    let s = PALETTE.nearest_except(astryx_core::Color::rgba(r, g, b, 1.0), RESERVED);
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
        let pairs: [(&str, Get, Get, f32); 17] = [
            (
                "text on secondary button",
                |c| c.text_primary,
                |c| c.neutral,
                4.5,
            ),
            (
                "destructive focus ring on body",
                |c| c.error,
                |c| c.background_body,
                3.0,
            ),
            (
                "destructive focus ring on surface",
                |c| c.error,
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
        // Pairs the palette cannot meet, with the ratio they reach.
        // A light error fill cannot also be a 3:1 focus ring on light
        // ground; Button rings Destructive in `error` (it was 1.89 with
        // rose-2). Ringing it in `borderError` (ink-plum, 9.89) would pass.
        let known: [(&str, &str, f32); 2] = [
            ("light", "destructive focus ring on body", 2.0),
            ("light", "destructive focus ring on surface", 2.5),
        ];
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
