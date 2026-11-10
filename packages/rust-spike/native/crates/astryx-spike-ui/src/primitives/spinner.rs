//! Spinner: an arc turning on a track, for work of unknown length.

use crate::util::{color, component, len, square};
use astryx_core::authoring::{Base, Build, BuildCx, Draw, Node, Painter, Semantics};
use astryx_core::contract::{Color, DrawCmd, PathCmd, Point, Rect, Role};
use astryx_core::Text;
use astryx_spike_foundations::{args, CxExt};
use taffy::{AlignItems, FlexDirection, Style};

/// Ring diameter: 10, 14, 18 or 28 points.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SpinnerSize {
    Sm,
    #[default]
    Md,
    Lg,
    Xl,
}

impl SpinnerSize {
    /// Ring diameter and stroke width in points.
    pub fn geometry(self) -> (f32, f32) {
        match self {
            SpinnerSize::Sm => (10.0, 2.0),
            SpinnerSize::Md => (14.0, 3.0),
            SpinnerSize::Lg => (18.0, 3.0),
            SpinnerSize::Xl => (28.0, 4.0),
        }
    }
}

/// Color shade for light or dark grounds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SpinnerShade {
    #[default]
    Default,
    Subtle,
    OnMedia,
    Inherit,
}

/// The fraction of the ring the moving arc covers (135 degrees).
const ARC_FRACTION: f32 = 0.375;
/// One turn at the theme's `duration-slow-min`, and 3 s under reduced motion.
const TURN_SECONDS: f32 = 0.73;
const REDUCED_TURN_SECONDS: f32 = 3.0;

/// An animated loading indicator (`Spinner`). Reduced motion slows the turn
/// but never stops it: a still ring reads as broken.
#[derive(Clone, Debug, Default)]
pub struct Spinner {
    base: Base,
    size: SpinnerSize,
    shade: SpinnerShade,
    label: Option<String>,
    aria_label: Option<String>,
}

impl Spinner {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn size(mut self, size: SpinnerSize) -> Self {
        self.size = size;
        self
    }
    pub fn shade(mut self, shade: SpinnerShade) -> Self {
        self.shade = shade;
        self
    }
    /// Visible words below the ring; they also name it.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    /// The accessible name when there is no visible label.
    pub fn aria_label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = Some(label.into());
        self
    }
}

component!(Spinner);

/// Points on a circle, as cubic segments of at most a quarter turn each.
fn arc(center: Point, r: f32, start: f32, sweep: f32) -> Vec<PathCmd> {
    let point = |a: f32| Point {
        x: center.x + r * a.cos(),
        y: center.y + r * a.sin(),
    };
    let segments = (sweep / std::f32::consts::FRAC_PI_2).ceil().max(1.0) as usize;
    let step = sweep / segments as f32;
    let k = 4.0 / 3.0 * (step / 4.0).tan() * r;
    let mut out = vec![PathCmd::MoveTo(point(start))];
    for i in 0..segments {
        let a0 = start + step * i as f32;
        let a1 = a0 + step;
        let (p0, p3) = (point(a0), point(a1));
        let c1 = Point {
            x: p0.x - k * a0.sin(),
            y: p0.y + k * a0.cos(),
        };
        let c2 = Point {
            x: p3.x + k * a1.sin(),
            y: p3.y - k * a1.cos(),
        };
        out.push(PathCmd::CubicTo(c1, c2, p3));
    }
    out
}

/// The ring for one frame: the full track, then the arc from the top,
/// turned by the clock.
pub(crate) fn ring_painter(
    diameter: f32,
    stroke: f32,
    arc_color: Color,
    track: Color,
    turn_seconds: f32,
) -> Painter {
    Painter::new(true, move |rect: Rect, time: f32| {
        let c = rect.center();
        let r = diameter / 2.0;
        let tau = std::f32::consts::TAU;
        let turn = (time / turn_seconds).fract() * tau;
        let start = -std::f32::consts::FRAC_PI_2 + turn;
        let mut out = Vec::with_capacity(2);
        if track.a > 0.0 {
            out.push(DrawCmd::Path {
                commands: arc(c, r, 0.0, tau),
                fill: None,
                stroke: Some((track, stroke)),
            });
        }
        out.push(DrawCmd::Path {
            commands: arc(c, r, start, ARC_FRACTION * tau),
            fill: None,
            stroke: Some((arc_color, stroke)),
        });
        out
    })
}

impl<Msg: 'static> Build<Msg> for Spinner {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let inherited = cx.inherited().color;
        let (arc_color, track, track_opacity) = match self.shade {
            SpinnerShade::Default => (color(c.accent), color(c.track), 1.0),
            SpinnerShade::Subtle => (color(c.text_secondary), color(c.track), 1.0),
            SpinnerShade::OnMedia => (color(c.on_dark), color(c.on_dark), 77.0 / 255.0),
            SpinnerShade::Inherit => (inherited, inherited, 0.3),
        };
        let track = track.with_alpha(track.a * track_opacity);
        let (diameter, stroke) = self.size.geometry();
        let side = diameter + stroke * 2.0;
        let name = self
            .label
            .clone()
            .or(self.aria_label)
            .unwrap_or_else(|| cx.t("@astryx.spinner.loading", &args! {}));
        let turn = if cx.prefs.reduced_motion {
            REDUCED_TURN_SECONDS
        } else {
            TURN_SECONDS
        };
        // The ring turns under reduced motion too, so frames keep coming.
        cx.animating = true;
        cx.key(self.base.id.as_deref());
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::CENTER,
                gap: taffy::Size {
                    width: len(0.0),
                    height: len(t.space.s2),
                },
                flex_shrink: 0.0,
                ..Style::default()
            })
            .sem(Semantics::new(Role::Status, name)),
        );
        cx.leaf(
            Node::new(Style {
                size: square(side),
                flex_shrink: 0.0,
                ..Style::default()
            })
            .draw(Draw::Paint(ring_painter(
                diameter, stroke, arc_color, track, turn,
            ))),
        );
        if let Some(label) = self.label {
            cx.child(Text::new(label).into());
        }
        cx.end();
    }
}

/// The ring of an inherit-shade spinner in `ink`, for controls that show
/// their own busy state (Button, Switch, inputs). Keeps frames coming.
pub(crate) fn spinner_painter<Msg>(
    cx: &mut BuildCx<Msg>,
    diameter: f32,
    stroke: f32,
    ink: Color,
) -> Painter {
    cx.animating = true;
    let turn = if cx.prefs.reduced_motion {
        REDUCED_TURN_SECONDS
    } else {
        TURN_SECONDS
    };
    ring_painter(diameter, stroke, ink, ink.with_alpha(ink.a * 0.3), turn)
}
