//! Keyboard to [`Input`] mapping.

use astryx_core::Input;

/// Maps a `KeyboardEvent.key` value to an input. Returns `None` for keys the
/// shell leaves to the browser.
pub(crate) fn map_key(key: &str, shift: bool, command_modifier: bool) -> Option<Input> {
    // Ctrl, Alt, and Meta chords stay with the browser and the screen reader.
    if command_modifier {
        return None;
    }
    Some(match key {
        "ArrowUp" | "Up" => Input::Up,
        "ArrowDown" | "Down" => Input::Down,
        "ArrowLeft" | "Left" => Input::Left,
        "ArrowRight" | "Right" => Input::Right,
        "Tab" if shift => Input::Previous,
        "Tab" => Input::Next,
        "Enter" | " " | "Spacebar" => Input::A,
        "Escape" | "Esc" => Input::B,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_navigation_and_actions() {
        assert_eq!(map_key("ArrowUp", false, false), Some(Input::Up));
        assert_eq!(map_key("ArrowDown", false, false), Some(Input::Down));
        assert_eq!(map_key("ArrowLeft", false, false), Some(Input::Left));
        assert_eq!(map_key("ArrowRight", false, false), Some(Input::Right));
        assert_eq!(map_key("Tab", false, false), Some(Input::Next));
        assert_eq!(map_key("Tab", true, false), Some(Input::Previous));
        assert_eq!(map_key("Enter", false, false), Some(Input::A));
        assert_eq!(map_key(" ", false, false), Some(Input::A));
        assert_eq!(map_key("Escape", false, false), Some(Input::B));
    }

    #[test]
    fn leaves_other_keys_and_chords_to_the_browser() {
        assert_eq!(map_key("a", false, false), None);
        assert_eq!(map_key("F5", false, false), None);
        assert_eq!(map_key("Tab", false, true), None);
        assert_eq!(map_key("ArrowDown", false, true), None);
    }
}
