//! Themes: every Astryx brand in light and dark, switched at runtime, plus
//! the few tokens the generated set does not carry.

use astryx_core::{Color, Mode, ThemeChoice};
use astryx_tokens::{Theme, Tokens};

/// Which brand and color mode an app paints with. Keep one in app state and
/// return [`ThemeState::choice`] from `App::theme`; changing it is an
/// ordinary message, and the runtime repaints without a reload.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeState {
    pub brand: &'static Theme,
    /// `None` follows the person's light or dark preference.
    pub mode: Option<Mode>,
}

impl Default for ThemeState {
    fn default() -> Self {
        ThemeState {
            brand: Theme::ASTRYX,
            mode: None,
        }
    }
}

impl ThemeState {
    /// Every published brand, the default first.
    pub fn brands() -> &'static [&'static Theme] {
        Theme::ALL
    }

    pub fn by_name(name: &str) -> Option<ThemeState> {
        Theme::by_name(name).map(|brand| ThemeState { brand, mode: None })
    }

    pub fn choice(&self) -> ThemeChoice {
        ThemeChoice {
            theme: self.brand,
            mode: self.mode,
        }
    }

    /// The next brand in [`Self::brands`], wrapping.
    pub fn next_brand(self) -> Self {
        let all = Self::brands();
        let ix = all
            .iter()
            .position(|t| std::ptr::eq(*t, self.brand))
            .unwrap_or(0);
        ThemeState {
            brand: all[(ix + 1) % all.len()],
            ..self
        }
    }

    pub fn with_mode(self, mode: Option<Mode>) -> Self {
        ThemeState { mode, ..self }
    }
}

/// Tokens Astryx's components read that the generated token set does not
/// carry, derived for a theme and mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExtraTokens {
    /// `--focus-outline-*`: a solid ring in the accent color.
    pub focus_outline_color: Color,
    pub focus_outline_width: f32,
    pub focus_outline_offset: f32,
    /// `--shadow-inset-*`: a 2 point inset ring at 30% for field status.
    pub shadow_inset_success: Color,
    pub shadow_inset_warning: Color,
    pub shadow_inset_error: Color,
    pub shadow_inset_width: f32,
}

impl ExtraTokens {
    pub fn of(tokens: &Tokens) -> ExtraTokens {
        let c = |rgba: [f32; 4]| Color::from(rgba);
        ExtraTokens {
            focus_outline_color: c(tokens.color.accent),
            focus_outline_width: astryx_core::FOCUS_RING_WIDTH,
            focus_outline_offset: astryx_core::FOCUS_RING_OFFSET,
            shadow_inset_success: Color::rgba(38.0 / 255.0, 167.0 / 255.0, 86.0 / 255.0, 0.3),
            shadow_inset_warning: Color::rgba(226.0 / 255.0, 164.0 / 255.0, 0.0, 0.3),
            shadow_inset_error: Color::rgba(227.0 / 255.0, 25.0 / 255.0, 59.0 / 255.0, 0.3),
            shadow_inset_width: 2.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astryx_core::Preferences;

    #[test]
    fn eight_brands_each_with_light_and_dark() {
        assert_eq!(ThemeState::brands().len(), 8);
        // Gothic is dark in both modes by design; every other brand differs.
        let distinct = ThemeState::brands()
            .iter()
            .filter(|b| b.light.color.background_body != b.dark.color.background_body)
            .count();
        assert!(distinct >= 7, "{distinct}");
    }

    #[test]
    fn switching_brand_and_mode_changes_tokens() {
        let prefs = Preferences::default();
        let a = ThemeState::default().with_mode(Some(Mode::Light));
        let b = a.next_brand();
        assert!(!std::ptr::eq(a.brand, b.brand));
        let dark = a.with_mode(Some(Mode::Dark));
        assert!(std::ptr::eq(dark.choice().tokens(&prefs), &a.brand.dark));
        let mut s = a;
        for _ in 0..ThemeState::brands().len() {
            s = s.next_brand();
        }
        assert!(std::ptr::eq(s.brand, a.brand), "cycles");
    }

    #[test]
    fn extras_follow_the_accent() {
        let t = &Theme::ASTRYX.dark;
        assert_eq!(
            ExtraTokens::of(t).focus_outline_color,
            Color::from(t.color.accent)
        );
    }
}
