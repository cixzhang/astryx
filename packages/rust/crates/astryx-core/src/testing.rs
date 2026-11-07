//! Helpers for tests: a deterministic text measurer and a frame driver.

use crate::contract::{
    Command, DisplayList, FrameInfo, Input, InputPhase, Size, Surface, TextMeasure, TextStyle,
};

/// Measures every character as `advance × size` points wide and wraps at
/// spaces, so layout tests do not depend on fonts.
#[derive(Clone, Copy, Debug)]
pub struct FixedMeasure {
    /// Width of one character as a multiple of the font size.
    pub advance: f32,
}

impl Default for FixedMeasure {
    fn default() -> Self {
        FixedMeasure { advance: 0.5 }
    }
}

impl FixedMeasure {
    fn width(&self, text: &str, style: &TextStyle) -> f32 {
        text.chars().count() as f32 * self.advance * style.size
    }
}

impl TextMeasure for FixedMeasure {
    fn measure(&self, text: &str, style: &TextStyle, max_width: Option<f32>) -> Size {
        let line_height = style.size * style.leading;
        let Some(max) = max_width else {
            return Size {
                width: self.width(text, style),
                height: line_height,
            };
        };
        let (mut lines, mut widest, mut line) = (1u32, 0.0f32, String::new());
        for word in text.split(' ') {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if self.width(&candidate, style) <= max + 0.001 || line.is_empty() {
                line = candidate;
            } else {
                widest = widest.max(self.width(&line, style));
                lines += 1;
                line = word.to_string();
            }
        }
        widest = widest.max(self.width(&line, style));
        Size {
            width: widest,
            height: lines as f32 * line_height,
        }
    }
}

/// A frame at scale 1 with no elapsed time.
pub fn frame(width: u32, height: u32) -> FrameInfo {
    FrameInfo {
        width,
        height,
        scale: 1.0,
        dt: 0.0,
    }
}

/// Draws one frame of `surface` at `width × height` points with [`FixedMeasure`].
pub fn draw(surface: &mut dyn Surface, width: u32, height: u32) -> DisplayList {
    surface.frame(&frame(width, height), &FixedMeasure::default())
}

/// Presses and releases `input`, returning the command of the release.
pub fn tap(surface: &mut dyn Surface, input: Input) -> Command {
    let pressed = surface.input(input, InputPhase::Pressed);
    let released = surface.input(input, InputPhase::Released);
    if pressed != Command::None {
        pressed
    } else {
        released
    }
}
