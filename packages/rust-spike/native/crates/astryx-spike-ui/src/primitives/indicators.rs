//! The choice marks other components draw: a checkbox box, a radio circle,
//! and a single-selection check. They are decorative: the control that hosts
//! one owns its role, state and focus.

use super::icon::icon_node;
use crate::util::{color, mix, radii, square};
use astryx_core::authoring::{BoxDraw, Draw, Node, PathDraw};
use astryx_core::authoring::{Build, BuildCx};
use astryx_core::contract::{Color, PathCmd, Point};
use astryx_core::Element;
use taffy::{AlignItems, JustifyContent, Style};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum IndicatorSize {
    Sm,
    #[default]
    Md,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CheckboxState {
    #[default]
    Unchecked,
    Checked,
    Indeterminate,
}

/// Fill and border for a mark: accent when on, the surface when off, and the
/// hover tint mixed in when `hovered`.
fn mark_colors<Msg>(cx: &BuildCx<Msg>, on: bool, disabled: bool, hovered: bool) -> (Color, Color) {
    let c = &cx.tokens.color;
    let tint = color(c.tint_hover);
    let (mut fill, mut border) = if on {
        (color(c.accent), color(c.accent))
    } else {
        (color(c.background_surface), color(c.border_emphasized))
    };
    if hovered && !disabled {
        if on {
            fill = mix(fill, tint, 0.15);
            border = mix(border, tint, 0.15);
        } else {
            fill = mix(fill, tint, 0.05);
            border = mix(border, tint, 0.2);
        }
    }
    if disabled {
        border = if on { border } else { color(c.border) };
        if !on {
            fill = color(c.background_muted);
        }
    }
    (fill, border)
}

/// The box a checkbox draws (`CheckboxIndicator`).
#[derive(Clone, Copy, Debug, Default)]
pub struct CheckboxIndicator {
    pub state: CheckboxState,
    pub size: IndicatorSize,
    pub is_disabled: bool,
    pub is_hovered: bool,
}

impl CheckboxIndicator {
    pub fn new(state: CheckboxState) -> Self {
        CheckboxIndicator {
            state,
            ..Self::default()
        }
    }
    pub fn size(mut self, size: IndicatorSize) -> Self {
        self.size = size;
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    pub fn is_hovered(mut self, on: bool) -> Self {
        self.is_hovered = on;
        self
    }
}

impl<Msg: 'static> From<CheckboxIndicator> for Element<Msg> {
    fn from(c: CheckboxIndicator) -> Self {
        astryx_core::authoring::element(c)
    }
}

impl<Msg> Build<Msg> for CheckboxIndicator {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let on = self.state != CheckboxState::Unchecked;
        let (fill, border) = mark_colors(cx, on, self.is_disabled, self.is_hovered);
        let (box_side, check_side, bar) = match self.size {
            IndicatorSize::Sm => (20.0, 12.0, (10.0, 2.0)),
            IndicatorSize::Md => (24.0, 14.0, (12.0, 2.0)),
        };
        let on_accent = color(t.color.on_accent);
        cx.begin(
            Node::new(Style {
                size: square(box_side),
                align_items: AlignItems::CENTER,
                justify_content: JustifyContent::CENTER,
                flex_shrink: 0.0,
                ..Style::default()
            })
            .draw(Draw::Box(BoxDraw {
                border: Some((border, t.size.border_width)),
                ..BoxDraw::fill(fill, radii(t.radius.inner))
            }))
            .opacity(if self.is_disabled { 0.5 } else { 1.0 }),
        );
        match self.state {
            CheckboxState::Checked => {
                let p = |x: f32, y: f32| Point { x, y };
                cx.leaf(
                    Node::new(Style {
                        size: square(check_side),
                        ..Style::default()
                    })
                    .draw(Draw::Path(PathDraw {
                        commands: vec![
                            PathCmd::MoveTo(p(8.5, 2.5)),
                            PathCmd::LineTo(p(4.0, 7.5)),
                            PathCmd::LineTo(p(1.5, 5.0)),
                        ],
                        view_box: 10.0,
                        fill: None,
                        stroke: Some((on_accent, 1.5 * check_side / 10.0)),
                    })),
                );
            }
            CheckboxState::Indeterminate => {
                cx.leaf(
                    Node::new(Style {
                        size: taffy::Size {
                            width: taffy::Dimension::length(bar.0),
                            height: taffy::Dimension::length(bar.1),
                        },
                        ..Style::default()
                    })
                    .draw(Draw::Box(BoxDraw::fill(on_accent, radii(1.0)))),
                );
            }
            CheckboxState::Unchecked => {}
        }
        cx.end();
    }
}

/// The circle a radio draws (`RadioIndicator`).
#[derive(Clone, Copy, Debug, Default)]
pub struct RadioIndicator {
    pub is_checked: bool,
    pub size: IndicatorSize,
    pub is_disabled: bool,
    pub is_hovered: bool,
}

impl RadioIndicator {
    pub fn new(is_checked: bool) -> Self {
        RadioIndicator {
            is_checked,
            ..Self::default()
        }
    }
    pub fn size(mut self, size: IndicatorSize) -> Self {
        self.size = size;
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    pub fn is_hovered(mut self, on: bool) -> Self {
        self.is_hovered = on;
        self
    }
}

impl<Msg: 'static> From<RadioIndicator> for Element<Msg> {
    fn from(c: RadioIndicator) -> Self {
        astryx_core::authoring::element(c)
    }
}

impl<Msg> Build<Msg> for RadioIndicator {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let (fill, border) = mark_colors(cx, self.is_checked, self.is_disabled, self.is_hovered);
        let (side, dot) = match self.size {
            IndicatorSize::Sm => (20.0, 8.0),
            IndicatorSize::Md => (24.0, 10.0),
        };
        cx.begin(
            Node::new(Style {
                size: square(side),
                align_items: AlignItems::CENTER,
                justify_content: JustifyContent::CENTER,
                flex_shrink: 0.0,
                ..Style::default()
            })
            .draw(Draw::Box(BoxDraw {
                border: Some((border, t.size.border_width)),
                ..BoxDraw::fill(fill, radii(side / 2.0))
            }))
            .opacity(if self.is_disabled { 0.5 } else { 1.0 }),
        );
        if self.is_checked {
            cx.leaf(
                Node::new(Style {
                    size: square(dot),
                    ..Style::default()
                })
                .draw(Draw::Box(BoxDraw::fill(
                    color(t.color.on_accent),
                    radii(dot / 2.0),
                ))),
            );
        }
        cx.end();
    }
}

/// The single-selection mark (`CheckIndicator`): a check when chosen, an
/// empty slot of the same size when not, so rows keep their alignment.
#[derive(Clone, Copy, Debug, Default)]
pub struct CheckIndicator {
    pub is_checked: bool,
    pub is_disabled: bool,
}

impl CheckIndicator {
    pub fn new(is_checked: bool) -> Self {
        CheckIndicator {
            is_checked,
            is_disabled: false,
        }
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
}

impl<Msg: 'static> From<CheckIndicator> for Element<Msg> {
    fn from(c: CheckIndicator) -> Self {
        astryx_core::authoring::element(c)
    }
}

impl<Msg> Build<Msg> for CheckIndicator {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let c = &cx.tokens.color;
        let ink = if self.is_disabled {
            color(c.icon_disabled)
        } else {
            color(c.accent)
        };
        let side = 16.0;
        cx.begin(Node::new(Style {
            size: square(side),
            align_items: AlignItems::CENTER,
            justify_content: JustifyContent::CENTER,
            flex_shrink: 0.0,
            ..Style::default()
        }));
        if self.is_checked {
            if let Some(node) = icon_node("check", side, ink) {
                cx.leaf(node);
            }
        }
        cx.end();
    }
}
