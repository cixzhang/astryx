//! Typeahead: typing letters jumps to the next item whose label starts with
//! them, as Astryx's `useTypeahead` and `typeaheadMatch` do.

use unicode_normalization::UnicodeNormalization;

/// How long typed letters stay in the buffer, in milliseconds.
pub const TYPEAHEAD_RESET_MS: f64 = 750.0;

/// Folds text for matching: no accents, stroked letters plain, lower case.
pub fn fold_for_typeahead(text: &str) -> String {
    let stripped: String = text
        .nfd()
        .filter(|c| !('\u{0300}'..='\u{036f}').contains(c))
        .map(|c| match c {
            'đ' | 'Đ' => 'd',
            'ł' | 'Ł' => 'l',
            'ø' | 'Ø' => 'o',
            'ħ' | 'Ħ' => 'h',
            'ŧ' | 'Ŧ' => 't',
            'ƀ' | 'Ƀ' => 'b',
            'ı' => 'i',
            other => other,
        })
        .collect();
    stripped.nfc().collect::<String>().to_lowercase()
}

fn exact_fold(text: &str) -> String {
    text.nfc()
        .map(|c| match c {
            'I' => 'ı',
            'İ' => 'i',
            other => other,
        })
        .collect::<String>()
        .to_lowercase()
}

fn has_mark(typed: &str) -> bool {
    typed.chars().any(|c| "đĐłŁøØħĦŧŦƀɃıİ".contains(c))
        || typed.nfd().any(|c| ('\u{0300}'..='\u{036f}').contains(&c))
}

fn prefix_match(label: &str, typed: &str, exact: bool) -> bool {
    let label = label.trim();
    if exact {
        return exact_fold(label).starts_with(&exact_fold(typed));
    }
    let typed = fold_for_typeahead(typed);
    !typed.is_empty() && fold_for_typeahead(label).starts_with(&typed)
}

/// The typed buffer and its reset clock.
#[derive(Clone, Debug, Default)]
pub struct Typeahead {
    buffer: String,
    last_ms: f64,
    reset_ms: Option<f64>,
}

impl Typeahead {
    pub fn new() -> Self {
        Self::default()
    }

    /// Uses another reset time than [`TYPEAHEAD_RESET_MS`].
    pub fn reset_after(mut self, ms: f64) -> Self {
        self.reset_ms = Some(ms);
        self
    }

    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
    }

    /// Adds `typed` (one key's text) at `now_ms` and returns the item to
    /// focus, if any matches. `labels` are the items in order; `current` is
    /// the focused one. Repeating one letter cycles through the items that
    /// start with it. Returns `None` for a key typeahead does not take
    /// (a space before any letter).
    pub fn input<S: AsRef<str>>(
        &mut self,
        typed: &str,
        now_ms: f64,
        labels: &[S],
        current: Option<usize>,
        disabled: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        if now_ms - self.last_ms > self.reset_ms.unwrap_or(TYPEAHEAD_RESET_MS) {
            self.buffer.clear();
        }
        if typed.is_empty() || (typed == " " && self.buffer.is_empty()) {
            return None;
        }
        if self.buffer.is_empty() && fold_for_typeahead(typed).is_empty() {
            return None;
        }
        let repeat = !self.buffer.is_empty()
            && self
                .buffer
                .chars()
                .all(|c| fold_for_typeahead(&c.to_string()) == fold_for_typeahead(typed));
        if repeat {
            self.buffer = typed.to_string();
        } else {
            self.buffer.push_str(typed);
        }
        self.last_ms = now_ms;
        let count = labels.len();
        if count == 0 {
            return None;
        }
        let start = current.unwrap_or(0);
        let offset = usize::from(current.is_some() && self.buffer.chars().count() == 1);
        let order: Vec<usize> = (0..count)
            .map(|i| (start + offset + i) % count)
            .filter(|&i| !disabled(i))
            .collect();
        let passes: &[bool] = if has_mark(&self.buffer) {
            &[true, false]
        } else {
            &[false]
        };
        for &exact in passes {
            if let Some(&ix) = order
                .iter()
                .find(|&&i| prefix_match(labels[i].as_ref(), &self.buffer, exact))
            {
                return Some(ix);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRUIT: [&str; 6] = [
        "Apple",
        "Apricot",
        "Banana",
        "Blueberry",
        "Éclair",
        "Øresund",
    ];
    const NONE: fn(usize) -> bool = |_| false;

    #[test]
    fn prefix_extends_within_the_reset_time() {
        let mut t = Typeahead::new();
        assert_eq!(t.input("a", 0.0, &FRUIT, None, NONE), Some(0));
        assert_eq!(t.input("p", 100.0, &FRUIT, Some(0), NONE), Some(0));
        assert_eq!(t.input("r", 200.0, &FRUIT, Some(0), NONE), Some(1));
        assert_eq!(t.buffer(), "apr");
        // After the reset time a new letter starts over.
        assert_eq!(t.input("b", 2000.0, &FRUIT, Some(1), NONE), Some(2));
    }

    #[test]
    fn repeating_a_letter_cycles() {
        let mut t = Typeahead::new();
        assert_eq!(t.input("b", 0.0, &FRUIT, Some(0), NONE), Some(2));
        assert_eq!(t.input("b", 100.0, &FRUIT, Some(2), NONE), Some(3));
        assert_eq!(t.input("b", 200.0, &FRUIT, Some(3), NONE), Some(2));
    }

    #[test]
    fn accents_and_stroked_letters_fold() {
        let mut t = Typeahead::new();
        assert_eq!(t.input("e", 0.0, &FRUIT, None, NONE), Some(4));
        t.reset();
        assert_eq!(t.input("o", 0.0, &FRUIT, None, NONE), Some(5));
        assert_eq!(fold_for_typeahead("Łódź"), "lodz");
    }

    #[test]
    fn disabled_items_and_leading_space_are_skipped() {
        let mut t = Typeahead::new();
        assert_eq!(t.input(" ", 0.0, &FRUIT, None, NONE), None);
        assert_eq!(t.input("a", 0.0, &FRUIT, None, |i| i == 0), Some(1));
    }
}
