//! Layout time of theme and mode switches at Brick size, from the
//! runtime's own timings. Run with `--release -- --ignored --nocapture`.

use astryx_core::{ColorScheme, FrameInfo, Input, InputPhase, Preferences, Surface};
use astryx_gallery::{Gallery, Section};
use astryx_render_soft::SoftRenderer;
use std::time::Duration;

#[test]
#[ignore]
fn theme_switch_layout_time() {
    let renderer = SoftRenderer::new();
    let frame = FrameInfo {
        width: 2048,
        height: 1536,
        scale: 2.0,
        dt: 1.0 / 60.0,
    };
    let gallery = Gallery {
        page: Some(Section::Buttons),
        ..Gallery::new()
    };
    let mut rt = astryx_gallery::runtime(gallery);
    for _ in 0..3 {
        rt.frame(&frame, renderer.text());
    }
    let run = |rt: &mut astryx_core::Runtime<Gallery>, input: Input, label: &str| {
        let mut total = Duration::ZERO;
        let n = 20;
        for _ in 0..n {
            rt.input(input, InputPhase::Pressed);
            rt.input(input, InputPhase::Released);
            rt.frame(&frame, renderer.text());
            total += rt.timings().layout;
            rt.frame(&frame, renderer.text());
        }
        println!("{label}: layout {:?} per switch", total / n);
    };
    run(
        &mut rt,
        Input::Select,
        "mode switch with the Select shortcut (its label changes)",
    );
    run(&mut rt, Input::R1, "theme switch with R1 (next theme)");
    // The system color scheme flipping under System mode changes colors only.
    let mut total = Duration::ZERO;
    let mut laid_out = 0;
    for i in 0..20 {
        let scheme = if i % 2 == 0 {
            ColorScheme::Dark
        } else {
            ColorScheme::Light
        };
        rt.set_preferences(&Preferences {
            color_scheme: scheme,
            ..Preferences::default()
        });
        rt.frame(&frame, renderer.text());
        total += rt.timings().layout;
        laid_out += rt.timings().laid_out as u32;
    }
    println!(
        "system light/dark (colors only): layout {:?} per switch, {laid_out} of 20 laid out",
        total / 20
    );
}
