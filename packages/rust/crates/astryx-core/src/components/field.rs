//! Shared field parts: the label row, description, and status message that
//! Switch, TextInput, and Selector draw (Astryx `FieldLabel` and `FieldStatus`).

use super::text::{text_node, type_style};
use super::{len, radii, strings};
use crate::contract::{Color, FontFamily, Live, Role};
use crate::element::{BoxDraw, BuildCx, Draw, Node, Semantics};
use taffy::{AlignItems, FlexDirection, Style};

/// The kind of a field `status`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatusType {
    Warning,
    Error,
    Success,
}

/// A field's validation `status`: its type and an optional message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub status_type: StatusType,
    pub message: Option<String>,
}

impl Status {
    pub fn new(status_type: StatusType, message: Option<String>) -> Self {
        Status {
            status_type,
            message,
        }
    }
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(StatusType::Error, Some(message.into()))
    }
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(StatusType::Warning, Some(message.into()))
    }
    pub fn success(message: impl Into<String>) -> Self {
        Self::new(StatusType::Success, Some(message.into()))
    }
}

/// What every field shares.
#[derive(Clone, Debug, Default)]
pub(crate) struct FieldParts {
    pub label: String,
    pub is_label_hidden: bool,
    pub description: Option<String>,
    pub is_optional: bool,
    pub is_required: bool,
    pub is_disabled: bool,
    pub status: Option<Status>,
}

/// Adds the label row (label and optional or required marker) and the
/// description under the open node. They are read through the control, so
/// they carry no semantics of their own.
pub(crate) fn label_column<Msg>(cx: &mut BuildCx<Msg>, f: &FieldParts) {
    let t = cx.tokens;
    let c = &t.color;
    if !f.is_label_hidden {
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            gap: taffy::Size {
                width: len(t.space.s1),
                height: len(0.0),
            },
            ..Style::default()
        }));
        let mut style = type_style(t.font.label, FontFamily::Body);
        style.weight = t.font.weight.medium;
        let color: Color = if f.is_disabled {
            c.text_disabled.into()
        } else {
            c.text_secondary.into()
        };
        cx.leaf(text_node(f.label.clone(), style, color));
        let marker = if f.is_required {
            Some(strings::REQUIRED)
        } else if f.is_optional {
            Some(strings::OPTIONAL)
        } else {
            None
        };
        if let Some(marker) = marker {
            cx.leaf(text_node(
                format!("\u{b7} {marker}"),
                type_style(t.font.supporting, FontFamily::Body),
                c.text_secondary.into(),
            ));
        }
        cx.end();
    }
    if let Some(description) = &f.description {
        cx.leaf(text_node(
            description.clone(),
            type_style(t.font.supporting, FontFamily::Body),
            c.text_secondary.into(),
        ));
    }
}

/// A detached status message box, when the status has a message.
pub(crate) fn status_box<Msg>(cx: &mut BuildCx<Msg>, status: &Status) {
    let Some(message) = &status.message else {
        return;
    };
    let t = cx.tokens;
    let c = &t.color;
    let (bg, fg) = match status.status_type {
        StatusType::Warning => (c.warning_muted, c.on_warning_muted),
        StatusType::Error => (c.error_muted, c.on_error_muted),
        StatusType::Success => (c.success_muted, c.on_success_muted),
    };
    let pad = len(t.space.s2);
    cx.begin(
        Node::new(Style {
            flex_direction: FlexDirection::Row,
            padding: taffy::Rect {
                left: pad,
                right: pad,
                top: pad,
                bottom: pad,
            },
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw::fill(bg.into(), radii(t.radius.element))))
        .sem(Semantics {
            // Errors interrupt; warnings and successes wait, as FieldStatus announces them.
            live: Some(if status.status_type == StatusType::Error {
                Live::Assertive
            } else {
                Live::Polite
            }),
            ..Semantics::new(Role::Text, message.clone())
        }),
    );
    cx.leaf(text_node(
        message.clone(),
        type_style(t.font.supporting, FontFamily::Body),
        fg.into(),
    ));
    cx.end();
}
