//! Switch: an on/off toggle with a label.

use super::field::{field_methods, label_column, status_box, FieldParts};
use crate::layout::{size_dim, SizeValue};
use crate::primitives::spinner_painter;
use crate::util::{color, component, dim, len, len_auto, radii, square};
use astryx_core::authoring::{Base, BoxDraw, Build, BuildCx, Draw, Indicator, Interact, Node, Semantics};
use astryx_core::contract::{Live, Role};
use astryx_spike_foundations::{args, CxExt};
use std::rc::Rc;
use taffy::{AlignContent, AlignItems, FlexDirection, LengthPercentageAuto, Position, Style};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SwitchSize {
    Sm,
    #[default]
    Md,
}

/// Switch `labelPosition`: the label after (default) or before the track.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LabelPosition {
    Start,
    #[default]
    End,
}

/// Switch `labelSpacing`: next to the track, or pushed to the far end.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LabelSpacing {
    #[default]
    Hug,
    Spread,
}

struct Geometry {
    track: (f32, f32),
    /// Thumb side, inline offset, block offset.
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

/// An on/off toggle (`Switch`). Astryx's `changeAction` promise is the
/// app's message plus `is_loading`; a failed change sends the old value.
pub struct Switch<Msg> {
    base: Base,
    field: FieldParts,
    value: bool,
    on_change: Option<Rc<dyn Fn(bool) -> Msg>>,
    is_loading: bool,
    size: SwitchSize,
    disabled_message: Option<String>,
    label_position: LabelPosition,
    label_spacing: LabelSpacing,
    width: Option<SizeValue>,
}

impl<Msg> Switch<Msg> {
    pub fn new(label: impl Into<String>, value: bool) -> Self {
        Switch {
            base: Base::default(),
            field: FieldParts::new(label),
            value,
            on_change: None,
            is_loading: false,
            size: SwitchSize::Md,
            disabled_message: None,
            label_position: LabelPosition::End,
            label_spacing: LabelSpacing::Hug,
            width: None,
        }
    }
    pub fn on_change(mut self, f: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
    pub fn is_loading(mut self, on: bool) -> Self {
        self.is_loading = on;
        self
    }
    pub fn size(mut self, size: SwitchSize) -> Self {
        self.size = size;
        self
    }
    /// Why it is disabled, shown on hover or focus.
    pub fn disabled_message(mut self, message: impl Into<String>) -> Self {
        self.disabled_message = Some(message.into());
        self
    }
    pub fn label_position(mut self, p: LabelPosition) -> Self {
        self.label_position = p;
        self
    }
    pub fn label_spacing(mut self, s: LabelSpacing) -> Self {
        self.label_spacing = s;
        self
    }
    pub fn width(mut self, w: SizeValue) -> Self {
        self.width = Some(w);
        self
    }
    field_methods!();
}

component!(Switch<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for Switch<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let g = self.size.geometry();
        let f = &self.field;
        let mut outer = Style {
            flex_direction: FlexDirection::Column,
            ..Style::default()
        };
        if let Some(w) = self.width {
            outer.size.width = size_dim(w);
        }
        cx.key(self.base.id.as_deref());
        cx.begin(Node::new(outer));

        let mut sem = Semantics::new(Role::Switch, f.name(cx));
        sem.checked = Some(self.value);
        sem.disabled = f.is_disabled;
        sem.description = f.described();
        let gap = if f.is_label_hidden { 0.0 } else { t.space.s2 };
        let spread = self.label_spacing == LabelSpacing::Spread;
        cx.begin(
            Node::new(Style {
                flex_direction: if self.label_position == LabelPosition::Start {
                    FlexDirection::RowReverse
                } else {
                    FlexDirection::Row
                },
                justify_content: if spread {
                    AlignContent::SPACE_BETWEEN
                } else if self.label_position == LabelPosition::Start {
                    AlignContent::FLEX_END
                } else {
                    AlignContent::FLEX_START
                },
                align_items: AlignItems::FLEX_START,
                gap: taffy::Size {
                    width: len(gap),
                    height: len(0.0),
                },
                ..Style::default()
            })
            .interact(
                Interact::new(
                    self.on_change.as_ref().map(|on| on(!self.value)),
                    f.is_disabled,
                )
                .busy(self.is_loading),
            )
            .sem(sem)
            .tooltip(if f.is_disabled {
                self.disabled_message.clone().or(f.label_tooltip.clone())
            } else {
                None
            }),
        );

        let pill = radii(t.radius.full);
        let overlay = Some(color(c.overlay_pressed));
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
            .draw(Draw::Box(BoxDraw::fill(color(track_fill), pill)))
            .indicator(Indicator::ring(color(c.accent), pill, overlay))
            .opacity(if f.is_disabled { 0.5 } else { 1.0 }),
        );
        let (side, x, y) = if self.value { g.on } else { g.off };
        let thumb_inset = {
            let (left, right) = cx.inline(len_auto(x), LengthPercentageAuto::auto());
            taffy::Rect {
                left,
                right,
                top: len_auto(y),
                bottom: LengthPercentageAuto::auto(),
            }
        };
        cx.begin(
            Node::new(Style {
                position: Position::Absolute,
                // The thumb travels toward inline end, which mirrors in RTL.
                inset: thumb_inset,
                size: square(side),
                ..Style::default()
            })
            .moves()
            .draw(Draw::Box(BoxDraw::fill(color(c.background_surface), pill)))
            .indicator(Indicator {
                ring: None,
                radii: pill,
                overlay,
                field: None,
                hold_fill: None,
            }),
        );
        let loading = cx.t("@astryx.switch.loading", &args! {});
        cx.live_region(self.is_loading.then_some(loading.as_str()), Live::Polite);
        if self.is_loading {
            let d = side - 6.0;
            let mut spinner = Node::new(Style {
                position: Position::Absolute,
                size: square(side - 2.0),
                ..Style::default()
            })
            .draw(Draw::Paint(spinner_painter(cx, d, 2.0, color(c.text_secondary))));
            spinner.centered = true;
            cx.leaf(spinner);
        }
        cx.end();
        cx.end();

        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            justify_content: AlignContent::CENTER,
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

        if f.status.is_some() {
            cx.begin(Node::new(Style {
                margin: taffy::Rect {
                    top: len_auto(t.space.s2),
                    ..taffy::Rect::zero()
                },
                flex_direction: FlexDirection::Column,
                ..Style::default()
            }));
            status_box(cx, f);
            cx.end();
        }
        cx.end();
    }
}
