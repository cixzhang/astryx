//! Each theme's motion tokens (`spec:AST-108`), generated from
//! `tokens/motion.json` by `tools/export-motion-tokens.py`, which reads
//! Astryx core's sources.

use crate::easing::CubicBezier;
use crate::spring::SpringSpec;

/// How lively a theme's motion feels (`spec:AST-108` FR1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Character {
    /// Decelerates without overshoot.
    Crisp,
    /// Astryx's own.
    #[default]
    Standard,
    /// Overshoots slightly on entrances and moves; exits and fades never do.
    Expressive,
}

/// Whose Reduce Motion setting a theme follows (`spec:AST-108` FR1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeReducedMotion {
    /// Each person's own setting.
    #[default]
    System,
    /// Reduced for everyone (`design:reduced-motion` DR2a).
    Always,
}

/// The knobs a theme sets; `None` keeps the base theme's value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ThemeMotionKnobs {
    pub tempo: Option<f32>,
    pub character: Option<Character>,
    pub reduced_motion: Option<ThemeReducedMotion>,
}

/// The knobs with defaults filled in and `tempo` clamped.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedKnobs {
    pub tempo: f32,
    pub character: Character,
    pub reduced_motion: ThemeReducedMotion,
    /// Whether the theme sets `tempo` itself: only then do springs follow it
    /// (`spec:AST-108` FR5).
    pub tempo_is_set: bool,
}

/// Duration tokens in milliseconds (`--duration-*`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Durations {
    pub fast_min: f32,
    pub fast: f32,
    pub fast_max: f32,
    pub medium_min: f32,
    pub medium: f32,
    pub medium_max: f32,
    pub slow_min: f32,
    pub slow: f32,
    pub slow_max: f32,
    /// The standard dismissal duration, `var(--duration-fast)` (`spec:AST-090` FR12).
    pub exit: f32,
    /// A dialog surface's exit; 0 turns its exit off (`spec:AST-090` FR1, FR2).
    pub exit_dialog: f32,
    /// A layer surface's exit; 0 turns its exit off (`spec:AST-090` FR1, FR2).
    pub exit_layer: f32,
}

/// `--duration-exit-dialog` and `--duration-exit-layer` default to 0 ms:
/// exits are off until a theme turns a kind on (`spec:AST-090` FR1).
pub const EXIT_KIND_DEFAULT_MS: f32 = 0.0;

/// One theme's motion tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionTokens {
    pub name: &'static str,
    pub knobs: ResolvedKnobs,
    /// The theme sets a duration scale, whose durations win over `tempo`.
    pub duration_scale: bool,
    /// The theme's duration scale sets its own `--ease-standard`.
    pub easing_override: bool,
    pub durations: Durations,
    /// `--ease-standard`: arrivals and state changes.
    pub ease_standard: CubicBezier,
    /// `--ease-entrance`: a layer or element arriving.
    pub ease_entrance: CubicBezier,
    /// `--ease-move`: an element changing place or size.
    pub ease_move: CubicBezier,
    /// `--ease-exit`: things leaving; the same at every character.
    pub ease_exit: CubicBezier,
}

include!(concat!(env!("OUT_DIR"), "/motion_tokens.rs"));

/// The default theme's motion tokens.
pub static DEFAULT: &MotionTokens = &ASTRYX;

impl MotionTokens {
    /// A published theme's motion tokens by name.
    pub fn by_name(name: &str) -> Option<&'static MotionTokens> {
        ALL.iter().copied().find(|t| t.name == name)
    }

    /// The tokens with `knobs` applied, as core's `expandThemeMotion` does
    /// (`spec:AST-108` FR1, FR2, FR5):
    /// - `tempo`, clamped, scales every duration to the ms, unless the theme's
    ///   own duration scale sets them;
    /// - `character` sets the entrance and move curves; `standard` is the
    ///   theme's `--ease-standard`;
    /// - exits keep their curve at every character.
    pub fn with_knobs(&self, knobs: ThemeMotionKnobs) -> MotionTokens {
        let mut out = *self;
        if let Some(tempo) = knobs.tempo.filter(|t| t.is_finite()) {
            let tempo = tempo.clamp(TEMPO_RANGE.0, TEMPO_RANGE.1);
            out.knobs.tempo = tempo;
            out.knobs.tempo_is_set = true;
            if !self.duration_scale {
                let d = &mut out.durations;
                for v in [
                    &mut d.fast_min,
                    &mut d.fast,
                    &mut d.fast_max,
                    &mut d.medium_min,
                    &mut d.medium,
                    &mut d.medium_max,
                    &mut d.slow_min,
                    &mut d.slow,
                    &mut d.slow_max,
                ] {
                    *v = js_round(*v * tempo);
                }
                d.exit = d.fast;
            }
        }
        if let Some(character) = knobs.character {
            out.knobs.character = character;
            let ease = match character {
                Character::Crisp => CRISP_EASE,
                Character::Expressive => EXPRESSIVE_EASE,
                Character::Standard => self.ease_standard,
            };
            out.ease_entrance = ease;
            out.ease_move = ease;
        }
        if let Some(rm) = knobs.reduced_motion {
            // `always` only adds: no knob loosens Reduce Motion (DR2a).
            if rm == ThemeReducedMotion::Always {
                out.knobs.reduced_motion = rm;
            }
        }
        out
    }

    /// A spring role at this theme's tempo. Springs follow `tempo` only when
    /// the theme sets it (`spec:AST-108` FR3, FR5); finger tracking never
    /// does (FR4).
    pub fn spring(&self, role: SpringSpec) -> SpringSpec {
        if self.knobs.tempo_is_set && role != springs::FINGER_TRACKING {
            role.at_tempo(self.knobs.tempo)
        } else {
            role
        }
    }

    /// The Reduce Motion cross-fade length: `tempo` may shorten it but never
    /// lengthens it past 200 ms (`spec:AST-108` FR4, `design:reduced-motion` DR2).
    pub fn reduced_motion_cross_fade_ms(&self) -> f32 {
        REDUCED_MOTION_CROSS_FADE_MS * self.knobs.tempo.min(1.0)
    }
}

/// JavaScript's `Math.round`: halves round up.
fn js_round(x: f32) -> f32 {
    (x + 0.5).floor()
}

#[cfg(test)]
mod tests {
    use super::*;

    // The released values (spec:AST-108 FR2) at tempo 1 and 'standard'.
    #[test]
    fn default_theme_keeps_the_released_tokens() {
        let d = ASTRYX.durations;
        let got = [
            d.fast_min,
            d.fast,
            d.fast_max,
            d.medium_min,
            d.medium,
            d.medium_max,
            d.slow_min,
            d.slow,
            d.slow_max,
        ];
        assert_eq!(
            got,
            [130.0, 175.0, 230.0, 310.0, 410.0, 550.0, 730.0, 975.0, 1300.0]
        );
        assert_eq!(d.exit, d.fast);
        assert_eq!(d.exit_dialog, 0.0);
        assert_eq!(d.exit_layer, 0.0);
        assert_eq!(ASTRYX.ease_standard, CubicBezier::new(0.24, 1.0, 0.4, 1.0));
        assert_eq!(ASTRYX.ease_entrance, ASTRYX.ease_standard);
        assert_eq!(ASTRYX.ease_move, ASTRYX.ease_standard);
        assert_eq!(ASTRYX.ease_exit, CubicBezier::new(0.3, 0.0, 0.6, 0.6));
    }

    // expandMotionScale: min = base × ratio, max = base / ratio, to 5 ms.
    #[test]
    fn scale_themes_follow_expand_motion_scale() {
        let n = MotionTokens::by_name("neutral").unwrap().durations;
        assert_eq!([n.fast_min, n.fast, n.fast_max], [95.0, 125.0, 165.0]);
        assert_eq!(
            [n.medium_min, n.medium, n.medium_max],
            [225.0, 300.0, 400.0]
        );
        assert_eq!([n.slow_min, n.slow, n.slow_max], [525.0, 700.0, 935.0]);
        let y = MotionTokens::by_name("y2k").unwrap().durations;
        assert_eq!([y.fast_min, y.fast, y.fast_max], [80.0, 100.0, 125.0]);
        assert_eq!(ALL.len(), 8);
        assert_eq!(ALL[0].name, "astryx");
    }

    #[test]
    fn tempo_scales_durations_and_is_clamped() {
        let t = ASTRYX.with_knobs(ThemeMotionKnobs {
            tempo: Some(2.0),
            ..Default::default()
        });
        assert_eq!(t.knobs.tempo, 1.5);
        assert_eq!(t.durations.fast, js_round(175.0 * 1.5));
        assert_eq!(t.durations.exit, t.durations.fast);
        let t = ASTRYX.with_knobs(ThemeMotionKnobs {
            tempo: Some(0.8),
            ..Default::default()
        });
        assert_eq!(t.durations.medium, 328.0);
    }

    #[test]
    fn a_duration_scale_wins_over_tempo() {
        let n = MotionTokens::by_name("neutral").unwrap();
        let t = n.with_knobs(ThemeMotionKnobs {
            tempo: Some(1.5),
            ..Default::default()
        });
        assert_eq!(t.durations, n.durations);
        assert!(t.knobs.tempo_is_set);
    }

    #[test]
    fn character_sets_entrance_and_move_only() {
        let t = ASTRYX.with_knobs(ThemeMotionKnobs {
            character: Some(Character::Crisp),
            ..Default::default()
        });
        assert_eq!(t.ease_entrance, CRISP_EASE);
        assert_eq!(t.ease_move, CRISP_EASE);
        assert_eq!(t.ease_exit, ASTRYX.ease_exit);
        assert_eq!(t.ease_standard, ASTRYX.ease_standard);
        let e = ASTRYX.with_knobs(ThemeMotionKnobs {
            character: Some(Character::Expressive),
            ..Default::default()
        });
        assert_eq!(e.ease_entrance, CubicBezier::new(0.3, 1.2, 0.5, 1.0));
    }

    #[test]
    fn springs_follow_tempo_only_when_set_and_finger_tracking_never() {
        let role = springs::STATE_CHANGE;
        assert_eq!(ASTRYX.spring(role), role);
        let slow = ASTRYX.with_knobs(ThemeMotionKnobs {
            tempo: Some(1.2),
            ..Default::default()
        });
        assert_eq!(slow.spring(role), role.at_tempo(1.2));
        assert_eq!(
            slow.spring(springs::FINGER_TRACKING),
            springs::FINGER_TRACKING
        );
    }

    #[test]
    fn cross_fade_never_lengthens() {
        let slow = ASTRYX.with_knobs(ThemeMotionKnobs {
            tempo: Some(1.5),
            ..Default::default()
        });
        assert_eq!(slow.reduced_motion_cross_fade_ms(), 200.0);
        let fast = ASTRYX.with_knobs(ThemeMotionKnobs {
            tempo: Some(0.75),
            ..Default::default()
        });
        assert_eq!(fast.reduced_motion_cross_fade_ms(), 150.0);
    }

    #[test]
    fn always_only_adds() {
        let a = ASTRYX.with_knobs(ThemeMotionKnobs {
            reduced_motion: Some(ThemeReducedMotion::Always),
            ..Default::default()
        });
        let b = a.with_knobs(ThemeMotionKnobs {
            reduced_motion: Some(ThemeReducedMotion::System),
            ..Default::default()
        });
        assert_eq!(b.knobs.reduced_motion, ThemeReducedMotion::Always);
    }
}
