//! Images drawn on a pixel grid stay crisp: every device pixel shows one
//! image pixel, never a blend, with or without the fast programs, under a
//! camera offset, and when the same image is also drawn off the grid in
//! the same frame.

mod common;

use astryx_core::{
    Color, DisplayList, DrawCmd, FrameInfo, ImageData, ImageFit, ImageId, Point, Radii, Rect,
};
use std::collections::BTreeSet;

const SPRITE: ImageId = ImageId(7);
const A: [u8; 4] = [200, 40, 60, 255];
const B: [u8; 4] = [30, 160, 90, 255];

/// A 4x4 checkerboard of A and B.
fn checker() -> ImageData {
    let mut rgba = Vec::new();
    for y in 0..4 {
        for x in 0..4 {
            rgba.extend_from_slice(if (x + y) % 2 == 0 { &A } else { &B });
        }
    }
    ImageData {
        width: 4,
        height: 4,
        rgba: rgba.into(),
    }
}

fn image(x: f32, y: f32) -> DrawCmd {
    DrawCmd::Image {
        rect: Rect::new(x, y, 32.0, 32.0),
        radii: Radii::default(),
        image: SPRITE,
        fit: ImageFit::Fill,
    }
}

/// Colors inside `area` (device pixels, inset by one) of a frame.
fn colors(rgba: &[u8], width: u32, area: (u32, u32, u32, u32)) -> BTreeSet<[u8; 4]> {
    let (x0, y0, x1, y1) = area;
    let mut out = BTreeSet::new();
    for y in y0 + 1..y1 - 1 {
        for x in x0 + 1..x1 - 1 {
            let i = ((y * width + x) * 4) as usize;
            out.insert([rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]);
        }
    }
    out
}

#[test]
fn grid_images_are_crisp_in_every_case() {
    let mut gl = match common::OffscreenGl::new() {
        Ok(gl) => gl,
        Err(e) => {
            eprintln!("skipping: {e}");
            return;
        }
    };
    gl.renderer.set_image(SPRITE, Some(&checker()));
    let frame = FrameInfo {
        width: 256,
        height: 128,
        scale: 2.0,
        dt: 0.0,
    };
    let grid = |cmds: Vec<DrawCmd>| {
        let mut v = vec![DrawCmd::PushPixelGrid { cell: 1.0 }];
        v.extend(cmds);
        v.push(DrawCmd::PopPixelGrid);
        v
    };
    let cases: Vec<(&str, Vec<DrawCmd>)> = vec![
        ("on the grid", grid(vec![image(0.0, 0.0)])),
        (
            "on the grid under a camera offset",
            grid(vec![
                DrawCmd::PushTransform {
                    origin: Point { x: 0.0, y: 0.0 },
                    scale: 1.0,
                    translate: Point { x: 0.25, y: 0.75 },
                },
                image(0.0, 0.0),
                DrawCmd::PopTransform,
            ]),
        ),
        (
            "also drawn off the grid after it, in the same frame",
            [grid(vec![image(0.0, 0.0)]), vec![image(64.0, 0.0)]].concat(),
        ),
        (
            "also drawn off the grid before it, in the same frame",
            [vec![image(64.0, 0.0)], grid(vec![image(0.0, 0.0)])].concat(),
        ),
    ];
    for fast in [true, false] {
        gl.renderer.set_fast_paths(fast);
        for (name, commands) in &cases {
            let list = DisplayList {
                clear: Color::rgba(0.0, 0.0, 0.0, 1.0),
                commands: commands.clone(),
            };
            let rgba = gl.render(&list, &frame);
            if let Some(dir) = std::env::var_os("PIXEL_IMAGES_OUT") {
                let path = std::path::Path::new(&dir).join(format!("{name}-fast-{fast}.rgba"));
                std::fs::write(path, &rgba).unwrap();
            }
            let seen = colors(&rgba, frame.width, (3, 3, 61, 61));
            assert_eq!(
                seen,
                BTreeSet::from([A, B]),
                "{name} (fast paths {fast}): {} colors",
                seen.len()
            );
            if name.contains("off the grid") {
                let smooth = colors(&rgba, frame.width, (128, 0, 192, 64));
                assert!(smooth.len() > 2, "{name}: the off-grid copy stays smooth");
            }
        }
    }
}
