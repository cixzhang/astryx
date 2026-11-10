//! The ledger's Layers exercise, end to end through the core runtime:
//! a popover over a dialog.

use super::*;
use astryx_core::authoring::{Build, BuildCx, Node, Semantics};
use astryx_core::testing::{draw, tap};
use astryx_core::{
    A11yNode, App, Button, Command, Input, PointerEvent, PointerKind, PointerPhase, Surface, VStack,
};
use astryx_core::{Runtime, Text};

#[derive(Default)]
struct Demo {
    dialog: bool,
    popover: bool,
    log: Vec<String>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
enum Msg {
    OpenDialog,
    OpenPopover,
    CloseDialog(DismissReason),
    ClosePopover(DismissReason),
    Noop,
}

impl App for Demo {
    type Msg = Msg;
    fn view(&self) -> Element<Msg> {
        let mut page = VStack::new()
            .child(
                Button::new("Open dialog")
                    .id("open-dialog")
                    .on_click(Msg::OpenDialog),
            )
            .child(Button::new("Behind").on_click(Msg::Noop));
        if self.dialog {
            let mut body = VStack::new()
                .child(Text::new("Settings"))
                .child(
                    Button::new("Open popover")
                        .id("open-popover")
                        .on_click(Msg::OpenPopover),
                )
                .child(Button::new("Save").on_click(Msg::Noop));
            if self.popover {
                body = body.child(
                    Layer::popover(
                        "open-popover",
                        VStack::new()
                            .child(Button::new("Inside popover").on_click(Msg::Noop))
                            .into(),
                    )
                    .label("Options")
                    .placement(Placement::new(Side::Below, Align::Start).offset(4.0))
                    .on_dismiss(Msg::ClosePopover),
                );
            }
            page = page.child(
                Layer::dialog(body.into())
                    .label("Settings")
                    .backdrop(Color::rgba(0.0, 0.0, 0.0, 0.4))
                    .on_dismiss(Msg::CloseDialog),
            );
        }
        page.into()
    }
    fn update(&mut self, msg: Msg) -> Command {
        self.log.push(format!("{msg:?}"));
        match msg {
            Msg::OpenDialog => self.dialog = true,
            Msg::OpenPopover => self.popover = true,
            Msg::CloseDialog(_) => {
                self.dialog = false;
                self.popover = false;
            }
            Msg::ClosePopover(_) => self.popover = false,
            Msg::Noop => {}
        }
        Command::None
    }
}

fn find<'a>(n: &'a A11yNode, label: &str) -> Option<&'a A11yNode> {
    if n.label.as_deref() == Some(label) {
        return Some(n);
    }
    n.children.iter().find_map(|c| find(c, label))
}

fn focused_label(rt: &Runtime<Demo>) -> Option<String> {
    fn walk(n: &A11yNode) -> Option<String> {
        if n.focused {
            return n.label.clone();
        }
        n.children.iter().find_map(walk)
    }
    walk(&rt.accessibility()?)
}

fn focus(rt: &mut Runtime<Demo>, label: &str) {
    let id = find(&rt.accessibility().unwrap(), label).unwrap().id;
    rt.a11y_action(id, astryx_core::A11yAction::Focus);
    draw(rt, 800, 600);
}

fn press(rt: &mut Runtime<Demo>, x: f32, y: f32) -> bool {
    let e = |phase| PointerEvent {
        id: 1,
        kind: PointerKind::Mouse,
        phase,
        at: astryx_core::Point { x, y },
        button: 0,
    };
    let down = rt.pointer(e(PointerPhase::Down));
    rt.pointer(e(PointerPhase::Up));
    draw(rt, 800, 600);
    down.handled
}

fn open_both() -> Runtime<Demo> {
    let mut rt = Runtime::new(Demo::default());
    draw(&mut rt, 800, 600);
    focus(&mut rt, "Open dialog");
    tap(&mut rt, Input::A);
    draw(&mut rt, 800, 600);
    focus(&mut rt, "Open popover");
    tap(&mut rt, Input::A);
    draw(&mut rt, 800, 600);
    assert!(rt.app().dialog && rt.app().popover);
    rt
}

#[test]
fn escape_closes_popover_then_dialog_and_focus_returns_to_each_opener() {
    let mut rt = open_both();
    tap(&mut rt, Input::B);
    draw(&mut rt, 800, 600);
    assert!(rt.app().dialog && !rt.app().popover, "{:?}", rt.app().log);
    assert_eq!(focused_label(&rt).as_deref(), Some("Open popover"));
    tap(&mut rt, Input::B);
    draw(&mut rt, 800, 600);
    assert!(!rt.app().dialog);
    assert_eq!(focused_label(&rt).as_deref(), Some("Open dialog"));
    // One message per close: each layer closed exactly once.
    let closes = rt
        .app()
        .log
        .iter()
        .filter(|l| l.starts_with("Close"))
        .count();
    assert_eq!(closes, 2, "{:?}", rt.app().log);
}

#[test]
fn outside_press_closes_only_the_popover() {
    let mut rt = open_both();
    let tree = rt.accessibility().unwrap();
    let dialog = find(&tree, "Settings").unwrap().bounds;
    let popover = find(&tree, "Options").unwrap().bounds;
    // Inside the dialog, outside the popover.
    let (x, y) = (dialog.x + 2.0, dialog.y + 2.0);
    assert!(
        !(x >= popover.x
            && x < popover.x + popover.width
            && y >= popover.y
            && y < popover.y + popover.height)
    );
    assert!(press(&mut rt, x, y), "the press is taken by the dismissal");
    assert!(rt.app().dialog && !rt.app().popover);
    // The dialog does not close on outside presses; the page stays inert.
    assert!(press(&mut rt, 2.0, 590.0));
    assert!(rt.app().dialog);
    assert!(!rt.app().log.iter().any(|l| l.starts_with("CloseDialog")));
}

#[test]
fn page_behind_a_modal_dialog_is_inert_and_focus_is_trapped() {
    let mut rt = open_both();
    let tree = rt.accessibility().unwrap();
    assert!(
        find(&tree, "Behind").is_none(),
        "hidden from assistive technology"
    );
    assert!(find(&tree, "Inside popover").is_some());
    let dialog = find(&tree, "Settings").unwrap();
    assert_eq!(dialog.role, Role::Dialog);
    assert!(dialog.state.modal);
    // Close the popover: Tab cycles inside the dialog only.
    tap(&mut rt, Input::B);
    draw(&mut rt, 800, 600);
    let mut seen = Vec::new();
    for _ in 0..4 {
        tap(&mut rt, Input::Next);
        draw(&mut rt, 800, 600);
        seen.push(focused_label(&rt).unwrap());
    }
    assert!(
        seen.iter().all(|l| l == "Open popover" || l == "Save"),
        "{seen:?}"
    );
}

#[test]
fn popover_sits_below_its_anchor_with_its_offset() {
    let rt = open_both();
    let tree = rt.accessibility().unwrap();
    let anchor = find(&tree, "Open popover").unwrap().bounds;
    let popover = find(&tree, "Options").unwrap().bounds;
    assert_eq!(popover.x, anchor.x);
    assert_eq!(popover.y, anchor.y + anchor.height + 4.0);
    let dialog = find(&tree, "Settings").unwrap().bounds;
    assert!(
        (dialog.x + dialog.width / 2.0 - 400.0).abs() < 1.0,
        "dialog centered: {dialog:?}"
    );
}

#[test]
fn backdrop_paints_over_the_page() {
    let mut rt = open_both();
    let list = draw(&mut rt, 800, 600);
    let tint = list.commands.iter().any(|c| {
        matches!(c, astryx_core::DrawCmd::Rect { rect, fill, .. }
            if rect.width == 800.0 && rect.height == 600.0 && (fill.a - 0.4).abs() < 1e-3)
    });
    assert!(tint);
}

// Hover intent.

#[derive(Default)]
struct Tip {
    shown: bool,
}

#[derive(Clone)]
enum TipMsg {
    Hover(bool),
}

impl App for Tip {
    type Msg = TipMsg;
    fn view(&self) -> Element<TipMsg> {
        let mut page = VStack::new()
            .child(Hover::new(Button::new("Info").id("info"), TipMsg::Hover).id("info-hover"));
        if self.shown {
            page = page.child(Layer::tooltip(
                "info-hover",
                Text::new("More about this").into(),
            ));
        }
        page.into()
    }
    fn update(&mut self, msg: TipMsg) -> Command {
        let TipMsg::Hover(on) = msg;
        self.shown = on;
        Command::None
    }
}

fn frame_ms(rt: &mut Runtime<Tip>, ms: f32) {
    let mut f = astryx_core::testing::frame(800, 600);
    f.dt = ms / 1000.0;
    rt.frame(&f, &astryx_core::testing::FixedMeasure::default());
}

#[test]
fn tooltip_shows_after_the_hover_delay_and_hides_on_leave() {
    // Before any key, focus is not visible, so only hover counts.
    let mut rt = Runtime::new(Tip::default()).hide_focus_until_keyboard();
    frame_ms(&mut rt, 0.0);
    let button = find(&rt.accessibility().unwrap(), "Info").unwrap().bounds;
    rt.pointer_move(Some(button.center()));
    frame_ms(&mut rt, 16.0);
    frame_ms(&mut rt, 100.0);
    assert!(!rt.app().shown, "not before 200 ms");
    frame_ms(&mut rt, 120.0);
    assert!(rt.app().shown);
    assert!(rt.wants_frames() || rt.app().shown);
    rt.pointer_move(Some(astryx_core::Point { x: 700.0, y: 500.0 }));
    frame_ms(&mut rt, 16.0);
    frame_ms(&mut rt, 16.0);
    assert!(!rt.app().shown, "hides at once");
    let tree = rt.accessibility().unwrap();
    assert!(find(&tree, "More about this").is_none());
}

#[test]
fn keyboard_focus_shows_the_tooltip_at_once() {
    let mut rt = Runtime::new(Tip::default());
    frame_ms(&mut rt, 0.0);
    tap(&mut rt, Input::Next);
    frame_ms(&mut rt, 16.0);
    frame_ms(&mut rt, 16.0);
    assert!(rt.app().shown);
    let tree = rt.accessibility().unwrap();
    let tip = find(&tree, "More about this").unwrap();
    let info = find(&tree, "Info").unwrap();
    assert!(
        tip.bounds.y + tip.bounds.height <= info.bounds.y + 0.5 || tip.bounds.y >= info.bounds.y,
        "placed beside its anchor"
    );
}

// Uncontrolled: a disclosure keeps its own open state, with no app message.

struct Disclosure;

impl Build<()> for Disclosure {
    fn build(self: Box<Self>, cx: &mut BuildCx<()>) {
        cx.key(Some("disclosure"));
        let open = cx.local_handle::<bool>();
        let toggle = open.clone();
        let mut trigger = Node::new(taffy::Style {
            size: taffy::Size {
                width: taffy::Dimension::length(80.0),
                height: taffy::Dimension::length(30.0),
            },
            ..Default::default()
        })
        .sem(Semantics::new(Role::Button, "Toggle"))
        .interact(
            astryx_core::authoring::Interact::new(None, false)
                .effect(move || toggle.update(|o| *o = !*o)),
        );
        trigger.name = None;
        cx.key(Some("disclosure"));
        cx.begin(trigger);
        cx.end();
        if open.get() {
            let close = open.clone();
            cx.child(
                Layer::popover("disclosure", Text::new("Panel").into())
                    .label("Panel")
                    .on_dismiss_effect(move |_| close.set(false))
                    .into(),
            );
        }
    }
}

struct Uncontrolled;

impl App for Uncontrolled {
    type Msg = ();
    fn view(&self) -> Element<()> {
        VStack::new()
            .child(astryx_core::authoring::element(Disclosure))
            .into()
    }
    fn update(&mut self, _: ()) -> Command {
        Command::None
    }
}

#[test]
fn an_uncontrolled_popover_opens_and_closes_on_local_state() {
    let mut rt = Runtime::new(Uncontrolled);
    draw(&mut rt, 800, 600);
    let tree = rt.accessibility().unwrap();
    let toggle = find(&tree, "Toggle").unwrap().id;
    rt.a11y_action(toggle, astryx_core::A11yAction::Activate);
    draw(&mut rt, 800, 600);
    assert!(
        find(&rt.accessibility().unwrap(), "Panel").is_some(),
        "opened"
    );
    tap(&mut rt, Input::B);
    draw(&mut rt, 800, 600);
    assert!(
        find(&rt.accessibility().unwrap(), "Panel").is_none(),
        "closed itself"
    );
    rt.a11y_action(toggle, astryx_core::A11yAction::Activate);
    draw(&mut rt, 800, 600);
    assert!(
        find(&rt.accessibility().unwrap(), "Panel").is_some(),
        "state survives rebuilds"
    );
}
