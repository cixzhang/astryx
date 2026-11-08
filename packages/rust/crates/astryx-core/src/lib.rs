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
mod layout;
mod light;
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
pub use runtime::{App, FrameTimings, Runtime};
pub use theme::{Direction, Mode, ThemeChoice};
