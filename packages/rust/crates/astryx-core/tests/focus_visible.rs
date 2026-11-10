//! The focus ring and the input that moved focus: keys, the D-pad and face
//! buttons always show it (a device with no pointer, such as the Brick,
//! shows it from the first frame); a pointer press hides it until the next
//! key or button.

use astryx_core::testing::draw;
use astryx_core::{
    A11yAction, App, Button, Command, DisplayList, DrawCmd, Element, Input, InputPhase, Surface,
    VStack, FOCUS_RING_WIDTH,
};

struct Buttons;

impl App for Buttons {
    type Msg = ();
    fn view(&self) -> Element<()> {
        VStack::new()
            .child(Button::new("One").on_click(()))
            .child(Button::new("Two").on_click(()))
            .child(Button::new("Three").on_click(()))
            .into()
    }
    fn update(&mut self, _: ()) -> Command {
        Command::None
    }
}

fn rings(list: &DisplayList) -> usize {
    list.commands
        .iter()
        .filter(|c| {
            matches!(c, DrawCmd::Rect { stroke: Some((_, w)), .. } if (*w - FOCUS_RING_WIDTH).abs() < 1e-3)
        })
        .count()
}

fn button(rt: &astryx_core::Runtime<Buttons>, label: &str) -> u64 {
    fn find(n: &astryx_core::A11yNode, label: &str) -> Option<u64> {
        if n.label.as_deref() == Some(label) {
            return Some(n.id);
        }
        n.children.iter().find_map(|c| find(c, label))
    }
    find(&rt.accessibility().unwrap(), label).unwrap()
}

fn press(rt: &mut astryx_core::Runtime<Buttons>, input: Input) -> DisplayList {
    rt.input(input, InputPhase::Pressed);
    rt.input(input, InputPhase::Released);
    draw(rt, 300, 300)
}

#[test]
fn a_gamepad_only_stream_always_shows_the_ring() {
    let mut rt = astryx_core::Runtime::new(Buttons);
    assert_eq!(
        rings(&draw(&mut rt, 300, 300)),
        1,
        "first frame, no input yet"
    );
    for input in [
        Input::Down,
        Input::Down,
        Input::Up,
        Input::A,
        Input::B,
        Input::Next,
        Input::L1,
    ] {
        assert_eq!(rings(&press(&mut rt, input)), 1, "after {input:?}");
    }
}

#[test]
fn a_pointer_press_hides_the_ring_until_the_next_button() {
    let mut rt = astryx_core::Runtime::new(Buttons);
    let tree = draw(&mut rt, 300, 300);
    assert_eq!(rings(&tree), 1);
    let two = button(&rt, "Two");
    rt.a11y_action(two, A11yAction::PressStart);
    rt.a11y_action(two, A11yAction::PressEnd);
    assert_eq!(
        rings(&draw(&mut rt, 300, 300)),
        0,
        "pointer focus is not visible"
    );
    assert_eq!(
        rings(&press(&mut rt, Input::A)),
        1,
        "a face button shows it again"
    );
    rt.a11y_action(two, A11yAction::PressStart);
    rt.a11y_action(two, A11yAction::PressEnd);
    assert_eq!(rings(&press(&mut rt, Input::Down)), 1, "so does the D-pad");
    // A screen reader moving focus is not a pointer.
    let one = button(&rt, "One");
    rt.a11y_action(one, A11yAction::Focus);
    assert_eq!(rings(&draw(&mut rt, 300, 300)), 1);
}
