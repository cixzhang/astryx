//! Roving focus: one focus stop for a group (a toolbar, a button group, a
//! radio group, a menu, a list), moved inside the group by the arrow keys.

use super::Key;
use astryx_core::Direction;

/// Which arrows move focus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Orientation {
    /// Up and Down.
    #[default]
    Vertical,
    /// Left and Right, mirrored in right-to-left.
    Horizontal,
    /// All four arrows, in reading order.
    Both,
}

/// Where focus goes in a group of `count` items, as Astryx's `useListFocus`
/// moves it: arrows step past disabled items, Home and End jump to the ends,
/// and `wrap` cycles from the last item to the first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RovingFocus {
    pub orientation: Orientation,
    pub wrap: bool,
    pub has_home_end: bool,
    pub direction: Direction,
}

impl Default for RovingFocus {
    fn default() -> Self {
        RovingFocus {
            orientation: Orientation::Vertical,
            wrap: true,
            has_home_end: true,
            direction: Direction::Ltr,
        }
    }
}

impl RovingFocus {
    pub fn horizontal() -> Self {
        RovingFocus {
            orientation: Orientation::Horizontal,
            ..Self::default()
        }
    }

    pub fn vertical() -> Self {
        Self::default()
    }

    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// The item focus moves to from `current` for `key`, skipping items
    /// `disabled` reports; `None` when focus stays (or the key is not this
    /// group's).
    pub fn next(
        &self,
        count: usize,
        current: Option<usize>,
        key: Key,
        disabled: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        if count == 0 {
            return None;
        }
        let rtl = self.direction.is_rtl();
        let step: i64 = match (self.orientation, key) {
            (Orientation::Vertical | Orientation::Both, Key::Down) => 1,
            (Orientation::Vertical | Orientation::Both, Key::Up) => -1,
            (Orientation::Horizontal | Orientation::Both, Key::Right) => {
                if rtl {
                    -1
                } else {
                    1
                }
            }
            (Orientation::Horizontal | Orientation::Both, Key::Left) => {
                if rtl {
                    1
                } else {
                    -1
                }
            }
            (_, Key::Home) if self.has_home_end => {
                return (0..count)
                    .find(|&i| !disabled(i))
                    .filter(|&i| Some(i) != current);
            }
            (_, Key::End) if self.has_home_end => {
                return (0..count)
                    .rev()
                    .find(|&i| !disabled(i))
                    .filter(|&i| Some(i) != current);
            }
            _ => return None,
        };
        let start: i64 = match current {
            Some(c) => c as i64,
            // Nothing focused yet: the first arrow lands on an end.
            None if step > 0 => -1,
            None => count as i64,
        };
        let mut ix = start;
        for _ in 0..count {
            ix += step;
            if ix < 0 || ix >= count as i64 {
                if !self.wrap {
                    return None;
                }
                ix = ix.rem_euclid(count as i64);
            }
            if Some(ix as usize) == current {
                return None;
            }
            if !disabled(ix as usize) {
                return Some(ix as usize);
            }
        }
        None
    }

    /// The item that holds the group's one tab stop: the selected item when
    /// there is one and it is enabled, else the first enabled item.
    pub fn tab_stop(
        count: usize,
        selected: Option<usize>,
        disabled: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        selected
            .filter(|&s| s < count && !disabled(s))
            .or_else(|| (0..count).find(|&i| !disabled(i)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: fn(usize) -> bool = |_| false;

    #[test]
    fn arrows_step_and_wrap() {
        let r = RovingFocus::vertical();
        assert_eq!(r.next(4, Some(0), Key::Down, NONE), Some(1));
        assert_eq!(r.next(4, Some(3), Key::Down, NONE), Some(0));
        assert_eq!(r.next(4, Some(0), Key::Up, NONE), Some(3));
        assert_eq!(r.next(4, Some(1), Key::Left, NONE), None, "other axis");
        let no_wrap = r.wrap(false);
        assert_eq!(no_wrap.next(4, Some(3), Key::Down, NONE), None);
    }

    #[test]
    fn disabled_items_are_skipped() {
        let r = RovingFocus::vertical();
        let dis = |i| i == 1 || i == 2;
        assert_eq!(r.next(4, Some(0), Key::Down, dis), Some(3));
        assert_eq!(r.next(4, Some(3), Key::Up, dis), Some(0));
        assert_eq!(
            r.next(3, Some(0), Key::Down, |i| i != 0),
            None,
            "only one enabled"
        );
    }

    #[test]
    fn horizontal_mirrors_in_rtl() {
        let ltr = RovingFocus::horizontal();
        let rtl = RovingFocus::horizontal().direction(Direction::Rtl);
        assert_eq!(ltr.next(3, Some(1), Key::Right, NONE), Some(2));
        assert_eq!(rtl.next(3, Some(1), Key::Right, NONE), Some(0));
        assert_eq!(rtl.next(3, Some(1), Key::Left, NONE), Some(2));
    }

    #[test]
    fn home_end_and_first_arrow() {
        let r = RovingFocus::vertical();
        let dis = |i| i == 0 || i == 4;
        assert_eq!(r.next(5, Some(2), Key::Home, dis), Some(1));
        assert_eq!(r.next(5, Some(2), Key::End, dis), Some(3));
        assert_eq!(r.next(5, None, Key::Down, NONE), Some(0));
        assert_eq!(r.next(5, None, Key::Up, NONE), Some(4));
    }

    #[test]
    fn tab_stop_prefers_selection() {
        assert_eq!(RovingFocus::tab_stop(4, Some(2), NONE), Some(2));
        assert_eq!(
            RovingFocus::tab_stop(4, Some(2), |i| i == 2 || i == 0),
            Some(1)
        );
        assert_eq!(RovingFocus::tab_stop(4, None, NONE), Some(0));
    }
}
