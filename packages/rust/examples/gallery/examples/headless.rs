//! Renders the gallery to PNG files without a window:
//! `cargo run -p astryx-gallery --example headless -- <out-dir> [scale]`.
//! Writes one image per section in light and dark mode, and the whole page.

use astryx_core::{Mode, Runtime, Theme};
use astryx_gallery::{Gallery, Section};
use astryx_headless::render_png;
use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "gallery-out".into()));
    let scale: f32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1.0);
    std::fs::create_dir_all(&out)?;
    for section in Section::ALL {
        for mode in [Mode::Light, Mode::Dark] {
            let mut rt = Runtime::new(Gallery::section(section).with_theme(Theme::ASTRYX, mode));
            let png = render_png(&mut rt, 720, 900, scale, &[]);
            let name = format!(
                "{}-{}.png",
                section.slug(),
                if mode == Mode::Dark { "dark" } else { "light" }
            );
            std::fs::write(out.join(&name), png)?;
            println!("{}", out.join(name).display());
        }
    }
    let mut rt = Runtime::new(Gallery::new());
    std::fs::write(
        out.join("gallery.png"),
        render_png(&mut rt, 960, 640, scale, &[]),
    )?;
    Ok(())
}
