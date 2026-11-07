//! [`Animated`]: a value that moves toward a target, driven by a frame clock.
//!
//! Interruption follows the platforms Astryx matches:
//! - A spring started while anything runs starts from the current value
//!   **and velocity**, so a retarget never jolts (`spec:AST-107` FR3, the
//!   animator's `animate(target, {velocity})`; `spec:AST-090` FR8: a reopened
//!   surface returns to open from where it is, with no hide and show).
//! - A curve started while anything runs starts from the current value; a
//!   curve carries no velocity. When it reverses a running curve (its target
//!   is that curve's start), its duration shortens by how far the running
//!   curve got, as CSS transitions do (CSS Transitions Level 1, "reversing
//!   shortening factor").
//! - `set` jumps and stops; `stop` halts where the value is.

use crate::easing::Easing;
use crate::spring::{Spring, SETTLING_EPSILON};

/// A value [`Animated`] can drive: up to four `f32` components.
pub trait Animatable: Copy + PartialEq {
    /// How many components the value has (1 to 4).
    const LEN: usize;
    fn to_components(&self) -> [f32; 4];
    fn from_components(c: [f32; 4]) -> Self;
}

impl Animatable for f32 {
    const LEN: usize = 1;
    fn to_components(&self) -> [f32; 4] {
        [*self, 0.0, 0.0, 0.0]
    }
    fn from_components(c: [f32; 4]) -> Self {
        c[0]
    }
}

macro_rules! array_animatable {
    ($n:literal) => {
        impl Animatable for [f32; $n] {
            const LEN: usize = $n;
            fn to_components(&self) -> [f32; 4] {
                let mut out = [0.0; 4];
                out[..$n].copy_from_slice(self);
                out
            }
            fn from_components(c: [f32; 4]) -> Self {
                let mut out = [0.0; $n];
                out.copy_from_slice(&c[..$n]);
                out
            }
        }
    };
}
array_animatable!(2);
array_animatable!(3);
array_animatable!(4);

impl Animatable for (f32, f32) {
    const LEN: usize = 2;
    fn to_components(&self) -> [f32; 4] {
        [self.0, self.1, 0.0, 0.0]
    }
    fn from_components(c: [f32; 4]) -> Self {
        (c[0], c[1])
    }
}

/// How a change plays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Motion {
    /// Lands at once.
    Instant,
    /// A duration-and-curve transition, as CSS plays the theme's tokens.
    Curve { duration_ms: f32, easing: Easing },
    /// A spring, as the animator plays a motion role.
    Spring(Spring),
}

impl Motion {
    pub fn curve(duration_ms: f32, easing: impl Into<Easing>) -> Motion {
        if duration_ms <= 0.0 {
            Motion::Instant
        } else {
            Motion::Curve {
                duration_ms,
                easing: easing.into(),
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Run {
    Idle,
    Curve {
        from: [f32; 4],
        to: [f32; 4],
        elapsed: f32,
        duration: f32,
        easing: Easing,
    },
    Spring {
        spring: Spring,
        from: [f32; 4],
        v0: [f32; 4],
        elapsed: f64,
        end: f64,
    },
}

/// A value animated toward a target. Call [`Animated::tick`] once a frame
/// with the frame's `dt` in seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Animated<T: Animatable> {
    value: [f32; 4],
    velocity: [f32; 4],
    target: [f32; 4],
    run: Run,
    _value: std::marker::PhantomData<T>,
}

impl<T: Animatable> Animated<T> {
    pub fn new(value: T) -> Self {
        let c = value.to_components();
        Animated {
            value: c,
            velocity: [0.0; 4],
            target: c,
            run: Run::Idle,
            _value: std::marker::PhantomData,
        }
    }

    pub fn value(&self) -> T {
        T::from_components(self.value)
    }

    /// Units per second, per component.
    pub fn velocity(&self) -> T {
        T::from_components(self.velocity)
    }

    pub fn target(&self) -> T {
        T::from_components(self.target)
    }

    pub fn is_animating(&self) -> bool {
        !matches!(self.run, Run::Idle)
    }

    /// Jumps to `value` and stops.
    pub fn set(&mut self, value: T) {
        self.value = value.to_components();
        self.target = self.value;
        self.velocity = [0.0; 4];
        self.run = Run::Idle;
    }

    /// Halts where the value is now.
    pub fn stop(&mut self) {
        self.target = self.value;
        self.velocity = [0.0; 4];
        self.run = Run::Idle;
    }

    /// Moves toward `target` with `motion`, from wherever the value is now.
    /// The same target while already moving there changes nothing.
    pub fn animate_to(&mut self, target: T, motion: Motion) {
        let to = target.to_components();
        if to == self.target && self.is_animating() {
            return;
        }
        match motion {
            Motion::Instant => self.set(target),
            Motion::Curve {
                duration_ms,
                easing,
            } => {
                let mut duration = duration_ms / 1000.0;
                if let Run::Curve {
                    from,
                    elapsed,
                    duration: d,
                    easing: e,
                    ..
                } = self.run
                {
                    if from == to && d > 0.0 {
                        // CSS: a reversal plays for as long as the way back.
                        let factor = e.ease(elapsed / d).clamp(0.0, 1.0);
                        duration *= factor;
                    }
                }
                self.target = to;
                self.velocity = [0.0; 4];
                self.run = if duration <= 0.0 {
                    self.value = to;
                    Run::Idle
                } else {
                    Run::Curve {
                        from: self.value,
                        to,
                        elapsed: 0.0,
                        duration,
                        easing,
                    }
                };
            }
            Motion::Spring(spring) => {
                self.target = to;
                let mut end: f64 = 0.0;
                let lanes = self.value.iter().zip(&self.velocity).zip(&to).take(T::LEN);
                for ((&value, &velocity), &target) in lanes {
                    let c = spring.solve(value as f64, target as f64, velocity as f64);
                    let scale = ((value - target).abs() as f64).max(1.0);
                    end = end.max(c.settle_time(SETTLING_EPSILON * scale));
                }
                self.run = if end <= 0.0 {
                    self.value = to;
                    self.velocity = [0.0; 4];
                    Run::Idle
                } else {
                    Run::Spring {
                        spring,
                        from: self.value,
                        v0: self.velocity,
                        elapsed: 0.0,
                        end,
                    }
                };
            }
        }
    }

    /// Advances by `dt` seconds. Returns true while the value still moves.
    pub fn tick(&mut self, dt: f32) -> bool {
        match &mut self.run {
            Run::Idle => false,
            Run::Curve {
                from,
                to,
                elapsed,
                duration,
                easing,
            } => {
                *elapsed += dt.max(0.0);
                let x = (*elapsed / *duration).min(1.0);
                let p = easing.ease(x);
                let slope = easing.slope(x) / *duration;
                for i in 0..T::LEN {
                    let span = to[i] - from[i];
                    self.value[i] = from[i] + span * p;
                    self.velocity[i] = span * slope;
                }
                if x >= 1.0 {
                    self.value = *to;
                    self.velocity = [0.0; 4];
                    self.run = Run::Idle;
                    return false;
                }
                true
            }
            Run::Spring {
                spring,
                from,
                v0,
                elapsed,
                end,
            } => {
                *elapsed += dt.max(0.0) as f64;
                if *elapsed >= *end {
                    self.value = self.target;
                    self.velocity = [0.0; 4];
                    self.run = Run::Idle;
                    return false;
                }
                for i in 0..T::LEN {
                    let c = spring.solve(from[i] as f64, self.target[i] as f64, v0[i] as f64);
                    self.value[i] = c.value(*elapsed) as f32;
                    self.velocity[i] = c.velocity(*elapsed) as f32;
                }
                true
            }
        }
    }

    /// Seconds until the running motion lands (0 when idle). A part timed to
    /// land with a spring uses this (`spec:AST-108` FR4a).
    pub fn remaining(&self) -> f32 {
        match self.run {
            Run::Idle => 0.0,
            Run::Curve {
                elapsed, duration, ..
            } => (duration - elapsed).max(0.0),
            Run::Spring { elapsed, end, .. } => (end - elapsed).max(0.0) as f32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::{springs, ASTRYX};

    fn run<T: Animatable>(a: &mut Animated<T>, seconds: f32) {
        let steps = (seconds * 120.0).round() as usize;
        for _ in 0..steps {
            a.tick(1.0 / 120.0);
        }
    }

    #[test]
    fn a_curve_follows_its_easing_and_lands() {
        let ease = ASTRYX.ease_standard;
        let mut a = Animated::new(0.0f32);
        a.animate_to(100.0, Motion::curve(ASTRYX.durations.medium, ease));
        a.tick(0.1);
        let want = 100.0 * ease.ease(0.1 / 0.41);
        assert!((a.value() - want).abs() < 1e-3);
        run(&mut a, 0.5);
        assert_eq!(a.value(), 100.0);
        assert!(!a.is_animating());
    }

    #[test]
    fn zero_duration_is_instant() {
        let mut a = Animated::new(0.0f32);
        a.animate_to(
            1.0,
            Motion::curve(ASTRYX.durations.exit_dialog, ASTRYX.ease_exit),
        );
        assert_eq!(a.value(), 1.0);
        assert!(!a.is_animating());
    }

    #[test]
    fn reversing_a_curve_shortens_it_like_css() {
        let linear = Easing::Linear;
        let mut a = Animated::new(0.0f32);
        a.animate_to(1.0, Motion::curve(400.0, linear));
        a.tick(0.1); // a quarter of the way
        a.animate_to(0.0, Motion::curve(400.0, linear));
        assert!((a.remaining() - 0.1).abs() < 1e-6);
        // A retarget elsewhere plays its full duration from where it is.
        let mut b = Animated::new(0.0f32);
        b.animate_to(1.0, Motion::curve(400.0, linear));
        b.tick(0.1);
        b.animate_to(2.0, Motion::curve(400.0, linear));
        assert!((b.remaining() - 0.4).abs() < 1e-6);
        assert!((b.value() - 0.25).abs() < 1e-6);
    }

    #[test]
    fn a_spring_interrupt_keeps_position_and_velocity() {
        let spring = springs::SETTLE.spring();
        let mut a = Animated::new(0.0f32);
        a.animate_to(100.0, Motion::Spring(spring));
        run(&mut a, 0.1);
        let (p, v) = (a.value(), a.velocity());
        assert!(v > 0.0);
        a.animate_to(-50.0, Motion::Spring(spring));
        assert_eq!(a.value(), p);
        assert_eq!(a.velocity(), v);
        a.tick(1.0 / 120.0);
        // Still moving forward for a moment: momentum carried over.
        assert!(a.value() > p);
        run(&mut a, 2.0);
        assert_eq!(a.value(), -50.0);
    }

    #[test]
    fn a_curve_after_a_spring_keeps_position() {
        let mut a = Animated::new(0.0f32);
        a.animate_to(10.0, Motion::Spring(springs::SETTLE.spring()));
        run(&mut a, 0.05);
        let p = a.value();
        a.animate_to(0.0, Motion::curve(175.0, ASTRYX.ease_exit));
        assert_eq!(a.value(), p);
        assert_eq!(a.velocity(), 0.0);
    }

    #[test]
    fn multi_component_values() {
        let mut a = Animated::new([0.0f32, 0.0]);
        a.animate_to(
            [10.0, -10.0],
            Motion::Spring(springs::STATE_CHANGE.spring()),
        );
        run(&mut a, 3.0);
        assert_eq!(a.value(), [10.0, -10.0]);
        assert!(!a.is_animating());
    }

    #[test]
    fn spring_settles_near_its_settling_duration() {
        let spring = springs::STATE_CHANGE.spring();
        let mut a = Animated::new(0.0f32);
        a.animate_to(1.0, Motion::Spring(spring));
        let expect = spring.settling_duration(0.0) as f32;
        assert!((a.remaining() - expect).abs() < 1e-4);
    }
}
