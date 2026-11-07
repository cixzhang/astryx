//! Reads [`Preferences`] from the browser's media queries.

use astryx_core::{ColorScheme, Preferences};

/// Media queries that change a [`Preferences`] field.
pub(crate) const QUERIES: [&str; 4] = [
    "(prefers-reduced-motion: reduce)",
    "(prefers-color-scheme: dark)",
    "(prefers-contrast: more)",
    "(forced-colors: active)",
];

/// Reads the person's current preferences. Returns defaults outside a
/// browser. The shell passes them to `Surface::set_preferences` when it
/// starts and whenever they change.
pub fn detect_preferences() -> Preferences {
    let Some(window) = window() else {
        return Preferences::default();
    };
    let matches = |query: &str| {
        window
            .match_media(query)
            .ok()
            .flatten()
            .is_some_and(|m| m.matches())
    };
    Preferences {
        reduced_motion: matches(QUERIES[0]),
        color_scheme: if matches(QUERIES[1]) {
            ColorScheme::Dark
        } else {
            ColorScheme::Light
        },
        more_contrast: matches(QUERIES[2]),
        forced_colors: matches(QUERIES[3]),
    }
}

#[cfg(target_arch = "wasm32")]
fn window() -> Option<web_sys::Window> {
    web_sys::window()
}

#[cfg(not(target_arch = "wasm32"))]
fn window() -> Option<web_sys::Window> {
    None
}
