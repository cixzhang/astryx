//! Stack, HStack and VStack: flex rows and columns on the spacing scale.

use super::{pad_rect, Padding};
use crate::util::{component, len};
use astryx_core::authoring::{Base, Build, BuildCx, Node};
use astryx_core::{Element, Space};
pub use astryx_core::{SizeValue, StackAlign, StackJustify, StackWrap};
use taffy::{
    AlignContent, AlignItems, Dimension, FlexDirection, FlexWrap, LengthPercentageAuto, Overflow,
    Style,
};

/// Stack `direction`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StackDirection {
    Horizontal,
    #[default]
    Vertical,
}

/// Main- or cross-axis alignment, as Astryx's `hAlign` and `vAlign` take:
/// the distribution values only apply on the main axis, and `Stretch` only
/// on the cross axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StackAxisAlign {
    Start,
    Center,
    End,
    Between,
    Around,
    Evenly,
    Stretch,
}

pub(crate) fn size_dim(v: SizeValue) -> Dimension {
    match v {
        SizeValue::Points(p) => Dimension::length(p),
        SizeValue::Percent(p) => Dimension::percent(p / 100.0),
    }
}

pub(crate) fn size_auto(v: SizeValue) -> LengthPercentageAuto {
    match v {
        SizeValue::Points(p) => LengthPercentageAuto::length(p),
        SizeValue::Percent(p) => LengthPercentageAuto::percent(p / 100.0),
    }
}

/// Sizing props shared by the layout containers.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sizing {
    pub width: Option<SizeValue>,
    pub height: Option<SizeValue>,
    pub max_width: Option<SizeValue>,
    pub min_height: Option<SizeValue>,
}

impl Sizing {
    pub(crate) fn apply(&self, style: &mut Style) {
        if let Some(w) = self.width {
            style.size.width = size_dim(w);
        }
        if let Some(h) = self.height {
            style.size.height = size_dim(h);
        }
        if let Some(w) = self.max_width {
            style.max_size.width = size_auto(w);
        }
        if let Some(h) = self.min_height {
            style.min_size.height = size_auto(h);
        }
    }
}

macro_rules! sizing_methods {
    () => {
        pub fn width(mut self, width: SizeValue) -> Self {
            self.sizing.width = Some(width);
            self
        }
        pub fn height(mut self, height: SizeValue) -> Self {
            self.sizing.height = Some(height);
            self
        }
        pub fn max_width(mut self, width: SizeValue) -> Self {
            self.sizing.max_width = Some(width);
            self
        }
        pub fn min_height(mut self, height: SizeValue) -> Self {
            self.sizing.min_height = Some(height);
            self
        }
    };
}
pub(crate) use sizing_methods;

macro_rules! padding_methods {
    () => {
        pub fn padding(mut self, step: Space) -> Self {
            self.padding.all = Some(step);
            self
        }
        pub fn padding_inline(mut self, step: Space) -> Self {
            self.padding.inline = Some(step);
            self
        }
        pub fn padding_inline_start(mut self, step: Space) -> Self {
            self.padding.inline_start = Some(step);
            self
        }
        pub fn padding_inline_end(mut self, step: Space) -> Self {
            self.padding.inline_end = Some(step);
            self
        }
        pub fn padding_block(mut self, step: Space) -> Self {
            self.padding.block = Some(step);
            self
        }
        pub fn padding_block_start(mut self, step: Space) -> Self {
            self.padding.block_start = Some(step);
            self
        }
        pub fn padding_block_end(mut self, step: Space) -> Self {
            self.padding.block_end = Some(step);
            self
        }
    };
}
pub(crate) use padding_methods;

/// A flex row or column (`Stack`). [`HStack`] and [`VStack`] fix the
/// direction.
pub struct Stack<Msg> {
    base: Base,
    direction: StackDirection,
    gap: Option<Space>,
    padding: Padding,
    sizing: Sizing,
    is_scrollable: bool,
    h_align: Option<StackAxisAlign>,
    v_align: Option<StackAxisAlign>,
    justify: Option<StackJustify>,
    align: Option<StackAlign>,
    wrap: StackWrap,
    children: Vec<Element<Msg>>,
}

impl<Msg> Default for Stack<Msg> {
    fn default() -> Self {
        Stack {
            base: Base::default(),
            direction: StackDirection::Vertical,
            gap: None,
            padding: Padding::default(),
            sizing: Sizing::default(),
            is_scrollable: false,
            h_align: None,
            v_align: None,
            justify: None,
            align: None,
            wrap: StackWrap::Nowrap,
            children: Vec::new(),
        }
    }
}

impl<Msg> Stack<Msg> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn direction(mut self, direction: StackDirection) -> Self {
        self.direction = direction;
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
    pub fn gap(mut self, gap: Space) -> Self {
        self.gap = Some(gap);
        self
    }
    /// Scrolls its content on the main axis when it overflows.
    pub fn is_scrollable(mut self, on: bool) -> Self {
        self.is_scrollable = on;
        self
    }
    /// Horizontal alignment: main axis in a row, cross axis in a column.
    pub fn h_align(mut self, align: StackAxisAlign) -> Self {
        self.h_align = Some(align);
        self
    }
    /// Vertical alignment: cross axis in a row, main axis in a column.
    pub fn v_align(mut self, align: StackAxisAlign) -> Self {
        self.v_align = Some(align);
        self
    }
    pub fn justify(mut self, justify: StackJustify) -> Self {
        self.justify = Some(justify);
        self
    }
    pub fn align(mut self, align: StackAlign) -> Self {
        self.align = Some(align);
        self
    }
    pub fn wrap(mut self, wrap: StackWrap) -> Self {
        self.wrap = wrap;
        self
    }
    padding_methods!();
    sizing_methods!();
}

component!(Stack<Msg>);

fn main_axis(a: StackAxisAlign) -> Option<AlignContent> {
    Some(match a {
        StackAxisAlign::Start | StackAxisAlign::Stretch => AlignContent::FLEX_START,
        StackAxisAlign::Center => AlignContent::CENTER,
        StackAxisAlign::End => AlignContent::FLEX_END,
        StackAxisAlign::Between => AlignContent::SPACE_BETWEEN,
        StackAxisAlign::Around => AlignContent::SPACE_AROUND,
        StackAxisAlign::Evenly => AlignContent::SPACE_EVENLY,
    })
}

fn justify_axis(j: StackJustify) -> Option<AlignContent> {
    main_axis(match j {
        StackJustify::Start => StackAxisAlign::Start,
        StackJustify::Center => StackAxisAlign::Center,
        StackJustify::End => StackAxisAlign::End,
        StackJustify::Between => StackAxisAlign::Between,
        StackJustify::Around => StackAxisAlign::Around,
        StackJustify::Evenly => StackAxisAlign::Evenly,
    })
}

fn cross_axis(a: StackAxisAlign) -> Option<AlignItems> {
    Some(match a {
        StackAxisAlign::Start => AlignItems::FLEX_START,
        StackAxisAlign::Center => AlignItems::CENTER,
        StackAxisAlign::End => AlignItems::FLEX_END,
        _ => AlignItems::STRETCH,
    })
}

fn align_items(a: StackAlign) -> Option<AlignItems> {
    Some(match a {
        StackAlign::Start => AlignItems::FLEX_START,
        StackAlign::Center => AlignItems::CENTER,
        StackAlign::End => AlignItems::FLEX_END,
        StackAlign::Stretch => AlignItems::STRETCH,
    })
}

impl<Msg: Clone + 'static> Build<Msg> for Stack<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let row = self.direction == StackDirection::Horizontal;
        let (main, cross) = if row {
            (self.h_align, self.v_align)
        } else {
            (self.v_align, self.h_align)
        };
        let gap = self.gap.map_or(0.0, |g| g.points(t));
        let mut style = Style {
            flex_direction: if row {
                FlexDirection::Row
            } else {
                FlexDirection::Column
            },
            flex_wrap: match self.wrap {
                StackWrap::Nowrap => FlexWrap::NoWrap,
                StackWrap::Wrap => FlexWrap::Wrap,
                StackWrap::WrapReverse => FlexWrap::WrapReverse,
            },
            gap: taffy::Size {
                width: len(gap),
                height: len(gap),
            },
            padding: pad_rect(&self.padding, cx),
            justify_content: self
                .justify
                .and_then(justify_axis)
                .or(main.and_then(main_axis))
                .unwrap_or(AlignContent::NORMAL),
            align_items: self
                .align
                .and_then(align_items)
                .or(cross.and_then(cross_axis))
                .unwrap_or(AlignItems::STRETCH),
            min_size: taffy::Size {
                width: LengthPercentageAuto::length(0.0),
                height: LengthPercentageAuto::length(0.0),
            },
            ..Style::default()
        };
        self.sizing.apply(&mut style);
        if self.is_scrollable {
            style.overflow = if row {
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
            style.flex_grow = 1.0;
            style.flex_shrink = 1.0;
        }
        let mut node = Node::new(style);
        if self.is_scrollable {
            node.scroll = Some(if row {
                astryx_core::authoring::Axis::Horizontal
            } else {
                astryx_core::authoring::Axis::Vertical
            });
        }
        cx.key(self.base.id.as_deref());
        cx.begin(node);
        for child in self.children {
            cx.child(child);
        }
        cx.end();
    }
}

/// A horizontal [`Stack`] (`HStack`). Children stretch to its height unless
/// `v_align` says otherwise.
pub struct HStack<Msg>(Stack<Msg>);
/// A vertical [`Stack`] (`VStack`). Children stretch to its width unless
/// `h_align` says otherwise.
pub struct VStack<Msg>(Stack<Msg>);

macro_rules! fixed_stack {
    ($name:ident, $dir:expr) => {
        impl<Msg> Default for $name<Msg> {
            fn default() -> Self {
                $name(Stack::default().direction($dir))
            }
        }
        impl<Msg> $name<Msg> {
            pub fn new() -> Self {
                Self::default()
            }
            pub fn with_children<E: Into<Element<Msg>>>(
                children: impl IntoIterator<Item = E>,
            ) -> Self {
                Self::default().children(children)
            }
            pub fn id(self, id: impl Into<String>) -> Self {
                $name(self.0.id(id))
            }
            pub fn is_hidden(self, on: bool) -> Self {
                $name(self.0.is_hidden(on))
            }
            pub fn child(self, child: impl Into<Element<Msg>>) -> Self {
                $name(self.0.child(child))
            }
            pub fn children<E: Into<Element<Msg>>>(
                self,
                children: impl IntoIterator<Item = E>,
            ) -> Self {
                $name(self.0.children(children))
            }
            pub fn gap(self, gap: Space) -> Self {
                $name(self.0.gap(gap))
            }
            pub fn padding(self, step: Space) -> Self {
                $name(self.0.padding(step))
            }
            pub fn padding_inline(self, step: Space) -> Self {
                $name(self.0.padding_inline(step))
            }
            pub fn padding_inline_start(self, step: Space) -> Self {
                $name(self.0.padding_inline_start(step))
            }
            pub fn padding_inline_end(self, step: Space) -> Self {
                $name(self.0.padding_inline_end(step))
            }
            pub fn padding_block(self, step: Space) -> Self {
                $name(self.0.padding_block(step))
            }
            pub fn padding_block_start(self, step: Space) -> Self {
                $name(self.0.padding_block_start(step))
            }
            pub fn padding_block_end(self, step: Space) -> Self {
                $name(self.0.padding_block_end(step))
            }
            pub fn is_scrollable(self, on: bool) -> Self {
                $name(self.0.is_scrollable(on))
            }
            pub fn h_align(self, align: StackAxisAlign) -> Self {
                $name(self.0.h_align(align))
            }
            pub fn v_align(self, align: StackAxisAlign) -> Self {
                $name(self.0.v_align(align))
            }
            pub fn justify(self, justify: StackJustify) -> Self {
                $name(self.0.justify(justify))
            }
            pub fn align(self, align: StackAlign) -> Self {
                $name(self.0.align(align))
            }
            pub fn wrap(self, wrap: StackWrap) -> Self {
                $name(self.0.wrap(wrap))
            }
            pub fn width(self, v: SizeValue) -> Self {
                $name(self.0.width(v))
            }
            pub fn height(self, v: SizeValue) -> Self {
                $name(self.0.height(v))
            }
            pub fn max_width(self, v: SizeValue) -> Self {
                $name(self.0.max_width(v))
            }
            pub fn min_height(self, v: SizeValue) -> Self {
                $name(self.0.min_height(v))
            }
        }
        impl<Msg: Clone + 'static> From<$name<Msg>> for Element<Msg> {
            fn from(s: $name<Msg>) -> Self {
                s.0.into()
            }
        }
    };
}

fixed_stack!(HStack, StackDirection::Horizontal);
fixed_stack!(VStack, StackDirection::Vertical);
