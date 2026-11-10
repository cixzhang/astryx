//! Test helpers: run one element as an app, read its accessibility tree,
//! and press keys.

use astryx_core::contract::{A11yNode, Command, Input, Role, Surface};
use astryx_core::testing::{draw, tap};
use astryx_core::{App, Element, Runtime};
use std::cell::RefCell;
use std::rc::Rc;

/// An app that shows what `view` builds and records every message.
pub struct Harness<Msg: Clone + 'static> {
    view: Rc<dyn Fn() -> Element<Msg>>,
    pub sent: Rc<RefCell<Vec<Msg>>>,
}

impl<Msg: Clone + 'static> App for Harness<Msg> {
    type Msg = Msg;
    fn view(&self) -> Element<Msg> {
        (self.view)()
    }
    fn update(&mut self, msg: Msg) -> Command {
        self.sent.borrow_mut().push(msg);
        Command::None
    }
}

/// A runtime over `view`, drawn once at `width × height`.
pub struct Rig<Msg: Clone + 'static> {
    pub rt: Runtime<Harness<Msg>>,
    pub sent: Rc<RefCell<Vec<Msg>>>,
    size: (u32, u32),
}

impl<Msg: Clone + 'static> Rig<Msg> {
    pub fn new(view: impl Fn() -> Element<Msg> + 'static) -> Self {
        Self::sized(view, 480, 360)
    }

    pub fn sized(view: impl Fn() -> Element<Msg> + 'static, width: u32, height: u32) -> Self {
        let sent = Rc::new(RefCell::new(Vec::new()));
        let mut rt = Runtime::new(Harness {
            view: Rc::new(view),
            sent: sent.clone(),
        });
        draw(&mut rt, width, height);
        Rig {
            rt,
            sent,
            size: (width, height),
        }
    }

    pub fn redraw(&mut self) {
        draw(&mut self.rt, self.size.0, self.size.1);
    }

    pub fn tree(&self) -> A11yNode {
        self.rt.accessibility().expect("an accessibility tree")
    }

    pub fn press(&mut self, input: Input) -> Command {
        let c = tap(&mut self.rt, input);
        self.redraw();
        c
    }

    /// A mouse drag from `from` to `to` in `steps` moves of 16 ms.
    pub fn drag(&mut self, from: astryx_core::Point, to: astryx_core::Point, steps: u32) {
        use astryx_core::contract::{PointerEvent, PointerKind, PointerPhase, Surface};
        let e = |phase, at, t: f64| PointerEvent {
            id: 1,
            kind: PointerKind::Mouse,
            phase,
            at,
            button: 0,
            time_ms: t,
        };
        self.rt.pointer(e(PointerPhase::Down, from, 0.0));
        let mut t = 0.0;
        for i in 1..=steps {
            t += 16.0;
            let k = i as f32 / steps as f32;
            let at = astryx_core::Point {
                x: from.x + (to.x - from.x) * k,
                y: from.y + (to.y - from.y) * k,
            };
            self.rt.pointer(e(PointerPhase::Move, at, t));
            self.redraw();
        }
        self.rt.pointer(e(PointerPhase::Up, to, t + 4.0));
        self.redraw();
    }
    /// Presses and releases `key` with `modifiers`.
    pub fn key(&mut self, key: astryx_core::Key, modifiers: astryx_core::Modifiers) {
        use astryx_core::contract::Surface;
        for phase in [
            astryx_core::InputPhase::Pressed,
            astryx_core::InputPhase::Released,
        ] {
            self.rt.key(astryx_core::KeyEvent {
                key: key.clone(),
                phase,
                modifiers,
                repeat: false,
            });
        }
        self.redraw();
    }
    /// A secondary (right) click at `at`.
    pub fn right_click(&mut self, at: astryx_core::Point) {
        use astryx_core::contract::{PointerEvent, PointerKind, PointerPhase, Surface};
        for phase in [PointerPhase::Down, PointerPhase::Up] {
            self.rt.pointer(PointerEvent {
                id: 2,
                kind: PointerKind::Mouse,
                phase,
                at,
                button: 2,
                time_ms: 0.0,
            });
        }
        self.redraw();
    }
    pub fn messages(&self) -> Vec<Msg> {
        self.sent.borrow().clone()
    }
}

/// Every node of the tree, depth first.
pub fn flatten(node: &A11yNode) -> Vec<&A11yNode> {
    let mut out = vec![node];
    for c in &node.children {
        out.extend(flatten(c));
    }
    out
}

/// The first node with `role` and a label equal to `label`.
pub fn find<'a>(tree: &'a A11yNode, role: Role, label: &str) -> Option<&'a A11yNode> {
    flatten(tree)
        .into_iter()
        .find(|n| n.role == role && n.label.as_deref() == Some(label))
}

/// Every node with `role`.
pub fn all(tree: &A11yNode, role: Role) -> Vec<&A11yNode> {
    flatten(tree)
        .into_iter()
        .filter(|n| n.role == role)
        .collect()
}
