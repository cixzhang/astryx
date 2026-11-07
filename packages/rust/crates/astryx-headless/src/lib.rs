//! Headless Astryx shell. Drives a [`Surface`] with scripted input, draws it
//! with the CPU renderer, and returns PNG bytes. Includes golden image
//! helpers for tests and fixed display lists that cover every draw command.

pub mod fixtures;
pub mod golden;

use astryx_core::{FrameInfo, Input, InputPhase, Surface};
use astryx_render_soft::tiny_skia::Pixmap;
use astryx_render_soft::SoftRenderer;

pub use astryx_render_soft;

/// Seconds between scripted frames.
const FRAME_DT: f32 = 1.0 / 60.0;

/// The frame for a logical size at a scale. The pixel size rounds to whole pixels.
pub fn frame_info(width: u32, height: u32, scale: f32) -> FrameInfo {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    FrameInfo {
        width: ((width as f32 * scale).round() as u32).max(1),
        height: ((height as f32 * scale).round() as u32).max(1),
        scale,
        dt: FRAME_DT,
    }
}

/// Hands the surface's image changes to the renderer, as a shell does
/// before each frame it draws.
pub fn drain_images(renderer: &mut SoftRenderer, surface: &mut dyn Surface) {
    for (id, data) in surface.take_image_updates() {
        renderer.set_image(id, data.as_ref());
    }
}

/// Renders `surface` at `width`×`height` logical points and `scale` pixels
/// per point. It draws one frame, then presses and releases each input in
/// turn with a frame after each, and returns the last frame.
pub fn render_pixmap(
    renderer: &mut SoftRenderer,
    surface: &mut dyn Surface,
    width: u32,
    height: u32,
    scale: f32,
    inputs: &[Input],
) -> Pixmap {
    let frame = frame_info(width, height, scale);
    let list = surface.frame(&frame, renderer.text());
    for &input in inputs {
        surface.input(input, InputPhase::Pressed);
        surface.input(input, InputPhase::Released);
        let _ = surface.frame(&frame, renderer.text());
    }
    let list = if inputs.is_empty() {
        list
    } else {
        surface.frame(&frame, renderer.text())
    };
    drain_images(renderer, surface);
    renderer.render_list(&list, &frame);
    renderer.pixmap().clone()
}

/// [`render_pixmap`] with a fresh renderer, encoded as PNG.
pub fn render_png(
    surface: &mut dyn Surface,
    width: u32,
    height: u32,
    scale: f32,
    inputs: &[Input],
) -> Vec<u8> {
    let mut renderer = SoftRenderer::new();
    render_pixmap(&mut renderer, surface, width, height, scale, inputs);
    renderer.encode_png()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fixtures::FixtureSurface;

    #[test]
    fn png_has_the_scaled_size() {
        let mut s = FixtureSurface::new("shapes").unwrap();
        let png = render_png(&mut s, 100, 50, 2.0, &[]);
        let img = Pixmap::decode_png(&png).unwrap();
        assert_eq!((img.width(), img.height()), (200, 100));
    }

    #[test]
    fn inputs_change_the_frame() {
        let mut a = FixtureSurface::new("gallery").unwrap();
        let mut b = FixtureSurface::new("gallery").unwrap();
        let before = render_png(&mut a, 480, 320, 1.0, &[]);
        let after = render_png(&mut b, 480, 320, 1.0, &[Input::Next, Input::Next, Input::A]);
        assert_ne!(before, after);
    }
}
