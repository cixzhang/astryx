//! Theme selection: which published theme and which color mode a subtree uses.

use crate::contract::{Color, ColorScheme, Preferences, Radii, ShadowLayer};
use astryx_tokens::{Rgba, Theme, Tokens};
use std::sync::{Mutex, OnceLock};

/// Light or dark color mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Mode {
    #[default]
    Light,
    Dark,
}

impl Mode {
    /// The other mode.
    pub fn toggled(self) -> Self {
        match self {
            Mode::Light => Mode::Dark,
            Mode::Dark => Mode::Light,
        }
    }
}

/// A published theme and a color mode. Apps return one from
/// [`App::theme`](crate::App::theme); the runtime rebuilds when it changes.
/// Without a mode, the theme follows the person's light or dark preference.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeChoice {
    pub theme: &'static Theme,
    /// `None` follows the person's color scheme.
    pub mode: Option<Mode>,
}

impl ThemeChoice {
    /// `theme` pinned to `mode`, whatever the person prefers.
    pub const fn new(theme: &'static Theme, mode: Mode) -> Self {
        Self {
            theme,
            mode: Some(mode),
        }
    }

    /// `theme` in the person's preferred color scheme.
    pub const fn system(theme: &'static Theme) -> Self {
        Self { theme, mode: None }
    }

    /// The mode this choice draws in under `preferences`.
    pub fn resolved_mode(&self, preferences: &Preferences) -> Mode {
        self.mode.unwrap_or(match preferences.color_scheme {
            ColorScheme::Light => Mode::Light,
            ColorScheme::Dark => Mode::Dark,
        })
    }

    /// The tokens this choice paints with under `preferences`. With more
    /// contrast, borders take the emphasized border color, as the Swift port
    /// does under increased contrast.
    pub fn tokens(&self, preferences: &Preferences) -> &'static Tokens {
        let base = match self.resolved_mode(preferences) {
            Mode::Light => &self.theme.light,
            Mode::Dark => &self.theme.dark,
        };
        if preferences.more_contrast {
            more_contrast(base)
        } else {
            base
        }
    }
}

impl Default for ThemeChoice {
    fn default() -> Self {
        Self::system(Theme::ASTRYX)
    }
}

/// The higher-contrast variant of a theme's tokens, made once per theme and
/// mode and kept for the life of the program.
fn more_contrast(base: &'static Tokens) -> &'static Tokens {
    static CACHE: OnceLock<Mutex<Vec<(usize, &'static Tokens)>>> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let key = base as *const Tokens as usize;
    if let Some(&(_, t)) = cache.iter().find(|(k, _)| *k == key) {
        return t;
    }
    let mut tokens = *base;
    tokens.color.border = tokens.color.border_emphasized;
    let leaked: &'static Tokens = Box::leak(Box::new(tokens));
    cache.push((key, leaked));
    leaked
}

impl From<Rgba> for Color {
    fn from([r, g, b, a]: Rgba) -> Self {
        Color { r, g, b, a }
    }
}

pub(crate) fn shadow_layers(layers: &[astryx_tokens::ShadowLayer]) -> Vec<ShadowLayer> {
    layers
        .iter()
        .map(|l| ShadowLayer {
            x: l.x,
            y: l.y,
            blur: l.blur,
            spread: l.spread,
            inset: l.is_inset,
            color: l.color.into(),
        })
        .collect()
}

/// Radii no larger than half the shorter side, so a `full` radius makes a
/// pill or a circle.
pub(crate) fn clamp_radii(width: f32, height: f32, radii: Radii) -> Radii {
    let max = (width.min(height) / 2.0).max(0.0);
    Radii(radii.0.map(|r| r.min(max)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_choice_follows_the_color_scheme_and_pinned_does_not() {
        let dark = Preferences {
            color_scheme: ColorScheme::Dark,
            ..Preferences::default()
        };
        let system = ThemeChoice::system(Theme::NEUTRAL);
        assert!(std::ptr::eq(system.tokens(&dark), &Theme::NEUTRAL.dark));
        assert!(std::ptr::eq(
            system.tokens(&Preferences::default()),
            &Theme::NEUTRAL.light
        ));
        let pinned = ThemeChoice::new(Theme::NEUTRAL, Mode::Light);
        assert!(std::ptr::eq(pinned.tokens(&dark), &Theme::NEUTRAL.light));
    }

    #[test]
    fn more_contrast_emphasizes_borders_once() {
        let prefs = Preferences {
            more_contrast: true,
            ..Preferences::default()
        };
        let choice = ThemeChoice::new(Theme::ASTRYX, Mode::Light);
        let a = choice.tokens(&prefs);
        assert_eq!(a.color.border, Theme::ASTRYX.light.color.border_emphasized);
        assert_eq!(a.color.accent, Theme::ASTRYX.light.color.accent);
        assert!(std::ptr::eq(a, choice.tokens(&prefs)), "cached");
    }

    #[test]
    fn radii_clamp_to_half_the_short_side() {
        assert_eq!(
            clamp_radii(40.0, 24.0, Radii::all(9999.0)),
            Radii::all(12.0)
        );
    }
}
