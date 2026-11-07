//! Maps keyboard keys and gamepad buttons to Astryx [`Input`].

use astryx_core::Input;
use winit::keyboard::{Key, ModifiersState, NamedKey};

/// The input for a key, given whether Shift is held.
pub fn key_input(key: &Key, shift: bool) -> Option<Input> {
    match key {
        Key::Named(named) => Some(match named {
            NamedKey::ArrowUp => Input::Up,
            NamedKey::ArrowDown => Input::Down,
            NamedKey::ArrowLeft => Input::Left,
            NamedKey::ArrowRight => Input::Right,
            NamedKey::Tab if shift => Input::Previous,
            NamedKey::Tab => Input::Next,
            NamedKey::Enter | NamedKey::Space => Input::A,
            NamedKey::Escape => Input::B,
            NamedKey::Backspace => Input::Backspace,
            NamedKey::Delete => Input::Delete,
            NamedKey::Home => Input::Home,
            NamedKey::End => Input::End,
            NamedKey::ContextMenu => Input::Menu,
            _ => return None,
        }),
        // Some layouts report Space as a character.
        Key::Character(c) if c == " " => Some(Input::A),
        _ => None,
    }
}

/// True for keys that type into a focused text field instead of moving
/// focus or confirming.
pub fn is_text_key(key: &Key) -> bool {
    matches!(key, Key::Character(_) | Key::Named(NamedKey::Space))
}

/// The text a key press types, without control characters. Shortcuts with
/// Ctrl or the Super key type nothing.
pub fn typed_text(text: Option<&str>, modifiers: ModifiersState) -> Option<String> {
    if modifiers.control_key() || modifiers.super_key() {
        return None;
    }
    let text: String = text?.chars().filter(|c| !c.is_control()).collect();
    (!text.is_empty()).then_some(text)
}

#[cfg(feature = "gamepad")]
pub fn gamepad_input(button: gilrs::Button) -> Option<Input> {
    use gilrs::Button as B;
    Some(match button {
        B::South => Input::A,
        B::East => Input::B,
        B::West => Input::X,
        B::North => Input::Y,
        B::DPadUp => Input::Up,
        B::DPadDown => Input::Down,
        B::DPadLeft => Input::Left,
        B::DPadRight => Input::Right,
        B::LeftTrigger => Input::L1,
        B::RightTrigger => Input::R1,
        B::LeftTrigger2 => Input::L2,
        B::RightTrigger2 => Input::R2,
        B::Start => Input::Start,
        B::Select => Input::Select,
        B::Mode => Input::Menu,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_map_to_inputs() {
        assert_eq!(
            key_input(&Key::Named(NamedKey::ArrowLeft), false),
            Some(Input::Left)
        );
        assert_eq!(
            key_input(&Key::Named(NamedKey::Tab), false),
            Some(Input::Next)
        );
        assert_eq!(
            key_input(&Key::Named(NamedKey::Tab), true),
            Some(Input::Previous)
        );
        assert_eq!(
            key_input(&Key::Named(NamedKey::Enter), false),
            Some(Input::A)
        );
        assert_eq!(
            key_input(&Key::Named(NamedKey::Space), false),
            Some(Input::A)
        );
        assert_eq!(
            key_input(&Key::Character(" ".into()), false),
            Some(Input::A)
        );
        assert_eq!(
            key_input(&Key::Named(NamedKey::Escape), false),
            Some(Input::B)
        );
        assert_eq!(key_input(&Key::Character("q".into()), false), None);
    }

    #[test]
    fn typed_text_drops_controls_and_shortcuts() {
        let none = ModifiersState::empty();
        assert_eq!(typed_text(Some("é"), none).as_deref(), Some("é"));
        assert_eq!(typed_text(Some(" "), none).as_deref(), Some(" "));
        assert_eq!(typed_text(Some("\r"), none), None);
        assert_eq!(typed_text(Some("\u{8}"), none), None);
        assert_eq!(typed_text(Some("a"), ModifiersState::CONTROL), None);
        assert_eq!(
            typed_text(Some("ä"), ModifiersState::ALT).as_deref(),
            Some("ä")
        );
        assert_eq!(typed_text(None, none), None);
        assert!(is_text_key(&Key::Named(NamedKey::Space)));
        assert!(!is_text_key(&Key::Named(NamedKey::Enter)));
    }

    #[cfg(feature = "gamepad")]
    #[test]
    fn gamepad_buttons_map_to_inputs() {
        assert_eq!(gamepad_input(gilrs::Button::South), Some(Input::A));
        assert_eq!(gamepad_input(gilrs::Button::East), Some(Input::B));
        assert_eq!(gamepad_input(gilrs::Button::DPadUp), Some(Input::Up));
    }
}
