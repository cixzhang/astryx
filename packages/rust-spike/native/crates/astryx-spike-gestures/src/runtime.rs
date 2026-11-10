//! Gestures on the core runtime: a [`Drag`] element whose pan the runtime's
//! arena hands drags to, and a [`SnapDrag`] model for sheets.
//!
//! The core runtime routes pointer drags to the innermost handler that
//! wants the drag's direction (touch scroll containers included), so a list
//! inside a sheet scrolls first. This module gives a handler Astryx's pan:
//! axis, hysteresis, and the release velocity filter.

use crate::{resolve_snap, Axis, PanEvent, PanPhase, Sample, VelocityTracker};
use astryx_core::authoring::{Build, BuildCx, Node};
use astryx_core::gesture::{DragEvent, DragPhase, DragSpec};
use astryx_core::{Element, Point};
use std::rc::Rc;

/// Makes its child draggable along `axis`; reports Astryx pan events.
pub struct Drag<Msg> {
    child: Element<Msg>,
    axis: Axis,
    on_pan: Option<Rc<dyn Fn(PanEvent) -> Option<Msg>>>,
    effect: Option<Rc<dyn Fn(PanEvent)>>,
    id: Option<String>,
}

impl<Msg: 'static> Drag<Msg> {
    /// `on_pan` returns the message an event sends, if any (`None` to send
    /// only at the release, for instance).
    pub fn new(
        child: impl Into<Element<Msg>>,
        axis: Axis,
        on_pan: impl Fn(PanEvent) -> Option<Msg> + 'static,
    ) -> Self {
        Drag {
            child: child.into(),
            axis,
            on_pan: Some(Rc::new(on_pan)),
            effect: None,
            id: None,
        }
    }

    /// A drag that changes only component state: `effect` runs per event.
    pub fn local(
        child: impl Into<Element<Msg>>,
        axis: Axis,
        effect: impl Fn(PanEvent) + 'static,
    ) -> Self {
        Drag {
            child: child.into(),
            axis,
            on_pan: None,
            effect: Some(Rc::new(effect)),
            id: None,
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}

/// Turns core drag events into pan events with Astryx's velocity filter.
/// The tracker lives in the node's local state, so it survives the
/// rebuilds each event causes.
fn pan_events(
    axis: Axis,
    tracker: astryx_core::authoring::LocalHandle<VelocityTracker>,
) -> impl Fn(&DragEvent) -> PanEvent {
    move |e: &DragEvent| {
        let mut t = tracker.get();
        let sample = Sample {
            x: e.at.x,
            y: e.at.y,
            time_ms: e.time_ms,
        };
        if e.phase == DragPhase::Start {
            t.reset();
            t.add(Sample {
                x: e.start.x,
                y: e.start.y,
                time_ms: e.start_ms,
            });
        }
        if e.phase != DragPhase::Cancel {
            t.add(sample);
        }
        tracker.set(t.clone());
        let raw = Point {
            x: e.at.x - e.start.x,
            y: e.at.y - e.start.y,
        };
        let translation = match axis {
            Axis::X => Point { x: raw.x, y: 0.0 },
            Axis::Y => Point { x: 0.0, y: raw.y },
            Axis::Both => raw,
        };
        let velocity = t.velocity(e.time_ms);
        let velocity = match axis {
            Axis::X => Point {
                x: velocity.x,
                y: 0.0,
            },
            Axis::Y => Point {
                x: 0.0,
                y: velocity.y,
            },
            Axis::Both => velocity,
        };
        PanEvent {
            phase: match e.phase {
                DragPhase::Start => PanPhase::Began,
                DragPhase::Move => PanPhase::Changed,
                DragPhase::End => PanPhase::Ended,
                DragPhase::Cancel => PanPhase::Cancelled,
            },
            translation,
            velocity,
        }
    }
}

impl<Msg: 'static> Build<Msg> for Drag<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let axis = self.axis;
        cx.key(self.id.as_deref());
        let to_pan = Rc::new(pan_events(axis, cx.local_handle::<VelocityTracker>()));
        let on_pan = self.on_pan;
        let effect = self.effect;
        let to_pan2 = to_pan.clone();
        let mut node = Node::new(taffy::Style {
            flex_direction: taffy::FlexDirection::Column,
            ..Default::default()
        });
        node.drag = Some(DragSpec {
            wants: Rc::new(move |d: Point| match axis {
                Axis::X => d.x.abs() >= d.y.abs(),
                Axis::Y => d.y.abs() >= d.x.abs(),
                Axis::Both => true,
            }),
            on_drag: on_pan.map(|f| {
                Rc::new(move |e: &DragEvent| f(to_pan(e))) as Rc<dyn Fn(&DragEvent) -> Option<Msg>>
            }),
            effect: effect
                .map(|f| Rc::new(move |e: &DragEvent| f(to_pan2(e))) as Rc<dyn Fn(&DragEvent)>),
        });
        cx.key(self.id.as_deref());
        cx.begin(node);
        cx.child(self.child);
        cx.end();
    }
}

impl<Msg: 'static> From<Drag<Msg>> for Element<Msg> {
    fn from(d: Drag<Msg>) -> Self {
        astryx_core::authoring::element(d)
    }
}

/// A sheet or drawer's position along one axis: follows a pan, and at the
/// release settles on a snap point by Astryx's rules.
#[derive(Clone, Debug, PartialEq)]
pub struct SnapDrag {
    pub points: Vec<f32>,
    /// The snap point it rests on.
    pub index: usize,
    /// Where it is drawn now.
    pub offset: f32,
    start: Option<f32>,
}

impl SnapDrag {
    pub fn new(points: Vec<f32>, index: usize) -> Self {
        let offset = points[index];
        SnapDrag {
            points,
            index,
            offset,
            start: None,
        }
    }

    /// Follows `e` along `y`; returns the snap index chosen at a release.
    pub fn pan(&mut self, e: &PanEvent) -> Option<usize> {
        match e.phase {
            PanPhase::Began => {
                self.start = Some(self.offset);
                self.offset = self.points[self.index] + e.translation.y;
                None
            }
            PanPhase::Changed => {
                self.offset = self.start.unwrap_or(self.offset) + e.translation.y;
                None
            }
            PanPhase::Ended => {
                let start = self.start.take();
                self.offset = start.unwrap_or(self.offset) + e.translation.y;
                self.index = resolve_snap(&self.points, self.offset, e.velocity.y, start);
                self.offset = self.points[self.index];
                Some(self.index)
            }
            _ => {
                self.start = None;
                self.offset = self.points[self.index];
                None
            }
        }
    }
}
