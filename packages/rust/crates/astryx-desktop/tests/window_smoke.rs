//! Opens a real window, draws the gallery fixture through the GL path, and
//! checks the screenshot against the CPU renderer. Needs a display; run it
//! under Xvfb on headless machines (`xvfb-run cargo test -p astryx-desktop`).
//! It is a plain `main` because winit wants the main thread.

use astryx_core::Surface;
use astryx_desktop::{run_with, Options};
use astryx_headless::fixtures::{FixtureSurface, FIXTURE_SIZE};
use astryx_headless::golden::compare_png;
use astryx_headless::{astryx_render_soft::SoftRenderer, frame_info};

fn main() {
    if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
        println!("window_smoke: skipped, no display");
        return;
    }
    let path =
        std::env::temp_dir().join(format!("astryx-desktop-smoke-{}.png", std::process::id()));
    let options = Options {
        title: "Astryx smoke test".into(),
        size: FIXTURE_SIZE,
        screenshot: Some(path.clone()),
        exit_after_screenshot: true,
    };
    run_with(FixtureSurface::new("gallery").unwrap(), options).expect("desktop shell runs");
    let shot = std::fs::read(&path).expect("screenshot written");

    // The window may get a different scale or size from the window manager;
    // compare only when it matches the requested logical size at scale 1.
    let (_, w, h) = astryx_headless::golden::decode_png(&shot).unwrap();
    if (w, h) == FIXTURE_SIZE {
        let frame = frame_info(w, h, 1.0);
        let mut soft = SoftRenderer::new();
        let list = FixtureSurface::new("gallery")
            .unwrap()
            .frame(&frame, soft.text());
        soft.render_list(&list, &frame);
        let cmp = compare_png(&shot, &soft.encode_png(), 16).unwrap();
        println!(
            "window_smoke: {w}x{h}, max difference {} from the CPU renderer, {} pixels over 16",
            cmp.max_diff, cmp.differing
        );
        assert!(cmp.differing < (w * h) as usize / 200);
    } else {
        println!("window_smoke: window is {w}x{h}, skipped pixel comparison");
    }
    let _ = std::fs::remove_file(&path);
    println!("window_smoke: ok");
}
