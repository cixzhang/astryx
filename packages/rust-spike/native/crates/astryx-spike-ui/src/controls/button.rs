//! Button, IconButton and ButtonGroup.

use crate::layout::{size_dim, SizeValue};
use crate::primitives::{spinner_painter, Badge, Icon, IconSize};
use crate::util::{color, component, len, radii};
use astryx_core::authoring::{
    text_draw, text_node, type_style, Base, BoxDraw, Build, BuildCx, Draw, Indicator, Interact,
    Node, Semantics,
};
use astryx_core::contract::{Color, FontFamily, Live, Radii, Role};
use astryx_core::{Element, Elevation};
use astryx_spike_foundations::{args, provide_size, ComponentSize, CxExt};
use std::rc::Rc;
use taffy::{
    AlignItems, Dimension, FlexDirection, JustifyContent, LengthPercentageAuto, Position, Style,
};

/// Button `size`, shared with the size cascade (ButtonGroup, InputGroup).
pub type ButtonSize = ComponentSize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonVariant {
    Primary,
    #[default]
    Secondary,
    Ghost,
    Destructive,
}

/// What follows the label: an icon or a badge.
pub enum EndContent {
    Icon(Icon),
    Badge(Badge),
}

impl From<Icon> for EndContent {
    fn from(i: Icon) -> Self {
        EndContent::Icon(i)
    }
}

impl From<Badge> for EndContent {
    fn from(b: Badge) -> Self {
        EndContent::Badge(b)
    }
}

/// ButtonGroup `orientation`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Which group a member is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GroupKind {
    /// A ButtonGroup: members draw a divider on their leading edge.
    Buttons,
    /// An InputGroup: members overlap their borders by one point.
    Inputs,
}

/// What a ButtonGroup tells each member: its orientation, whether the group
/// is disabled, and where the member sits, so it squares its inner corners
/// and draws the divider on its leading edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GroupMember {
    pub kind: GroupKind,
    pub orientation: Orientation,
    pub is_disabled: bool,
    pub first: bool,
    pub last: bool,
    pub group: u64,
}

/// A labeled action (`Button`). Disabled and loading buttons keep their
/// focus stop, as Astryx keeps them focusable.
pub struct Button<Msg> {
    base: Base,
    label: String,
    variant: ButtonVariant,
    size: Option<ButtonSize>,
    elevation: Elevation,
    is_loading: bool,
    is_interruptible: bool,
    is_disabled: bool,
    icon: Option<Icon>,
    is_icon_only: bool,
    width: Option<SizeValue>,
    end_content: Option<EndContent>,
    tooltip: Option<String>,
    href: Option<String>,
    on_click: Option<Msg>,
    /// The tooltip words, kept as the description once a Tooltip shows them.
    described: Option<String>,
    /// Runs on press before `on_click`, for components that keep their own
    /// state (a DropdownMenu's trigger).
    effect: Option<Rc<dyn Fn()>>,
}

impl<Msg> Button<Msg> {
    pub fn new(label: impl Into<String>) -> Self {
        Button {
            base: Base::default(),
            label: label.into(),
            variant: ButtonVariant::Secondary,
            size: None,
            elevation: Elevation::None,
            is_loading: false,
            is_interruptible: false,
            is_disabled: false,
            icon: None,
            is_icon_only: false,
            width: None,
            end_content: None,
            tooltip: None,
            href: None,
            on_click: None,
            described: None,
            effect: None,
        }
    }
    pub(crate) fn effect(mut self, f: Rc<dyn Fn()>) -> Self {
        self.effect = Some(f);
        self
    }
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
    /// Without a size, a button takes its group's size, else `md`.
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = Some(size);
        self
    }
    pub fn elevation(mut self, elevation: Elevation) -> Self {
        self.elevation = elevation;
        self
    }
    /// A spinner over the label; presses are ignored unless interruptible.
    /// Astryx's `clickAction` promise is the app's message plus this flag.
    pub fn is_loading(mut self, on: bool) -> Self {
        self.is_loading = on;
        self
    }
    pub fn is_interruptible(mut self, on: bool) -> Self {
        self.is_interruptible = on;
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    /// An icon before the label, sized to the button.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
    /// Shows only the icon; the label still names the button.
    pub fn is_icon_only(mut self, on: bool) -> Self {
        self.is_icon_only = on;
        self
    }
    pub fn width(mut self, width: SizeValue) -> Self {
        self.width = Some(width);
        self
    }
    pub fn end_content(mut self, content: impl Into<EndContent>) -> Self {
        self.end_content = Some(content.into());
        self
    }
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
    /// A link styled as a button; the shell opens it after `on_click`.
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }
    pub fn on_click(mut self, msg: Msg) -> Self {
        self.on_click = Some(msg);
        self
    }
}

component!(Button<Msg>);

/// An icon-only button (`IconButton`): the label names it and shows as its
/// tooltip.
pub struct IconButton<Msg>(Button<Msg>);

impl<Msg> IconButton<Msg> {
    pub fn new(label: impl Into<String>, icon: Icon) -> Self {
        let label = label.into();
        IconButton(
            Button::new(label.clone())
                .icon(icon)
                .is_icon_only(true)
                .tooltip(label),
        )
    }
    pub fn variant(self, v: ButtonVariant) -> Self {
        IconButton(self.0.variant(v))
    }
    pub fn size(self, s: ButtonSize) -> Self {
        IconButton(self.0.size(s))
    }
    pub fn elevation(self, e: Elevation) -> Self {
        IconButton(self.0.elevation(e))
    }
    pub fn is_loading(self, on: bool) -> Self {
        IconButton(self.0.is_loading(on))
    }
    pub fn is_interruptible(self, on: bool) -> Self {
        IconButton(self.0.is_interruptible(on))
    }
    pub fn is_disabled(self, on: bool) -> Self {
        IconButton(self.0.is_disabled(on))
    }
    pub fn tooltip(self, t: impl Into<String>) -> Self {
        IconButton(self.0.tooltip(t))
    }
    pub fn width(self, w: SizeValue) -> Self {
        IconButton(self.0.width(w))
    }
    pub fn href(self, h: impl Into<String>) -> Self {
        IconButton(self.0.href(h))
    }
    pub fn on_click(self, msg: Msg) -> Self {
        IconButton(self.0.on_click(msg))
    }
    pub fn id(self, id: impl Into<String>) -> Self {
        IconButton(self.0.id(id))
    }
}

impl<Msg: Clone + 'static> From<IconButton<Msg>> for Element<Msg> {
    fn from(b: IconButton<Msg>) -> Self {
        b.0.into()
    }
}

/// The fill and ink of a variant.
pub(crate) fn variant_colors<Msg>(cx: &BuildCx<Msg>, v: ButtonVariant) -> (Color, Color) {
    let c = &cx.tokens.color;
    match v {
        ButtonVariant::Primary => (color(c.accent), color(c.on_accent)),
        ButtonVariant::Secondary => (color(c.neutral), color(c.text_primary)),
        ButtonVariant::Ghost => (Color::TRANSPARENT, color(c.text_primary)),
        ButtonVariant::Destructive => (color(c.error), color(c.on_error)),
    }
}

pub(crate) fn element_height(t: &astryx_core::Tokens, size: ComponentSize) -> f32 {
    match size {
        ComponentSize::Sm => t.size.element_sm,
        ComponentSize::Md => t.size.element_md,
        ComponentSize::Lg => t.size.element_lg,
    }
}

/// Corner radii for a member of a connected group: only its outer corners
/// round. Radii run top-left, top-right, bottom-right, bottom-left.
pub(crate) fn member_radii(r: f32, m: Option<GroupMember>, rtl: bool) -> Radii {
    let Some(m) = m else {
        return Radii::all(r);
    };
    let (lead, trail) = (if m.first { r } else { 0.0 }, if m.last { r } else { 0.0 });
    match m.orientation {
        Orientation::Horizontal => {
            let (left, right) = if rtl { (trail, lead) } else { (lead, trail) };
            Radii([left, right, right, left])
        }
        Orientation::Vertical => Radii([lead, lead, trail, trail]),
    }
}

impl<Msg: Clone + 'static> Build<Msg> for Button<Msg> {
    fn build(mut self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        // A tooltip shows on hover and keyboard focus, as Astryx's does.
        if let Some(tip) = self.tooltip.take() {
            let id = self.base.id.clone().unwrap_or_else(|| {
                let h = self.label.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
                    (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
                });
                format!("button-tip-{h:x}")
            });
            self.described = Some(tip.clone());
            let inner: Element<Msg> = astryx_core::authoring::element(*self);
            cx.child(crate::overlays::Tooltip::new(id, inner, tip).into());
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let member = cx.context::<GroupMember>().map(|m| *m);
        let size = cx.size(self.size, ComponentSize::Md);
        let height = element_height(t, size);
        let disabled = self.is_disabled || member.is_some_and(|m| m.is_disabled);
        let (bg, fg) = variant_colors(cx, self.variant);
        // The ring matches the variant it rings, so a red button does not
        // read as another control's focus.
        let ring = if self.variant == ButtonVariant::Destructive {
            color(c.error)
        } else {
            color(c.accent)
        };
        let icon_size = match size {
            ComponentSize::Lg => IconSize::Md,
            _ => IconSize::Sm,
        };
        let radius = member_radii(t.radius.element, member, cx.rtl());
        let pad = if self.is_icon_only { 0.0 } else { t.space.s3 };
        let mut style = Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            justify_content: JustifyContent::CENTER,
            gap: taffy::Size {
                width: len(t.space.s2),
                height: len(0.0),
            },
            padding: taffy::Rect {
                left: len(pad),
                right: len(pad),
                top: len(0.0),
                bottom: len(0.0),
            },
            size: taffy::Size {
                width: if self.is_icon_only {
                    Dimension::length(height)
                } else {
                    self.width.map_or(Dimension::max_content(), size_dim)
                },
                height: Dimension::auto(),
            },
            min_size: taffy::Size {
                width: LengthPercentageAuto::length(0.0),
                height: LengthPercentageAuto::length(height),
            },
            max_size: taffy::Size {
                width: if self.is_icon_only {
                    LengthPercentageAuto::auto()
                } else {
                    LengthPercentageAuto::percent(1.0)
                },
                height: LengthPercentageAuto::auto(),
            },
            flex_shrink: if self.is_icon_only { 0.0 } else { 1.0 },
            ..Style::default()
        };
        if let Some(m) = member {
            style.align_self = Some(AlignItems::STRETCH);
            if m.kind == GroupKind::Inputs && !m.first {
                let bw = t.size.border_width;
                let (l, r) = cx.inline(-bw, 0.0);
                style.margin.left = LengthPercentageAuto::length(l);
                style.margin.right = LengthPercentageAuto::length(r);
            }
        }
        let busy = self.is_loading && !self.is_interruptible;
        let mut sem = Semantics::new(Role::Button, self.label.clone());
        sem.disabled = disabled;
        if self.is_loading {
            sem.value = Some(cx.t("@astryx.button.loading", &args! {}));
        }
        if let Some(tip) = &self.described {
            if tip.trim() != self.label.trim() {
                sem.description = Some(tip.clone());
            }
        }
        let mut node = Node::new(style)
            .draw(Draw::Box(BoxDraw {
                shadow: crate::layout::elevation_layers(self.elevation, t),
                ..BoxDraw::fill(bg, radius)
            }))
            .interact({
                let mut i = Interact::new(self.on_click, disabled)
                    .busy(busy)
                    .href(self.href.as_deref());
                if let Some(e) = self.effect {
                    i = i.effect(move || e());
                }
                i
            })
            .indicator(Indicator::ring(
                ring,
                radius,
                Some(color(c.overlay_pressed)),
            ))
            .sem(sem)
            .opacity(if disabled { 0.5 } else { 1.0 });
        if member.is_none() {
            node = node.press_scale();
        }
        if let Some(m) = member.filter(|m| m.kind == GroupKind::Buttons) {
            node.group = Some(m.group);
            node.group_entry = m.first;
        }
        cx.key(self.base.id.as_deref());
        cx.begin(node);
        if let Some(m) = member.filter(|m| !m.first && m.kind == GroupKind::Buttons) {
            // The divider on the member's leading edge.
            let solid = matches!(
                self.variant,
                ButtonVariant::Primary | ButtonVariant::Destructive
            );
            let ink = if solid {
                color(c.on_accent)
            } else {
                color(c.border)
            };
            divider(cx, m.orientation, ink, t.size.border_width);
        }
        let label_color = if self.is_loading {
            Color::TRANSPARENT
        } else {
            fg
        };
        if let Some(icon) = self.icon {
            let mut wrap = Node::new(Style {
                align_items: AlignItems::CENTER,
                justify_content: JustifyContent::CENTER,
                flex_shrink: 0.0,
                ..Style::default()
            });
            wrap.opacity = if self.is_loading { 0.0 } else { 1.0 };
            cx.begin(wrap);
            // The icon takes the button's ink unless it sets its own color.
            let pushed = astryx_core::authoring::Inherited {
                color: fg,
                ..cx.inherited()
            };
            cx.inherit.push(pushed);
            cx.child(icon.default_size(icon_size).into());
            cx.inherit.pop();
            cx.end();
        }
        if !self.is_icon_only {
            let mut style = type_style(t.font.label, FontFamily::Body);
            style.weight = t.font.weight.medium;
            let mut label = text_node(self.label, style, label_color);
            text_draw(&mut label).max_lines = Some(1);
            label.style.min_size.width = LengthPercentageAuto::length(0.0);
            label.style.flex_shrink = 1.0;
            label.sem = None;
            cx.leaf(label);
            if let Some(end) = self.end_content {
                let pushed = astryx_core::authoring::Inherited {
                    color: fg,
                    ..cx.inherited()
                };
                cx.inherit.push(pushed);
                match end {
                    EndContent::Icon(i) => cx.child(i.default_size(icon_size).into()),
                    EndContent::Badge(b) => cx.child(b.into()),
                }
                cx.inherit.pop();
            }
        }
        // The busy start is announced politely.
        let loading_words = cx.t("@astryx.button.loading", &args! {});
        cx.live_region(
            self.is_loading.then_some(loading_words.as_str()),
            Live::Polite,
        );
        if self.is_loading {
            let (d, s) = crate::primitives::SpinnerSize::Sm.geometry();
            let side = d + 2.0 * s;
            let mut spinner = Node::new(Style {
                position: Position::Absolute,
                size: taffy::Size {
                    width: Dimension::length(side),
                    height: Dimension::length(side),
                },
                ..Style::default()
            })
            .draw(Draw::Paint(spinner_painter(cx, d, s, fg)));
            spinner.centered = true;
            cx.leaf(spinner);
        }
        cx.end();
    }
}

/// A 1 point rule on the leading edge of the open node.
fn divider<Msg>(cx: &mut BuildCx<Msg>, o: Orientation, ink: Color, w: f32) {
    let zero = LengthPercentageAuto::length(0.0);
    let auto = LengthPercentageAuto::auto();
    let (inset, size) = match o {
        Orientation::Horizontal => {
            let (left, right) = cx.inline(zero, auto);
            (
                taffy::Rect {
                    left,
                    right,
                    top: zero,
                    bottom: zero,
                },
                taffy::Size {
                    width: Dimension::length(w),
                    height: Dimension::auto(),
                },
            )
        }
        Orientation::Vertical => (
            taffy::Rect {
                left: zero,
                right: zero,
                top: zero,
                bottom: auto,
            },
            taffy::Size {
                width: Dimension::auto(),
                height: Dimension::length(w),
            },
        ),
    };
    cx.leaf(
        Node::new(Style {
            position: Position::Absolute,
            inset,
            size,
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw::fill(ink, radii(0.0)))),
    );
}

/// Connected buttons (`ButtonGroup`): one tab stop, arrow keys move between
/// members, and the group's size and disabled state reach every member
/// through context, including members nested in other components.
pub struct ButtonGroup<Msg> {
    base: Base,
    label: String,
    orientation: Orientation,
    size: Option<ButtonSize>,
    is_disabled: bool,
    elevation: Elevation,
    children: Vec<Element<Msg>>,
}

impl<Msg> ButtonGroup<Msg> {
    pub fn new(label: impl Into<String>) -> Self {
        ButtonGroup {
            base: Base::default(),
            label: label.into(),
            orientation: Orientation::Horizontal,
            size: None,
            is_disabled: false,
            elevation: Elevation::None,
            children: Vec::new(),
        }
    }
    pub fn orientation(mut self, o: Orientation) -> Self {
        self.orientation = o;
        self
    }
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = Some(size);
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    pub fn elevation(mut self, e: Elevation) -> Self {
        self.elevation = e;
        self
    }
    pub fn child(mut self, child: impl Into<Element<Msg>>) -> Self {
        self.children.push(child.into());
        self
    }
    pub fn children<E: Into<Element<Msg>>>(
        mut self,
        children: impl IntoIterator<Item = E>,
    ) -> Self {
        self.children.extend(children.into_iter().map(Into::into));
        self
    }
}

component!(ButtonGroup<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for ButtonGroup<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let size = cx.size(self.size, ComponentSize::Md);
        let radius = radii(if self.elevation == Elevation::None {
            0.0
        } else {
            t.radius.element
        });
        cx.key(self.base.id.as_deref());
        // a11y-role: Group with aria-label (role="group").
        let ix = cx.begin(
            Node::new(Style {
                flex_direction: match self.orientation {
                    Orientation::Horizontal => FlexDirection::Row,
                    Orientation::Vertical => FlexDirection::Column,
                },
                align_items: AlignItems::STRETCH,
                align_self: Some(AlignItems::FLEX_START),
                ..Style::default()
            })
            .draw(Draw::Box(BoxDraw {
                shadow: crate::layout::elevation_layers(self.elevation, t),
                ..BoxDraw::fill(Color::TRANSPARENT, radius)
            }))
            .sem(Semantics::new(Role::Group, self.label)),
        );
        let group = cx.id_of(ix);
        let n = self.children.len();
        for (i, child) in self.children.into_iter().enumerate() {
            let member = GroupMember {
                kind: GroupKind::Buttons,
                orientation: self.orientation,
                is_disabled: self.is_disabled,
                first: i == 0,
                last: i + 1 == n,
                group,
            };
            // Size and position reach the member however deep it sits.
            let el = provide_size(child, Some(size)).provide(Rc::new(member));
            cx.child(el);
        }
        cx.end();
    }
}
