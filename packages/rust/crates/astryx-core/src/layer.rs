//! Layers: content drawn above the page (popovers, menus, tooltips,
//! dialogs, sheets), placed against an anchor after layout, with
//! dismissal, focus trapping and restoring, and an inert page behind
//! modal layers.
//!
//! The runtime owns the mechanism; a component decides when it opens and
//! closes and how it looks. [`LayerSpec::place`] decides where the layer
//! goes, so placement policy (sides, flipping, viewport gutter) lives with
//! the component library, not in the runtime.

use crate::contract::{Point, Rect, Size};
use std::rc::Rc;

/// What a layer is placed against.
#[derive(Clone, Debug, PartialEq)]
pub enum LayerAnchor {
    /// No anchor: `place` gets only the viewport (dialogs, sheets).
    None,
    /// The element with this `id` prop.
    Name(String),
    /// The node with this accessibility id.
    Id(u64),
    /// A point in the viewport, such as where a context menu was asked for.
    Point(Point),
}

/// Why a layer asks to close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DismissReason {
    /// Escape, or B on a gamepad.
    Escape,
    /// A press outside the layer.
    OutsidePress,
}

/// What `place` knows when it places a layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaceInput {
    /// The anchor's painted rect, when the layer has one and it is drawn.
    pub anchor: Option<Rect>,
    /// The layer's laid-out size.
    pub size: Size,
    /// The drawable area in points.
    pub viewport: Rect,
    /// The layer was built in a right-to-left region.
    pub rtl: bool,
}

/// Decides a layer's top-left corner.
pub type Place = Rc<dyn Fn(&PlaceInput) -> Point>;

/// Makes a node the root of a layer. Build it with
/// [`BuildCx::layer`](crate::element::BuildCx::layer) so it draws above the
/// page.
pub struct LayerSpec<Msg> {
    pub anchor: LayerAnchor,
    pub place: Place,
    /// Focus stays inside the layer and the page behind it is inert: hidden
    /// from assistive technology and unreachable by pointer and keyboard.
    pub modal: bool,
    /// Focus stays inside the layer (always true for modal layers).
    pub trap_focus: bool,
    /// When the layer goes away, focus returns to the element that had it
    /// when the layer opened.
    pub restore_focus: bool,
    /// A press outside the layer asks it to close.
    pub light_dismiss: bool,
    /// Escape asks the topmost such layer to close.
    pub escape_dismiss: bool,
    /// The message a dismissal sends. Without one, the layer never closes
    /// itself.
    pub on_dismiss: Option<Rc<dyn Fn(DismissReason) -> Msg>>,
    /// A tint drawn over the page behind a modal layer.
    pub backdrop: Option<crate::contract::Color>,
}

impl<Msg> LayerSpec<Msg> {
    /// A layer placed by `place` with no dismissal and no focus handling.
    pub fn new(anchor: LayerAnchor, place: Place) -> Self {
        LayerSpec {
            anchor,
            place,
            modal: false,
            trap_focus: false,
            restore_focus: false,
            light_dismiss: false,
            escape_dismiss: false,
            on_dismiss: None,
            backdrop: None,
        }
    }

    /// Centers the layer in the viewport.
    pub fn centered() -> Self {
        Self::new(
            LayerAnchor::None,
            Rc::new(|input: &PlaceInput| Point {
                x: input.viewport.x + (input.viewport.width - input.size.width).max(0.0) / 2.0,
                y: input.viewport.y + (input.viewport.height - input.size.height).max(0.0) / 2.0,
            }),
        )
    }

    pub fn on_dismiss(mut self, f: impl Fn(DismissReason) -> Msg + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(f));
        self
    }
}

/// When a [`HoverSpec`] node reports hover, after its delays.
pub struct HoverSpec<Msg> {
    /// How long the pointer must rest before `on_change(true)`.
    pub enter_ms: f64,
    /// How long after the pointer leaves before `on_change(false)`. Moving
    /// into a layer anchored to this node counts as staying.
    pub leave_ms: f64,
    /// Keyboard focus on this node (focus visible) counts as hover at once.
    pub focus: bool,
    pub on_change: Rc<dyn Fn(bool) -> Msg>,
}

/// A paint state a node's [`Draw`](crate::element::Draw) can change in, of
/// the nearest focus stop around it (or itself).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum When {
    Hovered,
    Pressed,
    Focused,
    /// Focused by keyboard, as CSS `:focus-visible`.
    FocusVisible,
}
