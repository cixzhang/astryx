//! Renders a few sections right to left: `cargo run -p astryx-gallery --example rtl -- <out-dir>`.

use astryx_core::{Direction, Runtime};
use astryx_gallery::{Gallery, Section};
use astryx_headless::render_png;

fn main() -> std::io::Result<()> {
    let out = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "gallery-rtl".into()),
    );
    std::fs::create_dir_all(&out)?;
    for section in Section::ALL {
        let mut rt = Runtime::new(Gallery::section(section).with_direction(Direction::Rtl));
        std::fs::write(
            out.join(format!("{}-rtl.png", section.slug())),
            render_png(&mut rt, 720, 900, 1.0, &[]),
        )?;
    }
    let mut rt = Runtime::new(Gallery::new().with_direction(Direction::Rtl));
    std::fs::write(
        out.join("home-rtl.png"),
        render_png(&mut rt, 960, 640, 1.0, &[]),
    )?;
    Ok(())
}
