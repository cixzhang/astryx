//! Runs the gallery in a desktop window:
//! `cargo run -p astryx-gallery --example desktop [-- <screenshot.png>]`.
//! With a path, it writes the first frame there and exits.

use astryx_desktop::{run_with, Options};
use astryx_gallery::Gallery;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let screenshot = std::env::args().nth(1).map(Into::into);
    let options = Options {
        title: "Astryx gallery".into(),
        exit_after_screenshot: screenshot.is_some(),
        screenshot,
        ..Options::default()
    };
    run_with(astryx_gallery::runtime(Gallery::new()), options)
}
