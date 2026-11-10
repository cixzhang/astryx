//! Astryx layers for the Rust stack.
//!
//! The core runtime owns the layer mechanism (stacking above the page,
//! placement after layout, Escape and outside-press dismissal, focus trap
//! and restore, an inert page behind modal layers, hover intent). This
//! crate gives components Astryx's policy on top of it:
//!
//! - [`placement`]: logical sides and alignment, flips, and the viewport
//!   gutter.
//! - [`Layer`]: popovers, dialogs, tooltips, and context menus as elements.
//! - [`Hover`]: hover intent with Astryx's delays, for tooltips and hover
//!   cards.
//!
//! ```
//! use astryx_core::{Button, Element, VStack};
//! use astryx_spike_layers::{Layer, placement::{Placement, Side, Align}};
//!
//! #[derive(Clone)]
//! enum Msg { Close }
//!
//! let view: Element<Msg> = VStack::new()
//!     .child(Button::new("Settings").id("settings"))
//!     .child(
//!         Layer::popover("settings", VStack::new().into())
//!             .placement(Placement::new(Side::Below, Align::Start).offset(4.0))
//!             .on_dismiss(|_| Msg::Close),
//!     )
//!     .into();
//! # let _ = view;
//! ```

pub mod placement;

use astryx_core::authoring::{Build, BuildCx, Node, Semantics};
use astryx_core::layer::{DismissReason, HoverSpec, LayerAnchor, LayerSpec};
use astryx_core::{Color, Element, Point, Role};
use placement::{Align, Placement, Side};
use std::rc::Rc;
use taffy::Style;

/// Astryx's tooltip delays: shown after the pointer rests 200 ms, hidden at
/// once when it leaves.
pub const TOOLTIP_ENTER_MS: f64 = 200.0;
pub const TOOLTIP_LEAVE_MS: f64 = 0.0;

/// What kind of layer, for its defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Anchored, non-modal; closes on Escape and outside presses; gives
    /// focus back to its opener.
    Popover,
    /// Centered, modal; traps focus; closes on Escape; gives focus back.
    Dialog,
    /// Anchored, never takes focus; closes on Escape.
    Tooltip,
    /// At a point, non-modal; traps focus like a menu; closes on Escape and
    /// outside presses; gives focus back.
    Menu,
}

/// A layer as an element: declare it where its opener is, and it draws
/// above the page.
pub struct Layer<Msg> {
    kind: Kind,
    anchor: LayerAnchor,
    placement: Placement,
    content: Element<Msg>,
    on_dismiss: Option<Rc<dyn Fn(DismissReason) -> Msg>>,
    modal: bool,
    trap_focus: bool,
    restore_focus: bool,
    light_dismiss: bool,
    escape_dismiss: bool,
    backdrop: Option<Color>,
    role: Role,
    label: Option<String>,
    /// The layer is as wide as its anchor (a combobox's list).
    match_anchor_width: bool,
    id: Option<String>,
}

impl<Msg: 'static> Layer<Msg> {
    fn new(kind: Kind, anchor: LayerAnchor, content: Element<Msg>) -> Self {
        let (modal, trap, restore, light, role, placement) = match kind {
            Kind::Popover => (
                false,
                false,
                true,
                true,
                Role::Dialog,
                Placement::new(Side::Below, Align::Start),
            ),
            Kind::Dialog => (true, true, true, false, Role::Dialog, Placement::default()),
            Kind::Tooltip => (
                false,
                false,
                false,
                false,
                Role::Tooltip,
                Placement::new(Side::Above, Align::Center),
            ),
            Kind::Menu => (
                false,
                true,
                true,
                true,
                Role::Menu,
                Placement::new(Side::Below, Align::Start),
            ),
        };
        Layer {
            kind,
            anchor,
            placement,
            content,
            on_dismiss: None,
            modal,
            trap_focus: trap,
            restore_focus: restore,
            light_dismiss: light,
            escape_dismiss: true,
            backdrop: None,
            role,
            label: None,
            match_anchor_width: false,
            id: None,
        }
    }

    /// A popover anchored to the element with `id` prop `anchor`.
    pub fn popover(anchor: &str, content: Element<Msg>) -> Self {
        Self::new(Kind::Popover, LayerAnchor::Name(anchor.into()), content)
    }

    /// A modal dialog centered in the viewport.
    pub fn dialog(content: Element<Msg>) -> Self {
        Self::new(Kind::Dialog, LayerAnchor::None, content)
    }

    /// A tooltip anchored to the element with `id` prop `anchor`.
    pub fn tooltip(anchor: &str, content: Element<Msg>) -> Self {
        Self::new(Kind::Tooltip, LayerAnchor::Name(anchor.into()), content)
    }

    /// A menu at a point, such as where a context menu was asked for.
    pub fn menu_at(at: Point, content: Element<Msg>) -> Self {
        Self::new(Kind::Menu, LayerAnchor::Point(at), content)
    }

    /// A menu anchored to the element with `id` prop `anchor`.
    pub fn menu(anchor: &str, content: Element<Msg>) -> Self {
        Self::new(Kind::Menu, LayerAnchor::Name(anchor.into()), content)
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    pub fn on_dismiss(mut self, f: impl Fn(DismissReason) -> Msg + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(f));
        self
    }

    pub fn modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self
    }

    pub fn trap_focus(mut self, trap: bool) -> Self {
        self.trap_focus = trap;
        self
    }

    pub fn light_dismiss(mut self, on: bool) -> Self {
        self.light_dismiss = on;
        self
    }

    pub fn escape_dismiss(mut self, on: bool) -> Self {
        self.escape_dismiss = on;
        self
    }

    pub fn restore_focus(mut self, on: bool) -> Self {
        self.restore_focus = on;
        self
    }

    /// Tints the page behind a modal layer.
    pub fn backdrop(mut self, color: Color) -> Self {
        self.backdrop = Some(color);
        self
    }

    pub fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn match_anchor_width(mut self, on: bool) -> Self {
        self.match_anchor_width = on;
        self
    }

    /// The layer root's `id` prop, so other layers can anchor to it.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}

impl<Msg: 'static> Build<Msg> for Layer<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let this = *self;
        let mut spec = LayerSpec::new(this.anchor.clone(), this.placement.place());
        spec.modal = this.modal;
        spec.trap_focus = this.trap_focus;
        spec.restore_focus = this.restore_focus;
        spec.light_dismiss = this.light_dismiss;
        spec.escape_dismiss = this.escape_dismiss;
        spec.on_dismiss = this.on_dismiss;
        spec.backdrop = this.backdrop;
        // The layer never grows past the viewport less its gutter.
        let gutter = this.placement.gutter;
        let (vw, vh) = cx.viewport;
        let anchor_width = match &this.anchor {
            LayerAnchor::Name(name) if this.match_anchor_width => {
                cx.last_rect(name).map(|r| r.width)
            }
            _ => None,
        };
        let role = this.role;
        let label = this.label;
        let modal = this.modal;
        let content = this.content;
        let from = match (this.kind, &this.anchor) {
            (Kind::Dialog, _) | (_, LayerAnchor::None) => astryx_spike_motion::From::Center,
            _ => match this.placement.side {
                Side::Above => astryx_spike_motion::From::Above,
                Side::Below => astryx_spike_motion::From::Below,
                Side::Start => astryx_spike_motion::From::Start,
                Side::End => astryx_spike_motion::From::End,
            },
        };
        let motion = astryx_spike_motion::tokens(cx);
        let rtl = cx.rtl();
        cx.key(this.id.as_deref());
        cx.layer(spec, move |cx| {
            let mut style = Style {
                flex_direction: taffy::FlexDirection::Column,
                max_size: taffy::Size {
                    width: taffy::LengthPercentageAuto::length((vw - 2.0 * gutter).max(0.0)),
                    height: taffy::LengthPercentageAuto::length((vh - 2.0 * gutter).max(0.0)),
                },
                ..Style::default()
            };
            if let Some(w) = anchor_width {
                style.min_size.width = taffy::LengthPercentageAuto::length(w);
            }
            let sem = Semantics {
                label,
                ..Semantics::role(role)
            }
            .with_state(|s| s.modal = modal);
            // Astryx's layer entrance and one exit copy on close.
            let mut node = Node::new(style).sem(sem);
            node.anims = astryx_spike_motion::layer_enter(motion, from, rtl);
            node.exit = Some(astryx_spike_motion::layer_exit(motion));
            cx.begin(node);
            cx.child(content);
            cx.end();
        });
    }
}

impl<Msg: 'static> From<Layer<Msg>> for Element<Msg> {
    fn from(layer: Layer<Msg>) -> Self {
        astryx_core::authoring::element(layer)
    }
}

/// Reports hover on its child after delays, as Astryx's hover recognizer
/// does: pointer rest for `enter_ms`, and leaving for `leave_ms`; moving
/// into a layer anchored to the child counts as staying. Keyboard focus
/// inside the child counts as hover at once.
pub struct Hover<Msg> {
    child: Element<Msg>,
    enter_ms: f64,
    leave_ms: f64,
    focus: bool,
    on_change: Rc<dyn Fn(bool) -> Msg>,
    id: Option<String>,
}

impl<Msg: 'static> Hover<Msg> {
    /// Tooltip timing: [`TOOLTIP_ENTER_MS`] and [`TOOLTIP_LEAVE_MS`].
    pub fn new(child: impl Into<Element<Msg>>, on_change: impl Fn(bool) -> Msg + 'static) -> Self {
        Hover {
            child: child.into(),
            enter_ms: TOOLTIP_ENTER_MS,
            leave_ms: TOOLTIP_LEAVE_MS,
            focus: true,
            on_change: Rc::new(on_change),
            id: None,
        }
    }

    pub fn delays(mut self, enter_ms: f64, leave_ms: f64) -> Self {
        self.enter_ms = enter_ms;
        self.leave_ms = leave_ms;
        self
    }

    pub fn focus(mut self, on: bool) -> Self {
        self.focus = on;
        self
    }

    /// The wrapper's `id` prop, so a layer can anchor to it.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}

impl<Msg: 'static> Build<Msg> for Hover<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let mut node = Node::new(Style {
            flex_direction: taffy::FlexDirection::Column,
            ..Style::default()
        });
        node.hover = Some(HoverSpec {
            enter_ms: self.enter_ms,
            leave_ms: self.leave_ms,
            focus: self.focus,
            on_change: self.on_change,
        });
        cx.key(self.id.as_deref());
        cx.begin(node);
        cx.child(self.child);
        cx.end();
    }
}

impl<Msg: 'static> From<Hover<Msg>> for Element<Msg> {
    fn from(h: Hover<Msg>) -> Self {
        astryx_core::authoring::element(h)
    }
}

#[cfg(test)]
mod tests;
