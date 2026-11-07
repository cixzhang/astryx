//! Icon, Badge, Divider, and ProgressBar.

use super::icon_paths::icon_path;
use super::text::{text_draw, text_node, type_style};
use super::{dim, len, len_auto, radii, square};
use crate::contract::{Color, FontFamily, Role};
use crate::element::{base_props, Base, BoxDraw, Build, BuildCx, Draw, Node, PathDraw, Semantics};
use taffy::{
    AlignItems, Dimension, FlexDirection, JustifyContent, LengthPercentageAuto, Position, Style,
};

pub use super::icon_paths::BUILT_IN as BUILT_IN_ICONS;

/// Icon `color`: a semantic icon color, or the surrounding text color.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum IconColor {
    Primary,
    Secondary,
    Tertiary,
    Disabled,
    Accent,
    Info,
    Success,
    Error,
    Warning,
    #[default]
    Inherit,
}

/// Icon `size`: 12, 16, 20, or 24 points.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum IconSize {
    Xsm,
    Sm,
    #[default]
    Md,
    Lg,
}

impl IconSize {
    pub fn points(self) -> f32 {
        match self {
            IconSize::Xsm => 12.0,
            IconSize::Sm => 16.0,
            IconSize::Md => 20.0,
            IconSize::Lg => 24.0,
        }
    }
}

impl IconColor {
    pub(crate) fn resolve<Msg>(self, cx: &BuildCx<Msg>) -> Color {
        let c = &cx.tokens.color;
        match self {
            IconColor::Primary => c.icon_primary.into(),
            IconColor::Secondary | IconColor::Tertiary => c.icon_secondary.into(),
            IconColor::Disabled => c.icon_disabled.into(),
            IconColor::Accent => c.accent.into(),
            IconColor::Info => c.icon_info.into(),
            IconColor::Success => c.icon_success.into(),
            IconColor::Error => c.icon_error.into(),
            IconColor::Warning => c.icon_warning.into(),
            IconColor::Inherit => cx.inherited().color,
        }
    }
}

/// An icon node, or `None` for a name Astryx does not define.
pub(crate) fn icon_node<Msg>(name: &str, side: f32, color: Color) -> Option<Node<Msg>> {
    let (commands, filled, width) = icon_path(name)?;
    let mut node = Node::new(Style {
        size: square(side),
        flex_shrink: 0.0,
        ..Style::default()
    })
    .draw(Draw::Path(PathDraw {
        commands: commands.to_vec(),
        view_box: 24.0,
        fill: filled.then_some(color),
        stroke: (!filled).then_some((color, width)),
    }));
    node.icon = BUILT_IN_ICONS
        .iter()
        .find(|n| **n == name)
        .map(|n| (*n, side));
    Some(node)
}

/// A shared icon by name (`Icon`), such as `"check"` or `"chevronDown"`.
/// Drawn from Astryx's built-in icon set; an unknown name draws nothing.
#[derive(Clone, Debug)]
pub struct Icon {
    base: Base,
    icon: String,
    color: IconColor,
    size: IconSize,
    label: Option<String>,
}

impl Icon {
    pub fn new(icon: impl Into<String>) -> Self {
        Icon {
            base: Base::default(),
            icon: icon.into(),
            color: IconColor::default(),
            size: IconSize::default(),
            label: None,
        }
    }
    pub fn color(mut self, color: IconColor) -> Self {
        self.color = color;
        self
    }
    pub fn size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }
    /// The accessible name of a meaningful icon; without one it is decorative.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into()).filter(|l| !l.is_empty());
        self
    }
}

base_props!(Icon);

impl Icon {
    pub(crate) fn label_text(&self) -> Option<&str> {
        self.label.as_deref()
    }
    pub(crate) fn side(&self) -> f32 {
        self.size.points()
    }
    pub(crate) fn name(&self) -> &str {
        &self.icon
    }
    pub(crate) fn resolved_color<Msg>(&self, cx: &BuildCx<Msg>) -> Color {
        self.color.resolve(cx)
    }
}

impl<Msg> Build<Msg> for Icon {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let color = self.color.resolve(cx);
        let Some(mut node) = icon_node(&self.icon, self.size.points(), color) else {
            debug_assert!(false, "Astryx has no icon named {:?}", self.icon);
            return;
        };
        node.sem = self.label.map(|label| Semantics::new(Role::Image, label));
        cx.key(self.base.id.as_deref());
        cx.leaf(node);
    }
}

/// Badge `variant`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BadgeVariant {
    #[default]
    Neutral,
    Info,
    Success,
    Warning,
    Error,
    Blue,
    Cyan,
    Green,
    Orange,
    Pink,
    Purple,
    Red,
    Teal,
    Yellow,
}

/// A short status or count label on a pill (`Badge`).
#[derive(Clone, Debug)]
pub struct Badge {
    base: Base,
    label: String,
    variant: BadgeVariant,
}

impl Badge {
    pub fn new(label: impl Into<String>) -> Self {
        Badge {
            base: Base::default(),
            label: label.into(),
            variant: BadgeVariant::default(),
        }
    }
    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }
}

base_props!(Badge);

impl<Msg> Build<Msg> for Badge {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let (bg, fg) = match self.variant {
            BadgeVariant::Neutral => (c.neutral, c.text_primary),
            BadgeVariant::Info => (c.info, c.on_info),
            BadgeVariant::Success => (c.success, c.on_success),
            BadgeVariant::Warning => (c.warning, c.on_warning),
            BadgeVariant::Error => (c.error, c.on_error),
            BadgeVariant::Blue => (c.background_blue, c.text_blue),
            BadgeVariant::Cyan => (c.background_cyan, c.text_cyan),
            BadgeVariant::Green => (c.background_green, c.text_green),
            BadgeVariant::Orange => (c.background_orange, c.text_orange),
            BadgeVariant::Pink => (c.background_pink, c.text_pink),
            BadgeVariant::Purple => (c.background_purple, c.text_purple),
            BadgeVariant::Red => (c.background_red, c.text_red),
            BadgeVariant::Teal => (c.background_teal, c.text_teal),
            BadgeVariant::Yellow => (c.background_yellow, c.text_yellow),
        };
        let mut style = type_style(t.font.supporting, FontFamily::Body);
        style.weight = t.font.weight.medium;
        let node = Node::new(Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            justify_content: JustifyContent::CENTER,
            // A badge hugs its label, as an inline element does.
            align_self: Some(AlignItems::FLEX_START),
            size: taffy::Size {
                width: Dimension::auto(),
                height: dim(t.space.s5),
            },
            padding: taffy::Rect {
                left: len(t.space.s2),
                right: len(t.space.s2),
                top: len(0.0),
                bottom: len(0.0),
            },
            // Never shorter than the label's line, for themes with large type.
            min_size: taffy::Size {
                width: len_auto(0.0),
                height: len_auto(style.size * style.leading),
            },
            max_size: taffy::Size {
                width: LengthPercentageAuto::percent(1.0),
                height: LengthPercentageAuto::auto(),
            },
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw::fill(bg.into(), radii(t.radius.full))))
        .sem(Semantics::new(Role::Text, self.label.clone()));
        cx.key(self.base.id.as_deref());
        cx.begin(node);
        let mut label = text_node(self.label, style, fg.into());
        text_draw(&mut label).max_lines = Some(1);
        label.style.min_size.width = len_auto(0.0);
        cx.leaf(label);
        cx.end();
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DividerOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DividerVariant {
    #[default]
    Subtle,
    Strong,
}

/// A line between content, with an optional label (`Divider`).
#[derive(Clone, Debug, Default)]
pub struct Divider {
    base: Base,
    orientation: DividerOrientation,
    label: Option<String>,
    variant: DividerVariant,
    is_full_bleed: bool,
}

impl Divider {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn orientation(mut self, orientation: DividerOrientation) -> Self {
        self.orientation = orientation;
        self
    }
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    pub fn variant(mut self, variant: DividerVariant) -> Self {
        self.variant = variant;
        self
    }
    /// Reaches through the padding of the Card around it.
    pub fn is_full_bleed(mut self, on: bool) -> Self {
        self.is_full_bleed = on;
        self
    }
}

base_props!(Divider);

impl<Msg> Build<Msg> for Divider {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let horizontal = self.orientation == DividerOrientation::Horizontal;
        let line_color: Color = if self.variant == DividerVariant::Strong {
            c.border_emphasized.into()
        } else {
            c.border.into()
        };
        let bleed = if self.is_full_bleed {
            cx.container_padding.last().copied().unwrap_or(0.0)
        } else {
            0.0
        };
        let mut style = Style {
            flex_direction: if horizontal {
                FlexDirection::Row
            } else {
                FlexDirection::Column
            },
            align_items: AlignItems::CENTER,
            align_self: Some(AlignItems::STRETCH),
            flex_shrink: 0.0,
            ..Style::default()
        };
        if horizontal {
            style.margin.left = len_auto(-bleed);
            style.margin.right = len_auto(-bleed);
        } else {
            style.margin.top = len_auto(-bleed);
            style.margin.bottom = len_auto(-bleed);
        }
        let mut sem = Semantics::new(Role::Separator, "");
        sem.label = self.label.clone();
        let node = Node::new(style).sem(sem);
        cx.key(self.base.id.as_deref());
        cx.begin(node);
        let bw = t.size.border_width;
        let line = |cx: &mut BuildCx<Msg>| {
            let size = if horizontal {
                taffy::Size {
                    width: Dimension::auto(),
                    height: dim(bw),
                }
            } else {
                taffy::Size {
                    width: dim(bw),
                    height: Dimension::auto(),
                }
            };
            cx.leaf(
                Node::new(Style {
                    size,
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    ..Style::default()
                })
                .draw(Draw::Box(BoxDraw::fill(line_color, radii(0.0)))),
            );
        };
        line(cx);
        if let Some(label) = self.label {
            let pad = len(t.space.s3);
            let padding = if horizontal {
                taffy::Rect {
                    left: pad,
                    right: pad,
                    top: len(0.0),
                    bottom: len(0.0),
                }
            } else {
                taffy::Rect {
                    left: len(0.0),
                    right: len(0.0),
                    top: pad,
                    bottom: pad,
                }
            };
            cx.begin(Node::new(Style {
                padding,
                flex_shrink: 0.0,
                ..Style::default()
            }));
            cx.leaf(text_node(
                label,
                type_style(t.font.supporting, FontFamily::Body),
                c.text_secondary.into(),
            ));
            cx.end();
            line(cx);
        }
        cx.end();
    }
}

/// ProgressBar `variant`: the fill color.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ProgressBarVariant {
    #[default]
    Accent,
    Success,
    Warning,
    Error,
    Neutral,
}

/// A target mark on a ProgressBar track, in the bar's `0..max` scale.
#[derive(Clone, Debug, PartialEq)]
pub struct ProgressBarMark {
    pub value: f64,
    pub label: String,
}

/// Progress toward completion (`ProgressBar`): a label, an optional value
/// label, and an 8 point track.
#[derive(Clone, Debug)]
pub struct ProgressBar {
    base: Base,
    label: String,
    value: f64,
    max: f64,
    is_label_hidden: bool,
    has_value_label: bool,
    variant: ProgressBarVariant,
    is_indeterminate: bool,
    marks: Vec<ProgressBarMark>,
    is_disabled: bool,
}

impl ProgressBar {
    pub fn new(label: impl Into<String>) -> Self {
        ProgressBar {
            base: Base::default(),
            label: label.into(),
            value: 0.0,
            max: 100.0,
            is_label_hidden: false,
            has_value_label: false,
            variant: ProgressBarVariant::default(),
            is_indeterminate: false,
            marks: Vec::new(),
            is_disabled: false,
        }
    }

    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }
    /// The value at completion; 100 when unset.
    pub fn max(mut self, max: f64) -> Self {
        self.max = max;
        self
    }
    pub fn is_label_hidden(mut self, on: bool) -> Self {
        self.is_label_hidden = on;
        self
    }
    /// Shows the value as a whole percentage beside the label.
    pub fn has_value_label(mut self, on: bool) -> Self {
        self.has_value_label = on;
        self
    }
    pub fn variant(mut self, variant: ProgressBarVariant) -> Self {
        self.variant = variant;
        self
    }
    /// A sliding fill for work of unknown length.
    pub fn is_indeterminate(mut self, on: bool) -> Self {
        self.is_indeterminate = on;
        self
    }
    pub fn marks(mut self, marks: impl IntoIterator<Item = ProgressBarMark>) -> Self {
        self.marks = marks.into_iter().collect();
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    fn fraction(&self) -> f32 {
        if self.max <= 0.0 {
            return 0.0;
        }
        (self.value / self.max).clamp(0.0, 1.0) as f32
    }
}

base_props!(ProgressBar);

/// Seconds for one indeterminate slide.
const SLIDE_SECONDS: f32 = 1.5;

impl<Msg> Build<Msg> for ProgressBar {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let fraction = self.fraction();
        let percent = format!("{}%", (fraction * 100.0).round());
        let mut sem = Semantics::new(Role::ProgressBar, self.label.clone());
        sem.value = (!self.is_indeterminate).then(|| percent.clone());
        sem.disabled = self.is_disabled;
        cx.key(self.base.id.as_deref());
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Column,
                gap: taffy::Size {
                    width: len(0.0),
                    height: len(t.space.s1),
                },
                align_self: Some(AlignItems::STRETCH),
                ..Style::default()
            })
            .sem(sem),
        );
        let show_value = self.has_value_label && !self.is_indeterminate;
        if !self.is_label_hidden || show_value {
            cx.begin(Node::new(Style {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SPACE_BETWEEN,
                align_items: AlignItems::BASELINE,
                ..Style::default()
            }));
            let (label_color, value_color): (Color, Color) = if self.is_disabled {
                (c.text_disabled.into(), c.text_disabled.into())
            } else {
                (c.text_primary.into(), c.text_secondary.into())
            };
            if !self.is_label_hidden {
                let mut style = type_style(t.font.body, FontFamily::Body);
                style.weight = t.font.weight.medium;
                cx.leaf(text_node(self.label.clone(), style, label_color));
            }
            if show_value {
                cx.leaf(text_node(
                    percent,
                    type_style(t.font.body, FontFamily::Body),
                    value_color,
                ));
            }
            cx.end();
        }
        let fill: Color = if self.is_disabled {
            c.text_disabled.into()
        } else {
            match self.variant {
                ProgressBarVariant::Accent => c.accent,
                ProgressBarVariant::Success => c.success,
                ProgressBarVariant::Warning => c.warning,
                ProgressBarVariant::Error => c.error,
                ProgressBarVariant::Neutral => c.text_disabled,
            }
            .into()
        };
        let pill = radii(t.radius.full);
        let mut track = Node::new(Style {
            size: taffy::Size {
                width: Dimension::percent(1.0),
                height: dim(8.0),
            },
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw::fill(c.background_muted.into(), pill)));
        track.clip = Some(pill);
        cx.begin(track);
        if self.is_indeterminate {
            // Under reduced motion the fill spans the track and stays still.
            let slides = cx.animate();
            let ix = cx.leaf(
                Node::new(Style {
                    position: Position::Absolute,
                    size: taffy::Size {
                        width: Dimension::percent(if slides { 0.4 } else { 1.0 }),
                        height: Dimension::percent(1.0),
                    },
                    ..Style::default()
                })
                .draw(Draw::Box(BoxDraw::fill(fill, pill))),
            );
            if slides {
                cx.nodes[ix as usize].slide = Some(SLIDE_SECONDS);
            }
        } else if fraction > 0.0 {
            cx.leaf(
                Node::new(Style {
                    size: taffy::Size {
                        width: Dimension::percent(fraction),
                        height: Dimension::percent(1.0),
                    },
                    ..Style::default()
                })
                .draw(Draw::Box(BoxDraw::fill(fill, pill))),
            );
        }
        if !self.is_indeterminate {
            for mark in &self.marks {
                let at = if self.max > 0.0 {
                    (mark.value / self.max).clamp(0.0, 1.0) as f32
                } else {
                    0.0
                };
                // A mark takes its color from what it sits on.
                let on_fill = at <= fraction && fraction > 0.0;
                let color: Color = match (on_fill, self.is_disabled) {
                    (_, true) => c.text_secondary.into(),
                    (true, false) => match self.variant {
                        ProgressBarVariant::Accent => c.on_accent,
                        ProgressBarVariant::Success => c.on_success,
                        ProgressBarVariant::Warning => c.on_warning,
                        ProgressBarVariant::Error => c.on_error,
                        ProgressBarVariant::Neutral => c.text_primary,
                    }
                    .into(),
                    (false, false) => c.text_primary.into(),
                };
                cx.leaf(
                    Node::new(Style {
                        position: Position::Absolute,
                        // Marks measure from inline start, as the fill grows.
                        inset: {
                            let (left, right) = cx.inline(
                                LengthPercentageAuto::percent(at),
                                LengthPercentageAuto::auto(),
                            );
                            taffy::Rect {
                                left,
                                right,
                                top: len_auto(0.0),
                                bottom: LengthPercentageAuto::auto(),
                            }
                        },
                        margin: {
                            let (left, right) = cx.inline(len_auto(-1.0), len_auto(0.0));
                            taffy::Rect {
                                left,
                                right,
                                ..taffy::Rect::zero()
                            }
                        },
                        size: taffy::Size {
                            width: dim(2.0),
                            height: dim(8.0),
                        },
                        ..Style::default()
                    })
                    .draw(Draw::Box(BoxDraw::fill(color, radii(0.0))))
                    .sem(Semantics::new(Role::Image, mark.label.clone())),
                );
            }
        }
        cx.end();
        cx.end();
    }
}
