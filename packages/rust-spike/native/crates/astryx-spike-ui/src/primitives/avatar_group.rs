//! AvatarGroup: overlapping avatars with an optional "+N" indicator.

use crate::util::{color, component, len, len_auto, radii, rect};
use astryx_core::authoring::{
    text_draw, text_node, type_style, Base, BoxDraw, Build, BuildCx, Draw, Indicator, Interact,
    Node, Semantics,
};
use astryx_core::contract::{FontFamily, Role};
use astryx_core::{Avatar, AvatarShape, AvatarSize, Element};
use astryx_spike_foundations::{args, CxExt};
use taffy::{AlignItems, Dimension, FlexDirection, JustifyContent, LengthPercentageAuto, Style};

/// Each avatar overlaps the one before it by this share of its size.
const OVERLAP_RATIO: f32 = 0.25;
/// The ring that separates stacked avatars.
const RING: f32 = 2.0;
const OVERFLOW_FONT_RATIO: f32 = 0.35;

/// Stacked avatars (`AvatarGroup`). The group sets every avatar's size and
/// shape, as Astryx's group context does; callers pass only the avatars to
/// show, then an [`AvatarGroupOverflow`] for the rest.
pub struct AvatarGroup<Msg> {
    base: Base,
    size: AvatarSize,
    shape: AvatarShape,
    label: Option<String>,
    avatars: Vec<Avatar<Msg>>,
    overflow: Option<AvatarGroupOverflow<Msg>>,
}

impl<Msg> Default for AvatarGroup<Msg> {
    fn default() -> Self {
        AvatarGroup {
            base: Base::default(),
            size: AvatarSize::Md,
            shape: AvatarShape::Circle,
            label: None,
            avatars: Vec::new(),
            overflow: None,
        }
    }
}

impl<Msg> AvatarGroup<Msg> {
    pub fn new(avatars: impl IntoIterator<Item = Avatar<Msg>>) -> Self {
        AvatarGroup {
            avatars: avatars.into_iter().collect(),
            ..Self::default()
        }
    }
    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }
    pub fn shape(mut self, shape: AvatarShape) -> Self {
        self.shape = shape;
        self
    }
    /// The group's accessible name (`aria-label`).
    pub fn aria_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    pub fn overflow(mut self, overflow: AvatarGroupOverflow<Msg>) -> Self {
        self.overflow = Some(overflow);
        self
    }
}

component!(AvatarGroup<Msg>);

/// The "+N" indicator after the visible avatars (`AvatarGroupOverflow`).
pub struct AvatarGroupOverflow<Msg> {
    count: u32,
    label: Option<String>,
    on_click: Option<Msg>,
}

impl<Msg> AvatarGroupOverflow<Msg> {
    pub fn new(count: u32) -> Self {
        AvatarGroupOverflow {
            count,
            label: None,
            on_click: None,
        }
    }
    /// Content in place of "+N".
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    /// Makes the indicator a button.
    pub fn on_click(mut self, msg: Msg) -> Self {
        self.on_click = Some(msg);
        self
    }
}

fn shape_radius(shape: AvatarShape, side: f32) -> f32 {
    match shape {
        AvatarShape::Circle => side / 2.0,
        AvatarShape::Rounded => (side * 0.2).round(),
        AvatarShape::Square => 0.0,
    }
}

impl<Msg: Clone + 'static> Build<Msg> for AvatarGroup<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let side = self.size.points();
        let overlap = (side * OVERLAP_RATIO).round();
        let surface = color(t.color.background_surface);
        let (start, _) = cx.inline(0usize, 1usize);
        let margin_start = |m: f32| {
            let mut r = rect(len_auto(0.0));
            if start == 0 {
                r.left = len_auto(m);
            } else {
                r.right = len_auto(m);
            }
            r
        };
        let mut padding = rect(len(0.0));
        if !self.avatars.is_empty() || self.overflow.is_some() {
            if start == 0 {
                padding.left = len(overlap);
            } else {
                padding.right = len(overlap);
            }
        }
        let label = self
            .label
            .unwrap_or_else(|| cx.t("@astryx.avatarGroup.label", &args! {}));
        cx.key(self.base.id.as_deref());
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                padding,
                flex_shrink: 0.0,
                ..Style::default()
            })
            .sem(Semantics::new(Role::Group, label)),
        );
        let ring_radius = shape_radius(self.shape, side + RING * 2.0);
        for avatar in self.avatars {
            cx.begin(
                Node::new(Style {
                    padding: rect(len(RING)),
                    margin: margin_start(-overlap),
                    flex_shrink: 0.0,
                    ..Style::default()
                })
                .draw(Draw::Box(BoxDraw::fill(surface, radii(ring_radius)))),
            );
            let el: Element<Msg> = avatar.size(self.size).shape(self.shape).into();
            cx.child(el);
            cx.end();
        }
        if let Some(o) = self.overflow {
            let outer = side + RING * 2.0;
            let words = o.label.unwrap_or_else(|| format!("+{}", o.count));
            let mut style = type_style(t.font.supporting, FontFamily::Body);
            style.size = style.size.max(side * OVERFLOW_FONT_RATIO);
            style.weight = t.font.weight.medium;
            style.leading = 1.0;
            let radius = radii(shape_radius(self.shape, outer));
            let mut node = Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                justify_content: JustifyContent::CENTER,
                min_size: taffy::Size {
                    width: LengthPercentageAuto::length(outer),
                    height: LengthPercentageAuto::length(outer),
                },
                size: taffy::Size {
                    width: Dimension::auto(),
                    height: Dimension::length(outer),
                },
                padding: taffy::Rect {
                    left: len(t.space.s2),
                    right: len(t.space.s2),
                    top: len(0.0),
                    bottom: len(0.0),
                },
                margin: margin_start(-overlap),
                flex_shrink: 0.0,
                ..Style::default()
            })
            .draw(Draw::Box(BoxDraw {
                border: Some((surface, RING)),
                ..BoxDraw::fill(
                    crate::util::mix(surface, color(t.color.neutral), color(t.color.neutral).a),
                    radius,
                )
            }));
            let more = cx.t("@astryx.avatarGroup.overflow", &args! {"count" => o.count});
            if let Some(msg) = o.on_click {
                node = node
                    .interact(Interact::new(Some(msg), false))
                    .indicator(Indicator::ring(
                        color(t.color.accent),
                        radius,
                        Some(color(t.color.overlay_pressed)),
                    ))
                    .sem(Semantics::new(Role::Button, more));
            } else {
                node = node.sem(Semantics::new(Role::Text, more));
            }
            cx.begin(node);
            let mut text = text_node(words, style, color(t.color.text_secondary));
            text_draw(&mut text).max_lines = Some(1);
            text.sem = None;
            cx.leaf(text);
            cx.end();
        }
        cx.end();
    }
}
