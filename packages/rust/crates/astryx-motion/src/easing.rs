//! CSS easing functions as Astryx defines and uses them.

/// A CSS `cubic-bezier(x1, y1, x2, y2)` timing function: from (0, 0) to
/// (1, 1), x is time progress, y is eased progress.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl CubicBezier {
    pub const fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        CubicBezier { x1, y1, x2, y2 }
    }

    /// Eased progress at time progress `x`; 0 at or before 0, 1 at or after 1.
    ///
    /// Solved for x by Newton's method with a bisection fallback, the same
    /// steps and tolerances as Astryx core's `cubicBezier` (motion/bezier.ts)
    /// and browser engines.
    pub fn ease(&self, x: f32) -> f32 {
        self.ease_f64(x as f64) as f32
    }

    pub fn ease_f64(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        if x >= 1.0 {
            return 1.0;
        }
        let c = Coefficients::new(self);
        c.sample_y(c.solve_t(x))
    }

    /// The slope dy/dx at time progress `x`, for an eased value's velocity.
    pub fn slope(&self, x: f32) -> f32 {
        let x = (x as f64).clamp(0.0, 1.0);
        let c = Coefficients::new(self);
        let t = c.solve_t(x);
        let dx = c.slope_x(t);
        if dx.abs() < 1e-9 {
            return 0.0;
        }
        (c.slope_y(t) / dx) as f32
    }
}

struct Coefficients {
    ax: f64,
    bx: f64,
    cx: f64,
    ay: f64,
    by: f64,
    cy: f64,
}

impl Coefficients {
    fn new(p: &CubicBezier) -> Self {
        let (x1, y1, x2, y2) = (p.x1 as f64, p.y1 as f64, p.x2 as f64, p.y2 as f64);
        let cx = 3.0 * x1;
        let bx = 3.0 * (x2 - x1) - cx;
        let ax = 1.0 - cx - bx;
        let cy = 3.0 * y1;
        let by = 3.0 * (y2 - y1) - cy;
        let ay = 1.0 - cy - by;
        Coefficients {
            ax,
            bx,
            cx,
            ay,
            by,
            cy,
        }
    }

    fn sample_x(&self, t: f64) -> f64 {
        ((self.ax * t + self.bx) * t + self.cx) * t
    }

    fn sample_y(&self, t: f64) -> f64 {
        ((self.ay * t + self.by) * t + self.cy) * t
    }

    fn slope_x(&self, t: f64) -> f64 {
        (3.0 * self.ax * t + 2.0 * self.bx) * t + self.cx
    }

    fn slope_y(&self, t: f64) -> f64 {
        (3.0 * self.ay * t + 2.0 * self.by) * t + self.cy
    }

    fn solve_t(&self, x: f64) -> f64 {
        let mut t = x;
        for _ in 0..8 {
            let error = self.sample_x(t) - x;
            if error.abs() < 1e-7 {
                return t;
            }
            let slope = self.slope_x(t);
            if slope.abs() < 1e-6 {
                break;
            }
            t -= error / slope;
        }
        let (mut lo, mut hi) = (0.0, 1.0);
        t = x;
        for _ in 0..60 {
            let value = self.sample_x(t);
            if (value - x).abs() < 1e-7 {
                break;
            }
            if value < x {
                lo = t;
            } else {
                hi = t;
            }
            t = (lo + hi) / 2.0;
        }
        t
    }
}

/// Where a `steps()` function jumps (CSS Easing Functions Level 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepPosition {
    JumpStart,
    JumpEnd,
    JumpNone,
    JumpBoth,
}

/// An easing function.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Easing {
    Linear,
    CubicBezier(CubicBezier),
    /// CSS `steps(count, position)`. No Astryx motion token uses it today.
    Steps {
        count: u32,
        position: StepPosition,
    },
}

impl Easing {
    /// Eased progress at time progress `x`, clamped to 0..=1 input.
    pub fn ease(&self, x: f32) -> f32 {
        match self {
            Easing::Linear => x.clamp(0.0, 1.0),
            Easing::CubicBezier(c) => c.ease(x),
            Easing::Steps { count, position } => steps(*count, *position, x),
        }
    }

    /// The slope of the eased progress at `x` (0 between the jumps of `steps()`).
    pub fn slope(&self, x: f32) -> f32 {
        match self {
            Easing::Linear => 1.0,
            Easing::CubicBezier(c) => c.slope(x),
            Easing::Steps { .. } => 0.0,
        }
    }
}

impl From<CubicBezier> for Easing {
    fn from(c: CubicBezier) -> Self {
        Easing::CubicBezier(c)
    }
}

fn steps(count: u32, position: StepPosition, x: f32) -> f32 {
    let count = count.max(1) as f32;
    let x = x.clamp(0.0, 1.0);
    let mut step = (x * count).floor();
    if matches!(position, StepPosition::JumpStart | StepPosition::JumpBoth) {
        step += 1.0;
    }
    let jumps = match position {
        StepPosition::JumpBoth => count + 1.0,
        StepPosition::JumpNone => (count - 1.0).max(1.0),
        _ => count,
    };
    step.min(jumps) / jumps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens;

    fn parametric(a: f64, b: f64, t: f64) -> f64 {
        3.0 * a * t * (1.0 - t).powi(2) + 3.0 * b * t * t * (1.0 - t) + t.powi(3)
    }

    // The same check as core's bezier.test.ts: the solver against the curve's
    // own parametric form, for every curve Astryx's tokens use.
    #[test]
    fn matches_the_parametric_curve() {
        let curves = [
            tokens::ASTRYX.ease_standard,
            tokens::ASTRYX.ease_exit,
            tokens::CRISP_EASE,
            tokens::EXPRESSIVE_EASE,
            tokens::REDUCED_MOTION_CROSS_FADE_CURVE,
        ];
        for c in curves {
            let (x1, y1, x2, y2) = (c.x1 as f64, c.y1 as f64, c.x2 as f64, c.y2 as f64);
            for i in 0..=256 {
                let t = i as f64 / 256.0;
                let got = c.ease_f64(parametric(x1, x2, t));
                let want = parametric(y1, y2, t);
                assert!((got - want).abs() < 5e-6, "{c:?} at t={t}: {got} vs {want}");
            }
        }
    }

    #[test]
    fn runs_from_zero_to_one_and_clamps() {
        let c = tokens::ASTRYX.ease_standard;
        assert_eq!(c.ease(0.0), 0.0);
        assert_eq!(c.ease(1.0), 1.0);
        assert_eq!(c.ease(-0.5), 0.0);
        assert_eq!(c.ease(1.5), 1.0);
    }

    #[test]
    fn ease_in_out_is_symmetric() {
        let c = tokens::REDUCED_MOTION_CROSS_FADE_CURVE;
        assert!((c.ease(0.5) - 0.5).abs() < 1e-6);
        for i in 0..=10 {
            let x = i as f32 * 0.05;
            assert!((c.ease(x) + c.ease(1.0 - x) - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn standard_curve_values() {
        // cubic-bezier(0.24, 1, 0.4, 1): a fast start that decelerates.
        let c = tokens::ASTRYX.ease_standard;
        assert!(c.ease(0.25) > 0.7);
        // cubic-bezier(0.3, 0, 0.6, 0.6): moves off slowly, leaves fast.
        let e = tokens::ASTRYX.ease_exit;
        assert!(e.ease(0.25) < 0.25);
        assert!(e.slope(1.0) > e.slope(0.0));
    }

    #[test]
    fn expressive_overshoots_and_crisp_does_not() {
        let max = |c: CubicBezier| {
            (0..=100)
                .map(|i| c.ease(i as f32 / 100.0))
                .fold(0.0, f32::max)
        };
        assert!(max(tokens::EXPRESSIVE_EASE) > 1.0);
        assert!(max(tokens::CRISP_EASE) <= 1.0);
    }

    #[test]
    fn steps_follow_css() {
        let s = |n, p, x| {
            Easing::Steps {
                count: n,
                position: p,
            }
            .ease(x)
        };
        assert_eq!(s(4, StepPosition::JumpEnd, 0.0), 0.0);
        assert_eq!(s(4, StepPosition::JumpEnd, 0.3), 0.25);
        assert_eq!(s(4, StepPosition::JumpEnd, 1.0), 1.0);
        assert_eq!(s(4, StepPosition::JumpStart, 0.0), 0.25);
        assert_eq!(s(4, StepPosition::JumpBoth, 0.0), 0.2);
        assert_eq!(s(3, StepPosition::JumpNone, 0.5), 0.5);
        assert_eq!(s(3, StepPosition::JumpNone, 1.0), 1.0);
    }
}
