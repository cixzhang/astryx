//! Geometry in physical pixels that every renderer shares, so edges land on
//! the same pixels whichever backend draws them.

use astryx_core::{DrawCmd, PathCmd, Point, Radii, Rect, ShadowLayer};

/// One path segment in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    Move(f32, f32),
    Line(f32, f32),
    Quad(f32, f32, f32, f32),
    Cubic(f32, f32, f32, f32, f32, f32),
    Close,
}

/// An axis-aligned rectangle in physical pixels, stored by its edges.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PxRect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl PxRect {
    pub fn width(&self) -> f32 {
        self.x1 - self.x0
    }

    pub fn height(&self) -> f32 {
        self.y1 - self.y0
    }

    pub fn is_empty(&self) -> bool {
        self.width() <= 0.0 || self.height() <= 0.0
    }

    pub fn offset(&self, dx: f32, dy: f32) -> Self {
        Self {
            x0: self.x0 + dx,
            y0: self.y0 + dy,
            x1: self.x1 + dx,
            y1: self.y1 + dy,
        }
    }

    /// Grows each edge by `by`; a negative value shrinks it, never past the center.
    pub fn outset(&self, by: f32) -> Self {
        let cx = (self.x0 + self.x1) / 2.0;
        let cy = (self.y0 + self.y1) / 2.0;
        Self {
            x0: (self.x0 - by).min(cx),
            y0: (self.y0 - by).min(cy),
            x1: (self.x1 + by).max(cx),
            y1: (self.y1 + by).max(cy),
        }
    }
}

/// Maps a logical rectangle to physical pixels with every edge on the pixel
/// grid, as browsers snap layout boxes.
pub fn snap_rect(rect: &Rect, scale: f32) -> PxRect {
    PxRect {
        x0: (rect.x * scale).round(),
        y0: (rect.y * scale).round(),
        x1: ((rect.x + rect.width) * scale).round(),
        y1: ((rect.y + rect.height) * scale).round(),
    }
}

/// Maps a logical rectangle to physical pixels, on the pixel grid when
/// `snap` is set.
pub fn rect_px(rect: &Rect, scale: f32, snap: bool) -> PxRect {
    if snap {
        snap_rect(rect, scale)
    } else {
        PxRect {
            x0: rect.x * scale,
            y0: rect.y * scale,
            x1: (rect.x + rect.width) * scale,
            y1: (rect.y + rect.height) * scale,
        }
    }
}

/// A border or line width in physical pixels, whole pixels when `snap` is set.
pub fn width_px(width: f32, scale: f32, snap: bool) -> f32 {
    if snap {
        snap_width(width, scale)
    } else {
        (width * scale).max(0.0)
    }
}

/// An axis-aligned line as a filled rectangle, snapped to whole pixels when
/// `snap` is set. Returns `None` for diagonal lines.
pub fn line_rect(from: Point, to: Point, width: f32, scale: f32, snap: bool) -> Option<PxRect> {
    if snap {
        return snap_line(from, to, width, scale);
    }
    let w = width * scale;
    if w <= 0.0 {
        return None;
    }
    let (a, b) = ((from.x.min(to.x)) * scale, (from.x.max(to.x)) * scale);
    let (c, d) = ((from.y.min(to.y)) * scale, (from.y.max(to.y)) * scale);
    if from.y == to.y {
        Some(PxRect {
            x0: a,
            y0: c - w / 2.0,
            x1: b,
            y1: c + w / 2.0,
        })
    } else if from.x == to.x {
        Some(PxRect {
            x0: a - w / 2.0,
            y0: c,
            x1: a + w / 2.0,
            y1: d,
        })
    } else {
        None
    }
}

/// A uniform scale followed by a translation, in logical points:
/// `p' = p * scale + offset`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    pub scale: f32,
    pub offset: Point,
}

impl Affine {
    pub const IDENTITY: Affine = Affine {
        scale: 1.0,
        offset: Point { x: 0.0, y: 0.0 },
    };

    /// This transform applied after a `PushTransform` of `origin`, `scale`,
    /// and `translate`.
    pub fn then(self, origin: Point, scale: f32, translate: Point) -> Affine {
        let inner = Point {
            x: origin.x * (1.0 - scale) + translate.x,
            y: origin.y * (1.0 - scale) + translate.y,
        };
        Affine {
            scale: self.scale * scale,
            offset: self.point(inner),
        }
    }

    pub fn is_identity(&self) -> bool {
        *self == Affine::IDENTITY
    }

    /// Pixel snapping holds while nothing is scaled.
    pub fn snaps(&self) -> bool {
        self.scale == 1.0
    }

    pub fn point(&self, p: Point) -> Point {
        Point {
            x: p.x * self.scale + self.offset.x,
            y: p.y * self.scale + self.offset.y,
        }
    }

    pub fn rect(&self, r: &Rect) -> Rect {
        let p = self.point(Point { x: r.x, y: r.y });
        Rect::new(p.x, p.y, r.width * self.scale, r.height * self.scale)
    }

    /// The command with its geometry moved. Shadow layers scale with their box.
    pub fn apply(&self, cmd: &DrawCmd) -> DrawCmd {
        let k = self.scale;
        let radii = |r: &Radii| Radii(r.0.map(|v| v * k));
        match cmd {
            DrawCmd::Rect {
                rect,
                radii: r,
                fill,
                stroke,
            } => DrawCmd::Rect {
                rect: self.rect(rect),
                radii: radii(r),
                fill: *fill,
                stroke: stroke.map(|(c, w)| (c, w * k)),
            },
            DrawCmd::Shadow {
                rect,
                radii: r,
                layers,
            } => DrawCmd::Shadow {
                rect: self.rect(rect),
                radii: radii(r),
                layers: layers
                    .iter()
                    .map(|l| ShadowLayer {
                        x: l.x * k,
                        y: l.y * k,
                        blur: l.blur * k,
                        spread: l.spread * k,
                        ..*l
                    })
                    .collect(),
            },
            DrawCmd::Text {
                rect,
                text,
                style,
                color,
                align,
            } => DrawCmd::Text {
                rect: self.rect(rect),
                text: text.clone(),
                style: astryx_core::TextStyle {
                    direction: None,
                    size: style.size * k,
                    ..*style
                },
                color: *color,
                align: *align,
            },
            DrawCmd::Line {
                from,
                to,
                color,
                width,
            } => DrawCmd::Line {
                from: self.point(*from),
                to: self.point(*to),
                color: *color,
                width: width * k,
            },
            DrawCmd::Path {
                commands,
                fill,
                stroke,
            } => DrawCmd::Path {
                commands: commands
                    .iter()
                    .map(|c| match *c {
                        PathCmd::MoveTo(a) => PathCmd::MoveTo(self.point(a)),
                        PathCmd::LineTo(a) => PathCmd::LineTo(self.point(a)),
                        PathCmd::QuadTo(a, b) => PathCmd::QuadTo(self.point(a), self.point(b)),
                        PathCmd::CubicTo(a, b, c) => {
                            PathCmd::CubicTo(self.point(a), self.point(b), self.point(c))
                        }
                        PathCmd::Close => PathCmd::Close,
                    })
                    .collect(),
                fill: *fill,
                stroke: stroke.map(|(c, w)| (c, w * k)),
            },
            DrawCmd::PushClip { rect, radii: r } => DrawCmd::PushClip {
                rect: self.rect(rect),
                radii: radii(r),
            },
            DrawCmd::Image {
                rect,
                radii: r,
                image,
                fit,
            } => DrawCmd::Image {
                rect: self.rect(rect),
                radii: radii(r),
                image: *image,
                fit: *fit,
            },
            DrawCmd::Light(light) => DrawCmd::Light(astryx_core::Light {
                center: self.point(light.center),
                radius_x: light.radius_x * k,
                radius_y: light.radius_y * k,
                ..*light
            }),
            DrawCmd::Emissive { rect, color } => DrawCmd::Emissive {
                rect: self.rect(rect),
                color: *color,
            },
            other => other.clone(),
        }
    }
}

/// A border or line width in whole physical pixels, at least one pixel when
/// the logical width is positive.
pub fn snap_width(width: f32, scale: f32) -> f32 {
    if width <= 0.0 {
        0.0
    } else {
        (width * scale).floor().max(1.0)
    }
}

/// Scales corner radii to physical pixels and shrinks them as CSS does when
/// adjacent radii would overlap.
pub fn fit_radii(radii: &Radii, scale: f32, rect: &PxRect) -> [f32; 4] {
    let mut r = radii.0.map(|r| (r * scale).max(0.0));
    let (w, h) = (rect.width().max(0.0), rect.height().max(0.0));
    let mut f: f32 = 1.0;
    for (sum, side) in [
        (r[0] + r[1], w),
        (r[3] + r[2], w),
        (r[0] + r[3], h),
        (r[1] + r[2], h),
    ] {
        if sum > side && sum > 0.0 {
            f = f.min(side / sum);
        }
    }
    if f < 1.0 {
        for v in &mut r {
            *v *= f;
        }
    }
    r
}

/// A corner radius after the box grows by `delta`, following the CSS rule
/// for `box-shadow` spread.
pub fn spread_radius(r: f32, delta: f32) -> f32 {
    if r <= 0.0 {
        0.0
    } else if delta >= 0.0 {
        let ratio = r / delta.max(f32::MIN_POSITIVE);
        if ratio < 1.0 {
            r + delta * (1.0 + (ratio - 1.0).powi(3))
        } else {
            r + delta
        }
    } else {
        (r + delta).max(0.0)
    }
}

// Control point distance for a quarter circle drawn as one cubic.
const KAPPA: f32 = 0.552_284_8;

/// Appends a closed, clockwise rounded rectangle. Radii are top-left,
/// top-right, bottom-right, bottom-left and must already fit the rectangle.
pub fn rounded_rect(rect: &PxRect, radii: [f32; 4], out: &mut Vec<Seg>) {
    let PxRect { x0, y0, x1, y1 } = *rect;
    let [tl, tr, br, bl] = radii;
    let k = 1.0 - KAPPA;
    out.push(Seg::Move(x0 + tl, y0));
    out.push(Seg::Line(x1 - tr, y0));
    if tr > 0.0 {
        out.push(Seg::Cubic(x1 - tr * k, y0, x1, y0 + tr * k, x1, y0 + tr));
    }
    out.push(Seg::Line(x1, y1 - br));
    if br > 0.0 {
        out.push(Seg::Cubic(x1, y1 - br * k, x1 - br * k, y1, x1 - br, y1));
    }
    out.push(Seg::Line(x0 + bl, y1));
    if bl > 0.0 {
        out.push(Seg::Cubic(x0 + bl * k, y1, x0, y1 - bl * k, x0, y1 - bl));
    }
    out.push(Seg::Line(x0, y0 + tl));
    if tl > 0.0 {
        out.push(Seg::Cubic(x0, y0 + tl * k, x0 + tl * k, y0, x0 + tl, y0));
    }
    out.push(Seg::Close);
}

/// The inside border ring of a rounded rectangle, filled with the even-odd
/// rule. Returns false when the border covers the whole box, in which case
/// only the outer outline is appended and a plain fill draws it.
pub fn border_ring(rect: &PxRect, radii: [f32; 4], width: f32, out: &mut Vec<Seg>) -> bool {
    rounded_rect(rect, radii, out);
    let inner = PxRect {
        x0: rect.x0 + width,
        y0: rect.y0 + width,
        x1: rect.x1 - width,
        y1: rect.y1 - width,
    };
    if inner.is_empty() {
        return false;
    }
    rounded_rect(&inner, radii.map(|r| (r - width).max(0.0)), out);
    true
}

/// Converts a logical icon path to physical pixels.
pub fn path_segs(commands: &[PathCmd], scale: f32, out: &mut Vec<Seg>) {
    let p = |pt: &Point| (pt.x * scale, pt.y * scale);
    for cmd in commands {
        out.push(match cmd {
            PathCmd::MoveTo(a) => {
                let (x, y) = p(a);
                Seg::Move(x, y)
            }
            PathCmd::LineTo(a) => {
                let (x, y) = p(a);
                Seg::Line(x, y)
            }
            PathCmd::QuadTo(c, a) => {
                let ((cx, cy), (x, y)) = (p(c), p(a));
                Seg::Quad(cx, cy, x, y)
            }
            PathCmd::CubicTo(c1, c2, a) => {
                let ((ax, ay), (bx, by), (x, y)) = (p(c1), p(c2), p(a));
                Seg::Cubic(ax, ay, bx, by, x, y)
            }
            PathCmd::Close => Seg::Close,
        });
    }
}

/// An axis-aligned line as a filled, pixel-snapped rectangle, so one pixel
/// lines stay crisp. Returns `None` for diagonal lines.
pub fn snap_line(from: Point, to: Point, width: f32, scale: f32) -> Option<PxRect> {
    let w = snap_width(width, scale);
    if w <= 0.0 {
        return None;
    }
    if from.y == to.y {
        let y0 = (from.y * scale - w / 2.0).round();
        let (a, b) = (from.x.min(to.x), from.x.max(to.x));
        Some(PxRect {
            x0: (a * scale).round(),
            y0,
            x1: (b * scale).round(),
            y1: y0 + w,
        })
    } else if from.x == to.x {
        let x0 = (from.x * scale - w / 2.0).round();
        let (a, b) = (from.y.min(to.y), from.y.max(to.y));
        Some(PxRect {
            x0,
            y0: (a * scale).round(),
            x1: x0 + w,
            y1: (b * scale).round(),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radii_shrink_when_they_overlap() {
        let r = PxRect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 20.0,
        };
        assert_eq!(fit_radii(&Radii::all(999.0), 1.0, &r), [10.0; 4]);
        assert_eq!(fit_radii(&Radii::all(4.0), 2.0, &r), [8.0; 4]);
    }

    #[test]
    fn spread_follows_css() {
        assert_eq!(spread_radius(0.0, 10.0), 0.0);
        assert_eq!(spread_radius(20.0, 10.0), 30.0);
        assert!((spread_radius(5.0, 10.0) - 13.75).abs() < 1e-4);
        assert_eq!(spread_radius(5.0, -10.0), 0.0);
    }

    #[test]
    fn transforms_compose_like_nested_groups() {
        let p = |x, y| Point { x, y };
        // Scale 2 about (10, 10), then inside it move by (5, 0).
        let t = Affine::IDENTITY.then(p(10.0, 10.0), 2.0, p(0.0, 0.0)).then(
            p(0.0, 0.0),
            1.0,
            p(5.0, 0.0),
        );
        assert_eq!(t.point(p(10.0, 10.0)), p(20.0, 10.0));
        assert_eq!(t.point(p(12.0, 10.0)), p(24.0, 10.0));
        assert!(!t.snaps());
        let moved = Affine::IDENTITY.then(p(0.0, 0.0), 1.0, p(3.5, 0.0));
        assert!(moved.snaps());
        assert_eq!(
            moved.rect(&Rect::new(1.0, 2.0, 3.0, 4.0)),
            Rect::new(4.5, 2.0, 3.0, 4.0)
        );
    }

    #[test]
    fn unsnapped_geometry_keeps_fractions() {
        let r = Rect::new(1.25, 2.0, 3.0, 4.0);
        assert_eq!(
            rect_px(&r, 2.0, false),
            PxRect {
                x0: 2.5,
                y0: 4.0,
                x1: 8.5,
                y1: 12.0
            }
        );
        assert_eq!(
            rect_px(&r, 2.0, true),
            PxRect {
                x0: 3.0,
                y0: 4.0,
                x1: 9.0,
                y1: 12.0
            }
        );
        assert_eq!(width_px(0.5, 1.0, false), 0.5);
        assert_eq!(width_px(0.5, 1.0, true), 1.0);
    }

    #[test]
    fn hairlines_snap_to_whole_pixels() {
        let a = Point { x: 0.0, y: 10.0 };
        let b = Point { x: 50.0, y: 10.0 };
        assert_eq!(
            snap_line(a, b, 1.0, 1.0),
            Some(PxRect {
                x0: 0.0,
                y0: 10.0,
                x1: 50.0,
                y1: 11.0
            })
        );
        assert_eq!(
            snap_line(a, b, 1.0, 2.0),
            Some(PxRect {
                x0: 0.0,
                y0: 19.0,
                x1: 100.0,
                y1: 21.0
            })
        );
    }
}
