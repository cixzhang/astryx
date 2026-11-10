//! The spike gallery on the web. `gallery.html` browses every family;
//! `gallery.html?case=<id>&scheme=light|dark&dir=ltr|rtl` shows one case
//! for capture; `?case=playground-0001` to `-0004` are interactive pages
//! with live state; `?template=shell-nav|settings-dialog` runs a template.
//! The page marks `qa:first-frame` when the first frame paints.

use astryx_core::{Direction, Mode, Runtime};
use astryx_spike_gallery::Gallery;
use astryx_web::Options;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let search = window.location().search().unwrap_or_default();
    let params = web_sys::UrlSearchParams::new_with_str(&search)?;
    if let Some(name) = params.get("template") {
        let mode = if params.get("scheme").as_deref() == Some("dark") {
            Mode::Dark
        } else {
            Mode::Light
        };
        let options = Options {
            label: Some(format!("Astryx template {name}")),
            ..Options::default()
        };
        return match name.as_str() {
            "shell-nav" => astryx_web::run_with(
                "#app",
                Runtime::new(astryx_spike_templates::ShellNav {
                    mode,
                    ..Default::default()
                })
                .hide_focus_until_keyboard(),
                options,
            ),
            "settings-dialog" => astryx_web::run_with(
                "#app",
                Runtime::new(astryx_spike_templates::SettingsDialog {
                    mode,
                    ..Default::default()
                })
                .hide_focus_until_keyboard(),
                options,
            ),
            other => Err(format!("no template {other:?}").into()),
        };
    }
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
