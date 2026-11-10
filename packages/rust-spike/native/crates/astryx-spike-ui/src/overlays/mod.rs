//! Wave 4: light overlays — Tooltip, InfoTip, HoverCard, Popover and
//! BottomSheet, on the infra lane's layers.
//!
//! These are controlled: the app keeps `is_open` and changes it from the
//! `on_open_change` message, as Astryx's controlled `isOpen` and
//! `onOpenChange` do. (Uncontrolled forms wait on local element state in
//! the runtime.)

use crate::layout::Padding;
use crate::primitives::icon_node;
use crate::util::{color, len, radii, rect};
use astryx_core::authoring::{
    text_node, type_style, Base, BoxDraw, Build, BuildCx, Draw, Indicator, Interact, LocalHandle,
    Node, Semantics,
};
use astryx_core::contract::{Color, FontFamily, Role};
use astryx_core::layer::{DismissReason, LayerAnchor, LayerSpec, PlaceInput};
use astryx_core::{Element, Point, Space};
use astryx_spike_layers::placement::{Align, Placement, Side};
use astryx_spike_layers::{Hover, Layer};
use std::rc::Rc;
use taffy::{AlignItems, Dimension, FlexDirection, JustifyContent, LengthPercentageAuto, Style};

pub use astryx_spike_layers::placement::{Align as OverlayAlign, Side as OverlaySide};

/// A surface drawn behind overlay content: ground, radius and shadow.
struct Surface<Msg> {
    ground: Color,
    radius: f32,
    shadow: &'static [astryx_tokens::ShadowLayer],
    padding: (f32, f32),
    max_width: Option<f32>,
    width: Option<f32>,
    child: Element<Msg>,
}

impl<Msg: 'static> Build<Msg> for Surface<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let mut style = Style {
            flex_direction: FlexDirection::Column,
            padding: taffy::Rect {
                left: len(self.padding.0),
                right: len(self.padding.0),
                top: len(self.padding.1),
                bottom: len(self.padding.1),
            },
            ..Style::default()
        };
        if let Some(w) = self.max_width {
            style.max_size.width = LengthPercentageAuto::length(w);
        }
        if let Some(w) = self.width {
            style.size.width = Dimension::length(w);
        }
        let r = radii(self.radius);
        let mut node = Node::new(style).draw(Draw::Box(BoxDraw {
            shadow: self.shadow,
            ..BoxDraw::fill(self.ground, r)
        }));
        node.clip = Some(r);
        cx.begin(node);
        cx.container_padding.push(self.padding.0);
        cx.child(self.child);
        cx.container_padding.pop();
        cx.end();
    }
}

fn surface<Msg: 'static>(s: Surface<Msg>) -> Element<Msg> {
    astryx_core::authoring::element(s)
}

/// Words in the tooltip bubble: inverted ground, body type.
struct TipBody(String);

impl<Msg: 'static> Build<Msg> for TipBody {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let mut words = text_node(
            self.0,
            type_style(t.font.body, FontFamily::Body),
            color(t.color.background_surface),
        );
        words.sem = None;
        cx.leaf(words);
    }
}

/// An anchor wrapper with an `id`, so a layer can find what it hangs from.
struct Anchor<Msg> {
    id: String,
    child: Element<Msg>,
}

impl<Msg: 'static> Build<Msg> for Anchor<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        cx.key(Some(&self.id));
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            align_self: Some(AlignItems::FLEX_START),
            ..Style::default()
        }));
        cx.child(self.child);
        cx.end();
    }
}

fn anchor<Msg: 'static>(id: &str, child: Element<Msg>) -> Element<Msg> {
    astryx_core::authoring::element(Anchor {
        id: id.to_string(),
        child,
    })
}

/// Open state: the controlled value when the caller gives one, else the
/// component's own state (starting at `default`).
pub(crate) fn open_state<Msg>(
    cx: &mut BuildCx<Msg>,
    owner: &str,
    controlled: Option<bool>,
    default: bool,
) -> (bool, LocalHandle<Option<bool>>) {
    cx.key(Some(owner));
    let h = cx.local_handle::<Option<bool>>();
    let open = controlled.unwrap_or_else(|| h.get().unwrap_or(default));
    (open, h)
}

/// A plain column node for an owner of local state.
pub(crate) fn owner_node<Msg>() -> Node<Msg> {
    Node::new(Style {
        flex_direction: FlexDirection::Column,
        align_self: Some(AlignItems::FLEX_START),
        ..Style::default()
    })
}

/// A short hint about its child, shown while the pointer rests on it or
/// keyboard focus is in it (`Tooltip`).
pub struct Tooltip<Msg> {
    id: String,
    child: Element<Msg>,
    content: String,
    placement: Side,
    alignment: Align,
    delay: f64,
    hide_delay: f64,
    is_enabled: bool,
    is_open: Option<bool>,
    is_default_open: bool,
    on_open_change: Option<Rc<dyn Fn(bool) -> Msg>>,
}

impl<Msg: 'static> Tooltip<Msg> {
    /// `id` names the anchor; it must be unique in the view.
    pub fn new(
        id: impl Into<String>,
        child: impl Into<Element<Msg>>,
        content: impl Into<String>,
    ) -> Self {
        Tooltip {
            id: id.into(),
            child: child.into(),
            content: content.into(),
            placement: Side::Above,
            alignment: Align::Center,
            delay: 200.0,
            hide_delay: 0.0,
            is_enabled: true,
            is_open: None,
            is_default_open: false,
            on_open_change: None,
        }
    }
    pub fn placement(mut self, side: Side) -> Self {
        self.placement = side;
        self
    }
    pub fn alignment(mut self, align: Align) -> Self {
        self.alignment = align;
        self
    }
    pub fn delay(mut self, ms: f64) -> Self {
        self.delay = ms;
        self
    }
    pub fn hide_delay(mut self, ms: f64) -> Self {
        self.hide_delay = ms;
        self
    }
    pub fn is_enabled(mut self, on: bool) -> Self {
        self.is_enabled = on;
        self
    }
    /// Controls it: the caller keeps the open state. Without this the
    /// tooltip opens and closes itself.
    pub fn is_open(mut self, open: bool) -> Self {
        self.is_open = Some(open);
        self
    }
    pub fn is_default_open(mut self, open: bool) -> Self {
        self.is_default_open = open;
        self
    }
    pub fn on_open_change(mut self, f: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_open_change = Some(Rc::new(f));
        self
    }
}

impl<Msg: 'static> Build<Msg> for Tooltip<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let owner = format!("{}:tooltip", self.id);
        let (open, h) = open_state(cx, &owner, self.is_open, self.is_default_open);
        cx.begin(owner_node());
        let child = match (&self.on_open_change, self.is_enabled, self.is_open) {
            (Some(on), true, Some(_)) => {
                let on = on.clone();
                Hover::new(self.child, move |o| on(o))
                    .delays(self.delay, self.hide_delay)
                    .into()
            }
            (_, true, None) => {
                let h = h.clone();
                Hover::local(self.child, move |o| h.set(Some(o)))
                    .delays(self.delay, self.hide_delay)
                    .into()
            }
            _ => self.child,
        };
        cx.child(anchor(&self.id, child));
        if open && self.is_enabled {
            let bubble = surface(Surface {
                ground: color(t.color.text_primary),
                radius: t.radius.container,
                shadow: &[],
                padding: (t.space.s2, t.space.s1),
                max_width: Some(300.0),
                width: None,
                child: astryx_core::authoring::element(TipBody(self.content.clone())),
            });
            let mut layer = Layer::tooltip(&self.id, bubble)
                .placement(Placement::new(self.placement, self.alignment).offset(t.space.s1))
                .label(self.content)
                .on_dismiss_effect(move |_| h.set(Some(false)));
            if let Some(on) = &self.on_open_change {
                let on = on.clone();
                layer = layer.on_dismiss(move |_| on(false));
            }
            cx.child(layer.into());
        }
        cx.end();
    }
}

impl<Msg: 'static> From<Tooltip<Msg>> for Element<Msg> {
    fn from(t: Tooltip<Msg>) -> Self {
        astryx_core::authoring::element(t)
    }
}

/// InfoTip `size`: the info icon's size.
pub use crate::primitives::IconSize as InfoTipSize;

/// An info icon that shows more words on hover, focus or press (`InfoTip`).
pub struct InfoTip<Msg> {
    id: String,
    content: String,
    label: String,
    size: InfoTipSize,
    is_open: Option<bool>,
    on_open_change: Option<Rc<dyn Fn(bool) -> Msg>>,
}

impl<Msg: 'static> InfoTip<Msg> {
    pub fn new(id: impl Into<String>, content: impl Into<String>) -> Self {
        InfoTip {
            id: id.into(),
            content: content.into(),
            label: "More information".into(),
            size: InfoTipSize::Sm,
            is_open: None,
            on_open_change: None,
        }
    }
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = l.into();
        self
    }
    pub fn size(mut self, s: InfoTipSize) -> Self {
        self.size = s;
        self
    }
    pub fn is_open(mut self, open: bool) -> Self {
        self.is_open = Some(open);
        self
    }
    pub fn on_open_change(mut self, f: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_open_change = Some(Rc::new(f));
        self
    }
}

/// The InfoTip's trigger: a round, focusable info icon.
struct InfoTrigger<Msg> {
    label: String,
    side: f32,
    describes: String,
    on_press: Option<Msg>,
    effect: Option<Rc<dyn Fn()>>,
}

impl<Msg: 'static> Build<Msg> for InfoTrigger<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let pill = radii(t.radius.full);
        let mut sem = Semantics::new(Role::Button, self.label);
        sem.description = Some(self.describes);
        cx.begin(
            Node::new(Style {
                padding: rect(len(t.space.s0_5)),
                align_items: AlignItems::CENTER,
                justify_content: JustifyContent::CENTER,
                flex_shrink: 0.0,
                ..Style::default()
            })
            .interact({
                let mut i = Interact::new(self.on_press, false);
                if let Some(e) = self.effect {
                    i = i.effect(move || e());
                }
                i
            })
            .indicator(Indicator::ring(
                color(t.color.accent),
                pill,
                Some(color(t.color.overlay_pressed)),
            ))
            .sem(sem),
        );
        if let Some(n) = icon_node::<Msg>("info", self.side, color(t.color.icon_secondary)) {
            cx.leaf(n);
        }
        cx.end();
    }
}

impl<Msg: Clone + 'static> Build<Msg> for InfoTip<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let this = *self;
        let owner = format!("{}:infotip", this.id);
        let (open, h) = open_state(cx, &owner, this.is_open, false);
        cx.begin(owner_node());
        let uncontrolled = this.is_open.is_none();
        let trigger = astryx_core::authoring::element(InfoTrigger {
            label: this.label,
            side: this.size.points(),
            describes: this.content.clone(),
            on_press: this.on_open_change.as_ref().map(|f| f(!open)),
            effect: uncontrolled.then(|| {
                let h = h.clone();
                Rc::new(move || h.set(Some(!open))) as Rc<dyn Fn()>
            }),
        });
        let mut tip = Tooltip::new(this.id, trigger, this.content).is_open(open);
        match this.on_open_change {
            Some(f) if !uncontrolled => tip = tip.on_open_change(move |o| f(o)),
            _ => {}
        }
        cx.child(tip.into());
        cx.end();
    }
}

impl<Msg: Clone + 'static> From<InfoTip<Msg>> for Element<Msg> {
    fn from(t: InfoTip<Msg>) -> Self {
        astryx_core::authoring::element(t)
    }
}

/// Rich content about its child on hover or focus, such as a person's card
/// (`HoverCard`). Waits 300 ms to open and 200 ms to close; moving into the
/// card keeps it open.
pub struct HoverCard<Msg> {
    id: String,
    child: Element<Msg>,
    content: Element<Msg>,
    placement: Side,
    alignment: Align,
    delay: f64,
    hide_delay: f64,
    is_enabled: bool,
    label: Option<String>,
    is_open: Option<bool>,
    on_open_change: Option<Rc<dyn Fn(bool) -> Msg>>,
}

impl<Msg: 'static> HoverCard<Msg> {
    pub fn new(
        id: impl Into<String>,
        child: impl Into<Element<Msg>>,
        content: impl Into<Element<Msg>>,
    ) -> Self {
        HoverCard {
            id: id.into(),
            child: child.into(),
            content: content.into(),
            placement: Side::Above,
            alignment: Align::Center,
            delay: 300.0,
            hide_delay: 200.0,
            is_enabled: true,
            label: None,
            is_open: None,
            on_open_change: None,
        }
    }
    pub fn placement(mut self, side: Side) -> Self {
        self.placement = side;
        self
    }
    pub fn alignment(mut self, align: Align) -> Self {
        self.alignment = align;
        self
    }
    pub fn delay(mut self, ms: f64) -> Self {
        self.delay = ms;
        self
    }
    pub fn hide_delay(mut self, ms: f64) -> Self {
        self.hide_delay = ms;
        self
    }
    pub fn is_enabled(mut self, on: bool) -> Self {
        self.is_enabled = on;
        self
    }
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }
    /// Controls it; without this it opens and closes itself.
    pub fn is_open(mut self, open: bool) -> Self {
        self.is_open = Some(open);
        self
    }
    pub fn on_open_change(mut self, f: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_open_change = Some(Rc::new(f));
        self
    }
}

impl<Msg: 'static> Build<Msg> for HoverCard<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let owner = format!("{}:hovercard", self.id);
        let (open, h) = open_state(cx, &owner, self.is_open, false);
        cx.begin(owner_node());
        let child = match (&self.on_open_change, self.is_enabled, self.is_open) {
            (Some(on), true, Some(_)) => {
                let on = on.clone();
                Hover::new(self.child, move |o| on(o))
                    .delays(self.delay, self.hide_delay)
                    .into()
            }
            (_, true, None) => {
                let h = h.clone();
                Hover::local(self.child, move |o| h.set(Some(o)))
                    .delays(self.delay, self.hide_delay)
                    .into()
            }
            _ => self.child,
        };
        cx.child(anchor(&self.id, child));
        if open && self.is_enabled {
            let card = surface(Surface {
                ground: color(t.color.background_surface),
                radius: t.radius.container,
                shadow: t.shadow.med,
                padding: (t.space.s3, t.space.s3),
                max_width: Some(360.0),
                width: None,
                child: self.content,
            });
            let mut layer = Layer::popover(&self.id, card)
                .placement(Placement::new(self.placement, self.alignment).offset(t.space.s1))
                .restore_focus(false)
                .role(Role::Dialog)
                .on_dismiss_effect(move |_| h.set(Some(false)));
            if let Some(l) = self.label {
                layer = layer.label(l);
            }
            if let Some(on) = &self.on_open_change {
                let on = on.clone();
                layer = layer.on_dismiss(move |_| on(false));
            }
            cx.child(layer.into());
        }
        cx.end();
    }
}

impl<Msg: 'static> From<HoverCard<Msg>> for Element<Msg> {
    fn from(t: HoverCard<Msg>) -> Self {
        astryx_core::authoring::element(t)
    }
}

/// Popover `width`: hug the content, match the trigger, or a fixed width.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum PopoverWidth {
    #[default]
    Auto,
    Trigger,
    Points(f32),
}

/// Content anchored to a trigger, opened by pressing it (`Popover`). It
/// closes on Escape and outside presses and gives focus back to the
/// trigger. The child is the trigger's content.
pub struct Popover<Msg> {
    base: Base,
    id: String,
    child: Element<Msg>,
    content: Element<Msg>,
    placement: Side,
    alignment: Align,
    is_open: Option<bool>,
    on_open_change: Option<Rc<dyn Fn(bool) -> Msg>>,
    is_enabled: bool,
    width: PopoverWidth,
    padding: Space,
    label: Option<String>,
    role: Role,
    is_modal: bool,
    has_light_dismiss: bool,
    has_escape_dismiss: bool,
}

impl<Msg: 'static> Popover<Msg> {
    pub fn new(
        id: impl Into<String>,
        child: impl Into<Element<Msg>>,
        content: impl Into<Element<Msg>>,
    ) -> Self {
        Popover {
            base: Base::default(),
            id: id.into(),
            child: child.into(),
            content: content.into(),
            placement: Side::Below,
            alignment: Align::Start,
            is_open: None,
            on_open_change: None,
            is_enabled: true,
            width: PopoverWidth::Auto,
            padding: Space::S3,
            label: None,
            role: Role::Dialog,
            is_modal: true,
            has_light_dismiss: true,
            has_escape_dismiss: true,
        }
    }
    pub fn placement(mut self, side: Side) -> Self {
        self.placement = side;
        self
    }
    pub fn alignment(mut self, align: Align) -> Self {
        self.alignment = align;
        self
    }
    /// Controls it; without this, pressing the trigger opens and closes it.
    pub fn is_open(mut self, open: bool) -> Self {
        self.is_open = Some(open);
        self
    }
    pub fn on_open_change(mut self, f: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_open_change = Some(Rc::new(f));
        self
    }
    pub fn is_enabled(mut self, on: bool) -> Self {
        self.is_enabled = on;
        self
    }
    pub fn width(mut self, w: PopoverWidth) -> Self {
        self.width = w;
        self
    }
    pub fn padding(mut self, p: Space) -> Self {
        self.padding = p;
        self
    }
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }
    /// `Dialog` (default) or `Group` for Astryx's `role="none"`.
    pub fn role(mut self, r: Role) -> Self {
        self.role = r;
        self
    }
    /// Focus stays in the popover while it is open.
    pub fn is_modal(mut self, on: bool) -> Self {
        self.is_modal = on;
        self
    }
    pub fn has_light_dismiss(mut self, on: bool) -> Self {
        self.has_light_dismiss = on;
        self
    }
    pub fn has_escape_dismiss(mut self, on: bool) -> Self {
        self.has_escape_dismiss = on;
        self
    }
}

/// The popover's trigger: presses toggle it, and it reads as a button with
/// a dialog popup.
struct Trigger<Msg> {
    child: Element<Msg>,
    open: bool,
    msg: Option<Msg>,
    effect: Option<Rc<dyn Fn()>>,
}

impl<Msg: 'static> Build<Msg> for Trigger<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let r = radii(t.radius.element);
        let mut interact = Interact::new(self.msg, false);
        if let Some(e) = self.effect {
            interact = interact.effect(move || e());
        }
        let open = self.open;
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Column,
                align_self: Some(AlignItems::FLEX_START),
                ..Style::default()
            })
            .interact(interact)
            .indicator(Indicator::ring(color(t.color.accent), r, None))
            .sem(Semantics::role(Role::Button).with_state(|s| {
                s.expanded = Some(open);
                s.has_popup = Some(astryx_core::contract::HasPopup::Dialog);
            })),
        );
        cx.child(self.child);
        cx.end();
    }
}

impl<Msg: Clone + 'static> Build<Msg> for Popover<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let owner = format!("{}:popover", self.id);
        let (open, h) = open_state(cx, &owner, self.is_open, false);
        cx.begin(owner_node());
        let uncontrolled = self.is_open.is_none();
        let toggle = {
            let h = h.clone();
            Rc::new(move || h.set(Some(!open))) as Rc<dyn Fn()>
        };
        let trigger = astryx_core::authoring::element(Trigger {
            child: self.child,
            open,
            msg: self.on_open_change.as_ref().map(|f| f(!open)),
            effect: (uncontrolled && self.is_enabled).then_some(toggle),
        });
        cx.child(anchor(&self.id, trigger));
        if open && self.is_enabled {
            let pad = Padding::all(self.padding).resolve(t).0;
            let card = surface(Surface {
                ground: color(t.color.background_popover),
                radius: t.radius.container,
                shadow: t.shadow.low,
                padding: (pad, pad),
                max_width: None,
                width: match self.width {
                    PopoverWidth::Points(w) => Some(w),
                    _ => None,
                },
                child: self.content,
            });
            let mut layer = Layer::popover(&self.id, card)
                .placement(Placement::new(self.placement, self.alignment).offset(t.space.s1))
                .role(self.role)
                .trap_focus(self.is_modal)
                .light_dismiss(self.has_light_dismiss)
                .escape_dismiss(self.has_escape_dismiss)
                .match_anchor_width(self.width == PopoverWidth::Trigger)
                .on_dismiss_effect(move |_| h.set(Some(false)));
            if let Some(l) = self.label {
                layer = layer.label(l);
            }
            if let Some(on) = &self.on_open_change {
                let on = on.clone();
                layer = layer.on_dismiss(move |_| on(false));
            }
            cx.child(layer.into());
        }
        cx.end();
    }
}

impl<Msg: Clone + 'static> From<Popover<Msg>> for Element<Msg> {
    fn from(p: Popover<Msg>) -> Self {
        astryx_core::authoring::element(p)
    }
}

/// BottomSheet `purpose`: how it may be dismissed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SheetPurpose {
    /// Must be answered: no outside-press or Escape dismissal.
    Required,
    /// A form: Escape closes, outside presses do not.
    Form,
    /// Information: Escape and outside presses close it.
    #[default]
    Info,
}

/// BottomSheet `height`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SheetHeight {
    /// As tall as its content.
    Hug,
    /// Up to half the viewport.
    #[default]
    Capped,
    /// Up to the viewport less a top gap.
    Tall,
    Points(f32),
}

/// A sheet that rises from the bottom edge (`BottomSheet`): modal, with a
/// scrim, a grab bar and up to 640 points wide.
pub struct BottomSheet<Msg> {
    label: String,
    is_open: bool,
    on_open_change: Option<Rc<dyn Fn(bool) -> Msg>>,
    purpose: SheetPurpose,
    padding: Option<Space>,
    height: SheetHeight,
    has_scrim: bool,
    children: Vec<Element<Msg>>,
}

impl<Msg> BottomSheet<Msg> {
    pub fn new(label: impl Into<String>, is_open: bool) -> Self {
        BottomSheet {
            label: label.into(),
            is_open,
            on_open_change: None,
            purpose: SheetPurpose::Info,
            padding: None,
            height: SheetHeight::Capped,
            has_scrim: true,
            children: Vec::new(),
        }
    }
    pub fn on_open_change(mut self, f: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_open_change = Some(Rc::new(f));
        self
    }
    pub fn purpose(mut self, p: SheetPurpose) -> Self {
        self.purpose = p;
        self
    }
    pub fn padding(mut self, p: Space) -> Self {
        self.padding = Some(p);
        self
    }
    pub fn height(mut self, h: SheetHeight) -> Self {
        self.height = h;
        self
    }
    pub fn has_scrim(mut self, on: bool) -> Self {
        self.has_scrim = on;
        self
    }
    pub fn child(mut self, c: impl Into<Element<Msg>>) -> Self {
        self.children.push(c.into());
        self
    }
}

const SHEET_MAX_WIDTH: f32 = 640.0;
const GRAB_BAR: (f32, f32) = (36.0, 4.0);

impl<Msg: Clone + 'static> Build<Msg> for BottomSheet<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if !self.is_open {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let (vw, vh) = cx.viewport;
        // Before the first layout the viewport is unknown; assume a phone.
        let vh = if vh > 0.0 { vh } else { 800.0 };
        // The viewport is (0, 0) before the first layout.
        let width = if vw > 0.0 {
            vw.min(SHEET_MAX_WIDTH)
        } else {
            SHEET_MAX_WIDTH
        };
        let max_h = match self.height {
            SheetHeight::Hug => vh - t.space.s10,
            SheetHeight::Capped => (vh * 0.5).max(240.0).min(vh - t.space.s10),
            SheetHeight::Tall => vh - t.space.s10,
            SheetHeight::Points(h) => h.min(vh),
        };
        let fixed = matches!(self.height, SheetHeight::Tall | SheetHeight::Points(_));
        let mut spec = LayerSpec::new(
            LayerAnchor::None,
            Rc::new(|input: &PlaceInput| Point {
                x: input.viewport.x + (input.viewport.width - input.size.width).max(0.0) / 2.0,
                y: input.viewport.y + input.viewport.height - input.size.height,
            }),
        );
        spec.modal = true;
        spec.trap_focus = true;
        spec.restore_focus = true;
        spec.escape_dismiss = self.purpose != SheetPurpose::Required;
        spec.light_dismiss = self.purpose == SheetPurpose::Info;
        spec.backdrop = self.has_scrim.then(|| color(c.overlay));
        if let Some(on) = &self.on_open_change {
            let on = on.clone();
            spec.on_dismiss = Some(Rc::new(move |_: DismissReason| on(false)));
        }
        let pad = self.padding.map_or(t.space.s4, |p| p.points(t));
        let label = self.label;
        let children = self.children;
        let r = t.radius.container;
        let bw = t.size.border_width;
        let border = color(c.border);
        let ground = color(c.background_surface);
        let shadow = t.shadow.high;
        let bar = color(c.border_emphasized);
        cx.layer(spec, move |cx| {
            let radius = astryx_core::contract::Radii([r, r, 0.0, 0.0]);
            let mut node = Node::new(Style {
                flex_direction: FlexDirection::Column,
                size: taffy::Size {
                    width: Dimension::length(width),
                    height: if fixed {
                        Dimension::length(max_h)
                    } else {
                        Dimension::auto()
                    },
                },
                max_size: taffy::Size {
                    width: LengthPercentageAuto::length(SHEET_MAX_WIDTH),
                    height: LengthPercentageAuto::length(max_h),
                },
                border: taffy::Rect {
                    left: len(bw),
                    right: len(bw),
                    top: len(bw),
                    bottom: len(0.0),
                },
                ..Style::default()
            })
            .draw(Draw::Box(BoxDraw {
                border: Some((border, bw)),
                shadow,
                ..BoxDraw::fill(ground, radius)
            }))
            .sem(Semantics::new(Role::Dialog, label).with_state(|s| s.modal = true));
            node.clip = Some(radius);
            cx.begin(node);
            // The grab bar.
            cx.begin(Node::new(Style {
                align_items: AlignItems::CENTER,
                justify_content: JustifyContent::CENTER,
                padding: taffy::Rect {
                    left: len(0.0),
                    right: len(0.0),
                    top: len(8.0),
                    bottom: len(4.0),
                },
                flex_shrink: 0.0,
                ..Style::default()
            }));
            cx.leaf(
                Node::new(Style {
                    size: taffy::Size {
                        width: Dimension::length(GRAB_BAR.0),
                        height: Dimension::length(GRAB_BAR.1),
                    },
                    ..Style::default()
                })
                .draw(Draw::Box(BoxDraw::fill(bar, radii(GRAB_BAR.1 / 2.0)))),
            );
            cx.end();
            let mut body = Node::new(Style {
                flex_direction: FlexDirection::Column,
                padding: taffy::Rect {
                    left: len(pad),
                    right: len(pad),
                    top: len((pad - 16.0).max(0.0)),
                    bottom: len(pad),
                },
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_size: taffy::Size {
                    width: LengthPercentageAuto::length(0.0),
                    height: LengthPercentageAuto::length(0.0),
                },
                overflow: taffy::Point {
                    x: taffy::Overflow::Visible,
                    y: taffy::Overflow::Scroll,
                },
                ..Style::default()
            });
            body.scroll = Some(astryx_core::authoring::Axis::Vertical);
            cx.begin(body);
            cx.container_padding.push(pad);
            for ch in children {
                cx.child(ch);
            }
            cx.container_padding.pop();
            cx.end();
            cx.end();
        });
    }
}

impl<Msg: Clone + 'static> From<BottomSheet<Msg>> for Element<Msg> {
    fn from(s: BottomSheet<Msg>) -> Self {
        astryx_core::authoring::element(s)
    }
}
