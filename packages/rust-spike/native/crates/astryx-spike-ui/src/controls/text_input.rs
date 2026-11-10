//! TextInput: a single-line text field with the full field system.

use super::button::element_height;
use super::field::{
    field_methods, label_column, status_border, status_box, status_icon, FieldParts, StatusVariant,
};
use crate::layout::{size_dim, SizeValue};
use crate::primitives::{icon_node, spinner_painter, Icon, IconSize};
use crate::util::{color, component, len, len_auto, radii, square};
use astryx_core::authoring::{
    text_draw, text_node, type_style, Base, BoxDraw, Build, BuildCx, Draw, Edit, Indicator,
    Interact, Node, Semantics,
};
use astryx_core::contract::{FontFamily, Live, Role};
use astryx_spike_foundations::{args, ComponentSize, CxExt};
use std::rc::Rc;
use taffy::{AlignItems, FlexDirection, Style};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextInputType {
    #[default]
    Text,
    Password,
    Email,
    Search,
    Tel,
    Url,
}

/// A single-line text field (`TextInput`). Typing, selection, paste and
/// IME composition come from the shell through the runtime's edit support.
pub struct TextInput<Msg> {
    base: Base,
    field: FieldParts,
    value: String,
    input_type: TextInputType,
    on_change: Option<Rc<dyn Fn(String) -> Msg>>,
    on_enter: Option<Msg>,
    size: Option<ComponentSize>,
    is_read_only: bool,
    disabled_message: Option<String>,
    is_loading: bool,
    placeholder: Option<String>,
    start_icon: Option<Icon>,
    has_clear: bool,
    has_auto_focus: bool,
    width: Option<SizeValue>,
}

impl<Msg> TextInput<Msg> {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        TextInput {
            base: Base::default(),
            field: FieldParts::new(label),
            value: value.into(),
            input_type: TextInputType::Text,
            on_change: None,
            on_enter: None,
            size: None,
            is_read_only: false,
            disabled_message: None,
            is_loading: false,
            placeholder: None,
            start_icon: None,
            has_clear: false,
            has_auto_focus: false,
            width: None,
        }
    }
    pub fn r#type(mut self, t: TextInputType) -> Self {
        self.input_type = t;
        self
    }
    pub fn on_change(mut self, f: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
    pub fn on_enter(mut self, msg: Msg) -> Self {
        self.on_enter = Some(msg);
        self
    }
    /// Without a size, the field takes its group's size, else `md`.
    pub fn size(mut self, size: ComponentSize) -> Self {
        self.size = Some(size);
        self
    }
    pub fn is_read_only(mut self, on: bool) -> Self {
        self.is_read_only = on;
        self
    }
    pub fn disabled_message(mut self, m: impl Into<String>) -> Self {
        self.disabled_message = Some(m.into());
        self
    }
    pub fn is_loading(mut self, on: bool) -> Self {
        self.is_loading = on;
        self
    }
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = Some(p.into());
        self
    }
    pub fn start_icon(mut self, icon: Icon) -> Self {
        self.start_icon = Some(icon);
        self
    }
    /// A clear (×) button while the field has a value.
    pub fn has_clear(mut self, on: bool) -> Self {
        self.has_clear = on;
        self
    }
    pub fn has_auto_focus(mut self, on: bool) -> Self {
        self.has_auto_focus = on;
        self
    }
    pub fn width(mut self, w: SizeValue) -> Self {
        self.width = Some(w);
        self
    }
    field_methods!();
}

component!(TextInput<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for TextInput<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let member = cx.context::<super::button::GroupMember>().map(|m| *m);
        let mut field = self.field.clone();
        if let Some(m) = member {
            // The group owns the label and status; the field fills its slot.
            field.is_label_hidden = true;
            field.status = None;
            field.is_disabled |= m.is_disabled;
        }
        let f = &field;
        let size = cx.size(self.size, ComponentSize::Md);
        let mut outer = Style {
            flex_direction: FlexDirection::Column,
            gap: taffy::Size {
                width: len(0.0),
                height: len(t.space.s1),
            },
            ..Style::default()
        };
        if let Some(w) = self.width {
            outer.size.width = size_dim(w);
        }
        if let Some(m) = member {
            outer.flex_grow = 1.0;
            outer.flex_shrink = 1.0;
            outer.min_size.width = len_auto(0.0);
            if !m.first {
                let bw = t.size.border_width;
                let (l, r) = cx.inline(-bw, 0.0);
                outer.margin.left = len_auto(l);
                outer.margin.right = len_auto(r);
            }
        }
        cx.begin(Node::new(outer));
        label_column(cx, f);

        let masked = self.input_type == TextInputType::Password;
        let shown = if masked {
            "\u{2022}".repeat(self.value.chars().count())
        } else {
            self.value.clone()
        };
        let mut sem = Semantics::new(Role::TextInput, f.name(cx));
        sem.value = Some(shown.clone());
        sem.secure = masked;
        sem.description = f.described();
        sem.disabled = f.is_disabled;
        let radius = super::button::member_radii(t.radius.element, member, cx.rtl());
        let bw = t.size.border_width;
        // In the attached status, the field's border takes the status color.
        let border = status_border(cx, f);
        let ground = if self.is_read_only {
            color(c.background_muted)
        } else {
            color(c.background_surface)
        };
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
            border: crate::util::rect(len(bw)),
            min_size: taffy::Size {
                width: len_auto(0.0),
                height: len_auto(element_height(t, size)),
            },
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw {
            border: Some((border, bw)),
            ..BoxDraw::fill(ground, radius)
        }))
        .interact(Interact::new(self.on_enter.clone(), f.is_disabled))
        .indicator(Indicator {
            ring: None,
            radii: radius,
            overlay: None,
            field: Some((color(c.accent), color(c.accent_muted))),
            hold_fill: None,
        })
        .sem(sem)
        .tooltip(if f.is_disabled {
            self.disabled_message.clone()
        } else {
            None
        })
        .opacity(if f.is_disabled { 0.5 } else { 1.0 });
        node.autofocus = self.has_auto_focus;
        cx.key(self.base.id.as_deref());
        let field_ix = cx.begin(node);
        if let Some(icon) = self.start_icon {
            let pushed = astryx_core::authoring::Inherited {
                color: color(c.icon_secondary),
                ..cx.inherited()
            };
            cx.inherit.push(pushed);
            cx.child(icon.default_size(IconSize::Sm).into());
            cx.inherit.pop();
        }
        let body = type_style(t.font.body, FontFamily::Body);
        let mut text = if shown.is_empty() {
            text_node(
                self.placeholder.clone().unwrap_or_default(),
                body,
                color(c.text_secondary),
            )
        } else {
            text_node(shown, body, color(c.text_primary))
        };
        text.sem = None;
        let editable = !f.is_disabled && !self.is_read_only;
        match (&self.on_change, editable) {
            (Some(on_change), true) => {
                // The value is clipped by its field and scrolls to the caret,
                // as a native input does, instead of truncating.
                let mut area = Node::new(Style {
                    flex_direction: FlexDirection::Row,
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    min_size: taffy::Size {
                        width: len_auto(0.0),
                        height: taffy::LengthPercentageAuto::auto(),
                    },
                    ..Style::default()
                });
                area.clip = Some(astryx_core::contract::Radii::default());
                cx.begin(area);
                text.style.flex_shrink = 0.0;
                let text_ix = cx.leaf(text);
                cx.end();
                let edit = Edit {
                    value: self.value.clone(),
                    on_change: on_change.clone(),
                    text_node: text_ix,
                    masked,
                    caret: color(c.text_primary),
                };
                if let Some(i) = cx.nodes[field_ix as usize].interact.as_mut() {
                    i.edit = Some(edit);
                }
            }
            _ => {
                text_draw(&mut text).max_lines = Some(1);
                text.style.flex_grow = 1.0;
                text.style.flex_shrink = 1.0;
                text.style.min_size.width = len_auto(0.0);
                cx.leaf(text);
            }
        }
        let loading = cx.t("@astryx.button.loading", &args! {});
        cx.live_region(self.is_loading.then_some(loading.as_str()), Live::Polite);
        if self.is_loading {
            let (d, s) = crate::primitives::SpinnerSize::Sm.geometry();
            let painter = spinner_painter(cx, d, s, color(c.text_secondary));
            cx.leaf(
                Node::new(Style {
                    size: square(d + 2.0 * s),
                    flex_shrink: 0.0,
                    ..Style::default()
                })
                .draw(Draw::Paint(painter)),
            );
        }
        if f.status_variant != StatusVariant::Attached
            || f.status.as_ref().is_some_and(|s| s.message.is_none())
        {
            if let Some((icon, ink)) = status_icon(cx, f) {
                if let Some(n) = icon_node::<Msg>(icon, 16.0, ink) {
                    cx.leaf(n);
                }
            }
        }
        let clears = self.has_clear && !self.value.is_empty() && editable;
        if let (true, Some(on_change)) = (clears, &self.on_change) {
            let words = cx.t(
                "@astryx.textInput.clearLabel",
                &args! {"label" => f.label.clone()},
            );
            clear_button(cx, on_change(String::new()), words);
        }
        cx.end();
        status_box(cx, f);
        cx.end();
    }
}

/// A field's clear (×) action: its own focus stop, named after the field.
pub(crate) fn clear_button<Msg>(cx: &mut BuildCx<Msg>, msg: Msg, name: String) {
    let t = cx.tokens;
    let c = &t.color;
    let pill = radii(t.radius.full);
    cx.begin(
        Node::new(Style {
            size: square(20.0),
            flex_shrink: 0.0,
            align_items: AlignItems::CENTER,
            justify_content: taffy::JustifyContent::CENTER,
            ..Style::default()
        })
        .interact(Interact::new(Some(msg), false))
        .indicator(Indicator::ring(
            color(c.accent),
            pill,
            Some(color(c.overlay_pressed)),
        ))
        .sem(Semantics::new(Role::Button, name)),
    );
    if let Some(n) = icon_node::<Msg>("close", 16.0, color(c.icon_secondary)) {
        cx.leaf(n);
    }
    cx.end();
}
