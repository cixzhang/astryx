//! Shared helpers for the integration tests.
#![allow(dead_code)]

use astryx_core::testing::{draw, FixedMeasure};
use astryx_core::*;

/// An app whose view is a closure, recording every message it gets.
pub struct View<M: Clone + 'static> {
    pub view: Box<dyn Fn() -> Element<M>>,
    pub sent: Vec<M>,
    pub choice: ThemeChoice,
}

impl<M: Clone + 'static> App for View<M> {
    type Msg = M;
    fn view(&self) -> Element<M> {
        (self.view)()
    }
    fn update(&mut self, msg: M) -> Command {
        self.sent.push(msg);
        Command::None
    }
    fn theme(&self) -> ThemeChoice {
        self.choice
    }
}

pub fn runtime<M: Clone + 'static>(view: impl Fn() -> Element<M> + 'static) -> Runtime<View<M>> {
    Runtime::new(View {
        view: Box::new(view),
        sent: Vec::new(),
        choice: ThemeChoice::default(),
    })
}

/// Draws one 400 × 600 frame of a single element.
pub fn paint<M: Clone + 'static>(view: impl Fn() -> Element<M> + 'static) -> DisplayList {
    let mut rt = runtime(view);
    draw(&mut rt, 400, 600)
}

pub fn light() -> &'static Tokens {
    &Theme::ASTRYX.light
}

pub fn c(rgba: tokens::Rgba) -> Color {
    rgba.into()
}

/// A painted rect: its box, corners, fill, and stroke.
pub type Painted = (Rect, Radii, Color, Option<(Color, f32)>);

pub fn rects(list: &DisplayList) -> Vec<Painted> {
    list.commands
        .iter()
        .filter_map(|cmd| match cmd {
            DrawCmd::Rect {
                rect,
                radii,
                fill,
                stroke,
            } => Some((*rect, *radii, *fill, *stroke)),
            _ => None,
        })
        .collect()
}

pub type Filled = (Rect, Radii, Option<(Color, f32)>);

/// The first filled rect with this fill color.
pub fn rect_filled(list: &DisplayList, fill: Color) -> Option<Filled> {
    rects(list)
        .into_iter()
        .find(|r| r.2 == fill)
        .map(|r| (r.0, r.1, r.3))
}

/// Rects stroked in `color` with a transparent fill, such as focus rings.
pub fn rings(list: &DisplayList, color: Color) -> Vec<(Rect, Radii, f32)> {
    rects(list)
        .into_iter()
        .filter_map(|(rect, radii, fill, stroke)| match stroke {
            Some((sc, w)) if sc == color && fill.a == 0.0 => Some((rect, radii, w)),
            _ => None,
        })
        .collect()
}

pub fn texts(list: &DisplayList) -> Vec<(Rect, String, TextStyle, Color)> {
    list.commands
        .iter()
        .filter_map(|cmd| match cmd {
            DrawCmd::Text {
                rect,
                text,
                style,
                color,
                ..
            } => Some((*rect, text.clone(), *style, *color)),
            _ => None,
        })
        .collect()
}

pub fn text(list: &DisplayList, words: &str) -> (Rect, TextStyle, Color) {
    texts(list)
        .into_iter()
        .find(|t| t.1 == words)
        .map(|t| (t.0, t.2, t.3))
        .unwrap_or_else(|| panic!("no text {words:?} in {:#?}", texts(list)))
}

pub fn has(list: &DisplayList, pred: impl Fn(&DrawCmd) -> bool) -> bool {
    list.commands.iter().any(pred)
}

/// Every node of the accessibility tree, depth first.
pub fn a11y_nodes(root: &A11yNode) -> Vec<A11yNode> {
    let mut out = vec![root.clone()];
    for c in &root.children {
        out.extend(a11y_nodes(c));
    }
    out
}

pub fn a11y<M: Clone + 'static>(rt: &Runtime<View<M>>, role: Role) -> Vec<A11yNode> {
    a11y_nodes(&rt.accessibility().expect("tree"))
        .into_iter()
        .filter(|n| n.role == role)
        .collect()
}

pub fn measure() -> FixedMeasure {
    FixedMeasure::default()
}

pub fn ring_rect(r: Rect) -> Rect {
    let out = FOCUS_RING_OFFSET + FOCUS_RING_WIDTH;
    Rect::new(
        r.x - out,
        r.y - out,
        r.width + 2.0 * out,
        r.height + 2.0 * out,
    )
}
