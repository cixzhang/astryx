//! EmptyState: what a region shows when it has nothing yet.

use crate::util::{color, component, len, len_auto};
use astryx_core::authoring::{
    text_draw, text_node, type_style, Base, Build, BuildCx, Node, Semantics,
};
use astryx_core::contract::{FontFamily, Role, TextAlign};
use astryx_core::Element;
use taffy::{AlignItems, FlexDirection, JustifyContent, LengthPercentageAuto, Style};

/// A message for an empty region, with an optional icon and actions
/// (`EmptyState`).
pub struct EmptyState<Msg> {
    base: Base,
    title: String,
    description: Option<String>,
    icon: Option<Element<Msg>>,
    actions: Vec<Element<Msg>>,
    heading_level: u8,
    is_compact: bool,
}

impl<Msg> EmptyState<Msg> {
    pub fn new(title: impl Into<String>) -> Self {
        EmptyState {
            base: Base::default(),
            title: title.into(),
            description: None,
            icon: None,
            actions: Vec::new(),
            heading_level: 3,
            is_compact: false,
        }
    }
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
    /// A decorative icon or illustration above the title.
    pub fn icon(mut self, icon: impl Into<Element<Msg>>) -> Self {
        self.icon = Some(icon.into());
        self
    }
    pub fn action(mut self, action: impl Into<Element<Msg>>) -> Self {
        self.actions.push(action.into());
        self
    }
    pub fn heading_level(mut self, level: u8) -> Self {
        self.heading_level = level.clamp(1, 6);
        self
    }
    /// Tighter spacing and smaller type, for panels and cards.
    pub fn is_compact(mut self, on: bool) -> Self {
        self.is_compact = on;
        self
    }
}

component!(EmptyState<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for EmptyState<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let s = &t.space;
        let (gap, pad_block, pad_inline) = if self.is_compact {
            (s.s2, s.s4, s.s4)
        } else {
            (s.s4, s.s8, s.s6)
        };
        cx.key(self.base.id.as_deref());
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::CENTER,
            justify_content: JustifyContent::CENTER,
            gap: taffy::Size {
                width: len(0.0),
                height: len(gap),
            },
            padding: taffy::Rect {
                left: len(pad_inline),
                right: len(pad_inline),
                top: len(pad_block),
                bottom: len(pad_block),
            },
            ..Style::default()
        }));
        if let Some(icon) = self.icon {
            cx.child(icon);
        }
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::CENTER,
            max_size: taffy::Size {
                width: LengthPercentageAuto::length(360.0),
                height: LengthPercentageAuto::auto(),
            },
            ..Style::default()
        }));
        let mut title_style = type_style(t.font.large, FontFamily::Body);
        title_style.weight = t.font.weight.semibold;
        if self.is_compact {
            title_style.size = t.font.label.size;
        }
        let mut title = text_node(self.title, title_style, color(t.color.text_primary));
        text_draw(&mut title).align = TextAlign::Center;
        title.sem = Some(Semantics::new(
            Role::Heading(self.heading_level),
            text_draw(&mut title).text.clone(),
        ));
        cx.leaf(title);
        if let Some(d) = self.description {
            let mut style = type_style(t.font.body, FontFamily::Body);
            style.weight = t.font.weight.normal;
            if self.is_compact {
                style.size = t.font.supporting.size;
            }
            let mut desc = text_node(d, style, color(t.color.text_secondary));
            text_draw(&mut desc).align = TextAlign::Center;
            cx.leaf(desc);
        }
        cx.end();
        if !self.actions.is_empty() {
            cx.begin(Node::new(Style {
                flex_direction: if self.is_compact {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                },
                align_items: AlignItems::CENTER,
                gap: taffy::Size {
                    width: len(s.s2),
                    height: len(s.s2),
                },
                margin: taffy::Rect {
                    left: len_auto(0.0),
                    right: len_auto(0.0),
                    top: len_auto(s.s1),
                    bottom: len_auto(0.0),
                },
                ..Style::default()
            }));
            for a in self.actions {
                cx.child(a);
            }
            cx.end();
        }
        cx.end();
    }
}
