//! Paints the laid-out node tree into a [`DisplayList`].

use std::collections::HashMap;

use crate::contract::{Color, DisplayList, DrawCmd, PathCmd, Point, Radii, Rect, TextAlign};
use crate::element::{BoxDraw, Draw, Node};
use crate::layout::{line_height, Layout};
use crate::pixel;
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
    /// The band each resting pager shows (top and bottom, from its top
    /// edge), by node index.
    pub page_clips: HashMap<usize, (f32, f32)>,
    /// Page indicators, drawn after content.
    pub page_marks: Vec<DrawCmd>,
}

/// A node's transform: scale about `origin`, then translate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Transform {
    pub origin: Point,
    pub scale: f32,
    pub translate: Point,
}

impl Transform {
    /// The transform on a pixel grid: translations move by whole cells and
    /// a scale that is not a whole number is left out, so the element keeps
    /// its pixels. `None` when nothing is left.
    fn on_grid(self, cell: f32) -> Option<Transform> {
        let scale = if self.scale.fract() == 0.0 && self.scale > 0.0 {
            self.scale
        } else {
            1.0
        };
        let translate = Point {
            x: pixel::snap(self.translate.x, cell),
            y: pixel::snap(self.translate.y, cell),
        };
        let t = Transform {
            scale,
            translate,
            ..self
        };
        (scale != 1.0 || translate.x != 0.0 || translate.y != 0.0).then_some(t)
    }

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
    /// The pixel grid already open around this node.
    grid: Option<f32>,
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
            grid: None,
        };
        let scene = Scene {
            nodes,
            layout,
            rects,
            state,
        };
        paint_node(&scene, 0, ctx, &mut list.commands, &mut overlay);
    }
    list.commands.extend(state.page_marks.iter().cloned());
    list.commands.extend(overlay);
    let grid = nodes.first().and_then(|n| n.grid);
    if let Some(tip) = &state.tooltip {
        match grid {
            Some(cell) => {
                list.commands.push(DrawCmd::PushPixelGrid { cell });
                let rect = pixel::snap_rect(tip.rect, cell);
                let fill = pixel::fill(rect, tip.radii, cell);
                pixel::push_rects(&mut list.commands, fill, tip.fill);
            }
            None => list.commands.push(DrawCmd::Rect {
                rect: tip.rect,
                radii: tip.radii,
                fill: tip.fill,
                stroke: None,
            }),
        }
        let text_rect = match grid {
            Some(cell) => {
                let r = pixel::snap_rect(tip.text_rect, cell);
                Rect::new(r.x, r.y, r.width.max(tip.text_rect.width), r.height)
            }
            None => tip.text_rect,
        };
        list.commands.push(DrawCmd::Text {
            rect: text_rect,
            text: tip.text.clone(),
            style: tip.style,
            color: tip.color,
            align: crate::contract::TextAlign::Start,
        });
        if grid.is_some() {
            list.commands.push(DrawCmd::PopPixelGrid);
        }
    }
    for indicator in &state.indicators {
        let faded = indicator.opacity < 1.0;
        if faded {
            list.commands.push(DrawCmd::PushOpacity(indicator.opacity));
        }
        let full = Radii::all(9999.0);
        match grid {
            Some(cell) => pixel::push_rects(
                &mut list.commands,
                pixel::fill(indicator.thumb, full, cell),
                indicator.color,
            ),
            None => list.commands.push(DrawCmd::Rect {
                rect: indicator.thumb,
                radii: clamp_radii(indicator.thumb.width, indicator.thumb.height, full),
                fill: indicator.color,
                stroke: None,
            }),
        }
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
    let grid = node.grid;
    if let Some(cell) = grid {
        rect = pixel::snap_rect(rect, cell);
    }
    let owner = match &node.interact {
        Some(i) => Owner {
            focused: state.focused == Some(node.id),
            pressed: state.pressed == Some(node.id) && !i.disabled && !i.busy,
        },
        None => owner,
    };
    let transform = state.transforms.get(&ix).copied();
    let transform = match grid {
        Some(cell) => transform.and_then(|t| t.on_grid(cell)),
        None => transform,
    };
    if let Some(t) = transform {
        out.push(t.push());
    }
    let faded = node.opacity < 1.0;
    if faded {
        out.push(DrawCmd::PushOpacity(node.opacity));
    }
    let opens_grid = grid.is_some() && grid != ctx.grid;
    if let (true, Some(cell)) = (opens_grid, grid) {
        out.push(DrawCmd::PushPixelGrid { cell });
    }
    match (&node.draw, grid) {
        (Draw::Box(b), Some(cell)) => grid_box(out, rect, b, cell),
        (Draw::Box(b), None) => {
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
        (Draw::Image(i), Some(cell)) => {
            // Stepped corners: the image drawn once per row band of its
            // stepped shape, each clipped square.
            let image = DrawCmd::Image {
                rect,
                radii: Radii::default(),
                image: i.image,
                fit: i.fit,
            };
            if i.radii.0.iter().all(|&r| r <= 0.0) {
                out.push(image);
            } else {
                for band in pixel::fill(rect, i.radii, cell) {
                    out.push(DrawCmd::PushClip {
                        rect: band,
                        radii: Radii::default(),
                    });
                    out.push(image.clone());
                    out.push(DrawCmd::PopClip);
                }
            }
        }
        (Draw::Image(i), None) => out.push(DrawCmd::Image {
            rect,
            radii: clamp_radii(rect.width, rect.height, i.radii),
            image: i.image,
            fit: i.fit,
        }),
        _ => {}
    }
    let pressed = owner.pressed && state.press_strength > 0.0;
    if let (Some(ind), true, Some(cell)) = (node.indicator, pressed, grid) {
        if let Some(overlay) = ind.overlay {
            let fill = overlay.with_alpha(overlay.a * state.press_strength);
            pixel::push_rects(out, pixel::fill(rect, ind.radii, cell), fill);
        }
    } else if let (Some(ind), true) = (node.indicator, pressed) {
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
                        if let Some(cell) = grid {
                            let y = pixel::snap(y - cell / 2.0, cell);
                            let band = Rect::new(rect.x, y, rect.width, cell);
                            pixel::push_rects(out, vec![band], t.color);
                            continue;
                        }
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
        Draw::Bitmap(rows, color) => {
            // Centered in its box, on whole icon pixels.
            let side = node.icon.map_or(rect.width.min(rect.height), |(_, s)| s);
            let px = side / rows.len().max(1) as f32;
            let offset = |free: f32| (free / 2.0 / px).round() * px;
            let art = Rect::new(
                rect.x + offset(rect.width - side),
                rect.y + offset(rect.height - side),
                side,
                side,
            );
            out.extend(pixel::bitmap(art, rows, *color));
        }
        Draw::Spinner(color) => match grid {
            Some(cell) => out.extend(pixel::spinner(rect, *color, state.time, cell)),
            None => out.push(spinner(rect, *color, state.time)),
        },
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
        let rect = match grid {
            Some(cell) => {
                let x = pixel::snap(c.rect.x, cell);
                let r = pixel::snap_rect(c.rect, cell);
                Rect::new(x, r.y, pixel::snap_width(c.rect.width, cell), r.height)
            }
            None => c.rect,
        };
        out.push(DrawCmd::Rect {
            rect,
            radii: Radii::default(),
            fill: c.color,
            stroke: None,
        });
    }
    let clip = match (node.clip, node.scroll) {
        // Clips stay square on a grid; content draws its own steps.
        (Some(_), _) if grid.is_some() => Some(Radii::default()),
        (Some(radii), _) => Some(clamp_radii(rect.width, rect.height, radii)),
        (None, Some(_)) => Some(Radii::default()),
        (None, None) => None,
    };
    // A resting pager shows its page only, never the top of the next.
    let clip_rect = match state.page_clips.get(&ix) {
        Some(&(top, bottom)) => Rect::new(
            rect.x,
            rect.y + top,
            rect.width,
            bottom.min(rect.height) - top,
        ),
        None => rect,
    };
    if let Some(radii) = clip {
        out.push(DrawCmd::PushClip {
            rect: clip_rect,
            radii,
        });
    }
    let inner = Ctx {
        owner,
        viewport: if node.scroll.is_some() || node.camera {
            Some(clip_rect)
        } else {
            ctx.viewport
        },
        opacity: ctx.opacity * node.opacity,
        grid: grid.or(ctx.grid),
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
            match grid {
                Some(cell) => {
                    overlay.push(DrawCmd::PushPixelGrid { cell });
                    grid_ring(overlay, rect, ind.radii, ring, cell);
                    overlay.push(DrawCmd::PopPixelGrid);
                }
                None => overlay.push(focus_ring(rect, ind.radii, ring)),
            }
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
        if let (Some((border, inner)), Some(cell)) = (ind.field, grid) {
            let bw = match &node.draw {
                Draw::Box(b) => b.border.map_or(cell, |(_, w)| w),
                _ => cell,
            };
            grid_field(out, rect, ind.radii, bw, (border, inner), cell);
        } else if let Some((border, inner)) = ind.field {
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
    if opens_grid {
        out.push(DrawCmd::PopPixelGrid);
    }
    if faded {
        out.push(DrawCmd::PopOpacity);
    }
    if transform.is_some() {
        out.push(DrawCmd::PopTransform);
    }
}

/// A box on a pixel grid: hard shadows, fill, and border as whole cells.
/// Shadows with blur have no pixel form and stay smooth.
fn grid_box(out: &mut Vec<DrawCmd>, rect: Rect, b: &BoxDraw, cell: f32) {
    let radii = b.radii;
    // CSS paints the first layer on top.
    for layer in b.shadow.iter().rev() {
        let color: Color = layer.color.into();
        if layer.blur > 0.0 {
            out.push(DrawCmd::Shadow {
                rect,
                radii: clamp_radii(rect.width, rect.height, radii),
                layers: shadow_layers(std::slice::from_ref(layer)),
            });
        } else if layer.is_inset {
            if layer.spread > 0.0 {
                let r = Rect::new(rect.x + layer.x, rect.y + layer.y, rect.width, rect.height);
                pixel::push_rects(out, pixel::frame(r, radii, layer.spread, cell), color);
            }
        } else {
            let s = layer.spread;
            let r = Rect::new(
                rect.x + layer.x - s,
                rect.y + layer.y - s,
                rect.width + 2.0 * s,
                rect.height + 2.0 * s,
            );
            let grown = Radii(
                radii
                    .0
                    .map(|v| if v > 0.0 { (v + s).max(0.0) } else { 0.0 }),
            );
            pixel::push_rects(out, pixel::fill(r, grown, cell), color);
        }
    }
    if b.fill.a > 0.0 {
        pixel::push_rects(out, pixel::fill(rect, radii, cell), b.fill);
    }
    if let Some((color, width)) = b.border {
        pixel::push_rects(out, pixel::frame(rect, radii, width, cell), color);
    }
}

/// The focus ring on a pixel grid: one cell wide, one cell outside the
/// edge, stepped.
fn grid_ring(out: &mut Vec<DrawCmd>, rect: Rect, radii: Radii, color: Color, cell: f32) {
    let by = 2.0 * cell;
    let r = Rect::new(
        rect.x - by,
        rect.y - by,
        rect.width + 2.0 * by,
        rect.height + 2.0 * by,
    );
    let radii = Radii(radii.0.map(|v| if v > 0.0 { v + by } else { 0.0 }));
    pixel::push_rects(out, pixel::frame(r, radii, cell, cell), color);
}

/// A text field's focus on a pixel grid: its border and an inner ring,
/// both stepped.
fn grid_field(
    out: &mut Vec<DrawCmd>,
    rect: Rect,
    radii: Radii,
    border_width: f32,
    (border, inner): (Color, Color),
    cell: f32,
) {
    let bw = pixel::snap_width(border_width, cell);
    let inside = Radii(radii.0.map(|r| (r - bw).max(0.0)));
    let ring = pixel::snap_width(2.0, cell);
    pixel::push_rects(out, pixel::frame(rect.inset(bw), inside, ring, cell), inner);
    pixel::push_rects(out, pixel::frame(rect, radii, bw, cell), border);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transforms_on_a_grid_keep_whole_pixels() {
        let t = |scale, x, y| Transform {
            origin: Point { x: 10.0, y: 10.0 },
            scale,
            translate: Point { x, y },
        };
        // A press scale alone has no pixel form.
        assert_eq!(t(0.98, 0.0, 0.0).on_grid(2.0), None);
        // A moving thumb steps by whole cells.
        assert_eq!(t(1.0, 4.9, -1.2).on_grid(2.0), Some(t(1.0, 4.0, -2.0)));
        assert_eq!(t(0.5, 3.1, 0.0).on_grid(2.0), Some(t(1.0, 4.0, 0.0)));
        assert_eq!(t(2.0, 0.0, 0.0).on_grid(2.0), Some(t(2.0, 0.0, 0.0)));
    }
}
