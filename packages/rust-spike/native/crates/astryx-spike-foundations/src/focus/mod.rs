//! Keyboard focus models, as state machines components drive from key
//! events: roving focus over a group, typeahead, two-dimensional grid
//! focus, and virtual focus (`aria-activedescendant`) for comboboxes.
//!
//! They hold no UI. A component maps its keys to [`Key`], asks the model
//! where focus goes, and renders the answer.

mod grid;
mod roving;
mod typeahead;
mod virtual_focus;

pub use grid::GridFocus;
pub use roving::{Orientation, RovingFocus};
pub use typeahead::{fold_for_typeahead, Typeahead, TYPEAHEAD_RESET_MS};
pub use virtual_focus::VirtualFocus;

/// A navigation key, after the shell's key mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    /// Ctrl or Cmd with Home: the first cell of a grid.
    CtrlHome,
    /// Ctrl or Cmd with End: the last cell of a grid.
    CtrlEnd,
}

impl Key {
    /// The key for a core [`Input`](astryx_core::Input), when it is one.
    pub fn from_input(input: astryx_core::Input) -> Option<Key> {
        use astryx_core::Input;
        Some(match input {
            Input::Up => Key::Up,
            Input::Down => Key::Down,
            Input::Left => Key::Left,
            Input::Right => Key::Right,
            Input::Home => Key::Home,
            Input::End => Key::End,
            Input::L2 => Key::PageUp,
            Input::R2 => Key::PageDown,
            _ => return None,
        })
    }
}
