//! Avatar, AvatarStatusDot, and Thumbnail.
//!
//! The display list has no image command yet, so `src` is kept but not
//! drawn: an Avatar shows its initials or person symbol and a Thumbnail its
//! placeholder, as both do while an image loads.

use super::display::{icon_node, Icon, IconSize};
use super::{dim, len_auto, radii, square};
use crate::contract::{Color, FontFamily, PathCmd, Point, Radii, Role, TextAlign, TextStyle};
use crate::element::{
    base_props, Base, BoxDraw, Build, BuildCx, Draw, Indicator, Interact, Node, PathDraw,
    Semantics, TextDraw,
};
use taffy::{AlignItems, FlexDirection, JustifyContent, LengthPercentageAuto, Position, Style};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AvatarSize {
    /// 20 points.
    Xsm,
    /// 24 points.
    Sm,
    /// 36 points.
    #[default]
    Md,
    /// 48 points.
    Lg,
    /// 128 points.
    Xl,
}

impl AvatarSize {
    pub fn points(self) -> f32 {
        match self {
            AvatarSize::Xsm => 20.0,
            AvatarSize::Sm => 24.0,
            AvatarSize::Md => 36.0,
            AvatarSize::Lg => 48.0,
            AvatarSize::Xl => 128.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AvatarShape {
    #[default]
    Circle,
    Rounded,
    Square,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AvatarStatusDotVariant {
    /// A filled dot.
    #[default]
    Success,
    /// A ring.
    Neutral,
    /// A dot with a minus bar.
    Error,
}

/// A presence dot for an Avatar's `status` slot (`AvatarStatusDot`). Each
/// variant pairs a color with a shape, so status never rests on color alone.
#[derive(Clone, Debug, Default)]
pub struct AvatarStatusDot {
    base: Base,
    variant: AvatarStatusDotVariant,
    label: Option<String>,
}

impl AvatarStatusDot {
    pub fn new(variant: AvatarStatusDotVariant) -> Self {
        AvatarStatusDot {
            variant,
            ..Self::default()
        }
    }
    /// Names the status, such as "Online"; an Avatar reads it after its name.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

base_props!(AvatarStatusDot);

/// Dot side, border, and glyph stroke for an avatar side: small up to 36
/// points, medium up to 72, large beyond.
fn dot_tier(avatar_side: Option<f32>) -> (f32, f32, f32) {
    match avatar_side.unwrap_or(36.0) {
        s if s <= 36.0 => (10.0, 1.0, 1.0),
        s if s <= 72.0 => (20.0, 2.0, 1.5),
        _ => (32.0, 4.0, 2.0),
    }
}

impl AvatarStatusDot {
    fn build_in<Msg>(&self, cx: &mut BuildCx<Msg>, avatar_side: Option<f32>, sem: bool) {
        if self.base.is_hidden {
            return;
        }
        let c = &cx.tokens.color;
        let (dot, border, stroke) = dot_tier(avatar_side);
        let (plate, ink): (Color, Color) = match self.variant {
            AvatarStatusDotVariant::Success => (c.success.into(), c.on_success.into()),
            AvatarStatusDotVariant::Neutral => {
                (c.background_surface.into(), c.text_secondary.into())
            }
            AvatarStatusDotVariant::Error => (c.error.into(), c.on_error.into()),
        };
        let pill = radii(9999.0);
        let mut node = Node::new(Style {
            size: square(dot),
            flex_shrink: 0.0,
            align_items: AlignItems::CENTER,
            justify_content: JustifyContent::CENTER,
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw {
            border: Some((c.background_surface.into(), border)),
            ..BoxDraw::fill(plate, pill)
        }));
        if sem {
            node.sem = self.label.clone().map(|l| Semantics::new(Role::Image, l));
        }
        cx.key(self.base.id.as_deref());
        cx.begin(node);
        let field = dot - border * 2.0;
        match self.variant {
            AvatarStatusDotVariant::Neutral => {
                cx.leaf(
                    Node::new(Style {
                        size: square(field),
                        ..Style::default()
                    })
                    .draw(Draw::Box(BoxDraw {
                        border: Some((ink, stroke)),
                        ..BoxDraw::fill(Color::TRANSPARENT, pill)
                    })),
                );
            }
            AvatarStatusDotVariant::Error => {
                cx.leaf(
                    Node::new(Style {
                        size: taffy::Size {
                            width: dim(field * 0.75),
                            height: dim(stroke),
                        },
                        ..Style::default()
                    })
                    .draw(Draw::Box(BoxDraw::fill(ink, pill))),
                );
            }
            AvatarStatusDotVariant::Success => {}
        }
        cx.end();
    }
}

impl<Msg> Build<Msg> for AvatarStatusDot {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        self.build_in(cx, None, true);
    }
}

/// What an Avatar's `status` slot holds.
#[derive(Clone, Debug)]
pub enum AvatarStatus {
    Dot(AvatarStatusDot),
    Icon(Icon),
}

impl From<AvatarStatusDot> for AvatarStatus {
    fn from(dot: AvatarStatusDot) -> Self {
        AvatarStatus::Dot(dot)
    }
}

impl From<Icon> for AvatarStatus {
    fn from(icon: Icon) -> Self {
        AvatarStatus::Icon(icon)
    }
}

/// A person or entity (`Avatar`): initials from `name` on a neutral plate,
/// or a person symbol. With `on_click` it is a focus stop.
pub struct Avatar<Msg> {
    base: Base,
    src: Option<String>,
    fallback_src: Option<String>,
    name: Option<String>,
    alt: Option<String>,
    size: AvatarSize,
    shape: AvatarShape,
    tooltip: Option<String>,
    href: Option<String>,
    on_click: Option<Msg>,
    status: Option<AvatarStatus>,
}

impl<Msg> Default for Avatar<Msg> {
    fn default() -> Self {
        Avatar {
            base: Base::default(),
            src: None,
            fallback_src: None,
            name: None,
            alt: None,
            size: AvatarSize::default(),
            shape: AvatarShape::default(),
            tooltip: None,
            href: None,
            on_click: None,
            status: None,
        }
    }
}

impl<Msg> Avatar<Msg> {
    pub fn new() -> Self {
        Self::default()
    }
    /// The person's or entity's name, for initials and the accessible name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    /// Kept for the Astryx contract; images are not drawn yet.
    pub fn src(mut self, src: impl Into<String>) -> Self {
        self.src = Some(src.into());
        self
    }
    /// Kept for the Astryx contract; images are not drawn yet.
    pub fn fallback_src(mut self, src: impl Into<String>) -> Self {
        self.fallback_src = Some(src.into());
        self
    }
    /// The accessible name, when it differs from `name`.
    pub fn alt(mut self, alt: impl Into<String>) -> Self {
        self.alt = Some(alt.into());
        self
    }
    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }
    pub fn shape(mut self, shape: AvatarShape) -> Self {
        self.shape = shape;
        self
    }
    /// Kept for the Astryx contract; tooltips are not drawn yet.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
    /// Kept for the Astryx contract; the core does not open links.
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }
    pub fn on_click(mut self, msg: Msg) -> Self {
        self.on_click = Some(msg);
        self
    }
    /// A status dot or icon in the bottom trailing corner.
    pub fn status(mut self, status: impl Into<AvatarStatus>) -> Self {
        self.status = Some(status.into());
        self
    }
}

base_props!(Avatar<Msg>);

/// Astryx's initials: the first letter, digit, or emoji of the first and
/// last words that have one, uppercased.
pub(crate) fn initials(name: &str) -> String {
    let stands = |c: &char| {
        c.is_alphanumeric() || (*c as u32 > 0x7f && !c.is_whitespace() && !c.is_ascii_punctuation())
    };
    let letters: Vec<char> = name
        .split_whitespace()
        .filter_map(|w| w.chars().find(stands))
        .collect();
    match letters.as_slice() {
        [] => String::new(),
        [one] => one.to_uppercase().collect(),
        [first, .., last] => first.to_uppercase().chain(last.to_uppercase()).collect(),
    }
}

fn p(x: f32, y: f32) -> Point {
    Point { x, y }
}

/// A head and shoulders in a 24-point box.
fn person_path() -> Vec<PathCmd> {
    let k = 0.552_284_8 * 4.0;
    vec![
        PathCmd::MoveTo(p(16.0, 8.0)),
        PathCmd::CubicTo(p(16.0, 8.0 + k), p(12.0 + k, 12.0), p(12.0, 12.0)),
        PathCmd::CubicTo(p(12.0 - k, 12.0), p(8.0, 8.0 + k), p(8.0, 8.0)),
        PathCmd::CubicTo(p(8.0, 8.0 - k), p(12.0 - k, 4.0), p(12.0, 4.0)),
        PathCmd::CubicTo(p(12.0 + k, 4.0), p(16.0, 8.0 - k), p(16.0, 8.0)),
        PathCmd::Close,
        PathCmd::MoveTo(p(4.0, 20.0)),
        PathCmd::CubicTo(p(4.0, 16.5), p(7.5, 14.0), p(12.0, 14.0)),
        PathCmd::CubicTo(p(16.5, 14.0), p(20.0, 16.5), p(20.0, 20.0)),
        PathCmd::LineTo(p(20.0, 21.0)),
        PathCmd::LineTo(p(4.0, 21.0)),
        PathCmd::Close,
    ]
}

fn meaningful(s: &Option<String>) -> Option<&str> {
    s.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

impl<Msg> Build<Msg> for Avatar<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let side = self.size.points();
        let shape_radii = match self.shape {
            AvatarShape::Circle => radii(t.radius.full),
            AvatarShape::Rounded => radii(t.radius.element),
            AvatarShape::Square => Radii::default(),
        };
        let status_label = match &self.status {
            Some(AvatarStatus::Dot(d)) => d.label.clone(),
            Some(AvatarStatus::Icon(i)) => i.label_text().map(str::to_string),
            None => None,
        };
        let name = meaningful(&self.alt).or(meaningful(&self.name));
        let label = match (name, status_label.as_deref()) {
            (Some(n), Some(s)) => Some(format!("{n}, {s}")),
            (n, s) => n.or(s).map(str::to_string),
        };
        let mut node = Node::new(Style {
            size: square(side),
            flex_shrink: 0.0,
            ..Style::default()
        });
        if let Some(msg) = self.on_click {
            node = node
                .interact(Interact::new(Some(msg), false))
                .indicator(Indicator::ring(
                    c.accent.into(),
                    shape_radii,
                    Some(c.overlay_pressed.into()),
                ))
                .sem(Semantics::new(
                    Role::Button,
                    label.clone().unwrap_or_default(),
                ));
        } else if let Some(label) = label {
            node = node.sem(Semantics::new(Role::Image, label));
        }
        cx.key(self.base.id.as_deref());
        cx.begin(node);
        let mut plate = Node::new(Style {
            size: taffy::Size {
                width: taffy::Dimension::percent(1.0),
                height: taffy::Dimension::percent(1.0),
            },
            align_items: AlignItems::CENTER,
            justify_content: JustifyContent::CENTER,
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw::fill(c.neutral.into(), shape_radii)));
        plate.clip = Some(shape_radii);
        cx.begin(plate);
        let letters = self.name.as_deref().map(initials).unwrap_or_default();
        if letters.is_empty() {
            cx.leaf(
                Node::new(Style {
                    size: square(side * 0.6),
                    ..Style::default()
                })
                .draw(Draw::Path(PathDraw {
                    commands: person_path(),
                    view_box: 24.0,
                    fill: Some(c.text_secondary.into()),
                    stroke: None,
                })),
            );
        } else {
            let style = TextStyle {
                family: FontFamily::Body,
                size: side * 0.4,
                weight: t.font.weight.medium,
                leading: 1.2,
            };
            cx.leaf(Node::new(Style::default()).draw(Draw::Text(TextDraw {
                text: letters,
                style,
                color: c.text_secondary.into(),
                align: TextAlign::Center,
                max_lines: Some(1),
                strikethrough: false,
            })));
        }
        cx.end();
        if let Some(status) = self.status {
            // On a circle the status centers on the edge at the bottom
            // trailing 45 degree point; otherwise it overhangs the corner by a
            // quarter of its size.
            let status_side = match &status {
                AvatarStatus::Dot(_) => dot_tier(Some(side)).0,
                AvatarStatus::Icon(i) => i.side(),
            };
            let offset = match self.shape {
                AvatarShape::Circle => {
                    status_side / 2.0 - side * (1.0 - std::f32::consts::FRAC_1_SQRT_2) / 2.0
                }
                AvatarShape::Rounded | AvatarShape::Square => status_side / 4.0,
            };
            cx.begin(Node::new(Style {
                position: Position::Absolute,
                inset: taffy::Rect {
                    right: len_auto(-offset),
                    bottom: len_auto(-offset),
                    left: LengthPercentageAuto::auto(),
                    top: LengthPercentageAuto::auto(),
                },
                flex_direction: FlexDirection::Row,
                ..Style::default()
            }));
            match status {
                AvatarStatus::Dot(d) => d.build_in(cx, Some(side), false),
                AvatarStatus::Icon(i) => {
                    if let Some(n) = icon_node::<Msg>(i.name(), i.side(), i.resolved_color(cx)) {
                        cx.leaf(n);
                    }
                }
            }
            cx.end();
        }
        cx.end();
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ThumbnailShowRemoveOn {
    Always,
    #[default]
    Hover,
}

/// A 64 point preview tile (`Thumbnail`) with an optional remove button and
/// an optional press to open it. Each is its own focus stop.
pub struct Thumbnail<Msg> {
    base: Base,
    src: Option<String>,
    alt: Option<String>,
    label: Option<String>,
    on_remove: Option<Msg>,
    on_click: Option<Msg>,
    is_loading: bool,
    is_disabled: bool,
    show_remove_on: ThumbnailShowRemoveOn,
}

impl<Msg> Default for Thumbnail<Msg> {
    fn default() -> Self {
        Thumbnail {
            base: Base::default(),
            src: None,
            alt: None,
            label: None,
            on_remove: None,
            on_click: None,
            is_loading: false,
            is_disabled: false,
            show_remove_on: ThumbnailShowRemoveOn::default(),
        }
    }
}

impl<Msg> Thumbnail<Msg> {
    pub fn new() -> Self {
        Self::default()
    }
    /// Kept for the Astryx contract; images are not drawn yet.
    pub fn src(mut self, src: impl Into<String>) -> Self {
        self.src = Some(src.into());
        self
    }
    pub fn alt(mut self, alt: impl Into<String>) -> Self {
        self.alt = Some(alt.into());
        self
    }
    /// A name for the file, such as "beach.jpg".
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    /// Adds a remove button that sends this message.
    pub fn on_remove(mut self, msg: Msg) -> Self {
        self.on_remove = Some(msg);
        self
    }
    pub fn on_click(mut self, msg: Msg) -> Self {
        self.on_click = Some(msg);
        self
    }
    pub fn is_loading(mut self, on: bool) -> Self {
        self.is_loading = on;
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    /// Kept for the Astryx contract. Focus-first screens have no hover, so
    /// the remove button always shows, as on touch screens.
    pub fn show_remove_on(mut self, show: ThumbnailShowRemoveOn) -> Self {
        self.show_remove_on = show;
        self
    }
}

base_props!(Thumbnail<Msg>);

/// Thumbnail's side in points.
const THUMBNAIL_SIDE: f32 = 64.0;

/// A framed landscape in a 24-point box.
fn image_path() -> Vec<PathCmd> {
    vec![
        PathCmd::MoveTo(p(4.0, 5.0)),
        PathCmd::LineTo(p(20.0, 5.0)),
        PathCmd::LineTo(p(20.0, 19.0)),
        PathCmd::LineTo(p(4.0, 19.0)),
        PathCmd::Close,
        PathCmd::MoveTo(p(4.0, 16.0)),
        PathCmd::LineTo(p(9.0, 11.0)),
        PathCmd::LineTo(p(13.0, 15.0)),
        PathCmd::LineTo(p(15.5, 12.5)),
        PathCmd::LineTo(p(20.0, 17.0)),
    ]
}

impl<Msg> Build<Msg> for Thumbnail<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let radius = t.radius.element;
        let name = meaningful(&self.alt)
            .or(meaningful(&self.label))
            .unwrap_or("Thumbnail")
            .to_string();
        let mut node = Node::new(Style {
            size: square(THUMBNAIL_SIDE),
            flex_shrink: 0.0,
            align_items: AlignItems::CENTER,
            justify_content: JustifyContent::CENTER,
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw::fill(c.neutral.into(), radii(radius))))
        .opacity(if self.is_disabled { 0.5 } else { 1.0 });
        let mut sem = match &self.on_click {
            Some(_) => Semantics::new(Role::Button, format!("Open {name}")),
            None => Semantics::new(Role::Image, name.clone()),
        };
        sem.disabled = self.is_disabled;
        node.sem = Some(sem);
        if let Some(msg) = self.on_click {
            node = node
                .interact(Interact::new(Some(msg), self.is_disabled).busy(self.is_loading))
                .indicator(Indicator::ring(
                    c.accent.into(),
                    radii(radius),
                    Some(c.overlay_pressed.into()),
                ));
        }
        node.clip = Some(radii(radius));
        cx.key(self.base.id.as_deref());
        cx.begin(node);
        cx.leaf(
            Node::new(Style {
                size: square(24.0),
                ..Style::default()
            })
            .draw(Draw::Path(PathDraw {
                commands: image_path(),
                view_box: 24.0,
                fill: None,
                stroke: Some((c.icon_secondary.into(), 1.5)),
            })),
        );
        if self.is_loading {
            cx.leaf(
                Node::new(Style {
                    position: Position::Absolute,
                    size: taffy::Size {
                        width: taffy::Dimension::percent(1.0),
                        height: taffy::Dimension::percent(1.0),
                    },
                    ..Style::default()
                })
                .draw(Draw::Box(BoxDraw::fill(c.overlay.into(), radii(0.0)))),
            );
            let mut spinner = Node::new(Style {
                position: Position::Absolute,
                size: square(16.0),
                ..Style::default()
            })
            .draw(Draw::Spinner(c.on_dark.into()));
            spinner.centered = true;
            cx.leaf(spinner);
            cx.animate();
        }
        if let Some(msg) = self.on_remove {
            let inner = (radius - t.space.s1).max(0.0);
            let mut sem = Semantics::new(Role::Button, format!("Remove {name}"));
            sem.disabled = self.is_disabled;
            cx.begin(
                Node::new(Style {
                    position: Position::Absolute,
                    inset: taffy::Rect {
                        top: len_auto(t.space.s1),
                        right: len_auto(t.space.s1),
                        left: LengthPercentageAuto::auto(),
                        bottom: LengthPercentageAuto::auto(),
                    },
                    size: square(20.0),
                    align_items: AlignItems::CENTER,
                    justify_content: JustifyContent::CENTER,
                    ..Style::default()
                })
                .draw(Draw::Box(BoxDraw::fill(c.overlay.into(), radii(inner))))
                .interact(Interact::new(Some(msg), self.is_disabled))
                .indicator(Indicator::ring(
                    c.accent.into(),
                    radii(inner),
                    Some(c.overlay_pressed.into()),
                ))
                .sem(sem),
            );
            if let Some(icon) = icon_node("close", IconSize::Xsm.points(), c.on_dark.into()) {
                cx.leaf(icon);
            }
            cx.end();
        }
        cx.end();
    }
}
