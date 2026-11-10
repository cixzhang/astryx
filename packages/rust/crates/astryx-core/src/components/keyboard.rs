//! OnScreenKeyboard: text entry with a gamepad or remote. Experimental:
//! Astryx has no on-screen keyboard spec yet, so it is public only through
//! `astryx_core::lab`.
//!
//! Two layouts, switched with L1 and R1 pressed together:
//!
//! - **Chord** (the default): a 3×3 grid placed as the D-pad is. Each of
//!   the eight directions chooses a group of four keys, and A, B, X, or Y
//!   presses one of them. The four keys sit where their buttons sit (see
//!   [`FaceLayout`]), so a key carries no button label. Hold a
//!   direction and press a face button, or tap a direction to choose its
//!   group and then press a face button. One group (down, in the English
//!   layout) holds Space, Delete, Shift (a capital for the next letter),
//!   and Done, in the same place on every layer. L1 and R1 cycle the
//!   layers: lowercase, uppercase, numbers, symbols. Start is done, Select
//!   cancels.
//! - **Grid**: rows of keys. The D-pad moves between keys, A types the
//!   highlighted one, B deletes, X types a space, Y toggles capitals, L1
//!   and R1 (or L2 and R2) switch between letters and symbols, Start is
//!   done, Select cancels.
//!
//! L1 and R1 pressed together cycle layouts instead of changing the layer.
//! L1 or R1 alone acts when it is released, so the two count as together
//! whenever both are down at once: pressed within a few milliseconds of
//! each other, or one pressed while the other is held. A brief toast names
//! the new layout, and the app hears the choice, so it can keep it as the
//! person's preference.
//!
//! Layouts are data: [`ChordLayout`] and [`GridLayout`] hold the keys, so
//! other languages can supply their own. A keyboard on the device, a
//! screen reader, or an input method types into the field directly.

use super::text::{text_draw, text_node, type_style};
use super::{len, radii};
use crate::contract::{
    Color, FaceLayout, FontFamily, Input, InputPhase, Live, PathCmd, Point, Role,
};
use crate::element::{
    base_props, Base, BoxDraw, Build, BuildCx, Draw, Edit, Indicator, Interact, Node, Semantics,
};
use std::rc::Rc;
use taffy::prelude::*;

/// Which keyboard shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KeyboardKind {
    #[default]
    Chord,
    Grid,
}

impl KeyboardKind {
    pub fn name(self) -> &'static str {
        match self {
            KeyboardKind::Chord => "Chord keyboard",
            KeyboardKind::Grid => "Grid keyboard",
        }
    }
    fn next(self) -> Self {
        match self {
            KeyboardKind::Chord => KeyboardKind::Grid,
            KeyboardKind::Grid => KeyboardKind::Chord,
        }
    }
}

/// The eight directions, clockwise from up: the order of a chord layer's
/// groups.
pub const DIRECTIONS: [&str; 8] = [
    "up",
    "up right",
    "right",
    "down right",
    "down",
    "down left",
    "left",
    "up left",
];

/// The face buttons in the order of a group's keys: A (bottom), B (right),
/// X (left), Y (top).
pub const FACE_BUTTONS: [Input; 4] = [Input::A, Input::B, Input::X, Input::Y];

/// One key of a chord group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChordKey {
    Char(char),
    Space,
    Delete,
    /// A capital for the next letter.
    Shift,
    Done,
}

/// One layer of a chord layout: eight groups of four keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChordLayer {
    /// A short name shown in the middle of the grid, such as "abc".
    pub name: &'static str,
    pub groups: [[ChordKey; 4]; 8],
}

/// The layers of a chord keyboard, cycled with L1 and R1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChordLayout {
    pub layers: Vec<ChordLayer>,
}

/// The control group: Space on A, Delete on B, Shift on X, Done on Y.
const CONTROLS: [ChordKey; 4] = [
    ChordKey::Space,
    ChordKey::Delete,
    ChordKey::Shift,
    ChordKey::Done,
];

/// A layer from seven groups of characters, in `DIRECTIONS` order without
/// down, each four in `FACE_BUTTONS` order; down holds the controls.
fn layer(name: &'static str, groups: [[char; 4]; 7]) -> ChordLayer {
    let mut out = [CONTROLS; 8];
    for (i, g) in groups.iter().enumerate() {
        out[if i < 4 { i } else { i + 1 }] = g.map(ChordKey::Char);
    }
    ChordLayer { name, groups: out }
}

impl ChordLayout {
    /// English, by letter frequency: the most common letters sit on the
    /// three free straight directions and on A, the easiest press; the
    /// rarest on the diagonals and Y. Down holds the controls.
    pub fn english() -> Self {
        // Up, up right, right, down right, down left, left, up left.
        let lower = [
            ['e', 'o', 's', 'd'],
            ['u', 'g', 'v', 'q'],
            ['t', 'i', 'h', 'l'],
            ['w', 'p', 'j', '.'],
            ['f', 'b', 'x', ','],
            ['a', 'n', 'r', 'c'],
            ['m', 'y', 'k', 'z'],
        ];
        let upper = lower.map(|g| g.map(|c| c.to_ascii_uppercase()));
        let numbers = [
            ['1', '2', '3', '4'],
            ['+', '-', '*', '/'],
            ['5', '6', '7', '8'],
            [':', ';', '\'', '"'],
            ['#', '$', '@', '&'],
            ['9', '0', '.', ','],
            ['=', '%', '(', ')'],
        ];
        let symbols = [
            ['!', '?', '-', '_'],
            ['[', ']', '{', '}'],
            ['@', '#', '$', '%'],
            ['~', '`', '^', '|'],
            [':', ';', '+', '='],
            ['&', '*', '(', ')'],
            ['<', '>', '/', '\\'],
        ];
        ChordLayout {
            layers: vec![
                layer("abc", lower),
                layer("ABC", upper),
                layer("123", numbers),
                layer("#+=", symbols),
            ],
        }
    }
}

/// The pages of a grid keyboard: rows of characters. The bottom row of
/// special keys (capitals, symbols, space, delete, done) is always added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridLayout {
    pub pages: Vec<Vec<&'static str>>,
}

impl GridLayout {
    /// QWERTY letters, then numbers and symbols.
    pub fn qwerty() -> Self {
        GridLayout {
            pages: vec![
                vec!["qwertyuiop", "asdfghjkl'", "zxcvbnm,.?"],
                vec!["1234567890", "@#$%&*-+()", "!\"':;/_=~<"],
            ],
        }
    }
}

/// The special keys of a grid's bottom row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Special {
    Shift,
    Page,
    Space,
    Delete,
    Done,
}

const SPECIALS: [Special; 5] = [
    Special::Shift,
    Special::Page,
    Special::Space,
    Special::Delete,
    Special::Done,
];

/// What the keyboard asks the app to do besides editing the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardAction {
    /// Start, or the Done key.
    Done,
    /// Select.
    Cancel,
    /// L1 and R1 together chose another layout; keep it as a preference.
    Switched(KeyboardKind),
}

/// A change from the keyboard: its new state and text, and an action when
/// one was asked for.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyboardUpdate {
    pub state: KeyboardState,
    pub value: String,
    pub action: Option<KeyboardAction>,
}

/// How long the toast naming a new layout shows.
const TOAST_MS: f64 = 1500.0;

/// What a keyboard remembers between presses. The app keeps it, with the
/// text, and replaces both from each [`KeyboardUpdate`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeyboardState {
    pub kind: KeyboardKind,
    /// The chord layer, or the grid page.
    pub layer: usize,
    /// Directions held: up 1, right 2, down 4, left 8.
    held: u8,
    /// The most directions held together since the last time none were.
    peak: u8,
    /// A face button was pressed during this hold, so releasing it does
    /// not latch the group.
    typed_in_hold: bool,
    /// A group chosen by a tap, kept until a key is pressed.
    latched: Option<usize>,
    /// The next letter is a capital (chord Shift).
    pub shift: bool,
    /// A chord key was pressed at least once.
    pub typed_once: bool,
    /// L1 and R1 while held, and whether the pair made a chord since both
    /// were last up.
    l1: bool,
    r1: bool,
    chorded: bool,
    /// The grid's highlighted key: row and column. The special row is the
    /// last row.
    pub cursor: (usize, usize),
    /// Capitals on the grid.
    pub caps: bool,
    /// What a screen reader announces now.
    pub announce: String,
    /// The toast and when it hides, in the runtime's milliseconds.
    pub toast: Option<(String, f64)>,
}

impl KeyboardState {
    pub fn new(kind: KeyboardKind) -> Self {
        KeyboardState {
            kind,
            ..KeyboardState::default()
        }
    }

    /// The chord group held or latched now.
    pub fn group(&self) -> Option<usize> {
        direction_group(self.held).or(self.latched)
    }

    /// Applies one input at `now` (milliseconds) to `value`. Returns the
    /// action it asks for, if any.
    pub fn input(
        &mut self,
        chord: &ChordLayout,
        grid: &GridLayout,
        value: &mut String,
        input: Input,
        phase: InputPhase,
        now: f64,
    ) -> Option<KeyboardAction> {
        if matches!(input, Input::L1 | Input::R1) {
            return self.shoulders(chord, grid, input, phase, now);
        }
        let pressed = phase == InputPhase::Pressed;
        match input {
            Input::Start if pressed => return Some(KeyboardAction::Done),
            Input::Select if pressed => return Some(KeyboardAction::Cancel),
            Input::Backspace | Input::Delete if pressed => {
                value.pop();
                self.announce = "Delete".into();
                return None;
            }
            _ => {}
        }
        match self.kind {
            KeyboardKind::Chord => self.chord_input(chord, value, input, phase),
            KeyboardKind::Grid => self.grid_input(grid, value, input, phase),
        }
    }

    /// L1 and R1: alone, the previous and next layer when released;
    /// together, the next layout.
    fn shoulders(
        &mut self,
        chord: &ChordLayout,
        grid: &GridLayout,
        input: Input,
        phase: InputPhase,
        now: f64,
    ) -> Option<KeyboardAction> {
        let (mine, other) = match input {
            Input::L1 => (&mut self.l1, self.r1),
            _ => (&mut self.r1, self.l1),
        };
        match phase {
            InputPhase::Pressed => {
                if *mine {
                    return None;
                }
                *mine = true;
                if other && !self.chorded {
                    self.chorded = true;
                    self.kind = self.kind.next();
                    self.layer = 0;
                    self.latched = None;
                    self.shift = false;
                    self.cursor = (0, 0);
                    self.toast = Some((self.kind.name().into(), now + TOAST_MS));
                    self.announce = self.kind.name().into();
                    return Some(KeyboardAction::Switched(self.kind));
                }
                None
            }
            InputPhase::Released => {
                if !std::mem::take(mine) {
                    return None;
                }
                if self.chorded {
                    if !self.l1 && !self.r1 {
                        self.chorded = false;
                    }
                    return None;
                }
                let forward = input == Input::R1;
                match self.kind {
                    KeyboardKind::Chord => {
                        let n = chord.layers.len().max(1);
                        self.layer = if forward {
                            (self.layer + 1) % n
                        } else {
                            (self.layer + n - 1) % n
                        };
                        self.announce = chord.layers[self.layer].name.into();
                        self.announce_group(chord);
                    }
                    KeyboardKind::Grid => self.turn_page(grid, forward),
                }
                None
            }
        }
    }

    fn chord_input(
        &mut self,
        layout: &ChordLayout,
        value: &mut String,
        input: Input,
        phase: InputPhase,
    ) -> Option<KeyboardAction> {
        let bit = match input {
            Input::Up => 1,
            Input::Right => 2,
            Input::Down => 4,
            Input::Left => 8,
            _ => 0,
        };
        if bit != 0 {
            match phase {
                InputPhase::Pressed => {
                    self.held |= bit;
                    self.peak |= self.held;
                }
                InputPhase::Released => {
                    self.held &= !bit;
                    if self.held == 0 {
                        // A tap latches its group; tapping the latched group
                        // again lets it go. A hold that typed does neither.
                        if !self.typed_in_hold {
                            let tapped = direction_group(self.peak);
                            self.latched = if tapped == self.latched { None } else { tapped };
                        }
                        self.peak = 0;
                        self.typed_in_hold = false;
                    }
                }
            }
            self.announce_group(layout);
            return None;
        }
        if phase != InputPhase::Pressed {
            return None;
        }
        let slot = FACE_BUTTONS.iter().position(|&b| b == input)?;
        let group = self.group()?;
        let n = layout.layers.len().max(1);
        let key = layout.layers[self.layer % n].groups[group][slot];
        if self.held != 0 {
            self.typed_in_hold = true;
        }
        self.latched = None;
        self.typed_once = true;
        match key {
            ChordKey::Char(ch) => {
                let ch = if std::mem::take(&mut self.shift) {
                    ch.to_uppercase().next().unwrap_or(ch)
                } else {
                    ch
                };
                value.push(ch);
                self.announce = spoken_char(ch);
            }
            ChordKey::Space => {
                value.push(' ');
                self.announce = "Space".into();
            }
            ChordKey::Delete => {
                value.pop();
                self.announce = "Delete".into();
            }
            ChordKey::Shift => {
                self.shift = !self.shift;
                self.announce = if self.shift { "Shift on" } else { "Shift off" }.into();
            }
            ChordKey::Done => return Some(KeyboardAction::Done),
        }
        None
    }

    fn announce_group(&mut self, layout: &ChordLayout) {
        if let Some(g) = self.group() {
            let n = layout.layers.len().max(1);
            let keys = layout.layers[self.layer % n].groups[g];
            let names: Vec<String> = keys.iter().map(|&k| spoken_key(k)).collect();
            self.announce = format!("{}: {}", DIRECTIONS[g], names.join(", "));
        }
    }

    fn grid_input(
        &mut self,
        layout: &GridLayout,
        value: &mut String,
        input: Input,
        phase: InputPhase,
    ) -> Option<KeyboardAction> {
        if phase != InputPhase::Pressed {
            return None;
        }
        let page = &layout.pages[self.layer % layout.pages.len().max(1)];
        let rows = page.len() + 1;
        let row_len = |r: usize| {
            if r < page.len() {
                page[r].chars().count()
            } else {
                SPECIALS.len()
            }
        };
        let (mut r, mut c) = self.cursor;
        r = r.min(rows - 1);
        c = c.min(row_len(r) - 1);
        // Moving between rows keeps the key's place across the row.
        let across = |from: usize, to: usize, c: usize| -> usize {
            let (a, b) = (row_len(from).max(1), row_len(to).max(1));
            ((c as f32 + 0.5) * b as f32 / a as f32)
                .floor()
                .min(b as f32 - 1.0) as usize
        };
        match input {
            Input::Left => c = (c + row_len(r) - 1) % row_len(r),
            Input::Right => c = (c + 1) % row_len(r),
            Input::Up => {
                let to = (r + rows - 1) % rows;
                c = across(r, to, c);
                r = to;
            }
            Input::Down => {
                let to = (r + 1) % rows;
                c = across(r, to, c);
                r = to;
            }
            Input::A => {
                if r < page.len() {
                    let ch = page[r].chars().nth(c).unwrap_or(' ');
                    let ch = if self.caps {
                        ch.to_uppercase().next().unwrap_or(ch)
                    } else {
                        ch
                    };
                    value.push(ch);
                    self.announce = spoken_char(ch);
                } else {
                    match SPECIALS[c] {
                        Special::Shift => self.toggle_caps(),
                        Special::Page => self.turn_page(layout, true),
                        Special::Space => {
                            value.push(' ');
                            self.announce = "Space".into();
                        }
                        Special::Delete => {
                            value.pop();
                            self.announce = "Delete".into();
                        }
                        Special::Done => return Some(KeyboardAction::Done),
                    }
                }
                return None;
            }
            Input::B => {
                value.pop();
                self.announce = "Delete".into();
                return None;
            }
            Input::X => {
                value.push(' ');
                self.announce = "Space".into();
                return None;
            }
            Input::Y => {
                self.toggle_caps();
                return None;
            }
            Input::L2 | Input::R2 => {
                self.turn_page(layout, input == Input::R2);
                return None;
            }
            _ => return None,
        }
        self.cursor = (r, c);
        self.announce = if r < page.len() {
            spoken_char(page[r].chars().nth(c).unwrap_or(' '))
        } else {
            special_name(SPECIALS[c]).into()
        };
        None
    }

    fn toggle_caps(&mut self) {
        self.caps = !self.caps;
        self.announce = if self.caps {
            "Capitals on"
        } else {
            "Capitals off"
        }
        .into();
    }

    fn turn_page(&mut self, layout: &GridLayout, forward: bool) {
        let n = layout.pages.len().max(1);
        self.layer = if forward {
            (self.layer + 1) % n
        } else {
            (self.layer + n - 1) % n
        };
        self.announce = if self.layer == 0 {
            "Letters"
        } else {
            "Symbols"
        }
        .into();
    }
}

/// The group for a set of held directions, or none for no direction or an
/// impossible pair.
fn direction_group(held: u8) -> Option<usize> {
    Some(match held {
        1 => 0,
        3 => 1,
        2 => 2,
        6 => 3,
        4 => 4,
        12 => 5,
        8 => 6,
        9 => 7,
        _ => return None,
    })
}

fn spoken_char(c: char) -> String {
    match c {
        ' ' => "Space".into(),
        '.' => "Period".into(),
        ',' => "Comma".into(),
        '?' => "Question mark".into(),
        '!' => "Exclamation mark".into(),
        '\'' => "Apostrophe".into(),
        '-' => "Hyphen".into(),
        c if c.is_uppercase() => format!("Capital {c}"),
        c => c.to_string(),
    }
}

fn spoken_key(k: ChordKey) -> String {
    match k {
        ChordKey::Char(c) => spoken_char(c),
        ChordKey::Space => "Space".into(),
        ChordKey::Delete => "Delete".into(),
        ChordKey::Shift => "Shift".into(),
        ChordKey::Done => "Done".into(),
    }
}

fn special_name(s: Special) -> &'static str {
    match s {
        Special::Shift => "Capitals",
        Special::Page => "Symbols",
        Special::Space => "Space",
        Special::Delete => "Delete",
        Special::Done => "Done",
    }
}

const fn pt(x: f32, y: f32) -> Point {
    Point { x, y }
}

/// Control icons in a 24-point box, drawn with a 2-point stroke, and
/// 12-row bitmaps for pixel themes without icons of these names.
const SPACE_ICON: (&str, &[PathCmd], &[&str]) = (
    "space",
    &[
        PathCmd::MoveTo(pt(4.0, 10.0)),
        PathCmd::LineTo(pt(4.0, 16.0)),
        PathCmd::LineTo(pt(20.0, 16.0)),
        PathCmd::LineTo(pt(20.0, 10.0)),
    ],
    &[
        "............",
        "............",
        "............",
        "............",
        "............",
        "##........##",
        "##........##",
        "##........##",
        "############",
        "############",
        "............",
        "............",
    ],
);
const DELETE_ICON: (&str, &[PathCmd], &[&str]) = (
    "backspace",
    &[
        PathCmd::MoveTo(pt(9.0, 5.0)),
        PathCmd::LineTo(pt(21.0, 5.0)),
        PathCmd::LineTo(pt(21.0, 19.0)),
        PathCmd::LineTo(pt(9.0, 19.0)),
        PathCmd::LineTo(pt(2.0, 12.0)),
        PathCmd::Close,
        PathCmd::MoveTo(pt(12.0, 9.0)),
        PathCmd::LineTo(pt(17.0, 15.0)),
        PathCmd::MoveTo(pt(17.0, 9.0)),
        PathCmd::LineTo(pt(12.0, 15.0)),
    ],
    &[
        "............",
        "...#########",
        "..##.......#",
        ".##..#...#.#",
        "##....#.#..#",
        "#......#...#",
        "##....#.#..#",
        ".##..#...#.#",
        "..##.......#",
        "...#########",
        "............",
        "............",
    ],
);
const SHIFT_ICON: (&str, &[PathCmd], &[&str]) = (
    "shift",
    &[
        PathCmd::MoveTo(pt(12.0, 3.0)),
        PathCmd::LineTo(pt(21.0, 12.0)),
        PathCmd::LineTo(pt(16.0, 12.0)),
        PathCmd::LineTo(pt(16.0, 20.0)),
        PathCmd::LineTo(pt(8.0, 20.0)),
        PathCmd::LineTo(pt(8.0, 12.0)),
        PathCmd::LineTo(pt(3.0, 12.0)),
        PathCmd::Close,
    ],
    &[
        ".....##.....",
        "....####....",
        "...##..##...",
        "..##....##..",
        ".##......##.",
        "####....####",
        "...#....#...",
        "...#....#...",
        "...#....#...",
        "...######...",
        "............",
        "............",
    ],
);
const DONE_ICON: (&str, &[PathCmd], &[&str]) = (
    "check",
    &[
        PathCmd::MoveTo(pt(4.0, 12.0)),
        PathCmd::LineTo(pt(10.0, 18.0)),
        PathCmd::LineTo(pt(20.0, 6.0)),
    ],
    &[
        "............",
        "..........##",
        ".........##.",
        "........##..",
        ".......##...",
        "##....##....",
        ".##..##.....",
        "..####......",
        "...##.......",
        "............",
        "............",
        "............",
    ],
);

/// Draws one chord key centered in a `side`-sized box: a character in
/// `style`, or a control's icon at the style's size.
fn key_face<Msg>(
    cx: &mut BuildCx<Msg>,
    key: ChordKey,
    shift: bool,
    style: crate::contract::TextStyle,
    ink: Color,
) {
    let icon = match key {
        ChordKey::Char(ch) => {
            let ch = if shift {
                ch.to_uppercase().next().unwrap_or(ch)
            } else {
                ch
            };
            let mut text = text_node(glyph(ch), style, ink);
            text.sem = None;
            cx.leaf(text);
            return;
        }
        ChordKey::Space => SPACE_ICON,
        ChordKey::Delete => DELETE_ICON,
        ChordKey::Shift => SHIFT_ICON,
        ChordKey::Done => DONE_ICON,
    };
    let (name, paths, bits) = icon;
    let side = (style.size * 1.1).round();
    let node = crate::lab::named_icon(cx, (name, bits), paths.to_vec(), side, ink, false);
    cx.leaf(node);
}

/// The cue for one keyboard input: typing and pressing keys select,
/// choosing groups and layers moves, finishing succeeds, cancelling goes
/// back, and a key press that changes nothing is blocked.
fn feedback(
    (was, had): &(KeyboardState, String),
    now: &KeyboardState,
    value: &str,
    action: Option<KeyboardAction>,
    input: Input,
    phase: InputPhase,
) -> Option<crate::runtime::FeedbackKind> {
    use crate::runtime::FeedbackKind as F;
    match action {
        Some(KeyboardAction::Done) => return Some(F::Success),
        Some(KeyboardAction::Cancel) => return Some(F::Back),
        Some(KeyboardAction::Switched(_)) => return Some(F::Select),
        None => {}
    }
    if value != had || now.shift != was.shift || now.caps != was.caps {
        return Some(F::Select);
    }
    if now.group() != was.group() || now.layer != was.layer || now.cursor != was.cursor {
        return Some(F::Move);
    }
    let key_press = phase == InputPhase::Pressed
        && (FACE_BUTTONS.contains(&input) || matches!(input, Input::Backspace | Input::Delete));
    key_press.then_some(F::Blocked)
}

/// A text field with an on-screen keyboard under it, for gamepads and
/// remotes (`OnScreenKeyboard`). The app keeps the [`KeyboardState`] and
/// the text, and replaces both from each [`KeyboardUpdate`]. While the
/// field has focus, the gamepad types; Next and Previous still move focus.
pub struct OnScreenKeyboard<Msg> {
    base: Base,
    label: String,
    value: String,
    state: KeyboardState,
    chord: Rc<ChordLayout>,
    grid: Rc<GridLayout>,
    on_change: Option<Rc<dyn Fn(KeyboardUpdate) -> Msg>>,
    faces: Option<FaceLayout>,
}

/// Each face button's place, in `FACE_BUTTONS` order, as (x, y) steps from
/// a diamond's middle (y grows downward), so a group's keys sit where the
/// buttons that press them sit and no key needs a button label.
fn offsets(layout: FaceLayout) -> [(f32, f32); 4] {
    match layout {
        FaceLayout::Xbox => [(0.0, 1.0), (1.0, 0.0), (-1.0, 0.0), (0.0, -1.0)],
        FaceLayout::Nintendo | FaceLayout::Unknown => {
            [(1.0, 0.0), (0.0, 1.0), (0.0, -1.0), (-1.0, 0.0)]
        }
    }
}

impl<Msg> OnScreenKeyboard<Msg> {
    /// A keyboard for the field named `label`, showing `value`.
    pub fn new(label: impl Into<String>, value: impl Into<String>, state: KeyboardState) -> Self {
        OnScreenKeyboard {
            base: Base::default(),
            label: label.into(),
            value: value.into(),
            state,
            chord: Rc::new(ChordLayout::english()),
            grid: Rc::new(GridLayout::qwerty()),
            on_change: None,
            faces: None,
        }
    }
    /// Where the gamepad's face buttons sit, in place of the shell's
    /// `Preferences::face_layout`.
    pub fn face_layout(mut self, faces: FaceLayout) -> Self {
        self.faces = Some(faces);
        self
    }
    pub fn chord_layout(mut self, layout: ChordLayout) -> Self {
        self.chord = Rc::new(layout);
        self
    }
    pub fn grid_layout(mut self, layout: GridLayout) -> Self {
        self.grid = Rc::new(layout);
        self
    }
    pub fn on_change(mut self, on_change: impl Fn(KeyboardUpdate) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(on_change));
        self
    }
}

base_props!(OnScreenKeyboard<Msg>);

impl<Msg: 'static> Build<Msg> for OnScreenKeyboard<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let snap = |v: f32| match cx.theme.grid {
            Some(cell) => (v / cell).round().max(1.0) * cell,
            None => v.round(),
        };
        cx.key(self.base.id.as_deref());
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::CENTER,
            gap: Size {
                width: len(0.0),
                height: len(snap(t.space.s2)),
            },
            ..Style::default()
        }));
        // The field: the focus stop that takes the gamepad, and typed text.
        let body = type_style(t.font.body, FontFamily::Body);
        let field_radius = radii(t.radius.element);
        let bw = t.size.border_width;
        let mut sem = Semantics::new(Role::TextInput, self.label.clone());
        sem.value = Some(self.value.clone());
        sem.description = Some(
            match self.state.kind {
                KeyboardKind::Chord => "Hold or tap a direction, then press A, B, X, or Y",
                KeyboardKind::Grid => "Move with the D-pad, press A to type",
            }
            .into(),
        );
        let text_ix = cx.nodes.len() as u32 + 1;
        let mut interact = Interact::new(None, false);
        let on_change = self.on_change.clone();
        if let Some(f) = on_change.clone() {
            let state = self.state.clone();
            interact.edit = Some(Edit {
                value: self.value.clone(),
                on_change: Some(Rc::new(move |value| {
                    f(KeyboardUpdate {
                        state: state.clone(),
                        value,
                        action: None,
                    })
                })),
                effect: None,
                text_node: text_ix,
                masked: false,
                multiline: false,
                caret: c.text_primary.into(),
            });
        }
        let mut field = Node::new(Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            size: Size {
                width: Dimension::percent(1.0),
                height: Dimension::auto(),
            },
            min_size: Size {
                width: auto(),
                height: length(t.size.element_md),
            },
            padding: taffy::Rect {
                left: len(t.space.s3 - bw),
                right: len(t.space.s3 - bw),
                top: len(0.0),
                bottom: len(0.0),
            },
            border: taffy::Rect {
                left: len(bw),
                right: len(bw),
                top: len(bw),
                bottom: len(bw),
            },
            overflow: taffy::Point {
                x: taffy::Overflow::Hidden,
                y: taffy::Overflow::Visible,
            },
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw {
            border: Some((c.border.into(), bw)),
            ..BoxDraw::fill(c.background_surface.into(), field_radius)
        }))
        .interact(interact)
        .indicator(Indicator {
            ring: None,
            radii: field_radius,
            overlay: None,
            field: Some((c.accent.into(), c.accent_muted.into())),
            hold_fill: None,
        })
        .sem(sem);
        if let Some(f) = on_change.clone() {
            let (state, value, chord, grid) = (
                self.state.clone(),
                self.value.clone(),
                self.chord.clone(),
                self.grid.clone(),
            );
            let capture_change = f.clone();
            // Inputs can arrive faster than the view rebuilds, so the
            // closure carries the latest state forward itself.
            let latest = Rc::new(std::cell::RefCell::new((state, value)));
            field.capture = Some(Rc::new(move |input: Input, phase: InputPhase, now: f64| {
                if matches!(input, Input::Next | Input::Previous) {
                    return (None, None);
                }
                let mut latest = latest.borrow_mut();
                let (state, value) = &mut *latest;
                let before = (state.clone(), value.clone());
                let action = state.input(&chord, &grid, value, input, phase, now);
                let cue = feedback(&before, state, value, action, input, phase);
                if action.is_none() && (state.clone(), value.clone()) == before {
                    return (None, cue);
                }
                let update = KeyboardUpdate {
                    state: state.clone(),
                    value: value.clone(),
                    action,
                };
                (Some(capture_change(update)), cue)
            }));
            // The toast hides on time.
            if let Some((_, until)) = &self.state.toast {
                let mut cleared = self.state.clone();
                cleared.toast = None;
                field.timer = Some((
                    *until,
                    f(KeyboardUpdate {
                        state: cleared,
                        value: self.value.clone(),
                        action: None,
                    }),
                ));
            }
        }
        cx.begin(field);
        let mut text = text_node(self.value.clone(), body, c.text_primary.into());
        text_draw(&mut text).max_lines = Some(1);
        text.style.flex_shrink = 0.0;
        cx.leaf(text);
        cx.end();
        cx.live_region(
            Some(self.state.announce.as_str()).filter(|s| !s.is_empty()),
            Live::Polite,
        );
        match self.state.kind {
            KeyboardKind::Chord => {
                let faces = self.faces.unwrap_or(cx.prefs.face_layout);
                chord_view(cx, &self.state, &self.chord, faces, &snap)
            }
            KeyboardKind::Grid => grid_view(cx, &self.state, &self.grid, &snap),
        }
        if let Some((name, _)) = &self.state.toast {
            toast(cx, name);
        }
        cx.end();
    }
}

/// One keycap: a padded face, the accent when `on`, centered text.
fn cap<Msg>(
    cx: &mut BuildCx<Msg>,
    glyph: &str,
    w: f32,
    h: f32,
    at: Option<(f32, f32)>,
    on: bool,
    big: bool,
) {
    let t = cx.tokens;
    let c = &t.color;
    let (fill, ink): (Color, Color) = if on {
        (c.accent.into(), c.on_accent.into())
    } else {
        (c.neutral.into(), c.text_primary.into())
    };
    let mut style = type_style(
        if big { t.font.label } else { t.font.supporting },
        FontFamily::Body,
    );
    style.weight = t.font.weight.medium;
    let position = match at {
        Some((x, y)) => Style {
            position: Position::Absolute,
            inset: taffy::Rect {
                left: length(x),
                top: length(y),
                right: auto(),
                bottom: auto(),
            },
            ..Style::default()
        },
        None => Style::default(),
    };
    cx.begin(
        Node::new(Style {
            size: Size {
                width: length(w),
                height: length(h),
            },
            align_items: AlignItems::CENTER,
            justify_content: JustifyContent::CENTER,
            flex_shrink: 0.0,
            ..position
        })
        .draw(Draw::Box(BoxDraw::fill(fill, radii(t.radius.inner)))),
    );
    let mut text = text_node(glyph.to_string(), style, ink);
    text.sem = None;
    cx.leaf(text);
    cx.end();
}

fn glyph(c: char) -> String {
    if c == ' ' {
        "␣".into()
    } else {
        c.to_string()
    }
}

/// Where each group sits in the 3×3 grid, as (column, row), matching the
/// D-pad: up is top middle, up right is top right, and so on.
const GRID_CELLS: [(f32, f32); 8] = [
    (1.0, 0.0),
    (2.0, 0.0),
    (2.0, 1.0),
    (2.0, 2.0),
    (1.0, 2.0),
    (0.0, 2.0),
    (0.0, 1.0),
    (0.0, 0.0),
];

/// A box at an absolute place in its parent.
fn placed(x: f32, y: f32, w: f32, h: f32) -> Style {
    Style {
        position: Position::Absolute,
        inset: taffy::Rect {
            left: length(x),
            top: length(y),
            right: auto(),
            bottom: auto(),
        },
        size: Size {
            width: length(w),
            height: length(h),
        },
        align_items: AlignItems::CENTER,
        justify_content: JustifyContent::CENTER,
        ..Style::default()
    }
}

/// One key, centered in a `side`-sized square.
#[allow(clippy::too_many_arguments)]
fn key_at<Msg>(
    cx: &mut BuildCx<Msg>,
    key: ChordKey,
    shift: bool,
    x: f32,
    y: f32,
    side: f32,
    style: crate::contract::TextStyle,
    ink: Color,
) {
    cx.begin(Node::new(placed(x, y, side, side)));
    key_face(cx, key, shift, style, ink);
    cx.end();
}

/// The chord keyboard: a 3×3 grid placed as the D-pad is, each cell showing
/// its group's four characters as a small diamond, with the layer's name
/// in the middle. A held or latched group grows into one large diamond
/// whose characters carry their A, B, X, Y buttons, and the rest dim.
fn chord_view<Msg>(
    cx: &mut BuildCx<Msg>,
    state: &KeyboardState,
    layout: &ChordLayout,
    faces: FaceLayout,
    snap: &dyn Fn(f32) -> f32,
) {
    let t = cx.tokens;
    let c = &t.color;
    let layer = state.layer % layout.layers.len().max(1);
    let group = state.group();
    let body = type_style(t.font.body, FontFamily::Body);
    let row = snap((body.size * 1.1).ceil());
    let pad = snap(3.0);
    let gap = snap(4.0);
    // The side letters sit between the top and bottom ones, so rows can
    // overlap by a quarter without their glyphs meeting.
    let (cw, ch) = (snap(3.4 * row) + 2.0 * pad, snap(2.5 * row) + 2.0 * pad);
    let (side_w, side) = (3.0 * cw + 2.0 * gap, 3.0 * ch + 2.0 * gap);
    let radius = radii(t.radius.element);
    let surface = |on: bool| -> BoxDraw {
        let (fill, edge): (Color, Color) = if on {
            (c.accent_muted.into(), c.accent.into())
        } else {
            (c.background_surface.into(), c.border.into())
        };
        BoxDraw {
            border: Some((edge, t.size.border_width)),
            ..BoxDraw::fill(fill, radius)
        }
    };
    cx.begin(Node::new(Style {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::CENTER,
        justify_content: JustifyContent::CENTER,
        gap: Size {
            width: len(snap(t.space.s4)),
            height: len(0.0),
        },
        size: Size {
            width: Dimension::percent(1.0),
            height: auto(),
        },
        ..Style::default()
    }));
    // The grid.
    cx.begin(Node::new(Style {
        size: Size {
            width: length(side_w),
            height: length(side),
        },
        flex_shrink: 0.0,
        ..Style::default()
    }));
    let dim = |g: usize| {
        if group.is_some_and(|held| held != g) {
            0.35
        } else {
            1.0
        }
    };
    for (g, &(col, rw)) in GRID_CELLS.iter().enumerate() {
        let (x, y) = (col * (cw + gap), rw * (ch + gap));
        cx.begin(
            Node::new(placed(x, y, cw, ch))
                .draw(Draw::Box(surface(false)))
                .opacity(dim(g)),
        );
        let chars = layout.layers[layer].groups[g];
        let (mid_x, mid_y) = ((cw - row) / 2.0, (ch - row) / 2.0);
        let (reach_x, reach_y) = (mid_x - pad, mid_y - pad);
        // Each key sits where its face button sits.
        for (key, (dx, dy)) in chars.iter().zip(offsets(faces)) {
            let (lx, ly) = (mid_x + dx * reach_x, mid_y + dy * reach_y);
            let ink: Color = if *key == ChordKey::Shift && state.shift {
                c.accent.into()
            } else {
                c.text_primary.into()
            };
            key_at(cx, *key, state.shift, lx, ly, row, body, ink);
        }
        cx.end();
    }
    // The middle: the layer's name.
    let label = type_style(t.font.label, FontFamily::Body);
    cx.begin(
        Node::new(placed(cw + gap, ch + gap, cw, ch)).opacity(if group.is_some() {
            0.35
        } else {
            1.0
        }),
    );
    let mut name = text_node(layout.layers[layer].name, label, c.text_secondary.into());
    name.sem = None;
    cx.leaf(name);
    cx.end();
    // The held group, grown over the grid toward its own cell.
    if let Some(g) = group {
        let (big_w, big_h) = (2.0 * cw + gap, 2.0 * ch + gap);
        let (col, rw) = GRID_CELLS[g];
        let at = |v: f32, cell: f32, all: f32, big: f32| {
            ((v - 1.0) * (cell + gap) / 2.0 + (all - big) / 2.0).clamp(0.0, all - big)
        };
        big_diamond(
            cx,
            layout.layers[layer].groups[g],
            state.shift,
            faces,
            (
                snap(at(col, cw, side_w, big_w)),
                snap(at(rw, ch, side, big_h)),
            ),
            (big_w, big_h),
            snap,
        );
    }
    cx.end();
    // Beside the grid: the hints.
    cx.begin(Node::new(Style {
        flex_direction: FlexDirection::Column,
        gap: Size {
            width: len(0.0),
            height: len(gap),
        },
        flex_shrink: 1.0,
        flex_grow: 0.0,
        min_size: Size {
            width: length(0.0),
            height: auto(),
        },
        ..Style::default()
    }));
    Build::build(
        Box::new(
            crate::lab::KeyHints::new()
                .key("L1 R1", "Layer")
                .key("Start", "Done")
                .key("Select", "Cancel")
                .key("L1+R1", "Keyboard"),
        ),
        cx,
    );
    // Until the first key, one line says how a key is chosen.
    if !state.typed_once {
        let mut first = text_node(
            "Press the button in the same spot",
            type_style(t.font.supporting, FontFamily::Body),
            c.text_secondary.into(),
        );
        first.sem = None;
        cx.leaf(first);
    }
    cx.end();
    cx.end();
}

/// One group large: its four characters as a diamond, each beside the
/// keycap of its face button.
#[allow(clippy::too_many_arguments)]
fn big_diamond<Msg>(
    cx: &mut BuildCx<Msg>,
    keys: [ChordKey; 4],
    shift: bool,
    faces: FaceLayout,
    (x, y): (f32, f32),
    (w, h): (f32, f32),
    snap: &dyn Fn(f32) -> f32,
) {
    let t = cx.tokens;
    let c = &t.color;
    let mut style = type_style(t.font.display3, FontFamily::Body);
    style.weight = t.font.weight.medium;
    cx.begin(Node::new(placed(x, y, w, h)).draw(Draw::Box(BoxDraw {
        border: Some((c.accent.into(), t.size.border_width)),
        ..BoxDraw::fill(c.background_surface.into(), radii(t.radius.container))
    })));
    let slot = (w.min(h) / 3.0).floor();
    // As large as a slot allows, up to the display size.
    style.size = style.size.min((slot / style.leading).floor());
    for (key, (dx, dy)) in keys.iter().zip(offsets(faces)) {
        // Inside the border, with room for a pixel font's larger cells.
        let inset = snap(t.space.s2);
        let sx = (w - slot) / 2.0 + dx * ((w - slot) / 2.0 - inset);
        let sy = (h - slot) / 2.0 + dy * ((h - slot) / 2.0 - inset);
        cx.begin(Node::new(placed(snap(sx), snap(sy), slot, slot)));
        let ink: Color = if *key == ChordKey::Shift && shift {
            c.accent.into()
        } else {
            c.text_primary.into()
        };
        key_face(cx, *key, shift, style, ink);
        cx.end();
    }
    cx.end();
}

/// Rows of keys with the special row under them; the highlighted key is in
/// the accent.
fn grid_view<Msg>(
    cx: &mut BuildCx<Msg>,
    state: &KeyboardState,
    layout: &GridLayout,
    snap: &dyn Fn(f32) -> f32,
) {
    let page = &layout.pages[state.layer % layout.pages.len().max(1)];
    let key = snap(34.0);
    let gap = snap(4.0);
    let row = |cx: &mut BuildCx<Msg>| {
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Row,
            gap: Size {
                width: len(gap),
                height: len(0.0),
            },
            ..Style::default()
        }));
    };
    cx.begin(Node::new(Style {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::CENTER,
        gap: Size {
            width: len(0.0),
            height: len(gap),
        },
        ..Style::default()
    }));
    for (r, keys) in page.iter().enumerate() {
        row(cx);
        for (col, ch) in keys.chars().enumerate() {
            let ch = if state.caps {
                ch.to_uppercase().next().unwrap_or(ch)
            } else {
                ch
            };
            cap(
                cx,
                &glyph(ch),
                key,
                key,
                None,
                state.cursor == (r, col),
                false,
            );
        }
        cx.end();
    }
    row(cx);
    let last = page.len();
    for (col, special) in SPECIALS.iter().enumerate() {
        let (label, wide) = match special {
            Special::Shift => (if state.caps { "ABC" } else { "abc" }, 2.0),
            Special::Page => (if state.layer == 0 { "123" } else { "abc" }, 2.0),
            Special::Space => ("Space", 4.0),
            Special::Delete => ("Delete", 2.0),
            Special::Done => ("Done", 2.0),
        };
        let w = key * wide + gap * (wide - 1.0);
        cap(cx, label, w, key, None, state.cursor == (last, col), false);
    }
    cx.end();
    cx.end();
    Build::build(
        Box::new(
            crate::lab::KeyHints::new()
                .key("B", "Delete")
                .key("X", "Space")
                .key("Y", "Capitals")
                .key("L1 R1", "Symbols")
                .key("Start", "Done")
                .key("L1+R1", "Keyboard"),
        ),
        cx,
    );
}

/// A brief pill naming the new layout.
fn toast<Msg>(cx: &mut BuildCx<Msg>, name: &str) {
    let t = cx.tokens;
    let mut style = type_style(t.font.supporting, FontFamily::Body);
    style.weight = t.font.weight.medium;
    // Over the bottom of the keyboard, so showing it moves nothing.
    cx.begin(Node::new(Style {
        position: Position::Absolute,
        inset: taffy::Rect {
            left: length(0.0),
            right: length(0.0),
            top: auto(),
            bottom: length(t.space.s2),
        },
        justify_content: JustifyContent::CENTER,
        ..Style::default()
    }));
    cx.begin(
        Node::new(Style {
            padding: taffy::Rect {
                left: len(t.space.s3),
                right: len(t.space.s3),
                top: len(t.space.s1),
                bottom: len(t.space.s1),
            },
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw::fill(
            t.color.background_inverted.into(),
            radii(t.radius.full),
        ))),
    );
    let mut text = text_node(name.to_string(), style, t.color.on_dark.into());
    text.sem = None;
    cx.leaf(text);
    cx.end();
    cx.end();
}

#[cfg(test)]
mod tests {
    use super::*;
    use InputPhase::{Pressed, Released};

    struct Pad {
        s: KeyboardState,
        v: String,
        chord: ChordLayout,
        grid: GridLayout,
        now: f64,
    }

    impl Pad {
        fn new(kind: KeyboardKind) -> Self {
            Pad {
                s: KeyboardState::new(kind),
                v: String::new(),
                chord: ChordLayout::english(),
                grid: GridLayout::qwerty(),
                now: 0.0,
            }
        }
        fn send(&mut self, input: Input, phase: InputPhase) -> Option<KeyboardAction> {
            self.now += 10.0;
            self.s
                .input(&self.chord, &self.grid, &mut self.v, input, phase, self.now)
        }
        fn tap(&mut self, input: Input) -> Option<KeyboardAction> {
            let a = self.send(input, Pressed);
            self.send(input, Released).or(a)
        }
    }

    #[test]
    fn every_layer_keeps_the_controls_in_the_down_group() {
        let layout = ChordLayout::english();
        assert_eq!(
            layout.layers.iter().map(|l| l.name).collect::<Vec<_>>(),
            ["abc", "ABC", "123", "#+="]
        );
        for l in &layout.layers {
            assert_eq!(l.groups[4], CONTROLS, "{}", l.name);
        }
        let mut letters: Vec<char> = layout.layers[0]
            .groups
            .iter()
            .flatten()
            .filter_map(|k| match k {
                ChordKey::Char(c) if c.is_ascii_lowercase() => Some(*c),
                _ => None,
            })
            .collect();
        letters.sort();
        assert_eq!(
            letters.iter().collect::<String>(),
            "abcdefghijklmnopqrstuvwxyz"
        );
    }

    #[test]
    fn holding_a_direction_and_pressing_a_face_button_types() {
        let mut p = Pad::new(KeyboardKind::Chord);
        p.send(Input::Up, Pressed);
        assert_eq!(p.s.group(), Some(0));
        p.tap(Input::A);
        p.tap(Input::B);
        p.send(Input::Up, Released);
        assert_eq!(p.v, "eo");
        assert_eq!(p.s.group(), None, "a hold that typed does not latch");
        // A diagonal: up and right together.
        p.send(Input::Up, Pressed);
        p.send(Input::Right, Pressed);
        assert_eq!(p.s.group(), Some(1));
        p.tap(Input::A);
        p.send(Input::Up, Released);
        p.send(Input::Right, Released);
        assert_eq!(p.v, "eou");
    }

    #[test]
    fn tapping_a_direction_latches_its_group() {
        let mut p = Pad::new(KeyboardKind::Chord);
        p.tap(Input::Left);
        assert_eq!(p.s.group(), Some(6));
        assert_eq!(p.s.announce, "left: a, n, r, c");
        p.tap(Input::X);
        assert_eq!(p.v, "r");
        assert_eq!(p.s.group(), None, "typing lets the latch go");
        p.tap(Input::Right);
        p.tap(Input::Right);
        assert_eq!(
            p.s.group(),
            None,
            "tapping the latched group again lets it go"
        );
    }

    #[test]
    fn the_control_group_spaces_deletes_shifts_and_finishes() {
        let mut p = Pad::new(KeyboardKind::Chord);
        p.v = "hi".into();
        p.tap(Input::Down);
        assert_eq!(p.s.announce, "down: Space, Delete, Shift, Done");
        p.tap(Input::A);
        assert_eq!(p.v, "hi ");
        p.tap(Input::Down);
        p.tap(Input::B);
        assert_eq!(p.v, "hi");
        // Shift capitalizes the next letter only.
        p.tap(Input::Down);
        p.tap(Input::X);
        assert!(p.s.shift);
        p.tap(Input::Up);
        p.tap(Input::A);
        p.tap(Input::Up);
        p.tap(Input::A);
        assert_eq!(p.v, "hiEe");
        p.tap(Input::Down);
        assert_eq!(p.tap(Input::Y), Some(KeyboardAction::Done));
        assert_eq!(p.tap(Input::Start), Some(KeyboardAction::Done));
        assert_eq!(p.tap(Input::Select), Some(KeyboardAction::Cancel));
    }

    #[test]
    fn l1_and_r1_cycle_layers_and_together_switch_layouts() {
        let mut p = Pad::new(KeyboardKind::Chord);
        p.tap(Input::R1);
        assert_eq!(p.s.layer, 1);
        p.tap(Input::Up);
        p.tap(Input::A);
        assert_eq!(p.v, "E");
        p.tap(Input::R1);
        p.tap(Input::R1);
        assert_eq!(p.s.layer, 3);
        p.tap(Input::R1);
        assert_eq!(p.s.layer, 0, "wraps to lowercase");
        p.tap(Input::L1);
        assert_eq!(p.s.layer, 3);
        assert_eq!(p.s.announce, "#+=");
        p.tap(Input::L2);
        p.tap(Input::R2);
        assert_eq!(p.s.layer, 3, "L2 and R2 do nothing on the chord keyboard");
        // Both together: no layer change, the next layout.
        p.send(Input::L1, Pressed);
        let a = p.send(Input::R1, Pressed);
        assert_eq!(a, Some(KeyboardAction::Switched(KeyboardKind::Grid)));
        p.send(Input::L1, Released);
        p.send(Input::R1, Released);
        assert_eq!((p.s.kind, p.s.layer), (KeyboardKind::Grid, 0));
        assert_eq!(
            p.s.toast.as_ref().map(|t| t.0.as_str()),
            Some("Grid keyboard")
        );
        assert_eq!(p.v, "E");
        // Holding R1, then pressing L1, is a chord too.
        p.send(Input::R1, Pressed);
        p.now += 500.0;
        let a = p.send(Input::L1, Pressed);
        assert_eq!(a, Some(KeyboardAction::Switched(KeyboardKind::Chord)));
        p.send(Input::R1, Released);
        p.send(Input::L1, Released);
        assert_eq!(p.s.layer, 0);
    }

    #[test]
    fn the_grid_moves_types_and_finishes() {
        let mut p = Pad::new(KeyboardKind::Grid);
        p.tap(Input::Right);
        p.tap(Input::A);
        assert_eq!(p.v, "w");
        p.tap(Input::Y);
        p.tap(Input::Down);
        p.tap(Input::A);
        assert_eq!(p.v, "wS");
        p.tap(Input::B);
        p.tap(Input::X);
        assert_eq!(p.v, "w ");
        p.tap(Input::R1);
        assert_eq!(p.s.layer, 1, "R1 turns to symbols");
        p.tap(Input::L1);
        // Down to the special row, right to Done.
        p.tap(Input::Down);
        p.tap(Input::Down);
        for _ in 0..4 {
            p.tap(Input::Right);
        }
        assert_eq!(p.tap(Input::A), Some(KeyboardAction::Done));
    }
}
