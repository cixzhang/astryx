//! Images go to the GPU when a frame first draws them. What a frame shows
//! is the same as uploading at once: the latest image set for an id, and
//! nothing for one freed.

mod common;

use std::sync::Arc;

use astryx_core::{
    Color, DisplayList, DrawCmd, FrameInfo, ImageData, ImageFit, ImageId, Radii, Rect,
};

fn frame() -> FrameInfo {
    FrameInfo {
        width: 32,
        height: 32,
        scale: 1.0,
        dt: 0.0,
    }
}

fn solid(rgba: [u8; 4]) -> ImageData {
    ImageData {
        width: 4,
        height: 4,
        rgba: Arc::from(rgba.repeat(16)),
    }
}

fn draw(id: ImageId) -> DisplayList {
    DisplayList {
        clear: Color::rgba(1.0, 1.0, 1.0, 1.0),
        commands: vec![DrawCmd::Image {
            rect: Rect::new(0.0, 0.0, 32.0, 32.0),
            radii: Radii::all(0.0),
            image: id,
            fit: ImageFit::Fill,
        }],
    }
}

fn blank() -> DisplayList {
    DisplayList {
        clear: Color::rgba(1.0, 1.0, 1.0, 1.0),
        commands: Vec::new(),
    }
}

/// The pixel in the middle of the frame.
fn middle(px: &[u8]) -> [u8; 4] {
    let i = (16 * 32 + 16) * 4;
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}

#[test]
fn an_image_shows_as_last_set_when_first_drawn() {
    let mut gl = match common::OffscreenGl::new() {
        Ok(gl) => gl,
        Err(e) => {
            println!("images_on_first_draw: skipped, no EGL device: {e}");
            return;
        }
    };
    let (red, blue, gone) = (ImageId::of("red"), ImageId::of("blue"), ImageId::of("gone"));
    gl.renderer.set_image(red, Some(&solid([255, 0, 0, 255])));
    gl.renderer.set_image(blue, Some(&solid([0, 255, 0, 255])));
    // Replaced before any frame drew it.
    gl.renderer.set_image(blue, Some(&solid([0, 0, 255, 255])));
    gl.renderer.set_image(gone, Some(&solid([0, 0, 0, 255])));
    // Freed before any frame drew it.
    gl.renderer.set_image(gone, None);
    // A frame that draws none of them, then frames that do.
    gl.render(&blank(), &frame());
    assert_eq!(middle(&gl.render(&draw(red), &frame())), [255, 0, 0, 255]);
    assert_eq!(middle(&gl.render(&draw(blue), &frame())), [0, 0, 255, 255]);
    assert_eq!(
        middle(&gl.render(&draw(gone), &frame())),
        [255, 255, 255, 255],
        "a freed image draws nothing"
    );
    // Replaced after it was drawn.
    gl.renderer.set_image(red, Some(&solid([0, 255, 0, 255])));
    assert_eq!(middle(&gl.render(&draw(red), &frame())), [0, 255, 0, 255]);
}
