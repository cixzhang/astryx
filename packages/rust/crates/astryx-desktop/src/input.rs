//! Maps keyboard keys and gamepad buttons to Astryx [`Input`].

use astryx_core::{A11yNode, Command, Input, Rect, Role, Surface};
use winit::event::Ime;
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

/// What one input method event asks of the surface, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImeStep {
    /// `Surface::text_compose`: composing text and its cursor, a UTF-8 byte
    /// offset; empty text ends composing.
    Compose(String, u32),
    /// `Surface::text_input`: committed text.
    Commit(String),
}

/// The surface calls for an input method event, as the web shell makes
/// them for composition events: composing text goes to `text_compose` with
/// the input method's cursor (the end of the text when it hides the
/// cursor); a commit ends composing, then types the text; an input method
/// that turns off ends composing.
pub fn ime_steps(event: &Ime) -> Vec<ImeStep> {
    match event {
        Ime::Enabled => Vec::new(),
        Ime::Preedit(text, cursor) => {
            let at = match cursor {
                _ if text.is_empty() => 0,
                Some((_, end)) => (*end).min(text.len()),
                None => text.len(),
            };
            vec![ImeStep::Compose(text.clone(), at as u32)]
        }
        Ime::Commit(text) => {
            let mut steps = vec![ImeStep::Compose(String::new(), 0)];
            if !text.is_empty() {
                steps.push(ImeStep::Commit(text.clone()));
            }
            steps
        }
        Ime::Disabled => vec![ImeStep::Compose(String::new(), 0)],
    }
}

/// Where the input method works: the focused text field's bounds, in
/// logical points, from the accessibility tree; `None` when no text field
/// has focus. The shell turns the input method on while there is one and
/// places its candidate window there.
pub fn ime_area(tree: Option<&A11yNode>) -> Option<Rect> {
    fn find(node: &A11yNode) -> Option<Rect> {
        if node.focused && node.role == Role::TextInput {
            return Some(node.bounds);
        }
        node.children.iter().find_map(find)
    }
    tree.and_then(find)
}

/// Gives `steps` to `surface`; the command the commits returned.
pub fn apply_ime<S: Surface + ?Sized>(surface: &mut S, steps: Vec<ImeStep>) -> Command {
    let mut command = Command::None;
    for step in steps {
        match step {
            ImeStep::Compose(text, cursor) => surface.text_compose(&text, cursor),
            ImeStep::Commit(text) => command = command.and(surface.text_input(&text)),
        }
    }
    command
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

    use astryx_core::{DisplayList, FrameInfo, InputPhase, TextMeasure};
    use winit::event::Ime;

    #[test]
    fn composing_text_goes_to_text_compose_with_the_input_methods_cursor() {
        assert_eq!(
            ime_steps(&Ime::Preedit("にほ".into(), Some((3, 3)))),
            [ImeStep::Compose("にほ".into(), 3)]
        );
        // A selection inside the composing text: the caret is at its end.
        assert_eq!(
            ime_steps(&Ime::Preedit("にほん".into(), Some((3, 6)))),
            [ImeStep::Compose("にほん".into(), 6)]
        );
        // A hidden cursor sits at the end, as the web shell puts it.
        assert_eq!(
            ime_steps(&Ime::Preedit("ä".into(), None)),
            [ImeStep::Compose("ä".into(), 2)]
        );
        // Empty composing text ends composing.
        assert_eq!(
            ime_steps(&Ime::Preedit(String::new(), None)),
            [ImeStep::Compose(String::new(), 0)]
        );
    }

    #[test]
    fn a_commit_ends_composing_then_types_and_turning_off_ends_composing() {
        assert_eq!(
            ime_steps(&Ime::Commit("日本".into())),
            [
                ImeStep::Compose(String::new(), 0),
                ImeStep::Commit("日本".into())
            ]
        );
        assert_eq!(
            ime_steps(&Ime::Commit(String::new())),
            [ImeStep::Compose(String::new(), 0)]
        );
        assert_eq!(
            ime_steps(&Ime::Disabled),
            [ImeStep::Compose(String::new(), 0)]
        );
        assert_eq!(ime_steps(&Ime::Enabled), []);
    }

    /// Records the text calls a surface receives.
    #[derive(Default)]
    struct Calls(Vec<String>);

    impl Surface for Calls {
        fn input(&mut self, _: Input, _: InputPhase) -> Command {
            Command::None
        }
        fn frame(&mut self, _: &FrameInfo, _: &dyn TextMeasure) -> DisplayList {
            DisplayList::default()
        }
        fn accessibility(&self) -> Option<A11yNode> {
            None
        }
        fn wants_frames(&self) -> bool {
            false
        }
        fn text_compose(&mut self, text: &str, cursor: u32) {
            self.0.push(format!("compose {text:?} {cursor}"));
        }
        fn text_input(&mut self, text: &str) -> Command {
            self.0.push(format!("input {text:?}"));
            Command::None
        }
    }

    #[test]
    fn a_composition_reaches_the_surface_as_the_web_shell_sends_it() {
        let mut s = Calls::default();
        for event in [
            Ime::Enabled,
            Ime::Preedit("k".into(), Some((1, 1))),
            Ime::Preedit("か".into(), Some((3, 3))),
            Ime::Preedit(String::new(), None),
            Ime::Commit("か".into()),
            Ime::Preedit("n".into(), Some((1, 1))),
            Ime::Disabled,
        ] {
            apply_ime(&mut s, ime_steps(&event));
        }
        assert_eq!(
            s.0,
            [
                "compose \"k\" 1",
                "compose \"か\" 3",
                "compose \"\" 0",
                "compose \"\" 0",
                "input \"か\"",
                "compose \"n\" 1",
                "compose \"\" 0",
            ]
        );
    }
}
