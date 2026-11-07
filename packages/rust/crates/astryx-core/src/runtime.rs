//! The Elm-style app model and the runtime that drives it as a [`Surface`].

use crate::contract::{
    A11yAction, A11yNode, Command, DisplayList, FrameInfo, Input, InputPhase, Preferences, Rect,
    Surface, TextMeasure,
};
use crate::element::{Axis, BuildCx, Element, Node, PopupAction};
use crate::layout::Layout;
use crate::paint::{self, PaintState, FOCUS_RING_OFFSET, FOCUS_RING_WIDTH};
use crate::theme::ThemeChoice;
use crate::{a11y, focus};
use std::collections::HashMap;
use taffy::{FlexDirection, Style};

/// An Astryx app: state, a view of it, and an update for each message.
///
/// ```
/// use astryx_core::{App, Button, Command, Element, Text, VStack};
///
/// struct Counter(u32);
///
/// #[derive(Clone)]
/// enum Msg {
///     Add,
/// }
///
/// impl App for Counter {
///     type Msg = Msg;
///     fn view(&self) -> Element<Msg> {
///         VStack::new()
///             .child(Text::new(format!("Count: {}", self.0)))
///             .child(Button::new("Add").on_click(Msg::Add))
///             .into()
///     }
///     fn update(&mut self, msg: Msg) -> Command {
///         match msg {
///             Msg::Add => self.0 += 1,
///         }
///         Command::None
///     }
/// }
/// ```
pub trait App {
    type Msg: Clone + 'static;

    fn view(&self) -> Element<Self::Msg>;

    fn update(&mut self, msg: Self::Msg) -> Command;

    /// The theme and color mode. Read after every update, so switching it is
    /// an ordinary message.
    fn theme(&self) -> ThemeChoice {
        ThemeChoice::default()
    }

    /// Inputs focus does not use: B (back), X, Y, the shoulder buttons,
    /// Start, Select, and Menu, on press. Return a message to handle one.
    fn on_input(&self, input: Input) -> Option<Self::Msg> {
        let _ = input;
        None
    }
}

/// Runs an [`App`]: builds its view, lays it out, moves focus, and paints.
pub struct Runtime<A: App> {
    app: A,
    theme: ThemeChoice,
    nodes: Vec<Node<A::Msg>>,
    layout: Option<Layout>,
    stale: bool,
    relayout: bool,
    viewport: (f32, f32),
    /// Focus stops in reading order: node index and id.
    stops: Vec<(usize, u64)>,
    focused: Option<u64>,
    /// The focused box, kept to place focus when its element goes away.
    focused_box: Option<Rect>,
    pressed: Option<u64>,
    prefs: Preferences,
    /// The control whose popup is open.
    popup: Option<u64>,
    scroll: HashMap<u64, f32>,
    reveal: bool,
    /// Focus or press changed since the last frame.
    repaint: bool,
    animating: bool,
    time: f32,
    a11y: Option<A11yNode>,
}

impl<A: App> Runtime<A> {
    pub fn new(app: A) -> Self {
        let theme = app.theme();
        Runtime {
            app,
            theme,
            nodes: Vec::new(),
            layout: None,
            stale: true,
            relayout: true,
            viewport: (0.0, 0.0),
            stops: Vec::new(),
            focused: None,
            focused_box: None,
            pressed: None,
            prefs: Preferences::default(),
            popup: None,
            scroll: HashMap::new(),
            reveal: false,
            repaint: true,
            animating: false,
            time: 0.0,
            a11y: None,
        }
    }

    pub fn app(&self) -> &A {
        &self.app
    }

    /// Changes app state from outside a message; the next frame rebuilds.
    pub fn app_mut(&mut self) -> &mut A {
        self.stale = true;
        &mut self.app
    }

    /// Sends a message as a control would.
    pub fn send(&mut self, msg: A::Msg) -> Command {
        let command = self.app.update(msg);
        self.stale = true;
        command
    }

    /// The accessibility id of the focused element.
    pub fn focused(&self) -> Option<u64> {
        self.focused
    }

    fn rebuild(&mut self) {
        self.theme = self.app.theme();
        let tokens = self.theme.tokens(&self.prefs);
        let mut cx = BuildCx::new(tokens);
        cx.prefs = self.prefs;
        cx.viewport = self.viewport;
        cx.popup = self.popup.and_then(|id| {
            let ix = self.nodes.iter().position(|n| n.id == id)?;
            Some((id, self.layout.as_ref()?.rects.get(ix).copied()?))
        });
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            ..Style::default()
        }));
        cx.child(self.app.view());
        // Popups go last, so they paint above the view and are not clipped by it.
        for overlay in std::mem::take(&mut cx.overlays) {
            overlay(&mut cx);
        }
        cx.end();
        self.animating = cx.animating;
        self.nodes = cx.nodes;
        if let Some(id) = self.popup {
            if !self.nodes.iter().any(|n| n.trap) || !self.nodes.iter().any(|n| n.id == id) {
                self.popup = None;
            }
        }
        self.layout = Some(Layout::new(&self.nodes));
        self.stale = false;
        self.relayout = true;
    }

    fn stop_index(&self, id: Option<u64>) -> Option<usize> {
        id.and_then(|id| self.stops.iter().position(|&(_, s)| s == id))
    }

    fn stop_rects(&self) -> Vec<Rect> {
        let Some(layout) = &self.layout else {
            return Vec::new();
        };
        self.stops.iter().map(|&(ix, _)| layout.rects[ix]).collect()
    }

    fn set_focus(&mut self, stop: usize) {
        let (ix, id) = self.stops[stop];
        if self.focused != Some(id) {
            self.pressed = None;
        }
        self.focused = Some(id);
        self.focused_box = self.layout.as_ref().map(|l| l.boxes[ix]);
        self.reveal = true;
    }

    /// Keeps focus on a live stop after a rebuild: the same id when it
    /// survives, else the stop nearest to where focus was, else the first.
    fn settle_focus(&mut self) {
        let trapped = |nodes: &[Node<A::Msg>], mut ix: usize| loop {
            if nodes[ix].trap {
                return true;
            }
            match nodes[ix].parent {
                Some(p) => ix = p as usize,
                None => return false,
            }
        };
        let has_trap = self.nodes.iter().any(|n| n.trap);
        self.stops = self
            .nodes
            .iter()
            .enumerate()
            .filter(|&(i, n)| n.interact.is_some() && (!has_trap || trapped(&self.nodes, i)))
            .map(|(i, n)| (i, n.id))
            .collect();
        if self.stops.is_empty() {
            self.focused = None;
            return;
        }
        if let Some(stop) = self.stop_index(self.focused) {
            self.focused_box = self.layout.as_ref().map(|l| l.boxes[self.stops[stop].0]);
            return;
        }
        if let Some(stop) = self
            .stops
            .iter()
            .position(|&(ix, _)| self.nodes[ix].autofocus)
        {
            self.set_focus(stop);
            return;
        }
        let nearest = match (self.focused_box, &self.layout) {
            (Some(old), Some(layout)) => {
                let c = old.center();
                let dist = |&(ix, _): &(usize, u64)| {
                    let d = layout.boxes[ix].center();
                    (d.x - c.x).powi(2) + (d.y - c.y).powi(2)
                };
                (0..self.stops.len())
                    .min_by(|&a, &b| dist(&self.stops[a]).total_cmp(&dist(&self.stops[b])))
                    .unwrap_or(0)
            }
            _ => 0,
        };
        self.set_focus(nearest);
    }

    /// Scrolls every scroll container around the focused element so the
    /// element and its ring are in view.
    fn reveal_focus(&mut self) {
        let Some(stop) = self.stop_index(self.focused) else {
            return;
        };
        let target = self.stops[stop].0;
        let margin = FOCUS_RING_OFFSET + FOCUS_RING_WIDTH;
        let mut ancestor = self.nodes[target].parent;
        while let Some(a) = ancestor {
            let a = a as usize;
            if let Some(axis) = self.nodes[a].scroll {
                let layout = self.layout.as_mut().expect("layout");
                layout.place(&self.nodes, &self.scroll);
                let (view, item) = (layout.rects[a], layout.rects[target]);
                let (v0, v1, i0, i1) = match axis {
                    Axis::Vertical => (
                        view.y,
                        view.y + view.height,
                        item.y - margin,
                        item.y + item.height + margin,
                    ),
                    Axis::Horizontal => (
                        view.x,
                        view.x + view.width,
                        item.x - margin,
                        item.x + item.width + margin,
                    ),
                };
                let current = self.scroll.get(&self.nodes[a].id).copied().unwrap_or(0.0);
                let delta = if i0 < v0 {
                    i0 - v0
                } else if i1 > v1 {
                    (i1 - v1).min(i0 - v0)
                } else {
                    0.0
                };
                let next = (current + delta).clamp(0.0, layout.overflow[a]);
                self.scroll.insert(self.nodes[a].id, next);
            }
            ancestor = self.nodes[a].parent;
        }
        if let Some(layout) = self.layout.as_mut() {
            layout.place(&self.nodes, &self.scroll);
        }
    }

    fn activate(&mut self, id: u64) -> Command {
        let Some(node) = self.nodes.iter().find(|n| n.id == id) else {
            return Command::None;
        };
        let Some(interact) = &node.interact else {
            return Command::None;
        };
        if interact.disabled || interact.busy {
            return Command::None;
        }
        let msg = interact.on_activate.clone();
        match interact.popup {
            PopupAction::None => {}
            PopupAction::Toggle => {
                self.popup = if self.popup == Some(id) {
                    None
                } else {
                    Some(id)
                };
                self.stale = true;
            }
            PopupAction::Close => self.close_popup(),
        }
        match msg {
            Some(msg) => self.send(msg),
            None => Command::None,
        }
    }

    /// Closes the open popup and puts focus back on its owner.
    fn close_popup(&mut self) {
        if let Some(owner) = self.popup.take() {
            self.focused = Some(owner);
            self.reveal = true;
            self.stale = true;
        }
    }
}

impl<A: App> Surface for Runtime<A> {
    fn input(&mut self, input: Input, phase: InputPhase) -> Command {
        self.repaint = true;
        match (input, phase) {
            (Input::A, InputPhase::Pressed) => {
                // A repeat while held keeps the first press.
                if self.pressed.is_none() {
                    self.pressed = self.focused;
                }
                Command::None
            }
            (Input::A, InputPhase::Released) => match self.pressed.take() {
                Some(id) if Some(id) == self.focused => self.activate(id),
                _ => Command::None,
            },
            (
                Input::Up
                | Input::Down
                | Input::Left
                | Input::Right
                | Input::Next
                | Input::Previous,
                InputPhase::Pressed,
            ) => {
                let rects = self.stop_rects();
                if let Some(stop) = focus::navigate(&rects, self.stop_index(self.focused), input) {
                    self.set_focus(stop);
                    self.reveal_focus();
                }
                Command::None
            }
            (Input::B, InputPhase::Pressed) if self.popup.is_some() => {
                self.close_popup();
                Command::None
            }
            (_, InputPhase::Pressed) => match self.app.on_input(input) {
                Some(msg) => self.send(msg),
                None => Command::None,
            },
            _ => Command::None,
        }
    }

    fn frame(&mut self, frame: &FrameInfo, text: &dyn TextMeasure) -> DisplayList {
        // Under reduced motion spinners hold still.
        if !self.prefs.reduced_motion {
            self.time += frame.dt;
        }
        self.repaint = false;
        let size = frame.logical_size();
        let viewport = (size.width, size.height);
        if self.stale || self.app.theme() != self.theme {
            self.rebuild();
        }
        if self.relayout || viewport != self.viewport {
            self.viewport = viewport;
            self.layout
                .as_mut()
                .expect("layout")
                .compute(&self.nodes, viewport, text);
            self.relayout = false;
            self.layout
                .as_mut()
                .expect("layout")
                .place(&self.nodes, &self.scroll);
            self.settle_focus();
        }
        if std::mem::take(&mut self.reveal) {
            self.reveal_focus();
        }
        let layout = self.layout.as_ref().expect("layout");
        self.a11y = a11y::tree(&self.nodes, &layout.rects, self.focused);
        let state = PaintState {
            focused: self.focused,
            pressed: self.pressed,
            time: self.time,
        };
        paint::paint(
            &self.nodes,
            layout,
            &state,
            self.theme.tokens(&self.prefs).color.background_body.into(),
        )
    }

    fn a11y_action(&mut self, id: u64, action: A11yAction) -> Command {
        self.repaint = true;
        let Some(stop) = self.stop_index(Some(id)) else {
            return Command::None;
        };
        self.set_focus(stop);
        self.reveal_focus();
        match action {
            A11yAction::Focus => Command::None,
            A11yAction::Activate => self.activate(id),
        }
    }

    fn set_preferences(&mut self, preferences: &Preferences) {
        if *preferences != self.prefs {
            self.prefs = *preferences;
            self.stale = true;
        }
    }

    fn accessibility(&self) -> Option<A11yNode> {
        self.a11y.clone()
    }

    fn wants_frames(&self) -> bool {
        self.animating || self.stale || self.repaint
    }
}
