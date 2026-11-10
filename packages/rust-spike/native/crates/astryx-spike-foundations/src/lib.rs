//! Astryx foundations for the Rust stack.
//!
//! - [`theme`]: every Astryx brand in light and dark, switched at runtime.
//! - [`i18n`]: locales, direction, Astryx's 30 message catalogs, ICU
//!   messages with plurals, and locale formats for numbers, dates and lists.
//! - [`focus`]: roving focus, typeahead, grid focus, and virtual focus.
//! - [`size`]: the component size cascade.
//! - [`CxExt`]: how components read these while they build.

pub mod cx;
pub mod focus;
pub mod i18n;
pub mod size;
pub mod theme;

pub use cx::{provide_i18n, CxExt};
pub use i18n::I18n;
pub use size::{provide_size, ComponentSize};
pub use theme::{ExtraTokens, ThemeState};
