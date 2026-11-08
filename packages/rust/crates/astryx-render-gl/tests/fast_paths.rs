//! The fast programs (solid and textured quads with coverage 1) draw the
//! same pixels as the full program, for every fixture at 1x and 2x.

mod common;

use astryx_core::Surface;
use astryx_headless::fixtures::{FixtureSurface, FIXTURES, FIXTURE_SIZE};
use astryx_headless::frame_info;

#[test]
fn fast_programs_draw_the_same_pixels() {
    let mut gl = match common::OffscreenGl::new() {
        Ok(gl) => gl,
        Err(e) => {
            eprintln!("skipping: {e}");
            return;
        }
    };
    let text = astryx_fonts::engine();
    for scale in [1.0f32, 2.0] {
        for name in FIXTURES {
            let frame = frame_info(FIXTURE_SIZE.0, FIXTURE_SIZE.1, scale);
            let mut surface = FixtureSurface::new(name).unwrap();
            for (id, data) in surface.take_image_updates() {
                gl.renderer.set_image(id, data.as_ref());
            }
            let list = surface.frame(&frame, &text);
            gl.renderer.set_fast_paths(false);
            let full = gl.render(&list, &frame);
            gl.renderer.set_fast_paths(true);
            let fast = gl.render(&list, &frame);
            if let Some(dir) = std::env::var_os("FAST_PATHS_OUT") {
                let dir = std::path::PathBuf::from(dir);
                std::fs::create_dir_all(&dir).unwrap();
                for (tag, rgba) in [("full", &full), ("fast", &fast)] {
                    let path = dir.join(format!("{name}@{scale}x-{tag}.rgba"));
                    std::fs::write(path, rgba).unwrap();
                }
            }
            let differing = full.iter().zip(&fast).filter(|(a, b)| a != b).count();
            assert_eq!(differing, 0, "{name}@{scale}x: {differing} bytes differ");
        }
    }
}
