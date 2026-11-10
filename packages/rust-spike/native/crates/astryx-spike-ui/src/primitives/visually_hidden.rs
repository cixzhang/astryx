//! VisuallyHidden: words assistive technology reads but nobody sees.

use crate::util::component;
use astryx_core::authoring::{Base, Build, BuildCx, Node, Semantics};
use astryx_core::contract::Role;
use taffy::Style;

/// Text in the accessibility tree with no size on screen (`VisuallyHidden`).
#[derive(Clone, Debug, Default)]
pub struct VisuallyHidden {
    base: Base,
    children: String,
}

impl VisuallyHidden {
    pub fn new(children: impl Into<String>) -> Self {
        VisuallyHidden {
            base: Base::default(),
            children: children.into(),
        }
    }
}

component!(VisuallyHidden);

impl<Msg> Build<Msg> for VisuallyHidden {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        cx.key(self.base.id.as_deref());
        cx.leaf(
            Node::new(Style {
                position: taffy::Position::Absolute,
                size: taffy::Size {
                    width: taffy::Dimension::length(1.0),
                    height: taffy::Dimension::length(1.0),
                },
                ..Style::default()
            })
            .sem(Semantics::new(Role::Text, self.children)),
        );
    }
}
