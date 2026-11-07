//! The Astryx gallery on the real core runtime, in the browser.
//!
//! Build it with `crates/astryx-web/scripts/build.sh` and open `gallery.html`.

use astryx_core::Runtime;
use astryx_gallery::Gallery;
use astryx_web::Options;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let options = Options {
        label: Some("Astryx gallery".into()),
        ..Options::default()
    };
    astryx_web::run_with("#app", Runtime::new(Gallery::new()), options)
}
