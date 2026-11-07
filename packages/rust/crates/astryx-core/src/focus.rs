//! Focus movement: reading order for Next/Previous, geometry for the arrows.

use crate::contract::{Input, Rect};

/// Where focus goes from `current` (an index into `rects`, in reading
/// order), or `None` when it stays.
pub(crate) fn navigate(rects: &[Rect], current: Option<usize>, input: Input) -> Option<usize> {
    if rects.is_empty() {
        return None;
    }
    let Some(cur) = current else {
        return match input {
            Input::Previous => Some(rects.len() - 1),
            Input::Next | Input::Up | Input::Down | Input::Left | Input::Right => Some(0),
            _ => None,
        };
    };
    match input {
        // Reading order wraps, as Tab cycles inside an app window.
        Input::Next => Some((cur + 1) % rects.len()),
        Input::Previous => Some((cur + rects.len() - 1) % rects.len()),
        Input::Up | Input::Down | Input::Left | Input::Right => spatial(rects, cur, input),
        _ => None,
    }
}

/// The nearest focus stop in the direction of `input`.
///
/// A candidate must lie wholly past the current box's leading edge in that
/// direction. Among those, the score is the gap along the direction plus
/// twice the sideways gap, so a stop in the same row or column beats a
/// closer one off to the side. Ties go to the earlier stop in reading order.
fn spatial(rects: &[Rect], cur: usize, input: Input) -> Option<usize> {
    let from = rects[cur];
    let mut best: Option<(f32, usize)> = None;
    for (ix, to) in rects.iter().enumerate() {
        if ix == cur || (to.width <= 0.0 && to.height <= 0.0) {
            continue;
        }
        let Some((along, across)) = gaps(&from, to, input) else {
            continue;
        };
        let score = along + 2.0 * across;
        if best.is_none_or(|(s, _)| score < s - 0.01) {
            best = Some((score, ix));
        }
    }
    best.map(|(_, ix)| ix)
}

const EPS: f32 = 0.5;

/// The gap along the direction and the sideways gap from `a` to `b`, or
/// `None` when `b` is not in that direction.
fn gaps(a: &Rect, b: &Rect, input: Input) -> Option<(f32, f32)> {
    let (a_lo, a_hi, b_lo, b_hi, a_side, b_side) = match input {
        Input::Down | Input::Up => (
            a.y,
            a.y + a.height,
            b.y,
            b.y + b.height,
            (a.x, a.x + a.width),
            (b.x, b.x + b.width),
        ),
        _ => (
            a.x,
            a.x + a.width,
            b.x,
            b.x + b.width,
            (a.y, a.y + a.height),
            (b.y, b.y + b.height),
        ),
    };
    let along = match input {
        Input::Down | Input::Right if b_lo >= a_hi - EPS => b_lo - a_hi,
        Input::Up | Input::Left if b_hi <= a_lo + EPS => a_lo - b_hi,
        // Partly overlapping boxes count when the far edge moves on, as a
        // tall cell beside short ones does.
        Input::Down | Input::Right if b_lo > a_lo + EPS && b_hi > a_hi + EPS => 0.0,
        Input::Up | Input::Left if b_hi < a_hi - EPS && b_lo < a_lo - EPS => 0.0,
        _ => return None,
    };
    let across = if b_side.1 < a_side.0 {
        a_side.0 - b_side.1
    } else if b_side.0 > a_side.1 {
        b_side.0 - a_side.1
    } else {
        0.0
    };
    Some((along.max(0.0), across))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::new(x, y, w, h)
    }

    // A 3 x 2 grid of 100 x 40 cells with 10 point gaps.
    fn grid() -> Vec<Rect> {
        let mut out = Vec::new();
        for row in 0..2 {
            for col in 0..3 {
                out.push(r(col as f32 * 110.0, row as f32 * 50.0, 100.0, 40.0));
            }
        }
        out
    }

    #[test]
    fn reading_order_wraps_both_ways() {
        let g = grid();
        assert_eq!(navigate(&g, Some(0), Input::Next), Some(1));
        assert_eq!(navigate(&g, Some(5), Input::Next), Some(0));
        assert_eq!(navigate(&g, Some(0), Input::Previous), Some(5));
        assert_eq!(navigate(&g, None, Input::Next), Some(0));
        assert_eq!(navigate(&g, None, Input::Previous), Some(5));
        assert_eq!(navigate(&[], None, Input::Next), None);
    }

    #[test]
    fn arrows_move_through_a_grid_and_stop_at_edges() {
        let g = grid();
        assert_eq!(navigate(&g, Some(0), Input::Right), Some(1));
        assert_eq!(navigate(&g, Some(1), Input::Down), Some(4));
        assert_eq!(navigate(&g, Some(4), Input::Left), Some(3));
        assert_eq!(navigate(&g, Some(3), Input::Up), Some(0));
        assert_eq!(navigate(&g, Some(2), Input::Right), None);
        assert_eq!(navigate(&g, Some(0), Input::Up), None);
        assert_eq!(navigate(&g, Some(5), Input::Down), None);
    }

    #[test]
    fn same_column_beats_a_closer_diagonal() {
        // A wide button below, and a near one down and to the right.
        let rects = [
            r(0.0, 0.0, 100.0, 40.0),
            r(150.0, 45.0, 100.0, 40.0),
            r(0.0, 120.0, 100.0, 40.0),
        ];
        assert_eq!(navigate(&rects, Some(0), Input::Down), Some(2));
    }

    #[test]
    fn a_stack_of_rows_moves_one_row_at_a_time() {
        let rows: Vec<Rect> = (0..4)
            .map(|i| r(0.0, i as f32 * 44.0, 300.0, 44.0))
            .collect();
        assert_eq!(navigate(&rows, Some(1), Input::Down), Some(2));
        assert_eq!(navigate(&rows, Some(1), Input::Up), Some(0));
        assert_eq!(navigate(&rows, Some(1), Input::Left), None);
    }

    #[test]
    fn tall_neighbor_is_reachable_sideways() {
        // A tall card on the right spans both rows on the left.
        let rects = [
            r(0.0, 0.0, 100.0, 40.0),
            r(0.0, 50.0, 100.0, 40.0),
            r(120.0, 0.0, 100.0, 90.0),
        ];
        assert_eq!(navigate(&rects, Some(1), Input::Right), Some(2));
        assert_eq!(navigate(&rects, Some(2), Input::Left), Some(0));
    }

    #[test]
    fn other_inputs_do_not_move_focus() {
        let g = grid();
        for input in [Input::A, Input::B, Input::Start, Input::L1] {
            assert_eq!(navigate(&g, Some(0), input), None);
        }
    }
}
