//! AspectRatio: holds its content to a width-to-height ratio.

use crate::util::{component, radii};
use astryx_core::authoring::{Base, Build, BuildCx, Node};
use astryx_core::Element;
use taffy::{AlignContent, AlignItems, Dimension, Style};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AspectRatioShape {
    #[default]
    Rectangle,
    Ellipse,
}

/// How content fills the box: `Cover` and `Contain` stretch the child to
/// the box, `Center` centers it at its own size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AspectRatioFit {
    #[default]
    Cover,
    Contain,
    Center,
}

/// A box of a fixed ratio that fills its row's width (`AspectRatio`).
pub struct AspectRatio<Msg> {
    base: Base,
    ratio: f32,
    shape: AspectRatioShape,
    fit: Option<AspectRatioFit>,
    child: Option<Element<Msg>>,
}

impl<Msg> AspectRatio<Msg> {
    pub fn new(ratio: f32, child: impl Into<Element<Msg>>) -> Self {
        AspectRatio {
            base: Base::default(),
            ratio,
            shape: AspectRatioShape::Rectangle,
            fit: None,
            child: Some(child.into()),
        }
    }
    pub fn shape(mut self, shape: AspectRatioShape) -> Self {
        self.shape = shape;
        self
    }
    pub fn fit(mut self, fit: AspectRatioFit) -> Self {
        self.fit = Some(fit);
        self
    }
}

component!(AspectRatio<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for AspectRatio<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let center = self.fit == Some(AspectRatioFit::Center);
        let mut node = Node::new(Style {
            size: taffy::Size {
                width: Dimension::percent(1.0),
                height: Dimension::auto(),
            },
            aspect_ratio: Some(self.ratio.max(0.01)),
            flex_shrink: 0.0,
            justify_content: if center {
                AlignContent::CENTER
            } else {
                AlignContent::NORMAL
            },
            align_items: if center {
                AlignItems::CENTER
            } else {
                AlignItems::STRETCH
            },
            overflow: taffy::Point {
                x: taffy::Overflow::Clip,
                y: taffy::Overflow::Clip,
            },
            ..Style::default()
        });
        node.clip = Some(match self.shape {
            // A large radius on a box draws its ellipse inscribed.
            AspectRatioShape::Ellipse => radii(1.0e5),
            AspectRatioShape::Rectangle => radii(0.0),
        });
        cx.key(self.base.id.as_deref());
        cx.begin(node);
        if let Some(child) = self.child {
            if center {
                cx.child(child);
            } else {
                // The child fills the box.
                cx.begin(Node::new(Style {
                    flex_grow: 1.0,
                    size: taffy::Size {
                        width: Dimension::percent(1.0),
                        height: Dimension::percent(1.0),
                    },
                    ..Style::default()
                }));
                cx.child(child);
                cx.end();
            }
        }
        cx.end();
    }
}
