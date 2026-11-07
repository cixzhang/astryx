//! The Reduce Motion policy (`design:reduced-motion`, `spec:AST-108` FR4, FR6).
//!
//! Under reduced motion nothing travels, scales, resizes or springs (DR1):
//! - with a coarse primary pointer (touch), a transition becomes an
//!   opacity-only cross-fade of 200 ms or less (DR2);
//! - with any other pointer (mouse, keyboard, gamepad), it lands at once (DR3);
//! - an in-place value and a color change of state apply at once on any
//!   pointer (DEC-3, DEC-4); a touch press's release keeps its short fade;
//! - content the person moves by hand, and the momentum of a throw, keep
//!   moving: they are not transitions (DR2, DEC-2).
//!
//! No option loosens it (DR4). A theme on `always` reduces motion for
//! everyone (DR2a).

use crate::animated::Motion;
use crate::easing::{CubicBezier, Easing};
use crate::tokens::{MotionTokens, ThemeReducedMotion, REDUCED_MOTION_CROSS_FADE_CURVE};

/// The primary pointer, as `(pointer: coarse)` reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pointer {
    /// Mouse, trackpad, pen, keyboard or gamepad focus.
    #[default]
    Fine,
    /// A touch screen.
    Coarse,
}

/// What kind of change is about to animate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// An entrance, exit, switch between presentations, or change of state
    /// that moves, scales or resizes (DR1).
    Transition,
    /// A transition a touch drag's release starts, such as a snap or a
    /// dismissal: it cross-fades whatever the primary pointer (DR2).
    TouchReleaseTransition,
    /// A continuous value changing in place, such as a progress fill (DEC-3).
    InPlaceValue,
    /// A hover or selection color, or the onset of a press color (DEC-4).
    Color,
    /// A press highlight fading out after release; `touch` when a finger
    /// pressed (DEC-4 keeps its fade of 200 ms or less).
    PressRelease { touch: bool },
    /// Content following the pointer or finger (not a transition).
    DirectManipulation,
    /// The coast that continues a throw, and its settle at an edge (DEC-2).
    Momentum,
}

/// What a change becomes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Plan {
    /// Play the motion as designed.
    Animate,
    /// Land in one frame.
    Instant,
    /// Opacity only: land the change while transparent (see [`CrossFade`]).
    CrossFade {
        duration_ms: f32,
        easing: CubicBezier,
    },
}

/// The person's and the theme's motion settings.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MotionPolicy {
    /// The person's Reduce Motion setting (`prefers-reduced-motion: reduce`).
    pub prefers_reduced_motion: bool,
    pub pointer: Pointer,
}

impl MotionPolicy {
    /// Whether motion is reduced for this theme: the person's setting, or
    /// the theme's `always` (DR2a).
    pub fn reduces(&self, tokens: &MotionTokens) -> bool {
        self.prefers_reduced_motion || tokens.knobs.reduced_motion == ThemeReducedMotion::Always
    }

    /// What `change` becomes under these settings.
    pub fn plan(&self, change: Change, tokens: &MotionTokens) -> Plan {
        if !self.reduces(tokens) {
            return Plan::Animate;
        }
        let fade = Plan::CrossFade {
            duration_ms: tokens.reduced_motion_cross_fade_ms(),
            easing: REDUCED_MOTION_CROSS_FADE_CURVE,
        };
        match change {
            Change::DirectManipulation | Change::Momentum => Plan::Animate,
            Change::InPlaceValue | Change::Color => Plan::Instant,
            Change::TouchReleaseTransition => fade,
            Change::PressRelease { touch: true } => fade,
            Change::PressRelease { touch: false } => Plan::Instant,
            Change::Transition => match self.pointer {
                Pointer::Coarse => fade,
                Pointer::Fine => Plan::Instant,
            },
        }
    }

    /// `motion` for a value that is not opacity, under this policy: travel
    /// becomes instant whenever motion is reduced, since a cross-fade
    /// changes opacity only (DR1). Pair it with [`CrossFade`] for opacity.
    pub fn travel(&self, change: Change, motion: Motion, tokens: &MotionTokens) -> Motion {
        match self.plan(change, tokens) {
            Plan::Animate => motion,
            Plan::Instant | Plan::CrossFade { .. } => Motion::Instant,
        }
    }
}

/// How a cross-fade connects the two states (core reducedMotionPlan.ts).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FadeShape {
    /// An entering element appears in its final place and fades in.
    Enter,
    /// A leaving element fades out where it is, then lands.
    Exit,
    /// Anything else that moves dips: fades out, lands, fades back in.
    Dip,
}

/// A Reduce Motion cross-fade: opacity over time, and when the change lands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrossFade {
    pub shape: FadeShape,
    pub duration_ms: f32,
    pub easing: CubicBezier,
}

impl CrossFade {
    /// The opacity multiplier `elapsed_ms` into the fade (1 = fully shown).
    pub fn opacity(&self, elapsed_ms: f32) -> f32 {
        let d = self.duration_ms.max(1e-3);
        let e = Easing::CubicBezier(self.easing);
        let x = (elapsed_ms / d).clamp(0.0, 1.0);
        match self.shape {
            FadeShape::Enter => e.ease(x),
            FadeShape::Exit => 1.0 - e.ease(x),
            FadeShape::Dip if x < 0.5 => 1.0 - e.ease(x * 2.0),
            FadeShape::Dip => e.ease(x * 2.0 - 1.0),
        }
    }

    /// When the change itself (position, size, content) should land, in ms:
    /// at once for an entrance, at the end for an exit, while transparent for
    /// a dip.
    pub fn lands_at_ms(&self) -> f32 {
        match self.shape {
            FadeShape::Enter => 0.0,
            FadeShape::Exit => self.duration_ms,
            FadeShape::Dip => self.duration_ms / 2.0,
        }
    }

    pub fn is_done(&self, elapsed_ms: f32) -> bool {
        elapsed_ms >= self.duration_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::{ThemeMotionKnobs, ASTRYX};

    const REDUCED_FINE: MotionPolicy = MotionPolicy {
        prefers_reduced_motion: true,
        pointer: Pointer::Fine,
    };
    const REDUCED_TOUCH: MotionPolicy = MotionPolicy {
        prefers_reduced_motion: true,
        pointer: Pointer::Coarse,
    };

    #[test]
    fn no_preference_animates() {
        let p = MotionPolicy::default();
        assert_eq!(p.plan(Change::Transition, &ASTRYX), Plan::Animate);
        assert_eq!(p.plan(Change::Color, &ASTRYX), Plan::Animate);
    }

    #[test]
    fn fine_pointer_lands_at_once() {
        for c in [
            Change::Transition,
            Change::InPlaceValue,
            Change::Color,
            Change::PressRelease { touch: false },
        ] {
            assert_eq!(REDUCED_FINE.plan(c, &ASTRYX), Plan::Instant, "{c:?}");
        }
    }

    #[test]
    fn touch_transitions_cross_fade_within_200ms() {
        match REDUCED_TOUCH.plan(Change::Transition, &ASTRYX) {
            Plan::CrossFade { duration_ms, .. } => assert!(duration_ms <= 200.0),
            other => panic!("{other:?}"),
        }
        // A drag's release cross-fades even with a fine primary pointer.
        assert!(matches!(
            REDUCED_FINE.plan(Change::TouchReleaseTransition, &ASTRYX),
            Plan::CrossFade { .. }
        ));
        // Values and colors apply at once on touch too.
        assert_eq!(
            REDUCED_TOUCH.plan(Change::InPlaceValue, &ASTRYX),
            Plan::Instant
        );
        assert_eq!(REDUCED_TOUCH.plan(Change::Color, &ASTRYX), Plan::Instant);
        assert!(matches!(
            REDUCED_TOUCH.plan(Change::PressRelease { touch: true }, &ASTRYX),
            Plan::CrossFade { .. }
        ));
    }

    #[test]
    fn hands_and_momentum_keep_moving() {
        assert_eq!(
            REDUCED_TOUCH.plan(Change::DirectManipulation, &ASTRYX),
            Plan::Animate
        );
        assert_eq!(REDUCED_FINE.plan(Change::Momentum, &ASTRYX), Plan::Animate);
    }

    #[test]
    fn travel_never_animates_under_reduced_motion() {
        let spring = Motion::Spring(crate::tokens::springs::SETTLE.spring());
        assert_eq!(
            REDUCED_TOUCH.travel(Change::Transition, spring, &ASTRYX),
            Motion::Instant
        );
        assert_eq!(
            MotionPolicy::default().travel(Change::Transition, spring, &ASTRYX),
            spring
        );
    }

    #[test]
    fn a_theme_on_always_reduces_for_everyone() {
        let always = ASTRYX.with_knobs(ThemeMotionKnobs {
            reduced_motion: Some(ThemeReducedMotion::Always),
            ..Default::default()
        });
        assert_eq!(
            MotionPolicy::default().plan(Change::Transition, &always),
            Plan::Instant
        );
    }

    #[test]
    fn cross_fade_shapes() {
        let f = |shape| CrossFade {
            shape,
            duration_ms: 200.0,
            easing: REDUCED_MOTION_CROSS_FADE_CURVE,
        };
        let enter = f(FadeShape::Enter);
        assert_eq!(enter.opacity(0.0), 0.0);
        assert_eq!(enter.opacity(200.0), 1.0);
        assert_eq!(enter.lands_at_ms(), 0.0);
        let exit = f(FadeShape::Exit);
        assert_eq!(exit.opacity(200.0), 0.0);
        assert_eq!(exit.lands_at_ms(), 200.0);
        let dip = f(FadeShape::Dip);
        assert_eq!(dip.opacity(100.0), 0.0);
        assert_eq!(dip.opacity(200.0), 1.0);
        assert_eq!(dip.lands_at_ms(), 100.0);
    }
}
