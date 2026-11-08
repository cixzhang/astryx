//! Kbd: keycaps for a keyboard shortcut or gamepad buttons.

use super::text::{text_node, type_style};
use super::{len, radii};
use crate::contract::{FontFamily, Role};
use crate::element::{base_props, Base, BoxDraw, Build, BuildCx, Draw, Node, Semantics};
use taffy::prelude::*;

/// One keycap: what it shows and what assistive technology says.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Cap {
    glyph: String,
    spoken: String,
}

/// A keyboard shortcut or gamepad buttons as keycaps (`Kbd`): one cap per
/// key, read as one image whose name joins the keys' spoken names, such as
/// "Control + K" or "B button". Each cap is a padded neutral badge with a
/// darker bottom edge, in medium supporting text, and sits on the line of
/// the text beside it.
pub struct Kbd {
    base: Base,
    caps: Vec<Cap>,
    /// The caps are alternatives ("L1 or R1"), not a chord.
    either: bool,
}

impl Kbd {
    /// Keyboard keys, as Astryx's `keys`: separated by `+`. Special keys:
    /// mod (Control; Command on Apple keyboards), ctrl, alt, shift, enter,
    /// backspace, escape, tab, up, down, left, right, plus, with esc and
    /// return as aliases. Other keys show uppercased.
    pub fn new(keys: &str) -> Self {
        Kbd {
            base: Base::default(),
            caps: segments(keys).map(|key| keyboard_cap(&key)).collect(),
            either: false,
        }
    }

    /// Gamepad buttons: a, b, x, y, l1, r1, l2, r2, start, select, menu,
    /// and dpad (with dpad-up, dpad-down, dpad-left, dpad-right), each read
    /// as "A button", "D-pad", and so on. Separate buttons pressed
    /// together with `+`, and either-or buttons with spaces: "L1 R1" is
    /// read as "L1 or R1 buttons".
    pub fn gamepad(buttons: &str) -> Self {
        let either = !buttons.contains('+') && buttons.split_whitespace().count() > 1;
        let caps = if either {
            buttons
                .split_whitespace()
                .map(|b| gamepad_cap(&b.to_lowercase()))
                .collect()
        } else {
            segments(buttons)
                .map(|button| gamepad_cap(&button))
                .collect()
        };
        Kbd {
            base: Base::default(),
            caps,
            either,
        }
    }

    /// The name assistive technology reads.
    pub fn spoken(&self) -> String {
        if self.either {
            // "L1 or R1 buttons": one plural noun for the alternatives.
            let all_buttons = self.caps.iter().all(|c| c.spoken.ends_with(" button"));
            let names: Vec<&str> = self
                .caps
                .iter()
                .map(|c| {
                    if all_buttons {
                        c.spoken.trim_end_matches(" button")
                    } else {
                        c.spoken.as_str()
                    }
                })
                .collect();
            let joined = names.join(" or ");
            return if all_buttons {
                format!("{joined} buttons")
            } else {
                joined
            };
        }
        self.caps
            .iter()
            .map(|c| c.spoken.as_str())
            .collect::<Vec<_>>()
            .join(" + ")
    }
}

base_props!(Kbd);

fn segments(keys: &str) -> impl Iterator<Item = String> + '_ {
    keys.split('+')
        .map(|k| k.trim().to_lowercase())
        .filter(|k| !k.is_empty())
}

fn cap(glyph: &str, spoken: &str) -> Cap {
    Cap {
        glyph: glyph.into(),
        spoken: spoken.into(),
    }
}

fn keyboard_cap(key: &str) -> Cap {
    match key {
        // Platforms are not known to the core, so `mod` shows Control, as
        // Astryx does off Apple platforms.
        "mod" | "ctrl" => cap("Ctrl", "Control"),
        "alt" => cap("Alt", "Alt"),
        "shift" => cap("Shift", "Shift"),
        "enter" | "return" => cap("Enter", "Enter"),
        "backspace" => cap("Backspace", "Backspace"),
        "escape" | "esc" => cap("Esc", "Escape"),
        "tab" => cap("Tab", "Tab"),
        "up" => cap("↑", "Up arrow"),
        "down" => cap("↓", "Down arrow"),
        "left" => cap("←", "Left arrow"),
        "right" => cap("→", "Right arrow"),
        "plus" => cap("+", "Plus"),
        other => {
            let upper = other.to_uppercase();
            cap(&upper, &upper)
        }
    }
}

fn gamepad_cap(button: &str) -> Cap {
    match button {
        "dpad" | "d-pad" => cap("D-pad", "D-pad"),
        "dpad-up" => cap("↑", "D-pad up"),
        "dpad-down" => cap("↓", "D-pad down"),
        "dpad-left" => cap("←", "D-pad left"),
        "dpad-right" => cap("→", "D-pad right"),
        "start" => cap("Start", "Start button"),
        "select" => cap("Select", "Select button"),
        "menu" => cap("Menu", "Menu button"),
        other => {
            let upper = other.to_uppercase();
            cap(&upper, &format!("{upper} button"))
        }
    }
}

impl<Msg> Build<Msg> for Kbd {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden || self.caps.is_empty() {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let spoken = self.spoken();
        let mut style = type_style(t.font.supporting, FontFamily::Body);
        style.weight = t.font.weight.medium;
        let line = (style.size * style.leading - 0.01).ceil();
        // The bottom edge is 2 points, or whole cells on a pixel grid.
        let edge = match cx.theme.grid {
            Some(cell) => (2.0 / cell).round().max(1.0) * cell,
            None => 2.0,
        };
        let corner = radii(t.radius.inner);
        // At least 4 points either side of the glyph, so caps stay padded in
        // themes with a small spacing scale, in whole cells on a pixel grid.
        let pad = match cx.theme.grid {
            Some(cell) => (t.space.s1.max(4.0) / cell).round() * cell,
            None => t.space.s1.max(4.0),
        };
        cx.key(self.base.id.as_deref());
        cx.begin(
            Node::new(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                gap: Size {
                    width: len(t.space.s1),
                    height: len(0.0),
                },
                flex_shrink: 0.0,
                ..Style::default()
            })
            .sem(Semantics::new(Role::Image, spoken)),
        );
        for key in self.caps {
            // The face is the translucent neutral over whatever is behind
            // the cap; a bar of the emphasized border color draws its
            // bottom edge, as CSS draws a bottom border.
            cx.begin(
                Node::new(Style {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::CENTER,
                    justify_content: JustifyContent::CENTER,
                    padding: taffy::Rect {
                        left: len(pad),
                        right: len(pad),
                        top: len(0.0),
                        bottom: len(edge),
                    },
                    min_size: Size {
                        width: length(t.space.s5),
                        height: length(t.space.s5.max(line + edge)),
                    },
                    flex_shrink: 0.0,
                    ..Style::default()
                })
                .draw(Draw::Box(BoxDraw::fill(c.neutral.into(), corner))),
            );
            cx.leaf(text_node(key.glyph, style, c.text_primary.into()));
            let r = t.radius.inner;
            cx.leaf(
                Node::new(Style {
                    position: Position::Absolute,
                    // Inset so its ends stay inside the face's rounded corners.
                    inset: taffy::Rect {
                        left: length(r / 2.0),
                        right: length(r / 2.0),
                        top: auto(),
                        bottom: length(0.0),
                    },
                    size: Size {
                        width: auto(),
                        height: length(edge),
                    },
                    ..Style::default()
                })
                .draw(Draw::Box(BoxDraw::fill(
                    c.border_emphasized.into(),
                    crate::contract::Radii([0.0, 0.0, edge, edge]),
                ))),
            );
            cx.end();
        }
        cx.end();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_parse_and_speak() {
        assert_eq!(Kbd::new("mod+k").spoken(), "Control + K");
        assert_eq!(
            Kbd::new(" Esc + shift + return").spoken(),
            "Escape + Shift + Enter"
        );
        assert_eq!(Kbd::gamepad("B").spoken(), "B button");
        assert_eq!(Kbd::gamepad("l1+r1").spoken(), "L1 button + R1 button");
        assert_eq!(Kbd::gamepad("dpad").spoken(), "D-pad");
        assert_eq!(Kbd::gamepad("L1 R1").spoken(), "L1 or R1 buttons");
        assert_eq!(Kbd::gamepad("L1 R1").caps.len(), 2);
        assert_eq!(
            Kbd::new("a+a").caps.len(),
            2,
            "a repeated key gets its own cap"
        );
    }
}
