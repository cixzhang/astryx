//! Text and Heading.

use crate::contract::{Color, FontFamily, Role, TextAlign, TextStyle};
use crate::element::{base_props, Base, Build, BuildCx, Draw, Node, Semantics, TextDraw};
use astryx_tokens::{Tokens, TypeStyle};
use taffy::Style;

/// Text `type`: one of the Astryx type styles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextType {
    #[default]
    Body,
    Large,
    Label,
    Supporting,
    Code,
    Display1,
    Display2,
    Display3,
    /// The surrounding component's type style and color.
    Inherit,
}

/// Text `size`, replacing the type style's size. `S4xs` is Astryx `4xs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextSize {
    S4xs,
    S3xs,
    S2xs,
    Xsm,
    Sm,
    Base,
    Lg,
    Xl,
    S2xl,
    S3xl,
    S4xl,
}

/// Text and Heading `color`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextColor {
    Primary,
    Secondary,
    Disabled,
    /// Drawn in the secondary text color, as Astryx does.
    Placeholder,
    Accent,
    Inherit,
}

/// Text and Heading `weight`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextWeight {
    Normal,
    Medium,
    Semibold,
    Bold,
}

/// Text and Heading `justify`: alignment of each line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextJustify {
    #[default]
    Start,
    Center,
    End,
}

/// Heading `type`: a display style that replaces the level's look.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HeadingType {
    Display1,
    Display2,
    Display3,
}

pub(crate) fn type_style(t: TypeStyle, family: FontFamily) -> TextStyle {
    TextStyle {
        direction: None,
        family,
        size: t.size,
        weight: t.weight,
        leading: t.leading,
    }
}

impl TextSize {
    fn points(self, tokens: &Tokens) -> f32 {
        let s = &tokens.font.size;
        match self {
            TextSize::S4xs => s.s4xs,
            TextSize::S3xs => s.s3xs,
            TextSize::S2xs => s.s2xs,
            TextSize::Xsm => s.xs,
            TextSize::Sm => s.sm,
            TextSize::Base => s.base,
            TextSize::Lg => s.lg,
            TextSize::Xl => s.xl,
            TextSize::S2xl => s.s2xl,
            TextSize::S3xl => s.s3xl,
            TextSize::S4xl => s.s4xl,
        }
    }
}

impl TextWeight {
    pub(crate) fn value(self, tokens: &Tokens) -> u16 {
        let w = &tokens.font.weight;
        match self {
            TextWeight::Normal => w.normal,
            TextWeight::Medium => w.medium,
            TextWeight::Semibold => w.semibold,
            TextWeight::Bold => w.bold,
        }
    }
}

impl TextColor {
    /// The color, or `inherited` for `Inherit`.
    pub(crate) fn resolve(self, tokens: &Tokens, inherited: Color) -> Color {
        let c = &tokens.color;
        match self {
            TextColor::Primary => c.text_primary.into(),
            TextColor::Secondary | TextColor::Placeholder => c.text_secondary.into(),
            TextColor::Disabled => c.text_disabled.into(),
            TextColor::Accent => c.text_accent.into(),
            TextColor::Inherit => inherited,
        }
    }
}

impl From<TextJustify> for TextAlign {
    fn from(j: TextJustify) -> Self {
        match j {
            TextJustify::Start => TextAlign::Start,
            TextJustify::Center => TextAlign::Center,
            TextJustify::End => TextAlign::End,
        }
    }
}

/// A text leaf node. Its accessible text is its words unless `sem` says otherwise.
pub(crate) fn text_node<Msg>(text: impl Into<String>, style: TextStyle, color: Color) -> Node<Msg> {
    let text = text.into();
    Node::new(Style::default())
        .sem(Semantics::new(Role::Text, text.clone()))
        .draw(Draw::Text(TextDraw {
            text,
            style,
            color,
            align: TextAlign::Start,
            max_lines: None,
            strikethrough: false,
            shorter: Vec::new(),
        }))
}

pub(crate) fn text_draw<Msg>(node: &mut Node<Msg>) -> &mut TextDraw {
    match &mut node.draw {
        Draw::Text(t) => t,
        _ => unreachable!("text_draw on a node without text"),
    }
}

/// Words in one of Astryx's type styles (`Text`).
#[derive(Clone, Debug, Default)]
pub struct Text {
    base: Base,
    children: String,
    text_type: Option<TextType>,
    size: Option<TextSize>,
    color: Option<TextColor>,
    weight: Option<TextWeight>,
    max_lines: Option<u32>,
    justify: Option<TextJustify>,
    has_strikethrough: bool,
    has_tabular_numbers: bool,
}

impl Text {
    /// Text with `children` as its words.
    pub fn new(children: impl Into<String>) -> Self {
        Text {
            children: children.into(),
            ..Text::default()
        }
    }
    /// Astryx `type`; `body` when unset.
    pub fn r#type(mut self, text_type: TextType) -> Self {
        self.text_type = Some(text_type);
        self
    }
    pub fn size(mut self, size: TextSize) -> Self {
        self.size = Some(size);
        self
    }
    /// Secondary for `supporting`, primary otherwise, when unset.
    pub fn color(mut self, color: TextColor) -> Self {
        self.color = Some(color);
        self
    }
    pub fn weight(mut self, weight: TextWeight) -> Self {
        self.weight = Some(weight);
        self
    }
    /// Lines shown before an ellipsis; 0 shows every line.
    pub fn max_lines(mut self, max_lines: u32) -> Self {
        self.max_lines = (max_lines > 0).then_some(max_lines);
        self
    }
    pub fn justify(mut self, justify: TextJustify) -> Self {
        self.justify = Some(justify);
        self
    }
    pub fn has_strikethrough(mut self, on: bool) -> Self {
        self.has_strikethrough = on;
        self
    }
    /// Kept for the Astryx contract. The display list has no figure
    /// variants yet, so digits keep the font's own widths.
    pub fn has_tabular_numbers(mut self, on: bool) -> Self {
        self.has_tabular_numbers = on;
        self
    }
}

base_props!(Text);

impl<Msg> Build<Msg> for Text {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let tokens = cx.tokens;
        let inherited = cx.inherited();
        let f = &tokens.font;
        let text_type = self.text_type.unwrap_or_default();
        let mut style = match text_type {
            TextType::Body => type_style(f.body, FontFamily::Body),
            TextType::Large => type_style(f.large, FontFamily::Body),
            TextType::Label => type_style(f.label, FontFamily::Body),
            TextType::Supporting => type_style(f.supporting, FontFamily::Body),
            TextType::Code => type_style(f.code, FontFamily::Code),
            // Display types draw in the heading family, as Astryx's type scale sets them.
            TextType::Display1 => type_style(f.display1, FontFamily::Heading),
            TextType::Display2 => type_style(f.display2, FontFamily::Heading),
            TextType::Display3 => type_style(f.display3, FontFamily::Heading),
            TextType::Inherit => inherited.style,
        };
        if let Some(size) = self.size {
            style.size = size.points(tokens);
        }
        if let Some(weight) = self.weight {
            style.weight = weight.value(tokens);
        }
        let color = self.color.unwrap_or(match text_type {
            TextType::Supporting => TextColor::Secondary,
            TextType::Inherit => TextColor::Inherit,
            _ => TextColor::Primary,
        });
        let mut node = text_node(self.children, style, color.resolve(tokens, inherited.color));
        let t = text_draw(&mut node);
        t.max_lines = self.max_lines;
        t.align = self.justify.unwrap_or_default().into();
        t.strikethrough = self.has_strikethrough;
        cx.key(self.base.id.as_deref());
        cx.leaf(node);
    }
}

/// A heading at levels 1 through 6 (`Heading`).
#[derive(Clone, Debug)]
pub struct Heading {
    base: Base,
    level: u8,
    children: String,
    heading_type: Option<HeadingType>,
    weight: Option<TextWeight>,
    accessibility_level: Option<u8>,
    color: Option<TextColor>,
    max_lines: Option<u32>,
    justify: Option<TextJustify>,
    has_strikethrough: bool,
}

impl Heading {
    /// A heading at `level` (1 through 6), which sets both its place in the
    /// outline and its look.
    pub fn new(level: u8, children: impl Into<String>) -> Self {
        debug_assert!(
            (1..=6).contains(&level),
            "heading level {level} is outside 1..=6"
        );
        Heading {
            base: Base::default(),
            level: level.clamp(1, 6),
            children: children.into(),
            heading_type: None,
            weight: None,
            accessibility_level: None,
            color: None,
            max_lines: None,
            justify: None,
            has_strikethrough: false,
        }
    }
    /// Astryx `type`: a display style replacing the level's look.
    pub fn r#type(mut self, heading_type: HeadingType) -> Self {
        self.heading_type = Some(heading_type);
        self
    }
    pub fn weight(mut self, weight: TextWeight) -> Self {
        self.weight = Some(weight);
        self
    }
    /// The level assistive technology reads, when it differs from `level`.
    pub fn accessibility_level(mut self, level: u8) -> Self {
        self.accessibility_level = Some(level.clamp(1, 6));
        self
    }
    pub fn color(mut self, color: TextColor) -> Self {
        self.color = Some(color);
        self
    }
    pub fn max_lines(mut self, max_lines: u32) -> Self {
        self.max_lines = (max_lines > 0).then_some(max_lines);
        self
    }
    pub fn justify(mut self, justify: TextJustify) -> Self {
        self.justify = Some(justify);
        self
    }
    pub fn has_strikethrough(mut self, on: bool) -> Self {
        self.has_strikethrough = on;
        self
    }
}

base_props!(Heading);

impl<Msg> Build<Msg> for Heading {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let tokens = cx.tokens;
        let f = &tokens.font;
        let look = match self.heading_type {
            Some(HeadingType::Display1) => f.display1,
            Some(HeadingType::Display2) => f.display2,
            Some(HeadingType::Display3) => f.display3,
            None => [
                f.heading1, f.heading2, f.heading3, f.heading4, f.heading5, f.heading6,
            ][self.level as usize - 1],
        };
        let mut style = type_style(look, FontFamily::Heading);
        if let Some(weight) = self.weight {
            style.weight = weight.value(tokens);
        }
        let color = self
            .color
            .unwrap_or(TextColor::Primary)
            .resolve(tokens, cx.inherited().color);
        let level = self.accessibility_level.unwrap_or(self.level);
        let mut node = text_node(self.children.clone(), style, color)
            .sem(Semantics::new(Role::Heading(level), self.children));
        let t = text_draw(&mut node);
        t.max_lines = self.max_lines;
        t.align = self.justify.unwrap_or_default().into();
        t.strikethrough = self.has_strikethrough;
        cx.key(self.base.id.as_deref());
        cx.leaf(node);
    }
}
