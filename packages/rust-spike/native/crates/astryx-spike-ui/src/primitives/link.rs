//! Link: navigates to a URL, or runs an action when it has no `href`.

use super::icon::icon_node;
use crate::util::{color, component, len, mix, radii};
use astryx_core::authoring::{
    text_draw, text_node, type_style, Base, Build, BuildCx, Indicator, Interact, Node, Semantics,
};
use astryx_core::contract::{FontFamily, Role};
use astryx_core::{TextColor, TextSize, TextWeight};
use astryx_spike_foundations::{args, CxExt};
use taffy::{AlignItems, FlexDirection, Style};

/// Link `display`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LinkDisplay {
    #[default]
    Inline,
    Block,
}

/// A text link (`Link`). Without `href` it acts as a button that looks like
/// a link. Disabled links keep their focus stop.
pub struct Link<Msg> {
    base: Base,
    children: String,
    href: Option<String>,
    has_underline: bool,
    is_disabled: bool,
    is_external_link: bool,
    new_tab_label: Option<String>,
    is_standalone: bool,
    size: Option<TextSize>,
    weight: Option<TextWeight>,
    color: TextColor,
    display: LinkDisplay,
    max_lines: u32,
    tooltip: Option<String>,
    label: Option<String>,
    on_click: Option<Msg>,
}

impl<Msg> Link<Msg> {
    pub fn new(children: impl Into<String>) -> Self {
        Link {
            base: Base::default(),
            children: children.into(),
            href: None,
            has_underline: false,
            is_disabled: false,
            is_external_link: false,
            new_tab_label: None,
            is_standalone: false,
            size: None,
            weight: None,
            color: TextColor::Accent,
            display: LinkDisplay::Inline,
            max_lines: 0,
            tooltip: None,
            label: None,
            on_click: None,
        }
    }
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }
    /// Always underlined, not only on hover.
    pub fn has_underline(mut self, on: bool) -> Self {
        self.has_underline = on;
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    /// Opens in a new tab, shows the external-link icon and says so to
    /// assistive technology.
    pub fn is_external_link(mut self, on: bool) -> Self {
        self.is_external_link = on;
        self
    }
    pub fn new_tab_label(mut self, label: impl Into<String>) -> Self {
        self.new_tab_label = Some(label.into());
        self
    }
    /// A link that stands alone rather than inside running text: it takes
    /// the label type style.
    pub fn is_standalone(mut self, on: bool) -> Self {
        self.is_standalone = on;
        self
    }
    pub fn size(mut self, size: TextSize) -> Self {
        self.size = Some(size);
        self
    }
    pub fn weight(mut self, weight: TextWeight) -> Self {
        self.weight = Some(weight);
        self
    }
    pub fn color(mut self, color: TextColor) -> Self {
        self.color = color;
        self
    }
    pub fn display(mut self, display: LinkDisplay) -> Self {
        self.display = display;
        self
    }
    /// Clamps to this many lines; 0 does not clamp.
    pub fn max_lines(mut self, lines: u32) -> Self {
        self.max_lines = lines;
        self
    }
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
    /// The accessible name, when it differs from the visible words.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    pub fn on_click(mut self, msg: Msg) -> Self {
        self.on_click = Some(msg);
        self
    }
}

component!(Link<Msg>);

fn size_points(size: TextSize, t: &astryx_core::Tokens) -> f32 {
    let s = &t.font.size;
    match size {
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

pub(crate) fn weight_value(weight: TextWeight, t: &astryx_core::Tokens) -> u16 {
    let w = &t.font.weight;
    match weight {
        TextWeight::Normal => w.normal,
        TextWeight::Medium => w.medium,
        TextWeight::Semibold => w.semibold,
        TextWeight::Bold => w.bold,
    }
}

impl<Msg: Clone + 'static> Build<Msg> for Link<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let inherited = cx.inherited();
        let mut style = if self.is_standalone {
            type_style(t.font.label, FontFamily::Body)
        } else {
            inherited.style
        };
        if let Some(size) = self.size {
            style.size = size_points(size, t);
        }
        if let Some(weight) = self.weight {
            style.weight = weight_value(weight, t);
        }
        let ink = match self.color {
            _ if self.is_disabled => color(c.text_disabled),
            TextColor::Primary => color(c.text_primary),
            TextColor::Secondary | TextColor::Placeholder => color(c.text_secondary),
            TextColor::Disabled => color(c.text_disabled),
            TextColor::Accent => color(c.text_accent),
            TextColor::Inherit => inherited.color,
        };
        let new_tab = self
            .new_tab_label
            .clone()
            .unwrap_or_else(|| cx.t("@astryx.link.newTab", &args! {}));
        let mut name = self.label.clone().unwrap_or_else(|| self.children.clone());
        if self.is_external_link {
            name = format!("{name} {new_tab}");
        }
        // Without an href it acts on press, so it is a button, as Astryx
        // renders a <button> for it.
        let role = if self.href.is_some() {
            Role::Link
        } else {
            Role::Button
        };
        let mut sem = Semantics::new(role, name);
        sem.disabled = self.is_disabled;
        let radius = radii(t.radius.inner);
        let node = Node::new(Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            gap: taffy::Size {
                width: len(t.space.s0_5),
                height: len(0.0),
            },
            min_size: taffy::Size {
                width: taffy::LengthPercentageAuto::length(0.0),
                height: taffy::LengthPercentageAuto::auto(),
            },
            flex_grow: if self.display == LinkDisplay::Block {
                1.0
            } else {
                0.0
            },
            ..Style::default()
        })
        .interact(Interact::new(self.on_click, self.is_disabled).href(self.href.as_deref()))
        .indicator(Indicator::ring(color(c.accent), radius, None))
        .sem(sem)
        .tooltip(self.tooltip);
        cx.key(self.base.id.as_deref());
        cx.begin(node);
        let mut label = text_node(self.children, style, ink);
        let draw = text_draw(&mut label);
        if self.max_lines > 0 {
            draw.max_lines = Some(self.max_lines);
        }
        label.sem = None;
        let underline = self.has_underline;
        cx.leaf(label);
        if underline {
            // A 1 point rule under the words, in the text color.
            let line_color = mix(ink, ink, 0.0);
            cx.leaf(
                Node::new(Style {
                    position: taffy::Position::Absolute,
                    inset: taffy::Rect {
                        left: taffy::LengthPercentageAuto::length(0.0),
                        right: taffy::LengthPercentageAuto::length(0.0),
                        top: taffy::LengthPercentageAuto::auto(),
                        bottom: taffy::LengthPercentageAuto::length(
                            (style.size * style.leading - style.size) / 2.0 - 1.0,
                        ),
                    },
                    size: taffy::Size {
                        width: taffy::Dimension::auto(),
                        height: taffy::Dimension::length(1.0),
                    },
                    ..Style::default()
                })
                .draw(astryx_core::authoring::Draw::Box(
                    astryx_core::authoring::BoxDraw::fill(line_color, radii(0.0)),
                )),
            );
        }
        if self.is_external_link {
            let side = (style.size * 0.875).round();
            if let Some(icon) = icon_node("externalLink", side, ink) {
                cx.leaf(icon);
            }
        }
        cx.end();
    }
}
