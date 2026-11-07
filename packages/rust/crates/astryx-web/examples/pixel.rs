//! The pixel theme with text in other scripts, so the web shell loads the
//! theme's pixel fallback fonts (Galmuri7 and Galmuri9) as text needs them.
//!
//! Build it with `PIXEL_FONTS_DIR=<dir with Galmuri7.ttf and Galmuri9.ttf>
//! crates/astryx-web/scripts/build.sh` and open `pixel.html`.

use astryx_core::{App, Command, Element, Mode, Runtime, Space, Text, ThemeChoice, VStack};
use astryx_theme_pixel::Palette;
use astryx_web::Options;
use wasm_bindgen::prelude::*;

/// One line per script. The theme's bundled Galmuri cuts draw Latin,
/// Vietnamese, Greek, and Cyrillic; fallbacks draw the rest.
pub const LINES: [&str; 7] = [
    "Latin: Pixel perfect",
    "Vietnamese: Ti\u{1ebf}ng Vi\u{1ec7}t",
    "Greek: \u{395}\u{3bb}\u{3bb}\u{3b7}\u{3bd}\u{3b9}\u{3ba}\u{3ac}",
    "Cyrillic: \u{420}\u{443}\u{441}\u{441}\u{43a}\u{438}\u{439}",
    "Kana: \u{3072}\u{3089}\u{304c}\u{306a} \u{30ab}\u{30bf}\u{30ab}\u{30ca}",
    "Hangul: \u{d55c}\u{ad6d}\u{c5b4}",
    "Han: \u{65e5}\u{672c}\u{8a9e} \u{4e2d}\u{6587}",
];

struct Scripts;

impl App for Scripts {
    type Msg = ();
    fn view(&self) -> Element<()> {
        LINES
            .iter()
            .fold(
                VStack::new().padding(Space::S4).gap(Space::S2),
                |stack, line| stack.child(Text::new(*line)),
            )
            .into()
    }
    fn update(&mut self, _: ()) -> Command {
        Command::None
    }
    fn theme(&self) -> ThemeChoice {
        ThemeChoice::new(Palette::GameBoy.theme(), Mode::Light)
    }
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let options = Options {
        label: Some("Pixel theme scripts".into()),
        ..Options::default()
    };
    astryx_web::run_with("#app", Runtime::new(Scripts), options)
}
