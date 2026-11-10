//! Gesture models, ported from Astryx core's gestures: the release
//! velocity filter, pan recognition, the gesture arena that decides which of
//! nested drags moves, momentum, rubber banding, snap points, and reorder.
//!
//! Positions are in points and times in milliseconds. The models hold no UI;
//! [`runtime`] wires them to the core runtime's pointer events.

pub mod runtime;

use astryx_core::Point;

/// One pointer sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub x: f32,
    pub y: f32,
    pub time_ms: f64,
}

/// UIKit's pan velocity weights and Astryx's release rule, as Astryx core's
/// `VelocityTracker` measures them.
pub const PAN_CURRENT_WEIGHT: f32 = 0.2;
pub const SCROLL_RELEASE_WEIGHT: f32 = 0.25;
/// A pointer that rests this long before lifting releases at 0.
pub const STILLNESS_MS: f64 = 40.0;
/// Distance a pan travels before it begins.
pub const PAN_HYSTERESIS: f32 = 10.0;
/// Release speed, in points per second, above which a drag carries to the
/// next snap point (UIKit's sheet flick).
pub const FLICK_THRESHOLD: f32 = 1008.0;
/// UIScrollView's normal deceleration rate, per millisecond.
pub const DECELERATION_RATE: f32 = 0.998;
/// UIScrollView's rubber band coefficient.
pub const RUBBER_BAND: f32 = 0.55;

/// The one release velocity for a stream of samples, as Astryx's
/// `VelocityTracker` computes it: a blend of the newest two instantaneous
/// velocities, and 0 when the pointer rested before lifting.
#[derive(Clone, Debug, Default)]
pub struct VelocityTracker {
    moved: Vec<Sample>,
    moves: u32,
    pan: Point,
    previous_pan: Point,
}

impl VelocityTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn add(&mut self, s: Sample) {
        if let Some(last) = self.moved.last().copied() {
            if s.time_ms < last.time_ms {
                return;
            }
            if s.x == last.x && s.y == last.y {
                return;
            }
            if s.time_ms == last.time_ms {
                *self.moved.last_mut().expect("last") = s;
            } else {
                self.moved.push(s);
                if self.moved.len() > 3 {
                    self.moved.remove(0);
                }
                self.moves += 1;
                self.previous_pan = self.pan;
            }
        } else {
            self.moved.push(s);
        }
        self.pan = pan_velocity(&self.moved);
    }

    fn releases(&self, release_ms: f64) -> bool {
        self.moves >= 2
            && self
                .moved
                .last()
                .is_some_and(|l| release_ms - l.time_ms <= STILLNESS_MS)
    }

    /// The pan velocity at a release at `release_ms`, in points per second.
    pub fn velocity(&self, release_ms: f64) -> Point {
        if self.releases(release_ms) {
            self.pan
        } else {
            Point { x: 0.0, y: 0.0 }
        }
    }

    /// A scroll view's release velocity.
    pub fn scroll_velocity(&self, release_ms: f64) -> Point {
        if !self.releases(release_ms) {
            return Point { x: 0.0, y: 0.0 };
        }
        blend(SCROLL_RELEASE_WEIGHT, self.pan, self.previous_pan)
    }
}

fn blend(w: f32, a: Point, b: Point) -> Point {
    Point {
        x: w * a.x + (1.0 - w) * b.x,
        y: w * a.y + (1.0 - w) * b.y,
    }
}

fn pair(a: Sample, b: Sample) -> Point {
    let dt = (b.time_ms - a.time_ms) as f32;
    if dt <= 0.0 {
        return Point { x: 0.0, y: 0.0 };
    }
    Point {
        x: (b.x - a.x) / dt * 1000.0,
        y: (b.y - a.y) / dt * 1000.0,
    }
}

fn pan_velocity(m: &[Sample]) -> Point {
    match m.len() {
        0 | 1 => Point { x: 0.0, y: 0.0 },
        2 => pair(m[0], m[1]),
        n => blend(
            PAN_CURRENT_WEIGHT,
            pair(m[n - 2], m[n - 1]),
            pair(m[n - 3], m[n - 2]),
        ),
    }
}

/// Which way a pan may move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Both,
}

/// Where a pan is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanPhase {
    /// Down, not yet past the hysteresis.
    Possible,
    Began,
    Changed,
    Ended,
    Cancelled,
    /// Moved past the hysteresis along the wrong axis.
    Failed,
}

/// What a pan reports: its phase, travel since the press, and velocity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanEvent {
    pub phase: PanPhase,
    pub translation: Point,
    pub velocity: Point,
}

/// A pan recognizer: begins after [`PAN_HYSTERESIS`] of travel, fails when
/// that travel is mostly across its axis.
#[derive(Clone, Debug)]
pub struct Pan {
    pub axis: Axis,
    pub hysteresis: f32,
    start: Option<Sample>,
    phase: PanPhase,
    tracker: VelocityTracker,
}

impl Pan {
    pub fn new(axis: Axis) -> Self {
        Pan {
            axis,
            hysteresis: PAN_HYSTERESIS,
            start: None,
            phase: PanPhase::Possible,
            tracker: VelocityTracker::new(),
        }
    }

    pub fn phase(&self) -> PanPhase {
        self.phase
    }

    fn translation(&self, s: Sample) -> Point {
        let start = self.start.unwrap_or(s);
        let (dx, dy) = (s.x - start.x, s.y - start.y);
        match self.axis {
            Axis::X => Point { x: dx, y: 0.0 },
            Axis::Y => Point { x: 0.0, y: dy },
            Axis::Both => Point { x: dx, y: dy },
        }
    }

    pub fn down(&mut self, s: Sample) {
        self.start = Some(s);
        self.phase = PanPhase::Possible;
        self.tracker.reset();
        self.tracker.add(s);
    }

    /// Feeds a move; returns an event once the pan has begun.
    pub fn moved(&mut self, s: Sample) -> Option<PanEvent> {
        let start = self.start?;
        self.tracker.add(s);
        match self.phase {
            PanPhase::Possible => {
                let (dx, dy) = ((s.x - start.x).abs(), (s.y - start.y).abs());
                if dx.max(dy) < self.hysteresis {
                    return None;
                }
                let along = match self.axis {
                    Axis::X => dx >= dy,
                    Axis::Y => dy >= dx,
                    Axis::Both => true,
                };
                self.phase = if along {
                    PanPhase::Began
                } else {
                    PanPhase::Failed
                };
            }
            PanPhase::Began => self.phase = PanPhase::Changed,
            PanPhase::Changed => {}
            _ => return None,
        }
        (self.phase != PanPhase::Failed).then(|| PanEvent {
            phase: self.phase,
            translation: self.translation(s),
            velocity: self.tracker.velocity(s.time_ms),
        })
    }

    /// Feeds the lift; returns the end event when the pan had begun.
    pub fn up(&mut self, s: Sample) -> Option<PanEvent> {
        let began = matches!(self.phase, PanPhase::Began | PanPhase::Changed);
        self.phase = if began {
            PanPhase::Ended
        } else {
            PanPhase::Failed
        };
        began.then(|| PanEvent {
            phase: PanPhase::Ended,
            translation: self.translation(s),
            velocity: self.tracker.velocity(s.time_ms),
        })
    }

    /// Forces the pan to begin, as when the arena hands it a drag another
    /// member declined.
    pub fn claim(&mut self) {
        if self.phase == PanPhase::Possible || self.phase == PanPhase::Failed {
            self.phase = PanPhase::Began;
        }
    }

    pub fn cancel(&mut self) -> Option<PanEvent> {
        let began = matches!(self.phase, PanPhase::Began | PanPhase::Changed);
        self.phase = PanPhase::Cancelled;
        began.then_some(PanEvent {
            phase: PanPhase::Cancelled,
            translation: Point { x: 0.0, y: 0.0 },
            velocity: Point { x: 0.0, y: 0.0 },
        })
    }
}

/// One member of a gesture arena: a drag handler at some depth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Member {
    pub axis: Axis,
    /// For a scroll container: how far it can still scroll back and
    /// forward along its axis. A pan has `None`: it takes any drag along
    /// its axis.
    pub scroll_room: Option<(f32, f32)>,
}

/// Decides which of nested drag handlers takes a drag, as Astryx's
/// `GestureArena` does: the innermost member that can use the drag's
/// direction wins, so a list inside a sheet scrolls first and the sheet
/// drags only when the list is at its edge.
///
/// `members` run innermost first; `delta` is the travel when the drag
/// passed its hysteresis. Returns the winner's index.
pub fn arbitrate(members: &[Member], delta: Point) -> Option<usize> {
    let vertical = delta.y.abs() >= delta.x.abs();
    members.iter().position(|m| {
        let along = match m.axis {
            Axis::X => !vertical,
            Axis::Y => vertical,
            Axis::Both => true,
        };
        if !along {
            return false;
        }
        match m.scroll_room {
            None => true,
            Some((back, forward)) => {
                // Dragging content down (positive delta) scrolls back.
                let d = if vertical { delta.y } else { delta.x };
                if d > 0.0 {
                    back > 0.5
                } else {
                    forward > 0.5
                }
            }
        }
    })
}

/// A throw's coast after release, as UIScrollView decelerates: velocity
/// decays by [`DECELERATION_RATE`] per millisecond.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Momentum {
    pub position: f32,
    pub velocity: f32,
    pub rate: f32,
}

impl Momentum {
    pub fn new(position: f32, velocity: f32) -> Self {
        Momentum {
            position,
            velocity,
            rate: DECELERATION_RATE,
        }
    }

    /// Where the coast comes to rest.
    pub fn rest(&self) -> f32 {
        // Σ v·rᵗ dt over milliseconds: v/1000 · r / (1 − r).
        self.position + self.velocity / 1000.0 * self.rate / (1.0 - self.rate)
    }

    /// Advances `dt_ms`; returns false once it stops (under 1 point/s).
    pub fn step(&mut self, dt_ms: f32) -> bool {
        let decay = self.rate.powf(dt_ms);
        // The exact integral of v·rᵗ over the step.
        let ln = self.rate.ln();
        self.position += self.velocity / 1000.0 * (decay - 1.0) / ln;
        self.velocity *= decay;
        self.velocity.abs() >= 1.0
    }
}

/// How far content shows past an edge when dragged `overflow` points past
/// it in a viewport of `dimension`, as UIScrollView resists.
pub fn rubber_band(overflow: f32, dimension: f32) -> f32 {
    if dimension <= 0.0 {
        return 0.0;
    }
    let x = overflow.abs();
    let banded = (1.0 - 1.0 / (x * RUBBER_BAND / dimension + 1.0)) * dimension;
    banded.copysign(overflow)
}

/// The snap point a drag released at `position` with `velocity` settles on,
/// as Astryx's `resolveSnapPoint` decides for `createDragToSnapPoints`: the
/// nearest point; a flick goes at least to the next point in its direction;
/// a release moving back toward where the drag started returns to the point
/// nearest that start.
pub fn resolve_snap(points: &[f32], position: f32, velocity: f32, start: Option<f32>) -> usize {
    assert!(!points.is_empty(), "snap points");
    let nearest = |p: f32| {
        (0..points.len())
            .min_by(|&a, &b| (points[a] - p).abs().total_cmp(&(points[b] - p).abs()))
            .expect("non-empty")
    };
    let mut index = nearest(position);
    if velocity.abs() >= FLICK_THRESHOLD {
        let next = if velocity > 0.0 {
            points.iter().position(|&d| d > position)
        } else {
            points.iter().rposition(|&d| d < position)
        };
        if let Some(n) = next {
            if (velocity > 0.0 && n > index) || (velocity < 0.0 && n < index) {
                index = n;
            }
        }
    }
    if let Some(s) = start {
        if velocity != 0.0 && velocity.signum() == (s - position).signum() {
            index = nearest(s);
        }
    }
    index
}

/// A list's reorder, by pointer and by keyboard, with what assistive
/// technology hears.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reorder {
    pub order: Vec<usize>,
    /// The item picked up (by position in `order`), while reordering.
    pub grabbed: Option<usize>,
}

impl Reorder {
    pub fn new(len: usize) -> Self {
        Reorder {
            order: (0..len).collect(),
            grabbed: None,
        }
    }

    /// Space on the grip: picks the item up or drops it.
    pub fn toggle_grab(&mut self, at: usize) -> String {
        match self.grabbed {
            None => {
                self.grabbed = Some(at);
                format!("Grabbed item {} of {}.", at + 1, self.order.len())
            }
            Some(pos) => {
                self.grabbed = None;
                format!("Dropped at position {} of {}.", pos + 1, self.order.len())
            }
        }
    }

    /// An arrow while grabbed moves the item one place.
    pub fn step(&mut self, up: bool) -> Option<String> {
        let pos = self.grabbed?;
        let to = if up {
            pos.checked_sub(1)?
        } else if pos + 1 < self.order.len() {
            pos + 1
        } else {
            return None;
        };
        self.order.swap(pos, to);
        self.grabbed = Some(to);
        Some(format!(
            "Moved to position {} of {}.",
            to + 1,
            self.order.len()
        ))
    }

    /// Escape puts the item back where it was picked up from.
    pub fn cancel(&mut self, original: &[usize]) -> String {
        self.order = original.to_vec();
        self.grabbed = None;
        "Reorder cancelled.".to_string()
    }

    /// A pointer drag of the item at `from` to the slot under `y`, for rows
    /// of `row_height` starting at `top`.
    pub fn drag_to(&mut self, from: usize, y: f32, top: f32, row_height: f32) -> usize {
        let len = self.order.len();
        let to = (((y - top) / row_height).floor().max(0.0) as usize).min(len.saturating_sub(1));
        let item = self.order.remove(from);
        self.order.insert(to, item);
        to
    }
}

#[cfg(test)]
mod tests;
