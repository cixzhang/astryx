//! The damped spring Astryx's animator uses (core motion/spring.ts), solved
//! in closed form: m·x″ + c·x′ + k·x = 0 in its under-, critically and
//! over-damped regimes. Time in seconds, velocity in value units per second.

use std::f64::consts::TAU;

/// A spring in physical terms. Every other description converts to this.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    pub mass: f64,
    pub stiffness: f64,
    pub damping: f64,
}

/// A spring as a motion table writes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpringSpec {
    /// WWDC18: the undamped period and the damping ratio.
    Response { response: f32, damping_ratio: f32 },
    /// SwiftUI `Spring(duration:bounce:)`.
    DurationBounce { duration: f32, bounce: f32 },
    /// Core Animation's mass, stiffness and damping.
    Physics {
        mass: f32,
        stiffness: f32,
        damping: f32,
    },
}

impl SpringSpec {
    /// The spring at a theme's tempo: the undamped period grows by `tempo`
    /// and the damping ratio stays, so the motion keeps its shape and takes
    /// `tempo` times as long (core resolveMotion.ts, `spec:AST-108` FR3).
    pub fn at_tempo(self, tempo: f32) -> SpringSpec {
        match self {
            SpringSpec::Response {
                response,
                damping_ratio,
            } => SpringSpec::Response {
                response: response * tempo,
                damping_ratio,
            },
            SpringSpec::DurationBounce { duration, bounce } => SpringSpec::DurationBounce {
                duration: duration * tempo,
                bounce,
            },
            SpringSpec::Physics {
                mass,
                stiffness,
                damping,
            } => SpringSpec::Physics {
                mass,
                stiffness: stiffness / (tempo * tempo),
                damping: damping / tempo,
            },
        }
    }

    pub fn spring(self) -> Spring {
        match self {
            SpringSpec::Response {
                response,
                damping_ratio,
            } => Spring::from_response(response as f64, damping_ratio as f64),
            SpringSpec::DurationBounce { duration, bounce } => {
                Spring::from_duration_bounce(duration as f64, bounce as f64)
            }
            SpringSpec::Physics {
                mass,
                stiffness,
                damping,
            } => Spring::from_physics(mass as f64, stiffness as f64, damping as f64),
        }
    }
}

impl Spring {
    /// SwiftUI's `Spring(duration:bounce:)`: the WWDC23 equations, bounce
    /// clamped to (−1, 1].
    pub fn from_duration_bounce(duration: f64, bounce: f64) -> Spring {
        let duration = duration.max(1e-6);
        let b = bounce.clamp(-0.999999, 1.0);
        let stiffness = (TAU / duration).powi(2);
        let damping = if b >= 0.0 {
            (1.0 - b) * 2.0 * TAU / duration
        } else {
            2.0 * TAU / (duration * (1.0 + b))
        };
        Spring {
            mass: 1.0,
            stiffness,
            damping,
        }
    }

    /// Response (the undamped period) and damping ratio.
    pub fn from_response(response: f64, damping_ratio: f64) -> Spring {
        let omega = TAU / response.max(1e-6);
        Spring {
            mass: 1.0,
            stiffness: omega * omega,
            damping: 2.0 * damping_ratio.max(0.0) * omega,
        }
    }

    /// Mass, stiffness, damping; damping above critical is clamped to it,
    /// as UIKit does (core `springFromPhysics`).
    pub fn from_physics(mass: f64, stiffness: f64, damping: f64) -> Spring {
        let critical = 2.0 * (stiffness * mass).sqrt();
        Spring {
            mass,
            stiffness,
            damping: damping.max(0.0).min(critical),
        }
    }

    pub fn natural_frequency(&self) -> f64 {
        (self.stiffness / self.mass).sqrt()
    }

    pub fn damping_ratio(&self) -> f64 {
        self.damping / (2.0 * (self.stiffness * self.mass).sqrt())
    }

    /// The closed-form motion from `from` to `to`, starting at `velocity`.
    pub fn solve(&self, from: f64, to: f64, velocity: f64) -> SpringCurve {
        SpringCurve::new(*self, from, to, velocity)
    }

    /// Seconds until a unit move stays within 0.001 of its target (UIKit's
    /// and SwiftUI's settling duration).
    pub fn settling_duration(&self, initial_velocity: f64) -> f64 {
        self.solve(0.0, 1.0, initial_velocity)
            .settle_time(SETTLING_EPSILON)
    }
}

/// The epsilon SwiftUI documents for `Spring.settlingDuration`.
pub const SETTLING_EPSILON: f64 = 0.001;

const CRITICAL_TOLERANCE: f64 = 1e-6;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Regime {
    Under {
        decay: f64,
        omega_d: f64,
        a: f64,
        b: f64,
        bv: f64,
    },
    Critical {
        b: f64,
    },
    Over {
        r1: f64,
        r2: f64,
        c1: f64,
        c2: f64,
    },
}

/// One spring motion, evaluated at any time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringCurve {
    pub spring: Spring,
    pub from: f64,
    pub to: f64,
    pub initial_velocity: f64,
    omega0: f64,
    zeta: f64,
    regime: Regime,
}

impl SpringCurve {
    fn new(spring: Spring, from: f64, to: f64, v0: f64) -> SpringCurve {
        let omega0 = spring.natural_frequency();
        let zeta = spring.damping_ratio();
        let x0 = from - to;
        let regime = if zeta < 1.0 - CRITICAL_TOLERANCE {
            let decay = zeta * omega0;
            let omega_d = omega0 * (1.0 - zeta * zeta).sqrt();
            Regime::Under {
                decay,
                omega_d,
                a: x0,
                b: (v0 + decay * x0) / omega_d,
                bv: (decay * v0 + omega0 * omega0 * x0) / omega_d,
            }
        } else if zeta <= 1.0 + CRITICAL_TOLERANCE {
            Regime::Critical {
                b: v0 + omega0 * x0,
            }
        } else {
            let root = omega0 * (zeta * zeta - 1.0).sqrt();
            let r1 = -zeta * omega0 + root;
            let r2 = -zeta * omega0 - root;
            let c1 = (v0 - r2 * x0) / (r1 - r2);
            Regime::Over {
                r1,
                r2,
                c1,
                c2: x0 - c1,
            }
        };
        SpringCurve {
            spring,
            from,
            to,
            initial_velocity: v0,
            omega0,
            zeta,
            regime,
        }
    }

    fn x0(&self) -> f64 {
        self.from - self.to
    }

    pub fn value(&self, t: f64) -> f64 {
        let d = match self.regime {
            Regime::Under {
                decay,
                omega_d,
                a,
                b,
                ..
            } => (-decay * t).exp() * (a * (omega_d * t).cos() + b * (omega_d * t).sin()),
            Regime::Critical { b } => (self.x0() + b * t) * (-self.omega0 * t).exp(),
            Regime::Over { r1, r2, c1, c2 } => c1 * (r1 * t).exp() + c2 * (r2 * t).exp(),
        };
        self.to + d
    }

    pub fn velocity(&self, t: f64) -> f64 {
        let v0 = self.initial_velocity;
        match self.regime {
            Regime::Under {
                decay, omega_d, bv, ..
            } => (-decay * t).exp() * (v0 * (omega_d * t).cos() - bv * (omega_d * t).sin()),
            Regime::Critical { b } => (v0 - self.omega0 * b * t) * (-self.omega0 * t).exp(),
            Regime::Over { r1, r2, c1, c2 } => r1 * c1 * (r1 * t).exp() + r2 * c2 * (r2 * t).exp(),
        }
    }

    fn envelope(&self, t: f64) -> f64 {
        match self.regime {
            Regime::Under { decay, a, b, .. } => (a.abs() + b.abs()) * (-decay * t).exp(),
            Regime::Critical { b } => (self.x0().abs() + b.abs() * t) * (-self.omega0 * t).exp(),
            Regime::Over { r1, r2, c1, c2 } => {
                c1.abs() * (r1 * t).exp() + c2.abs() * (r2 * t).exp()
            }
        }
    }

    /// Seconds until the displacement from the target stays within
    /// `threshold` (core `settleTime`).
    pub fn settle_time(&self, threshold: f64) -> f64 {
        if threshold <= 0.0 {
            return f64::INFINITY;
        }
        if self.zeta == 0.0 {
            return if self.x0() == 0.0 && self.initial_velocity == 0.0 {
                0.0
            } else {
                f64::INFINITY
            };
        }
        let start = match self.regime {
            Regime::Critical { b } if b != 0.0 => {
                (1.0 / self.omega0 - self.x0().abs() / b.abs()).max(0.0)
            }
            _ => 0.0,
        };
        if self.envelope(start) <= threshold {
            return 0.0;
        }
        if let Regime::Under { decay, .. } = self.regime {
            return (self.envelope(0.0) / threshold).ln() / decay;
        }
        let (mut lo, mut hi) = (start, start.max(1.0 / self.omega0));
        while self.envelope(hi) > threshold {
            lo = hi;
            hi *= 2.0;
            if hi > 1e6 {
                return f64::INFINITY;
            }
        }
        for _ in 0..100 {
            if hi - lo <= 1e-12 {
                break;
            }
            let mid = (lo + hi) / 2.0;
            if self.envelope(mid) > threshold {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        hi
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::springs;

    // Apple's CASpringAnimation documentation example, also in core's tests:
    // stiffness 100, damping 5 settles in about 2.85 s; damping 15 just over 1 s.
    #[test]
    fn matches_apples_documented_settling_durations() {
        let d5 = Spring::from_physics(1.0, 100.0, 5.0).settling_duration(0.0);
        assert!((d5 - 2.85).abs() < 0.05, "{d5}");
        let d15 = Spring::from_physics(1.0, 100.0, 15.0).settling_duration(0.0);
        assert!(d15 > 1.0 && d15 < 1.1, "{d15}");
    }

    #[test]
    fn conversions_agree_with_the_wwdc_equations() {
        let s = Spring::from_duration_bounce(0.5, 0.0);
        assert!((s.damping_ratio() - 1.0).abs() < 1e-9);
        assert!((TAU / s.natural_frequency() - 0.5).abs() < 1e-9);
        let r = Spring::from_response(0.55, 1.0);
        assert!((r.damping_ratio() - 1.0).abs() < 1e-9);
        let bouncy = Spring::from_duration_bounce(0.5, 0.3);
        assert!((bouncy.damping_ratio() - 0.7).abs() < 1e-9);
    }

    #[test]
    fn every_regime_starts_at_from_with_v0_and_lands_on_target() {
        for spring in [
            Spring::from_response(0.4, 0.5),
            Spring::from_response(0.4, 1.0),
            Spring {
                mass: 1.0,
                stiffness: 100.0,
                damping: 40.0,
            },
        ] {
            let c = spring.solve(10.0, 110.0, 250.0);
            assert!((c.value(0.0) - 10.0).abs() < 1e-9);
            assert!((c.velocity(0.0) - 250.0).abs() < 1e-6);
            let end = c.settle_time(0.1);
            assert!(end.is_finite());
            assert!((c.value(end) - 110.0).abs() <= 0.1 + 1e-9);
            // Velocity is the derivative of value.
            let (t, h) = (0.05, 1e-6);
            let numeric = (c.value(t + h) - c.value(t - h)) / (2.0 * h);
            assert!((numeric - c.velocity(t)).abs() < 1e-3);
        }
    }

    #[test]
    fn tempo_stretches_time_and_keeps_shape() {
        let base = springs::STATE_CHANGE.spring().settling_duration(0.0);
        let slow = springs::STATE_CHANGE
            .at_tempo(1.5)
            .spring()
            .settling_duration(0.0);
        assert!((slow / base - 1.5).abs() < 1e-6);
        let phys = SpringSpec::Physics {
            mass: 1.0,
            stiffness: 100.0,
            damping: 10.0,
        };
        let s1 = phys.spring();
        let s2 = phys.at_tempo(1.25).spring();
        assert!((s1.damping_ratio() - s2.damping_ratio()).abs() < 1e-9);
    }

    #[test]
    fn overdamped_physics_is_clamped_to_critical() {
        let view = springs::VIEW_CHANGE.spring();
        assert!((view.damping_ratio() - 1.0).abs() < 1e-9);
    }
}
