//! Experimental: lights placed in the element tree, so they move with what
//! they light.
//!
//! [`Lit`] lights its children as one light pass. A [`LightSource`] is a
//! zero-size point that a [`Board`](crate::world::Board) places with
//! `child_at` like a sprite; an [`Emissive`] is a rect that glows,
//! unlit. Both resolve their position from layout, so the camera, page
//! slides and board glides carry them with their sprites.
//!
//! None of this is an Astryx component yet; names and behavior may change.

use crate::contract::{Color, Light, Lighting, Point, Rect};
use crate::element::{base_props, Base, Build, BuildCx, Draw, Element, Node};
use taffy::prelude::*;

/// What a light element draws.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LightDraw {
    /// Opens a light pass around the node's children. `still` holds the
    /// lighting's motion: reduced motion, or nothing that moves.
    Pass { lighting: Lighting, still: bool },
    /// A light centered on the node's top left corner, in points from it.
    Source(Light),
    /// The node's rect, glowing.
    Emissive(Color),
}

impl LightDraw {
    /// It changes over time on its own: drifting shade, flicker or pulse.
    pub(crate) fn plays(&self) -> bool {
        match self {
            LightDraw::Pass { still, .. } => !still,
            LightDraw::Source(l) => l.flicker.amplitude > 0.0 || l.pulse.period > 0.0,
            LightDraw::Emissive(_) => false,
        }
    }

    /// The area it affects, for a node laid out at `rect`.
    pub(crate) fn reach(&self, rect: Rect) -> Rect {
        match self {
            LightDraw::Source(l) => {
                let c = placed(
                    l,
                    Point {
                        x: rect.x,
                        y: rect.y,
                    },
                )
                .center;
                Rect::new(
                    c.x - l.radius_x,
                    c.y - l.radius_y,
                    2.0 * l.radius_x,
                    2.0 * l.radius_y,
                )
            }
            _ => rect,
        }
    }
}

/// Lights its children as one light pass: they draw as usual, then the
/// light map of `lighting` and every [`LightSource`] inside multiplies over
/// them. What is drawn after it, such as the UI, stays unlit.
///
/// The pass's time comes from the runtime, which holds it still under
/// reduced motion; `lighting.time` is ignored. Light passes do not nest.
pub struct Lit<Msg> {
    base: Base,
    lighting: Lighting,
    children: Vec<Element<Msg>>,
}

impl<Msg> Lit<Msg> {
    pub fn new(lighting: Lighting) -> Self {
        Lit {
            base: Base::default(),
            lighting,
            children: Vec::new(),
        }
    }

    pub fn child(mut self, child: impl Into<Element<Msg>>) -> Self {
        self.children.push(child.into());
        self
    }
}

base_props!(Lit<Msg>);

impl<Msg> Build<Msg> for Lit<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let drifts = self
            .lighting
            .dapple
            .is_some_and(|d| d.velocity.x != 0.0 || d.velocity.y != 0.0);
        let still = !(drifts && cx.animate());
        cx.key(self.base.id.as_deref());
        cx.begin(
            Node::new(Style::default()).draw(Draw::Light(LightDraw::Pass {
                lighting: self.lighting,
                still,
            })),
        );
        for child in self.children {
            cx.child(child);
        }
        cx.end();
    }
}

/// A light at this element's place: a zero-size point, centered where a
/// board or stack puts its top left corner. It lights the [`Lit`] around
/// it, and draws nothing elsewhere. Flicker and pulse play unless the
/// person asked for reduced motion.
pub struct LightSource {
    base: Base,
    light: Light,
}

impl LightSource {
    /// `light.center` offsets the light from the element's place.
    pub fn new(light: Light) -> Self {
        LightSource {
            base: Base::default(),
            light,
        }
    }
}

base_props!(LightSource);

impl<Msg> Build<Msg> for LightSource {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let mut light = self.light;
        let moves = light.flicker.amplitude > 0.0 || light.pulse.period > 0.0;
        if moves && !cx.animate() {
            light.flicker = Default::default();
            light.pulse = Default::default();
        }
        cx.key(self.base.id.as_deref());
        cx.leaf(Node::new(zero_size()).draw(Draw::Light(LightDraw::Source(light))));
    }
}

/// A rect that glows in a [`Lit`]: drawn after the light map, so it is not
/// darkened, as a lit window or a lantern's glass.
pub struct Emissive {
    base: Base,
    color: Color,
    width: f32,
    height: f32,
}

impl Emissive {
    pub fn new(color: Color) -> Self {
        Emissive {
            base: Base::default(),
            color,
            width: 0.0,
            height: 0.0,
        }
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
}

base_props!(Emissive);

impl<Msg> Build<Msg> for Emissive {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        cx.key(self.base.id.as_deref());
        cx.leaf(
            Node::new(Style {
                size: Size {
                    width: length(self.width),
                    height: length(self.height),
                },
                flex_shrink: 0.0,
                ..Style::default()
            })
            .draw(Draw::Light(LightDraw::Emissive(self.color))),
        );
    }
}

fn zero_size() -> Style {
    Style {
        size: Size {
            width: length(0.0),
            height: length(0.0),
        },
        flex_shrink: 0.0,
        ..Style::default()
    }
}

/// The light of `source` placed at `at`, the node's top left corner.
pub(crate) fn placed(source: &Light, at: Point) -> Light {
    Light {
        center: Point {
            x: at.x + source.center.x,
            y: at.y + source.center.y,
        },
        ..*source
    }
}
