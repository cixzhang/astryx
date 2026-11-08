//! Button.

use super::text::{text_draw, text_node, type_style};
use super::{len, radii, square, strings, Elevation};
use crate::contract::{Color, FontFamily, Role};
use crate::element::BoxDraw;
use crate::element::{
    base_props, Base, Build, BuildCx, Draw, Indicator, Interact, Node, Semantics,
};
use taffy::{
    AlignItems, Dimension, FlexDirection, JustifyContent, LengthPercentageAuto, Position, Style,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonVariant {
    Primary,
    #[default]
    Secondary,
    Ghost,
    Destructive,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonSize {
    Sm,
    #[default]
    Md,
    Lg,
}

/// Button `width`: hug the label, or fill the width offered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonWidth {
    #[default]
    Content,
    Fill,
}

/// A labeled action (`Button`). A activates it; disabled and busy buttons
/// keep their focus stop, as Astryx keeps them focusable.
pub struct Button<Msg> {
    base: Base,
    label: String,
    variant: ButtonVariant,
    size: ButtonSize,
    elevation: Elevation,
    is_loading: bool,
    is_interruptible: bool,
    is_disabled: bool,
    width: ButtonWidth,
    tooltip: Option<String>,
    href: Option<String>,
    on_click: Option<Msg>,
    /// Activates only after a press held this many milliseconds.
    pub(crate) hold: Option<f32>,
}

impl<Msg> Button<Msg> {
    pub fn new(label: impl Into<String>) -> Self {
        Button {
            base: Base::default(),
            label: label.into(),
            variant: ButtonVariant::default(),
            size: ButtonSize::default(),
            elevation: Elevation::None,
            is_loading: false,
            is_interruptible: false,
            is_disabled: false,
            width: ButtonWidth::default(),
            tooltip: None,
            href: None,
            on_click: None,
            hold: None,
        }
    }
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }
    pub fn elevation(mut self, elevation: Elevation) -> Self {
        self.elevation = elevation;
        self
    }
    /// A spinner over the label; presses are ignored unless interruptible.
    pub fn is_loading(mut self, on: bool) -> Self {
        self.is_loading = on;
        self
    }
    /// Presses still land while loading.
    pub fn is_interruptible(mut self, on: bool) -> Self {
        self.is_interruptible = on;
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    pub fn width(mut self, width: ButtonWidth) -> Self {
        self.width = width;
        self
    }
    /// Kept for the Astryx contract; tooltips are not drawn yet.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
    /// A link: a press sends `on_click`, then asks the shell to open it
    /// (`Command::OpenUrl`). Script and HTML `data:` links are refused.
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }
    /// The message a press sends.
    pub fn on_click(mut self, msg: Msg) -> Self {
        self.on_click = Some(msg);
        self
    }
}

base_props!(Button<Msg>);

impl<Msg> Build<Msg> for Button<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let height = match self.size {
            ButtonSize::Sm => t.size.element_sm,
            ButtonSize::Md => t.size.element_md,
            ButtonSize::Lg => t.size.element_lg,
        };
        let (fg, bg): (Color, Color) = match self.variant {
            ButtonVariant::Primary => (c.on_accent.into(), c.accent.into()),
            ButtonVariant::Secondary => (c.text_primary.into(), c.neutral.into()),
            ButtonVariant::Ghost => (c.text_primary.into(), Color::TRANSPARENT),
            ButtonVariant::Destructive => (c.on_error.into(), c.error.into()),
        };
        // The ring matches the variant it rings, so a red button does not
        // read as another control's focus. It takes the error border color,
        // not the fill, so it stays visible on the ground when a theme's
        // error fill is light.
        let ring = if self.variant == ButtonVariant::Destructive {
            c.border_error
        } else {
            c.accent
        };
        let style = Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            justify_content: JustifyContent::CENTER,
            gap: taffy::Size {
                width: len(t.space.s2),
                height: len(0.0),
            },
            padding: taffy::Rect {
                left: len(t.space.s3),
                right: len(t.space.s3),
                top: len(0.0),
                bottom: len(0.0),
            },
            // Content width hugs the label instead of stretching in a column.
            size: taffy::Size {
                width: match self.width {
                    ButtonWidth::Content => Dimension::max_content(),
                    ButtonWidth::Fill => Dimension::percent(1.0),
                },
                height: Dimension::auto(),
            },
            min_size: taffy::Size {
                width: LengthPercentageAuto::length(0.0),
                height: LengthPercentageAuto::length(height),
            },
            max_size: taffy::Size {
                width: LengthPercentageAuto::percent(1.0),
                height: LengthPercentageAuto::auto(),
            },
            flex_shrink: 1.0,
            ..Style::default()
        };
        let radius = radii(t.radius.element);
        let busy = self.is_loading && !self.is_interruptible;
        let mut sem = Semantics::new(Role::Button, self.label.clone());
        sem.disabled = self.is_disabled;
        sem.value = self.is_loading.then(|| strings::LOADING.to_string());
        if self.hold.is_some() {
            sem.description = Some(strings::HOLD_TO_CONFIRM.to_string());
        }
        let node = Node::new(style)
            .draw(Draw::Box(BoxDraw {
                shadow: self.elevation.layers(t),
                ..BoxDraw::fill(bg, radius)
            }))
            .interact(Interact {
                hold: self.hold,
                ..Interact::new(self.on_click, self.is_disabled)
                    .busy(busy)
                    .href(self.href.as_deref())
            })
            .indicator(Indicator::ring(
                ring.into(),
                radius,
                Some(c.overlay_pressed.into()),
            ))
            .sem(sem)
            .press_scale()
            .tooltip(self.tooltip)
            .opacity(if self.is_disabled { 0.5 } else { 1.0 });
        cx.key(self.base.id.as_deref());
        cx.begin(node);
        // The label keeps its place while loading so the width holds.
        // The busy start is announced politely.
        cx.live_region(
            self.is_loading.then_some(super::strings::LOADING),
            crate::contract::Live::Polite,
        );
        let label_color = if self.is_loading {
            Color::TRANSPARENT
        } else {
            fg
        };
        let mut label = text_node(
            self.label,
            type_style(t.font.label, FontFamily::Body),
            label_color,
        );
        text_draw(&mut label).max_lines = Some(1);
        label.style.min_size.width = LengthPercentageAuto::length(0.0);
        cx.leaf(label);
        if self.is_loading {
            let side = t.space.s4;
            let mut spinner = Node::new(Style {
                position: Position::Absolute,
                size: square(side),
                ..Style::default()
            })
            .draw(Draw::Spinner(fg));
            spinner.centered = true;
            cx.leaf(spinner);
            cx.animate();
        }
        cx.end();
    }
}
