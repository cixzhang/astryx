//! Center and AspectRatio.

use super::stack::{padding_methods, sizing_methods, SizeValue, Sizing};
use super::{pad_rect, Padding};
use crate::util::component;
use astryx_core::authoring::{Base, Build, BuildCx, Node};
use astryx_core::{Element, Space};
use taffy::{AlignContent, AlignItems, Style};

/// Center `axis`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CenterAxis {
    #[default]
    Both,
    Horizontal,
    Vertical,
}

/// Centers its content on one or both axes (`Center`).
pub struct Center<Msg> {
    base: Base,
    axis: CenterAxis,
    sizing: Sizing,
    padding: Padding,
    is_inline: bool,
    children: Vec<Element<Msg>>,
}

impl<Msg> Default for Center<Msg> {
    fn default() -> Self {
        Center {
            base: Base::default(),
            axis: CenterAxis::Both,
            sizing: Sizing::default(),
            padding: Padding::default(),
            is_inline: false,
            children: Vec::new(),
        }
    }
}

impl<Msg> Center<Msg> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn axis(mut self, axis: CenterAxis) -> Self {
        self.axis = axis;
        self
    }
    /// Hugs its content instead of filling the row.
    pub fn is_inline(mut self, on: bool) -> Self {
        self.is_inline = on;
        self
    }
    pub fn child(mut self, child: impl Into<Element<Msg>>) -> Self {
        self.children.push(child.into());
        self
    }
    padding_methods!();
    sizing_methods!();
}

component!(Center<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for Center<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let horizontal = self.axis != CenterAxis::Vertical;
        let vertical = self.axis != CenterAxis::Horizontal;
        let mut style = Style {
            justify_content: if horizontal {
                AlignContent::CENTER
            } else {
                AlignContent::NORMAL
            },
            align_items: if vertical {
                AlignItems::CENTER
            } else {
                AlignItems::STRETCH
            },
            padding: pad_rect(&self.padding, cx),
            align_self: self.is_inline.then_some(AlignItems::FLEX_START),
            ..Style::default()
        };
        self.sizing.apply(&mut style);
        cx.key(self.base.id.as_deref());
        cx.begin(Node::new(style));
        for child in self.children {
            cx.child(child);
        }
        cx.end();
    }
}

#[allow(dead_code)]
fn _size_value_is_used(_: SizeValue, _: Space) {}
