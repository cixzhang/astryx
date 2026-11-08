//! Reads [`Preferences`] from the browser's media queries.

use astryx_core::{ColorScheme, DayPhase, Preferences};

/// Media queries that change a [`Preferences`] field.
pub(crate) const QUERIES: [&str; 5] = [
    "(prefers-reduced-motion: reduce)",
    "(prefers-color-scheme: dark)",
    "(prefers-contrast: more)",
    "(forced-colors: active)",
    "(pointer: coarse)",
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
        coarse_pointer: matches(QUERIES[4]),
        // The browser does not say; apps that light scenes read their own clock.
        day_phase: DayPhase::Day,
        day_time: None,
        idle: astryx_core::Idle::Active,
        device: None,
        sound_off: false,
        // The shell sets it from the gamepad that sends input.
        face_layout: astryx_core::FaceLayout::Unknown,
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
