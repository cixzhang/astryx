//! Astryx motion presets on the Rust stack.
//!
//! The core runtime plays animations (see `astryx_core::motion`); this
//! crate says which motion Astryx uses where, from the theme's motion
//! tokens, so components read the same durations, curves and springs as
//! Astryx's TypeScript components:
//!
//! - [`layer_enter`] and [`layer_exit`]: the layer runtime's keyframes
//!   (fade, an 8 point slide from the anchor, and a 0.95 scale).
//! - [`state_change`]: a toggle's travel (a Switch thumb) on a spring.
//! - [`collapse`]: an animated height that reflows the page (Collapsible).
//! - [`tokens`]: the motion tokens for the theme being built.

use astryx_core::authoring::BuildCx;
use astryx_core::motion::{Anim, Change, Channel, Exit, Motion, Resize};
use astryx_motion::tokens::{self as motion_tokens, springs, MotionTokens};

pub use astryx_core::motion;

/// The side a layer opens on, for its entrance slide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum From {
    Above,
    Below,
    Start,
    End,
    /// Centered layers (dialogs): fade and scale only.
    Center,
}

/// The motion tokens of the theme being built.
pub fn tokens<Msg>(cx: &BuildCx<Msg>) -> &'static MotionTokens {
    MotionTokens::by_name(cx.theme.name).unwrap_or(motion_tokens::DEFAULT)
}

/// How far a layer slides in, in points: Astryx's spacing-2 step.
pub const LAYER_SLIDE: f32 = 8.0;
/// The scale a layer grows from.
pub const LAYER_SCALE_FROM: f32 = 0.95;

/// A layer's entrance, as Astryx's layer animations play it: opacity 0 to
/// 1, a slide from the anchor side, and a scale from 0.95, over
/// `--duration-fast-max` on `--ease-standard`. Reduce Motion drops the
/// travel and keeps a cross-fade on touch screens.
pub fn layer_enter(t: &MotionTokens, from: From, rtl: bool) -> Vec<Anim> {
    let motion = Motion::curve(t.durations.fast_max, t.ease_standard);
    let mut out = vec![
        Anim::new(Channel::Opacity, 1.0, motion).from(0.0),
        Anim::new(Channel::Scale, 1.0, motion).from(LAYER_SCALE_FROM),
    ];
    let start_sign = if rtl { 1.0 } else { -1.0 };
    let slide = match from {
        From::Below => Some((Channel::TranslateY, -LAYER_SLIDE)),
        From::Above => Some((Channel::TranslateY, LAYER_SLIDE)),
        From::End => Some((Channel::TranslateX, start_sign * LAYER_SLIDE)),
        From::Start => Some((Channel::TranslateX, -start_sign * LAYER_SLIDE)),
        From::Center => None,
    };
    if let Some((channel, offset)) = slide {
        out.push(Anim::new(channel, 0.0, motion).from(offset));
    }
    out
}

/// A layer's exit: a fade over the theme's layer exit time on
/// `--ease-exit`, played once on an exit copy.
pub fn layer_exit(t: &MotionTokens) -> Exit {
    let ms = if t.durations.exit_layer > 0.0 {
        t.durations.exit_layer
    } else {
        t.durations.exit
    };
    Exit::fade(Motion::curve(ms, t.ease_exit))
}

/// A toggle's travel toward `to` (a Switch thumb's offset) on the
/// `stateChange` spring. A new target mid-way retargets from the current
/// position and velocity.
pub fn state_change(t: &MotionTokens, channel: Channel, to: f32) -> Anim {
    let spring = t.spring(springs::STATE_CHANGE).spring();
    Anim::new(channel, to, Motion::Spring(spring))
}

/// An expanding or collapsing height (Collapsible), on `--duration-medium`
/// and `--ease-move`.
pub fn collapse(t: &MotionTokens) -> Resize {
    Resize::new(Motion::curve(t.durations.medium, t.ease_move))
}

/// A color change of state (hover, selection): instant under Reduce Motion
/// on any pointer.
pub fn color_change(t: &MotionTokens, channel: Channel, to: f32) -> Anim {
    Anim::new(channel, to, Motion::curve(t.durations.fast, t.ease_standard)).change(Change::Color)
}

#[cfg(test)]
mod tests;
