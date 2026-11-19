//! What changed between two frames, so the renderer redraws only there.
//!
//! [`walk`] gives every command of a list the pixels it can touch (its ink
//! bounds, generous); a group's push and pop get the bounds of everything
//! inside the group, because changing a clip, an opacity or a transform
//! changes how all of it draws. [`changed`] compares two lists command by
//! command and returns the bounds of the commands that differ, in either
//! list.

use astryx_core::{DisplayList, DrawCmd, PathCmd, TextMeasure};
use astryx_raster::geom::{fit_radii, line_rect, rect_px, Affine, PxRect};

/// A list's commands with their bounds in physical pixels; `None` for a
/// command that draws nothing.
pub(crate) struct Walk {
    pub bounds: Vec<Option<PxRect>>,
    /// The list lights the frame: lighting covers everything, so frames
    /// with it redraw whole.
    pub lighting: bool,
    /// Frame rows, as pixel intervals, where some shape drawn as a path has
    /// a curve or an edge that is not vertical: bands must not meet there.
    pub no_seam: Vec<(f32, f32)>,
}

/// How a group opens.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Clip,
    Opacity,
    Transform,
    Grid,
    Lighting,
}

fn kind_of(cmd: &DrawCmd) -> Option<(Kind, bool)> {
    Some(match cmd {
        DrawCmd::PushClip { .. } => (Kind::Clip, true),
        DrawCmd::PopClip => (Kind::Clip, false),
        DrawCmd::PushOpacity(_) => (Kind::Opacity, true),
        DrawCmd::PopOpacity => (Kind::Opacity, false),
        DrawCmd::PushTransform { .. } => (Kind::Transform, true),
        DrawCmd::PopTransform => (Kind::Transform, false),
        DrawCmd::PushPixelGrid { .. } => (Kind::Grid, true),
        DrawCmd::PopPixelGrid => (Kind::Grid, false),
        DrawCmd::PushLighting(_) => (Kind::Lighting, true),
        DrawCmd::PopLighting => (Kind::Lighting, false),
        _ => return None,
    })
}

pub(crate) fn union(a: Option<PxRect>, b: Option<PxRect>) -> Option<PxRect> {
    match (a, b) {
        (Some(a), Some(b)) => Some(PxRect {
            x0: a.x0.min(b.x0),
            y0: a.y0.min(b.y0),
            x1: a.x1.max(b.x1),
            y1: a.y1.max(b.y1),
        }),
        (a, None) => a,
        (None, b) => b,
    }
}

fn grow(r: PxRect, by: f32) -> PxRect {
    PxRect {
        x0: r.x0 - by,
        y0: r.y0 - by,
        x1: r.x1 + by,
        y1: r.y1 + by,
    }
}

/// The pixels a command already moved by its transform can touch, with a
/// margin for antialiasing. `s` is the frame scale.
pub(crate) fn ink(cmd: &DrawCmd, s: f32, snap: bool, text: &dyn TextMeasure) -> Option<PxRect> {
    let r = match cmd {
        DrawCmd::Rect { rect, .. } | DrawCmd::Image { rect, .. } => rect_px(rect, s, snap),
        DrawCmd::Shadow { rect, layers, .. } => {
            let reach = layers.iter().fold(0.0f32, |a, l| {
                let outset = if l.inset {
                    0.0
                } else {
                    1.5 * l.blur.max(0.0) + l.spread.abs() + l.x.abs() + l.y.abs()
                };
                a.max(outset)
            });
            grow(rect_px(rect, s, snap), reach * s + 4.0)
        }
        DrawCmd::Text {
            text: t,
            rect,
            style,
            ..
        } => {
            // The lines can be wider or taller than the box (a long word,
            // more lines than fit), on either side of it as aligned, and
            // glyphs reach past their line box: overhangs, accents.
            let size = text.measure(t, style, Some(rect.width));
            let over_x = (size.width - rect.width).max(0.0);
            let over_y = (size.height - rect.height).max(0.0);
            let box_ = astryx_core::Rect::new(
                rect.x - over_x,
                rect.y,
                rect.width + 2.0 * over_x,
                rect.height + over_y,
            );
            grow(rect_px(&box_, s, false), style.size.max(0.0) * s + 2.0)
        }
        DrawCmd::Line {
            from, to, width, ..
        } => match line_rect(*from, *to, *width, s, snap) {
            Some(r) => r,
            None => grow(
                PxRect {
                    x0: from.x.min(to.x) * s,
                    y0: from.y.min(to.y) * s,
                    x1: from.x.max(to.x) * s,
                    y1: from.y.max(to.y) * s,
                },
                width.max(0.0) * s,
            ),
        },
        DrawCmd::Path {
            commands, stroke, ..
        } => {
            let mut b: Option<PxRect> = None;
            let mut add = |p: astryx_core::Point| {
                let q = PxRect {
                    x0: p.x * s,
                    y0: p.y * s,
                    x1: p.x * s,
                    y1: p.y * s,
                };
                b = union(b, Some(q));
            };
            for c in commands {
                match *c {
                    PathCmd::MoveTo(a) | PathCmd::LineTo(a) => add(a),
                    PathCmd::QuadTo(a, b) => {
                        add(a);
                        add(b);
                    }
                    PathCmd::CubicTo(a, b, c) => {
                        add(a);
                        add(b);
                        add(c);
                    }
                    PathCmd::Close => {}
                }
            }
            let width = stroke.map_or(0.0, |(_, w)| w.max(0.0) * s);
            grow(b?, width)
        }
        // Lights and emissive rects belong to a lighting pass, which
        // redraws the whole frame.
        DrawCmd::Light(_) | DrawCmd::Emissive { .. } => return None,
        _ => return None,
    };
    Some(grow(r, 2.0))
}

/// Bounds for every command of `list`, following its groups the way the
/// renderer does: a pop closes groups up to the nearest of its kind.
/// Rows where `cmd`, already moved, draws a path that is not just vertical
/// edges, given its ink bounds `b`: the top and bottom of a rect with its
/// rounded corners and border, or all of a path or an image.
fn no_seam(cmd: &DrawCmd, s: f32, snap: bool, b: Option<PxRect>, out: &mut Vec<(f32, f32)>) {
    // Rows above and below a rect's corners, borders and antialiased edges.
    let mut rect_rows = |rect: &astryx_core::Rect, radii: &astryx_core::Radii, border: f32| {
        let r = rect_px(rect, s, snap);
        let fit = fit_radii(radii, s, &r);
        let reach = fit.iter().fold(0.0f32, |a, &v| a.max(v)) + border * s + 2.0;
        out.push((r.y0 - 2.0, r.y0 + reach));
        out.push((r.y1 - reach, r.y1 + 2.0));
    };
    match cmd {
        DrawCmd::Rect {
            rect,
            radii,
            stroke,
            ..
        } => rect_rows(rect, radii, stroke.map_or(0.0, |(_, w)| w)),
        DrawCmd::PushClip { rect, radii } => rect_rows(rect, radii, 0.0),
        DrawCmd::Emissive { rect, .. } => rect_rows(rect, &astryx_core::Radii::default(), 0.0),
        // Glyphs and shadows are blended from masks at whole pixels.
        DrawCmd::Text { .. } | DrawCmd::Shadow { .. } => {}
        _ => {
            if let Some(b) = b {
                out.push((b.y0, b.y1));
            }
        }
    }
}

pub(crate) fn walk(list: &DisplayList, s: f32, text: &dyn TextMeasure) -> Walk {
    let mut bounds: Vec<Option<PxRect>> = vec![None; list.commands.len()];
    let mut lighting = false;
    let mut seams_off: Vec<(f32, f32)> = Vec::new();
    // Open groups: kind, index of the push, bounds so far, the transform
    // around the group.
    let mut open: Vec<(Kind, usize, Option<PxRect>, Affine)> = Vec::new();
    let mut t = Affine::IDENTITY;
    let close = |open: &mut Vec<(Kind, usize, Option<PxRect>, Affine)>,
                 bounds: &mut Vec<Option<PxRect>>,
                 t: &mut Affine,
                 end: Option<usize>| {
        let (kind, start, b, around) = open.pop().expect("open group");
        bounds[start] = b;
        if let Some(end) = end {
            bounds[end] = b;
        }
        if kind == Kind::Transform {
            *t = around;
        }
        if let Some(parent) = open.last_mut() {
            parent.2 = union(parent.2, b);
        }
        kind
    };
    for (i, cmd) in list.commands.iter().enumerate() {
        match kind_of(cmd) {
            Some((kind, true)) => {
                if kind == Kind::Lighting {
                    lighting = true;
                }
                if let DrawCmd::PushClip { .. } = cmd {
                    if t.is_identity() {
                        no_seam(cmd, s, true, None, &mut seams_off);
                    } else if let (DrawCmd::PushClip { rect, .. }, snaps) =
                        (t.apply(cmd), t.snaps())
                    {
                        if snaps {
                            no_seam(&t.apply(cmd), s, true, None, &mut seams_off);
                        } else {
                            let r = rect_px(&rect, s, false);
                            seams_off.push((r.y0 - 2.0, r.y1 + 2.0));
                        }
                    }
                }
                open.push((kind, i, None, t));
                if let DrawCmd::PushTransform {
                    origin,
                    scale,
                    translate,
                } = cmd
                {
                    t = t.then(*origin, *scale, *translate);
                }
            }
            Some((kind, false)) => {
                // Groups opened inside it and never closed close with it.
                while let Some(&(k, ..)) = open.last() {
                    let last = k == kind;
                    close(&mut open, &mut bounds, &mut t, last.then_some(i));
                    if last {
                        break;
                    }
                }
            }
            None => {
                if matches!(cmd, DrawCmd::Light(_) | DrawCmd::Emissive { .. }) {
                    lighting = true;
                }
                let b = if t.is_identity() {
                    let b = ink(cmd, s, true, text);
                    no_seam(cmd, s, true, b, &mut seams_off);
                    b
                } else {
                    let moved = t.apply(cmd);
                    let b = ink(&moved, s, t.snaps(), text);
                    if t.snaps() {
                        no_seam(&moved, s, true, b, &mut seams_off);
                    } else if let Some(b) = b {
                        // Scaled: anything may be drawn through a transform.
                        if !matches!(cmd, DrawCmd::Text { .. }) {
                            seams_off.push((b.y0, b.y1));
                        }
                    }
                    b
                };
                bounds[i] = b;
                if let Some(g) = open.last_mut() {
                    g.2 = union(g.2, b);
                }
            }
        }
    }
    while !open.is_empty() {
        close(&mut open, &mut bounds, &mut t, None);
    }
    Walk {
        bounds,
        lighting,
        no_seam: seams_off,
    }
}

/// How far ahead [`changed`] looks to line the lists up again after a
/// difference.
const LOOKAHEAD: usize = 48;

/// The bounds of every command that differs between the two lists, taken
/// from the list it is in. Equal commands draw the same pixels: a changed
/// group around them is itself a changed command whose bounds cover them. Commands that match in order cost nothing; an
/// inserted or removed run is found within [`LOOKAHEAD`] commands, and a
/// longer one counts as changed on both sides.
pub(crate) fn changed(
    old: &[DrawCmd],
    old_b: &[Option<PxRect>],
    new: &[DrawCmd],
    new_b: &[Option<PxRect>],
) -> Vec<PxRect> {
    let mut out: Vec<PxRect> = Vec::new();
    let mut add = |b: Option<PxRect>| {
        if let Some(b) = b {
            out.push(b);
        }
    };
    let (mut i, mut j) = (0, 0);
    while i < old.len() && j < new.len() {
        if old[i] == new[j] {
            i += 1;
            j += 1;
            continue;
        }
        // An insertion in the new list, or a removal from the old one,
        // whichever lines up sooner; otherwise both commands changed.
        let inserted = (1..=LOOKAHEAD)
            .take_while(|k| j + k < new.len())
            .find(|&k| old[i] == new[j + k]);
        let removed = (1..=LOOKAHEAD)
            .take_while(|k| i + k < old.len())
            .find(|&k| old[i + k] == new[j]);
        match (inserted, removed) {
            (Some(a), Some(b)) if a <= b => {
                (j..j + a).for_each(|k| add(new_b[k]));
                j += a;
            }
            (Some(a), None) => {
                (j..j + a).for_each(|k| add(new_b[k]));
                j += a;
            }
            (_, Some(b)) => {
                (i..i + b).for_each(|k| add(old_b[k]));
                i += b;
            }
            (None, None) => {
                add(old_b[i]);
                add(new_b[j]);
                i += 1;
                j += 1;
            }
        }
    }
    (i..old.len()).for_each(|k| add(old_b[k]));
    (j..new.len()).for_each(|k| add(new_b[k]));
    out
}

/// Whole-pixel rects to redraw for `damage`, inside a `w` x `h` frame:
/// overlapping or nearby rects merge, and many become their union.
pub(crate) fn regions(damage: &[PxRect], w: u32, h: u32) -> Vec<(u32, u32, u32, u32)> {
    let clamp = |r: &PxRect| {
        let x0 = r.x0.floor().max(0.0) as u32;
        let y0 = r.y0.floor().max(0.0) as u32;
        let x1 = (r.x1.ceil().max(0.0) as u32).min(w);
        let y1 = (r.y1.ceil().max(0.0) as u32).min(h);
        (x0 < x1 && y0 < y1).then_some((x0, y0, x1, y1))
    };
    let mut rects: Vec<(u32, u32, u32, u32)> = damage.iter().filter_map(clamp).collect();
    // Merge rects that overlap or nearly touch, until none do.
    const NEAR: u32 = 16;
    let mut merged = true;
    while merged {
        merged = false;
        'outer: for a in 0..rects.len() {
            for b in a + 1..rects.len() {
                let (p, q) = (rects[a], rects[b]);
                let near = p.0 <= q.2 + NEAR
                    && q.0 <= p.2 + NEAR
                    && p.1 <= q.3 + NEAR
                    && q.1 <= p.3 + NEAR;
                if near {
                    rects[a] = (p.0.min(q.0), p.1.min(q.1), p.2.max(q.2), p.3.max(q.3));
                    rects.swap_remove(b);
                    merged = true;
                    break 'outer;
                }
            }
        }
    }
    const MOST: usize = 8;
    if rects.len() > MOST {
        let all = rects.iter().fold(rects[0], |u, r| {
            (u.0.min(r.0), u.1.min(r.1), u.2.max(r.2), u.3.max(r.3))
        });
        rects = vec![all];
    }
    rects
}
