//! Paints the laid-out node tree into a [`DisplayList`].

use crate::contract::{Color, DisplayList, DrawCmd, PathCmd, Point, Radii, Rect};
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
    /// Seconds since the runtime started, for spinners.
    pub time: f32,
    pub caret: Option<Caret>,
}

/// The focused text field's caret.
pub(crate) struct Caret {
    /// The node that draws the field's value.
    pub text_node: usize,
    /// How far the value scrolls left so the caret stays in view.
    pub shift: f32,
    pub rect: Rect,
    pub color: Color,
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
        paint_node(
            nodes,
            layout,
            state,
            0,
            ctx,
            &mut list.commands,
            &mut overlay,
        );
    }
    list.commands.extend(overlay);
    list
}

fn paint_node<Msg>(
    nodes: &[Node<Msg>],
    layout: &Layout,
    state: &PaintState,
    ix: usize,
    ctx: Ctx,
    out: &mut Vec<DrawCmd>,
    overlay: &mut Vec<DrawCmd>,
) {
    let node = &nodes[ix];
    let owner = ctx.owner;
    let mut rect = layout.rects[ix];
    if node.centered {
        if let Some(p) = node.parent {
            let parent = layout.rects[p as usize];
            rect.x = parent.x + (parent.width - rect.width) / 2.0;
            rect.y = parent.y + (parent.height - rect.height) / 2.0;
        }
    }
    if let (Some(period), Some(p)) = (node.slide, node.parent) {
        let parent = layout.rects[p as usize];
        let phase = (state.time % period) / period;
        rect.x = parent.x + rect.width * (-1.0 + 3.5 * phase);
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
        _ => {}
    }
    if let (Some(ind), true) = (node.indicator, owner.pressed) {
        if let Some(overlay) = ind.overlay {
            out.push(DrawCmd::Rect {
                rect,
                radii: clamp_radii(rect.width, rect.height, ind.radii),
                fill: overlay,
                stroke: None,
            });
        }
    }
    match &node.draw {
        Draw::Text(t) => {
            let text = layout.texts[ix].clone().unwrap_or_else(|| t.text.clone());
            if t.color.a > 0.0 {
                out.push(DrawCmd::Text {
                    rect,
                    text,
                    style: t.style,
                    color: t.color,
                    align: t.align,
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
        paint_node(nodes, layout, state, child as usize, inner, out, overlay);
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
            overlay.push(focus_ring(rect, ind.radii, ring));
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
