//! Virtual focus: keyboard focus stays in a combobox's input while the
//! arrows move an active option through its list, which assistive
//! technology reads through `aria-activedescendant`.

use super::Key;

/// The active option of a list whose focus stays elsewhere.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VirtualFocus {
    active: Option<usize>,
    /// Arrows cycle past the ends.
    pub wrap: bool,
}

impl VirtualFocus {
    pub fn new() -> Self {
        VirtualFocus {
            active: None,
            wrap: true,
        }
    }

    pub fn active(&self) -> Option<usize> {
        self.active
    }

    /// Sets the active option, as a pointer hovering it does.
    pub fn set(&mut self, active: Option<usize>) {
        self.active = active;
    }

    /// Clears the active option, as typing a new query does.
    pub fn clear(&mut self) {
        self.active = None;
    }

    /// Keeps the active option valid after the list changes to `count`
    /// options.
    pub fn clamp(&mut self, count: usize) {
        if self.active.is_some_and(|a| a >= count) {
            self.active = count.checked_sub(1);
        }
    }

    /// Moves the active option for `key` and returns it. Down from no
    /// active option lands on the first enabled option, Up on the last.
    pub fn key(
        &mut self,
        count: usize,
        key: Key,
        disabled: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        let enabled: Vec<usize> = (0..count).filter(|&i| !disabled(i)).collect();
        if enabled.is_empty() {
            self.active = None;
            return None;
        }
        let pos = self
            .active
            .and_then(|a| enabled.iter().position(|&i| i == a));
        let last = enabled.len() - 1;
        let next = match (key, pos) {
            (Key::Down, None) | (Key::Home, _) => 0,
            (Key::Up, None) | (Key::End, _) => last,
            (Key::Down, Some(p)) if p < last => p + 1,
            (Key::Down, Some(p)) => {
                if self.wrap {
                    0
                } else {
                    p
                }
            }
            (Key::Up, Some(p)) if p > 0 => p - 1,
            (Key::Up, Some(p)) => {
                if self.wrap {
                    last
                } else {
                    p
                }
            }
            (Key::PageDown, p) => (p.unwrap_or(0) + 10).min(last),
            (Key::PageUp, p) => p.unwrap_or(0).saturating_sub(10),
            _ => return self.active,
        };
        self.active = Some(enabled[next]);
        self.active
    }

    /// The id assistive technology reads as the active descendant: `prefix`
    /// joined with the active index.
    pub fn descendant_id(&self, prefix: &str) -> Option<String> {
        self.active.map(|a| format!("{prefix}-{a}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn down_from_nothing_lands_on_first_enabled() {
        let mut v = VirtualFocus::new();
        assert_eq!(v.key(4, Key::Down, |i| i == 0), Some(1));
        assert_eq!(v.key(4, Key::Down, |i| i == 0), Some(2));
        assert_eq!(v.key(4, Key::Down, |i| i == 0), Some(3));
        assert_eq!(
            v.key(4, Key::Down, |i| i == 0),
            Some(1),
            "wraps past disabled"
        );
        assert_eq!(v.descendant_id("opt"), Some("opt-1".into()));
    }

    #[test]
    fn up_from_nothing_lands_on_last_and_clamps() {
        let mut v = VirtualFocus::new();
        assert_eq!(v.key(5, Key::Up, |_| false), Some(4));
        v.clamp(2);
        assert_eq!(v.active(), Some(1));
        v.clear();
        assert_eq!(v.active(), None);
        assert_eq!(v.key(0, Key::Down, |_| false), None);
    }
}
