//! Astryx Rust core: tokens, theme, layout, elements, focus, the runtime,
//! the accessibility tree, and the Astryx components.
//!
//! An app is an [`App`]: a `view` that returns an [`Element`] tree built from
//! components, and an `update` for each message. [`Runtime`] runs it as a
//! [`Surface`], which every shell drives.

pub mod contract;
pub use astryx_tokens as tokens;
pub use astryx_tokens::{Theme, Tokens};
pub use contract::*;

mod a11y;
mod components;
mod element;
mod focus;
pub mod gesture;
pub mod lab;
pub mod layer;
mod layout;
mod light;
pub mod motion;
pub mod paged;
mod paint;
mod pixel;
mod runtime;
pub mod testing;
mod theme;
pub mod world;

pub use components::*;
pub use element::{Element, Space};
pub use paint::{FOCUS_RING_OFFSET, FOCUS_RING_WIDTH};
pub use runtime::{App, FeedbackKind, FrameTimings, Runtime};
pub use theme::{Direction, Mode, ThemeChoice};

/// The building blocks components are made of, for component crates outside
/// this one. An out-of-crate component implements [`authoring::Build`] and
/// converts into an [`Element`] with [`authoring::element`]. These types are
/// the same ones the built-in components use, so they may change with them.
pub mod authoring {
    pub use crate::components::icon_paths::icon_path;
    pub use crate::components::text::{text_draw, text_node, type_style};
    pub use crate::element::{
        element, A11yRefs, Axis, Base, BoxDraw, Build, BuildCx, Capture, Draw, Edit, ImageDraw,
        Indicator, Inherited, Interact, LocalHandle, Locals, Node, Overlay, Painter, PathDraw,
        PopupAction, Scope, Semantics, TextDraw,
    };
    pub use crate::light::LightDraw;
    pub use crate::world::SpriteDraw;
}
