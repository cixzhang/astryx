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

/// The Astryx key for a winit key.
pub fn key_of(key: &Key) -> Option<astryx_core::Key> {
    use astryx_core::Key as K;
    Some(match key {
        Key::Character(c) if c == " " => K::Space,
        Key::Character(c) => K::Character(c.to_string()),
        Key::Named(named) => match named {
            NamedKey::Enter => K::Enter,
            NamedKey::Space => K::Space,
            NamedKey::Escape => K::Escape,
            NamedKey::Tab => K::Tab,
            NamedKey::ArrowUp => K::Up,
            NamedKey::ArrowDown => K::Down,
            NamedKey::ArrowLeft => K::Left,
            NamedKey::ArrowRight => K::Right,
            NamedKey::Home => K::Home,
            NamedKey::End => K::End,
            NamedKey::PageUp => K::PageUp,
            NamedKey::PageDown => K::PageDown,
            NamedKey::Backspace => K::Backspace,
            NamedKey::Delete => K::Delete,
            NamedKey::ContextMenu => K::ContextMenu,
            NamedKey::F1 => K::F(1),
            NamedKey::F2 => K::F(2),
            NamedKey::F3 => K::F(3),
            NamedKey::F4 => K::F(4),
            NamedKey::F5 => K::F(5),
            NamedKey::F6 => K::F(6),
            NamedKey::F7 => K::F(7),
            NamedKey::F8 => K::F(8),
            NamedKey::F9 => K::F(9),
            NamedKey::F10 => K::F(10),
            NamedKey::F11 => K::F(11),
            NamedKey::F12 => K::F(12),
            _ => return None,
        },
        _ => return None,
    })
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

/// Nintendo's USB vendor id. Its pads print A on the right, B at the
/// bottom, X at the top, and Y on the left.
pub const NINTENDO_VENDOR: u16 = 0x057e;

/// The input for a gamepad button. Face buttons map by their printed label,
/// as the Brick shell maps them: gilrs names them by position, so on a
/// Nintendo-style pad (`nintendo`) the right button is A and the bottom
/// one B.
#[cfg(feature = "gamepad")]
pub fn gamepad_input(button: gilrs::Button, nintendo: bool) -> Option<Input> {
    use gilrs::Button as B;
    Some(match (button, nintendo) {
        (B::South, false) | (B::East, true) => Input::A,
        (B::East, false) | (B::South, true) => Input::B,
        (B::West, false) | (B::North, true) => Input::X,
        (B::North, false) | (B::West, true) => Input::Y,
        (button, _) => return gamepad_other(button),
    })
}

#[cfg(feature = "gamepad")]
fn gamepad_other(button: gilrs::Button) -> Option<Input> {
    use gilrs::Button as B;
    Some(match button {
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
        use gilrs::Button as B;
        assert_eq!(gamepad_input(B::South, false), Some(Input::A));
        assert_eq!(gamepad_input(B::East, false), Some(Input::B));
        assert_eq!(gamepad_input(B::DPadUp, false), Some(Input::Up));
        // A Nintendo pad maps by its printed labels.
        assert_eq!(gamepad_input(B::East, true), Some(Input::A));
        assert_eq!(gamepad_input(B::South, true), Some(Input::B));
        assert_eq!(gamepad_input(B::North, true), Some(Input::X));
        assert_eq!(gamepad_input(B::West, true), Some(Input::Y));
    }
}
