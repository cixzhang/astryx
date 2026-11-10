//! InputGroup, InputGroupText, Field, FieldStatus and FormLayout.

use super::button::{element_height, GroupKind, GroupMember, Orientation};
use super::field::{
    field_methods, field_status, label_column, status_box, FieldParts, StatusType, StatusVariant,
};
use crate::layout::{size_dim, SizeValue};
use crate::util::{color, component, len, len_auto, radii};
use astryx_core::authoring::{
    text_node, type_style, Base, BoxDraw, Build, BuildCx, Draw, Node, Semantics,
};
use astryx_core::contract::{FontFamily, Radii, Role};
use astryx_core::Element;
use astryx_spike_foundations::{provide_size, ComponentSize, CxExt};
use std::rc::Rc;
use taffy::{AlignItems, Dimension, FlexDirection, Style};

/// Fields and add-ons joined into one control (`InputGroup`): a TextInput
/// with text or button add-ons. The group owns the label and status; its
/// size and disabled state reach every member through context.
pub struct InputGroup<Msg> {
    base: Base,
    field: FieldParts,
    size: Option<ComponentSize>,
    children: Vec<Element<Msg>>,
}

impl<Msg> InputGroup<Msg> {
    pub fn new(label: impl Into<String>) -> Self {
        InputGroup {
            base: Base::default(),
            field: FieldParts::new(label),
            size: None,
            children: Vec::new(),
        }
    }
    pub fn size(mut self, s: ComponentSize) -> Self {
        self.size = Some(s);
        self
    }
    pub fn child(mut self, c: impl Into<Element<Msg>>) -> Self {
        self.children.push(c.into());
        self
    }
    field_methods!();
}

component!(InputGroup<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for InputGroup<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let f = &self.field;
        let size = cx.size(self.size, ComponentSize::Md);
        cx.key(self.base.id.as_deref());
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Column,
                gap: taffy::Size {
                    width: len(0.0),
                    height: len(t.space.s1),
                },
                ..Style::default()
            })
            .sem(Semantics::new(Role::Group, f.name(cx)).with_state(|s| {
                s.required = f.is_required;
            })),
        );
        label_column(cx, f);
        let row = cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::STRETCH,
                size: taffy::Size {
                    width: Dimension::auto(),
                    height: Dimension::length(element_height(t, size)),
                },
                ..Style::default()
            })
            .opacity(if f.is_disabled { 0.5 } else { 1.0 }),
        );
        let group = cx.id_of(row);
        let n = self.children.len();
        for (i, child) in self.children.into_iter().enumerate() {
            let member = GroupMember {
                orientation: Orientation::Horizontal,
                is_disabled: f.is_disabled,
                first: i == 0,
                last: i + 1 == n,
                group,
                kind: GroupKind::Inputs,
            };
            cx.child(provide_size(child, Some(size)).provide(Rc::new(member)));
        }
        cx.end();
        status_box(cx, f);
        cx.end();
    }
}

/// Static text beside a field in an InputGroup (`InputGroupText`), such as
/// a unit or a URL prefix.
pub struct InputGroupText {
    base: Base,
    text: String,
}

impl InputGroupText {
    pub fn new(text: impl Into<String>) -> Self {
        InputGroupText {
            base: Base::default(),
            text: text.into(),
        }
    }
}

component!(InputGroupText);

impl<Msg> Build<Msg> for InputGroupText {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let member = cx.context::<GroupMember>().map(|m| *m);
        let r = super::button::member_radii(t.radius.element, member, cx.rtl());
        let bw = t.size.border_width;
        let lead = member.is_some_and(|m| !m.first);
        let (ml, mr) = cx.inline(if lead { -bw } else { 0.0 }, 0.0);
        cx.key(self.base.id.as_deref());
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                padding: taffy::Rect {
                    left: len(t.space.s2 - bw),
                    right: len(t.space.s2 - bw),
                    top: len(0.0),
                    bottom: len(0.0),
                },
                border: crate::util::rect(len(bw)),
                margin: taffy::Rect {
                    left: len_auto(ml),
                    right: len_auto(mr),
                    top: len_auto(0.0),
                    bottom: len_auto(0.0),
                },
                flex_shrink: 0.0,
                ..Style::default()
            })
            .draw(Draw::Box(BoxDraw {
                border: Some((color(c.border_emphasized), bw)),
                ..BoxDraw::fill(color(c.background_muted), r)
            })),
        );
        cx.leaf(text_node(
            self.text,
            type_style(t.font.body, FontFamily::Body),
            color(c.text_secondary),
        ));
        cx.end();
    }
}

/// A label, description and status around any control (`Field`), for
/// controls that have none of their own.
pub struct Field<Msg> {
    base: Base,
    field: FieldParts,
    width: Option<SizeValue>,
    child: Option<Element<Msg>>,
}

impl<Msg> Field<Msg> {
    pub fn new(label: impl Into<String>, child: impl Into<Element<Msg>>) -> Self {
        Field {
            base: Base::default(),
            field: FieldParts::new(label),
            width: None,
            child: Some(child.into()),
        }
    }
    pub fn width(mut self, w: SizeValue) -> Self {
        self.width = Some(w);
        self
    }
    field_methods!();
}

component!(Field<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for Field<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let f = &self.field;
        let mut style = Style {
            flex_direction: FlexDirection::Column,
            gap: taffy::Size {
                width: len(0.0),
                height: len(t.space.s1),
            },
            ..Style::default()
        };
        if let Some(w) = self.width {
            style.size.width = size_dim(w);
        }
        cx.key(self.base.id.as_deref());
        let mut sem = Semantics::new(Role::Group, f.name(cx));
        sem.description = f.described();
        cx.begin(Node::new(style).sem(sem));
        label_column(cx, f);
        if let Some(c) = self.child {
            cx.child(c);
        }
        status_box(cx, f);
        cx.end();
    }
}

/// A status message on its own (`FieldStatus`).
pub struct FieldStatus {
    base: Base,
    status_type: StatusType,
    message: String,
    variant: StatusVariant,
}

impl FieldStatus {
    pub fn new(status_type: StatusType, message: impl Into<String>) -> Self {
        FieldStatus {
            base: Base::default(),
            status_type,
            message: message.into(),
            variant: StatusVariant::Attached,
        }
    }
    /// `Attached` (default) or `Detached`.
    pub fn variant(mut self, v: StatusVariant) -> Self {
        self.variant = v;
        self
    }
}

component!(FieldStatus);

impl<Msg> Build<Msg> for FieldStatus {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        cx.key(self.base.id.as_deref());
        // Standalone, an attached status has no control above to tuck into.
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            padding: taffy::Rect {
                left: len(0.0),
                right: len(0.0),
                top: len(if self.variant == StatusVariant::Attached {
                    cx.tokens.space.s1_5
                } else {
                    0.0
                }),
                bottom: len(0.0),
            },
            ..Style::default()
        }));
        field_status(cx, self.status_type, &self.message, self.variant);
        cx.end();
    }
}

/// FormLayout `direction`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FormDirection {
    #[default]
    Vertical,
    Horizontal,
    HorizontalLabels,
}

/// Spacing and arrangement for a form's fields (`FormLayout`).
pub struct FormLayout<Msg> {
    base: Base,
    direction: FormDirection,
    children: Vec<Element<Msg>>,
}

impl<Msg> FormLayout<Msg> {
    pub fn new() -> Self {
        FormLayout {
            base: Base::default(),
            direction: FormDirection::Vertical,
            children: Vec::new(),
        }
    }
    pub fn direction(mut self, d: FormDirection) -> Self {
        self.direction = d;
        self
    }
    pub fn child(mut self, c: impl Into<Element<Msg>>) -> Self {
        self.children.push(c.into());
        self
    }
}

impl<Msg> Default for FormLayout<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

component!(FormLayout<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for FormLayout<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let gap = t.space.s4;
        let style = match self.direction {
            FormDirection::Vertical => Style {
                flex_direction: FlexDirection::Column,
                gap: taffy::Size {
                    width: len(0.0),
                    height: len(gap),
                },
                ..Style::default()
            },
            // Equal columns, one per field.
            FormDirection::Horizontal | FormDirection::HorizontalLabels => Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FLEX_START,
                gap: taffy::Size {
                    width: len(gap),
                    height: len(t.space.s3),
                },
                ..Style::default()
            },
        };
        cx.key(self.base.id.as_deref());
        cx.begin(Node::new(style).draw(Draw::Box(BoxDraw::fill(
            astryx_core::contract::Color::TRANSPARENT,
            radii(0.0),
        ))));
        let row = self.direction != FormDirection::Vertical;
        for c in self.children {
            if row {
                cx.begin(Node::new(Style {
                    flex_grow: 1.0,
                    flex_basis: Dimension::length(0.0),
                    flex_direction: FlexDirection::Column,
                    min_size: taffy::Size {
                        width: len_auto(0.0),
                        height: taffy::LengthPercentageAuto::auto(),
                    },
                    ..Style::default()
                }));
                cx.child(c);
                cx.end();
            } else {
                cx.child(c);
            }
        }
        cx.end();
    }
}

#[allow(dead_code)]
fn _unused(_: Radii) {}
