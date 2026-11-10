//! Drags: the runtime hands a pointer drag to the innermost handler that
//! wants its direction, so nested drags (a list inside a sheet) share one
//! pointer the way Astryx's gesture arena shares it.
//!
//! Members of a drag, innermost first, are the nodes under the press with a
//! [`DragSpec`], and scroll containers when a finger drags. Once the drag
//! travels [`DRAG_HYSTERESIS`], the first member that wants its direction
//! takes it: a scroll container wants a direction it can still scroll in;
//! a `DragSpec` decides with its own `wants`. The winner gets every event
//! of the drag; the press under the pointer is cancelled.

use crate::contract::{Point, PointerKind};
use std::rc::Rc;

/// Travel before a drag picks its handler (UIKit's pan hysteresis).
pub const DRAG_HYSTERESIS: f32 = 10.0;

/// Where a drag is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragPhase {
    Start,
    Move,
    End,
    Cancel,
}

/// One event of a drag a handler took.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragEvent {
    pub phase: DragPhase,
    pub kind: PointerKind,
    /// Where the pointer is, and when, in the shell's milliseconds.
    pub at: Point,
    pub time_ms: f64,
    /// Where and when the press began.
    pub start: Point,
    pub start_ms: f64,
}

/// A handler's message for a drag event.
pub type OnDrag<Msg> = Rc<dyn Fn(&DragEvent) -> Msg>;
/// A handler's change to component state for a drag event.
pub type DragEffect = Rc<dyn Fn(&DragEvent)>;

/// Makes a node a drag handler.
pub struct DragSpec<Msg> {
    /// Whether the handler takes a drag that has travelled this far from
    /// its start when it passes the hysteresis.
    pub wants: Rc<dyn Fn(Point) -> bool>,
    /// The message each event sends.
    pub on_drag: Option<OnDrag<Msg>>,
    /// Runs on each event before `on_drag`: a change to component state.
    pub effect: Option<DragEffect>,
}

/// A finger's coast on a scroll container after a throw: UIScrollView's
/// normal deceleration rate, per millisecond.
pub const SCROLL_DECELERATION: f32 = 0.998;
