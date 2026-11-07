//! Keyboard to [`Input`] mapping.

use astryx_core::Input;

/// Maps a `KeyboardEvent.key` value to an input. Returns `None` for keys the
/// shell leaves to the browser. In a text field, Space types a space, so it
/// arrives as text input instead of `A`.
pub(crate) fn map_key(
    key: &str,
    shift: bool,
    command_modifier: bool,
    in_text_field: bool,
) -> Option<Input> {
    // Ctrl, Alt, and Meta chords stay with the browser and the screen reader.
    if command_modifier {
        return None;
    }
    if in_text_field && matches!(key, " " | "Spacebar") {
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
        "Backspace" => Input::Backspace,
        "Delete" | "Del" => Input::Delete,
        "Home" => Input::Home,
        "End" => Input::End,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_navigation_and_actions() {
        assert_eq!(map_key("ArrowUp", false, false, false), Some(Input::Up));
        assert_eq!(map_key("ArrowDown", false, false, false), Some(Input::Down));
        assert_eq!(map_key("ArrowLeft", false, false, false), Some(Input::Left));
        assert_eq!(
            map_key("ArrowRight", false, false, false),
            Some(Input::Right)
        );
        assert_eq!(map_key("Tab", false, false, false), Some(Input::Next));
        assert_eq!(map_key("Tab", true, false, false), Some(Input::Previous));
        assert_eq!(map_key("Enter", false, false, false), Some(Input::A));
        assert_eq!(map_key(" ", false, false, false), Some(Input::A));
        assert_eq!(map_key("Escape", false, false, false), Some(Input::B));
    }

    #[test]
    fn maps_editing_keys_and_types_space_in_text_fields() {
        assert_eq!(
            map_key("Backspace", false, false, false),
            Some(Input::Backspace)
        );
        assert_eq!(map_key("Delete", false, false, true), Some(Input::Delete));
        assert_eq!(map_key("Home", false, false, true), Some(Input::Home));
        assert_eq!(map_key("End", false, false, true), Some(Input::End));
        assert_eq!(map_key(" ", false, false, true), None);
        assert_eq!(map_key("Enter", false, false, true), Some(Input::A));
        assert_eq!(map_key("ArrowLeft", false, false, true), Some(Input::Left));
    }

    #[test]
    fn leaves_other_keys_and_chords_to_the_browser() {
        assert_eq!(map_key("a", false, false, false), None);
        assert_eq!(map_key("F5", false, false, false), None);
        assert_eq!(map_key("Tab", false, true, false), None);
        assert_eq!(map_key("ArrowDown", false, true, false), None);
    }
}
