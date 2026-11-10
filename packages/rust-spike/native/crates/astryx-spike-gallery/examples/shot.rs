//! Renders cases to PNG with the CPU renderer, no window:
//! `cargo run -p astryx-spike-gallery --example shot -- <out-dir> [light|dark] [case-id-prefix]`.

use astryx_core::{Mode, Runtime};
use astryx_spike_gallery::{all_case_ids, Gallery};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or_else(|| "shots".into());
    let mode = match args.next().as_deref() {
        Some("dark") => Mode::Dark,
        _ => Mode::Light,
    };
    let filter = args.next().unwrap_or_default();
    std::fs::create_dir_all(&out)?;
    let mut n = 0;
    for id in all_case_ids()
        .into_iter()
        .filter(|id| id.starts_with(&filter))
    {
        let g = Gallery::case(&id).expect("case").with_mode(mode);
        let mut rt = Runtime::new(g).hide_focus_until_keyboard();
        astryx_spike_gallery::settle(&mut rt, 420, 300);
        let png = astryx_headless::render_png(&mut rt, 420, 300, 2.0, &[]);
        std::fs::write(format!("{out}/{id}.png"), png)?;
        n += 1;
    }
    println!("wrote {n} shots to {out}");
    Ok(())
}
