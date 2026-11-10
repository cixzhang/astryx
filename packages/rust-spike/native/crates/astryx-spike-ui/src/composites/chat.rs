//! Wave 9: the chat composer — ChatComposer, ChatComposerInput,
//! ChatSendButton and ChatComposerDrawer.

use crate::controls::{Button, ButtonVariant, ListDensity};
use crate::primitives::Icon;
use crate::util::{color, len, len_auto, radii};
use astryx_core::authoring::{
    text_node, type_style, BoxDraw, Build, BuildCx, Draw, Edit, Indicator, Interact, KeyResponse,
    LocalHandle, Node, Semantics,
};
use astryx_core::contract::{FontFamily, Role};
use astryx_core::{Element, InputPhase, Key, KeyEvent};
use astryx_spike_foundations::{args, ComponentSize, CxExt};
use std::rc::Rc;
use taffy::{AlignItems, Dimension, FlexDirection, JustifyContent, LengthPercentageAuto, Style};

/// The send button (`ChatSendButton`): sends, or stops a reply that is
/// streaming.
pub struct ChatSendButton<Msg> {
    is_stop_shown: bool,
    is_disabled: bool,
    on_send: Option<Msg>,
    on_stop: Option<Msg>,
    send_effect: Option<Rc<dyn Fn()>>,
    size: ComponentSize,
}

impl<Msg> Default for ChatSendButton<Msg> {
    fn default() -> Self {
        ChatSendButton {
            is_stop_shown: false,
            is_disabled: false,
            on_send: None,
            on_stop: None,
            send_effect: None,
            size: ComponentSize::Md,
        }
    }
}

impl<Msg> ChatSendButton<Msg> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn is_stop_shown(mut self, on: bool) -> Self {
        self.is_stop_shown = on;
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    pub fn on_send(mut self, m: Msg) -> Self {
        self.on_send = Some(m);
        self
    }
    pub fn on_stop(mut self, m: Msg) -> Self {
        self.on_stop = Some(m);
        self
    }
    pub fn size(mut self, s: ComponentSize) -> Self {
        self.size = s;
        self
    }
}

impl<Msg: Clone + 'static> From<ChatSendButton<Msg>> for Element<Msg> {
    fn from(b: ChatSendButton<Msg>) -> Self {
        let (label, icon, msg) = if b.is_stop_shown {
            ("Stop", "stop", b.on_stop)
        } else {
            ("Send", "arrowUp", b.on_send)
        };
        let mut btn = Button::new(label)
            .variant(ButtonVariant::Primary)
            .size(b.size)
            .icon(Icon::new(icon))
            .is_icon_only(true)
            .is_disabled(b.is_disabled && !b.is_stop_shown);
        if let Some(m) = msg {
            btn = btn.on_click(m);
        }
        if let (Some(e), false) = (b.send_effect, b.is_stop_shown) {
            btn = btn.effect(e);
        }
        btn.into()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Draft {
    text: Option<String>,
}

/// A message box for a conversation (`ChatComposer`): a growing text area
/// (Enter sends, Shift+Enter adds a line), header and footer actions, the
/// send or stop button, an optional drawer of attachments, and a status
/// line.
pub struct ChatComposer<Msg> {
    id: String,
    on_submit: Rc<dyn Fn(String) -> Msg>,
    on_stop: Option<Msg>,
    is_stop_shown: bool,
    value: Option<String>,
    on_change: Option<Rc<dyn Fn(String) -> Msg>>,
    placeholder: Option<String>,
    is_disabled: bool,
    density: ListDensity,
    elevated: bool,
    drawer: Option<Element<Msg>>,
    header_actions: Vec<Element<Msg>>,
    header_context: Option<Element<Msg>>,
    footer_actions: Vec<Element<Msg>>,
    status: Option<crate::controls::Status>,
    label: Option<String>,
}

impl<Msg> ChatComposer<Msg> {
    pub fn new(id: impl Into<String>, on_submit: impl Fn(String) -> Msg + 'static) -> Self {
        ChatComposer {
            id: id.into(),
            on_submit: Rc::new(on_submit),
            on_stop: None,
            is_stop_shown: false,
            value: None,
            on_change: None,
            placeholder: None,
            is_disabled: false,
            density: ListDensity::Balanced,
            elevated: true,
            drawer: None,
            header_actions: Vec::new(),
            header_context: None,
            footer_actions: Vec::new(),
            status: None,
            label: None,
        }
    }
    pub fn on_stop(mut self, m: Msg) -> Self {
        self.on_stop = Some(m);
        self
    }
    pub fn is_stop_shown(mut self, on: bool) -> Self {
        self.is_stop_shown = on;
        self
    }
    /// Controls the text; without it the composer keeps its own draft.
    pub fn value(mut self, v: impl Into<String>) -> Self {
        self.value = Some(v.into());
        self
    }
    pub fn on_change(mut self, f: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = Some(p.into());
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    pub fn density(mut self, d: ListDensity) -> Self {
        self.density = d;
        self
    }
    /// `elevation`: low (the default) or none.
    pub fn is_elevated(mut self, on: bool) -> Self {
        self.elevated = on;
        self
    }
    pub fn drawer(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.drawer = Some(e.into());
        self
    }
    pub fn header_action(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.header_actions.push(e.into());
        self
    }
    pub fn header_context(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.header_context = Some(e.into());
        self
    }
    pub fn footer_action(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.footer_actions.push(e.into());
        self
    }
    pub fn status(mut self, s: crate::controls::Status) -> Self {
        self.status = Some(s);
        self
    }
    /// The text area's name (`Message input`).
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }
}

impl<Msg: Clone + 'static> From<ChatComposer<Msg>> for Element<Msg> {
    fn from(c: ChatComposer<Msg>) -> Self {
        astryx_core::authoring::element(c)
    }
}

fn row<Msg>(gap: f32, justify: JustifyContent) -> Node<Msg> {
    Node::new(Style {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::CENTER,
        justify_content: justify,
        gap: taffy::Size {
            width: len(gap),
            height: len(0.0),
        },
        ..Style::default()
    })
}

impl<Msg: Clone + 'static> Build<Msg> for ChatComposer<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let c = &t.color;
        cx.key(Some(&self.id));
        let h: LocalHandle<Draft> = cx.local_handle();
        let text = self.value.clone().or(h.get().text).unwrap_or_default();
        let controlled = self.value.is_some();
        let pad = match self.density {
            ListDensity::Compact => t.space.s2,
            ListDensity::Balanced => t.space.s3,
            ListDensity::Spacious => t.space.s4,
        };
        let radius = t.radius.chat;
        let r = radii(radius);
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            ..Style::default()
        }));
        // The body.
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Column,
                gap: taffy::Size {
                    width: len(0.0),
                    height: len(t.space.s2),
                },
                padding: crate::util::rect(len(pad)),
                border: crate::util::rect(len(t.size.border_width)),
                ..Style::default()
            })
            .draw(Draw::Box(BoxDraw {
                shadow: if self.elevated { t.shadow.low } else { &[] },
                border: Some((color(c.border), t.size.border_width)),
                ..BoxDraw::fill(color(c.background_popover), r)
            }))
            .opacity(if self.is_disabled { 0.6 } else { 1.0 }),
        );
        if let Some(d) = self.drawer {
            cx.child(d);
        }
        if self.header_context.is_some() || !self.header_actions.is_empty() {
            let mut hn = row(t.space.s2, JustifyContent::SPACE_BETWEEN);
            hn.style.min_size.height = len_auto(28.0);
            cx.begin(hn);
            cx.begin(row(t.space.s1, JustifyContent::FLEX_START));
            if let Some(hc) = self.header_context {
                cx.child(hc);
            }
            cx.end();
            cx.begin(row(t.space.s2, JustifyContent::FLEX_END));
            for a in self.header_actions {
                cx.child(a);
            }
            cx.end();
            cx.end();
        }
        // The text area: Enter sends, Shift+Enter adds a line.
        let submit = {
            let h = h.clone();
            let on_submit = self.on_submit.clone();
            let text = text.clone();
            let stop = self.is_stop_shown;
            let disabled = self.is_disabled;
            Rc::new(move |e: &KeyEvent| -> KeyResponse<Msg> {
                if e.phase != InputPhase::Pressed || e.key != Key::Enter || disabled {
                    return KeyResponse::Pass;
                }
                if e.modifiers.shift {
                    let mut d = h.get();
                    let mut s = d.text.unwrap_or_else(|| text.clone());
                    s.push('\n');
                    d.text = Some(s);
                    h.set(d);
                    return KeyResponse::Handled;
                }
                if stop || text.trim().is_empty() {
                    return KeyResponse::Handled;
                }
                if !controlled {
                    h.set(Draft::default());
                }
                KeyResponse::Send(on_submit(text.clone()))
            })
        };
        let label = self
            .label
            .clone()
            .unwrap_or_else(|| cx.t("@astryx.chat.composerInput.label", &args! {}));
        let placeholder = self
            .placeholder
            .clone()
            .unwrap_or_else(|| cx.t("@astryx.chat.composer.placeholder", &args! {}));
        let mut sem = Semantics::new(Role::TextInput, label);
        sem.value = Some(text.clone());
        sem.disabled = self.is_disabled;
        let mut area = Node::new(Style {
            flex_direction: FlexDirection::Column,
            max_size: taffy::Size {
                width: LengthPercentageAuto::auto(),
                height: LengthPercentageAuto::length(176.0),
            },
            overflow: taffy::Point {
                x: taffy::Overflow::Clip,
                y: taffy::Overflow::Scroll,
            },
            ..Style::default()
        })
        .interact(Interact::new(None, self.is_disabled))
        .indicator(Indicator {
            ring: None,
            radii: radii((radius - pad).max(t.radius.element)),
            overlay: None,
            field: None,
            hold_fill: None,
        })
        .sem(sem);
        area.on_key = Some(submit);
        area.scroll = Some(astryx_core::authoring::Axis::Vertical);
        cx.key(Some("input"));
        let area_ix = cx.begin(area);
        let body = type_style(t.font.body, FontFamily::Body);
        let mut words = if text.is_empty() {
            text_node(placeholder, body, color(c.text_disabled))
        } else {
            text_node(text.clone(), body, color(c.text_primary))
        };
        words.sem = None;
        words.style.size.width = Dimension::percent(1.0);
        let text_ix = cx.leaf(words);
        cx.end();
        if !self.is_disabled {
            let effect = {
                let h = h.clone();
                Rc::new(move |s: String| h.set(Draft { text: Some(s) })) as Rc<dyn Fn(String)>
            };
            if let Some(i) = cx.nodes[area_ix as usize].interact.as_mut() {
                i.edit = Some(Edit {
                    value: text.clone(),
                    on_change: self.on_change.clone(),
                    effect: (!controlled).then_some(effect),
                    text_node: text_ix,
                    masked: false,
                    multiline: true,
                    caret: color(c.accent),
                });
            }
        }
        // The footer: actions, then send.
        let mut foot = row(t.space.s2, JustifyContent::SPACE_BETWEEN);
        foot.style.min_size.height = len_auto(32.0);
        cx.begin(foot);
        cx.begin(row(t.space.s1, JustifyContent::FLEX_START));
        for a in self.footer_actions {
            cx.child(a);
        }
        cx.end();
        cx.begin(row(t.space.s1, JustifyContent::FLEX_END));
        let mut send = ChatSendButton::new()
            .is_stop_shown(self.is_stop_shown)
            .is_disabled(self.is_disabled || text.trim().is_empty())
            .size(ComponentSize::Sm)
            .on_send((self.on_submit)(text.clone()));
        if !controlled {
            let h = h.clone();
            send.send_effect = Some(Rc::new(move || h.set(Draft::default())));
        }
        if let Some(m) = self.on_stop {
            send = send.on_stop(m);
        }
        cx.child(send.into());
        cx.end();
        cx.end();
        cx.end();
        if let Some(st) = self.status {
            let (ground, ink) = match st.status_type {
                crate::controls::StatusType::Error => (c.error_muted, c.on_error_muted),
                _ => (c.warning_muted, c.on_warning_muted),
            };
            let mut n = row(t.space.s2, JustifyContent::FLEX_START);
            n.style.padding = taffy::Rect {
                left: len(t.space.s4),
                right: len(t.space.s4),
                top: len(radius + t.space.s1),
                bottom: len(t.space.s2),
            };
            n.style.margin.top = len_auto(-radius);
            n = n.draw(Draw::Box(BoxDraw::fill(
                color(ground),
                astryx_core::contract::Radii([0.0, 0.0, radius, radius]),
            )));
            cx.begin(n);
            let mut m = text_node(
                st.message.unwrap_or_default(),
                type_style(t.font.supporting, FontFamily::Body),
                color(ink),
            );
            m.sem = Some(Semantics::new(Role::Status, "status"));
            cx.leaf(m);
            cx.end();
        }
        cx.end();
    }
}

/// Attachments above the text (`ChatComposerDrawer`): a count, a label,
/// and items that fold away.
pub struct ChatComposerDrawer<Msg> {
    label: String,
    count: Option<usize>,
    default_is_collapsed: bool,
    children: Vec<Element<Msg>>,
}

impl<Msg> ChatComposerDrawer<Msg> {
    pub fn new() -> Self {
        ChatComposerDrawer {
            label: "Items".into(),
            count: None,
            default_is_collapsed: false,
            children: Vec::new(),
        }
    }
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = l.into();
        self
    }
    pub fn count(mut self, n: usize) -> Self {
        self.count = Some(n);
        self
    }
    pub fn default_is_collapsed(mut self, on: bool) -> Self {
        self.default_is_collapsed = on;
        self
    }
    pub fn child(mut self, c: impl Into<Element<Msg>>) -> Self {
        self.children.push(c.into());
        self
    }
}

impl<Msg> Default for ChatComposerDrawer<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: Clone + 'static> From<ChatComposerDrawer<Msg>> for Element<Msg> {
    fn from(d: ChatComposerDrawer<Msg>) -> Self {
        let n = d.count.unwrap_or(d.children.len());
        let mut c = super::Collapsible::new(format!("{} ({n})", d.label))
            .default_is_open(!d.default_is_collapsed);
        let mut items = crate::layout::HStack::new()
            .gap(astryx_core::Space::S2)
            .wrap(crate::layout::StackWrap::Wrap);
        for ch in d.children {
            items = items.child(ch);
        }
        c = c.child(items);
        c.into()
    }
}
