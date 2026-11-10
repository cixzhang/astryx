//! A multi-line text field: Shift+Enter types a line break at the caret,
//! Enter does too when the field sends nothing on activate, Up and Down move
//! the caret between lines, and at the first or last line they move focus.

use astryx_core::authoring::{element, text_node, Build, BuildCx, Edit, Interact, Node, Semantics};
use astryx_core::testing::draw;
use astryx_core::{
    App, Button, Color, Command, Element, FontFamily, InputPhase, Key, KeyEvent, Modifiers, Role,
    Surface, TextStyle, VStack,
};
use std::rc::Rc;

struct Field {
    value: String,
    send_on_enter: bool,
}

impl Build<Msg> for Field {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let mut interact = Interact::new(self.send_on_enter.then_some(Msg::Send), false);
        let style = TextStyle {
            size: 10.0,
            ..astryx_core::authoring::type_style(cx.tokens.font.body, FontFamily::Body)
        };
        cx.key(Some("composer"));
        let field = cx.begin(
            Node::new(taffy::Style {
                size: taffy::Size {
                    width: taffy::Dimension::length(100.0),
                    height: taffy::Dimension::auto(),
                },
                ..Default::default()
            })
            .sem(Semantics::new(Role::TextInput, "Message")),
        );
        let mut text = text_node(self.value.clone(), style, Color::rgba(0.0, 0.0, 0.0, 1.0));
        text.style.size.width = taffy::Dimension::percent(1.0);
        let text_ix = cx.leaf(text);
        cx.end();
        interact.edit = Some(Edit {
            value: self.value,
            on_change: Some(Rc::new(Msg::Text)),
            effect: None,
            text_node: text_ix,
            masked: false,
            multiline: true,
            caret: Color::rgba(0.0, 0.0, 0.0, 1.0),
        });
        cx.nodes[field as usize].interact = Some(interact);
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Text(String),
    Send,
}

struct Composer {
    value: String,
    send_on_enter: bool,
    sent: u32,
}

impl App for Composer {
    type Msg = Msg;
    fn view(&self) -> Element<Msg> {
        VStack::new()
            .child(element(Field {
                value: self.value.clone(),
                send_on_enter: self.send_on_enter,
            }))
            .child(Button::new("After"))
            .into()
    }
    fn update(&mut self, msg: Msg) -> Command {
        match msg {
            Msg::Text(v) => self.value = v,
            Msg::Send => self.sent += 1,
        }
        Command::None
    }
}

fn key(rt: &mut astryx_core::Runtime<Composer>, key: Key, shift: bool) -> bool {
    let e = |phase| KeyEvent {
        key: key.clone(),
        phase,
        modifiers: Modifiers {
            shift,
            ..Modifiers::default()
        },
        repeat: false,
    };
    let handled = rt.key(e(InputPhase::Pressed)).handled;
    rt.key(e(InputPhase::Released));
    draw(rt, 300, 300);
    handled
}

fn typed(rt: &mut astryx_core::Runtime<Composer>, text: &str) {
    rt.text_input(text);
    draw(rt, 300, 300);
}

fn runtime(send_on_enter: bool) -> astryx_core::Runtime<Composer> {
    let mut rt = astryx_core::Runtime::new(Composer {
        value: String::new(),
        send_on_enter,
        sent: 0,
    });
    draw(&mut rt, 300, 300);
    rt
}

#[test]
fn shift_enter_breaks_the_line_at_the_caret() {
    let mut rt = runtime(true);
    typed(&mut rt, "hello");
    assert!(key(&mut rt, Key::Enter, true));
    typed(&mut rt, "world");
    assert_eq!(
        rt.app().value,
        "hello\nworld",
        "text after the break lands after it"
    );
    assert_eq!(rt.app().sent, 0);
    // Enter alone sends, as the composer's activate does.
    assert!(!key(&mut rt, Key::Enter, false));
    fn find(n: &astryx_core::A11yNode) -> Option<&astryx_core::A11yNode> {
        if n.label.as_deref() == Some("Message") {
            return Some(n);
        }
        n.children.iter().find_map(find)
    }
    let tree = rt.accessibility().unwrap();
    assert!(find(&tree).unwrap().state.multiline);
}

#[test]
fn enter_breaks_the_line_when_the_field_sends_nothing() {
    let mut rt = runtime(false);
    typed(&mut rt, "a");
    assert!(key(&mut rt, Key::Enter, false));
    typed(&mut rt, "b");
    assert_eq!(rt.app().value, "a\nb");
}

#[test]
fn up_and_down_move_between_lines_then_move_focus() {
    let mut rt = runtime(false);
    typed(&mut rt, "first");
    key(&mut rt, Key::Enter, false);
    typed(&mut rt, "second");
    // Up from the end of "second" lands at the end of "first" (nearest x).
    assert!(key(&mut rt, Key::Up, false));
    draw(&mut rt, 300, 300);
    typed(&mut rt, "!");
    assert_eq!(rt.app().value, "first!\nsecond");
    // On the first line, Up is not the field's: focus moves as usual.
    assert!(!key(&mut rt, Key::Up, false));
    // Down returns to the second line.
    assert!(key(&mut rt, Key::Down, false));
    draw(&mut rt, 300, 300);
    typed(&mut rt, "?");
    assert_eq!(rt.app().value, "first!\nsecond?");
    assert!(
        !key(&mut rt, Key::Down, false),
        "last line: Down moves focus"
    );
}

#[test]
fn long_text_wraps_and_the_caret_follows() {
    let mut rt = runtime(false);
    // 5 points a character at size 10: 100 points hold 20 characters.
    typed(&mut rt, "aaaa bbbb cccc dddd eeee ffff");
    assert!(
        key(&mut rt, Key::Up, false),
        "the wrapped text has a line above the caret"
    );
    draw(&mut rt, 300, 300);
    typed(&mut rt, "X");
    let v = rt.app().value.clone();
    let at = v.find('X').unwrap();
    assert!(at < 20, "the caret moved to the first visual line: {v:?}");
}
