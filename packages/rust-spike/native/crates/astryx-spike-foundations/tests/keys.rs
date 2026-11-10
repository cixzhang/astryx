//! Keyboard routing on the core runtime: a menu that handles its own
//! arrows and typeahead with roving focus, Tab still leaving it, and a
//! context menu by right click and Shift+F10.

use astryx_core::authoring::{element, Build, BuildCx, KeyResponse, Node};
use astryx_core::testing::draw;
use astryx_core::{
    A11yNode, App, Button, Command, Element, InputPhase, Key, KeyEvent, Modifiers, Point,
    PointerEvent, PointerKind, PointerPhase, Runtime, Surface, VStack,
};
use astryx_spike_foundations::focus::{Key as NavKey, RovingFocus, Typeahead};
use std::rc::Rc;

const ITEMS: [&str; 4] = ["Copy", "Cut", "Paste", "Delete"];

/// A menu whose container takes arrows and letters and moves focus.
struct Menu;

#[derive(Clone, Debug, Default)]
struct MenuState {
    active: usize,
    typeahead: Typeahead,
    clock: f64,
}

impl Build<Msg> for Menu {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        cx.key(Some("menu"));
        let state = cx.local_handle::<MenuState>();
        let focus = cx.focus_requests();
        let mut node = Node::new(taffy::Style {
            flex_direction: taffy::FlexDirection::Column,
            ..Default::default()
        });
        node.on_key = Some(Rc::new(move |e: &KeyEvent| {
            if e.phase != InputPhase::Pressed {
                return KeyResponse::Pass;
            }
            let mut s = state.get();
            let next = match &e.key {
                Key::Up => {
                    RovingFocus::vertical().next(ITEMS.len(), Some(s.active), NavKey::Up, |_| false)
                }
                Key::Down => {
                    RovingFocus::vertical()
                        .next(ITEMS.len(), Some(s.active), NavKey::Down, |_| false)
                }
                Key::Character(c) => {
                    s.clock += 100.0;
                    let clock = s.clock;
                    s.typeahead
                        .input(c, clock, &ITEMS, Some(s.active), |_| false)
                }
                _ => return KeyResponse::Pass,
            };
            if let Some(i) = next {
                s.active = i;
                focus.request(format!("item-{i}"));
            }
            state.set(s);
            KeyResponse::Handled
        }));
        node.on_context = Some(Rc::new(|at: Point| Some(Msg::Context(at.x, at.y))));
        cx.key(Some("menu"));
        cx.begin(node);
        for (i, label) in ITEMS.iter().enumerate() {
            cx.child(Button::new(*label).id(format!("item-{i}")).into());
        }
        cx.end();
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Context(f32, f32),
    KeyMenu,
}

#[derive(Default)]
struct Demo {
    log: Vec<Msg>,
}

impl App for Demo {
    type Msg = Msg;
    fn view(&self) -> Element<Msg> {
        VStack::new()
            .child(element(Menu))
            .child(element(ShiftF10))
            .child(Button::new("After"))
            .into()
    }
    fn update(&mut self, msg: Msg) -> Command {
        self.log.push(msg);
        Command::None
    }
}

/// Opens a menu on Shift+F10 or the context menu key, as ContextMenu does.
struct ShiftF10;

impl Build<Msg> for ShiftF10 {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let mut node = Node::new(Default::default());
        node.on_key = Some(Rc::new(|e: &KeyEvent| {
            match (&e.key, e.modifiers.shift, e.phase) {
                (Key::F(10), true, InputPhase::Pressed)
                | (Key::ContextMenu, _, InputPhase::Pressed) => KeyResponse::Send(Msg::KeyMenu),
                _ => KeyResponse::Pass,
            }
        }));
        cx.begin(node);
        cx.child(Button::new("Row").into());
        cx.end();
    }
}

fn press(rt: &mut Runtime<Demo>, key: Key, shift: bool) -> bool {
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
    draw(rt, 400, 400);
    handled
}

fn focused(rt: &Runtime<Demo>) -> Option<String> {
    fn walk(n: &A11yNode) -> Option<String> {
        if n.focused {
            return n.label.clone();
        }
        n.children.iter().find_map(walk)
    }
    walk(&rt.accessibility()?)
}

#[test]
fn a_menu_takes_its_arrows_and_letters_and_tab_still_leaves() {
    let mut rt = Runtime::new(Demo::default());
    draw(&mut rt, 400, 400);
    assert_eq!(focused(&rt).as_deref(), Some("Copy"));
    assert!(press(&mut rt, Key::Down, false));
    assert_eq!(focused(&rt).as_deref(), Some("Cut"));
    assert!(press(&mut rt, Key::Up, false));
    assert!(press(&mut rt, Key::Up, false), "wraps");
    assert_eq!(focused(&rt).as_deref(), Some("Delete"));
    assert!(press(&mut rt, Key::Character("p".into()), false));
    assert_eq!(focused(&rt).as_deref(), Some("Paste"), "typeahead");
    // Tab is not the menu's: the runtime moves focus out of it.
    assert!(!press(&mut rt, Key::Tab, false));
    rt.input(astryx_core::Input::Next, InputPhase::Pressed);
    draw(&mut rt, 400, 400);
    assert_eq!(
        focused(&rt).as_deref(),
        Some("Delete"),
        "Tab order goes on from Paste"
    );
}

#[test]
fn context_menus_open_by_right_click_and_shift_f10() {
    let mut rt = Runtime::new(Demo::default());
    draw(&mut rt, 400, 400);
    let out = rt.pointer(PointerEvent {
        id: 1,
        kind: PointerKind::Mouse,
        phase: PointerPhase::Down,
        at: Point { x: 10.0, y: 12.0 },
        button: 2,
        time_ms: 0.0,
    });
    assert!(out.handled);
    assert_eq!(rt.app().log, vec![Msg::Context(10.0, 12.0)]);
    // Focus the row, then Shift+F10.
    for _ in 0..4 {
        rt.input(astryx_core::Input::Next, InputPhase::Pressed);
    }
    draw(&mut rt, 400, 400);
    assert_eq!(focused(&rt).as_deref(), Some("Row"));
    assert!(press(&mut rt, Key::F(10), true));
    assert!(!press(&mut rt, Key::F(10), false), "F10 alone passes");
    assert_eq!(rt.app().log.last(), Some(&Msg::KeyMenu));
}
