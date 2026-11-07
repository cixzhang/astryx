//! Paints the laid-out node tree into a [`DisplayList`].

use std::collections::HashMap;

use crate::contract::{Color, DisplayList, DrawCmd, PathCmd, Point, Radii, Rect, TextAlign};
use crate::element::{Draw, Node};
use crate::layout::{line_height, Layout};
use crate::theme::{clamp_radii, shadow_layers};

/// Keyboard focus ring: 2 points wide, 3 points outside the element, as the
/// Astryx `--focus-outline-*` tokens set it.
pub const FOCUS_RING_WIDTH: f32 = 2.0;
pub const FOCUS_RING_OFFSET: f32 = 3.0;

/// The interaction state the painter needs.
pub(crate) struct PaintState {
    pub focused: Option<u64>,
    pub pressed: Option<u64>,
    /// The pressed overlay's strength, 0 to 1.
    pub press_strength: f32,
    /// Seconds since the runtime started, for spinners.
    pub time: f32,
    pub caret: Option<Caret>,
    /// Scroll indicators, drawn above everything.
    pub indicators: Vec<Indicator>,
    /// The focused element's tooltip, drawn above everything.
    pub tooltip: Option<Tooltip>,
    /// Transforms by node index: the press scale and moving nodes on their way.
    pub transforms: HashMap<usize, Transform>,
}

/// A node's transform: scale about `origin`, then translate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Transform {
    pub origin: Point,
    pub scale: f32,
    pub translate: Point,
}

impl Transform {
    fn push(self) -> DrawCmd {
        DrawCmd::PushTransform {
            origin: self.origin,
            scale: self.scale,
            translate: self.translate,
        }
    }
}

/// A tooltip bubble: an inverted surface with wrapped text.
pub(crate) struct Tooltip {
    pub rect: Rect,
    pub radii: Radii,
    pub fill: Color,
    pub text: String,
    pub text_rect: Rect,
    pub style: crate::contract::TextStyle,
    pub color: Color,
}

/// A scroll container's position thumb.
pub(crate) struct Indicator {
    pub thumb: Rect,
    pub opacity: f32,
    pub color: Color,
}

/// The focused text field's caret.
pub(crate) struct Caret {
    /// The node that draws the field's value.
    pub text_node: usize,
    /// How far the value scrolls left so the caret stays in view.
    pub shift: f32,
    pub rect: Rect,
    pub color: Color,
    /// The value with composing text in it, and its width, while composing.
    pub text: Option<(String, f32)>,
    /// The line under the composing text.
    pub underline: Option<Rect>,
}

#[derive(Clone, Copy, Default)]
struct Owner {
    focused: bool,
    pressed: bool,
}

/// What a node inherits from its ancestors while painting.
#[derive(Clone, Copy)]
struct Ctx {
    owner: Owner,
    /// The painted rect of the nearest scroll container.
    viewport: Option<Rect>,
    /// The product of the ancestors' opacities.
    opacity: f32,
}

pub(crate) fn paint<Msg>(
    nodes: &[Node<Msg>],
    layout: &Layout,
    rects: &[Rect],
    state: &PaintState,
    clear: Color,
) -> DisplayList {
    let mut list = DisplayList {
        clear,
        commands: Vec::with_capacity(nodes.len() * 2),
    };
    // Focus rings paint after all content, so no card or other clip cuts
    // them; only the nearest scroll viewport, grown by the ring, clips them.
    let mut overlay = Vec::new();
    if !nodes.is_empty() {
        let ctx = Ctx {
            owner: Owner::default(),
            viewport: None,
            opacity: 1.0,
        };
        let scene = Scene {
            nodes,
            layout,
            rects,
            state,
        };
        paint_node(&scene, 0, ctx, &mut list.commands, &mut overlay);
    }
    list.commands.extend(overlay);
    if let Some(tip) = &state.tooltip {
        list.commands.push(DrawCmd::Rect {
            rect: tip.rect,
            radii: tip.radii,
            fill: tip.fill,
            stroke: None,
        });
        list.commands.push(DrawCmd::Text {
            rect: tip.text_rect,
            text: tip.text.clone(),
            style: tip.style,
            color: tip.color,
            align: crate::contract::TextAlign::Start,
        });
    }
    for indicator in &state.indicators {
        let faded = indicator.opacity < 1.0;
        if faded {
            list.commands.push(DrawCmd::PushOpacity(indicator.opacity));
        }
        list.commands.push(DrawCmd::Rect {
            rect: indicator.thumb,
            radii: clamp_radii(
                indicator.thumb.width,
                indicator.thumb.height,
                Radii::all(9999.0),
            ),
            fill: indicator.color,
            stroke: None,
        });
        if faded {
            list.commands.push(DrawCmd::PopOpacity);
        }
    }
    list
}

/// What every node's painting reads.
struct Scene<'a, Msg> {
    nodes: &'a [Node<Msg>],
    layout: &'a Layout,
    rects: &'a [Rect],
    state: &'a PaintState,
}

fn paint_node<Msg>(
    scene: &Scene<'_, Msg>,
    ix: usize,
    ctx: Ctx,
    out: &mut Vec<DrawCmd>,
    overlay: &mut Vec<DrawCmd>,
) {
    let (nodes, layout, rects, state) = (scene.nodes, scene.layout, scene.rects, scene.state);
    let node = &nodes[ix];
    let owner = ctx.owner;
    let mut rect = rects[ix];
    if node.centered {
        if let Some(p) = node.parent {
            let parent = rects[p as usize];
            rect.x = parent.x + (parent.width - rect.width) / 2.0;
            rect.y = parent.y + (parent.height - rect.height) / 2.0;
        }
    }
    if let (Some(period), Some(p)) = (node.slide, node.parent) {
        let parent = rects[p as usize];
        let phase = (state.time % period) / period;
        let travel = rect.width * (-1.0 + 3.5 * phase);
        // The slide runs from inline start to inline end.
        rect.x = if node.rtl {
            parent.x + parent.width - rect.width - travel
        } else {
            parent.x + travel
        };
    }
    let caret = state.caret.as_ref().filter(|c| c.text_node == ix);
    if let Some(c) = caret {
        rect.x -= c.shift;
    }
    let owner = match &node.interact {
        Some(i) => Owner {
            focused: state.focused == Some(node.id),
            pressed: state.pressed == Some(node.id) && !i.disabled && !i.busy,
        },
        None => owner,
    };
    let transform = state.transforms.get(&ix).copied();
    if let Some(t) = transform {
        out.push(t.push());
    }
    let faded = node.opacity < 1.0;
    if faded {
        out.push(DrawCmd::PushOpacity(node.opacity));
    }
    match &node.draw {
        Draw::None => {}
        Draw::Box(b) => {
            let radii = clamp_radii(rect.width, rect.height, b.radii);
            if !b.shadow.is_empty() {
                out.push(DrawCmd::Shadow {
                    rect,
                    radii,
                    layers: shadow_layers(b.shadow),
                });
            }
            if b.fill.a > 0.0 || b.border.is_some() {
                out.push(DrawCmd::Rect {
                    rect,
                    radii,
                    fill: b.fill,
                    stroke: b.border,
                });
            }
        }
        Draw::Image(i) => out.push(DrawCmd::Image {
            rect,
            radii: clamp_radii(rect.width, rect.height, i.radii),
            image: i.image,
            fit: i.fit,
        }),
        _ => {}
    }
    if let (Some(ind), true) = (node.indicator, owner.pressed && state.press_strength > 0.0) {
        if let Some(overlay) = ind.overlay {
            out.push(DrawCmd::Rect {
                rect,
                radii: clamp_radii(rect.width, rect.height, ind.radii),
                fill: overlay.with_alpha(overlay.a * state.press_strength),
                stroke: None,
            });
        }
    }
    match &node.draw {
        Draw::Text(t) => {
            let mut text = layout.texts[ix].clone().unwrap_or_else(|| t.text.clone());
            let mut rect = rect;
            let mut color = t.color;
            if let Some((composed, width)) = caret.and_then(|c| c.text.as_ref()) {
                text = composed.clone();
                rect.width = rect.width.max(*width);
                color = caret.map_or(color, |c| c.color);
            }
            let t = &crate::element::TextDraw { color, ..t.clone() };
            if t.color.a > 0.0 {
                // Renderers align physically; start is the right edge under RTL.
                let align = match (t.align, node.rtl) {
                    (TextAlign::Start, true) => TextAlign::End,
                    (TextAlign::End, true) => TextAlign::Start,
                    (align, _) => align,
                };
                out.push(DrawCmd::Text {
                    rect,
                    text,
                    style: t.style,
                    color: t.color,
                    align,
                });
                if t.strikethrough {
                    let lh = line_height(t);
                    let lines = ((rect.height / lh).round() as usize).max(1);
                    for line in 0..lines {
                        let y = rect.y + lh * (line as f32 + 0.5);
                        out.push(DrawCmd::Line {
                            from: Point { x: rect.x, y },
                            to: Point {
                                x: rect.x + rect.width,
                                y,
                            },
                            color: t.color,
                            width: 1.0,
                        });
                    }
                }
            }
        }
        Draw::Path(p) => {
            let s = rect.width.min(rect.height) / p.view_box;
            let (ox, oy) = (
                rect.x + (rect.width - p.view_box * s) / 2.0,
                rect.y + (rect.height - p.view_box * s) / 2.0,
            );
            let map = |pt: Point| Point {
                x: ox + pt.x * s,
                y: oy + pt.y * s,
            };
            let commands = p
                .commands
                .iter()
                .map(|c| match *c {
                    PathCmd::MoveTo(a) => PathCmd::MoveTo(map(a)),
                    PathCmd::LineTo(a) => PathCmd::LineTo(map(a)),
                    PathCmd::QuadTo(a, b) => PathCmd::QuadTo(map(a), map(b)),
                    PathCmd::CubicTo(a, b, c) => PathCmd::CubicTo(map(a), map(b), map(c)),
                    PathCmd::Close => PathCmd::Close,
                })
                .collect();
            out.push(DrawCmd::Path {
                commands,
                fill: p.fill,
                stroke: p.stroke.map(|(c, w)| (c, w * s)),
            });
        }
        Draw::Spinner(color) => out.push(spinner(rect, *color, state.time)),
        _ => {}
    }
    if let Some(line) = caret.and_then(|c| c.underline) {
        out.push(DrawCmd::Rect {
            rect: line,
            radii: Radii::default(),
            fill: caret.map_or(Color::TRANSPARENT, |c| c.color),
            stroke: None,
        });
    }
    if let Some(c) = caret {
        out.push(DrawCmd::Rect {
            rect: c.rect,
            radii: Radii::default(),
            fill: c.color,
            stroke: None,
        });
    }
    let clip = match (node.clip, node.scroll) {
        (Some(radii), _) => Some(clamp_radii(rect.width, rect.height, radii)),
        (None, Some(_)) => Some(Radii::default()),
        (None, None) => None,
    };
    if let Some(radii) = clip {
        out.push(DrawCmd::PushClip { rect, radii });
    }
    let inner = Ctx {
        owner,
        viewport: if node.scroll.is_some() {
            Some(rect)
        } else {
            ctx.viewport
        },
        opacity: ctx.opacity * node.opacity,
    };
    for &child in &node.children {
        paint_node(scene, child as usize, inner, out, overlay);
    }
    if clip.is_some() {
        out.push(DrawCmd::PopClip);
    }
    if let (Some(ind), true) = (node.indicator, owner.focused) {
        if let Some(ring) = ind.ring {
            if let Some(view) = ctx.viewport {
                let reach = FOCUS_RING_OFFSET + FOCUS_RING_WIDTH;
                let grown = Rect::new(
                    view.x - reach,
                    view.y - reach,
                    view.width + 2.0 * reach,
                    view.height + 2.0 * reach,
                );
                overlay.push(DrawCmd::PushClip {
                    rect: grown,
                    radii: Radii::default(),
                });
            }
            if inner.opacity < 1.0 {
                overlay.push(DrawCmd::PushOpacity(inner.opacity));
            }
            // The ring scales with its element, as a CSS outline does.
            if let Some(t) = transform {
                overlay.push(t.push());
            }
            overlay.push(focus_ring(rect, ind.radii, ring));
            if transform.is_some() {
                overlay.push(DrawCmd::PopTransform);
            }
            if inner.opacity < 1.0 {
                overlay.push(DrawCmd::PopOpacity);
            }
            if ctx.viewport.is_some() {
                overlay.push(DrawCmd::PopClip);
            }
        }
        if let Some((border, inner)) = ind.field {
            let bw = match &node.draw {
                Draw::Box(b) => b.border.map_or(1.0, |(_, w)| w),
                _ => 1.0,
            };
            let radii = clamp_radii(rect.width, rect.height, ind.radii);
            let inside = Radii(radii.0.map(|r| (r - bw).max(0.0)));
            out.push(DrawCmd::Rect {
                rect: rect.inset(bw),
                radii: inside,
                fill: Color::TRANSPARENT,
                stroke: Some((inner, 2.0)),
            });
            out.push(DrawCmd::Rect {
                rect,
                radii,
                fill: Color::TRANSPARENT,
                stroke: Some((border, bw)),
            });
        }
    }
    if faded {
        out.push(DrawCmd::PopOpacity);
    }
    if transform.is_some() {
        out.push(DrawCmd::PopTransform);
    }
}

/// The ring around `rect`: `offset` outside the edge and `width` wide, with
/// corners that follow the element's own, as a CSS outline does. Strokes
/// draw inside their rect, so the ring's rect is its outer edge.
pub(crate) fn focus_ring(rect: Rect, radii: Radii, color: Color) -> DrawCmd {
    let out = FOCUS_RING_OFFSET + FOCUS_RING_WIDTH;
    let radii = clamp_radii(rect.width, rect.height, radii);
    let ring = Rect::new(
        rect.x - out,
        rect.y - out,
        rect.width + 2.0 * out,
        rect.height + 2.0 * out,
    );
    DrawCmd::Rect {
        rect: ring,
        radii: Radii(radii.0.map(|r| if r > 0.0 { r + out } else { 0.0 })),
        fill: Color::TRANSPARENT,
        stroke: Some((color, FOCUS_RING_WIDTH)),
    }
}

/// A three-quarter arc that turns once a second.
fn spinner(rect: Rect, color: Color, time: f32) -> DrawCmd {
    let width = (rect.width.min(rect.height) / 8.0).max(1.5);
    let r = rect.width.min(rect.height) / 2.0 - width / 2.0;
    let c = rect.center();
    let start = (time % 1.0) * std::f32::consts::TAU;
    let mut commands = Vec::with_capacity(4);
    let point = |a: f32| Point {
        x: c.x + r * a.cos(),
        y: c.y + r * a.sin(),
    };
    commands.push(PathCmd::MoveTo(point(start)));
    // Three quarter-circle cubic segments.
    let k = 0.552_284_8 * r;
    for q in 0..3 {
        let a0 = start + q as f32 * std::f32::consts::FRAC_PI_2;
        let a1 = a0 + std::f32::consts::FRAC_PI_2;
        let (p0, p3) = (point(a0), point(a1));
        let c1 = Point {
            x: p0.x - k * a0.sin(),
            y: p0.y + k * a0.cos(),
        };
        let c2 = Point {
            x: p3.x + k * a1.sin(),
            y: p3.y - k * a1.cos(),
        };
        commands.push(PathCmd::CubicTo(c1, c2, p3));
    }
    DrawCmd::Path {
        commands,
        fill: None,
        stroke: Some((color, width)),
    }
}
