//! A lantern and its light stay together inside a moved and scaled
//! transform, in both renderers: the light's brightest band is centered
//! on the lantern sprite.

mod common;

use astryx_core::{Color, DisplayList, DrawCmd, FrameInfo, Light, Lighting, Point, Radii, Rect};
use astryx_render_soft::SoftRenderer;

const LANTERN: Rect = Rect {
    x: 96.0,
    y: 76.0,
    width: 8.0,
    height: 8.0,
};

fn scene() -> DisplayList {
    DisplayList {
        clear: Color::rgba(0.5, 0.5, 0.5, 1.0),
        commands: vec![
            DrawCmd::PushTransform {
                origin: Point { x: 0.0, y: 0.0 },
                scale: 1.5,
                translate: Point { x: 30.0, y: 18.0 },
            },
            DrawCmd::PushLighting(Lighting {
                ambient: Color::rgba(0.0, 0.0, 0.0, 1.0),
                strength: 1.0,
                time: 0.0,
                dapple: None,
            }),
            DrawCmd::Rect {
                rect: Rect::new(0.0, 0.0, 240.0, 180.0),
                radii: Radii::default(),
                fill: Color::rgba(0.6, 0.6, 0.6, 1.0),
                stroke: None,
            },
            DrawCmd::Light(Light {
                center: Point {
                    x: LANTERN.x + 4.0,
                    y: LANTERN.y + 4.0,
                },
                radius_x: 40.0,
                radius_y: 40.0,
                color: Color::rgba(1.0, 1.0, 1.0, 1.0),
                intensity: 1.0,
                flicker: Default::default(),
                pulse: Default::default(),
            }),
            DrawCmd::Emissive {
                rect: LANTERN,
                color: Color::rgba(1.0, 0.0, 0.0, 1.0),
            },
            DrawCmd::PopLighting,
            DrawCmd::PopTransform,
        ],
    }
}

/// The centroid of lit pixels and of the red lantern, in device pixels.
fn centers(rgba: &[u8], width: u32) -> ((f32, f32), (f32, f32)) {
    let (mut lit, mut red) = ((0.0, 0.0, 0.0), (0.0, 0.0, 0.0));
    for (i, px) in rgba.as_chunks::<4>().0.iter().enumerate() {
        let (x, y) = (
            (i as u32 % width) as f32 + 0.5,
            (i as u32 / width) as f32 + 0.5,
        );
        if px[0] > 200 && px[1] < 60 {
            red = (red.0 + x, red.1 + y, red.2 + 1.0);
        } else if px[1] > 20 {
            let w = f32::from(px[1]);
            lit = (lit.0 + x * w, lit.1 + y * w, lit.2 + w);
        }
    }
    (
        (lit.0 / lit.2, lit.1 / lit.2),
        (red.0 / red.2, red.1 / red.2),
    )
}

#[test]
fn lights_follow_transforms_in_both_renderers() {
    let frame = FrameInfo {
        width: 960,
        height: 720,
        scale: 2.0,
        dt: 0.0,
    };
    let mut soft = SoftRenderer::new();
    soft.render_list(&scene(), &frame);
    let mut checks = vec![("soft", soft.rgba())];
    match common::OffscreenGl::new() {
        Ok(mut gl) => checks.push(("gl", gl.render(&scene(), &frame))),
        Err(e) => eprintln!("GL skipped: {e}"),
    }
    // Where the transform puts the lantern's center, in device pixels.
    let want = ((30.0 + 1.5 * 100.0) * 2.0, (18.0 + 1.5 * 80.0) * 2.0);
    for (name, rgba) in checks {
        let (lit, red) = centers(&rgba, frame.width);
        eprintln!("{name}: light at {lit:?}, lantern at {red:?}, expected {want:?}");
        for (got, label) in [(red, "lantern"), (lit, "light")] {
            assert!(
                (got.0 - want.0).abs() < 2.0 && (got.1 - want.1).abs() < 2.0,
                "{name}: {label} at {got:?}, expected {want:?}"
            );
        }
    }
}
