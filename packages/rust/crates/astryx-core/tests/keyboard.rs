//! OnScreenKeyboard through the runtime: typing, switching layouts, the
//! toast, accessibility, and fitting a 1024×768 screen at scale 2.

mod common;

use astryx_core::lab::{
    KeyboardAction, KeyboardKind, KeyboardState, KeyboardUpdate, OnScreenKeyboard,
};
use astryx_core::*;
use common::*;

#[derive(Default)]
struct Pad {
    state: KeyboardState,
    text: String,
    actions: Vec<KeyboardAction>,
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
        self.actions.extend(u.action);
        Command::None
    }
}

fn frame(rt: &mut Runtime<Pad>, w: u32, h: u32, scale: f32, dt: f32) -> DisplayList {
    rt.frame(
        &FrameInfo {
            width: w,
            height: h,
            scale,
            dt,
        },
        &measure(),
    )
}

fn press(rt: &mut Runtime<Pad>, input: Input) {
    rt.input(input, InputPhase::Pressed);
}

fn release(rt: &mut Runtime<Pad>, input: Input) {
    rt.input(input, InputPhase::Released);
}

fn tap(rt: &mut Runtime<Pad>, input: Input) {
    press(rt, input);
    release(rt, input);
}

fn all_nodes(rt: &Runtime<Pad>) -> Vec<A11yNode> {
    a11y_nodes(&rt.accessibility().unwrap())
}

#[test]
fn the_gamepad_types_into_the_focused_field() {
    let mut rt = Runtime::new(Pad::default());
    frame(&mut rt, 600, 500, 1.0, 0.0);
    // Hold down, press A then X; let go; tap up, press A.
    press(&mut rt, Input::Down);
    tap(&mut rt, Input::A);
    tap(&mut rt, Input::X);
    release(&mut rt, Input::Down);
    tap(&mut rt, Input::Up);
    tap(&mut rt, Input::A);
    tap(&mut rt, Input::R1);
    frame(&mut rt, 600, 500, 1.0, 0.0);
    assert_eq!(rt.app().text, "ale ");
    let field = all_nodes(&rt)
        .into_iter()
        .find(|n| n.role == Role::TextInput)
        .unwrap();
    assert_eq!(
        (field.label.as_deref(), field.value.as_deref()),
        (Some("Message"), Some("ale "))
    );
    assert!(field.focused, "the field is the focus stop");
    tap(&mut rt, Input::Start);
    tap(&mut rt, Input::Select);
    assert_eq!(
        rt.app().actions,
        [KeyboardAction::Done, KeyboardAction::Cancel]
    );
}

#[test]
fn choosing_a_group_announces_its_letters() {
    let mut rt = Runtime::new(Pad::default());
    frame(&mut rt, 600, 500, 1.0, 0.0);
    tap(&mut rt, Input::Right);
    frame(&mut rt, 600, 500, 1.0, 0.0);
    let live: Vec<String> = all_nodes(&rt)
        .into_iter()
        .filter(|n| n.live.is_some())
        .filter_map(|n| n.label.or(n.value))
        .collect();
    assert_eq!(live, ["right: t, n, d, m"]);
}

#[test]
fn a_keyboard_on_the_device_or_a_screen_reader_types_directly() {
    let mut rt = Runtime::new(Pad::default());
    frame(&mut rt, 600, 500, 1.0, 0.0);
    rt.text_input("hi");
    frame(&mut rt, 600, 500, 1.0, 0.0);
    assert_eq!(rt.app().text, "hi");
    tap(&mut rt, Input::Backspace);
    assert_eq!(rt.app().text, "h");
}

#[test]
fn l1_and_r1_together_switch_layouts_with_a_toast_that_hides() {
    let mut rt = Runtime::new(Pad::default());
    frame(&mut rt, 600, 500, 1.0, 0.0);
    press(&mut rt, Input::L1);
    press(&mut rt, Input::R1);
    release(&mut rt, Input::L1);
    release(&mut rt, Input::R1);
    let list = frame(&mut rt, 600, 500, 1.0, 0.0);
    assert_eq!(
        rt.app().actions,
        [KeyboardAction::Switched(KeyboardKind::Grid)]
    );
    assert_eq!(rt.app().text, "", "no delete, no space");
    assert!(
        texts(&list).iter().any(|t| t.1 == "Grid keyboard"),
        "the toast names it"
    );
    assert!(
        texts(&list).iter().any(|t| t.1 == "Space"),
        "the grid shows"
    );
    for _ in 0..100 {
        frame(&mut rt, 600, 500, 1.0, 1.0 / 60.0);
    }
    let list = frame(&mut rt, 600, 500, 1.0, 0.0);
    assert!(
        !texts(&list).iter().any(|t| t.1 == "Grid keyboard"),
        "the toast hides"
    );
    // The grid: right, A types w.
    tap(&mut rt, Input::Right);
    tap(&mut rt, Input::A);
    assert_eq!(rt.app().text, "w");
}

#[test]
fn both_layouts_fit_1024_by_768_at_scale_2() {
    for kind in [KeyboardKind::Chord, KeyboardKind::Grid] {
        let pad = Pad {
            state: KeyboardState::new(kind),
            ..Pad::default()
        };
        let mut rt = Runtime::new(pad);
        let list = frame(&mut rt, 1024, 768, 2.0, 0.0);
        let (w, h) = (512.0, 384.0);
        for (r, ..) in rects(&list) {
            assert!(
                r.x >= 0.0 && r.y >= 0.0 && r.x + r.width <= w + 0.5 && r.y + r.height <= h + 0.5,
                "{kind:?}: {r:?} is outside {w}×{h}"
            );
        }
        let field = all_nodes(&rt)
            .into_iter()
            .find(|n| n.role == Role::TextInput)
            .unwrap();
        assert!(field.bounds.y < 60.0, "the field is above the keys");
    }
}
