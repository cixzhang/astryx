//! Terrace: the pixel theme drawn only from [`PALETTE`]. Garden's sizes,
//! type, and shapes, with every color a palette swatch: UI ink is `ink`,
//! UI paper is `paper` and `paper-2`, the accent is `green-2`, errors are
//! `rose-2`, and dark mode sits on `soil-4`, `ink`, and `blue-3` with
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
    ("textSecondary", "blue-3", "water-2"),
    ("textDisabled", "stone-2", "stone-2"),
    ("textAccent", "green-2", "green-1"),
    // Rose text on paper is unreadable (1.9:1), so light error text and
    // icons are ink-plum; rose-2 stays the error fill and the dark text.
    ("textError", "ink-plum", "rose-2"),
    ("textSuccess", "green-2", "green-1"),
    ("textWarning", "wood-2", "straw-2"),
    ("textInfo", "blue-3", "water-2"),
    ("iconPrimary", "ink", "paper"),
    ("iconSecondary", "blue-3", "water-2"),
    ("iconDisabled", "stone-2", "stone-2"),
    ("iconAccent", "green-2", "green-1"),
    ("iconError", "ink-plum", "rose-2"),
    ("iconSuccess", "green-2", "green-1"),
    ("iconWarning", "wood-2", "straw-2"),
    ("iconInfo", "blue-3", "water-2"),
    ("accent", "green-2", "green-2"),
    ("accentMuted", "green-2", "green-2"),
    ("onAccent", "paper", "paper"),
    ("border", "ink", "stone-2"),
    ("borderEmphasized", "ink", "stone-2"),
    ("borderError", "ink-plum", "rose-2"),
    ("borderSuccess", "green-2", "green-1"),
    ("borderWarning", "wood-2", "straw-2"),
    ("borderInfo", "blue-3", "water-2"),
    ("error", "rose-2", "rose-2"),
    ("errorMuted", "rose-2", "rose-2"),
    ("onError", "ink", "ink"),
    ("onErrorMuted", "ink-plum", "rose-2"),
    ("success", "green-2", "green-1"),
    ("successMuted", "green-2", "green-1"),
    ("onSuccess", "paper", "ink"),
    ("onSuccessMuted", "green-2", "green-1"),
    ("warning", "straw-2", "straw-2"),
    ("warningMuted", "straw-2", "straw-2"),
    ("onWarning", "ink", "ink"),
    ("onWarningMuted", "wood-2", "straw-2"),
    ("info", "blue-3", "teal-1"),
    ("infoMuted", "blue-3", "teal-1"),
    ("onInfo", "paper", "ink"),
    ("onInfoMuted", "blue-3", "water-2"),
    ("onDark", "paper", "paper"),
    ("onLight", "ink", "ink"),
    ("neutral", "ink", "paper"),
    ("overlay", "ink", "ink"),
    ("overlayHover", "ink", "paper"),
    ("overlayPressed", "ink", "paper"),
    ("tintHover", "ink", "paper"),
    ("shadow", "ink", "ink"),
    ("skeleton", "stone-1", "blue-3"),
    ("track", "stone-1", "blue-3"),
    ("syntaxBackground", "paper-2", "ink"),
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
    let s = PALETTE.nearest(astryx_core::Color::rgba(r, g, b, 1.0));
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
        let pairs: [(&str, Get, Get, f32); 12] = [
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
        // Pairs palette v1 cannot meet, with the ratio they reach.
        let known: [(&str, &str, f32); 2] = [
            ("light", "on-accent text on accent", 4.09),
            ("dark", "on-accent text on accent", 4.09),
        ];
        let mut failing = Vec::new();
        for (mode, tokens) in [("light", &t.light), ("dark", &t.dark)] {
            for (name, fg, bg, need) in pairs {
                let ratio = contrast(c(tokens, fg), c(tokens, bg));
                if ratio < need {
                    failing.push((mode, name, (ratio * 100.0).round() / 100.0));
                }
            }
        }
        assert_eq!(failing, known);
    }
}
