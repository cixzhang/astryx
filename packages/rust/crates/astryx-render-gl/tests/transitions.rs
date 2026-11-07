//! Every transition kind halfway through, GPU against CPU.

mod common;

use astryx_core::{
    Color, DisplayList, DrawCmd, Easing, Edge, FrameInfo, Point, Radii, Rect, Transition,
    TransitionKind,
};
use astryx_render_soft::SoftRenderer;

fn scene(bg: Color, fg: Color) -> DisplayList {
    DisplayList {
        clear: bg,
        commands: vec![
            DrawCmd::Rect {
                rect: Rect::new(40.0, 30.0, 160.0, 90.0),
                radii: Radii::all(12.0),
                fill: fg,
                stroke: None,
            },
            DrawCmd::Rect {
                rect: Rect::new(150.0, 100.0, 60.0, 60.0),
                radii: Radii::all(30.0),
                fill: Color::rgba(1.0, 1.0, 1.0, 1.0),
                stroke: None,
            },
        ],
    }
}

#[test]
fn transitions_match_soft_halfway() {
    let mut gl = match common::OffscreenGl::new() {
        Ok(gl) => gl,
        Err(e) => {
            eprintln!("skipping: {e}");
            return;
        }
    };
    let old = scene(
        Color::rgba(0.1, 0.2, 0.5, 1.0),
        Color::rgba(0.9, 0.8, 0.2, 1.0),
    );
    let new = scene(
        Color::rgba(0.9, 0.5, 0.2, 1.0),
        Color::rgba(0.1, 0.5, 0.3, 1.0),
    );
    let out = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("transitions");
    std::fs::create_dir_all(&out).unwrap();
    for (name, kind) in [
        ("fade", TransitionKind::Fade),
        ("wipe", TransitionKind::Wipe(Edge::Left)),
        ("door", TransitionKind::Door),
        ("iris", TransitionKind::Iris(Point { x: 60.0, y: 140.0 })),
        ("slide", TransitionKind::Slide(Point { x: 30.0, y: 0.0 })),
        ("dissolve", TransitionKind::PixelDissolve(8.0)),
        ("swirl", TransitionKind::Swirl),
        ("encounter", TransitionKind::Encounter),
    ] {
        for scale in [1.0, 2.0] {
            let still = FrameInfo {
                width: (240.0 * scale) as u32,
                height: (180.0 * scale) as u32,
                scale,
                dt: 0.0,
            };
            let half = FrameInfo { dt: 0.3, ..still };
            let t = Transition {
                kind,
                duration_ms: 1000,
                easing: Easing::Linear,
                sound: None,
            };
            // For the encounter, 0.3 s falls between flashes; look at the bars.
            let half = if name == "encounter" {
                FrameInfo { dt: 0.8, ..half }
            } else {
                half
            };
            let mut soft = SoftRenderer::new();
            soft.render_list(&old, &still);
            soft.begin_transition(&t, &old, &still);
            soft.render_list(&new, &half);
            assert!(soft.transitioning(), "{name}");
            let want = soft.rgba();
            gl.render(&old, &still);
            gl.renderer.begin_transition(&t, &old, &still);
            let got = gl.render(&new, &half);
            assert!(gl.renderer.transitioning(), "{name}");
            let diff = got
                .iter()
                .zip(&want)
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            let over = got
                .as_chunks::<4>()
                .0
                .iter()
                .zip(want.as_chunks::<4>().0)
                .filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 16))
                .count();
            let share = over as f64 / (still.width * still.height) as f64;
            eprintln!(
                "{name}@{scale}x: max {diff}, over 16: {:.3}%",
                share * 100.0
            );
            soft.pixmap()
                .save_png(out.join(format!("{name}@{scale}x.soft.png")))
                .unwrap();
            // Edges may land one pixel apart, and swirl samples by float math.
            assert!(
                share < 0.01,
                "{name}@{scale}x differs in {:.3}% of pixels",
                share * 100.0
            );
            // Done after the duration: the new frame alone.
            let end = FrameInfo { dt: 1.0, ..still };
            gl.render(&new, &end);
            assert!(!gl.renderer.transitioning(), "{name}");
        }
    }
}
