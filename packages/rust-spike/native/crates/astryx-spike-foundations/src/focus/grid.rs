//! Two-dimensional focus over a grid of cells, as ARIA grids (Table) move
//! it: arrows by cell, Home and End along the row, Ctrl+Home and Ctrl+End
//! to the corners, Page Up and Page Down by a page of rows.

use super::Key;
use astryx_core::Direction;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridFocus {
    pub rows: usize,
    pub cols: usize,
    /// Rows Page Up and Page Down move.
    pub page_rows: usize,
    pub direction: Direction,
}

impl GridFocus {
    pub fn new(rows: usize, cols: usize) -> Self {
        GridFocus {
            rows,
            cols,
            page_rows: 10,
            direction: Direction::Ltr,
        }
    }

    pub fn page_rows(mut self, rows: usize) -> Self {
        self.page_rows = rows.max(1);
        self
    }

    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// The cell `(row, col)` focus moves to, or `None` at an edge. Grids do
    /// not wrap.
    pub fn next(&self, (row, col): (usize, usize), key: Key) -> Option<(usize, usize)> {
        if self.rows == 0 || self.cols == 0 {
            return None;
        }
        let last_row = self.rows - 1;
        let last_col = self.cols - 1;
        let (fwd, back) = if self.direction.is_rtl() {
            (Key::Left, Key::Right)
        } else {
            (Key::Right, Key::Left)
        };
        let to = match key {
            Key::Up => (row.checked_sub(1)?, col),
            Key::Down if row < last_row => (row + 1, col),
            k if k == fwd && col < last_col => (row, col + 1),
            k if k == back => (row, col.checked_sub(1)?),
            Key::Home => (row, 0),
            Key::End => (row, last_col),
            Key::CtrlHome => (0, 0),
            Key::CtrlEnd => (last_row, last_col),
            Key::PageUp => (row.saturating_sub(self.page_rows), col),
            Key::PageDown => ((row + self.page_rows).min(last_row), col),
            _ => return None,
        };
        (to != (row, col)).then_some(to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_stop_at_edges() {
        let g = GridFocus::new(3, 4);
        assert_eq!(g.next((0, 0), Key::Right), Some((0, 1)));
        assert_eq!(g.next((0, 3), Key::Right), None);
        assert_eq!(g.next((0, 0), Key::Up), None);
        assert_eq!(g.next((2, 1), Key::Down), None);
        assert_eq!(g.next((1, 1), Key::Down), Some((2, 1)));
    }

    #[test]
    fn row_ends_corners_and_pages() {
        let g = GridFocus::new(100, 4).page_rows(10);
        assert_eq!(g.next((5, 2), Key::Home), Some((5, 0)));
        assert_eq!(g.next((5, 2), Key::End), Some((5, 3)));
        assert_eq!(g.next((5, 2), Key::CtrlHome), Some((0, 0)));
        assert_eq!(g.next((5, 2), Key::CtrlEnd), Some((99, 3)));
        assert_eq!(g.next((5, 2), Key::PageDown), Some((15, 2)));
        assert_eq!(g.next((95, 2), Key::PageDown), Some((99, 2)));
        assert_eq!(g.next((5, 2), Key::PageUp), Some((0, 2)));
    }

    #[test]
    fn rtl_mirrors_left_and_right() {
        let g = GridFocus::new(2, 3).direction(Direction::Rtl);
        assert_eq!(g.next((0, 0), Key::Left), Some((0, 1)));
        assert_eq!(g.next((0, 1), Key::Right), Some((0, 0)));
    }
}
