//! Opens a window showing one headless fixture.
//!
//! cargo run -p astryx-desktop --example fixtures -- [fixture] [--screenshot out.png]

use astryx_desktop::{run_with, Options};
use astryx_headless::fixtures::{FixtureSurface, FIXTURE_SIZE};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut name = "gallery".to_owned();
    let mut options = Options {
        size: FIXTURE_SIZE,
        ..Options::default()
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--screenshot" => {
                options.screenshot = Some(args.next().ok_or("--screenshot needs a path")?.into());
                options.exit_after_screenshot = true;
            }
            other => name = other.to_owned(),
        }
    }
    let surface = FixtureSurface::new(&name).ok_or_else(|| format!("unknown fixture: {name}"))?;
    options.title = format!("Astryx — {name}");
    run_with(surface, options)
}
