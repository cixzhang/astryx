//! Text selection in a field: Ctrl or Cmd+A selects all, Shift with the
//! arrows, Home and End extends, typing and Backspace replace a selection,
//! and assistive technology sees the selection.

use astryx_core::testing::draw;
use astryx_core::{
    App, Command, Element, Input, InputPhase, Key, KeyEvent, Modifiers, Surface, TextInput, VStack,
};

struct Form {
    value: String,
}

#[derive(Clone)]
enum Msg {
    Text(String),
}

impl App for Form {
    type Msg = Msg;
    fn view(&self) -> Element<Msg> {
        VStack::new()
            .child(TextInput::new("Name", self.value.clone()).on_change(Msg::Text))
            .into()
    }
    fn update(&mut self, msg: Msg) -> Command {
        let Msg::Text(v) = msg;
        self.value = v;
        Command::None
    }
}

fn rt(value: &str) -> astryx_core::Runtime<Form> {
    let mut rt = astryx_core::Runtime::new(Form {
        value: value.into(),
    });
    draw(&mut rt, 300, 200);
    rt
}

fn key(rt: &mut astryx_core::Runtime<Form>, key: Key, modifiers: Modifiers) -> bool {
    let e = |phase| KeyEvent {
        key: key.clone(),
        phase,
        modifiers,
        repeat: false,
    };
    let handled = rt.key(e(InputPhase::Pressed)).handled;
    rt.key(e(InputPhase::Released));
    draw(rt, 300, 200);
    handled
}

fn ctrl() -> Modifiers {
    Modifiers {
        ctrl: true,
        ..Modifiers::default()
    }
}

fn shift() -> Modifiers {
    Modifiers {
        shift: true,
        ..Modifiers::default()
    }
}

fn type_text(rt: &mut astryx_core::Runtime<Form>, s: &str) {
    rt.text_input(s);
    draw(rt, 300, 200);
}

fn selection(rt: &astryx_core::Runtime<Form>) -> Option<(u32, u32)> {
    fn find(n: &astryx_core::A11yNode) -> Option<(u32, u32)> {
        n.text_selection
            .or_else(|| n.children.iter().find_map(find))
    }
    find(&rt.accessibility()?)
}

#[test]
fn select_all_then_typing_replaces() {
    let mut rt = rt("ab日XY");
    assert!(key(&mut rt, Key::Character("a".into()), ctrl()));
    assert_eq!(selection(&rt), Some((0, "ab日XY".len() as u32)));
    type_text(&mut rt, "z");
    assert_eq!(rt.app().value, "z");
    let cmd = Modifiers {
        meta: true,
        ..Modifiers::default()
    };
    assert!(key(&mut rt, Key::Character("A".into()), cmd), "Cmd+A too");
    rt.input(Input::Backspace, InputPhase::Pressed);
    draw(&mut rt, 300, 200);
    assert_eq!(rt.app().value, "", "Backspace deletes the selection");
}

#[test]
fn shift_arrows_extend_and_plain_arrows_collapse() {
    let mut rt = rt("hello");
    // The caret starts at the end.
    assert!(key(&mut rt, Key::Left, shift()));
    assert!(key(&mut rt, Key::Left, shift()));
    assert_eq!(
        selection(&rt),
        Some((5, 3)),
        "anchor at the end, caret moved back"
    );
    type_text(&mut rt, "P");
    assert_eq!(rt.app().value, "helP");
    assert!(key(&mut rt, Key::Home, shift()));
    assert!(
        key(&mut rt, Key::Right, Modifiers::default()),
        "collapses to the end"
    );
    type_text(&mut rt, "!");
    assert_eq!(rt.app().value, "helP!");
    // Without a selection, plain arrows are the runtime's (caret or focus).
    assert!(!key(&mut rt, Key::Left, Modifiers::default()));
}
