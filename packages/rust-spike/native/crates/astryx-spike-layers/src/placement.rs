//! Anchored placement: where a layer goes beside its anchor, as Astryx's
//! layer runtime places it with CSS anchor positioning.
//!
//! Placement is logical: `Start` and `End` follow the writing direction.
//! When the preferred spot overflows the viewport (inset by the gutter), it
//! tries, in order, the opposite side, the opposite alignment, both, and
//! then slides along the alignment axis; the layer never covers its anchor
//! on the placement axis while there is room on either side.

use astryx_core::layer::{Place, PlaceInput};
use astryx_core::{Point, Rect};
use std::rc::Rc;

/// The side of the anchor a layer sits on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Side {
    Above,
    #[default]
    Below,
    /// Inline start: left in left-to-right, right in right-to-left.
    Start,
    End,
}

impl Side {
    fn flipped(self) -> Side {
        match self {
            Side::Above => Side::Below,
            Side::Below => Side::Above,
            Side::Start => Side::End,
            Side::End => Side::Start,
        }
    }
    fn vertical(self) -> bool {
        matches!(self, Side::Above | Side::Below)
    }
}

/// Where along the anchor's edge the layer lines up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
}

impl Align {
    fn flipped(self) -> Align {
        match self {
            Align::Start => Align::End,
            Align::End => Align::Start,
            Align::Center => Align::Center,
        }
    }
}

/// How a layer is placed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub side: Side,
    pub align: Align,
    /// Clearance from the anchor along the placement axis, in points.
    pub offset: f32,
    /// Distance kept from every viewport edge (Astryx: the spacing-4 step).
    pub gutter: f32,
}

impl Default for Placement {
    fn default() -> Self {
        Placement {
            side: Side::Below,
            align: Align::Start,
            offset: 0.0,
            gutter: 16.0,
        }
    }
}

impl Placement {
    pub fn new(side: Side, align: Align) -> Self {
        Placement {
            side,
            align,
            ..Self::default()
        }
    }

    pub fn offset(mut self, offset: f32) -> Self {
        self.offset = offset;
        self
    }

    pub fn gutter(mut self, gutter: f32) -> Self {
        self.gutter = gutter;
        self
    }

    /// This placement as a layer's `place` function.
    pub fn place(self) -> Place {
        Rc::new(move |input: &PlaceInput| self.resolve(input))
    }

    /// The layer's top-left corner for `input`.
    pub fn resolve(&self, input: &PlaceInput) -> Point {
        let (w, h) = (input.size.width, input.size.height);
        let area = Rect::new(
            input.viewport.x + self.gutter,
            input.viewport.y + self.gutter,
            (input.viewport.width - 2.0 * self.gutter).max(0.0),
            (input.viewport.height - 2.0 * self.gutter).max(0.0),
        );
        let Some(anchor) = input.anchor else {
            return Point {
                x: area.x + (area.width - w).max(0.0) / 2.0,
                y: area.y + (area.height - h).max(0.0) / 2.0,
            };
        };
        let tries = [
            (self.side, self.align),
            (self.side.flipped(), self.align),
            (self.side, self.align.flipped()),
            (self.side.flipped(), self.align.flipped()),
        ];
        for (side, align) in tries {
            let p = self.at(anchor, w, h, side, align, input.rtl);
            if fits(area, p, w, h) {
                return p;
            }
        }
        // Slide: keep the side with more room and move along the alignment
        // axis into the viewport.
        let side = self.roomier_side(anchor, area);
        let p = self.at(anchor, w, h, side, self.align, input.rtl);
        Point {
            x: clamp(p.x, area.x, area.x + area.width - w),
            y: clamp(p.y, area.y, area.y + area.height - h),
        }
    }

    fn roomier_side(&self, anchor: Rect, area: Rect) -> Side {
        if !self.side.vertical() {
            return self.side;
        }
        let above = anchor.y - area.y;
        let below = area.y + area.height - (anchor.y + anchor.height);
        match (self.side, above > below) {
            (Side::Below, true) => Side::Above,
            (Side::Above, false) => Side::Below,
            (side, _) => side,
        }
    }

    /// The corner for `side` and `align`, with start and end mirrored in
    /// right-to-left.
    fn at(&self, a: Rect, w: f32, h: f32, side: Side, align: Align, rtl: bool) -> Point {
        let physical_left = |s: Side| matches!((s, rtl), (Side::Start, false) | (Side::End, true));
        let along_x = |align: Align| {
            // Inline alignment follows the writing direction.
            let (start, end) = if rtl {
                (a.x + a.width - w, a.x)
            } else {
                (a.x, a.x + a.width - w)
            };
            match align {
                Align::Start => start,
                Align::Center => a.x + (a.width - w) / 2.0,
                Align::End => end,
            }
        };
        let along_y = |align: Align| match align {
            Align::Start => a.y,
            Align::Center => a.y + (a.height - h) / 2.0,
            Align::End => a.y + a.height - h,
        };
        match side {
            Side::Above => Point {
                x: along_x(align),
                y: a.y - self.offset - h,
            },
            Side::Below => Point {
                x: along_x(align),
                y: a.y + a.height + self.offset,
            },
            s => Point {
                x: if physical_left(s) {
                    a.x - self.offset - w
                } else {
                    a.x + a.width + self.offset
                },
                y: along_y(align),
            },
        }
    }
}

fn fits(area: Rect, p: Point, w: f32, h: f32) -> bool {
    const EPS: f32 = 0.5;
    p.x >= area.x - EPS
        && p.y >= area.y - EPS
        && p.x + w <= area.x + area.width + EPS
        && p.y + h <= area.y + area.height + EPS
}

fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    if hi < lo {
        lo
    } else {
        v.clamp(lo, hi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astryx_core::Size;

    fn input(anchor: Rect, w: f32, h: f32, rtl: bool) -> PlaceInput {
        PlaceInput {
            anchor: Some(anchor),
            size: Size {
                width: w,
                height: h,
            },
            viewport: Rect::new(0.0, 0.0, 800.0, 600.0),
            rtl,
        }
    }

    #[test]
    fn below_start_with_offset() {
        let p = Placement::new(Side::Below, Align::Start).offset(4.0);
        let at = p.resolve(&input(
            Rect::new(100.0, 100.0, 80.0, 32.0),
            200.0,
            120.0,
            false,
        ));
        assert_eq!(at, Point { x: 100.0, y: 136.0 });
    }

    #[test]
    fn start_and_alignment_mirror_in_rtl() {
        let p = Placement::new(Side::Below, Align::Start);
        let at = p.resolve(&input(
            Rect::new(300.0, 100.0, 80.0, 32.0),
            200.0,
            120.0,
            true,
        ));
        assert_eq!(at.x, 180.0, "end edges line up in right-to-left");
        let side = Placement::new(Side::Start, Align::Center);
        let ltr = side.resolve(&input(
            Rect::new(300.0, 100.0, 80.0, 32.0),
            100.0,
            32.0,
            false,
        ));
        let rtl = side.resolve(&input(
            Rect::new(300.0, 100.0, 80.0, 32.0),
            100.0,
            32.0,
            true,
        ));
        assert_eq!((ltr.x, rtl.x), (200.0, 380.0));
    }

    #[test]
    fn flips_to_the_other_side_when_there_is_no_room() {
        let p = Placement::new(Side::Below, Align::Start).offset(4.0);
        let at = p.resolve(&input(
            Rect::new(100.0, 520.0, 80.0, 32.0),
            200.0,
            120.0,
            false,
        ));
        assert_eq!(at.y, 520.0 - 4.0 - 120.0, "above, keeping its clearance");
        let p = Placement::new(Side::Below, Align::Start);
        let at = p.resolve(&input(
            Rect::new(700.0, 100.0, 80.0, 32.0),
            200.0,
            120.0,
            false,
        ));
        assert_eq!(at.x, 780.0 - 200.0, "end-aligned near the right edge");
    }

    #[test]
    fn slides_inside_the_gutter_when_nothing_fits() {
        let p = Placement::new(Side::Below, Align::Center);
        let wide = input(Rect::new(10.0, 100.0, 40.0, 32.0), 700.0, 120.0, false);
        let at = p.resolve(&wide);
        assert_eq!(at.x, 16.0, "kept off the edge by the gutter");
        assert_eq!(at.y, 132.0, "still below its anchor");
    }

    #[test]
    fn no_anchor_centers_in_the_viewport() {
        let p = Placement::default();
        let mut i = input(Rect::default(), 400.0, 200.0, false);
        i.anchor = None;
        assert_eq!(p.resolve(&i), Point { x: 200.0, y: 200.0 });
    }

    #[test]
    fn a_point_anchor_places_at_the_pointer() {
        let p = Placement::new(Side::Below, Align::Start);
        let at = p.resolve(&input(
            Rect::new(250.0, 300.0, 0.0, 0.0),
            160.0,
            200.0,
            false,
        ));
        assert_eq!(at, Point { x: 250.0, y: 300.0 });
        let near_corner = p.resolve(&input(
            Rect::new(760.0, 560.0, 0.0, 0.0),
            160.0,
            200.0,
            false,
        ));
        assert_eq!(
            near_corner,
            Point { x: 600.0, y: 360.0 },
            "flips up and to the start"
        );
    }
}
