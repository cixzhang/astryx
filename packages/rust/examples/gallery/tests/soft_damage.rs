//! The CPU renderer redraws only what changed since its last frame, and
//! paints in bands. On every gallery section, while focus moves, controls
//! are pressed and the page scrolls, each frame it draws must equal, pixel
//! for pixel, the same frame drawn whole by a new renderer.

use astryx_core::{FrameInfo, ImageData, ImageId, Input, InputPhase, Point, Surface};
use astryx_gallery::{runtime, Gallery, Section};
use astryx_render_soft::SoftRenderer;

#[test]
fn gallery_frames_equal_the_same_frames_drawn_whole() {
    for section in Section::ALL {
        // Every section at 1x; a few at 2x, where pixels and shapes are
        // twice the size.
        let hi_dpi = matches!(
            section,
            Section::Buttons | Section::Switches | Section::Cards
        );
        for scale in [1.0f32, 2.0] {
            if scale > 1.0 && !hi_dpi {
                continue;
            }
            let frame = FrameInfo {
                width: (720.0 * scale) as u32,
                height: (900.0 * scale) as u32,
                scale,
                dt: 1.0 / 60.0,
            };
            let mut rt = runtime(Gallery::section(section));
            let mut kept = SoftRenderer::new();
            let mut images: Vec<(ImageId, Option<ImageData>)> = Vec::new();
            for step in 0..8 {
                match step {
                    1 | 2 => {
                        rt.input(Input::Next, InputPhase::Pressed);
                        rt.input(Input::Next, InputPhase::Released);
                    }
                    3 => {
                        rt.input(Input::A, InputPhase::Pressed);
                        rt.input(Input::A, InputPhase::Released);
                    }
                    5 => {
                        rt.scroll(Point { x: 360.0, y: 450.0 }, Point { x: 0.0, y: 40.0 });
                    }
                    _ => {}
                }
                let mut fresh = SoftRenderer::new();
                let list = rt.frame(&frame, fresh.text());
                for (id, data) in rt.take_image_updates() {
                    kept.set_image(id, data.as_ref());
                    images.push((id, data));
                }
                for (id, data) in &images {
                    fresh.set_image(*id, data.as_ref());
                }
                kept.render_list(&list, &frame);
                fresh.render_list(&list, &frame);
                assert!(
                    kept.pixmap().data() == fresh.pixmap().data(),
                    "{} at {scale}x, frame {step}: the redrawn frame differs from the whole frame",
                    section.slug()
                );
            }
        }
    }
}
