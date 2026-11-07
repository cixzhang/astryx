//! Astryx motion for Rust: portable, no GPU, window, or OS dependency.
//!
//! - [`tokens`]: each theme's motion tokens (durations, curves, knobs) and
//!   the spring roles, generated from `tokens/motion.json` (`spec:AST-108`).
//! - [`easing`]: CSS easing functions as Astryx uses them: `cubic-bezier()`,
//!   `steps()`, `linear`.
//! - [`spring`]: the damped-spring model Astryx's animator uses, solved in
//!   closed form.
//! - [`animated`]: [`Animated`], a value driven by a frame clock, with
//!   interruption rules for curves and springs.
//! - [`presence`]: enter and exit of a surface (`spec:AST-090`).
//! - [`reduced`]: the Reduce Motion policy (`design:reduced-motion`).
//! - [`gesture`]: focus-first gestures: press, long press, hold-to-repeat,
//!   paging.

pub mod animated;
pub mod easing;
pub mod gesture;
pub mod presence;
pub mod reduced;
pub mod spring;
pub mod tokens;

pub use animated::{Animatable, Animated, Motion};
pub use easing::{CubicBezier, Easing, StepPosition};
pub use reduced::{Change, MotionPolicy, Plan, Pointer};
pub use spring::{Spring, SpringSpec};
pub use tokens::{Character, MotionTokens, ThemeMotionKnobs};
