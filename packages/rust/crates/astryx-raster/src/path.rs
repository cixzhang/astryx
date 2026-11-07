//! Coverage masks for icon paths, rasterized on the CPU, so renderers
//! without a path engine draw them as textured quads.

use swash::zeno::{Cap, Command, Fill, Format, Join, Mask, Point, Stroke};

use crate::geom::Seg;

/// How a path is painted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathStyle {
    /// Nonzero fill.
    Fill,
    /// Stroke of this width in pixels. Round caps and joins when `round`,
    /// else butt caps and miter joins.
    Stroke { width: f32, round: bool },
}

/// The cache identity of a path mask: the path moved to a whole-pixel
/// origin, so the same icon at another whole-pixel position reuses it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PathKey {
    bits: Vec<u32>,
}

/// A path ready to rasterize: its key and the whole-pixel origin it was
/// moved by.
pub struct PreparedPath {
    pub key: PathKey,
    pub origin: (i32, i32),
    commands: Vec<Command>,
    style: PathStyle,
}

/// Coverage of a path, positioned in frame pixels.
#[derive(Debug, PartialEq)]
pub struct PathMask {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// Moves `segs` (physical pixels) next to the origin and builds the key.
pub fn prepare(segs: &[Seg], style: PathStyle) -> Option<PreparedPath> {
    let (mut min_x, mut min_y) = (f32::INFINITY, f32::INFINITY);
    let mut see = |x: f32, y: f32| {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
    };
    for seg in segs {
        match *seg {
            Seg::Move(x, y) | Seg::Line(x, y) => see(x, y),
            Seg::Quad(a, b, x, y) => {
                see(a, b);
                see(x, y);
            }
            Seg::Cubic(a, b, c, d, x, y) => {
                see(a, b);
                see(c, d);
                see(x, y);
            }
            Seg::Close => {}
        }
    }
    if !min_x.is_finite() {
        return None;
    }
    let (ox, oy) = (min_x.floor(), min_y.floor());
    let p = |x: f32, y: f32| Point::new(x - ox, y - oy);
    let commands: Vec<Command> = segs
        .iter()
        .map(|seg| match *seg {
            Seg::Move(x, y) => Command::MoveTo(p(x, y)),
            Seg::Line(x, y) => Command::LineTo(p(x, y)),
            Seg::Quad(a, b, x, y) => Command::QuadTo(p(a, b), p(x, y)),
            Seg::Cubic(a, b, c, d, x, y) => Command::CurveTo(p(a, b), p(c, d), p(x, y)),
            Seg::Close => Command::Close,
        })
        .collect();
    let mut bits = Vec::with_capacity(commands.len() * 3 + 2);
    match style {
        PathStyle::Fill => bits.push(0),
        PathStyle::Stroke { width, round } => {
            bits.push(1 + round as u32);
            bits.push(width.to_bits());
        }
    }
    let zero = Point::new(0.0, 0.0);
    for c in &commands {
        let (verb, points, n) = match *c {
            Command::MoveTo(a) => (10, [a, zero, zero], 1),
            Command::LineTo(a) => (11, [a, zero, zero], 1),
            Command::QuadTo(a, b) => (12, [a, b, zero], 2),
            Command::CurveTo(a, b, d) => (13, [a, b, d], 3),
            Command::Close => (14, [zero; 3], 0),
        };
        bits.push(verb);
        for q in &points[..n] {
            bits.push(q.x.to_bits());
            bits.push(q.y.to_bits());
        }
    }
    Some(PreparedPath {
        key: PathKey { bits },
        origin: (ox as i32, oy as i32),
        commands,
        style,
    })
}

impl PreparedPath {
    /// Rasterizes the path. Returns `None` when it covers nothing.
    pub fn rasterize(&self) -> Option<PathMask> {
        let mut mask = Mask::new(&self.commands[..]);
        mask.format(Format::Alpha);
        match self.style {
            PathStyle::Fill => {
                mask.style(Fill::NonZero);
            }
            PathStyle::Stroke { width, round } => {
                let mut stroke = Stroke::new(width);
                if round {
                    stroke.join(Join::Round).cap(Cap::Round);
                } else {
                    stroke.join(Join::Miter).cap(Cap::Butt);
                }
                mask.style(stroke);
            }
        }
        let (data, placement) = mask.render();
        if placement.width == 0 || placement.height == 0 {
            return None;
        }
        Some(PathMask {
            x: self.origin.0 + placement.left,
            y: self.origin.1 + placement.top,
            width: placement.width,
            height: placement.height,
            data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f32, y: f32) -> Vec<Seg> {
        vec![
            Seg::Move(x, y),
            Seg::Line(x + 10.0, y),
            Seg::Line(x + 10.0, y + 10.0),
            Seg::Line(x, y + 10.0),
            Seg::Close,
        ]
    }

    #[test]
    fn fill_covers_the_shape_at_its_position() {
        let m = prepare(&square(20.0, 30.0), PathStyle::Fill)
            .unwrap()
            .rasterize()
            .unwrap();
        assert_eq!((m.x, m.y, m.width, m.height), (20, 30, 10, 10));
        assert!(m.data.iter().all(|&c| c == 255));
    }

    #[test]
    fn whole_pixel_moves_share_a_key() {
        let a = prepare(&square(20.0, 30.0), PathStyle::Fill).unwrap();
        let b = prepare(&square(25.0, 31.0), PathStyle::Fill).unwrap();
        let c = prepare(&square(25.5, 31.0), PathStyle::Fill).unwrap();
        assert_eq!(a.key, b.key);
        assert_ne!(a.key, c.key);
    }

    #[test]
    fn strokes_cover_the_outline() {
        let style = PathStyle::Stroke {
            width: 2.0,
            round: true,
        };
        let m = prepare(&square(20.0, 30.0), style)
            .unwrap()
            .rasterize()
            .unwrap();
        assert_eq!((m.x, m.y), (19, 29));
        let at = |x: i32, y: i32| m.data[((y - m.y) * m.width as i32 + (x - m.x)) as usize];
        assert_eq!(at(25, 35), 0);
        assert!(at(25, 30) > 200);
    }
}
