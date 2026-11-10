//! The field system: label, description, optional or required marker,
//! label tooltip, and status message, shared by every input.

use crate::util::{color, len, radii};
use astryx_core::authoring::{text_node, type_style, BoxDraw, BuildCx, Draw, Node, Semantics};
use astryx_core::contract::{Color, FontFamily, Live, Role};
use astryx_spike_foundations::{args, CxExt};
use taffy::{AlignItems, FlexDirection, Style};

/// Field `status.type`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatusType {
    Error,
    Warning,
    Success,
}

/// Field `status`: a type and an optional message.
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

/// Where a field shows its status message.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StatusVariant {
    /// Under the control, joined to it.
    #[default]
    Attached,
    /// Under the control, as its own box.
    Detached,
    /// Behind a status button in the control.
    Tooltip,
}

/// What every field shares.
#[derive(Clone, Debug, Default)]
pub struct FieldParts {
    pub label: String,
    pub is_label_hidden: bool,
    pub description: Option<String>,
    pub is_optional: bool,
    pub is_required: bool,
    pub is_disabled: bool,
    pub label_tooltip: Option<String>,
    pub status: Option<Status>,
    pub status_variant: StatusVariant,
}

/// The builder methods every field shares, over a `field: FieldParts`.
macro_rules! field_methods {
    () => {
        /// Hides the visible label; it still names the control.
        pub fn is_label_hidden(mut self, on: bool) -> Self {
            self.field.is_label_hidden = on;
            self
        }
        pub fn description(mut self, description: impl Into<String>) -> Self {
            self.field.description = Some(description.into());
            self
        }
        pub fn is_optional(mut self, on: bool) -> Self {
            self.field.is_optional = on;
            self
        }
        pub fn is_required(mut self, on: bool) -> Self {
            self.field.is_required = on;
            self
        }
        pub fn is_disabled(mut self, on: bool) -> Self {
            self.field.is_disabled = on;
            self
        }
        pub fn label_tooltip(mut self, tooltip: impl Into<String>) -> Self {
            self.field.label_tooltip = Some(tooltip.into());
            self
        }
        pub fn status(mut self, status: $crate::controls::Status) -> Self {
            self.field.status = Some(status);
            self
        }
        pub fn status_variant(mut self, variant: $crate::controls::StatusVariant) -> Self {
            self.field.status_variant = variant;
            self
        }
    };
}
pub(crate) use field_methods;

impl FieldParts {
    pub fn new(label: impl Into<String>) -> Self {
        FieldParts {
            label: label.into(),
            ..Self::default()
        }
    }

    /// The control's accessible name: the label, with the marker spoken.
    pub fn name<Msg>(&self, cx: &BuildCx<Msg>) -> String {
        if self.is_required {
            format!(
                "{}, {}",
                self.label,
                cx.t("@astryx.field.required", &args! {})
            )
        } else if self.is_optional {
            format!(
                "{}, {}",
                self.label,
                cx.t("@astryx.field.optional", &args! {})
            )
        } else {
            self.label.clone()
        }
    }

    /// The control's accessible description: the description, then the
    /// status message.
    pub fn described(&self) -> Option<String> {
        let status = self.status.as_ref().and_then(|s| s.message.clone());
        match (&self.description, status) {
            (Some(d), Some(s)) => Some(format!("{d}. {s}")),
            (Some(d), None) => Some(d.clone()),
            (None, s) => s,
        }
    }
}

/// The border color of a control for its status.
pub(crate) fn status_border<Msg>(cx: &BuildCx<Msg>, f: &FieldParts) -> Color {
    let c = &cx.tokens.color;
    match &f.status {
        Some(s) if !f.is_disabled => color(match s.status_type {
            StatusType::Error => c.border_error,
            StatusType::Warning => c.border_warning,
            StatusType::Success => c.border_success,
        }),
        _ => color(c.border_emphasized),
    }
}

/// The status icon and its color.
pub(crate) fn status_icon<Msg>(cx: &BuildCx<Msg>, f: &FieldParts) -> Option<(&'static str, Color)> {
    let c = &cx.tokens.color;
    f.status.as_ref().map(|s| match s.status_type {
        StatusType::Error => ("error", color(c.icon_error)),
        StatusType::Warning => ("warning", color(c.icon_warning)),
        StatusType::Success => ("success", color(c.icon_success)),
    })
}

/// Adds the label row (label, marker, info icon) and the description under
/// the open node. They are read through the control, so they carry no
/// semantics of their own.
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
        let ink = if f.is_disabled {
            color(c.text_disabled)
        } else {
            color(c.text_secondary)
        };
        let mut label = text_node(f.label.clone(), style, ink);
        label.sem = None;
        cx.leaf(label);
        let marker = if f.is_required {
            Some(cx.t("@astryx.field.required", &args! {}))
        } else if f.is_optional {
            Some(cx.t("@astryx.field.optional", &args! {}))
        } else {
            None
        };
        if let Some(marker) = marker {
            let mut m = text_node(
                format!("\u{b7} {marker}"),
                type_style(t.font.supporting, FontFamily::Body),
                color(c.text_secondary),
            );
            m.sem = None;
            cx.leaf(m);
        }
        if let Some(tip) = &f.label_tooltip {
            // The InfoTip shows the words on hover or focus once layers land;
            // until then the icon carries them as its description.
            let mut icon =
                crate::primitives::icon_node::<Msg>("info", 14.0, color(c.icon_secondary))
                    .expect("info glyph");
            let about = cx.t(
                "@astryx.field.labelTooltip",
                &args! {"label" => f.label.clone()},
            );
            let mut sem = Semantics::new(Role::Image, about);
            sem.description = Some(tip.clone());
            icon.sem = Some(sem);
            icon.tooltip = Some(tip.clone());
            cx.leaf(icon);
        }
        cx.end();
    }
    if let Some(description) = &f.description {
        let mut d = text_node(
            description.clone(),
            type_style(t.font.supporting, FontFamily::Body),
            color(c.text_secondary),
        );
        d.sem = None;
        cx.leaf(d);
    }
}

/// The status message under a control (`FieldStatus`), when it has one.
pub(crate) fn status_box<Msg>(cx: &mut BuildCx<Msg>, f: &FieldParts) {
    let Some(status) = &f.status else {
        return;
    };
    let Some(message) = &status.message else {
        return;
    };
    if f.status_variant == StatusVariant::Tooltip {
        return;
    }
    field_status(cx, status.status_type, message, f.status_variant);
}

/// A status message: an icon and words, on the muted status ground when
/// detached, or plain colored text when attached.
pub(crate) fn field_status<Msg>(
    cx: &mut BuildCx<Msg>,
    status_type: StatusType,
    message: &str,
    variant: StatusVariant,
) {
    let t = cx.tokens;
    let c = &t.color;
    let (bg, fg, icon, ink) = match status_type {
        StatusType::Warning => (
            c.warning_muted,
            c.on_warning_muted,
            "warning",
            c.icon_warning,
        ),
        StatusType::Error => (c.error_muted, c.on_error_muted, "error", c.icon_error),
        StatusType::Success => (
            c.success_muted,
            c.on_success_muted,
            "success",
            c.icon_success,
        ),
    };
    let detached = variant == StatusVariant::Detached;
    // Attached, the box tucks 6 points under the control above it, so it
    // reads as one piece with rounded bottom corners.
    let overlap = t.space.s1_5;
    let s2 = t.space.s2;
    let r = t.radius.element;
    let node = Node::new(Style {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::FLEX_START,
        gap: taffy::Size {
            width: len(t.space.s1),
            height: len(0.0),
        },
        padding: taffy::Rect {
            left: len(s2),
            right: len(s2),
            top: len(if detached { s2 } else { overlap + s2 }),
            bottom: len(s2),
        },
        margin: taffy::Rect {
            left: crate::util::len_auto(0.0),
            right: crate::util::len_auto(0.0),
            top: crate::util::len_auto(if detached { t.space.s1 } else { -overlap }),
            bottom: crate::util::len_auto(0.0),
        },
        ..Style::default()
    })
    .draw(Draw::Box(BoxDraw::fill(
        color(bg),
        if detached {
            radii(r)
        } else {
            astryx_core::contract::Radii([0.0, 0.0, r, r])
        },
    )))
    .sem(Semantics {
        // Errors interrupt; warnings and successes wait.
        live: Some(if status_type == StatusType::Error {
            Live::Assertive
        } else {
            Live::Polite
        }),
        ..Semantics::new(Role::Text, message)
    });
    cx.begin(node);
    if detached {
        if let Some(i) = crate::primitives::icon_node::<Msg>(icon, 16.0, color(ink)) {
            cx.leaf(i);
        }
    }
    let text_color = color(fg);
    let mut words = text_node(
        message.to_string(),
        type_style(t.font.supporting, FontFamily::Body),
        text_color,
    );
    words.sem = None;
    cx.leaf(words);
    cx.end();
}
