//! TextInput and Selector.
//!
//! Shells send no text input yet, so TextInput draws its value and takes
//! focus but does not edit; `on_change` runs only from its clear button.

use super::display::{icon_node, IconSize};
use super::field::{label_column, status_box, FieldParts, Status, StatusType};
use super::text::{text_draw, text_node, type_style};
use super::{dim, len, len_auto, radii, square, Elevation};
use crate::contract::{Color, FontFamily, Role};
use crate::element::{
    base_props, Base, BoxDraw, Build, BuildCx, Draw, Indicator, Interact, Node, PopupAction,
    Semantics,
};
use std::rc::Rc;
use taffy::{AlignItems, FlexDirection, LengthPercentageAuto, Overflow, Position, Style};

/// Field height for `sm`, `md`, and `lg`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FieldSize {
    Sm,
    #[default]
    Md,
    Lg,
}

impl FieldSize {
    fn height(self, t: &astryx_tokens::Tokens) -> f32 {
        match self {
            FieldSize::Sm => t.size.element_sm,
            FieldSize::Md => t.size.element_md,
            FieldSize::Lg => t.size.element_lg,
        }
    }
}

/// TextInput `type`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextInputType {
    #[default]
    Text,
    /// Drawn masked.
    Password,
    Email,
}

/// Where a field's status message shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StatusVariant {
    #[default]
    Attached,
    Detached,
    /// Kept for the Astryx contract; tooltips are not drawn yet, so the
    /// message shows detached.
    Tooltip,
}

fn status_border(status: &Option<Status>, disabled: bool, c: &astryx_tokens::ColorTokens) -> Color {
    match status {
        Some(s) if !disabled => match s.status_type {
            StatusType::Error => c.border_error,
            StatusType::Warning => c.border_warning,
            StatusType::Success => c.border_success,
        },
        _ => c.border_emphasized,
    }
    .into()
}

fn status_icon(
    status: &Option<Status>,
    c: &astryx_tokens::ColorTokens,
) -> Option<(&'static str, Color)> {
    status.as_ref().map(|s| match s.status_type {
        StatusType::Error => ("error", c.icon_error.into()),
        StatusType::Warning => ("warning", c.icon_warning.into()),
        StatusType::Success => ("success", c.icon_success.into()),
    })
}

/// The field's accessible name: its label and optional or required note.
fn field_name(f: &FieldParts) -> String {
    if f.is_required {
        format!("{}, {}", f.label, super::strings::REQUIRED)
    } else if f.is_optional {
        format!("{}, {}", f.label, super::strings::OPTIONAL)
    } else {
        f.label.clone()
    }
}

/// A text field (`TextInput`): label, description, a bordered field, and an
/// optional status. Display-only for now: it shows `value` and takes focus.
pub struct TextInput<Msg> {
    base: Base,
    field: FieldParts,
    input_type: TextInputType,
    value: String,
    on_change: Option<Rc<dyn Fn(String) -> Msg>>,
    size: FieldSize,
    on_enter: Option<Msg>,
    is_read_only: bool,
    disabled_message: Option<String>,
    is_loading: bool,
    placeholder: Option<String>,
    label_tooltip: Option<String>,
    status_variant: StatusVariant,
    has_clear: bool,
    has_auto_focus: bool,
}

impl<Msg> TextInput<Msg> {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        TextInput {
            base: Base::default(),
            field: FieldParts {
                label: label.into(),
                ..FieldParts::default()
            },
            input_type: TextInputType::default(),
            value: value.into(),
            on_change: None,
            size: FieldSize::default(),
            on_enter: None,
            is_read_only: false,
            disabled_message: None,
            is_loading: false,
            placeholder: None,
            label_tooltip: None,
            status_variant: StatusVariant::default(),
            has_clear: false,
            has_auto_focus: false,
        }
    }
    /// Astryx `type`.
    pub fn r#type(mut self, input_type: TextInputType) -> Self {
        self.input_type = input_type;
        self
    }
    pub fn on_change(mut self, f: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
    pub fn size(mut self, size: FieldSize) -> Self {
        self.size = size;
        self
    }
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
    /// Sent when A is pressed on the focused field.
    pub fn on_enter(mut self, msg: Msg) -> Self {
        self.on_enter = Some(msg);
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.field.is_disabled = on;
        self
    }
    pub fn is_read_only(mut self, on: bool) -> Self {
        self.is_read_only = on;
        self
    }
    /// Kept for the Astryx contract; tooltips are not drawn yet.
    pub fn disabled_message(mut self, message: impl Into<String>) -> Self {
        self.disabled_message = Some(message.into());
        self
    }
    pub fn is_loading(mut self, on: bool) -> Self {
        self.is_loading = on;
        self
    }
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }
    /// Kept for the Astryx contract; tooltips are not drawn yet.
    pub fn label_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.label_tooltip = Some(tooltip.into());
        self
    }
    pub fn status(mut self, status: Status) -> Self {
        self.field.status = Some(status);
        self
    }
    pub fn status_variant(mut self, variant: StatusVariant) -> Self {
        self.status_variant = variant;
        self
    }
    /// A clear button while the field has a value; it sends `on_change("")`.
    pub fn has_clear(mut self, on: bool) -> Self {
        self.has_clear = on;
        self
    }
    /// Takes focus when the field first appears.
    pub fn has_auto_focus(mut self, on: bool) -> Self {
        self.has_auto_focus = on;
        self
    }
}

base_props!(TextInput<Msg>);

impl<Msg: Clone> Build<Msg> for TextInput<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let f = &self.field;
        let gap = len(t.space.s1);
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            gap: taffy::Size {
                width: len(0.0),
                height: gap,
            },
            ..Style::default()
        }));
        label_column(cx, f);

        let shown = match self.input_type {
            TextInputType::Password => "\u{2022}".repeat(self.value.chars().count()),
            _ => self.value.clone(),
        };
        let mut sem = Semantics::new(Role::TextInput, field_name(f));
        sem.value = Some(shown.clone());
        sem.disabled = f.is_disabled;
        let radius = radii(t.radius.element);
        let bw = t.size.border_width;
        let mut node = Node::new(Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            gap: taffy::Size {
                width: len(t.space.s2),
                height: len(0.0),
            },
            padding: taffy::Rect {
                left: len(t.space.s3 - bw),
                right: len(t.space.s3 - bw),
                top: len(0.0),
                bottom: len(0.0),
            },
            border: taffy::Rect {
                left: len(bw),
                right: len(bw),
                top: len(bw),
                bottom: len(bw),
            },
            min_size: taffy::Size {
                width: len_auto(0.0),
                height: len_auto(self.size.height(t)),
            },
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw {
            border: Some((status_border(&f.status, f.is_disabled, c), bw)),
            ..BoxDraw::fill(c.background_surface.into(), radius)
        }))
        .interact(Interact::new(
            self.on_enter.clone(),
            f.is_disabled || self.is_read_only,
        ))
        .indicator(Indicator {
            ring: None,
            radii: radius,
            overlay: None,
            field: Some((c.accent.into(), c.accent_muted.into())),
        })
        .sem(sem)
        .opacity(if f.is_disabled { 0.5 } else { 1.0 });
        node.autofocus = self.has_auto_focus;
        cx.key(self.base.id.as_deref());
        cx.begin(node);
        let body = type_style(t.font.body, FontFamily::Body);
        let mut text = if shown.is_empty() {
            text_node(
                self.placeholder.clone().unwrap_or_default(),
                body,
                c.text_secondary.into(),
            )
        } else {
            text_node(shown, body, c.text_primary.into())
        };
        text_draw(&mut text).max_lines = Some(1);
        text.style.flex_grow = 1.0;
        text.style.flex_shrink = 1.0;
        text.style.min_size.width = len_auto(0.0);
        cx.leaf(text);
        // The busy start is announced politely.
        cx.live_region(
            self.is_loading.then_some(super::strings::LOADING),
            crate::contract::Live::Polite,
        );
        if self.is_loading {
            cx.leaf(
                Node::new(Style {
                    size: square(IconSize::Sm.points()),
                    flex_shrink: 0.0,
                    ..Style::default()
                })
                .draw(Draw::Spinner(c.text_secondary.into())),
            );
            cx.animate();
        }
        if let Some((icon, color)) = status_icon(&f.status, c) {
            if let Some(n) = icon_node(icon, IconSize::Sm.points(), color) {
                cx.leaf(n);
            }
        }
        let clears =
            self.has_clear && !self.value.is_empty() && !f.is_disabled && !self.is_read_only;
        if let (true, Some(on_change)) = (clears, &self.on_change) {
            let pill = radii(t.radius.inner);
            cx.begin(
                Node::new(Style {
                    size: square(IconSize::Md.points()),
                    flex_shrink: 0.0,
                    align_items: AlignItems::CENTER,
                    justify_content: taffy::JustifyContent::CENTER,
                    ..Style::default()
                })
                .interact(Interact::new(Some(on_change(String::new())), false))
                .indicator(Indicator::ring(
                    c.accent.into(),
                    pill,
                    Some(c.overlay_pressed.into()),
                ))
                .sem(Semantics::new(Role::Button, format!("Clear {}", f.label))),
            );
            if let Some(n) = icon_node("close", IconSize::Sm.points(), c.text_secondary.into()) {
                cx.leaf(n);
            }
            cx.end();
        }
        cx.end();
        if let Some(status) = &f.status {
            status_box(cx, status);
        }
        cx.end();
    }
}

/// One choice in a Selector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectorOption {
    pub value: String,
    pub label: Option<String>,
    pub disabled: bool,
}

impl SelectorOption {
    pub fn new(value: impl Into<String>) -> Self {
        SelectorOption {
            value: value.into(),
            label: None,
            disabled: false,
        }
    }
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    pub fn disabled(mut self, on: bool) -> Self {
        self.disabled = on;
        self
    }
    fn text(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.value)
    }
}

impl<S: Into<String>> From<S> for SelectorOption {
    fn from(value: S) -> Self {
        SelectorOption::new(value)
    }
}

/// Selector `variant`: the trigger's look.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SelectorVariant {
    /// A bordered field.
    #[default]
    Input,
    Secondary,
    Ghost,
}

/// Where the trigger's chevron sits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SelectorIndicatorPosition {
    Start,
    #[default]
    End,
}

/// Picks one value from a list (`Selector`). A opens its menu; in the menu
/// the arrows move, A picks, and B closes, with focus kept inside.
pub struct Selector<Msg> {
    base: Base,
    field: FieldParts,
    options: Vec<SelectorOption>,
    value: Option<String>,
    on_change: Option<Rc<dyn Fn(String) -> Msg>>,
    has_clear: bool,
    has_search: bool,
    search_placeholder: Option<String>,
    placeholder: Option<String>,
    size: FieldSize,
    variant: SelectorVariant,
    is_read_only: bool,
    disabled_message: Option<String>,
    status_variant: StatusVariant,
    indicator_position: SelectorIndicatorPosition,
    is_loading: bool,
}

impl<Msg> Selector<Msg> {
    pub fn new<O: Into<SelectorOption>>(
        label: impl Into<String>,
        options: impl IntoIterator<Item = O>,
    ) -> Self {
        Selector {
            base: Base::default(),
            field: FieldParts {
                label: label.into(),
                ..FieldParts::default()
            },
            options: options.into_iter().map(Into::into).collect(),
            value: None,
            on_change: None,
            has_clear: false,
            has_search: false,
            search_placeholder: None,
            placeholder: None,
            size: FieldSize::default(),
            variant: SelectorVariant::default(),
            is_read_only: false,
            disabled_message: None,
            status_variant: StatusVariant::default(),
            indicator_position: SelectorIndicatorPosition::default(),
            is_loading: false,
        }
    }
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }
    pub fn on_change(mut self, f: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
    /// Kept for the Astryx contract; the menu has no clear row yet.
    pub fn has_clear(mut self, on: bool) -> Self {
        self.has_clear = on;
        self
    }
    /// Kept for the Astryx contract; shells send no text, so the menu has no search.
    pub fn has_search(mut self, on: bool) -> Self {
        self.has_search = on;
        self
    }
    pub fn search_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.search_placeholder = Some(placeholder.into());
        self
    }
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }
    pub fn size(mut self, size: FieldSize) -> Self {
        self.size = size;
        self
    }
    pub fn variant(mut self, variant: SelectorVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.field.is_disabled = on;
        self
    }
    pub fn is_read_only(mut self, on: bool) -> Self {
        self.is_read_only = on;
        self
    }
    /// Kept for the Astryx contract; tooltips are not drawn yet.
    pub fn disabled_message(mut self, message: impl Into<String>) -> Self {
        self.disabled_message = Some(message.into());
        self
    }
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
    pub fn status(mut self, status: Status) -> Self {
        self.field.status = Some(status);
        self
    }
    pub fn status_variant(mut self, variant: StatusVariant) -> Self {
        self.status_variant = variant;
        self
    }
    pub fn indicator_position(mut self, position: SelectorIndicatorPosition) -> Self {
        self.indicator_position = position;
        self
    }
    pub fn is_loading(mut self, on: bool) -> Self {
        self.is_loading = on;
        self
    }
}

base_props!(Selector<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for Selector<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let f = &self.field;
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            gap: taffy::Size {
                width: len(0.0),
                height: len(t.space.s1),
            },
            ..Style::default()
        }));
        label_column(cx, f);

        let selected = self
            .value
            .as_ref()
            .and_then(|v| self.options.iter().find(|o| &o.value == v));
        let mut sem = Semantics::new(Role::Button, field_name(f));
        sem.value = selected.map(|o| o.text().to_string());
        sem.disabled = f.is_disabled;
        let radius = radii(t.radius.element);
        let bw = t.size.border_width;
        let (fill, border): (Color, Option<(Color, f32)>) = match self.variant {
            SelectorVariant::Input => (
                c.background_surface.into(),
                Some((status_border(&f.status, f.is_disabled, c), bw)),
            ),
            SelectorVariant::Secondary => (c.neutral.into(), None),
            SelectorVariant::Ghost => (Color::TRANSPARENT, None),
        };
        let inset = t.space.s3 - border.map_or(0.0, |(_, w)| w);
        let indicator = match self.variant {
            SelectorVariant::Input => Indicator {
                ring: None,
                radii: radius,
                overlay: None,
                field: Some((c.accent.into(), c.accent_muted.into())),
            },
            _ => Indicator::ring(c.accent.into(), radius, Some(c.overlay_pressed.into())),
        };
        let mut interact =
            Interact::new(None, f.is_disabled || self.is_read_only).busy(self.is_loading);
        interact.popup = PopupAction::Toggle;
        let trigger = Node::new(Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            gap: taffy::Size {
                width: len(t.space.s2),
                height: len(0.0),
            },
            padding: taffy::Rect {
                left: len(inset),
                right: len(inset),
                top: len(0.0),
                bottom: len(0.0),
            },
            border: taffy::Rect {
                left: len(bw),
                right: len(bw),
                top: len(bw),
                bottom: len(bw),
            }
            .map(|b| if border.is_some() { b } else { len(0.0) }),
            min_size: taffy::Size {
                width: len_auto(0.0),
                height: len_auto(self.size.height(t)),
            },
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw {
            border,
            ..BoxDraw::fill(fill, radius)
        }))
        .interact(interact)
        .indicator(indicator)
        .sem(sem)
        .opacity(if f.is_disabled { 0.5 } else { 1.0 });
        cx.key(self.base.id.as_deref());
        let trigger_ix = cx.begin(trigger);
        let chevron = |cx: &mut BuildCx<Msg>| {
            if let Some(n) = icon_node(
                "chevronDown",
                IconSize::Sm.points(),
                c.icon_secondary.into(),
            ) {
                cx.leaf(n);
            }
        };
        if self.indicator_position == SelectorIndicatorPosition::Start {
            chevron(cx);
        }
        let body = type_style(t.font.body, FontFamily::Body);
        let mut text = match selected {
            Some(o) => text_node(o.text().to_string(), body, c.text_primary.into()),
            None => text_node(
                self.placeholder.clone().unwrap_or_default(),
                body,
                c.text_secondary.into(),
            ),
        };
        text_draw(&mut text).max_lines = Some(1);
        text.style.flex_grow = 1.0;
        text.style.min_size.width = len_auto(0.0);
        cx.leaf(text);
        // The busy start is announced politely.
        cx.live_region(
            self.is_loading.then_some(super::strings::LOADING),
            crate::contract::Live::Polite,
        );
        if self.is_loading {
            cx.leaf(
                Node::new(Style {
                    size: square(IconSize::Sm.points()),
                    flex_shrink: 0.0,
                    ..Style::default()
                })
                .draw(Draw::Spinner(c.text_secondary.into())),
            );
            cx.animate();
        }
        if let Some((icon, color)) = status_icon(&f.status, c) {
            if let Some(n) = icon_node(icon, IconSize::Sm.points(), color) {
                cx.leaf(n);
            }
        }
        if self.indicator_position == SelectorIndicatorPosition::End {
            chevron(cx);
        }
        cx.end();
        if let Some(status) = &f.status {
            status_box(cx, status);
        }
        cx.end();

        let id = cx.id_of(trigger_ix);
        if let Some((open, anchor)) = cx.popup {
            if open == id {
                let menu = Menu {
                    options: self.options,
                    value: self.value,
                    on_change: self.on_change,
                    anchor,
                };
                cx.overlays.push(Box::new(move |cx| menu.build(cx)));
            }
        }
    }
}

/// An open Selector's menu, placed under its trigger (or above it when the
/// space below is short).
struct Menu<Msg> {
    options: Vec<SelectorOption>,
    value: Option<String>,
    on_change: Option<Rc<dyn Fn(String) -> Msg>>,
    anchor: crate::contract::Rect,
}

impl<Msg: Clone + 'static> Menu<Msg> {
    fn build(self, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let c = &t.color;
        let pad = t.space.s1;
        let body = type_style(t.font.body, FontFamily::Body);
        let row = (body.size * body.leading - 0.01).ceil() + 2.0 * t.space.s2;
        let wanted = row * self.options.len() as f32 + 2.0 * pad;
        let gap = t.space.s1;
        let (vw, vh) = cx.viewport;
        let below = vh - (self.anchor.y + self.anchor.height + gap) - gap;
        let above = self.anchor.y - gap * 2.0;
        let (top, height) = if wanted <= below || below >= above {
            (
                self.anchor.y + self.anchor.height + gap,
                wanted.min(below.max(row)),
            )
        } else {
            let h = wanted.min(above.max(row));
            (self.anchor.y - gap - h, h)
        };
        let width = self
            .anchor
            .width
            .max(160.0)
            .min(vw - self.anchor.x.max(0.0));
        let radius = radii(t.radius.container);
        let mut menu = Node::new(Style {
            position: Position::Absolute,
            inset: taffy::Rect {
                left: len_auto(self.anchor.x),
                top: len_auto(top),
                right: LengthPercentageAuto::auto(),
                bottom: LengthPercentageAuto::auto(),
            },
            size: taffy::Size {
                width: dim(width),
                height: dim(height),
            },
            flex_direction: FlexDirection::Column,
            padding: taffy::Rect {
                left: len(pad),
                right: len(pad),
                top: len(pad),
                bottom: len(pad),
            },
            overflow: taffy::Point {
                x: Overflow::Visible,
                y: Overflow::Scroll,
            },
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw {
            shadow: Elevation::Med.layers(t),
            ..BoxDraw::fill(c.background_popover.into(), radius)
        }))
        .sem(Semantics {
            role: Role::List,
            label: None,
            value: None,
            checked: None,
            disabled: false,
            live: None,
        });
        menu.trap = true;
        menu.scroll = Some(crate::element::Axis::Vertical);
        cx.begin(menu);
        for option in &self.options {
            let chosen = self.value.as_deref() == Some(option.value.as_str());
            let mut interact = Interact::new(
                self.on_change.as_ref().map(|f| f(option.value.clone())),
                option.disabled,
            );
            interact.popup = PopupAction::Close;
            let mut sem = Semantics::new(Role::ListItem, option.text().to_string());
            sem.checked = Some(chosen);
            sem.disabled = option.disabled;
            let fill: Color = if chosen {
                c.accent_muted.into()
            } else {
                Color::TRANSPARENT
            };
            let mut node = Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                gap: taffy::Size {
                    width: len(t.space.s2),
                    height: len(0.0),
                },
                padding: taffy::Rect {
                    left: len(t.space.s2),
                    right: len(t.space.s2),
                    top: len(t.space.s2),
                    bottom: len(t.space.s2),
                },
                flex_shrink: 0.0,
                ..Style::default()
            })
            .draw(Draw::Box(BoxDraw::fill(fill, radii(t.radius.element))))
            .interact(interact)
            .indicator(Indicator::ring(
                c.accent.into(),
                radii(t.radius.element),
                Some(c.overlay_pressed.into()),
            ))
            .sem(sem)
            .opacity(if option.disabled { 0.5 } else { 1.0 });
            node.autofocus = chosen;
            cx.begin(node);
            let mut text = text_node(option.text().to_string(), body, c.text_primary.into());
            text_draw(&mut text).max_lines = Some(1);
            text.style.flex_grow = 1.0;
            text.style.min_size.width = len_auto(0.0);
            cx.leaf(text);
            if chosen {
                if let Some(n) = icon_node("check", IconSize::Sm.points(), c.icon_primary.into()) {
                    cx.leaf(n);
                }
            }
            cx.end();
        }
        cx.end();
    }
}
