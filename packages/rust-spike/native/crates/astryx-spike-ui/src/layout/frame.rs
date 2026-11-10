//! Layout: a page frame with a header, a footer, side panels and content.
//! Each slot pads its content by the layout's padding (step 4 by default);
//! a header or footer without a divider collapses the space it shares with
//! the content, so the regions read as one surface.

use super::stack::{size_auto, size_dim, SizeValue};
use crate::util::{color, component, len, radii};
use astryx_core::authoring::{Axis, Base, BoxDraw, Build, BuildCx, Draw, Node, Semantics};
use astryx_core::contract::Role;
use astryx_core::{Element, Space};
use taffy::{Dimension, FlexDirection, LengthPercentageAuto, Overflow, Style};

/// Layout `height`: fill the parent, or grow with the content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LayoutHeight {
    #[default]
    Fill,
    Auto,
}

/// A header or footer slot.
pub struct LayoutBar<Msg> {
    children: Vec<Element<Msg>>,
    has_divider: Option<bool>,
    height: Option<SizeValue>,
    label: Option<String>,
    padding: Option<Space>,
    padding_block_end: Option<Space>,
}

pub type LayoutHeader<Msg> = LayoutBar<Msg>;
pub type LayoutFooter<Msg> = LayoutBar<Msg>;

impl<Msg> Default for LayoutBar<Msg> {
    fn default() -> Self {
        LayoutBar {
            children: Vec::new(),
            has_divider: None,
            height: None,
            label: None,
            padding: None,
            padding_block_end: None,
        }
    }
}

impl<Msg> LayoutBar<Msg> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn child(mut self, child: impl Into<Element<Msg>>) -> Self {
        self.children.push(child.into());
        self
    }
    pub fn has_divider(mut self, on: bool) -> Self {
        self.has_divider = Some(on);
        self
    }
    pub fn height(mut self, height: SizeValue) -> Self {
        self.height = Some(height);
        self
    }
    /// The region's accessible name.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    pub fn padding(mut self, step: Space) -> Self {
        self.padding = Some(step);
        self
    }
    pub fn padding_block_end(mut self, step: Space) -> Self {
        self.padding_block_end = Some(step);
        self
    }
}

/// The main region (`LayoutContent`). It scrolls by default.
pub struct LayoutContent<Msg> {
    children: Vec<Element<Msg>>,
    padding: Option<Space>,
    is_scrollable: bool,
    label: Option<String>,
}

impl<Msg> Default for LayoutContent<Msg> {
    fn default() -> Self {
        LayoutContent {
            children: Vec::new(),
            padding: None,
            is_scrollable: true,
            label: None,
        }
    }
}

impl<Msg> LayoutContent<Msg> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn child(mut self, child: impl Into<Element<Msg>>) -> Self {
        self.children.push(child.into());
        self
    }
    pub fn padding(mut self, step: Space) -> Self {
        self.padding = Some(step);
        self
    }
    pub fn is_scrollable(mut self, on: bool) -> Self {
        self.is_scrollable = on;
        self
    }
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// A side panel (`LayoutPanel`), at the start or end of the content.
pub struct LayoutPanel<Msg> {
    children: Vec<Element<Msg>>,
    has_divider: bool,
    padding: Option<Space>,
    is_scrollable: bool,
    label: Option<String>,
    width: Option<f32>,
    role: Role,
}

impl<Msg> Default for LayoutPanel<Msg> {
    fn default() -> Self {
        LayoutPanel {
            children: Vec::new(),
            has_divider: false,
            padding: None,
            is_scrollable: true,
            label: None,
            width: None,
            role: Role::Complementary,
        }
    }
}

impl<Msg> LayoutPanel<Msg> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn child(mut self, child: impl Into<Element<Msg>>) -> Self {
        self.children.push(child.into());
        self
    }
    pub fn has_divider(mut self, on: bool) -> Self {
        self.has_divider = on;
        self
    }
    pub fn padding(mut self, step: Space) -> Self {
        self.padding = Some(step);
        self
    }
    pub fn is_scrollable(mut self, on: bool) -> Self {
        self.is_scrollable = on;
        self
    }
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    /// Width in points.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
    /// The panel's landmark: `Complementary` (default) or `Navigation`.
    pub fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }
}

/// A page frame (`Layout`).
pub struct Layout<Msg> {
    base: Base,
    header: Option<LayoutHeader<Msg>>,
    footer: Option<LayoutFooter<Msg>>,
    start: Option<LayoutPanel<Msg>>,
    end: Option<LayoutPanel<Msg>>,
    content: Option<LayoutContent<Msg>>,
    height: LayoutHeight,
    content_width: Option<SizeValue>,
    padding: Space,
    default_has_dividers: bool,
}

impl<Msg> Default for Layout<Msg> {
    fn default() -> Self {
        Layout {
            base: Base::default(),
            header: None,
            footer: None,
            start: None,
            end: None,
            content: None,
            height: LayoutHeight::Fill,
            content_width: None,
            padding: Space::S4,
            default_has_dividers: false,
        }
    }
}

impl<Msg> Layout<Msg> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn header(mut self, header: LayoutHeader<Msg>) -> Self {
        self.header = Some(header);
        self
    }
    pub fn footer(mut self, footer: LayoutFooter<Msg>) -> Self {
        self.footer = Some(footer);
        self
    }
    pub fn start(mut self, panel: LayoutPanel<Msg>) -> Self {
        self.start = Some(panel);
        self
    }
    pub fn end(mut self, panel: LayoutPanel<Msg>) -> Self {
        self.end = Some(panel);
        self
    }
    pub fn content(mut self, content: LayoutContent<Msg>) -> Self {
        self.content = Some(content);
        self
    }
    pub fn height(mut self, height: LayoutHeight) -> Self {
        self.height = height;
        self
    }
    /// The widest the aligned content of each slot grows; dividers stay
    /// full width.
    pub fn content_width(mut self, width: SizeValue) -> Self {
        self.content_width = Some(width);
        self
    }
    pub fn padding(mut self, step: Space) -> Self {
        self.padding = step;
        self
    }
    /// Headers and footers that do not set `has_divider` take this.
    pub fn default_has_dividers(mut self, on: bool) -> Self {
        self.default_has_dividers = on;
        self
    }
}

component!(Layout<Msg>);

fn border_box<Msg>(cx: &BuildCx<Msg>) -> Draw {
    Draw::Box(BoxDraw::fill(
        astryx_core::contract::Color::TRANSPARENT,
        radii(0.0),
    ))
    .with_border(color(cx.tokens.color.border), cx.tokens.size.border_width)
}

trait WithBorder {
    fn with_border(self, color: astryx_core::contract::Color, width: f32) -> Self;
}

impl WithBorder for Draw {
    fn with_border(self, c: astryx_core::contract::Color, width: f32) -> Self {
        match self {
            Draw::Box(b) => Draw::Box(BoxDraw {
                border: Some((c, width)),
                ..b
            }),
            other => other,
        }
    }
}

/// A 1 point rule on one edge of the open node, drawn absolutely.
fn rule<Msg>(cx: &mut BuildCx<Msg>, edge: u8) {
    let w = cx.tokens.size.border_width;
    let auto = LengthPercentageAuto::auto();
    let zero = LengthPercentageAuto::length(0.0);
    let (inset, size) = match edge {
        // 0 top, 1 bottom, 2 left, 3 right
        0 => (
            taffy::Rect {
                left: zero,
                right: zero,
                top: zero,
                bottom: auto,
            },
            taffy::Size {
                width: Dimension::auto(),
                height: Dimension::length(w),
            },
        ),
        1 => (
            taffy::Rect {
                left: zero,
                right: zero,
                top: auto,
                bottom: zero,
            },
            taffy::Size {
                width: Dimension::auto(),
                height: Dimension::length(w),
            },
        ),
        2 => (
            taffy::Rect {
                left: zero,
                right: auto,
                top: zero,
                bottom: zero,
            },
            taffy::Size {
                width: Dimension::length(w),
                height: Dimension::auto(),
            },
        ),
        _ => (
            taffy::Rect {
                left: auto,
                right: zero,
                top: zero,
                bottom: zero,
            },
            taffy::Size {
                width: Dimension::length(w),
                height: Dimension::auto(),
            },
        ),
    };
    let ink = color(cx.tokens.color.border);
    cx.leaf(
        Node::new(Style {
            position: taffy::Position::Absolute,
            inset,
            size,
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw::fill(ink, radii(0.0)))),
    );
}

impl<Msg: Clone + 'static> Build<Msg> for Layout<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let _ = border_box::<Msg>;
        let t = cx.tokens;
        let p = self.padding.points(t);
        let bleed = cx.container_padding.last().copied().unwrap_or(0.0);
        let header_divider = self
            .header
            .as_ref()
            .map(|h| h.has_divider.unwrap_or(self.default_has_dividers));
        let footer_divider = self
            .footer
            .as_ref()
            .map(|f| f.has_divider.unwrap_or(self.default_has_dividers));
        let max_content = self.content_width.map(size_auto);
        let neg = LengthPercentageAuto::length(-bleed);
        let mut root = Style {
            flex_direction: FlexDirection::Column,
            margin: taffy::Rect {
                left: neg,
                right: neg,
                top: neg,
                bottom: neg,
            },
            min_size: taffy::Size {
                width: LengthPercentageAuto::length(0.0),
                height: LengthPercentageAuto::length(0.0),
            },
            ..Style::default()
        };
        if self.height == LayoutHeight::Fill {
            root.flex_grow = 1.0;
            root.size.height = Dimension::percent(1.0);
        }
        cx.key(self.base.id.as_deref());
        cx.begin(Node::new(root));
        cx.container_padding.push(0.0);
        // An aligned inner box: full width, capped at the content width.
        let inner = |pad: taffy::Rect<taffy::LengthPercentage>| Style {
            flex_direction: FlexDirection::Column,
            padding: pad,
            max_size: taffy::Size {
                width: max_content.unwrap_or(LengthPercentageAuto::auto()),
                height: LengthPercentageAuto::auto(),
            },
            margin: taffy::Rect {
                left: LengthPercentageAuto::auto(),
                right: LengthPercentageAuto::auto(),
                top: LengthPercentageAuto::length(0.0),
                bottom: LengthPercentageAuto::length(0.0),
            },
            size: taffy::Size {
                width: Dimension::percent(1.0),
                height: Dimension::auto(),
            },
            ..Style::default()
        };
        if let Some(h) = self.header {
            let hp = h.padding.map_or(p, |s| s.points(t));
            let end = h.padding_block_end.map_or(hp, |s| s.points(t));
            let mut s = Style {
                flex_shrink: 0.0,
                ..Style::default()
            };
            if let Some(height) = h.height {
                s.size.height = size_dim(height);
            }
            let mut node = Node::new(s);
            node.sem = Some(match h.label {
                Some(l) => Semantics::new(Role::Banner, l),
                None => Semantics::role(Role::Banner),
            });
            cx.begin(node);
            cx.begin(Node::new(inner(taffy::Rect {
                left: len(hp),
                right: len(hp),
                top: len(hp),
                bottom: len(end),
            })));
            cx.container_padding.push(hp);
            for c in h.children {
                cx.child(c);
            }
            cx.container_padding.pop();
            cx.end();
            if header_divider == Some(true) {
                rule(cx, 1);
            }
            cx.end();
        }
        // The middle row: panels and content.
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Row,
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_size: taffy::Size {
                width: LengthPercentageAuto::length(0.0),
                height: LengthPercentageAuto::length(0.0),
            },
            ..Style::default()
        }));
        let collapse_top = header_divider == Some(false);
        let collapse_bottom = footer_divider == Some(false);
        let (left_panel, right_panel, left_is_start) = if cx.rtl() {
            (self.end, self.start, false)
        } else {
            (self.start, self.end, true)
        };
        let panel = |cx: &mut BuildCx<Msg>, panel: LayoutPanel<Msg>, on_left: bool| {
            let pp = panel.padding.map_or(p, |s| s.points(cx.tokens));
            let mut style = Style {
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                padding: taffy::Rect {
                    left: len(pp),
                    right: len(pp),
                    top: len(if collapse_top { 0.0 } else { pp }),
                    bottom: len(if collapse_bottom { 0.0 } else { pp }),
                },
                min_size: taffy::Size {
                    width: LengthPercentageAuto::length(0.0),
                    height: LengthPercentageAuto::length(0.0),
                },
                ..Style::default()
            };
            if let Some(w) = panel.width {
                style.size.width = Dimension::length(w);
            }
            if panel.is_scrollable {
                style.overflow = taffy::Point {
                    x: Overflow::Visible,
                    y: Overflow::Scroll,
                };
            }
            let mut node = Node::new(style);
            if panel.is_scrollable {
                node.scroll = Some(Axis::Vertical);
            }
            node.sem = Some(match panel.label.clone() {
                Some(l) => Semantics::new(panel.role, l),
                None => Semantics::role(panel.role),
            });
            cx.begin(node);
            cx.container_padding.push(pp);
            for c in panel.children {
                cx.child(c);
            }
            cx.container_padding.pop();
            if panel.has_divider {
                rule(cx, if on_left { 3 } else { 2 });
            }
            cx.end();
        };
        let _ = left_is_start;
        if let Some(lp) = left_panel {
            panel(cx, lp, true);
        }
        let content = self.content.unwrap_or_default();
        let cp = content.padding.map_or(p, |s| s.points(t));
        let mut cs = Style {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_size: taffy::Size {
                width: LengthPercentageAuto::length(0.0),
                height: LengthPercentageAuto::length(0.0),
            },
            ..Style::default()
        };
        if content.is_scrollable {
            cs.overflow = taffy::Point {
                x: Overflow::Visible,
                y: Overflow::Scroll,
            };
        }
        let mut cnode = Node::new(cs);
        if content.is_scrollable {
            cnode.scroll = Some(Axis::Vertical);
        }
        cnode.sem = Some(match content.label {
            Some(l) => Semantics::new(Role::Main, l),
            None => Semantics::role(Role::Main),
        });
        cx.begin(cnode);
        cx.begin(Node::new(inner(taffy::Rect {
            left: len(cp),
            right: len(cp),
            top: len(if collapse_top { 0.0 } else { cp }),
            bottom: len(if collapse_bottom { 0.0 } else { cp }),
        })));
        cx.container_padding.push(cp);
        for c in content.children {
            cx.child(c);
        }
        cx.container_padding.pop();
        cx.end();
        cx.end();
        if let Some(rp) = right_panel {
            panel(cx, rp, false);
        }
        cx.end();
        if let Some(f) = self.footer {
            let fp = f.padding.map_or(p, |s| s.points(t));
            let mut s = Style {
                flex_shrink: 0.0,
                ..Style::default()
            };
            if let Some(height) = f.height {
                s.size.height = size_dim(height);
            }
            let mut node = Node::new(s);
            node.sem = Some(match f.label {
                Some(l) => Semantics::new(Role::ContentInfo, l),
                None => Semantics::role(Role::ContentInfo),
            });
            cx.begin(node);
            cx.begin(Node::new(inner(crate::util::rect(len(fp)))));
            cx.container_padding.push(fp);
            for c in f.children {
                cx.child(c);
            }
            cx.container_padding.pop();
            cx.end();
            if footer_divider == Some(true) {
                rule(cx, 0);
            }
            cx.end();
        }
        cx.container_padding.pop();
        cx.end();
    }
}
