//! Astryx components on the native Rust stack.
//!
//! Each component is a builder with Astryx's prop names in snake_case
//! (`isDisabled` is `is_disabled`) and converts into an [`Element`]. Builders
//! start from Astryx's documented defaults, so a call names only the props
//! it changes.
//!
//! Components that `astryx-core` already ships at full Astryx parity are
//! re-exported as they are. The others are built here on
//! [`astryx_core::authoring`].

pub use astryx_core::{Element, Space};

mod util;

#[doc(hidden)]
pub mod testkit;

pub mod composites;
pub mod controls;
pub mod layout;
pub mod overlays;
pub mod primitives;

pub use composites::*;
pub use controls::*;
pub use layout::*;
pub use overlays::*;
pub use primitives::*;

/// Shadow depth, read from the theme's `shadow` tokens.
pub use astryx_core::Elevation;
