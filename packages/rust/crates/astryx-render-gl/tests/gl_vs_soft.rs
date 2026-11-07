//! Renders every fixture with the GPU and CPU renderers and compares pixels.
//! Uses EGL without a window, so Mesa's software rasterizer is enough.

mod common;

use astryx_core::Surface;
use astryx_headless::fixtures::{FixtureSurface, FIXTURES, FIXTURE_SIZE};
use astryx_headless::frame_info;
use astryx_headless::golden::compare_rgba;
use astryx_render_soft::tiny_skia::{IntSize, Pixmap};
use astryx_render_soft::SoftRenderer;

fn save(rgba: Vec<u8>, w: u32, h: u32, path: &std::path::Path) {
    let mut premul = rgba;
    for px in premul.as_chunks_mut::<4>().0 {
        let a = px[3] as u32;
        for c in &mut px[..3] {
            *c = ((*c as u32 * a + 127) / 255) as u8;
        }
    }
    Pixmap::from_vec(premul, IntSize::from_wh(w, h).unwrap())
        .unwrap()
        .save_png(path)
        .unwrap();
}

#[test]
fn gl_matches_soft() {
    let mut gl = match common::OffscreenGl::new() {
        Ok(gl) => gl,
        Err(e) => {
            eprintln!("skipping: {e}");
            return;
        }
    };
    eprintln!("GL: {}", gl.description);
    let out = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("gl-vs-soft");
    std::fs::create_dir_all(&out).unwrap();
    let mut soft = SoftRenderer::new();
    let mut worst = 0u8;
    let mut report = Vec::new();
    for scale in [1.0f32, 2.0] {
        for name in FIXTURES {
            let frame = frame_info(FIXTURE_SIZE.0, FIXTURE_SIZE.1, scale);
            let mut surface = FixtureSurface::new(name).unwrap();
            let list = surface.frame(&frame, soft.text());
            // Both renderers lay out with the same engine, so the list is shared.
            soft.render_list(&list, &frame);
            let expected = soft.rgba();
            let actual = gl.render(&list, &frame);
            let cmp = compare_rgba(&actual, &expected, frame.width, frame.height, 16);
            let total = (frame.width * frame.height) as usize;
            let share = cmp.differing as f64 / total as f64;
            report.push(format!(
                "{name}@{scale}x: max {:3}, mean {:.3}, over 16: {:.3}%",
                cmp.max_diff,
                cmp.mean_diff,
                share * 100.0
            ));
            let stem = format!("{name}@{scale}x");
            save(
                actual,
                frame.width,
                frame.height,
                &out.join(format!("{stem}.gl.png")),
            );
            soft.pixmap()
                .save_png(out.join(format!("{stem}.soft.png")))
                .unwrap();
            cmp.diff_image
                .save_png(out.join(format!("{stem}.diff.png")))
                .unwrap();
            worst = worst.max(cmp.max_diff);
            assert!(
                cmp.mean_diff < 0.5,
                "{stem}: mean difference {}",
                cmp.mean_diff
            );
            assert!(
                share < 0.005,
                "{stem}: {:.3}% of pixels differ by more than 16",
                share * 100.0
            );
        }
    }
    let frame = frame_info(FIXTURE_SIZE.0, FIXTURE_SIZE.1, 2.0);
    let list = FixtureSurface::new("gallery")
        .unwrap()
        .frame(&frame, soft.text());
    report.push(format!(
        "gallery@2x frame time on this GL: {:.2} ms",
        gl.time_frames(&list, &frame, 60)
    ));
    eprintln!(
        "{}\nmax difference over all fixtures: {worst}\nimages in {}",
        report.join("\n"),
        out.display()
    );
}
