//! Astryx Rust on the web: runs any [`Surface`] in a `<canvas>` through
//! WebAssembly.
//!
//! The shell draws with `astryx-render-gl` on WebGL2, sizes the canvas for
//! the device pixel ratio, draws frames only while the app animates or after
//! input, maps the keyboard and gamepads to [`Input`], loads font subsets by
//! script, passes browser preferences to the app, and mirrors each frame's
//! accessibility tree into transparent DOM elements over the canvas so
//! assistive tech can read and operate the app. The crate README lists what
//! works and what does not.
//!
//! ```ignore
//! astryx_web::run("#app", my_app_surface)?;
//! ```
//!
//! [`Surface`]: astryx_core::Surface
//! [`Input`]: astryx_core::Input

#[cfg(feature = "audio")]
mod audio;
mod focus;
mod fonts;
mod gamepad;
mod keys;
mod mirror;
mod prefs;
mod shell;

pub use astryx_core::{ColorScheme, Preferences};
pub use prefs::detect_preferences;
pub use shell::{run, run_with, Options, Target};
