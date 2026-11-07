//! A fixed test surface for the web shell: a small settings panel that
//! paints a display list and reports an accessibility tree by hand. It
//! stands in for the core runtime until that lands.
//!
//! Build it with `crates/astryx-web/scripts/build.sh`.

use astryx_core::{
    A11yAction, A11yNode, Color, Command, DisplayList, DrawCmd, FontFamily, FrameInfo, Input,
    InputPhase, Live, PathCmd, Point, Preferences, Radii, Rect, Role, ShadowLayer, Surface,
    TextAlign, TextMeasure, TextStyle,
};
use astryx_web::{ColorScheme, Options};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let options = Options {
        label: Some("Astryx web demo".into()),
        fonts_url: Some("fonts/".into()),
    };
    astryx_web::run_with("#app", Demo::new(), options)
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Switch,
    Checkbox,
    Button,
    Field,
}

struct Item {
    id: u64,
    kind: Kind,
    label: &'static str,
    checked: bool,
    disabled: bool,
    value: String,
}

struct Palette {
    background: Color,
    card: Color,
    text: Color,
    secondary: Color,
    accent: Color,
    track: Color,
    on_accent: Color,
}

fn hex(rgb: u32) -> Color {
    let c = |shift: u32| ((rgb >> shift) & 0xff) as f32 / 255.0;
    Color::rgba(c(16), c(8), c(0), 1.0)
}

impl Palette {
    fn for_scheme(scheme: ColorScheme) -> Self {
        match scheme {
            ColorScheme::Light => Palette {
                background: hex(0xf5f6f8),
                card: hex(0xffffff),
                text: hex(0x1c1e21),
                secondary: hex(0x606770),
                accent: hex(0x0064e0),
                track: hex(0xccd0d5),
                on_accent: hex(0xffffff),
            },
            ColorScheme::Dark => Palette {
                background: hex(0x18191a),
                card: hex(0x242526),
                text: hex(0xe4e6eb),
                secondary: hex(0xb0b3b8),
                accent: hex(0x2d88ff),
                track: hex(0x3e4042),
                on_accent: hex(0x000000),
            },
        }
    }
}

const ROOT: u64 = 1;
const HEADING: u64 = 2;
const HELP: u64 = 3;
const LOGO: u64 = 4;
const PROGRESS: u64 = 5;
const STATUS: u64 = 6;
const RECENT: u64 = 7;
const SAVE: u64 = 13;
const HELP_BUTTON: u64 = 15;
const NAME: u64 = 16;
const ROW: f32 = 48.0;
const GAP: f32 = 8.0;
const RING_SECONDS: f32 = 0.2;
// Cyrillic, Greek, and Vietnamese entries make the shell load those subsets.
const RECENT_FILES: [&str; 6] = [
    "Quarterly plan",
    "Design review notes",
    "Launch checklist",
    "Отчёт за квартал",
    "Σημειώσεις σχεδίου",
    "Kế hoạch ra mắt",
];

struct Demo {
    items: Vec<Item>,
    focus: Option<usize>,
    saves: u32,
    /// Focus ring animation progress, 0 to 1.
    ring: f32,
    preferences: Preferences,
    tree: Option<A11yNode>,
}

impl Demo {
    fn new() -> Self {
        let item = |id, kind, label, checked, disabled| Item {
            id,
            kind,
            label,
            checked,
            disabled,
            value: String::new(),
        };
        Self {
            items: vec![
                item(11, Kind::Switch, "Wi-Fi", true, false),
                item(12, Kind::Checkbox, "Send diagnostics", false, false),
                item(NAME, Kind::Field, "Name", false, false),
                item(SAVE, Kind::Button, "Save", false, false),
                item(14, Kind::Button, "Reset", false, true),
                item(HELP_BUTTON, Kind::Button, "Help", false, false),
            ],
            focus: Some(0),
            saves: 0,
            ring: 1.0,
            preferences: Preferences::default(),
            tree: None,
        }
    }

    /// Moves focus to the next enabled item. `wrap` is true for Tab order.
    fn step(&mut self, forward: bool, wrap: bool) {
        let n = self.items.len();
        let mut index = self.focus.unwrap_or(if forward { n - 1 } else { 0 });
        for _ in 0..n {
            let next = if forward {
                index + 1
            } else {
                index.wrapping_sub(1)
            };
            if next >= n {
                if !wrap {
                    return;
                }
                index = if forward { 0 } else { n - 1 };
            } else {
                index = next;
            }
            if !self.items[index].disabled {
                self.set_focus(index);
                return;
            }
        }
    }

    fn set_focus(&mut self, index: usize) {
        if self.focus != Some(index) {
            self.focus = Some(index);
            self.ring = if self.preferences.reduced_motion {
                1.0
            } else {
                0.0
            };
        }
    }

    fn activate(&mut self) -> Command {
        let Some(item) = self.focus.map(|i| &mut self.items[i]) else {
            return Command::None;
        };
        match item.kind {
            Kind::Switch | Kind::Checkbox => item.checked = !item.checked,
            Kind::Button if item.id == SAVE => self.saves += 1,
            Kind::Button if item.id == HELP_BUTTON => return Command::OpenUrl("help.html".into()),
            Kind::Button | Kind::Field => {}
        }
        Command::None
    }

    /// The focused text field, which takes typing and Backspace.
    fn field(&mut self) -> Option<&mut Item> {
        let index = self.focus?;
        Some(&mut self.items[index]).filter(|item| item.kind == Kind::Field)
    }

    fn progress(&self) -> u32 {
        (40 + 10 * self.saves).min(100)
    }
}

fn text_style(size: f32, weight: u16) -> TextStyle {
    TextStyle {
        family: FontFamily::Body,
        size,
        weight,
        leading: 1.3,
    }
}

fn node(id: u64, role: Role, label: Option<String>, bounds: Rect) -> A11yNode {
    A11yNode {
        text_selection: None,
        secure: false,
        description: None,
        selected: None,
        focusable: false,
        live: None,
        id,
        role,
        label,
        value: None,
        bounds,
        focused: false,
        disabled: false,
        checked: None,
        children: vec![],
    }
}

impl Surface for Demo {
    fn input(&mut self, input: Input, phase: InputPhase) -> Command {
        if phase == InputPhase::Released {
            return Command::None;
        }
        match input {
            Input::Down => self.step(true, false),
            Input::Up => self.step(false, false),
            Input::Next => self.step(true, true),
            Input::Previous => self.step(false, true),
            Input::A => return self.activate(),
            Input::B => self.set_focus(0),
            Input::Backspace => {
                if let Some(field) = self.field() {
                    field.value.pop();
                }
            }
            _ => {}
        }
        Command::None
    }

    fn text_input(&mut self, text: &str) -> Command {
        if let Some(field) = self.field() {
            field.value.push_str(text);
        }
        Command::None
    }

    fn frame(&mut self, frame: &FrameInfo, text: &dyn TextMeasure) -> DisplayList {
        let preferences = self.preferences;
        let p = Palette::for_scheme(preferences.color_scheme);
        if preferences.reduced_motion {
            self.ring = 1.0;
        } else {
            self.ring = (self.ring + frame.dt / RING_SECONDS).min(1.0);
        }
        let size = frame.logical_size();
        let mut cmds = Vec::new();
        let mut children = Vec::new();

        let width = (size.width - 48.0).clamp(0.0, 440.0);
        let card = Rect::new(16.0, 16.0, width + 16.0, (size.height - 32.0).max(0.0));
        cmds.push(DrawCmd::Shadow {
            rect: card,
            radii: Radii::all(12.0),
            layers: vec![ShadowLayer {
                x: 0.0,
                y: 2.0,
                blur: 8.0,
                spread: 0.0,
                inset: false,
                color: Color::rgba(0.0, 0.0, 0.0, 0.15),
            }],
        });
        cmds.push(DrawCmd::Rect {
            rect: card,
            radii: Radii::all(12.0),
            fill: p.card,
            stroke: None,
        });
        let x = 24.0;
        let mut y = 24.0;

        let heading_style = text_style(28.0, 700);
        let heading_rect = Rect::new(x, y, width - 40.0, 36.0);
        cmds.push(DrawCmd::Text {
            rect: heading_rect,
            text: "Settings".into(),
            style: heading_style,
            color: p.text,
            align: TextAlign::Start,
        });
        children.push(node(
            HEADING,
            Role::Heading(1),
            Some("Settings".into()),
            heading_rect,
        ));

        let logo = Rect::new(x + width - 32.0, y + 2.0, 32.0, 32.0);
        let c = logo.center();
        cmds.push(DrawCmd::Path {
            commands: vec![
                PathCmd::MoveTo(Point { x: c.x, y: logo.y }),
                PathCmd::LineTo(Point {
                    x: logo.x + logo.width,
                    y: c.y,
                }),
                PathCmd::LineTo(Point {
                    x: c.x,
                    y: logo.y + logo.height,
                }),
                PathCmd::LineTo(Point { x: logo.x, y: c.y }),
                PathCmd::Close,
            ],
            fill: Some(p.accent),
            stroke: None,
        });
        children.push(node(LOGO, Role::Image, Some("Astryx logo".into()), logo));
        y += 40.0;

        let help = "Arrow keys or Tab move focus. Enter or Space toggles.";
        let help_style = text_style(14.0, 400);
        let help_height = text.measure(help, &help_style, Some(width)).height;
        let help_rect = Rect::new(x, y, width, help_height);
        cmds.push(DrawCmd::Text {
            rect: help_rect,
            text: help.into(),
            style: help_style,
            color: p.secondary,
            align: TextAlign::Start,
        });
        children.push(node(HELP, Role::Text, Some(help.into()), help_rect));
        y += help_height + 16.0;

        let mut focus_rect = None;
        let label_style = text_style(16.0, 500);
        let mut button_x = x;
        for (index, item) in self.items.iter().enumerate() {
            let rect = match item.kind {
                Kind::Button => {
                    let w = (width - 24.0) / 3.0;
                    let r = Rect::new(button_x, y, w, 40.0);
                    button_x += w + 12.0;
                    r
                }
                _ => Rect::new(x, y, width, ROW),
            };
            let alpha = if item.disabled { 0.4 } else { 1.0 };
            cmds.push(DrawCmd::PushOpacity(alpha));
            match item.kind {
                Kind::Switch => {
                    let track = Rect::new(rect.x + rect.width - 52.0, rect.y + 12.0, 44.0, 24.0);
                    let fill = if item.checked { p.accent } else { p.track };
                    cmds.push(DrawCmd::Rect {
                        rect: track,
                        radii: Radii::all(12.0),
                        fill,
                        stroke: None,
                    });
                    let knob_x = if item.checked {
                        track.x + 22.0
                    } else {
                        track.x + 2.0
                    };
                    cmds.push(DrawCmd::Rect {
                        rect: Rect::new(knob_x, track.y + 2.0, 20.0, 20.0),
                        radii: Radii::all(10.0),
                        fill: hex(0xffffff),
                        stroke: None,
                    });
                }
                Kind::Checkbox => {
                    let b = Rect::new(rect.x + rect.width - 32.0, rect.y + 12.0, 24.0, 24.0);
                    let (fill, stroke) = if item.checked {
                        (p.accent, None)
                    } else {
                        (Color::TRANSPARENT, Some((p.secondary, 2.0)))
                    };
                    cmds.push(DrawCmd::Rect {
                        rect: b,
                        radii: Radii::all(4.0),
                        fill,
                        stroke,
                    });
                    if item.checked {
                        cmds.push(DrawCmd::Path {
                            commands: vec![
                                PathCmd::MoveTo(Point {
                                    x: b.x + 6.0,
                                    y: b.y + 12.0,
                                }),
                                PathCmd::LineTo(Point {
                                    x: b.x + 10.0,
                                    y: b.y + 16.0,
                                }),
                                PathCmd::LineTo(Point {
                                    x: b.x + 18.0,
                                    y: b.y + 8.0,
                                }),
                            ],
                            fill: None,
                            stroke: Some((p.on_accent, 2.5)),
                        });
                    }
                }
                Kind::Button => {
                    cmds.push(DrawCmd::Rect {
                        rect,
                        radii: Radii::all(8.0),
                        fill: p.accent,
                        stroke: None,
                    });
                }
                Kind::Field => {
                    let field = Rect::new(rect.x + 120.0, rect.y + 6.0, rect.width - 128.0, 36.0);
                    cmds.push(DrawCmd::Rect {
                        rect: field,
                        radii: Radii::all(6.0),
                        fill: Color::TRANSPARENT,
                        stroke: Some((p.secondary, 1.5)),
                    });
                    let value_style = text_style(16.0, 400);
                    cmds.push(DrawCmd::Text {
                        rect: Rect::new(field.x + 10.0, rect.y + 13.0, field.width - 20.0, 22.0),
                        text: item.value.clone(),
                        style: value_style,
                        color: p.text,
                        align: TextAlign::Start,
                    });
                    if self.focus == Some(index) {
                        let caret =
                            field.x + 10.0 + text.measure(&item.value, &value_style, None).width;
                        cmds.push(DrawCmd::Line {
                            from: Point {
                                x: caret,
                                y: field.y + 8.0,
                            },
                            to: Point {
                                x: caret,
                                y: field.y + 28.0,
                            },
                            color: p.text,
                            width: 1.5,
                        });
                    }
                }
            }
            let (label_rect, color, align) = match item.kind {
                Kind::Button => (
                    Rect::new(rect.x, rect.y + 9.0, rect.width, 22.0),
                    p.on_accent,
                    TextAlign::Center,
                ),
                _ => (
                    Rect::new(rect.x + 8.0, rect.y + 13.0, rect.width - 80.0, 22.0),
                    p.text,
                    TextAlign::Start,
                ),
            };
            cmds.push(DrawCmd::Text {
                rect: label_rect,
                text: item.label.into(),
                style: label_style,
                color,
                align,
            });
            cmds.push(DrawCmd::PopOpacity);

            let focused = self.focus == Some(index);
            if focused {
                focus_rect = Some(rect);
            }
            let role = match item.kind {
                Kind::Switch => Role::Switch,
                Kind::Checkbox => Role::Checkbox,
                Kind::Button => Role::Button,
                Kind::Field => Role::TextInput,
            };
            let mut n = node(item.id, role, Some(item.label.into()), rect);
            n.focused = focused;
            n.focusable = true;
            n.disabled = item.disabled;
            n.checked = matches!(item.kind, Kind::Switch | Kind::Checkbox).then_some(item.checked);
            if item.kind == Kind::Field {
                n.value = Some(item.value.clone());
            }
            children.push(n);
            if item.kind != Kind::Button || button_x > x + width {
                y += rect.height + GAP;
            }
        }
        if let Some(rect) = focus_rect {
            let grow = 2.0 + 4.0 * (1.0 - self.ring);
            cmds.push(DrawCmd::Rect {
                rect: rect.inset(-grow),
                radii: Radii::all(10.0 + grow),
                fill: Color::TRANSPARENT,
                stroke: Some((p.accent.with_alpha(0.5 + 0.5 * self.ring), 2.0)),
            });
        }
        y += 12.0;

        let small = text_style(14.0, 400);
        let progress = self.progress();
        cmds.push(DrawCmd::Text {
            rect: Rect::new(x, y, width, 20.0),
            text: format!("Storage used: {progress}%"),
            style: small,
            color: p.secondary,
            align: TextAlign::Start,
        });
        let bar = Rect::new(x, y + 24.0, width, 8.0);
        cmds.push(DrawCmd::Rect {
            rect: bar,
            radii: Radii::all(4.0),
            fill: p.track,
            stroke: None,
        });
        cmds.push(DrawCmd::Rect {
            rect: Rect::new(
                bar.x,
                bar.y,
                bar.width * progress as f32 / 100.0,
                bar.height,
            ),
            radii: Radii::all(4.0),
            fill: p.accent,
            stroke: None,
        });
        let mut bar_node = node(
            PROGRESS,
            Role::ProgressBar,
            Some("Storage used".into()),
            Rect::new(x, y, width, 32.0),
        );
        bar_node.value = Some(format!("{progress}%"));
        children.push(bar_node);
        y += 44.0;

        let status = match self.saves {
            0 => "Not saved yet".to_string(),
            1 => "Saved once".to_string(),
            n => format!("Saved {n} times"),
        };
        let status_rect = Rect::new(x, y, width, 20.0);
        cmds.push(DrawCmd::Text {
            rect: status_rect,
            text: status.clone(),
            style: small,
            color: p.text,
            align: TextAlign::Start,
        });
        let mut status_node = node(STATUS, Role::Text, Some(status), status_rect);
        status_node.live = Some(Live::Polite);
        children.push(status_node);
        y += 32.0;

        let list_top = y;
        let mut items = Vec::new();
        cmds.push(DrawCmd::Text {
            rect: Rect::new(x, y, width, 20.0),
            text: "Recent files".into(),
            style: text_style(14.0, 700),
            color: p.text,
            align: TextAlign::Start,
        });
        y += 24.0;
        for (i, file) in RECENT_FILES.iter().enumerate() {
            let rect = Rect::new(x, y, width, 24.0);
            cmds.push(DrawCmd::Line {
                from: Point { x, y },
                to: Point { x: x + width, y },
                color: p.track,
                width: 1.0,
            });
            cmds.push(DrawCmd::Text {
                rect: Rect::new(x, y + 2.0, width, 20.0),
                text: (*file).into(),
                style: small,
                color: p.text,
                align: TextAlign::Start,
            });
            items.push(node(
                100 + i as u64,
                Role::ListItem,
                Some((*file).into()),
                rect,
            ));
            y += 24.0;
        }
        let mut list = node(
            RECENT,
            Role::List,
            Some("Recent files".into()),
            Rect::new(x, list_top, width, y - list_top),
        );
        list.children = items;
        children.push(list);

        let mut root = node(
            ROOT,
            Role::Group,
            Some("Astryx web demo".into()),
            Rect::new(0.0, 0.0, size.width, size.height),
        );
        root.children = children;
        self.tree = Some(root);
        DisplayList {
            clear: p.background,
            commands: cmds,
        }
    }

    fn accessibility(&self) -> Option<A11yNode> {
        self.tree.clone()
    }

    fn wants_frames(&self) -> bool {
        self.ring < 1.0
    }

    fn set_preferences(&mut self, preferences: &Preferences) {
        self.preferences = *preferences;
    }

    fn a11y_action(&mut self, id: u64, action: A11yAction) -> Command {
        let Some(index) = self
            .items
            .iter()
            .position(|item| item.id == id && !item.disabled)
        else {
            return Command::None;
        };
        self.set_focus(index);
        if action == A11yAction::Activate {
            self.activate();
        }
        Command::None
    }
}
