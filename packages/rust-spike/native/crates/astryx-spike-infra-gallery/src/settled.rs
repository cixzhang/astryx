//! A surface whose first frame is its settled frame, for captures: the
//! first `frame` call runs the surface's animations to rest (layer
//! entrances, springs) before it returns, so a one-frame screenshot shows
//! what a person sees once the screen is still, as a browser capture taken
//! after animations end does.

use astryx_core::contract::{
    A11yAction, A11yNode, Command, DisplayList, FrameInfo, ImageData, ImageId, Input, InputPhase,
    KeyEvent, KeyOutcome, Point, PointerEvent, PointerOutcome, Preferences, SoundId, Surface,
    TextMeasure,
};

/// The most frames the first frame runs: 4 seconds at 60 frames a second.
const MAX_SETTLE_FRAMES: u32 = 240;

pub struct Settled<S> {
    inner: S,
    settled: bool,
}

impl<S: Surface> Settled<S> {
    pub fn new(inner: S) -> Self {
        Settled {
            inner,
            settled: false,
        }
    }
}

impl<S: Surface> Surface for Settled<S> {
    fn input(&mut self, input: Input, phase: InputPhase) -> Command {
        self.inner.input(input, phase)
    }

    fn frame(&mut self, frame: &FrameInfo, text: &dyn TextMeasure) -> DisplayList {
        let mut list = self.inner.frame(frame, text);
        if !self.settled {
            self.settled = true;
            let step = FrameInfo {
                dt: 1.0 / 60.0,
                ..*frame
            };
            let mut n = 0;
            while self.inner.wants_frames() && n < MAX_SETTLE_FRAMES {
                list = self.inner.frame(&step, text);
                n += 1;
            }
        }
        list
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

    fn text_compose(&mut self, text: &str, cursor: u32) {
        self.inner.text_compose(text, cursor)
    }

    fn scroll(&mut self, at: Point, delta: Point) -> bool {
        self.inner.scroll(at, delta)
    }

    fn pointer_move(&mut self, at: Option<Point>) {
        self.inner.pointer_move(at)
    }

    fn pointer(&mut self, event: PointerEvent) -> PointerOutcome {
        self.inner.pointer(event)
    }

    fn key(&mut self, event: KeyEvent) -> KeyOutcome {
        self.inner.key(event)
    }

    fn take_command(&mut self) -> Command {
        self.inner.take_command()
    }

    fn take_image_updates(&mut self) -> Vec<(ImageId, Option<ImageData>)> {
        self.inner.take_image_updates()
    }

    fn take_sound_updates(&mut self) -> Vec<(SoundId, Option<std::sync::Arc<[u8]>>)> {
        self.inner.take_sound_updates()
    }

    fn set_preferences(&mut self, preferences: &Preferences) {
        self.inner.set_preferences(preferences)
    }
}
