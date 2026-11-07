//! Goldens show the state an input leads to, not a frame partway through
//! its motion: after each input, motion runs half a second before the next
//! frame.

use astryx_core::{
    A11yAction, A11yNode, Command, DisplayList, FrameInfo, Input, InputPhase, Point, Preferences,
    Surface, TextMeasure,
};

pub struct Settled<'a, S: Surface> {
    inner: &'a mut S,
    pending: bool,
}

impl<S: Surface> Surface for Settled<'_, S> {
    fn input(&mut self, input: Input, phase: InputPhase) -> Command {
        self.pending = true;
        self.inner.input(input, phase)
    }
    fn frame(&mut self, frame: &FrameInfo, text: &dyn TextMeasure) -> DisplayList {
        if std::mem::take(&mut self.pending) {
            for _ in 0..30 {
                self.inner.frame(frame, text);
            }
        }
        self.inner.frame(frame, text)
    }
    fn accessibility(&self) -> Option<A11yNode> {
        self.inner.accessibility()
    }
    fn wants_frames(&self) -> bool {
        self.inner.wants_frames()
    }
    fn a11y_action(&mut self, id: u64, action: A11yAction) -> Command {
        self.inner.a11y_action(id, action)
    }
    fn text_input(&mut self, text: &str) -> Command {
        self.inner.text_input(text)
    }
    fn scroll(&mut self, at: Point, delta: Point) -> bool {
        self.inner.scroll(at, delta)
    }
    fn set_preferences(&mut self, preferences: &Preferences) {
        self.inner.set_preferences(preferences)
    }
}

/// `render_png` with motion settled after each input.
pub fn render_png<S: Surface>(
    surface: &mut S,
    width: u32,
    height: u32,
    scale: f32,
    inputs: &[Input],
) -> Vec<u8> {
    let mut settled = Settled {
        inner: surface,
        pending: false,
    };
    astryx_headless::render_png(&mut settled, width, height, scale, inputs)
}
