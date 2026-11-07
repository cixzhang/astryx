//! What every Astryx renderer shares, so their layout and pixels agree:
//! the [`text`] engine (shaping, wrapping, glyph bitmaps), pixel-snapped
//! [`geom`]etry, and CSS `box-shadow` masks in [`shadow`]. Pure Rust with no
//! GPU or OS dependency; builds for `wasm32-unknown-unknown`.

pub mod geom;
pub mod shadow;
pub mod text;
