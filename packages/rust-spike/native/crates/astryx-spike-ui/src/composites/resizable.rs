//! Resizable: a pane with a size the person drags (`useResizable` plus
//! `ResizeHandle`).

use crate::util::{color, len, radii};
use astryx_core::authoring::{Base, BoxDraw, Build, BuildCx, Draw, Node, Semantics};
use astryx_core::contract::{A11yOrientation, Role};
use astryx_core::Element;
use astryx_spike_gestures::runtime::Drag;
use astryx_spike_gestures::{Axis, PanPhase};
use taffy::{AlignItems, Dimension, FlexDirection, Position, Style};

/// ResizeHandle `direction`: the axis the pane grows along.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ResizeDirection {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct PaneState {
    size: Option<f32>,
    drag_from: Option<f32>,
}

/// A pane and the rest of the space, with a handle between them that
/// resizes the pane (`useResizable` + `ResizeHandle`). The pane keeps its
/// own size; `on_resize` reports each settled size.
pub struct Resizable<Msg> {
    base: Base,
    id: String,
    label: String,
    pane: Element<Msg>,
    rest: Element<Msg>,
    direction: ResizeDirection,
    default_size: f32,
    min_size: f32,
    max_size: f32,
    collapsible: bool,
    collapsed_size: f32,
    snaps: Vec<f32>,
    is_reversed: bool,
    is_disabled: bool,
    has_divider: bool,
}

impl<Msg> Resizable<Msg> {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        pane: impl Into<Element<Msg>>,
        rest: impl Into<Element<Msg>>,
    ) -> Self {
        Resizable {
            base: Base::default(),
            id: id.into(),
            label: label.into(),
            pane: pane.into(),
            rest: rest.into(),
            direction: ResizeDirection::Horizontal,
            default_size: 250.0,
            min_size: 50.0,
            max_size: f32::INFINITY,
            collapsible: false,
            collapsed_size: 40.0,
            snaps: Vec::new(),
            is_reversed: false,
            is_disabled: false,
            has_divider: false,
        }
    }
    pub fn direction(mut self, d: ResizeDirection) -> Self {
        self.direction = d;
        self
    }
    pub fn default_size(mut self, s: f32) -> Self {
        self.default_size = s;
        self
    }
    pub fn min_size(mut self, s: f32) -> Self {
        self.min_size = s;
        self
    }
    pub fn max_size(mut self, s: f32) -> Self {
        self.max_size = s;
        self
    }
    /// Dragging well below the minimum collapses the pane to
    /// `collapsed_size`.
    pub fn collapsible(mut self, on: bool) -> Self {
        self.collapsible = on;
        self
    }
    pub fn collapsed_size(mut self, s: f32) -> Self {
        self.collapsed_size = s;
        self
    }
    /// Sizes a release near snaps to (within 8 points).
    pub fn snaps(mut self, s: impl IntoIterator<Item = f32>) -> Self {
        self.snaps = s.into_iter().collect();
        self
    }
    /// The pane is at the end (right or bottom): dragging toward the start
    /// grows it.
    pub fn is_reversed(mut self, on: bool) -> Self {
        self.is_reversed = on;
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    pub fn has_divider(mut self, on: bool) -> Self {
        self.has_divider = on;
        self
    }
}

crate::util::component!(Resizable<Msg>);

/// The size a drag asks for, within the limits, collapsed or snapped.
pub fn resolve_size(
    wanted: f32,
    min: f32,
    max: f32,
    collapsible: bool,
    collapsed: f32,
    snaps: &[f32],
    released: bool,
) -> f32 {
    if collapsible && wanted < (min + collapsed) / 2.0 {
        return collapsed;
    }
    let size = wanted.clamp(min, max.max(min));
    if released {
        if let Some(s) = snaps.iter().copied().find(|s| (s - size).abs() <= 8.0) {
            return s.clamp(min, max.max(min));
        }
    }
    size
}

impl<Msg: 'static> Build<Msg> for Resizable<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let row = self.direction == ResizeDirection::Horizontal;
        cx.key(Some(&self.id));
        let h = cx.local_handle::<PaneState>();
        let state = h.get();
        let size = state.size.unwrap_or(self.default_size);
        cx.begin(Node::new(Style {
            flex_direction: match (row, self.is_reversed) {
                (true, false) => FlexDirection::Row,
                (true, true) => FlexDirection::RowReverse,
                (false, false) => FlexDirection::Column,
                (false, true) => FlexDirection::ColumnReverse,
            },
            align_items: AlignItems::STRETCH,
            flex_grow: 1.0,
            ..Style::default()
        }));
        // The pane.
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            size: if row {
                taffy::Size {
                    width: Dimension::length(size),
                    height: Dimension::auto(),
                }
            } else {
                taffy::Size {
                    width: Dimension::auto(),
                    height: Dimension::length(size),
                }
            },
            overflow: taffy::Point {
                x: taffy::Overflow::Clip,
                y: taffy::Overflow::Clip,
            },
            ..Style::default()
        }));
        cx.child(self.pane);
        cx.end();
        // The handle: a one-point rule with a 16 point hit area across it.
        let line = if self.has_divider || !self.is_disabled {
            color(t.color.border)
        } else {
            astryx_core::contract::Color::TRANSPARENT
        };
        let hit = 16.0;
        let (min, max, coll, cs) = (
            self.min_size,
            self.max_size,
            self.collapsible,
            self.collapsed_size,
        );
        let snaps = self.snaps.clone();
        let sign = if self.is_reversed { -1.0 } else { 1.0 };
        let hh = h.clone();
        let grip = astryx_core::authoring::element(HitArea { row, hit });
        let handle_child: Element<Msg> = if self.is_disabled {
            grip
        } else {
            Drag::local(grip, if row { Axis::X } else { Axis::Y }, move |e| {
                let mut s = hh.get();
                let from = *s.drag_from.get_or_insert(s.size.unwrap_or(size));
                let delta = sign
                    * if row {
                        e.translation.x
                    } else {
                        e.translation.y
                    };
                let released = matches!(e.phase, PanPhase::Ended | PanPhase::Cancelled);
                s.size = Some(resolve_size(
                    from + delta,
                    min,
                    max,
                    coll,
                    cs,
                    &snaps,
                    released,
                ));
                if released {
                    s.drag_from = None;
                }
                hh.set(s);
            })
            .id(format!("{}:handle", self.id))
            .into()
        };
        let mut sem = Semantics::new(Role::Separator, self.label);
        sem.value = Some(format!("{}", size.round()));
        sem.disabled = self.is_disabled;
        sem.state.orientation = Some(if row {
            A11yOrientation::Vertical
        } else {
            A11yOrientation::Horizontal
        });
        cx.begin(
            Node::new(Style {
                flex_shrink: 0.0,
                size: if row {
                    taffy::Size {
                        width: Dimension::length(t.size.border_width),
                        height: Dimension::auto(),
                    }
                } else {
                    taffy::Size {
                        width: Dimension::auto(),
                        height: Dimension::length(t.size.border_width),
                    }
                },
                overflow: taffy::Point {
                    x: taffy::Overflow::Visible,
                    y: taffy::Overflow::Visible,
                },
                ..Style::default()
            })
            .draw(Draw::Box(BoxDraw::fill(line, radii(0.0))))
            .sem(sem),
        );
        cx.child(handle_child);
        cx.end();
        // The rest of the space.
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_size: taffy::Size {
                width: taffy::LengthPercentageAuto::length(0.0),
                height: taffy::LengthPercentageAuto::length(0.0),
            },
            ..Style::default()
        }));
        cx.child(self.rest);
        cx.end();
        cx.end();
    }
}

struct HitArea {
    row: bool,
    hit: f32,
}

impl<Msg: 'static> Build<Msg> for HitArea {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let half = self.hit / 2.0;
        let (w, h) = if self.row {
            (Dimension::length(self.hit), Dimension::percent(1.0))
        } else {
            (Dimension::percent(1.0), Dimension::length(self.hit))
        };
        let _ = len(0.0);
        cx.leaf(Node::new(Style {
            position: Position::Absolute,
            inset: if self.row {
                taffy::Rect {
                    left: taffy::LengthPercentageAuto::length(-half),
                    right: taffy::LengthPercentageAuto::auto(),
                    top: taffy::LengthPercentageAuto::length(0.0),
                    bottom: taffy::LengthPercentageAuto::auto(),
                }
            } else {
                taffy::Rect {
                    left: taffy::LengthPercentageAuto::length(0.0),
                    right: taffy::LengthPercentageAuto::auto(),
                    top: taffy::LengthPercentageAuto::length(-half),
                    bottom: taffy::LengthPercentageAuto::auto(),
                }
            },
            size: taffy::Size {
                width: w,
                height: h,
            },
            ..Style::default()
        }));
    }
}
