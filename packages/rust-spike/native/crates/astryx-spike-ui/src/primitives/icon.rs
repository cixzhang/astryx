//! Icon: a named glyph from Astryx's built-in set, in any of its 20 colors.

use crate::util::{color, component, square};
use astryx_core::authoring::{icon_path, Base, Build, BuildCx, Draw, Node, PathDraw, Semantics};
use astryx_core::contract::{Color, PathCmd, Point, Role};
pub use astryx_core::IconSize;
use taffy::Style;

/// Icon `color`: a semantic or a palette icon color, or the surrounding text
/// color.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum IconColor {
    Primary,
    Secondary,
    Tertiary,
    Disabled,
    Accent,
    Info,
    Success,
    Error,
    Warning,
    #[default]
    Inherit,
    Blue,
    Red,
    Green,
    Gray,
    Cyan,
    Teal,
    Yellow,
    Orange,
    Pink,
    Purple,
}

impl IconColor {
    pub const ALL: [IconColor; 20] = [
        IconColor::Primary,
        IconColor::Secondary,
        IconColor::Tertiary,
        IconColor::Disabled,
        IconColor::Accent,
        IconColor::Info,
        IconColor::Success,
        IconColor::Error,
        IconColor::Warning,
        IconColor::Inherit,
        IconColor::Blue,
        IconColor::Red,
        IconColor::Green,
        IconColor::Gray,
        IconColor::Cyan,
        IconColor::Teal,
        IconColor::Yellow,
        IconColor::Orange,
        IconColor::Pink,
        IconColor::Purple,
    ];

    pub fn resolve<Msg>(self, cx: &BuildCx<Msg>) -> Color {
        let c = &cx.tokens.color;
        match self {
            IconColor::Primary => color(c.icon_primary),
            IconColor::Secondary | IconColor::Tertiary => color(c.icon_secondary),
            IconColor::Disabled => color(c.icon_disabled),
            IconColor::Accent => color(c.icon_accent),
            IconColor::Info => color(c.icon_info),
            IconColor::Success => color(c.icon_success),
            IconColor::Error => color(c.icon_error),
            IconColor::Warning => color(c.icon_warning),
            IconColor::Inherit => cx.inherited().color,
            IconColor::Blue => color(c.icon_blue),
            IconColor::Red => color(c.icon_red),
            IconColor::Green => color(c.icon_green),
            IconColor::Gray => color(c.icon_gray),
            IconColor::Cyan => color(c.icon_cyan),
            IconColor::Teal => color(c.icon_teal),
            IconColor::Yellow => color(c.icon_yellow),
            IconColor::Orange => color(c.icon_orange),
            IconColor::Pink => color(c.icon_pink),
            IconColor::Purple => color(c.icon_purple),
        }
    }
}

const fn p(x: f32, y: f32) -> Point {
    Point { x, y }
}

/// Glyphs Astryx registers under a component namespace.
const STEPPER_DOWN: &[PathCmd] = &[
    PathCmd::MoveTo(p(7.5, 9.75)),
    PathCmd::LineTo(p(12.0, 14.25)),
    PathCmd::LineTo(p(16.5, 9.75)),
];
const STEPPER_UP: &[PathCmd] = &[
    PathCmd::MoveTo(p(7.5, 14.25)),
    PathCmd::LineTo(p(12.0, 9.75)),
    PathCmd::LineTo(p(16.5, 14.25)),
];
const CHEVRON_UP: &[PathCmd] = &[
    PathCmd::MoveTo(p(6.0, 15.0)),
    PathCmd::LineTo(p(12.0, 9.0)),
    PathCmd::LineTo(p(18.0, 15.0)),
];
const PLUS: &[PathCmd] = &[
    PathCmd::MoveTo(p(12.0, 5.0)),
    PathCmd::LineTo(p(12.0, 19.0)),
    PathCmd::MoveTo(p(5.0, 12.0)),
    PathCmd::LineTo(p(19.0, 12.0)),
];
const MINUS: &[PathCmd] = &[
    PathCmd::MoveTo(p(5.0, 12.0)),
    PathCmd::LineTo(p(19.0, 12.0)),
];
/// Six dots in two columns: a drag handle.
const GRIP: &[PathCmd] = &[];

/// The path, whether it is filled, and its stroke width, for a glyph name.
pub fn glyph(name: &str) -> Option<(&'static [PathCmd], bool, f32)> {
    match name {
        "numberInput:stepperDown" => Some((STEPPER_DOWN, false, 1.5)),
        "numberInput:stepperUp" => Some((STEPPER_UP, false, 1.5)),
        "chevronUp" => Some((CHEVRON_UP, false, 1.5)),
        "plus" => Some((PLUS, false, 1.5)),
        "minus" => Some((MINUS, false, 1.5)),
        "grip" => Some((GRIP, true, 0.0)),
        _ => icon_path(name),
    }
}

/// An icon node of `side` points in `color`, or `None` for an unknown name.
pub(crate) fn icon_node<Msg>(name: &str, side: f32, color: Color) -> Option<Node<Msg>> {
    let (commands, filled, width) = glyph(name)?;
    let commands = if name == "grip" {
        grip_dots()
    } else {
        commands.to_vec()
    };
    Some(
        Node::new(Style {
            size: square(side),
            flex_shrink: 0.0,
            ..Style::default()
        })
        .draw(Draw::Path(PathDraw {
            commands,
            view_box: 24.0,
            fill: filled.then_some(color),
            stroke: (!filled).then_some((color, width)),
        })),
    )
}

fn grip_dots() -> Vec<PathCmd> {
    let mut out = Vec::new();
    for &x in &[9.0f32, 15.0] {
        for &y in &[6.0f32, 12.0, 18.0] {
            // A small square dot, 3 units wide.
            out.extend([
                PathCmd::MoveTo(p(x - 1.5, y - 1.5)),
                PathCmd::LineTo(p(x + 1.5, y - 1.5)),
                PathCmd::LineTo(p(x + 1.5, y + 1.5)),
                PathCmd::LineTo(p(x - 1.5, y + 1.5)),
                PathCmd::Close,
            ]);
        }
    }
    out
}

/// A shared icon by name (`Icon`), such as `"check"` or `"chevronDown"`.
#[derive(Clone, Debug)]
pub struct Icon {
    base: Base,
    icon: String,
    color: IconColor,
    size: Option<IconSize>,
    label: Option<String>,
}

impl Icon {
    pub fn new(icon: impl Into<String>) -> Self {
        Icon {
            base: Base::default(),
            icon: icon.into(),
            color: IconColor::default(),
            size: None,
            label: None,
        }
    }
    pub fn color(mut self, color: IconColor) -> Self {
        self.color = color;
        self
    }
    /// Without a size, an icon takes its container's icon size (a Button's,
    /// for example), else `md`.
    pub fn size(mut self, size: IconSize) -> Self {
        self.size = Some(size);
        self
    }
    /// The accessible name of a meaningful icon; without one it is decorative.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into()).filter(|l| !l.is_empty());
        self
    }
    /// The size a container asks for when the caller set none.
    #[allow(dead_code)]
    pub(crate) fn default_size(mut self, size: IconSize) -> Self {
        self.size.get_or_insert(size);
        self
    }
}

component!(Icon);

impl<Msg> Build<Msg> for Icon {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let color = self.color.resolve(cx);
        let side = self.size.unwrap_or_default().points();
        let Some(mut node) = icon_node(&self.icon, side, color) else {
            return;
        };
        node.sem = self.label.map(|label| Semantics::new(Role::Image, label));
        cx.key(self.base.id.as_deref());
        cx.leaf(node);
    }
}
