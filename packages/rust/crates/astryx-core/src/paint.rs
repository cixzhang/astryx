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
}

#[derive(Clone, Copy, Default)]
struct Owner {
    focused: bool,
    pressed: bool,
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
    if !nodes.is_empty() {
        paint_node(
            nodes,
            layout,
            state,
            0,
            Owner::default(),
            &mut list.commands,
        );
    }
    list
}

fn paint_node<Msg>(
    nodes: &[Node<Msg>],
    layout: &Layout,
    state: &PaintState,
    ix: usize,
    owner: Owner,
    out: &mut Vec<DrawCmd>,
) {
    let node = &nodes[ix];
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
    let clip = match (node.clip, node.scroll) {
        (Some(radii), _) => Some(clamp_radii(rect.width, rect.height, radii)),
        (None, Some(_)) => Some(Radii::default()),
        (None, None) => None,
    };
    if let Some(radii) = clip {
        out.push(DrawCmd::PushClip { rect, radii });
    }
    for &child in &node.children {
        paint_node(nodes, layout, state, child as usize, owner, out);
    }
    if clip.is_some() {
        out.push(DrawCmd::PopClip);
    }
    if let (Some(ind), true) = (node.indicator, owner.focused) {
        if let Some(ring) = ind.ring {
            out.push(focus_ring(rect, ind.radii, ring));
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
