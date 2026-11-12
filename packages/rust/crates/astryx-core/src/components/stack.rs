//! VStack and HStack: flex containers on the spacing scale.

use super::into_elements;
use crate::element::{base_props, Axis, Base, Build, BuildCx, Element, Node, Space};
use taffy::{
    AlignContent, AlignItems, Dimension, FlexDirection, FlexWrap, JustifyContent, Overflow, Style,
};

/// Main-axis distribution: `hAlign` on HStack, `vAlign` on VStack, and `justify`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StackJustify {
    Start,
    Center,
    End,
    Between,
    Around,
    Evenly,
}

/// Cross-axis alignment: `vAlign` on HStack, `hAlign` on VStack, and `align`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StackAlign {
    Start,
    Center,
    End,
    Stretch,
}

/// Stack `wrap`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StackWrap {
    #[default]
    Nowrap,
    Wrap,
    WrapReverse,
}

/// A size for a stack's width or height, as Astryx's `SizeValue`: points,
/// or a percent of the parent's size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SizeValue {
    Points(f32),
    Percent(f32),
}

impl SizeValue {
    fn dimension(self) -> Dimension {
        match self {
            SizeValue::Points(v) => Dimension::length(v),
            SizeValue::Percent(v) => Dimension::percent(v / 100.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct StackProps {
    width: Option<SizeValue>,
    height: Option<SizeValue>,
    min_height: Option<SizeValue>,
    gap: Option<Space>,
    padding: Option<Space>,
    padding_inline: Option<Space>,
    padding_inline_start: Option<Space>,
    padding_inline_end: Option<Space>,
    padding_block: Option<Space>,
    padding_block_start: Option<Space>,
    padding_block_end: Option<Space>,
    is_scrollable: bool,
    main: Option<StackJustify>,
    cross: Option<StackAlign>,
    justify: Option<StackJustify>,
    align: Option<StackAlign>,
    wrap: StackWrap,
}

impl StackJustify {
    fn content(self) -> JustifyContent {
        match self {
            StackJustify::Start => JustifyContent::FLEX_START,
            StackJustify::Center => JustifyContent::CENTER,
            StackJustify::End => JustifyContent::FLEX_END,
            StackJustify::Between => JustifyContent::SPACE_BETWEEN,
            StackJustify::Around => JustifyContent::SPACE_AROUND,
            StackJustify::Evenly => JustifyContent::SPACE_EVENLY,
        }
    }
}

impl StackAlign {
    fn items(self) -> AlignItems {
        match self {
            StackAlign::Start => AlignItems::FLEX_START,
            StackAlign::Center => AlignItems::CENTER,
            StackAlign::End => AlignItems::FLEX_END,
            StackAlign::Stretch => AlignItems::STRETCH,
        }
    }
}

fn build_stack<Msg>(
    base: Base,
    p: StackProps,
    children: Vec<Element<Msg>>,
    direction: FlexDirection,
    cx: &mut BuildCx<Msg>,
) {
    if base.is_hidden {
        return;
    }
    let t = cx.tokens;
    let pts = |s: Option<Space>| s.map(|s| s.points(t));
    // Each edge takes its own prop, then its axis prop, then `padding`.
    let edge = |own: Option<Space>, axis: Option<Space>| {
        super::len(pts(own.or(axis).or(p.padding)).unwrap_or(0.0))
    };
    let gap = super::len(pts(p.gap).unwrap_or(0.0));
    let axis = if direction == FlexDirection::Row {
        Axis::Horizontal
    } else {
        Axis::Vertical
    };
    let scroll = if axis == Axis::Horizontal {
        taffy::Point {
            x: Overflow::Scroll,
            y: Overflow::Visible,
        }
    } else {
        taffy::Point {
            x: Overflow::Visible,
            y: Overflow::Scroll,
        }
    };
    // Inline start is the right edge in a right-to-left region.
    let (inline_left, inline_right) = cx.inline(p.padding_inline_start, p.padding_inline_end);
    let style = Style {
        flex_direction: direction,
        flex_wrap: match p.wrap {
            StackWrap::Nowrap => FlexWrap::NoWrap,
            StackWrap::Wrap => FlexWrap::Wrap,
            StackWrap::WrapReverse => FlexWrap::WrapReverse,
        },
        gap: taffy::Size {
            width: gap,
            height: gap,
        },
        padding: taffy::Rect {
            left: edge(inline_left, p.padding_inline),
            right: edge(inline_right, p.padding_inline),
            top: edge(p.padding_block_start, p.padding_block),
            bottom: edge(p.padding_block_end, p.padding_block),
        },
        // Unset alignment keeps the CSS defaults: start and stretch.
        justify_content: p
            .main
            .or(p.justify)
            .map_or(JustifyContent::NORMAL, StackJustify::content),
        align_items: p
            .cross
            .or(p.align)
            .map_or(AlignItems::NORMAL, StackAlign::items),
        align_content: if p.wrap == StackWrap::Nowrap {
            AlignContent::NORMAL
        } else {
            AlignContent::FLEX_START
        },
        size: taffy::Size {
            width: p.width.map_or(Dimension::auto(), SizeValue::dimension),
            height: p.height.map_or(Dimension::auto(), SizeValue::dimension),
        },
        min_size: taffy::Size {
            width: taffy::LengthPercentageAuto::auto(),
            height: p
                .min_height
                .map_or(taffy::LengthPercentageAuto::auto(), |v| match v {
                    SizeValue::Points(v) => taffy::LengthPercentageAuto::length(v),
                    SizeValue::Percent(v) => taffy::LengthPercentageAuto::percent(v / 100.0),
                }),
        },
        overflow: if p.is_scrollable {
            scroll
        } else {
            taffy::Point {
                x: Overflow::Visible,
                y: Overflow::Visible,
            }
        },
        ..Style::default()
    };
    let mut node = Node::new(style);
    if p.is_scrollable {
        node.scroll = Some(axis);
    }
    cx.key(base.id.as_deref());
    cx.begin(node);
    for child in children {
        cx.child(child);
    }
    cx.end();
}

macro_rules! stack {
    ($name:ident, $direction:expr, $doc:literal, h_align: $h:ty, v_align: $v:ty, $h_field:ident, $v_field:ident) => {
        #[doc = $doc]
        pub struct $name<Msg> {
            base: Base,
            props: StackProps,
            children: Vec<Element<Msg>>,
        }

        impl<Msg> Default for $name<Msg> {
            fn default() -> Self {
                Self {
                    base: Base::default(),
                    props: StackProps::default(),
                    children: Vec::new(),
                }
            }
        }

        impl<Msg> $name<Msg> {
            pub fn new() -> Self {
                Self::default()
            }
            /// A stack holding `children` in order.
            pub fn with_children<E: Into<Element<Msg>>>(
                children: impl IntoIterator<Item = E>,
            ) -> Self {
                Self {
                    children: into_elements(children),
                    ..Self::default()
                }
            }
            /// Adds one child after the others.
            pub fn child(mut self, child: impl Into<Element<Msg>>) -> Self {
                self.children.push(child.into());
                self
            }
            /// Adds children after the others.
            pub fn children<E: Into<Element<Msg>>>(
                mut self,
                children: impl IntoIterator<Item = E>,
            ) -> Self {
                self.children.extend(into_elements(children));
                self
            }
            /// Space between children.
            pub fn gap(mut self, gap: Space) -> Self {
                self.props.gap = Some(gap);
                self
            }
            pub fn padding(mut self, padding: Space) -> Self {
                self.props.padding = Some(padding);
                self
            }
            pub fn padding_inline(mut self, padding: Space) -> Self {
                self.props.padding_inline = Some(padding);
                self
            }
            pub fn padding_inline_start(mut self, padding: Space) -> Self {
                self.props.padding_inline_start = Some(padding);
                self
            }
            pub fn padding_inline_end(mut self, padding: Space) -> Self {
                self.props.padding_inline_end = Some(padding);
                self
            }
            pub fn padding_block(mut self, padding: Space) -> Self {
                self.props.padding_block = Some(padding);
                self
            }
            pub fn padding_block_start(mut self, padding: Space) -> Self {
                self.props.padding_block_start = Some(padding);
                self
            }
            pub fn padding_block_end(mut self, padding: Space) -> Self {
                self.props.padding_block_end = Some(padding);
                self
            }
            /// Scrolls along the stack's direction; focus scrolls its
            /// children into view.
            pub fn is_scrollable(mut self, on: bool) -> Self {
                self.props.is_scrollable = on;
                self
            }
            /// Horizontal alignment.
            pub fn h_align(mut self, align: $h) -> Self {
                self.props.$h_field = Some(align);
                self
            }
            /// Vertical alignment.
            pub fn v_align(mut self, align: $v) -> Self {
                self.props.$v_field = Some(align);
                self
            }
            /// Main-axis distribution, when the matching `h_align`/`v_align` is unset.
            pub fn justify(mut self, justify: StackJustify) -> Self {
                self.props.justify = Some(justify);
                self
            }
            /// Cross-axis alignment, when the matching `h_align`/`v_align` is unset.
            pub fn align(mut self, align: StackAlign) -> Self {
                self.props.align = Some(align);
                self
            }
            pub fn wrap(mut self, wrap: StackWrap) -> Self {
                self.props.wrap = wrap;
                self
            }
            /// Width of the stack. A percent is of the parent's width.
            pub fn width(mut self, width: SizeValue) -> Self {
                self.props.width = Some(width);
                self
            }
            /// Height of the stack. A percent is of the parent's height;
            /// the app's root is the window, so `Percent(100.0)` fills it.
            pub fn height(mut self, height: SizeValue) -> Self {
                self.props.height = Some(height);
                self
            }
            pub fn min_height(mut self, height: SizeValue) -> Self {
                self.props.min_height = Some(height);
                self
            }
        }

        base_props!($name<Msg>);

        impl<Msg> Build<Msg> for $name<Msg> {
            fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
                let s = *self;
                build_stack(s.base, s.props, s.children, $direction, cx);
            }
        }
    };
}

stack!(VStack, FlexDirection::Column, "Children top to bottom (`VStack`).", h_align: StackAlign, v_align: StackJustify, cross, main);
stack!(HStack, FlexDirection::Row, "Children start to end (`HStack`).", h_align: StackJustify, v_align: StackAlign, main, cross);

/// A [`VStack`] of the given children: `vstack![a, b, c]`.
#[macro_export]
macro_rules! vstack {
    ($($child:expr),* $(,)?) => {
        $crate::VStack::new()$(.child($child))*
    };
}

/// An [`HStack`] of the given children: `hstack![a, b, c]`.
#[macro_export]
macro_rules! hstack {
    ($($child:expr),* $(,)?) => {
        $crate::HStack::new()$(.child($child))*
    };
}

/// How a [`StackItem`] sizes along its stack, as Astryx's `StackItem`
/// `size`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StackItemSize {
    /// Its content's size; it never grows or shrinks.
    #[default]
    Static,
    /// Takes the space left in the stack, shared evenly with other `Fill`
    /// items.
    Fill,
}

/// One child of a VStack or HStack with its own sizing (`StackItem`). Its
/// content stacks top to bottom across its full width. It may shrink below
/// its content's size, so a `Fill` item with a scrollable stack inside
/// scrolls instead of growing the page.
pub struct StackItem<Msg> {
    base: Base,
    size: StackItemSize,
    children: Vec<Element<Msg>>,
}

impl<Msg> Default for StackItem<Msg> {
    fn default() -> Self {
        Self {
            base: Base::default(),
            size: StackItemSize::default(),
            children: Vec::new(),
        }
    }
}

impl<Msg> StackItem<Msg> {
    pub fn new() -> Self {
        Self::default()
    }
    /// An item holding `child`.
    pub fn with_child(child: impl Into<Element<Msg>>) -> Self {
        Self::new().child(child)
    }
    pub fn size(mut self, size: StackItemSize) -> Self {
        self.size = size;
        self
    }
    /// Adds one child after the others.
    pub fn child(mut self, child: impl Into<Element<Msg>>) -> Self {
        self.children.push(child.into());
        self
    }
    /// Adds children after the others.
    pub fn children<E: Into<Element<Msg>>>(
        mut self,
        children: impl IntoIterator<Item = E>,
    ) -> Self {
        self.children.extend(into_elements(children));
        self
    }
}

base_props!(StackItem<Msg>);

impl<Msg> Build<Msg> for StackItem<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let s = *self;
        if s.base.is_hidden {
            return;
        }
        let (grow, shrink) = match s.size {
            StackItemSize::Static => (0.0, 0.0),
            StackItemSize::Fill => (1.0, 1.0),
        };
        let style = Style {
            flex_direction: FlexDirection::Column,
            flex_grow: grow,
            flex_shrink: shrink,
            // Astryx resets the flex minimum so an item can be smaller
            // than its content.
            min_size: taffy::Size {
                width: taffy::LengthPercentageAuto::length(0.0),
                height: taffy::LengthPercentageAuto::length(0.0),
            },
            ..Style::default()
        };
        cx.key(s.base.id.as_deref());
        cx.begin(Node::new(style));
        for child in s.children {
            cx.child(child);
        }
        cx.end();
    }
}
