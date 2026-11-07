//! Item and ListItem: a row with optional leading and trailing content, a
//! label, and a description.

use super::text::{text_draw, text_node, type_style};
use super::{len, len_auto, radii};
use crate::contract::{Color, FontFamily, Role};
use crate::element::{
    base_props, Base, BoxDraw, Build, BuildCx, Draw, Element, Indicator, Interact, Node, Semantics,
};
use taffy::{AlignItems, FlexDirection, JustifyContent, Style};

/// How the start and end content line up with the words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ItemAlign {
    #[default]
    Center,
    Start,
    /// Centered on the label's first line.
    FirstLine,
}

/// Item `density`: block padding of step 1, 2, or 3.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ItemDensity {
    Compact,
    #[default]
    Balanced,
    Spacious,
}

/// Item `layout`: the description under the label, or beside it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ItemLayout {
    #[default]
    Stacked,
    Inline,
}

struct ItemProps<Msg> {
    label: String,
    description: Option<String>,
    align: ItemAlign,
    density: ItemDensity,
    label_lines: u32,
    description_lines: u32,
    layout: ItemLayout,
    on_click: Option<Msg>,
    href: Option<String>,
    is_highlighted: bool,
    is_selected: bool,
    is_disabled: bool,
    start_content: Option<Element<Msg>>,
    end_content: Option<Element<Msg>>,
    marker: Option<Element<Msg>>,
}

impl<Msg> ItemProps<Msg> {
    fn new(label: String) -> Self {
        ItemProps {
            label,
            description: None,
            align: ItemAlign::default(),
            density: ItemDensity::default(),
            label_lines: 1,
            description_lines: 1,
            layout: ItemLayout::default(),
            on_click: None,
            href: None,
            is_highlighted: false,
            is_selected: false,
            is_disabled: false,
            start_content: None,
            end_content: None,
            marker: None,
        }
    }
}

/// A row (`Item`): marker and start content, a label over an optional
/// description, and end content. With `on_click` it is a focus stop.
pub struct Item<Msg> {
    base: Base,
    p: ItemProps<Msg>,
}

/// A row of a list (`ListItem`). Outside a List it draws as a balanced,
/// padded `Item`.
pub struct ListItem<Msg> {
    base: Base,
    p: ItemProps<Msg>,
}

macro_rules! row_props {
    () => {
        pub fn description(mut self, description: impl Into<String>) -> Self {
            self.p.description = Some(description.into());
            self
        }
        pub fn align(mut self, align: ItemAlign) -> Self {
            self.p.align = align;
            self
        }
        /// The message a press sends; the row becomes a focus stop.
        pub fn on_click(mut self, msg: Msg) -> Self {
            self.p.on_click = Some(msg);
            self
        }
        /// A link the row opens on a press, after `on_click`.
        pub fn href(mut self, href: impl Into<String>) -> Self {
            self.p.href = Some(href.into());
            self
        }
        /// Dims the words; a focusable row stays reachable and does nothing.
        pub fn is_disabled(mut self, on: bool) -> Self {
            self.p.is_disabled = on;
            self
        }
        pub fn is_selected(mut self, on: bool) -> Self {
            self.p.is_selected = on;
            self
        }
        /// Content before the words, such as an Icon or an Avatar.
        pub fn start_content(mut self, content: impl Into<Element<Msg>>) -> Self {
            self.p.start_content = Some(content.into());
            self
        }
        /// Content after the words, such as a Badge or a Switch.
        pub fn end_content(mut self, content: impl Into<Element<Msg>>) -> Self {
            self.p.end_content = Some(content.into());
            self
        }
    };
}

impl<Msg> Item<Msg> {
    pub fn new(label: impl Into<String>) -> Self {
        Item {
            base: Base::default(),
            p: ItemProps::new(label.into()),
        }
    }
    row_props!();
    pub fn density(mut self, density: ItemDensity) -> Self {
        self.p.density = density;
        self
    }
    /// Lines the label may take before an ellipsis; one by default, 0 for all.
    pub fn label_lines(mut self, lines: u32) -> Self {
        self.p.label_lines = lines;
        self
    }
    /// Lines the description may take before an ellipsis; one by default, 0 for all.
    pub fn description_lines(mut self, lines: u32) -> Self {
        self.p.description_lines = lines;
        self
    }
    pub fn layout(mut self, layout: ItemLayout) -> Self {
        self.p.layout = layout;
        self
    }
    /// The highlighted background, as for a pointer hover.
    pub fn is_highlighted(mut self, on: bool) -> Self {
        self.p.is_highlighted = on;
        self
    }
    /// A list marker before the start content.
    pub fn marker(mut self, marker: impl Into<Element<Msg>>) -> Self {
        self.p.marker = Some(marker.into());
        self
    }
}

impl<Msg> ListItem<Msg> {
    pub fn new(label: impl Into<String>) -> Self {
        ListItem {
            base: Base::default(),
            p: ItemProps::new(label.into()),
        }
    }
    row_props!();
}

base_props!(Item<Msg>);
base_props!(ListItem<Msg>);

impl<Msg> Build<Msg> for Item<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let s = *self;
        build_row(s.base, s.p, None, cx);
    }
}

impl<Msg> Build<Msg> for ListItem<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let s = *self;
        build_row(s.base, s.p, Some(Role::ListItem), cx);
    }
}

fn build_row<Msg>(base: Base, p: ItemProps<Msg>, static_role: Option<Role>, cx: &mut BuildCx<Msg>) {
    if base.is_hidden {
        return;
    }
    let t = cx.tokens;
    let c = &t.color;
    let block = match p.density {
        ItemDensity::Compact => t.space.s1,
        ItemDensity::Balanced => t.space.s2,
        ItemDensity::Spacious => t.space.s3,
    };
    let inline = if p.density == ItemDensity::Spacious {
        t.space.s3
    } else {
        t.space.s2
    };
    // Selected wins over highlighted, as Astryx orders them.
    let fill: Color = if p.is_selected {
        c.accent_muted.into()
    } else if p.is_highlighted {
        c.overlay_hover.into()
    } else {
        Color::TRANSPARENT
    };
    let radius = radii(t.radius.element);
    let body = type_style(t.font.body, FontFamily::Body);
    let first_line = (body.size * body.leading - 0.01).ceil();
    let mut node = Node::new(Style {
        flex_direction: FlexDirection::Row,
        align_items: match p.align {
            ItemAlign::Center => AlignItems::CENTER,
            ItemAlign::Start | ItemAlign::FirstLine => AlignItems::FLEX_START,
        },
        gap: taffy::Size {
            width: len(t.space.s2),
            height: len(0.0),
        },
        padding: taffy::Rect {
            left: len(inline),
            right: len(inline),
            top: len(block),
            bottom: len(block),
        },
        ..Style::default()
    })
    .draw(Draw::Box(BoxDraw::fill(fill, radius)));
    let href = p.href.as_deref().and_then(crate::element::safe_destination);
    let interactive = p.on_click.is_some() || href.is_some();
    let mut sem = if interactive {
        Some(Semantics::new(Role::Button, p.label.clone()))
    } else {
        static_role.map(|role| Semantics::new(role, p.label.clone()))
    };
    if let Some(sem) = &mut sem {
        sem.description = p.description.clone();
        sem.disabled = p.is_disabled;
        sem.selected = p.is_selected.then_some(true);
    }
    node.sem = sem;
    if interactive {
        node = node
            .interact(Interact::new(p.on_click, p.is_disabled).href(href.as_deref()))
            .indicator(Indicator::ring(
                c.accent.into(),
                radius,
                Some(c.overlay_pressed.into()),
            ));
    }
    cx.key(base.id.as_deref());
    cx.begin(node);

    let slot = |cx: &mut BuildCx<Msg>, content: Element<Msg>, dim: bool| {
        let mut style = Style {
            flex_direction: FlexDirection::Row,
            flex_shrink: 0.0,
            ..Style::default()
        };
        if p.align == ItemAlign::FirstLine {
            // The slot's center sits on the center of the label's first line.
            style.align_items = AlignItems::CENTER;
            style.min_size.height = len_auto(first_line);
        }
        cx.begin(Node::new(style).opacity(if dim { 0.5 } else { 1.0 }));
        cx.child(content);
        cx.end();
    };
    if let Some(marker) = p.marker {
        slot(cx, marker, false);
    }
    if let Some(start) = p.start_content {
        slot(cx, start, false);
    }

    let inline_layout = p.layout == ItemLayout::Inline && p.description.is_some();
    cx.begin(
        Node::new(Style {
            flex_direction: if inline_layout {
                FlexDirection::Row
            } else {
                FlexDirection::Column
            },
            align_items: if inline_layout {
                AlignItems::BASELINE
            } else {
                AlignItems::NORMAL
            },
            justify_content: JustifyContent::FLEX_START,
            gap: taffy::Size {
                width: len(if inline_layout { t.space.s1 } else { 0.0 }),
                height: len(0.0),
            },
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_size: taffy::Size {
                width: len_auto(0.0),
                height: taffy::LengthPercentageAuto::auto(),
            },
            ..Style::default()
        })
        .opacity(if p.is_disabled { 0.5 } else { 1.0 }),
    );
    let mut label = text_node(p.label, body, c.text_primary.into());
    text_draw(&mut label).max_lines = (p.label_lines > 0).then_some(p.label_lines);
    if inline_layout {
        // An inline label keeps its full width; the description truncates.
        label.style.flex_shrink = 0.0;
    }
    cx.leaf(label);
    if let Some(description) = p.description {
        let mut d = text_node(
            description,
            type_style(t.font.supporting, FontFamily::Body),
            c.text_secondary.into(),
        );
        text_draw(&mut d).max_lines = (p.description_lines > 0).then_some(p.description_lines);
        d.style.min_size.width = len_auto(0.0);
        d.style.flex_shrink = 1.0;
        cx.leaf(d);
    }
    cx.end();

    if let Some(end) = p.end_content {
        slot(cx, end, p.is_disabled);
    }
    cx.end();
}
