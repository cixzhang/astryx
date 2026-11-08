//! OnScreenKeyboard: text entry with a gamepad or remote. Experimental:
//! Astryx has no on-screen keyboard spec yet, so it is public only through
//! `astryx_core::lab`.
//!
//! Two layouts, switched with L1 and R1 pressed together:
//!
//! - **Chord** (the default), in the style of a daisy wheel: the D-pad's
//!   eight directions choose one of eight groups in a ring, and A, B, X,
//!   or Y types one of the group's four characters, laid out as the face
//!   buttons are. Hold a direction and press a face button, or tap a
//!   direction to latch its group and then press a face button. Three
//!   layers (lowercase, uppercase, numbers and symbols) hold 32 characters
//!   each: R2 moves to the next layer and L2 to the one before; holding L2
//!   or R2 while typing uses that layer for those characters only. L1
//!   deletes, R1 types a space, Start is done, Select cancels.
//! - **Grid**: rows of keys. The D-pad moves between keys, A types the
//!   highlighted one, B deletes, X types a space, Y toggles capitals, L2
//!   and R2 switch between letters and symbols, Start is done, Select
//!   cancels.
//!
//! L1 and R1 pressed together cycle layouts instead of deleting or adding
//! a space. L1 or R1 alone acts when it is released, so the two count as
//! together whenever both are down at once: pressed within a few
//! milliseconds of each other, or one pressed while the other is held. A brief
//! toast names the new layout, and the app hears the choice, so it can
//! keep it as the person's preference.
//!
//! Layouts are data: [`ChordLayout`] and [`GridLayout`] hold the
//! characters, so other languages can supply their own. A keyboard on the
//! device, a screen reader, or an input method types into the field
//! directly.

use super::text::{text_draw, text_node, type_style};
use super::{len, radii};
use crate::contract::{Color, FontFamily, Input, InputPhase, Live, Role};
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

/// The face buttons in the order of a group's characters: A (bottom), B
/// (right), X (left), Y (top).
pub const FACE_BUTTONS: [Input; 4] = [Input::A, Input::B, Input::X, Input::Y];

/// One layer of a chord layout: eight groups of four characters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChordLayer {
    /// A short name shown in the middle of the ring, such as "abc".
    pub name: &'static str,
    pub groups: [[char; 4]; 8],
}

/// The layers of a chord keyboard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChordLayout {
    pub layers: Vec<ChordLayer>,
}

/// A layer from 32 characters: groups in `DIRECTIONS` order, each four in
/// `FACE_BUTTONS` order.
const fn layer(name: &'static str, chars: [char; 32]) -> ChordLayer {
    let mut groups = [[' '; 4]; 8];
    let mut i = 0;
    while i < 32 {
        groups[i / 4][i % 4] = chars[i];
        i += 1;
    }
    ChordLayer { name, groups }
}

impl ChordLayout {
    /// English, by letter frequency: the most common letters sit on the
    /// four straight directions and on A, the easiest press; the rarest on
    /// the diagonals and Y.
    pub fn english() -> Self {
        // Straight directions (up, right, down, left) take e t a o on A,
        // i n s h on B, r d l c on X, u m w f on Y; the diagonals take the
        // rest of the alphabet and punctuation.
        let lower = [
            'e', 'i', 'r', 'u', //
            'g', 'v', 'q', '?', //
            't', 'n', 'd', 'm', //
            'y', 'k', 'z', '\'', //
            'a', 's', 'l', 'w', //
            'p', 'j', '.', '!', //
            'o', 'h', 'c', 'f', //
            'b', 'x', ',', '-',
        ];
        let upper = lower.map(|c| c.to_ascii_uppercase());
        let symbols = [
            '1', '5', '9', '$', //
            '(', '/', '"', '[', //
            '2', '6', '0', '%', //
            ')', '+', '<', ']', //
            '3', '7', '@', '&', //
            ':', '=', '>', '{', //
            '4', '8', '#', '*', //
            ';', '_', '~', '}',
        ];
        ChordLayout {
            layers: vec![
                layer("abc", lower),
                layer("ABC", upper),
                layer("123", symbols),
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
    /// A group chosen by a tap, kept until a character is typed.
    latched: Option<usize>,
    /// L2 or R2 held, and whether a character was typed while held.
    layer_held: Option<(Input, bool)>,
    /// L1 and R1: when each went down, while held; and whether the pair
    /// made a chord since both were last up.
    l1: Option<f64>,
    r1: Option<f64>,
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

    /// The chord layer in use for the next character.
    fn chord_layer(&self, layout: &ChordLayout) -> usize {
        let n = layout.layers.len().max(1);
        match self.layer_held {
            Some((Input::R2, _)) => (self.layer + 1) % n,
            Some((Input::L2, _)) => (self.layer + n - 1) % n,
            _ => self.layer % n,
        }
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
        if let Some(action) = self.shoulders(value, input, phase, now) {
            return Some(action);
        }
        if matches!(input, Input::L1 | Input::R1) {
            return None;
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
            KeyboardKind::Grid => return self.grid_input(grid, value, input, phase),
        }
        None
    }

    /// L1 and R1: alone, delete and space when released; together, the
    /// next layout.
    fn shoulders(
        &mut self,
        value: &mut String,
        input: Input,
        phase: InputPhase,
        now: f64,
    ) -> Option<KeyboardAction> {
        let (mine, other) = match input {
            Input::L1 => (&mut self.l1, self.r1),
            Input::R1 => (&mut self.r1, self.l1),
            _ => return None,
        };
        match phase {
            InputPhase::Pressed => {
                if mine.is_some() {
                    return None;
                }
                *mine = Some(now);
                if other.is_some() && !self.chorded {
                    self.chorded = true;
                    self.kind = self.kind.next();
                    self.layer = 0;
                    self.latched = None;
                    self.cursor = (0, 0);
                    self.toast = Some((self.kind.name().into(), now + TOAST_MS));
                    self.announce = self.kind.name().into();
                    return Some(KeyboardAction::Switched(self.kind));
                }
                None
            }
            InputPhase::Released => {
                mine.take()?;
                if self.chorded {
                    if self.l1.is_none() && self.r1.is_none() {
                        self.chorded = false;
                    }
                    return None;
                }
                if input == Input::L1 {
                    value.pop();
                    self.announce = "Delete".into();
                } else {
                    value.push(' ');
                    self.announce = "Space".into();
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
    ) {
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
            return;
        }
        match (input, phase) {
            (Input::L2 | Input::R2, InputPhase::Pressed) => {
                if self.layer_held.is_none() {
                    self.layer_held = Some((input, false));
                }
            }
            (Input::L2 | Input::R2, InputPhase::Released) => {
                // A tap moves to the next or previous layer; a hold that
                // typed was a shift.
                if let Some((held, typed)) = self.layer_held.take() {
                    if held == input && !typed {
                        let n = layout.layers.len().max(1);
                        self.layer = if input == Input::R2 {
                            (self.layer + 1) % n
                        } else {
                            (self.layer + n - 1) % n
                        };
                        self.announce = layout.layers[self.layer].name.into();
                    }
                }
            }
            (_, InputPhase::Pressed) => {
                let Some(slot) = FACE_BUTTONS.iter().position(|&b| b == input) else {
                    return;
                };
                let Some(group) = self.group() else {
                    return;
                };
                let ch = layout.layers[self.chord_layer(layout)].groups[group][slot];
                value.push(ch);
                self.announce = spoken_char(ch);
                if self.held != 0 {
                    self.typed_in_hold = true;
                }
                self.latched = None;
                if let Some((_, typed)) = self.layer_held.as_mut() {
                    *typed = true;
                }
            }
            _ => {}
        }
    }

    fn announce_group(&mut self, layout: &ChordLayout) {
        if let Some(g) = self.group() {
            let chars = layout.layers[self.chord_layer(layout)].groups[g];
            let names: Vec<String> = chars.iter().map(|&c| spoken_char(c)).collect();
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
                        Special::Page => self.next_page(layout),
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
                self.next_page(layout);
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

    fn next_page(&mut self, layout: &GridLayout) {
        self.layer = (self.layer + 1) % layout.pages.len().max(1);
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

fn special_name(s: Special) -> &'static str {
    match s {
        Special::Shift => "Capitals",
        Special::Page => "Symbols",
        Special::Space => "Space",
        Special::Delete => "Delete",
        Special::Done => "Done",
    }
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
        }
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
                on_change: Rc::new(move |value| {
                    f(KeyboardUpdate {
                        state: state.clone(),
                        value,
                        action: None,
                    })
                }),
                text_node: text_ix,
                masked: false,
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
                    return None;
                }
                let mut latest = latest.borrow_mut();
                let (state, value) = &mut *latest;
                let before = (state.clone(), value.clone());
                let action = state.input(&chord, &grid, value, input, phase, now);
                if action.is_none() && (state.clone(), value.clone()) == before {
                    return None;
                }
                Some(capture_change(KeyboardUpdate {
                    state: state.clone(),
                    value: value.clone(),
                    action,
                }))
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
            KeyboardKind::Chord => chord_view(cx, &self.state, &self.chord, &snap),
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

/// The ring of eight groups, each four keys as a diamond, around the
/// layer's name. The held group is larger and in the accent.
fn chord_view<Msg>(
    cx: &mut BuildCx<Msg>,
    state: &KeyboardState,
    layout: &ChordLayout,
    snap: &dyn Fn(f32) -> f32,
) {
    let t = cx.tokens;
    let layer = state.chord_layer(layout);
    let group = state.group();
    let key = snap(22.0);
    let big = snap(30.0);
    let radius = snap(100.0);
    let side = 2.0 * radius + 3.0 * big + snap(8.0);
    cx.begin(Node::new(Style {
        size: Size {
            width: length(side),
            height: length(side),
        },
        flex_shrink: 0.0,
        ..Style::default()
    }));
    let center = side / 2.0;
    // The held group paints last, so its larger keys sit over its
    // neighbors.
    let order = (0..8).filter(|&g| group != Some(g)).chain(group);
    for g in order {
        let angle = (g as f32) * std::f32::consts::FRAC_PI_4;
        let (gx, gy) = (center + radius * angle.sin(), center - radius * angle.cos());
        let on = group == Some(g);
        let k = if on { big } else { key };
        let gap = snap(2.0);
        // A round backdrop gathers the group's four keys.
        let d = snap(3.0 * k + 2.0 * gap + snap(4.0));
        let (fill, edge): (Color, Color) = if on {
            (t.color.accent_muted.into(), t.color.accent.into())
        } else {
            (t.color.background_surface.into(), t.color.border.into())
        };
        cx.leaf(
            Node::new(Style {
                position: Position::Absolute,
                inset: taffy::Rect {
                    left: length(snap(gx - d / 2.0)),
                    top: length(snap(gy - d / 2.0)),
                    right: auto(),
                    bottom: auto(),
                },
                size: Size {
                    width: length(d),
                    height: length(d),
                },
                ..Style::default()
            })
            .draw(Draw::Box(BoxDraw {
                border: Some((edge, t.size.border_width)),
                ..BoxDraw::fill(fill, radii(d / 2.0))
            })),
        );
        // A, B, X, Y as the face buttons sit: bottom, right, left, top.
        let offsets = [(0.0, 1.0), (1.0, 0.0), (-1.0, 0.0), (0.0, -1.0)];
        for (slot, (dx, dy)) in offsets.iter().enumerate() {
            let ch = layout.layers[layer].groups[g][slot];
            let x = snap(gx + dx * (k + gap) - k / 2.0);
            let y = snap(gy + dy * (k + gap) - k / 2.0);
            cap(cx, &glyph(ch), k, k, Some((x, y)), on, on);
        }
    }
    // The middle: the layer's name and the shoulder keys.
    let label = type_style(t.font.label, FontFamily::Body);
    let w = snap(96.0);
    cx.begin(
        Node::new(Style {
            position: Position::Absolute,
            inset: taffy::Rect {
                left: length(snap(center - w / 2.0)),
                top: length(snap(center - w / 2.0)),
                right: auto(),
                bottom: auto(),
            },
            size: Size {
                width: length(w),
                height: length(w),
            },
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::CENTER,
            justify_content: JustifyContent::CENTER,
            gap: Size {
                width: len(0.0),
                height: len(snap(4.0)),
            },
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw {
            border: Some((t.color.border.into(), t.size.border_width)),
            ..BoxDraw::fill(t.color.background_surface.into(), radii(w / 2.0))
        })),
    );
    let mut name = text_node(
        layout.layers[layer].name,
        label,
        t.color.text_primary.into(),
    );
    name.sem = None;
    cx.leaf(name);
    let mut hint = text_node(
        "L2 R2",
        type_style(t.font.supporting, FontFamily::Body),
        t.color.text_secondary.into(),
    );
    hint.sem = None;
    cx.leaf(hint);
    cx.end();
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
    fn holding_a_direction_and_pressing_a_face_button_types() {
        let mut p = Pad::new(KeyboardKind::Chord);
        p.send(Input::Up, Pressed);
        assert_eq!(p.s.group(), Some(0));
        p.tap(Input::A);
        p.tap(Input::B);
        p.send(Input::Up, Released);
        assert_eq!(p.v, "ei");
        assert_eq!(p.s.group(), None, "a hold that typed does not latch");
        // A diagonal: up and right together.
        p.send(Input::Up, Pressed);
        p.send(Input::Right, Pressed);
        assert_eq!(p.s.group(), Some(1));
        p.tap(Input::A);
        p.send(Input::Up, Released);
        p.send(Input::Right, Released);
        assert_eq!(p.v, "eig");
    }

    #[test]
    fn tapping_a_direction_latches_its_group() {
        let mut p = Pad::new(KeyboardKind::Chord);
        p.tap(Input::Down);
        assert_eq!(p.s.group(), Some(4));
        assert!(p.s.announce.starts_with("down: a"), "{}", p.s.announce);
        p.tap(Input::X);
        assert_eq!(p.v, "l");
        assert_eq!(p.s.group(), None, "typing lets the latch go");
        p.tap(Input::Left);
        p.tap(Input::Left);
        assert_eq!(
            p.s.group(),
            None,
            "tapping the latched group again lets it go"
        );
    }

    #[test]
    fn layers_switch_with_a_tap_and_shift_with_a_hold() {
        let mut p = Pad::new(KeyboardKind::Chord);
        p.tap(Input::R2);
        p.tap(Input::Up);
        p.tap(Input::A);
        assert_eq!(p.v, "E");
        p.tap(Input::R2);
        p.tap(Input::Up);
        p.tap(Input::A);
        assert_eq!(p.v, "E1");
        p.tap(Input::R2);
        // Held L2 shifts back one layer for the characters typed meanwhile.
        p.send(Input::L2, Pressed);
        p.tap(Input::Up);
        p.tap(Input::A);
        p.send(Input::L2, Released);
        assert_eq!(p.v, "E1".to_string() + "1");
        assert_eq!(p.s.layer, 0, "a hold that typed does not switch layers");
    }

    #[test]
    fn shoulders_delete_and_space_alone_and_switch_layouts_together() {
        let mut p = Pad::new(KeyboardKind::Chord);
        p.v = "ab".into();
        p.tap(Input::R1);
        assert_eq!(p.v, "ab ");
        p.tap(Input::L1);
        assert_eq!(p.v, "ab");
        // Both together: no delete, no space, the next layout.
        p.send(Input::L1, Pressed);
        let a = p.send(Input::R1, Pressed);
        assert_eq!(a, Some(KeyboardAction::Switched(KeyboardKind::Grid)));
        p.send(Input::L1, Released);
        p.send(Input::R1, Released);
        assert_eq!(p.v, "ab");
        assert_eq!(p.s.kind, KeyboardKind::Grid);
        assert_eq!(
            p.s.toast.as_ref().map(|t| t.0.as_str()),
            Some("Grid keyboard")
        );
        // Holding R1, then pressing L1, is a chord too; letting go of
        // either does nothing more.
        p.send(Input::R1, Pressed);
        p.now += 500.0;
        let a = p.send(Input::L1, Pressed);
        assert_eq!(a, Some(KeyboardAction::Switched(KeyboardKind::Chord)));
        p.send(Input::R1, Released);
        p.send(Input::L1, Released);
        assert_eq!(p.v, "ab");
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
        // Down to the special row, right to Done.
        p.tap(Input::Down);
        p.tap(Input::Down);
        let mut a = None;
        for _ in 0..4 {
            p.tap(Input::Right);
        }
        a = a.or(p.tap(Input::A));
        assert_eq!(a, Some(KeyboardAction::Done));
        assert_eq!(p.tap(Input::Start), Some(KeyboardAction::Done));
        assert_eq!(p.tap(Input::Select), Some(KeyboardAction::Cancel));
    }
}
