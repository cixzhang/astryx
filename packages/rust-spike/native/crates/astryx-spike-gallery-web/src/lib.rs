//! The spike gallery on the web. `gallery.html` browses every family;
//! `gallery.html?case=<id>&scheme=light|dark&dir=ltr|rtl` shows one case
//! for capture. The page marks `qa:first-frame` when the first frame paints.

use astryx_core::{Direction, Mode, Runtime};
use astryx_spike_gallery::Gallery;
use astryx_web::Options;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let search = window.location().search().unwrap_or_default();
    let params = web_sys::UrlSearchParams::new_with_str(&search)?;
    let gallery = match params.get("case") {
        Some(id) => Gallery::case(&id).ok_or_else(|| format!("no case {id:?}"))?,
        None => Gallery::new(),
    };
    let mode = match params.get("scheme").as_deref() {
        Some("dark") => Mode::Dark,
        _ => Mode::Light,
    };
    let dir = match params.get("dir").as_deref() {
        Some("rtl") => Direction::Rtl,
        _ => Direction::Ltr,
    };
    let options = Options {
        label: Some("Astryx spike gallery".into()),
        ..Options::default()
    };
    let mut rt = Runtime::new(
        gallery
            .with_mode(mode)
            .with_direction(dir)
            .with_theme(&params.get("theme").unwrap_or_default()),
    )
    .hide_focus_until_keyboard();
    // A case page shows its settled state on the first frame, so captures
    // do not catch a layer mid-entrance.
    if params.get("case").is_some() {
        let window = web_sys::window().expect("window");
        let w = window
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(1280.0) as u32;
        let h = window
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(800.0) as u32;
        astryx_spike_gallery::settle(&mut rt, w, h);
    }
    astryx_web::run_with("#app", rt, options)
}
