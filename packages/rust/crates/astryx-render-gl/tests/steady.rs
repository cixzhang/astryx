//! Perf guards: a steady lit frame with theme-font text and images costs
//! no font loads, rasterizing, uploads or textures, and its encode time
//! does not grow.

mod common;

use astryx_core::{
    Color, Dapple, DisplayList, DrawCmd, FontFamily, FrameInfo, ImageData, ImageFit, ImageId,
    Light, Lighting, Point, Radii, Rect, TextAlign, TextStyle, ThemeFont,
};

static FILES: [&[u8]; 1] = [include_bytes!(
    "../../astryx-theme-pixel/fonts/Galmuri9-Latin.ttf"
)];
// A family name the file does not use, as theme fonts may.
static FONT: ThemeFont = ThemeFont {
    family: "Guard Pixel",
    files: &FILES,
    em_pixels: 9,
    fallbacks: &[],
};

fn scene(time: f32) -> DisplayList {
    let text = |y: f32, s: &str| DrawCmd::Text {
        rect: Rect::new(20.0, y, 300.0, 20.0),
        text: s.into(),
        style: TextStyle {
            family: FontFamily::Theme(&FONT),
            size: 18.0,
            weight: 400,
            leading: 1.3,
            direction: None,
        },
        color: Color::rgba(1.0, 1.0, 1.0, 1.0),
        align: TextAlign::Start,
    };
    DisplayList {
        clear: Color::rgba(0.0, 0.0, 0.0, 1.0),
        commands: vec![
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
            DrawCmd::Image {
                rect: Rect::new(0.0, 0.0, 512.0, 384.0),
                radii: Radii::default(),
                image: ImageId(1),
                fit: ImageFit::Fill,
            },
            DrawCmd::Image {
                rect: Rect::new(200.0, 150.0, 32.0, 48.0),
                radii: Radii::default(),
                image: ImageId(2),
                fit: ImageFit::Fill,
            },
            DrawCmd::Light(Light {
                center: Point { x: 140.0, y: 250.0 },
                radius_x: 150.0,
                radius_y: 76.0,
                color: Color::rgba(1.0, 0.95, 0.72, 1.0),
                intensity: 1.0,
                flicker: Default::default(),
                pulse: Default::default(),
            }),
            DrawCmd::PopLighting,
            text(320.0, "Shrine terrace"),
            text(345.0, "CI passing, 3 beds ripe"),
        ],
    }
}

fn image(w: u32, h: u32) -> ImageData {
    let rgba: Vec<u8> = (0..w * h)
        .flat_map(|i| [(i % 251) as u8, 120, 60, 255])
        .collect();
    ImageData {
        width: w,
        height: h,
        rgba: rgba.into(),
    }
}

#[test]
fn steady_lit_frames_cost_nothing_new_and_do_not_grow() {
    let mut gl = match common::OffscreenGl::new() {
        Ok(gl) => gl,
        Err(e) => {
            eprintln!("skipping: {e}");
            return;
        }
    };
    gl.renderer.set_image(ImageId(1), Some(&image(256, 192)));
    gl.renderer.set_image(ImageId(2), Some(&image(16, 24)));
    let frame = FrameInfo {
        width: 1024,
        height: 768,
        scale: 2.0,
        dt: 1.0 / 60.0,
    };
    let mut early = std::time::Duration::ZERO;
    let mut generation = 0;
    for i in 0..900 {
        gl.render(&scene(i as f32 / 60.0), &frame);
        let s = gl.renderer.last_stats();
        if i == 10 {
            generation = astryx_core::TextMeasure::generation(gl.renderer.text());
        }
        if i >= 10 {
            assert_eq!(
                (s.glyphs, s.shadows, s.uploads, s.textures),
                (0, 0, 0, 0),
                "frame {i}: {s:?}"
            );
        }
        if (100..200).contains(&i) {
            early += s.encode;
        }
        if i >= 800 {
            // Allow noise, never growth.
            assert!(
                s.encode < early / 100 * 4 + std::time::Duration::from_micros(200),
                "frame {i}: encode {:?}, early average {:?}",
                s.encode,
                early / 100
            );
        }
    }
    assert_eq!(
        astryx_core::TextMeasure::generation(gl.renderer.text()),
        generation,
        "fonts were added on steady frames"
    );
}
