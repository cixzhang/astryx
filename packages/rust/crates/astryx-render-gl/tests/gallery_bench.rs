//! Times the gallery app through the GPU renderer at the Brick's size
//! (1024x768, scale 2), idle and while moving focus. Ignored by default:
//! `cargo test --release -p astryx-render-gl --test gallery_bench -- --ignored --nocapture`.

mod common;

use std::time::{Duration, Instant};

use astryx_core::{FrameInfo, Input, InputPhase, Runtime, Surface};
use astryx_gallery::Gallery;

fn save(mut rgba: Vec<u8>, w: u32, h: u32, path: &std::path::Path) {
    use astryx_render_soft::tiny_skia::{IntSize, Pixmap};
    for px in rgba.as_chunks_mut::<4>().0 {
        let a = px[3] as u32;
        for c in &mut px[..3] {
            *c = ((*c as u32 * a + 127) / 255) as u8;
        }
    }
    Pixmap::from_vec(rgba, IntSize::from_wh(w, h).unwrap())
        .unwrap()
        .save_png(path)
        .unwrap();
}

#[test]
#[ignore]
fn gallery_frame_times() {
    let mut gl = match common::OffscreenGl::new() {
        Ok(gl) => gl,
        Err(e) => {
            eprintln!("skipping: {e}");
            return;
        }
    };
    let frame = FrameInfo {
        width: 1024,
        height: 768,
        scale: 2.0,
        dt: 1.0 / 60.0,
    };
    let mut rt = Runtime::new(Gallery::new());
    let first = Instant::now();
    let list = rt.frame(&frame, gl.renderer.text());
    let rgba = gl.render(&list, &frame);
    let out = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    save(rgba, frame.width, frame.height, &out.join("gallery.gl.png"));
    let mut soft = astryx_render_soft::SoftRenderer::new();
    soft.render_list(&list, &frame);
    soft.pixmap()
        .save_png(out.join("gallery.soft.png"))
        .unwrap();
    eprintln!(
        "first frame {:?}: {:?} {:?}",
        first.elapsed(),
        rt.timings(),
        gl.renderer.last_stats()
    );
    let cold = Instant::now();
    for _ in 0..10 {
        gl.reset_renderer();
        let mut rt = Runtime::new(Gallery::new());
        let list = rt.frame(&frame, gl.renderer.text());
        gl.render(&list, &frame);
    }
    eprintln!(
        "cold first frame (new renderer and app) avg {:?}",
        cold.elapsed() / 10
    );
    // A busy section: every button variant and state.
    let mut rt = Runtime::new(Gallery::section(astryx_gallery::Section::Buttons));
    let list = rt.frame(&frame, gl.renderer.text());
    let rgba = gl.render(&list, &frame);
    save(
        rgba,
        frame.width,
        frame.height,
        &out.join("gallery-buttons.gl.png"),
    );
    // Theme switches on the home screen: rebuild, relayout, new text sizes.
    let mut rt = Runtime::new(Gallery::new());
    let list = rt.frame(&frame, gl.renderer.text());
    gl.render(&list, &frame);
    let (mut rebuild, mut layout, mut total) = (Duration::ZERO, Duration::ZERO, Duration::ZERO);
    for i in 0..16 {
        if i == 8 {
            eprintln!(
                "theme switch first pass avg: rebuild {:?}, layout {:?}, total {:?}",
                rebuild / 8,
                layout / 8,
                total / 8
            );
        }
        rt.input(Input::R1, InputPhase::Pressed);
        rt.input(Input::R1, InputPhase::Released);
        let t = Instant::now();
        let list = rt.frame(&frame, gl.renderer.text());
        gl.render(&list, &frame);
        total += t.elapsed();
        rebuild += rt.timings().rebuild;
        layout += rt.timings().layout;
    }
    eprintln!(
        "theme switch avg: rebuild {:?}, layout {:?}, total {:?}",
        rebuild / 16,
        layout / 16,
        total / 16
    );
    for (label, nav) in [("idle", false), ("navigating", true)] {
        let (mut app, mut enc, mut flush, mut total) = (
            Duration::ZERO,
            Duration::ZERO,
            Duration::ZERO,
            Duration::ZERO,
        );
        let n = 120;
        for i in 0..n {
            if nav {
                let dir = if (i / 30) % 2 == 0 {
                    Input::Down
                } else {
                    Input::Up
                };
                rt.input(dir, InputPhase::Pressed);
                rt.input(dir, InputPhase::Released);
            }
            let t = Instant::now();
            let list = rt.frame(&frame, gl.renderer.text());
            let a = t.elapsed();
            gl.render(&list, &frame);
            total += t.elapsed();
            app += a;
            let s = gl.renderer.last_stats();
            enc += s.encode;
            flush += s.flush;
        }
        let ms = |d: Duration| d.as_secs_f64() * 1000.0 / n as f64;
        eprintln!(
            "{label}: app {:.3} ms, encode {:.3} ms, flush {:.3} ms, total {:.3} ms; last {:?}",
            ms(app),
            ms(enc),
            ms(flush),
            ms(total),
            gl.renderer.last_stats()
        );
    }
}

/// Held Down on the home screen, every frame drawn by both renderers.
#[test]
#[ignore]
fn gallery_scroll_matches_soft() {
    let mut gl = match common::OffscreenGl::new() {
        Ok(gl) => gl,
        Err(e) => {
            eprintln!("skipping: {e}");
            return;
        }
    };
    let frame = FrameInfo {
        width: 1024,
        height: 768,
        scale: 2.0,
        dt: 1.0 / 60.0,
    };
    let mut rt = Runtime::new(Gallery::new());
    let mut soft = astryx_render_soft::SoftRenderer::new();
    let out = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let mut worst = (0u8, 0usize);
    for i in 0..120 {
        if i % 6 == 0 && i < 90 {
            rt.input(Input::Down, InputPhase::Pressed);
            rt.input(Input::Down, InputPhase::Released);
        }
        let list = rt.frame(&frame, gl.renderer.text());
        let a = gl.render(&list, &frame);
        soft.render_list(&list, &frame);
        let b = soft.rgba();
        let max = a
            .iter()
            .zip(b.iter())
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap();
        if i == 20 || i == 40 {
            soft.pixmap()
                .save_png(out.join(format!("scroll{i}.soft.png")))
                .unwrap();
        }
        if max > worst.0 {
            worst = (max, i);
            save(a, frame.width, frame.height, &out.join("scroll.gl.png"));
            soft.pixmap().save_png(out.join("scroll.soft.png")).unwrap();
        }
    }
    eprintln!("worst frame {} max diff {}", worst.1, worst.0);
}
