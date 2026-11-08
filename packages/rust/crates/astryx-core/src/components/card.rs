//! Card.

use super::{into_elements, len, radii, Elevation};
use crate::contract::{Color, Role};
use crate::element::{
    base_props, Base, BoxDraw, Build, BuildCx, Draw, Element, Indicator, Interact, Node, Semantics,
    Space,
};
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
    width: Option<f32>,
    children: Vec<Element<Msg>>,
}

impl<Msg> Default for Card<Msg> {
    fn default() -> Self {
        Card {
            base: Base::default(),
            padding: Space::S4,
            variant: CardVariant::default(),
            elevation: Elevation::None,
            width: None,
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
    /// The card's width in points.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
}

base_props!(Card<Msg>);

impl<Msg> Build<Msg> for Card<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let (mut node, inset) = card_node(cx, self.padding, self.variant, self.elevation, None);
        if let Some(w) = self.width {
            node.style.size.width = taffy::Dimension::length(w);
            node.style.flex_shrink = 0.0;
        }
        cx.key(self.base.id.as_deref());
        cx.begin(node);
        // Full-bleed children reach through the padding the card applies.
        cx.container_padding.push(inset);
        for child in self.children {
            cx.child(child);
        }
        cx.container_padding.pop();
        cx.end();
    }
}

/// A card's ground node, and the inset its full-bleed children reach
/// through. With `ring`, the border takes the selection ring's color.
fn card_node<Msg>(
    cx: &BuildCx<Msg>,
    padding: Space,
    variant: CardVariant,
    elevation: Elevation,
    ring: Option<Color>,
) -> (Node<Msg>, f32) {
    let t = cx.tokens;
    let c = &t.color;
    let ground: Color = match variant {
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
    let bordered = variant == CardVariant::Default;
    let border = if bordered { t.size.border_width } else { 0.0 };
    // The border sits inside the padding, so the content inset is the
    // padding step with or without a border.
    let inset = (padding.points(t) - border).max(0.0);
    let style = Style {
        flex_direction: FlexDirection::Column,
        padding: taffy::Rect {
            left: len(inset),
            right: len(inset),
            top: len(inset),
            bottom: len(inset),
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
        border: bordered.then(|| (ring.unwrap_or(c.border.into()), border)),
        shadow: elevation.layers(t),
    }));
    node.clip = Some(radius);
    (node, inset)
}

/// A card that toggles between selected and unselected, with an accent ring
/// when selected (`SelectableCard`). It is a checkbox for assistive
/// technology, named by `label`; for a single choice among cards, use
/// `lab::SelectableCardGroup`. For navigation, use a Button or ListItem.
pub struct SelectableCard<Msg> {
    base: Base,
    label: String,
    pub(crate) is_selected: bool,
    on_change: Option<std::rc::Rc<dyn Fn(bool) -> Msg>>,
    is_disabled: bool,
    padding: Space,
    variant: CardVariant,
    elevation: Elevation,
    children: Vec<Element<Msg>>,
    /// Inside a single-choice group: the message choosing this card.
    pub(crate) choice: Option<Msg>,
}

impl<Msg> SelectableCard<Msg> {
    pub fn new(label: impl Into<String>, is_selected: bool) -> Self {
        SelectableCard {
            base: Base::default(),
            label: label.into(),
            is_selected,
            on_change: None,
            is_disabled: false,
            padding: Space::S4,
            variant: CardVariant::default(),
            elevation: Elevation::None,
            children: Vec::new(),
            choice: None,
        }
    }
    /// Called with the new selection when the card is toggled.
    pub fn on_change(mut self, on_change: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_change = Some(std::rc::Rc::new(on_change));
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
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
}

base_props!(SelectableCard<Msg>);

impl<Msg> Build<Msg> for SelectableCard<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        // The built-in grounds ring in the accent; each palette variant in
        // its own border color.
        let ring: Color = match self.variant {
            CardVariant::Default | CardVariant::Transparent | CardVariant::Muted => c.accent,
            CardVariant::Blue => c.border_blue,
            CardVariant::Cyan => c.border_cyan,
            CardVariant::Gray => c.border_gray,
            CardVariant::Green => c.border_green,
            CardVariant::Orange => c.border_orange,
            CardVariant::Pink => c.border_pink,
            CardVariant::Purple => c.border_purple,
            CardVariant::Red => c.border_red,
            CardVariant::Teal => c.border_teal,
            CardVariant::Yellow => c.border_yellow,
        }
        .into();
        let selected = self.is_selected;
        let (node, inset) = card_node(
            cx,
            self.padding,
            self.variant,
            self.elevation,
            selected.then_some(ring),
        );
        let radius = radii(t.radius.container);
        let in_group = self.choice.is_some();
        let msg = match self.choice {
            Some(choice) => Some(choice),
            None => self.on_change.as_ref().map(|f| f(!selected)),
        };
        let mut sem = Semantics::new(
            if in_group {
                Role::ListBoxOption
            } else {
                Role::Checkbox
            },
            self.label,
        );
        if in_group {
            sem.selected = Some(selected);
        } else {
            sem.checked = Some(selected);
        }
        sem.disabled = self.is_disabled;
        let mut node = node
            .interact(Interact::new(msg, self.is_disabled))
            .indicator(Indicator::ring(
                c.accent.into(),
                radius,
                Some(c.overlay_pressed.into()),
            ))
            .sem(sem)
            .opacity(if self.is_disabled { 0.5 } else { 1.0 });
        node.group_entry = in_group && selected;
        cx.key(self.base.id.as_deref());
        cx.begin(node);
        cx.container_padding.push(inset);
        for child in self.children {
            cx.child(child);
        }
        cx.container_padding.pop();
        // The selection ring is inset and drawn over the content, so the
        // layout does not move when the selection changes.
        if selected {
            // Absolute children sit inside the border; the ring covers it.
            let edge = LengthPercentageAuto::length(if self.variant == CardVariant::Default {
                -t.size.border_width
            } else {
                0.0
            });
            cx.leaf(
                Node::new(Style {
                    position: taffy::Position::Absolute,
                    inset: taffy::Rect {
                        left: edge,
                        right: edge,
                        top: edge,
                        bottom: edge,
                    },
                    ..Style::default()
                })
                .draw(Draw::Box(BoxDraw {
                    fill: Color::TRANSPARENT,
                    radii: radius,
                    border: Some((ring, 2.0)),
                    shadow: &[],
                })),
            );
        }
        cx.end();
    }
}
