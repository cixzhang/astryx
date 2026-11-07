//! Per-pixel math of frame transitions, shared by the renderers. The GPU
//! renderer computes the same values in a shader.

use astryx_core::{Easing, Edge, TransitionKind};

use crate::light::LATTICE;

/// How one pixel of a transition frame is made.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mix {
    /// How much of the new frame shows, 0 to 1.
    pub new: f32,
    /// Where to read the new frame, relative to this pixel, in pixels.
    pub new_offset: (f32, f32),
    /// Where to read the old frame: this pixel turned by this many radians
    /// about the center.
    pub old_turn: f32,
    /// White over the result, 0 to 1.
    pub white: f32,
    /// Black over the result, 0 to 1.
    pub black: f32,
}

/// The parameters of a transition at one moment, in target pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Moment {
    pub kind: TransitionKind,
    /// Linear time, 0 to 1.
    pub t: f32,
    /// Eased progress.
    pub p: f32,
    pub width: f32,
    pub height: f32,
    /// Pixels per point.
    pub scale: f32,
}

impl Moment {
    pub fn new(
        kind: TransitionKind,
        easing: Easing,
        t: f32,
        width: f32,
        height: f32,
        scale: f32,
    ) -> Moment {
        let t = t.clamp(0.0, 1.0);
        Moment {
            kind,
            t,
            p: easing.apply(t),
            width,
            height,
            scale,
        }
    }

    /// The encounter's white over everything and the height of each bar:
    /// two flashes in the first half, then bars closing.
    pub fn encounter(&self) -> (f32, f32) {
        let t = self.t;
        if t < 0.5 {
            // Each flash is on for the first half of its quarter.
            let on = (t / 0.5 * 4.0).floor() as i32 % 2 == 0;
            (if on { 1.0 } else { 0.0 }, 0.0)
        } else if t < 1.0 {
            (
                0.0,
                Easing::EaseIn.apply((t - 0.5) / 0.5) * self.height / 2.0,
            )
        } else {
            (0.0, 0.0)
        }
    }

    /// The mix at pixel center `(x, y)`. `lattice` comes from
    /// [`crate::light::lattice`].
    pub fn at(&self, x: f32, y: f32, lattice: &[u8]) -> Mix {
        let (w, h, p) = (self.width, self.height, self.p);
        let mut mix = Mix {
            new: 0.0,
            new_offset: (0.0, 0.0),
            old_turn: 0.0,
            white: 0.0,
            black: 0.0,
        };
        let step = |inside: bool| if inside { 1.0 } else { 0.0 };
        match self.kind {
            TransitionKind::Fade => mix.new = p,
            TransitionKind::Wipe(edge) => {
                mix.new = step(match edge {
                    Edge::Left => x < p * w,
                    Edge::Right => x > (1.0 - p) * w,
                    Edge::Top => y < p * h,
                    Edge::Bottom => y > (1.0 - p) * h,
                })
            }
            TransitionKind::Door => {
                let inset = 0.5 * (1.0 - p);
                mix.new = step(
                    x > inset * w
                        && x < (1.0 - inset) * w
                        && y > inset * h
                        && y < (1.0 - inset) * h,
                );
            }
            TransitionKind::Iris(c) => {
                let (cx, cy) = (c.x * self.scale, c.y * self.scale);
                let far = [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)]
                    .iter()
                    .map(|(px, py)| ((px - cx).powi(2) + (py - cy).powi(2)).sqrt())
                    .fold(0.0, f32::max);
                let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
                mix.new = step(d < p * far);
            }
            TransitionKind::Slide(o) => {
                let (ox, oy) = (o.x * self.scale * (1.0 - p), o.y * self.scale * (1.0 - p));
                let (sx, sy) = (x - ox, y - oy);
                mix.new_offset = (-ox, -oy);
                mix.new = if (0.0..w).contains(&sx) && (0.0..h).contains(&sy) {
                    p
                } else {
                    0.0
                };
            }
            TransitionKind::PixelDissolve(cell) => {
                let c = (cell * self.scale).max(1.0);
                let (i, j) = ((x / c).floor() as i64, (y / c).floor() as i64);
                let n = LATTICE as i64;
                let at = (j.rem_euclid(n) * n + i.rem_euclid(n)) as usize;
                // Thresholds sit between steps, so cells never tie with p.
                let threshold = (f32::from(lattice[at]) + 0.5) / 256.0;
                mix.new = step(threshold < p);
            }
            TransitionKind::Swirl => {
                let (cx, cy) = (w / 2.0, h / 2.0);
                let far = (cx * cx + cy * cy).sqrt();
                let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
                mix.old_turn = p * std::f32::consts::PI * (1.0 - d / far).max(0.0);
                mix.new = p;
            }
            TransitionKind::Encounter => {
                let (white, bars) = self.encounter();
                mix.white = white;
                mix.black = step(y < bars || y > h - bars);
                mix.new = step(self.t >= 1.0);
            }
        }
        mix
    }
}

/// Turns `(x, y)` by `angle` radians about `center`.
pub fn turn(x: f32, y: f32, center: (f32, f32), angle: f32) -> (f32, f32) {
    if angle == 0.0 {
        return (x, y);
    }
    let (s, c) = angle.sin_cos();
    let (dx, dy) = (x - center.0, y - center.1);
    (center.0 + dx * c - dy * s, center.1 + dx * s + dy * c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::light::lattice;
    use astryx_core::Point;

    fn moment(kind: TransitionKind, t: f32) -> Moment {
        Moment::new(kind, Easing::Linear, t, 100.0, 80.0, 1.0)
    }

    #[test]
    fn transitions_start_old_and_end_new() {
        let lat = lattice();
        for kind in [
            TransitionKind::Fade,
            TransitionKind::Wipe(Edge::Left),
            TransitionKind::Wipe(Edge::Bottom),
            TransitionKind::Door,
            TransitionKind::Iris(Point { x: 10.0, y: 70.0 }),
            TransitionKind::Slide(Point { x: 30.0, y: 0.0 }),
            TransitionKind::PixelDissolve(4.0),
            TransitionKind::Swirl,
            TransitionKind::Encounter,
        ] {
            for (x, y) in [(0.5, 0.5), (50.5, 40.5), (99.5, 79.5)] {
                let start = moment(kind, 0.0).at(x, y, &lat);
                let end = moment(kind, 1.0).at(x, y, &lat);
                assert_eq!(start.new, 0.0, "{kind:?} start");
                assert_eq!(end.new, 1.0, "{kind:?} end");
                assert_eq!((end.white, end.black), (0.0, 0.0), "{kind:?}");
            }
        }
    }

    #[test]
    fn easings_run_from_zero_to_one() {
        for e in [
            Easing::Linear,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::Steps(4),
            Easing::Overshoot,
        ] {
            assert!(e.apply(0.0).abs() < 1e-6, "{e:?}");
            assert!((e.apply(1.0) - 1.0).abs() < 1e-6, "{e:?}");
        }
        assert_eq!(Easing::Steps(4).apply(0.3), 0.25);
        assert!(Easing::EaseIn.apply(0.5) < 0.5 && Easing::EaseOut.apply(0.5) > 0.5);
        let peak = (0..100)
            .map(|i| Easing::Overshoot.apply(i as f32 / 100.0))
            .fold(0.0, f32::max);
        assert!(peak > 1.05, "{peak}");
    }

    #[test]
    fn door_opens_from_the_center_and_iris_from_its_point() {
        let lat = lattice();
        let door = moment(TransitionKind::Door, 0.5);
        assert_eq!(door.at(50.0, 40.0, &lat).new, 1.0);
        assert_eq!(door.at(10.0, 40.0, &lat).new, 0.0);
        let iris = moment(TransitionKind::Iris(Point { x: 0.0, y: 0.0 }), 0.3);
        assert_eq!(iris.at(5.0, 5.0, &lat).new, 1.0);
        assert_eq!(iris.at(95.0, 75.0, &lat).new, 0.0);
    }

    #[test]
    fn encounter_flashes_twice_then_closes_bars() {
        let lat = lattice();
        let white: Vec<f32> = [0.05, 0.2, 0.3, 0.45]
            .iter()
            .map(|&t| {
                moment(TransitionKind::Encounter, t)
                    .at(50.0, 40.0, &lat)
                    .white
            })
            .collect();
        assert_eq!(white, [1.0, 0.0, 1.0, 0.0]);
        let late = moment(TransitionKind::Encounter, 0.9);
        assert_eq!(late.at(50.0, 2.0, &lat).black, 1.0);
        assert_eq!(late.at(50.0, 40.0, &lat).black, 0.0);
    }

    #[test]
    fn dissolve_cells_switch_whole_and_spread_over_time() {
        let lat = lattice();
        let m = moment(TransitionKind::PixelDissolve(4.0), 0.5);
        assert_eq!(m.at(0.5, 0.5, &lat).new, m.at(3.5, 3.5, &lat).new);
        let shown = (0..25)
            .flat_map(|i| (0..20).map(move |j| (i, j)))
            .filter(|&(i, j)| m.at(i as f32 * 4.0 + 1.0, j as f32 * 4.0 + 1.0, &lat).new > 0.0)
            .count();
        assert!((150..350).contains(&shown), "{shown}");
    }
}
