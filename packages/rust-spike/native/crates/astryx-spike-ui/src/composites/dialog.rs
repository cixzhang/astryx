//! Dialog and DialogHeader.

use crate::controls::{Button, ButtonVariant};
use crate::layout::{LayoutHeader, Padding};
use crate::primitives::{Icon, Text, TextColor};
use crate::util::{color, len, len_auto, radii};
use astryx_core::authoring::{
    text_node, type_style, BoxDraw, Build, BuildCx, Draw, Node, Semantics,
};
use astryx_core::contract::{FontFamily, Role};
use astryx_core::{Element, Space};
use astryx_spike_foundations::{args, CxExt};
use astryx_spike_layers::Layer;
use std::rc::Rc;
use taffy::{AlignItems, Dimension, FlexDirection, LengthPercentageAuto, Style};

/// Dialog `variant`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DialogVariant {
    #[default]
    Standard,
    Fullscreen,
}

/// Dialog `purpose`: how it may be dismissed.
pub use crate::overlays::SheetPurpose as DialogPurpose;

/// A modal window over the page (`Dialog`): focus is trapped inside and
/// returns to the opener when it closes. Compose its content with a
/// `Layout` whose header is a [`DialogHeader`].
pub struct Dialog<Msg> {
    is_open: bool,
    on_open_change: Rc<dyn Fn(bool) -> Msg>,
    width: f32,
    max_height: Option<f32>,
    variant: DialogVariant,
    purpose: DialogPurpose,
    padding: Option<Space>,
    label: Option<String>,
    is_inline: bool,
    children: Vec<Element<Msg>>,
}

impl<Msg> Dialog<Msg> {
    pub fn new(is_open: bool, on_open_change: impl Fn(bool) -> Msg + 'static) -> Self {
        Dialog {
            is_open,
            on_open_change: Rc::new(on_open_change),
            width: 400.0,
            max_height: None,
            variant: DialogVariant::Standard,
            purpose: DialogPurpose::Info,
            padding: None,
            label: None,
            is_inline: false,
            children: Vec::new(),
        }
    }
    /// Draws the dialog in the page flow, not as a modal layer (for
    /// documentation previews).
    pub fn is_inline(mut self, on: bool) -> Self {
        self.is_inline = on;
        self
    }
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// The tallest the dialog grows, in points; 75% of the viewport by
    /// default.
    pub fn max_height(mut self, h: f32) -> Self {
        self.max_height = Some(h);
        self
    }
    pub fn variant(mut self, v: DialogVariant) -> Self {
        self.variant = v;
        self
    }
    pub fn purpose(mut self, p: DialogPurpose) -> Self {
        self.purpose = p;
        self
    }
    pub fn padding(mut self, p: Space) -> Self {
        self.padding = Some(p);
        self
    }
    /// The dialog's accessible name (its header title names it otherwise).
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }
    pub fn child(mut self, c: impl Into<Element<Msg>>) -> Self {
        self.children.push(c.into());
        self
    }
}

struct DialogSurface<Msg> {
    width: f32,
    max_height: Option<f32>,
    fullscreen: bool,
    padding: f32,
    children: Vec<Element<Msg>>,
}

impl<Msg: 'static> Build<Msg> for DialogSurface<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let (vw, vh) = cx.viewport;
        let (vw, vh) = (
            if vw > 0.0 { vw } else { 1280.0 },
            if vh > 0.0 { vh } else { 800.0 },
        );
        let (w, max_h, r) = if self.fullscreen {
            (vw, vh, 0.0)
        } else {
            (
                self.width.min(vw - 2.0 * t.space.s4),
                self.max_height.unwrap_or(vh * 0.75),
                t.radius.container,
            )
        };
        let rr = radii(r);
        let mut node = Node::new(Style {
            flex_direction: FlexDirection::Column,
            size: taffy::Size {
                width: Dimension::length(w),
                height: if self.fullscreen {
                    Dimension::length(vh)
                } else {
                    Dimension::auto()
                },
            },
            max_size: taffy::Size {
                width: LengthPercentageAuto::auto(),
                height: LengthPercentageAuto::length(max_h),
            },
            padding: crate::util::rect(len(self.padding)),
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw {
            shadow: if self.fullscreen { &[] } else { t.shadow.high },
            ..BoxDraw::fill(color(t.color.background_surface), rr)
        }));
        node.clip = Some(rr);
        cx.begin(node);
        cx.container_padding.push(self.padding);
        for c in self.children {
            cx.child(c);
        }
        cx.container_padding.pop();
        cx.end();
    }
}

impl<Msg: Clone + 'static> Build<Msg> for Dialog<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if !self.is_open {
            return;
        }
        let t = cx.tokens;
        let pad = self.padding.map_or(0.0, |p| Padding::all(p).resolve(t).0);
        let surface = astryx_core::authoring::element(DialogSurface {
            width: self.width,
            max_height: self.max_height,
            fullscreen: self.variant == DialogVariant::Fullscreen,
            padding: pad,
            children: self.children,
        });
        if self.is_inline {
            let role = if self.purpose == DialogPurpose::Required {
                Role::AlertDialog
            } else {
                Role::Dialog
            };
            let sem = match self.label {
                Some(l) => Semantics::new(role, l),
                None => Semantics::role(role),
            };
            cx.begin(Node::new(Style::default()).sem(sem));
            cx.child(surface);
            cx.end();
            return;
        }
        let on = self.on_open_change.clone();
        let mut layer = Layer::dialog(surface)
            .backdrop(color(t.color.overlay))
            .escape_dismiss(self.purpose != DialogPurpose::Required)
            .light_dismiss(self.purpose == DialogPurpose::Info)
            .role(if self.purpose == DialogPurpose::Required {
                Role::AlertDialog
            } else {
                Role::Dialog
            })
            .on_dismiss(move |_| on(false));
        if let Some(l) = self.label {
            layer = layer.label(l);
        }
        cx.child(layer.into());
    }
}

impl<Msg: Clone + 'static> From<Dialog<Msg>> for Element<Msg> {
    fn from(d: Dialog<Msg>) -> Self {
        astryx_core::authoring::element(d)
    }
}

/// A dialog's title row (`DialogHeader`): a title, an optional subtitle,
/// start and end content, and a close button when `on_open_change` is set.
/// Use it as a Layout header: `Layout::new().header(header.into())`.
pub struct DialogHeader<Msg> {
    title: Option<String>,
    subtitle: Option<String>,
    on_open_change: Option<Rc<dyn Fn(bool) -> Msg>>,
    start_content: Option<Element<Msg>>,
    end_content: Option<Element<Msg>>,
    has_divider: bool,
}

impl<Msg> Default for DialogHeader<Msg> {
    fn default() -> Self {
        DialogHeader {
            title: None,
            subtitle: None,
            on_open_change: None,
            start_content: None,
            end_content: None,
            has_divider: true,
        }
    }
}

impl<Msg> DialogHeader<Msg> {
    pub fn new(title: impl Into<String>) -> Self {
        DialogHeader {
            title: Some(title.into()),
            ..Self::default()
        }
    }
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = Some(s.into());
        self
    }
    /// Shows the close button, which sends `f(false)`.
    pub fn on_open_change(mut self, f: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_open_change = Some(Rc::new(f));
        self
    }
    pub fn start_content(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.start_content = Some(e.into());
        self
    }
    pub fn end_content(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.end_content = Some(e.into());
        self
    }
    pub fn has_divider(mut self, on: bool) -> Self {
        self.has_divider = on;
        self
    }
}

struct HeaderRow<Msg> {
    title: Option<String>,
    subtitle: Option<String>,
    close: Option<Msg>,
    start: Option<Element<Msg>>,
    end: Option<Element<Msg>>,
}

impl<Msg: Clone + 'static> Build<Msg> for HeaderRow<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::FLEX_START,
            justify_content: taffy::JustifyContent::SPACE_BETWEEN,
            gap: taffy::Size {
                width: len(t.space.s3),
                height: len(0.0),
            },
            ..Style::default()
        }));
        if let Some(s) = self.start {
            cx.child(s);
        }
        // The title centers on the close button's midline.
        let h2 = t.font.heading2;
        let margin = (t.size.element_md / 2.0 - t.space.s2 - h2.size * h2.leading / 2.0).max(0.0);
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_size: taffy::Size {
                width: len_auto(0.0),
                height: LengthPercentageAuto::auto(),
            },
            margin: taffy::Rect {
                left: len_auto(0.0),
                right: len_auto(0.0),
                top: len_auto(margin),
                bottom: len_auto(margin),
            },
            ..Style::default()
        }));
        if let Some(title) = self.title {
            let style = type_style(h2, FontFamily::Heading);
            let mut n = text_node(title.clone(), style, color(t.color.text_primary));
            n.sem = Some(Semantics::new(Role::Heading(2), title));
            cx.leaf(n);
        }
        if let Some(s) = self.subtitle {
            cx.child(Text::new(s).color(TextColor::Secondary).into());
        }
        cx.end();
        if self.end.is_some() || self.close.is_some() {
            cx.begin(Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                gap: taffy::Size {
                    width: len(t.space.s2),
                    height: len(0.0),
                },
                flex_shrink: 0.0,
                margin: taffy::Rect {
                    left: len_auto(0.0),
                    right: len_auto(-t.space.s2),
                    top: len_auto(-t.space.s2),
                    bottom: len_auto(-t.space.s2),
                },
                ..Style::default()
            }));
            if let Some(e) = self.end {
                cx.child(e);
            }
            if let Some(msg) = self.close {
                let label = cx.t("@astryx.dialog.close", &args! {});
                cx.child(
                    Button::new(label.clone())
                        .variant(ButtonVariant::Ghost)
                        .icon(Icon::new("close"))
                        .is_icon_only(true)
                        .tooltip(label)
                        .on_click(msg)
                        .into(),
                );
            }
            cx.end();
        }
        cx.end();
    }
}

impl<Msg: Clone + 'static> From<DialogHeader<Msg>> for LayoutHeader<Msg> {
    fn from(h: DialogHeader<Msg>) -> Self {
        let row = astryx_core::authoring::element(HeaderRow {
            title: h.title,
            subtitle: h.subtitle,
            close: h.on_open_change.as_ref().map(|f| f(false)),
            start: h.start_content,
            end: h.end_content,
        });
        LayoutHeader::new().has_divider(h.has_divider).child(row)
    }
}

#[allow(dead_code)]
fn _unused<Msg>(cx: &BuildCx<Msg>) {
    let _ = BoxDraw::fill(color(cx.tokens.color.border), radii(0.0));
}
