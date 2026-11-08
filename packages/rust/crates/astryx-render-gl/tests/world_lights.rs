//! A lantern sprite and its LightSource, placed at one board point, stay
//! together under CameraFollow after focus moves, and in the middle of a
//! glide, in both renderers.

mod common;

use astryx_core::paged::CameraFollow;
use astryx_core::world::{Board, LightSource, Lit, Sprite};
use astryx_core::{
    App, Color, Command, Element, FrameInfo, ImageData, ImageId, Input, InputPhase, Light,
    Lighting, Point, Runtime, Surface,
};
use astryx_render_soft::SoftRenderer;

const LANTERN: ImageId = ImageId(500);
const TILE: ImageId = ImageId(501);

struct World {
    lantern_x: f32,
}

impl App for World {
    type Msg = f32;
    fn view(&self) -> Element<f32> {
        let mut board = Board::new(1200.0, 300.0);
        for i in 0..10 {
            board = board.child_at(
                i as f32 * 120.0,
                20.0,
                Sprite::new(TILE)
                    .size(32.0, 32.0)
                    .label(format!("t{i}"))
                    .on_click(0.0),
            );
        }
        let (x, y) = (self.lantern_x, 240.0);
        board = board
            .child_at(x, y, Sprite::new(LANTERN).size(8.0, 8.0))
            .child_at(
                x + 4.0,
                y + 4.0,
                LightSource::new(Light {
                    center: Point { x: 0.0, y: 0.0 },
                    radius_x: 30.0,
                    radius_y: 30.0,
                    color: Color::rgba(1.0, 1.0, 1.0, 1.0),
                    intensity: 1.0,
                    flicker: Default::default(),
                    pulse: Default::default(),
                }),
            );
        Lit::new(Lighting {
            ambient: Color::rgba(0.0, 0.0, 0.0, 1.0),
            strength: 1.0,
            time: 0.0,
            dapple: None,
        })
        .child(CameraFollow::new(board))
        .into()
    }
    fn update(&mut self, x: f32) -> Command {
        self.lantern_x = x;
        Command::None
    }
}

fn solid(rgba: [u8; 4]) -> ImageData {
    ImageData {
        width: 2,
        height: 2,
        rgba: rgba.repeat(4).into(),
    }
}

/// The centroids of red (lantern) pixels and of other lit pixels.
fn centers(rgba: &[u8], width: u32) -> ((f32, f32), (f32, f32)) {
    let (mut lit, mut red) = ((0.0, 0.0, 0.0), (0.0, 0.0, 0.0));
    for (i, px) in rgba.as_chunks::<4>().0.iter().enumerate() {
        let (x, y) = (
            (i as u32 % width) as f32 + 0.5,
            (i as u32 / width) as f32 + 0.5,
        );
        if px[0] > 100 && px[1] < 30 && px[2] < 30 {
            red = (red.0 + x, red.1 + y, red.2 + 1.0);
        } else if px[1] > 10 && px[0].abs_diff(px[1]) < 40 && px[1].abs_diff(px[2]) < 40 {
            // Gray lit by the white light; the unlit blue focus ring is left out.
            let w = f32::from(px[1]);
            lit = (lit.0 + x * w, lit.1 + y * w, lit.2 + w);
        }
    }
    (
        (lit.0 / lit.2, lit.1 / lit.2),
        (red.0 / red.2, red.1 / red.2),
    )
}

fn check(name: &str, rgba: &[u8], width: u32) {
    let (lit, red) = centers(rgba, width);
    eprintln!("{name}: light {lit:?}, lantern {red:?}");
    if red.0.is_nan() {
        // Off screen: the light must be too.
        assert!(
            lit.0.is_nan(),
            "{name}: a light shows without its lantern at {lit:?}"
        );
        return;
    }
    assert!(
        (lit.0 - red.0).abs() < 2.0 && (lit.1 - red.1).abs() < 2.0,
        "{name}: light at {lit:?}, lantern at {red:?}"
    );
}

#[test]
fn lights_move_with_their_sprites() {
    let mut gl = common::OffscreenGl::new().ok();
    if gl.is_none() {
        eprintln!("GL skipped");
    }
    let mut soft = SoftRenderer::new();
    let measure = astryx_fonts::engine();
    let mut rt = Runtime::new(World { lantern_x: 760.0 })
        .with_image(LANTERN, Some(solid([255, 0, 0, 255])))
        .with_image(TILE, Some(solid([90, 90, 90, 255])));
    let frame = FrameInfo {
        width: 800,
        height: 600,
        scale: 2.0,
        dt: 1.0 / 60.0,
    };
    let mut draw = |rt: &mut Runtime<World>, label: &str, dt: f32| {
        let frame = FrameInfo { dt, ..frame };
        for (id, data) in rt.take_image_updates() {
            soft.set_image(id, data.as_ref());
            if let Some(gl) = &mut gl {
                gl.renderer.set_image(id, data.as_ref());
            }
        }
        let list = rt.frame(&frame, soft.text());
        soft.render_list(&list, &frame);
        check(&format!("{label}, soft"), &soft.rgba(), frame.width);
        if let Some(gl) = &mut gl {
            let rgba = gl.render(&list, &frame);
            check(&format!("{label}, gl"), &rgba, frame.width);
        }
    };
    draw(&mut rt, "first frame", 0.0);
    // Walk focus right until the camera has scrolled the lantern into view.
    for _ in 0..6 {
        rt.input(Input::Right, InputPhase::Pressed);
        rt.input(Input::Right, InputPhase::Released);
        rt.frame(&FrameInfo { dt: 1.0, ..frame }, &measure);
    }
    draw(&mut rt, "camera moved", 1.0);
    // Move the lantern: both glide on the same motion.
    rt.send(800.0);
    draw(&mut rt, "glide start", 1.0 / 60.0);
    draw(&mut rt, "mid glide", 0.06);
    draw(&mut rt, "glide end", 1.0);
}
