//! Card.

use super::{into_elements, len, radii, Elevation};
use crate::contract::Color;
use crate::element::{base_props, Base, BoxDraw, Build, BuildCx, Draw, Element, Node, Space};
use taffy::{FlexDirection, LengthPercentageAuto, Style};

/// Card `variant`: the card ground. Only `Default` draws a border.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CardVariant {
    #[default]
    Default,
    Transparent,
    Muted,
    Blue,
    Cyan,
    Gray,
    Green,
    Orange,
    Pink,
    Purple,
    Red,
    Teal,
    Yellow,
}

/// One independent item on the card ground (`Card`). Children stack top to
/// bottom and are clipped to the card's corners.
pub struct Card<Msg> {
    base: Base,
    padding: Space,
    variant: CardVariant,
    elevation: Elevation,
    children: Vec<Element<Msg>>,
}

impl<Msg> Default for Card<Msg> {
    fn default() -> Self {
        Card {
            base: Base::default(),
            padding: Space::S4,
            variant: CardVariant::default(),
            elevation: Elevation::None,
            children: Vec::new(),
        }
    }
}

impl<Msg> Card<Msg> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_children<E: Into<Element<Msg>>>(children: impl IntoIterator<Item = E>) -> Self {
        Card {
            children: into_elements(children),
            ..Self::default()
        }
    }
    pub fn child(mut self, child: impl Into<Element<Msg>>) -> Self {
        self.children.push(child.into());
        self
    }
    pub fn children<E: Into<Element<Msg>>>(
        mut self,
        children: impl IntoIterator<Item = E>,
    ) -> Self {
        self.children.extend(into_elements(children));
        self
    }
    /// A step on the spacing scale, on every edge; step 4 when unset.
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
}

base_props!(Card<Msg>);

impl<Msg> Build<Msg> for Card<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let ground: Color = match self.variant {
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
        }
        .into();
        let bordered = self.variant == CardVariant::Default;
        let border = if bordered { t.size.border_width } else { 0.0 };
        // The border sits inside the padding, so the content inset is the
        // padding step with or without a border.
        let inset = len((self.padding.points(t) - border).max(0.0));
        let style = Style {
            flex_direction: FlexDirection::Column,
            padding: taffy::Rect {
                left: inset,
                right: inset,
                top: inset,
                bottom: inset,
            },
            border: taffy::Rect {
                left: len(border),
                right: len(border),
                top: len(border),
                bottom: len(border),
            },
            min_size: taffy::Size {
                width: LengthPercentageAuto::length(0.0),
                height: LengthPercentageAuto::auto(),
            },
            ..Style::default()
        };
        let radius = radii(t.radius.container);
        let mut node = Node::new(style).draw(Draw::Box(BoxDraw {
            fill: ground,
            radii: radius,
            border: bordered.then(|| (c.border.into(), border)),
            shadow: self.elevation.layers(t),
        }));
        node.clip = Some(radius);
        cx.key(self.base.id.as_deref());
        cx.begin(node);
        // Full-bleed children reach through the padding the card applies.
        cx.container_padding
            .push((self.padding.points(t) - border).max(0.0));
        for child in self.children {
            cx.child(child);
        }
        cx.container_padding.pop();
        cx.end();
    }
}
