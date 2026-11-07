//! Pixel-grid geometry for themes with a `grid`: shapes become rows of
//! whole cells, drawn as square rectangles that renderers keep crisp.
//!
//! A rounded shape is sampled at cell centers, so its corners become the
//! staircase a pixel artist would draw: a corner radius of 2 cells takes
//! one step, 4 cells take two steps, and a full radius makes a pixel
//! circle or pill.

use crate::contract::{Color, DrawCmd, Radii, Rect};
use std::f32::consts::TAU;

/// `v` rounded to whole cells.
pub(crate) fn snap(v: f32, cell: f32) -> f32 {
    (v / cell).round() * cell
}

/// A length in whole cells, at least one cell when `v` is positive.
pub(crate) fn snap_width(v: f32, cell: f32) -> f32 {
    if v <= 0.0 {
        0.0
    } else {
        snap(v, cell).max(cell)
    }
}

/// `rect` with every edge on the grid.
pub(crate) fn snap_rect(rect: Rect, cell: f32) -> Rect {
    let (x0, y0) = (snap(rect.x, cell), snap(rect.y, cell));
    let (x1, y1) = (
        snap(rect.x + rect.width, cell),
        snap(rect.y + rect.height, cell),
    );
    Rect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
}

/// A run of filled cells per row, as absolute cell indexes: `row -> (first, last)`.
type Rows = Vec<(i32, Option<(i32, i32)>)>;

/// The filled cells of a snapped rounded rectangle, row by row.
fn rows(rect: Rect, radii: Radii, cell: f32) -> Rows {
    let max = (rect.width.min(rect.height) / 2.0).max(0.0);
    let [tl, tr, br, bl] = radii.0.map(|r| r.clamp(0.0, max));
    let row0 = (rect.y / cell).round() as i32;
    let col0 = (rect.x / cell).round() as i32;
    let n_rows = (rect.height / cell).round() as i32;
    let n_cols = (rect.width / cell).round() as i32;
    // How far a corner of radius `r` cuts in at height `dy` past its center.
    let cut = |r: f32, dy: f32| {
        if dy <= 0.0 || r <= 0.0 {
            0.0
        } else {
            r - (r * r - dy.min(r) * dy.min(r)).sqrt()
        }
    };
    (0..n_rows)
        .map(|i| {
            let yc = (i as f32 + 0.5) * cell;
            let from_bottom = rect.height - yc;
            let left = cut(tl, tl - yc).max(cut(bl, bl - from_bottom));
            let right = cut(tr, tr - yc).max(cut(br, br - from_bottom));
            // A cell is in when its center is inside the shape.
            let first = (left / cell - 0.5 - 1e-4).ceil() as i32;
            let last = n_cols - 1 - (right / cell - 0.5 - 1e-4).ceil() as i32;
            let span = (first <= last).then_some((col0 + first, col0 + last));
            (row0 + i, span)
        })
        .collect()
}

/// Runs of filled cells on one row: `(first, last)` column pairs.
type Runs = Vec<(i32, i32)>;
/// Rows that share their runs: first row, last row, and the runs.
type Band = (i32, i32, Runs);

/// Joins equal runs on consecutive rows into rectangles.
fn bands(rows: impl Iterator<Item = (i32, Runs)>, cell: f32) -> Vec<Rect> {
    let mut out = Vec::new();
    let mut open: Option<Band> = None;
    let mut flush = |open: &mut Option<Band>| {
        if let Some((start, end, runs)) = open.take() {
            for (a, b) in runs {
                out.push(Rect::new(
                    a as f32 * cell,
                    start as f32 * cell,
                    (b - a + 1) as f32 * cell,
                    (end - start + 1) as f32 * cell,
                ));
            }
        }
    };
    for (row, runs) in rows {
        match &mut open {
            Some((_, end, open_runs)) if *end + 1 == row && *open_runs == runs => *end = row,
            _ => {
                flush(&mut open);
                if !runs.is_empty() {
                    open = Some((row, row, runs));
                }
            }
        }
    }
    flush(&mut open);
    out
}

/// Square rectangles that cover the stepped shape of `rect` with `radii`.
pub(crate) fn fill(rect: Rect, radii: Radii, cell: f32) -> Vec<Rect> {
    let rect = snap_rect(rect, cell);
    bands(
        rows(rect, radii, cell)
            .into_iter()
            .map(|(row, span)| (row, span.into_iter().collect())),
        cell,
    )
}

/// Square rectangles that cover a stepped border `width` wide inside the
/// edge of `rect`. The inner edge steps too, so the frame keeps one width
/// all the way around its corners.
pub(crate) fn frame(rect: Rect, radii: Radii, width: f32, cell: f32) -> Vec<Rect> {
    let rect = snap_rect(rect, cell);
    let width = snap_width(width, cell);
    let inner = rect.inset(width);
    let inner_radii = Radii(radii.0.map(|r| (r - width).max(0.0)));
    let inner_rows: std::collections::HashMap<i32, (i32, i32)> = rows(inner, inner_radii, cell)
        .into_iter()
        .filter_map(|(row, span)| Some((row, span?)))
        .collect();
    let runs = rows(rect, radii, cell).into_iter().map(|(row, span)| {
        let runs = match (span, inner_rows.get(&row)) {
            (None, _) => Vec::new(),
            (Some(outer), None) => vec![outer],
            (Some((a, b)), Some(&(c, d))) => [(a, c - 1), (d + 1, b)]
                .into_iter()
                .filter(|(x, y)| x <= y)
                .collect(),
        };
        (row, runs)
    });
    bands(runs, cell)
}

/// Square rectangles in `color` as draw commands.
pub(crate) fn push_rects(out: &mut Vec<DrawCmd>, rects: Vec<Rect>, color: Color) {
    if color.a <= 0.0 {
        return;
    }
    out.extend(rects.into_iter().map(|rect| DrawCmd::Rect {
        rect,
        radii: Radii::default(),
        fill: color,
        stroke: None,
    }));
}

/// A bitmap drawn into `rect`, one cell per character; `#` is filled.
pub(crate) fn bitmap(rect: Rect, rows: &[&str], color: Color) -> Vec<DrawCmd> {
    let n_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if n_cols == 0 {
        return Vec::new();
    }
    let cell = rect.width.min(rect.height) / n_cols.max(rows.len()) as f32;
    let mut out = Vec::new();
    // Rows as runs, joined into bands by `bands` in cell units.
    let runs = rows.iter().enumerate().map(|(y, row)| {
        let mut runs = Vec::new();
        let mut start = None;
        for (x, ch) in row.chars().chain(std::iter::once('.')).enumerate() {
            match (ch == '#', start) {
                (true, None) => start = Some(x as i32),
                (false, Some(s)) => {
                    runs.push((s, x as i32 - 1));
                    start = None;
                }
                _ => {}
            }
        }
        (y as i32, runs)
    });
    let rects = bands(runs, 1.0).into_iter().map(|r| {
        Rect::new(
            rect.x + r.x * cell,
            rect.y + r.y * cell,
            r.width * cell,
            r.height * cell,
        )
    });
    push_rects(&mut out, rects.collect(), color);
    out
}

/// A pixel spinner: three quarters of a stepped ring that turns once a
/// second in eight steps.
pub(crate) fn spinner(rect: Rect, color: Color, time: f32, cell: f32) -> Vec<DrawCmd> {
    let side = snap(rect.width.min(rect.height), cell).max(cell);
    let n = (side / cell).round() as i32;
    let (x0, y0) = (
        snap(rect.x + (rect.width - side) / 2.0, cell),
        snap(rect.y + (rect.height - side) / 2.0, cell),
    );
    let radius = n as f32 / 2.0;
    let thickness = (n as f32 / 6.0).round().max(1.0);
    let start = (time * 8.0).floor() / 8.0 * TAU;
    let runs = (0..n).map(|j| {
        let mut runs: Vec<(i32, i32)> = Vec::new();
        for i in 0..n {
            let (dx, dy) = (i as f32 + 0.5 - radius, j as f32 + 0.5 - radius);
            let d = (dx * dx + dy * dy).sqrt();
            let along = (dy.atan2(dx) - start).rem_euclid(TAU);
            if d <= radius && d > radius - thickness && along < 0.75 * TAU {
                match runs.last_mut() {
                    Some(run) if run.1 + 1 == i => run.1 = i,
                    _ => runs.push((i, i)),
                }
            }
        }
        (j, runs)
    });
    let rects = bands(runs, 1.0)
        .into_iter()
        .map(|r| {
            Rect::new(
                x0 + r.x * cell,
                y0 + r.y * cell,
                r.width * cell,
                r.height * cell,
            )
        })
        .collect();
    let mut out = Vec::new();
    push_rects(&mut out, rects, color);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The filled cells of `rects` on an `n` by `m` grid, as text rows.
    fn picture(rects: &[Rect], cols: usize, rows: usize) -> Vec<String> {
        let mut grid = vec![vec!['.'; cols]; rows];
        for r in rects {
            let (x0, x1) = (r.x as usize, (r.x + r.width) as usize);
            for row in &mut grid[r.y as usize..(r.y + r.height) as usize] {
                for c in &mut row[x0..x1] {
                    assert_eq!(*c, '.', "rectangles overlap");
                    *c = '#';
                }
            }
        }
        grid.into_iter().map(|r| r.into_iter().collect()).collect()
    }

    #[test]
    fn corners_step_by_radius() {
        let one = fill(Rect::new(0.0, 0.0, 6.0, 4.0), Radii::all(2.0), 1.0);
        assert_eq!(
            picture(&one, 6, 4),
            [".####.", "######", "######", ".####."]
        );
        let two = fill(Rect::new(0.0, 0.0, 10.0, 8.0), Radii::all(4.0), 1.0);
        assert_eq!(
            picture(&two, 10, 8),
            [
                "..######..",
                ".########.",
                "##########",
                "##########",
                "##########",
                "##########",
                ".########.",
                "..######.."
            ]
        );
    }

    #[test]
    fn a_full_radius_is_a_pixel_circle() {
        let dot = fill(Rect::new(0.0, 0.0, 8.0, 8.0), Radii::all(9999.0), 1.0);
        assert_eq!(
            picture(&dot, 8, 8),
            [
                "..####..", ".######.", "########", "########", "########", "########", ".######.",
                "..####.."
            ]
        );
    }

    #[test]
    fn frames_keep_one_width_around_the_steps() {
        let ring = frame(Rect::new(0.0, 0.0, 10.0, 8.0), Radii::all(4.0), 1.0, 1.0);
        assert_eq!(
            picture(&ring, 10, 8),
            [
                "..######..",
                ".#......#.",
                "#........#",
                "#........#",
                "#........#",
                "#........#",
                ".#......#.",
                "..######.."
            ]
        );
    }

    #[test]
    fn rects_snap_to_cells_at_any_scale() {
        let r = snap_rect(Rect::new(2.9, 1.2, 7.0, 4.1), 2.0);
        assert_eq!(r, Rect::new(2.0, 2.0, 8.0, 4.0));
        assert_eq!(snap_width(0.5, 2.0), 2.0);
        assert_eq!(snap_width(0.0, 2.0), 0.0);
    }

    #[test]
    fn bitmaps_draw_runs_of_cells() {
        let cmds = bitmap(
            Rect::new(10.0, 0.0, 6.0, 6.0),
            &["#.#", "###", "..."],
            Color::rgba(0.0, 0.0, 0.0, 1.0),
        );
        let rects: Vec<Rect> = cmds
            .iter()
            .map(|c| match c {
                DrawCmd::Rect { rect, .. } => *rect,
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(
            rects,
            [
                Rect::new(10.0, 0.0, 2.0, 2.0),
                Rect::new(14.0, 0.0, 2.0, 2.0),
                Rect::new(10.0, 2.0, 6.0, 2.0)
            ]
        );
    }

    #[test]
    fn the_spinner_turns_in_eight_steps() {
        let rect = Rect::new(0.0, 0.0, 12.0, 12.0);
        let c = Color::rgba(0.0, 0.0, 0.0, 1.0);
        assert_eq!(spinner(rect, c, 0.01, 1.0), spinner(rect, c, 0.1, 1.0));
        assert_ne!(spinner(rect, c, 0.01, 1.0), spinner(rect, c, 0.2, 1.0));
    }
}
