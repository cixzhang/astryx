//! Renders the gallery in a pixel palette:
//! `cargo run -p astryx-theme-pixel --example shot -- <palette> <light|dark> <section|all> <width> <height> <scale> <out.png>`.

use astryx_core::Mode;
use astryx_gallery::{Gallery, Section};
use astryx_headless::render_png;
use astryx_theme_pixel::Palette;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let palette = Palette::by_name(&a[0]).expect("palette name");
    let mode = if a[1] == "dark" {
        Mode::Dark
    } else {
        Mode::Light
    };
    let gallery = match Section::ALL.into_iter().find(|s| s.slug() == a[2]) {
        Some(section) => Gallery::section(section),
        None => Gallery::new(),
    };
    let size = |i: usize| a[i].parse::<u32>().expect("size");
    let scale: f32 = a[5].parse().expect("scale");
    let mut rt = astryx_gallery::runtime(gallery.with_theme(palette.theme(), mode));
    std::fs::write(&a[6], render_png(&mut rt, size(3), size(4), scale, &[])).unwrap();
}
