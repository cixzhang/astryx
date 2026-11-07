//! Maps keyboard keys and gamepad buttons to Astryx [`Input`].

use astryx_core::Input;
use winit::keyboard::{Key, NamedKey};

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
            NamedKey::ContextMenu => Input::Menu,
            _ => return None,
        }),
        // Some layouts report Space as a character.
        Key::Character(c) if c == " " => Some(Input::A),
        _ => None,
    }
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

    #[cfg(feature = "gamepad")]
    #[test]
    fn gamepad_buttons_map_to_inputs() {
        assert_eq!(gamepad_input(gilrs::Button::South), Some(Input::A));
        assert_eq!(gamepad_input(gilrs::Button::East), Some(Input::B));
        assert_eq!(gamepad_input(gilrs::Button::DPadUp), Some(Input::Up));
    }
}
