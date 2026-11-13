//! A settled app wants no frames, so a shell can sleep; each thing that
//! changes the screen wants frames until it has been drawn, then stops.

mod common;

use astryx_core::lab::{KeyboardState, KeyboardUpdate, OnScreenKeyboard};
use astryx_core::testing::tap;
use astryx_core::*;
use common::*;

/// Draws 1/60 s frames until the surface stops wanting them; the count.
fn settle<A: App>(rt: &mut Runtime<A>) -> usize {
    for n in 0..600 {
        if !rt.wants_frames() {
            return n;
        }
        frame(rt, 1.0 / 60.0);
    }
    panic!("still wants frames after 10 s");
}

fn frame<A: App>(rt: &mut Runtime<A>, dt: f32) -> DisplayList {
    rt.frame(
        &FrameInfo {
            width: 400,
            height: 600,
            scale: 1.0,
            dt,
        },
        &measure(),
    )
}

/// Settled, it stays settled: more frames and no input change nothing.
fn assert_idle<A: App>(rt: &mut Runtime<A>, what: &str) {
    settle(rt);
    for _ in 0..30 {
        frame(rt, 1.0 / 60.0);
        assert!(!rt.wants_frames(), "{what}: idle");
    }
}

#[derive(Default)]
struct Screen {
    count: u32,
    loading: bool,
    focus: Option<&'static str>,
}

impl App for Screen {
    type Msg = &'static str;
    fn view(&self) -> Element<&'static str> {
        VStack::new()
            .child(
                Button::new(format!("Count {}", self.count))
                    .id("count")
                    .on_click("count"),
            )
            .child(
                Button::new("Load")
                    .id("load")
                    .is_loading(self.loading)
                    .on_click("load"),
            )
            .child(Button::new("Focus last").id("refocus").on_click("refocus"))
            .child(Button::new("Last").id("last").on_click("last"))
            .into()
    }
    fn update(&mut self, msg: &'static str) -> Command {
        match msg {
            "count" => self.count += 1,
            "load" => self.loading = !self.loading,
            "refocus" => return Command::Focus(self.focus.unwrap_or("last").into()),
            _ => {}
        }
        Command::None
    }
}

fn screen() -> Runtime<Screen> {
    let mut rt = Runtime::new(Screen::default());
    frame(&mut rt, 0.0);
    assert_idle(&mut rt, "first frame");
    rt
}

#[test]
fn a_settled_app_wants_no_frames() {
    let mut rt = screen();
    assert!(!rt.wants_frames());
    assert_idle(&mut rt, "settled");
}

#[test]
fn an_input_that_moves_focus_wants_a_frame_then_settles() {
    let mut rt = screen();
    tap(&mut rt, Input::Down);
    assert!(rt.wants_frames(), "focus moved");
    assert_idle(&mut rt, "after the move");
}

#[test]
fn an_input_that_changes_nothing_wants_no_frame() {
    let mut rt = screen();
    tap(&mut rt, Input::Up);
    tap(&mut rt, Input::Up);
    settle(&mut rt);
    // Up from the first stop has nowhere to go; a blocked cue may draw a
    // frame, then nothing.
    tap(&mut rt, Input::Up);
    assert_idle(&mut rt, "blocked");
}

#[test]
fn a_message_that_changes_state_wants_a_frame_then_settles() {
    let mut rt = screen();
    tap(&mut rt, Input::A);
    assert_eq!(rt.app().count, 1);
    assert!(rt.wants_frames(), "rebuild");
    assert_idle(&mut rt, "after the rebuild");
    // From outside a message, as a shell's preferences or a loader do.
    rt.app_mut().count = 5;
    assert!(rt.wants_frames());
    assert_idle(&mut rt, "after app_mut");
}

#[test]
fn an_image_load_wants_a_frame_then_settles() {
    let mut rt = screen();
    rt.register_image(
        ImageId(9),
        Some(ImageData {
            width: 1,
            height: 1,
            rgba: vec![255u8, 0, 0, 255].into(),
        }),
    );
    assert!(
        rt.wants_frames(),
        "the image goes to the renderer on a frame"
    );
    // A shell takes the pixels when it draws that frame.
    assert_eq!(rt.take_image_updates().len(), 1);
    assert!(settle(&mut rt) >= 1);
    assert_idle(&mut rt, "after the image");
}

#[test]
fn a_focus_request_wants_the_frame_that_carries_it_out() {
    let mut rt = screen();
    for _ in 0..2 {
        tap(&mut rt, Input::Down);
    }
    settle(&mut rt);
    assert_eq!(tap(&mut rt, Input::A), Command::Handled);
    assert!(rt.wants_frames(), "the rebuild the request waits on");
    settle(&mut rt);
    let focused = a11y_nodes(&rt.accessibility().unwrap())
        .into_iter()
        .find(|n| n.focused)
        .and_then(|n| n.label);
    assert_eq!(focused.as_deref(), Some("Last"));
    assert_idle(&mut rt, "after the focus request");
}

#[test]
fn a_spinner_wants_frames_while_shown_and_stops_when_it_goes() {
    let mut rt = screen();
    tap(&mut rt, Input::Down);
    tap(&mut rt, Input::A);
    assert!(rt.app().loading);
    for _ in 0..120 {
        frame(&mut rt, 1.0 / 60.0);
        assert!(rt.wants_frames(), "the spinner turns");
    }
    rt.app_mut().loading = false;
    assert_idle(&mut rt, "after the spinner");
}

#[test]
fn holding_a_direction_wants_frames_for_the_repeat_and_stops_on_release() {
    let mut rt = screen();
    rt.input(Input::Down, InputPhase::Pressed);
    for _ in 0..60 {
        frame(&mut rt, 1.0 / 60.0);
        assert!(rt.wants_frames(), "the repeat runs on frames");
    }
    rt.input(Input::Down, InputPhase::Released);
    assert_idle(&mut rt, "after the hold");
}

#[derive(Default)]
struct Pad {
    state: KeyboardState,
    text: String,
}

impl App for Pad {
    type Msg = KeyboardUpdate;
    fn view(&self) -> Element<KeyboardUpdate> {
        VStack::new()
            .child(
                OnScreenKeyboard::new("Message", self.text.clone(), self.state.clone())
                    .on_change(|u| u),
            )
            .into()
    }
    fn update(&mut self, u: KeyboardUpdate) -> Command {
        self.state = u.state;
        self.text = u.value;
        Command::None
    }
}

#[test]
fn a_pending_timer_wants_frames_until_it_fires() {
    let mut rt = Runtime::new(Pad::default());
    frame(&mut rt, 0.0);
    assert_idle(&mut rt, "keyboard");
    // L1 and R1 together switch layouts and show a toast that hides on a
    // timer.
    rt.input(Input::L1, InputPhase::Pressed);
    rt.input(Input::R1, InputPhase::Pressed);
    rt.input(Input::L1, InputPhase::Released);
    rt.input(Input::R1, InputPhase::Released);
    frame(&mut rt, 0.0);
    assert!(rt.app().state.toast.is_some());
    let frames = settle(&mut rt);
    assert!(
        frames > 30,
        "frames until the toast's timer fired: {frames}"
    );
    assert!(rt.app().state.toast.is_none(), "the timer fired");
    assert_idle(&mut rt, "after the timer");
}
