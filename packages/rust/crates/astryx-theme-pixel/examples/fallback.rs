//! Shows how pixel fallbacks for other scripts join the runtime font set:
//! `cargo run -p astryx-theme-pixel --example fallback -- <out.png> [font files...]`.
//! Each file is added with `add_fallback_font`, after Silkscreen, which
//! the theme loads itself.

use astryx_core::{App, Command, Element, Mode, Runtime, Text, ThemeChoice, VStack};
use astryx_headless::{astryx_render_soft::SoftRenderer, render_pixmap};
use astryx_theme_pixel::Palette;

struct Scripts;

const LINES: [&str; 6] = [
    "Silkscreen: Pixel perfect",
    "Vietnamese: Tiếng Việt",
    "Greek: Ελληνικά",
    "Cyrillic: Русский",
    "CJK: 日本語 中文 한국어",
    "Kana: ひらがな カタカナ",
];

impl App for Scripts {
    type Msg = ();
    fn view(&self) -> Element<()> {
        LINES
            .iter()
            .fold(VStack::new().padding(astryx_core::Space::S4), |s, line| {
                s.child(Text::new(*line))
            })
            .into()
    }
    fn update(&mut self, _: ()) -> Command {
        Command::None
    }
    fn theme(&self) -> ThemeChoice {
        ThemeChoice::new(Palette::GameBoy.theme(), Mode::Light)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut renderer = SoftRenderer::new();
    for path in &args[1..] {
        let bytes = std::fs::read(path).expect("font file");
        renderer.add_fallback_font(bytes).expect("a readable font");
    }
    let mut rt = Runtime::new(Scripts);
    render_pixmap(&mut renderer, &mut rt, 440, 180, 3.0, &[]);
    std::fs::write(&args[0], renderer.encode_png()).unwrap();
}
