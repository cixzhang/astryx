//! The Astryx gallery on the real core runtime, in the browser.
//!
//! Build it with `crates/astryx-web/scripts/build.sh` and open `gallery.html`,
//! or `gallery.html#pixel` for the pixel theme.

use astryx_core::Mode;
use astryx_gallery::Gallery;
use astryx_theme_pixel::Palette;
use astryx_web::Options;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let options = Options {
        label: Some("Astryx gallery".into()),
        ..Options::default()
    };
    let mut gallery = Gallery::new();
    let hash = web_sys::window().and_then(|w| w.location().hash().ok());
    if hash.as_deref() == Some("#pixel") {
        gallery = gallery.with_theme(Palette::GameBoy.theme(), Mode::Light);
    }
    astryx_web::run_with("#app", astryx_gallery::runtime(gallery), options)
}
