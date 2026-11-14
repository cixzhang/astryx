//! Input method composition in the spike's text fields: the composing text
//! shows inline at the caret, underlined, with the caret inside it; the
//! accessibility tree carries the caret's bounds for the candidate window;
//! keys belong to the input method while it composes; a commit types the
//! text, and a cancel puts the field back as it was.

use astryx_core::contract::{
    A11yAction, A11yNode, Command, DisplayList, DrawCmd, InputPhase, Key, KeyEvent, Modifiers,
    Role, Surface,
};
use astryx_core::testing::draw;
use astryx_core::{App, Element, Runtime, VStack};
use astryx_spike_ui::{static_source, ChatComposer, Entry, NumberInput, TextInput, Typeahead};

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Text(String),
    Number(f64),
    Pick(Option<Entry>),
    Send(String),
}

#[derive(Clone, Copy)]
enum Field {
    Text,
    Number,
    Typeahead,
    Composer,
}

struct Form {
    field: Field,
    text: String,
    sent: Vec<Msg>,
}

impl App for Form {
    type Msg = Msg;
    fn view(&self) -> Element<Msg> {
        let field: Element<Msg> = match self.field {
            Field::Text => TextInput::new("Name", self.text.clone())
                .on_change(Msg::Text)
                .into(),
            Field::Number => NumberInput::new("qty", "Quantity", None)
                .on_change(Msg::Number)
                .into(),
            Field::Typeahead => Typeahead::new(
                "fruit",
                "Fruit",
                static_source(vec![Entry::new("apple", "Apple")]),
                None,
            )
            .on_change(Msg::Pick)
            .into(),
            Field::Composer => ChatComposer::new("chat", Msg::Send).into(),
        };
        VStack::new().child(field).into()
    }
    fn update(&mut self, msg: Msg) -> Command {
        if let Msg::Text(t) = &msg {
            self.text = t.clone();
        }
        self.sent.push(msg);
        Command::None
    }
}

fn find<'a>(n: &'a A11yNode, role: Role, label: &str) -> Option<&'a A11yNode> {
    if n.role == role && n.label.as_deref() == Some(label) {
        return Some(n);
    }
    n.children.iter().find_map(|c| find(c, role, label))
}

struct Case {
    rt: Runtime<Form>,
    role: Role,
    label: &'static str,
}

impl Case {
    fn new(field: Field, role: Role, label: &'static str, text: &str) -> Case {
        let mut rt = Runtime::new(Form {
            field,
            text: text.into(),
            sent: Vec::new(),
        });
        draw(&mut rt, 640, 400);
        let mut case = Case { rt, role, label };
        let id = case.node().id;
        case.rt.a11y_action(id, A11yAction::Focus);
        case.frame();
        assert!(case.node().focused, "{label}: focused");
        case
    }

    fn frame(&mut self) -> DisplayList {
        draw(&mut self.rt, 640, 400)
    }

    fn node(&self) -> A11yNode {
        find(&self.rt.accessibility().unwrap(), self.role, self.label)
            .cloned()
            .unwrap_or_else(|| panic!("no {}", self.label))
    }

    fn value(&self) -> String {
        self.node().value.unwrap_or_default()
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

/// The thin rectangles under composing text.
fn underlines(list: &DisplayList) -> usize {
    list.commands
        .iter()
        .filter(|c| matches!(c, DrawCmd::Rect { rect, stroke: None, .. } if rect.height == 1.0 && rect.width > 0.0))
        .count()
}

fn enter(rt: &mut Runtime<Form>) -> bool {
    let e = |phase| KeyEvent {
        key: Key::Enter,
        phase,
        modifiers: Modifiers::default(),
        repeat: false,
    };
    let handled = rt.key(e(InputPhase::Pressed)).handled;
    rt.key(e(InputPhase::Released));
    handled
}

/// Compose, cancel, compose again, commit: what the field shows and holds
/// at each step.
fn compose_cancel_commit(mut case: Case, before: &str) {
    let what = case.label;
    let caret_before = case.node().state.caret_bounds.expect("caret bounds");
    let shown_before = texts(&case.frame());
    case.rt.text_compose("にほ", "にほ".len() as u32);
    let list = case.frame();
    assert!(
        texts(&list)
            .iter()
            .any(|t| t.contains(&format!("{before}にほ"))),
        "{what}: the composing text shows inline at the caret: {:?}",
        texts(&list)
    );
    assert_eq!(underlines(&list), 1, "{what}: one underline under it");
    assert_eq!(
        case.value(),
        before,
        "{what}: the value waits for the commit"
    );
    let caret = case.node().state.caret_bounds.expect("caret bounds");
    assert!(
        caret.x > caret_before.x,
        "{what}: the caret moves to the end of the composing text"
    );
    let field = case.node().bounds;
    assert!(
        caret.x >= field.x && caret.x <= field.x + field.width,
        "{what}: the candidate window goes at the caret, inside the field"
    );
    assert!(!enter(&mut case.rt), "{what}: Enter is the input method's");
    let sent = case.rt.app().sent.len();
    // Cancel: the field is as it was.
    case.rt.text_compose("", 0);
    let list = case.frame();
    assert!(
        !texts(&list).iter().any(|t| t.contains("にほ")),
        "{what}: a cancel removes the composing text"
    );
    assert_eq!(underlines(&list), 0, "{what}: and its underline");
    assert_eq!(texts(&list), shown_before, "{what}: drawn as before");
    assert_eq!(case.value(), before);
    // Commit.
    case.rt.text_compose("にほん", 3);
    case.frame();
    case.rt.text_compose("", 0);
    case.rt.text_input("日本");
    case.frame();
    case.frame();
    assert_eq!(
        case.value(),
        format!("{before}日本"),
        "{what}: the commit types the text"
    );
    assert_eq!(
        case.rt.app().sent.len() - sent,
        usize::from(matches!(case.role, Role::TextInput) && what == "Name"),
        "{what}: only the field's own change message"
    );
}

#[test]
fn a_text_input_composes_inline() {
    compose_cancel_commit(Case::new(Field::Text, Role::TextInput, "Name", "ab"), "ab");
}

#[test]
fn a_number_input_composes_inline() {
    compose_cancel_commit(
        Case::new(Field::Number, Role::SpinButton, "Quantity", ""),
        "",
    );
}

#[test]
fn a_typeahead_composes_inline() {
    compose_cancel_commit(Case::new(Field::Typeahead, Role::ComboBox, "Fruit", ""), "");
}

#[test]
fn the_chat_composer_composes_inline_and_wraps() {
    compose_cancel_commit(
        Case::new(Field::Composer, Role::TextInput, "Message input", ""),
        "",
    );
    // A long composition wraps in the multi-line field, underlined on each
    // line, and the composer does not send on the input method's Enter.
    let mut case = Case::new(Field::Composer, Role::TextInput, "Message input", "");
    let long = "あいうえお".repeat(30);
    case.rt.text_compose(&long, long.len() as u32);
    let list = case.frame();
    assert!(
        underlines(&list) >= 2,
        "a wrapped composition is underlined per line"
    );
    let caret = case.node().state.caret_bounds.unwrap();
    let field = case.node().bounds;
    assert!(
        caret.y > field.y + 4.0,
        "the caret follows the composition onto a later line"
    );
    assert!(!enter(&mut case.rt));
    assert!(
        !case.rt.app().sent.iter().any(|m| matches!(m, Msg::Send(_))),
        "Enter while composing never sends"
    );
}
