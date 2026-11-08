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

/// The World board at the Brick's size: per-frame encode and what each
/// frame computes or uploads once it is steady.
#[test]
#[ignore]
fn world_frame_costs() {
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
    let gallery = Gallery {
        page: Some(astryx_gallery::Section::World),
        ..Gallery::new()
    };
    let mut rt = astryx_gallery::runtime(gallery);
    for _ in 0..5 {
        rt.input(Input::Down, InputPhase::Pressed);
        rt.input(Input::Down, InputPhase::Released);
    }
    let mut worst = Duration::ZERO;
    for i in 0..240 {
        for (id, data) in rt.take_image_updates() {
            gl.renderer.set_image(id, data.as_ref());
        }
        let t = Instant::now();
        let list = rt.frame(&frame, gl.renderer.text());
        let app = t.elapsed();
        gl.render(&list, &frame);
        let s = gl.renderer.last_stats();
        if i >= 60 {
            worst = worst.max(s.encode);
        }
        if i % 60 == 59 {
            eprintln!(
                "world frame {i}: app {app:?}, {} commands, {s:?}",
                list.commands.len()
            );
        }
    }
    eprintln!("world steady worst encode {worst:?}");
}

/// A garden-like lit scene over many frames: encode must not grow.
#[test]
#[ignore]
fn lit_scene_over_time() {
    use astryx_core::{
        Color, Dapple, DisplayList, DrawCmd, Flicker, Light, Lighting, Point, Pulse, Rect,
    };
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
    for i in 0..3000u32 {
        let time = i as f32 / 60.0;
        let mut commands = vec![
            DrawCmd::PushLighting(Lighting {
                ambient: Color::rgba(0.07, 0.09, 0.22, 1.0),
                strength: 0.84,
                time,
                dapple: Some(Dapple {
                    cell: 26.0,
                    velocity: Point { x: 2.0, y: 1.0 },
                    bands: 3,
                    depth: 0.35,
                }),
            }),
            DrawCmd::Rect {
                rect: Rect::new(0.0, 0.0, 512.0, 384.0),
                radii: Default::default(),
                fill: Color::rgba(0.4, 0.7, 0.3, 1.0),
                stroke: None,
            },
        ];
        for k in 0..5 {
            commands.push(DrawCmd::Light(Light {
                center: Point {
                    x: 80.0 + k as f32 * 80.0,
                    y: 200.0,
                },
                radius_x: 60.0,
                radius_y: 40.0,
                color: Color::rgba(1.0, 0.8, 0.5, 1.0),
                intensity: 1.0,
                flicker: Flicker {
                    amplitude: 0.1,
                    hz: 2.0,
                },
                pulse: Pulse {
                    period: if k == 0 { 2.0 } else { 0.0 },
                    min: 0.5,
                },
            }));
        }
        commands.push(DrawCmd::Emissive {
            rect: Rect::new(10.0, 10.0, 20.0, 20.0),
            color: Color::rgba(1.0, 0.9, 0.5, 1.0),
        });
        commands.push(DrawCmd::PopLighting);
        let list = DisplayList {
            clear: Color::rgba(0.0, 0.0, 0.0, 1.0),
            commands,
        };
        gl.render(&list, &frame);
        if i % 500 == 0 || i == 2999 {
            eprintln!("lit frame {i}: {:?}", gl.renderer.last_stats());
        }
    }
}

/// What a section's first frame uploads after the Brick shell's prewarm and
/// preload of the current theme.
#[test]
#[ignore]
fn first_use_uploads() {
    use astryx_core::{FontFamily, TextStyle};
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
    let mut rt = astryx_gallery::runtime(Gallery::new());
    let tokens = rt.app().theme.light;
    let f = &tokens.font;
    let mut styles = Vec::new();
    for (s, fam) in [
        (f.body, FontFamily::Body),
        (f.label, FontFamily::Body),
        (f.supporting, FontFamily::Body),
        (f.large, FontFamily::Body),
        (f.code, FontFamily::Code),
        (f.heading1, FontFamily::Heading),
        (f.heading2, FontFamily::Heading),
        (f.heading3, FontFamily::Heading),
        (f.heading4, FontFamily::Heading),
        (f.heading5, FontFamily::Heading),
        (f.heading6, FontFamily::Heading),
    ] {
        styles.push((
            TextStyle {
                family: fam,
                size: s.size,
                weight: s.weight,
                leading: s.leading,
                direction: None,
            },
            None,
        ));
    }
    let sz = &f.size;
    for size in [
        sz.base, sz.sm, sz.lg, sz.xs, sz.xl, sz.s2xl, sz.s2xs, sz.s3xs, sz.s4xs, sz.s3xl,
    ] {
        for w in [400, 500, 700] {
            styles.push((
                TextStyle {
                    family: FontFamily::Body,
                    size,
                    weight: w,
                    leading: 1.4,
                    direction: None,
                },
                None,
            ));
        }
    }
    let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.,:;!?'\"()[]{}<>/\\|-_+=*&^%$#@~`\u{2014}\u{2013}\u{2026}\u{00B7}\u{2022}\u{2019}\u{201C}\u{201D}";
    let mut warm = gl.renderer.text().prewarm(&styles, 2.0, chars, 64.0);
    while !gl.renderer.text().install_prewarmed(&mut warm) {
        std::thread::sleep(Duration::from_millis(5));
    }
    gl.renderer.preload_glyphs(&styles, 2.0);
    gl.renderer.finish_preload();
    let list = rt.frame(&frame, gl.renderer.text());
    gl.render(&list, &frame);
    eprintln!("home: {:?}", gl.renderer.last_stats());
    for _ in 0..4 {
        rt.input(Input::Down, InputPhase::Pressed);
        rt.input(Input::Down, InputPhase::Released);
        for _ in 0..30 {
            let list = rt.frame(&frame, gl.renderer.text());
            gl.render(&list, &frame);
            let s = gl.renderer.last_stats();
            if s.uploads > 0 {
                eprintln!("scroll: {s:?}");
            }
        }
    }
    rt.input(Input::A, InputPhase::Pressed);
    rt.input(Input::A, InputPhase::Released);
    for _ in 0..30 {
        let list = rt.frame(&frame, gl.renderer.text());
        gl.render(&list, &frame);
        let s = gl.renderer.last_stats();
        if s.uploads > 0 {
            eprintln!("section: {s:?}");
        }
    }
}
