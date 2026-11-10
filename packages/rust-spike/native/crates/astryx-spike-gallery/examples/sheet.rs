//! Renders the browse page of each family to PNG (CPU renderer), for a
//! quick look at a whole family: `cargo run --example sheet -- <out-dir> [light|dark]`.

use astryx_core::{Mode, Runtime};
use astryx_spike_gallery::{families, Gallery, Msg};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or_else(|| "sheets".into());
    let mode = match args.next().as_deref() {
        Some("dark") => Mode::Dark,
        _ => Mode::Light,
    };
    std::fs::create_dir_all(&out)?;
    for (i, f) in families::all().iter().enumerate() {
        let mut rt = Runtime::new(Gallery::new().with_mode(mode)).hide_focus_until_keyboard();
        rt.send(Msg::Family(i));
        let png = astryx_headless::render_png(&mut rt, 1280, 900, 1.0, &[]);
        std::fs::write(format!("{out}/{:02}-{}.png", i, f.name), png)?;
    }
    Ok(())
}
