//! Switch.

use super::field::{label_column, status_box, FieldParts, Status};
use super::{dim, len, len_auto, radii, square};
use crate::contract::Role;
use crate::element::{
    base_props, Base, BoxDraw, Build, BuildCx, Draw, Indicator, Interact, Node, Semantics,
};
use std::rc::Rc;
use taffy::{AlignItems, FlexDirection, JustifyContent, LengthPercentageAuto, Position, Style};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SwitchSize {
    /// A 32 × 20 track.
    Sm,
    /// A 40 × 24 track.
    #[default]
    Md,
}

/// Track and thumb geometry, as Astryx's Switch draws them: the thumb grows
/// when on and travels toward the inline end.
struct Geometry {
    track: (f32, f32),
    /// Thumb side and its top-left inside the track, off then on.
    off: (f32, f32, f32),
    on: (f32, f32, f32),
}

impl SwitchSize {
    fn geometry(self) -> Geometry {
        match self {
            SwitchSize::Sm => Geometry {
                track: (32.0, 20.0),
                off: (14.0, 2.0, 3.0),
                on: (16.0, 14.0, 2.0),
            },
            SwitchSize::Md => Geometry {
                track: (40.0, 24.0),
                off: (16.0, 4.0, 4.0),
                on: (20.0, 18.0, 2.0),
            },
        }
    }
}

/// An on/off control with a label (`Switch`). The whole row is one focus
/// stop; A toggles it and the ring draws on the track.
pub struct Switch<Msg> {
    base: Base,
    field: FieldParts,
    value: bool,
    on_change: Option<Rc<dyn Fn(bool) -> Msg>>,
    is_loading: bool,
    size: SwitchSize,
    disabled_message: Option<String>,
    label_tooltip: Option<String>,
}

impl<Msg> Switch<Msg> {
    /// A switch named `label`, on when `value` is true.
    pub fn new(label: impl Into<String>, value: bool) -> Self {
        Switch {
            base: Base::default(),
            field: FieldParts {
                label: label.into(),
                ..FieldParts::default()
            },
            value,
            on_change: None,
            is_loading: false,
            size: SwitchSize::default(),
            disabled_message: None,
            label_tooltip: None,
        }
    }
    /// Builds the message a toggle sends from the new value.
    pub fn on_change(mut self, f: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
    /// A spinner in the thumb; the switch keeps focus and refuses changes.
    pub fn is_loading(mut self, on: bool) -> Self {
        self.is_loading = on;
        self
    }
    /// Keeps the label as the accessible name without drawing it.
    pub fn is_label_hidden(mut self, on: bool) -> Self {
        self.field.is_label_hidden = on;
        self
    }
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.field.description = Some(description.into());
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.field.is_disabled = on;
        self
    }
    pub fn size(mut self, size: SwitchSize) -> Self {
        self.size = size;
        self
    }
    /// Kept for the Astryx contract; tooltips are not drawn yet.
    pub fn disabled_message(mut self, message: impl Into<String>) -> Self {
        self.disabled_message = Some(message.into());
        self
    }
    pub fn is_optional(mut self, on: bool) -> Self {
        self.field.is_optional = on;
        self
    }
    pub fn is_required(mut self, on: bool) -> Self {
        self.field.is_required = on;
        self
    }
    pub fn status(mut self, status: Status) -> Self {
        self.field.status = Some(status);
        self
    }
    /// Kept for the Astryx contract; tooltips are not drawn yet.
    pub fn label_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.label_tooltip = Some(tooltip.into());
        self
    }
}

base_props!(Switch<Msg>);

impl<Msg> Build<Msg> for Switch<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let g = self.size.geometry();
        let f = &self.field;
        cx.key(self.base.id.as_deref());
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            ..Style::default()
        }));

        let mut sem = Semantics::new(Role::Switch, f.label.clone());
        sem.checked = Some(self.value);
        sem.disabled = f.is_disabled;
        sem.description = f.description.clone();
        let gap = if f.is_label_hidden { 0.0 } else { t.space.s2 };
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                gap: taffy::Size {
                    width: len(gap),
                    height: len(0.0),
                },
                ..Style::default()
            })
            .interact(
                Interact::new(
                    self.on_change.as_ref().map(|f| f(!self.value)),
                    f.is_disabled,
                )
                .busy(self.is_loading),
            )
            .sem(sem)
            .tooltip(if f.is_disabled {
                self.disabled_message.clone().or(self.label_tooltip.clone())
            } else {
                self.label_tooltip.clone()
            }),
        );

        let pill = radii(t.radius.full);
        let overlay = Some(c.overlay_pressed.into());
        let track_fill = if self.value {
            c.accent
        } else {
            c.background_gray
        };
        cx.begin(
            Node::new(Style {
                size: taffy::Size {
                    width: dim(g.track.0),
                    height: dim(g.track.1),
                },
                flex_shrink: 0.0,
                ..Style::default()
            })
            .draw(Draw::Box(BoxDraw::fill(track_fill.into(), pill)))
            .indicator(Indicator::ring(c.accent.into(), pill, overlay))
            .opacity(if f.is_disabled { 0.5 } else { 1.0 }),
        );
        let (side, x, y) = if self.value { g.on } else { g.off };
        cx.begin(
            Node::new(Style {
                position: Position::Absolute,
                // The thumb travels toward inline end, which mirrors under RTL.
                inset: {
                    let (left, right) = cx.inline(len_auto(x), LengthPercentageAuto::auto());
                    taffy::Rect {
                        left,
                        right,
                        top: len_auto(y),
                        bottom: LengthPercentageAuto::auto(),
                    }
                },
                size: square(side),
                ..Style::default()
            })
            .moves()
            .draw(Draw::Box(BoxDraw::fill(c.background_surface.into(), pill)))
            .indicator(Indicator {
                ring: None,
                radii: pill,
                overlay,
                field: None,
                hold_fill: None,
            }),
        );
        // The busy start is announced politely.
        cx.live_region(
            self.is_loading.then_some(super::strings::LOADING),
            crate::contract::Live::Polite,
        );
        if self.is_loading {
            let mut spinner = Node::new(Style {
                position: Position::Absolute,
                size: square(side - 4.0),
                ..Style::default()
            })
            .draw(Draw::Spinner(c.text_secondary.into()));
            spinner.centered = true;
            cx.leaf(spinner);
            cx.animate();
        }
        cx.end();
        cx.end();

        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::CENTER,
            min_size: taffy::Size {
                width: len_auto(0.0),
                height: len_auto(g.track.1),
            },
            flex_shrink: 1.0,
            ..Style::default()
        }));
        label_column(cx, f);
        cx.end();
        cx.end();

        if let Some(status) = &f.status {
            cx.begin(Node::new(Style {
                margin: taffy::Rect {
                    top: len_auto(t.space.s2),
                    ..taffy::Rect::zero()
                },
                flex_direction: FlexDirection::Column,
                ..Style::default()
            }));
            status_box(cx, status);
            cx.end();
        }
        cx.end();
    }
}
