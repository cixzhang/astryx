//! The gallery in a desktop window.
//!
//! `cargo run -p astryx-spike-gallery --example desktop` browses every
//! family. The QA harness drives one case through the environment:
//! - `ASTRYX_CASE=<id>`: show only that case.
//! - `ASTRYX_SCHEME=light|dark`, `ASTRYX_DIR=ltr|rtl`.
//! - `ASTRYX_SIZE=<w>x<h>`: window size in points.
//! - `ASTRYX_SHOT=<png>`: write the first settled frame there and exit.
//! - `ASTRYX_A11Y=<json>`: write the accessibility tree there first.
//!
//! It prints `first_frame_ms=<n>` once the frame is written.

use astryx_core::contract::Surface;
use astryx_core::{Direction, Mode, Runtime};
use astryx_desktop::{run_with, Options};
use astryx_spike_gallery::{a11y_json, Gallery};
use std::time::Instant;

fn gallery() -> Gallery {
    let g = match std::env::var("ASTRYX_CASE") {
        Ok(id) => Gallery::case(&id).unwrap_or_else(|| {
            eprintln!("no case {id:?}");
            std::process::exit(2)
        }),
        Err(_) => Gallery::new(),
    };
    let mode = match std::env::var("ASTRYX_SCHEME").as_deref() {
        Ok("dark") => Mode::Dark,
        _ => Mode::Light,
    };
    let dir = match std::env::var("ASTRYX_DIR").as_deref() {
        Ok("rtl") => Direction::Rtl,
        _ => Direction::Ltr,
    };
    g.with_mode(mode).with_direction(dir)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    let (w, h) = std::env::var("ASTRYX_SIZE")
        .ok()
        .and_then(|s| {
            let (w, h) = s.split_once('x')?;
            Some((w.parse().ok()?, h.parse().ok()?))
        })
        .unwrap_or((1280, 800));
    if let Ok(path) = std::env::var("ASTRYX_A11Y") {
        let mut rt = Runtime::new(gallery());
        let renderer = astryx_render_soft::SoftRenderer::new();
        let frame = astryx_headless::frame_info(w, h, 1.0);
        let _ = rt.frame(&frame, renderer.text());
        let _ = rt.frame(&frame, renderer.text());
        let tree = rt.accessibility().map(|t| a11y_json::to_json(&t));
        std::fs::write(&path, serde_json::to_string_pretty(&tree)?)?;
    }
    let screenshot = std::env::var("ASTRYX_SHOT").ok().map(Into::into);
    let options = Options {
        title: "Astryx spike gallery".into(),
        size: (w, h),
        exit_after_screenshot: screenshot.is_some(),
        screenshot,
    };
    let shooting = options.exit_after_screenshot;
    run_with(Runtime::new(gallery()), options)?;
    if shooting {
        println!("first_frame_ms={}", start.elapsed().as_millis());
    }
    Ok(())
}
