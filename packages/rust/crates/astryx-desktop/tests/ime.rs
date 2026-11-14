//! A composition from an input method, as the desktop shell turns winit's
//! events into surface calls, drives a real runtime: the composing text
//! draws inline at the caret, underlined, and the commit types it.

use astryx_core::testing::{draw, tap};
use astryx_core::*;
use astryx_desktop::input::{apply_ime, ime_area, ime_steps};
use winit::event::Ime;

struct Field {
    value: String,
}

impl App for Field {
    type Msg = String;
    fn view(&self) -> Element<String> {
        // A button after the field, so focus can leave it.
        VStack::new()
            .child(TextInput::new("Name", self.value.clone()).on_change(|v| v))
            .child(Button::new("Done").on_click(String::new()))
            .into()
    }
    fn update(&mut self, v: String) -> Command {
        if !v.is_empty() {
            self.value = v;
        }
        Command::None
    }
}

fn texts(list: &DisplayList) -> Vec<String> {
    list.commands
        .iter()
        .filter_map(|c| match c {
            DrawCmd::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn field() -> Runtime<Field> {
    let mut rt = Runtime::new(Field { value: "ab".into() });
    draw(&mut rt, 300, 200);
    // Focus the field, as a click does.
    let id = {
        fn find(n: &A11yNode) -> Option<u64> {
            if n.role == Role::TextInput {
                return Some(n.id);
            }
            n.children.iter().find_map(find)
        }
        find(&rt.accessibility().unwrap()).unwrap()
    };
    rt.a11y_action(id, A11yAction::Focus);
    draw(&mut rt, 300, 200);
    rt
}

fn ime(rt: &mut Runtime<Field>, event: Ime) -> Command {
    apply_ime(rt, ime_steps(&event))
}

#[test]
fn a_composition_draws_inline_and_commits_into_the_field() {
    let mut rt = field();
    ime(&mut rt, Ime::Enabled);
    ime(&mut rt, Ime::Preedit("か".into(), Some((3, 3))));
    assert!(rt.wants_frames(), "a composing change asks for a frame");
    let list = draw(&mut rt, 300, 200);
    assert!(
        texts(&list).contains(&"abか".to_string()),
        "inline: {:?}",
        texts(&list)
    );
    assert_eq!(rt.app().value, "ab", "composing does not change the value");

    ime(&mut rt, Ime::Preedit(String::new(), None));
    ime(&mut rt, Ime::Commit("か".into()));
    let list = draw(&mut rt, 300, 200);
    assert_eq!(rt.app().value, "abか");
    assert!(texts(&list).contains(&"abか".to_string()));
}

#[test]
fn turning_the_input_method_off_drops_the_composition() {
    let mut rt = field();
    ime(&mut rt, Ime::Preedit("に".into(), Some((3, 3))));
    draw(&mut rt, 300, 200);
    ime(&mut rt, Ime::Disabled);
    let list = draw(&mut rt, 300, 200);
    assert_eq!(rt.app().value, "ab");
    assert!(
        !texts(&list).iter().any(|t| t.contains('に')),
        "{:?}",
        texts(&list)
    );
}

#[test]
fn the_input_method_works_at_the_focused_field_while_composing() {
    let mut rt = field();
    let field_rect = |rt: &Runtime<Field>| {
        fn find(n: &A11yNode) -> Option<Rect> {
            if n.role == Role::TextInput {
                return Some(n.bounds);
            }
            n.children.iter().find_map(find)
        }
        find(&rt.accessibility().unwrap()).unwrap()
    };
    let inside = |r: Rect, f: Rect| {
        r.x >= f.x && r.x <= f.x + f.width && r.y >= f.y && r.y + r.height <= f.y + f.height + 0.5
    };
    let at = ime_area(rt.accessibility().as_ref()).expect("a focused field");
    assert!(
        inside(at, field_rect(&rt)) && at.width <= 2.0,
        "the candidate window sits at the caret, in the field: {at:?}"
    );
    ime(&mut rt, Ime::Preedit("かな".into(), Some((6, 6))));
    draw(&mut rt, 300, 200);
    let composing = ime_area(rt.accessibility().as_ref()).expect("still the field");
    assert!(
        inside(composing, field_rect(&rt)) && composing.x > at.x,
        "while composing it follows the caret to the end of the composition: {at:?} -> {composing:?}"
    );
    // Focus leaves the field (to the button): the input method turns off.
    tap(&mut rt, Input::Next);
    draw(&mut rt, 300, 200);
    assert_eq!(ime_area(rt.accessibility().as_ref()), None);
}
