//! Section and Card: grounds for content, on the spacing scale.

use super::stack::{padding_methods, sizing_methods, SizeValue, Sizing};
use super::Padding;
use crate::util::{color, component, len, radii};
use astryx_core::authoring::{Base, BoxDraw, Build, BuildCx, Draw, Node};
use astryx_core::contract::Color;
use astryx_core::{Element, Elevation, Space};
use taffy::{FlexDirection, LengthPercentageAuto, Style};

/// Section `variant`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SectionVariant {
    #[default]
    Section,
    Transparent,
    Muted,
}

/// An edge that draws a divider.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SectionDivider {
    Top,
    Bottom,
    Start,
    End,
}

/// A full-width band of content (`Section`). Inside a padded container
/// (Card, Layout), it reaches through the container's inline padding.
pub struct Section<Msg> {
    base: Base,
    variant: SectionVariant,
    sizing: Sizing,
    dividers: Vec<SectionDivider>,
    padding: Padding,
    children: Vec<Element<Msg>>,
}

impl<Msg> Default for Section<Msg> {
    fn default() -> Self {
        Section {
            base: Base::default(),
            variant: SectionVariant::Section,
            sizing: Sizing::default(),
            dividers: Vec::new(),
            padding: Padding::all(Space::S4),
            children: Vec::new(),
        }
    }
}

impl<Msg> Section<Msg> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn variant(mut self, variant: SectionVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn dividers(mut self, dividers: impl IntoIterator<Item = SectionDivider>) -> Self {
        self.dividers = dividers.into_iter().collect();
        self
    }
    pub fn child(mut self, child: impl Into<Element<Msg>>) -> Self {
        self.children.push(child.into());
        self
    }
    pub fn children<E: Into<Element<Msg>>>(
        mut self,
        children: impl IntoIterator<Item = E>,
    ) -> Self {
        self.children.extend(children.into_iter().map(Into::into));
        self
    }
    padding_methods!();
    sizing_methods!();
}

component!(Section<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for Section<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let ground: Color = match self.variant {
            SectionVariant::Section => color(c.background_surface),
            SectionVariant::Transparent => Color::TRANSPARENT,
            SectionVariant::Muted => color(c.background_muted),
        };
        let (is, ie, bs, be) = self.padding.resolve(t);
        let (pl, pr) = cx.inline(is, ie);
        let bleed = cx.container_padding.last().copied().unwrap_or(0.0);
        let has = |d| self.dividers.contains(&d);
        let (left_div, right_div) = cx.inline(has(SectionDivider::Start), has(SectionDivider::End));
        let w = t.size.border_width;
        let edge = |on: bool| if on { len(w) } else { len(0.0) };
        let mut style = Style {
            flex_direction: FlexDirection::Column,
            padding: taffy::Rect {
                left: len(pl),
                right: len(pr),
                top: len(bs),
                bottom: len(be),
            },
            border: taffy::Rect {
                left: edge(left_div),
                right: edge(right_div),
                top: edge(has(SectionDivider::Top)),
                bottom: edge(has(SectionDivider::Bottom)),
            },
            margin: taffy::Rect {
                left: LengthPercentageAuto::length(-bleed),
                right: LengthPercentageAuto::length(-bleed),
                top: LengthPercentageAuto::length(0.0),
                bottom: LengthPercentageAuto::length(0.0),
            },
            min_size: taffy::Size {
                width: LengthPercentageAuto::length(0.0),
                height: LengthPercentageAuto::auto(),
            },
            ..Style::default()
        };
        self.sizing.apply(&mut style);
        let any_divider = !self.dividers.is_empty();
        let mut node = Node::new(style).draw(Draw::Box(BoxDraw::fill(ground, radii(0.0))));
        cx.key(self.base.id.as_deref());
        let dividers = self.dividers.clone();
        cx.begin(std::mem::replace(&mut node, Node::new(Style::default())));
        cx.container_padding.push(pl.min(pr));
        for child in self.children {
            cx.child(child);
        }
        cx.container_padding.pop();
        if any_divider {
            // Dividers are drawn as 1 point rules on the marked edges.
            let border = color(c.border);
            for d in dividers {
                let (l, r) = cx.inline(SectionDivider::Start, SectionDivider::End);
                let mut inset = taffy::Rect {
                    left: LengthPercentageAuto::length(-w),
                    right: LengthPercentageAuto::length(-w),
                    top: LengthPercentageAuto::length(-w),
                    bottom: LengthPercentageAuto::length(-w),
                };
                let mut size = taffy::Size {
                    width: taffy::Dimension::auto(),
                    height: taffy::Dimension::auto(),
                };
                match d {
                    SectionDivider::Top => {
                        inset.bottom = LengthPercentageAuto::auto();
                        size.height = taffy::Dimension::length(w);
                    }
                    SectionDivider::Bottom => {
                        inset.top = LengthPercentageAuto::auto();
                        size.height = taffy::Dimension::length(w);
                    }
                    side => {
                        if side == l {
                            inset.right = LengthPercentageAuto::auto();
                        } else if side == r {
                            inset.left = LengthPercentageAuto::auto();
                        }
                        size.width = taffy::Dimension::length(w);
                    }
                }
                cx.leaf(
                    Node::new(Style {
                        position: taffy::Position::Absolute,
                        inset,
                        size,
                        ..Style::default()
                    })
                    .draw(Draw::Box(BoxDraw::fill(border, radii(0.0)))),
                );
            }
        }
        cx.end();
    }
}

/// Card `variant`.
pub use astryx_core::CardVariant;

/// One independent item on the card ground (`Card`). Children stack top to
/// bottom and are clipped to the card's corners.
pub struct Card<Msg> {
    base: Base,
    padding: Space,
    variant: CardVariant,
    elevation: Elevation,
    sizing: Sizing,
    children: Vec<Element<Msg>>,
}

impl<Msg> Default for Card<Msg> {
    fn default() -> Self {
        Card {
            base: Base::default(),
            padding: Space::S4,
            variant: CardVariant::Default,
            elevation: Elevation::None,
            sizing: Sizing::default(),
            children: Vec::new(),
        }
    }
}

impl<Msg> Card<Msg> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn padding(mut self, padding: Space) -> Self {
        self.padding = padding;
        self
    }
    pub fn variant(mut self, variant: CardVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn elevation(mut self, elevation: Elevation) -> Self {
        self.elevation = elevation;
        self
    }
    pub fn child(mut self, child: impl Into<Element<Msg>>) -> Self {
        self.children.push(child.into());
        self
    }
    pub fn children<E: Into<Element<Msg>>>(
        mut self,
        children: impl IntoIterator<Item = E>,
    ) -> Self {
        self.children.extend(children.into_iter().map(Into::into));
        self
    }
    sizing_methods!();
}

component!(Card<Msg>);

/// The card ground for `variant`, and whether it draws a border.
pub(crate) fn card_ground<Msg>(cx: &BuildCx<Msg>, variant: CardVariant) -> (Color, bool) {
    let c = &cx.tokens.color;
    let ground = match variant {
        CardVariant::Default => c.background_card,
        CardVariant::Transparent => [0.0; 4],
        CardVariant::Muted => c.background_muted,
        CardVariant::Blue => c.background_blue,
        CardVariant::Cyan => c.background_cyan,
        CardVariant::Gray => c.background_gray,
        CardVariant::Green => c.background_green,
        CardVariant::Orange => c.background_orange,
        CardVariant::Pink => c.background_pink,
        CardVariant::Purple => c.background_purple,
        CardVariant::Red => c.background_red,
        CardVariant::Teal => c.background_teal,
        CardVariant::Yellow => c.background_yellow,
    };
    (color(ground), variant == CardVariant::Default)
}

impl<Msg: Clone + 'static> Build<Msg> for Card<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let (ground, bordered) = card_ground(cx, self.variant);
        let border = if bordered { t.size.border_width } else { 0.0 };
        // The border sits inside the padding, so content keeps the step's
        // inset with or without a border.
        let inset = (self.padding.points(t) - border).max(0.0);
        let mut style = Style {
            flex_direction: FlexDirection::Column,
            padding: crate::util::rect(len(inset)),
            border: crate::util::rect(len(border)),
            min_size: taffy::Size {
                width: LengthPercentageAuto::length(0.0),
                height: LengthPercentageAuto::auto(),
            },
            ..Style::default()
        };
        self.sizing.apply(&mut style);
        let radius = radii(t.radius.container);
        let mut node = Node::new(style).draw(Draw::Box(BoxDraw {
            fill: ground,
            radii: radius,
            border: bordered.then(|| (color(t.color.border), border)),
            shadow: self.elevation_layers(t),
        }));
        node.clip = Some(radius);
        cx.key(self.base.id.as_deref());
        cx.begin(node);
        cx.container_padding.push(inset + border);
        for child in self.children {
            cx.child(child);
        }
        cx.container_padding.pop();
        cx.end();
    }
}

impl<Msg> Card<Msg> {
    fn elevation_layers(
        &self,
        t: &'static astryx_core::Tokens,
    ) -> &'static [astryx_tokens::ShadowLayer] {
        elevation_layers(self.elevation, t)
    }
}

pub(crate) fn elevation_layers(
    e: Elevation,
    t: &'static astryx_core::Tokens,
) -> &'static [astryx_tokens::ShadowLayer] {
    match e {
        Elevation::None => &[],
        Elevation::Low => t.shadow.low,
        Elevation::Med => t.shadow.med,
        Elevation::High => t.shadow.high,
    }
}

#[allow(dead_code)]
fn _size_value_is_used(_: SizeValue) {}
