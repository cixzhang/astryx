//! List, Collapsible (+ CollapsibleGroup), Toolbar, Pagination and
//! OverflowList.

use crate::controls::{Button, ButtonVariant, ListDensity};
use crate::layout::{SectionDivider, SectionVariant};
use crate::primitives::{icon_node, Icon};
use crate::util::{color, component, len, len_auto, radii};
use astryx_core::authoring::{
    text_draw, text_node, type_style, Base, BoxDraw, Build, BuildCx, Draw, Indicator, Interact,
    Node, Semantics,
};
use astryx_core::contract::{FontFamily, Role};
use astryx_core::{Element, Space};
use astryx_spike_foundations::{args, ComponentSize, CxExt};
use std::rc::Rc;
use taffy::{AlignItems, Dimension, FlexDirection, JustifyContent, LengthPercentageAuto, Style};

pub use astryx_core::{Item, ItemAlign, ItemDensity, ItemLayout, ListItem};

/// List `edge`: how rows meet the container's padding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ListEdge {
    /// Rows inset their content; hover fills stay inside the padding.
    #[default]
    Padded,
    /// Content lines up with the container's content edge.
    Flush,
    /// Row fills reach into the container's padding.
    Outset,
    /// Rows run to the container's edges.
    Bleed,
}

/// List `listStyle`: a marker before each row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ListStyle {
    #[default]
    None,
    Disc,
    Decimal,
    Circle,
}

/// A vertical list of rows (`List`), usually [`ListItem`]s.
pub struct List<Msg> {
    base: Base,
    density: ListDensity,
    has_dividers: bool,
    edge: ListEdge,
    header: Option<Element<Msg>>,
    list_style: ListStyle,
    start: u32,
    label: Option<String>,
    children: Vec<Element<Msg>>,
}

impl<Msg> Default for List<Msg> {
    fn default() -> Self {
        List {
            base: Base::default(),
            density: ListDensity::Balanced,
            has_dividers: false,
            edge: ListEdge::Padded,
            header: None,
            list_style: ListStyle::None,
            start: 1,
            label: None,
            children: Vec::new(),
        }
    }
}

impl<Msg> List<Msg> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn density(mut self, d: ListDensity) -> Self {
        self.density = d;
        self
    }
    pub fn has_dividers(mut self, on: bool) -> Self {
        self.has_dividers = on;
        self
    }
    pub fn edge(mut self, e: ListEdge) -> Self {
        self.edge = e;
        self
    }
    pub fn header(mut self, h: impl Into<Element<Msg>>) -> Self {
        self.header = Some(h.into());
        self
    }
    pub fn list_style(mut self, s: ListStyle) -> Self {
        self.list_style = s;
        self
    }
    /// The first number of a decimal list.
    pub fn start(mut self, n: u32) -> Self {
        self.start = n;
        self
    }
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }
    pub fn child(mut self, c: impl Into<Element<Msg>>) -> Self {
        self.children.push(c.into());
        self
    }
    pub fn children<E: Into<Element<Msg>>>(mut self, cs: impl IntoIterator<Item = E>) -> Self {
        self.children.extend(cs.into_iter().map(Into::into));
        self
    }
}

component!(List<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for List<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let bleed = cx.container_padding.last().copied().unwrap_or(0.0);
        let reach = matches!(self.edge, ListEdge::Outset | ListEdge::Bleed);
        let m = if reach { -bleed } else { 0.0 };
        let sem = match &self.label {
            Some(l) => Semantics::new(Role::List, l.clone()),
            None => Semantics::role(Role::List),
        };
        cx.key(self.base.id.as_deref());
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            margin: taffy::Rect {
                left: len_auto(m),
                right: len_auto(m),
                top: len_auto(0.0),
                bottom: len_auto(0.0),
            },
            ..Style::default()
        }));
        if let Some(h) = self.header {
            cx.child(h);
        }
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Column,
                gap: taffy::Size {
                    width: len(0.0),
                    height: len(if self.has_dividers { 0.0 } else { t.space.s0_5 }),
                },
                ..Style::default()
            })
            .sem(sem),
        );
        let n = self.children.len();
        let pad_block = super::super::controls::density_pad(t, self.density) - t.space.s2;
        for (i, child) in self.children.into_iter().enumerate() {
            cx.key_slot(i as u64);
            cx.begin(Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                gap: taffy::Size {
                    width: len(t.space.s2),
                    height: len(0.0),
                },
                padding: taffy::Rect {
                    left: len(0.0),
                    right: len(0.0),
                    top: len(pad_block.max(0.0)),
                    bottom: len(pad_block.max(0.0)),
                },
                ..Style::default()
            }));
            match self.list_style {
                ListStyle::None => {}
                style => {
                    let marker = match style {
                        ListStyle::Decimal => format!("{}.", self.start + i as u32),
                        ListStyle::Circle => "\u{25e6}".into(),
                        _ => "\u{2022}".into(),
                    };
                    let mut mk = text_node(
                        marker,
                        type_style(t.font.body, FontFamily::Body),
                        color(c.text_secondary),
                    );
                    mk.sem = None;
                    mk.style.flex_shrink = 0.0;
                    cx.leaf(mk);
                }
            }
            cx.begin(Node::new(Style {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_size: taffy::Size {
                    width: len_auto(0.0),
                    height: LengthPercentageAuto::auto(),
                },
                ..Style::default()
            }));
            cx.child(child);
            cx.end();
            cx.end();
            if self.has_dividers && i + 1 < n {
                cx.leaf(
                    Node::new(Style {
                        size: taffy::Size {
                            width: Dimension::percent(1.0),
                            height: Dimension::length(t.size.border_width),
                        },
                        ..Style::default()
                    })
                    .draw(Draw::Box(BoxDraw::fill(color(c.border), radii(0.0)))),
                );
            }
        }
        cx.end();
        cx.end();
    }
}

/// Collapsible `chevronPosition`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ChevronPosition {
    Start,
    #[default]
    End,
}

/// A trigger that shows or hides its content (`Collapsible`). Controlled:
/// `is_open` and `on_open_change`.
pub struct Collapsible<Msg> {
    base: Base,
    trigger: String,
    is_open: Option<bool>,
    default_is_open: bool,
    on_open_change: Option<Rc<dyn Fn(bool) -> Msg>>,
    is_disabled: bool,
    chevron_position: ChevronPosition,
    children: Vec<Element<Msg>>,
}

impl<Msg> Collapsible<Msg> {
    /// Starts open and opens and closes itself; use [`Self::is_open`] to
    /// control it.
    pub fn new(trigger: impl Into<String>) -> Self {
        Collapsible {
            base: Base::default(),
            trigger: trigger.into(),
            is_open: None,
            default_is_open: true,
            on_open_change: None,
            is_disabled: false,
            chevron_position: ChevronPosition::End,
            children: Vec::new(),
        }
    }
    pub fn is_open(mut self, open: bool) -> Self {
        self.is_open = Some(open);
        self
    }
    pub fn default_is_open(mut self, open: bool) -> Self {
        self.default_is_open = open;
        self
    }
    pub fn on_open_change(mut self, f: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_open_change = Some(Rc::new(f));
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    pub fn chevron_position(mut self, p: ChevronPosition) -> Self {
        self.chevron_position = p;
        self
    }
    pub fn child(mut self, c: impl Into<Element<Msg>>) -> Self {
        self.children.push(c.into());
        self
    }
}

component!(Collapsible<Msg>);

/// What a CollapsibleGroup tells its members.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CollapsibleScope {
    pub chevron: ChevronPosition,
    pub density: Option<ListDensity>,
}

impl<Msg: Clone + 'static> Build<Msg> for Collapsible<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let scope = cx.context::<CollapsibleScope>().map(|s| *s);
        let chevron = scope.map_or(self.chevron_position, |s| s.chevron);
        let pad = scope
            .and_then(|s| s.density)
            .map_or(0.0, |d| super::super::controls::density_pad(t, d));
        let owner = self
            .base
            .id
            .clone()
            .unwrap_or_else(|| format!("collapsible:{}", self.trigger));
        let (open, h) = crate::overlays::open_state(cx, &owner, self.is_open, self.default_is_open);
        let uncontrolled = self.is_open.is_none();
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            ..Style::default()
        }));
        let large = t.font.large;
        let mut sem = Semantics::new(Role::Button, self.trigger.clone())
            .with_state(|s| s.expanded = Some(open));
        sem.disabled = self.is_disabled;
        let r = radii(t.radius.inner);
        cx.begin(
            Node::new(Style {
                flex_direction: if chevron == ChevronPosition::Start {
                    FlexDirection::RowReverse
                } else {
                    FlexDirection::Row
                },
                align_items: AlignItems::CENTER,
                justify_content: JustifyContent::SPACE_BETWEEN,
                gap: taffy::Size {
                    width: len(t.space.s2),
                    height: len(0.0),
                },
                padding: taffy::Rect {
                    left: len(0.0),
                    right: len(0.0),
                    top: len(pad),
                    bottom: len(pad),
                },
                ..Style::default()
            })
            .interact({
                let mut i = Interact::new(
                    self.on_open_change.as_ref().map(|f| f(!open)),
                    self.is_disabled,
                );
                if uncontrolled {
                    let h = h.clone();
                    i = i.effect(move || h.set(Some(!open)));
                }
                i
            })
            .indicator(Indicator::ring(color(c.accent), r, None))
            .sem(sem)
            .opacity(if self.is_disabled { 0.5 } else { 1.0 }),
        );
        let mut style = type_style(large, FontFamily::Body);
        style.weight = t.font.weight.semibold;
        let mut label = text_node(self.trigger, style, color(c.text_primary));
        label.style.flex_grow = 1.0;
        label.sem = None;
        cx.leaf(label);
        let glyph = if open { "chevronUp" } else { "chevronDown" };
        if let Some(n) = icon_node::<Msg>(glyph, large.size, color(c.text_primary)) {
            cx.leaf(n);
        }
        cx.end();
        // The content stays in the tree so its height can animate; closed,
        // it is zero tall, clipped, and inert (out of focus and the
        // accessibility tree).
        let mut region = Node::new(Style {
            flex_direction: FlexDirection::Column,
            size: taffy::Size {
                width: Dimension::auto(),
                height: if open {
                    Dimension::auto()
                } else {
                    Dimension::length(0.0)
                },
            },
            overflow: taffy::Point {
                x: taffy::Overflow::Clip,
                y: taffy::Overflow::Clip,
            },
            ..Style::default()
        });
        region.clip = Some(astryx_core::contract::Radii::default());
        region.resize = Some(astryx_spike_motion::collapse(astryx_spike_motion::tokens(
            cx,
        )));
        region.inert = !open;
        cx.key(Some("content"));
        cx.begin(region);
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            padding: taffy::Rect {
                left: len(0.0),
                right: len(0.0),
                top: len(t.space.s2),
                bottom: len(pad),
            },
            flex_shrink: 0.0,
            ..Style::default()
        }));
        for ch in self.children {
            cx.child(ch);
        }
        cx.end();
        cx.end();
        cx.end();
    }
}

/// CollapsibleGroup `type`: one open at a time, or any number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CollapsibleGroupType {
    #[default]
    Single,
    Multiple,
}

/// Collapsibles that share a chevron side, density and dividers, and in
/// `Single` mode keep one open (`CollapsibleGroup`). The app owns which
/// are open and passes each member its `is_open`.
pub struct CollapsibleGroup<Msg> {
    base: Base,
    group_type: CollapsibleGroupType,
    has_dividers: bool,
    density: Option<ListDensity>,
    chevron_position: ChevronPosition,
    children: Vec<Element<Msg>>,
}

impl<Msg> Default for CollapsibleGroup<Msg> {
    fn default() -> Self {
        CollapsibleGroup {
            base: Base::default(),
            group_type: CollapsibleGroupType::Single,
            has_dividers: false,
            density: None,
            chevron_position: ChevronPosition::End,
            children: Vec::new(),
        }
    }
}

impl<Msg> CollapsibleGroup<Msg> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn group_type(mut self, t: CollapsibleGroupType) -> Self {
        self.group_type = t;
        self
    }
    pub fn has_dividers(mut self, on: bool) -> Self {
        self.has_dividers = on;
        self
    }
    pub fn density(mut self, d: ListDensity) -> Self {
        self.density = Some(d);
        self
    }
    pub fn chevron_position(mut self, p: ChevronPosition) -> Self {
        self.chevron_position = p;
        self
    }
    pub fn child(mut self, c: impl Into<Element<Msg>>) -> Self {
        self.children.push(c.into());
        self
    }
}

component!(CollapsibleGroup<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for CollapsibleGroup<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let _ = self.group_type;
        cx.key(self.base.id.as_deref());
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            gap: taffy::Size {
                width: len(0.0),
                height: len(if self.has_dividers { 0.0 } else { t.space.s2 }),
            },
            ..Style::default()
        }));
        let scope = Rc::new(CollapsibleScope {
            chevron: self.chevron_position,
            density: self
                .density
                .or(self.has_dividers.then_some(ListDensity::Balanced)),
        });
        let n = self.children.len();
        for (i, ch) in self.children.into_iter().enumerate() {
            cx.child(ch.provide(scope.clone()));
            if self.has_dividers && i + 1 < n {
                cx.leaf(
                    Node::new(Style {
                        size: taffy::Size {
                            width: Dimension::percent(1.0),
                            height: Dimension::length(t.size.border_width),
                        },
                        ..Style::default()
                    })
                    .draw(Draw::Box(BoxDraw::fill(color(t.color.border), radii(0.0)))),
                );
            }
        }
        cx.end();
    }
}

/// Toolbar `orientation`.
pub use crate::controls::Orientation as ToolbarOrientation;

/// A row of controls with start, center and end regions (`Toolbar`). Its
/// size reaches the controls through the size cascade.
pub struct Toolbar<Msg> {
    base: Base,
    label: String,
    start: Vec<Element<Msg>>,
    center: Vec<Element<Msg>>,
    end: Vec<Element<Msg>>,
    size: Option<ComponentSize>,
    gap: Space,
    orientation: ToolbarOrientation,
    variant: SectionVariant,
    dividers: Vec<SectionDivider>,
}

impl<Msg> Toolbar<Msg> {
    pub fn new(label: impl Into<String>) -> Self {
        Toolbar {
            base: Base::default(),
            label: label.into(),
            start: Vec::new(),
            center: Vec::new(),
            end: Vec::new(),
            size: None,
            gap: Space::S1,
            orientation: ToolbarOrientation::Horizontal,
            variant: SectionVariant::Transparent,
            dividers: Vec::new(),
        }
    }
    pub fn start_content(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.start.push(e.into());
        self
    }
    pub fn center_content(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.center.push(e.into());
        self
    }
    pub fn end_content(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.end.push(e.into());
        self
    }
    pub fn size(mut self, s: ComponentSize) -> Self {
        self.size = Some(s);
        self
    }
    pub fn gap(mut self, g: Space) -> Self {
        self.gap = g;
        self
    }
    pub fn orientation(mut self, o: ToolbarOrientation) -> Self {
        self.orientation = o;
        self
    }
    pub fn variant(mut self, v: SectionVariant) -> Self {
        self.variant = v;
        self
    }
    pub fn dividers(mut self, d: impl IntoIterator<Item = SectionDivider>) -> Self {
        self.dividers = d.into_iter().collect();
        self
    }
}

component!(Toolbar<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for Toolbar<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let size = cx.size(self.size, ComponentSize::Md);
        let gap = self.gap.points(t);
        let row = self.orientation == ToolbarOrientation::Horizontal;
        let ground = match self.variant {
            SectionVariant::Section => color(c.background_surface),
            SectionVariant::Muted => color(c.background_muted),
            SectionVariant::Transparent => astryx_core::contract::Color::TRANSPARENT,
        };
        let bw = t.size.border_width;
        let has = |d| self.dividers.contains(&d);
        let (l, r) = cx.inline(has(SectionDivider::Start), has(SectionDivider::End));
        let edge = |on: bool| len(if on { bw } else { 0.0 });
        cx.key(self.base.id.as_deref());
        cx.begin(
            Node::new(Style {
                flex_direction: if row {
                    FlexDirection::Row
                } else {
                    FlexDirection::Column
                },
                align_items: AlignItems::CENTER,
                justify_content: JustifyContent::SPACE_BETWEEN,
                gap: taffy::Size {
                    width: len(gap),
                    height: len(gap),
                },
                padding: crate::util::rect(len(t.space.s2)),
                border: taffy::Rect {
                    left: edge(l),
                    right: edge(r),
                    top: edge(has(SectionDivider::Top)),
                    bottom: edge(has(SectionDivider::Bottom)),
                },
                ..Style::default()
            })
            .draw(Draw::Box(BoxDraw {
                border: (!self.dividers.is_empty()).then(|| (color(c.border), bw)),
                ..BoxDraw::fill(ground, radii(0.0))
            }))
            .sem(Semantics::new(Role::Toolbar, self.label).with_state(|s| {
                s.orientation = Some(if row {
                    astryx_core::contract::A11yOrientation::Horizontal
                } else {
                    astryx_core::contract::A11yOrientation::Vertical
                });
            })),
        );
        for region in [self.start, self.center, self.end] {
            cx.begin(Node::new(Style {
                flex_direction: if row {
                    FlexDirection::Row
                } else {
                    FlexDirection::Column
                },
                align_items: AlignItems::CENTER,
                gap: taffy::Size {
                    width: len(gap),
                    height: len(gap),
                },
                ..Style::default()
            }));
            for e in region {
                cx.child(astryx_spike_foundations::provide_size(e, Some(size)));
            }
            cx.end();
        }
        cx.end();
    }
}

/// Pagination `variant`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PaginationVariant {
    #[default]
    Pages,
    Count,
    Compact,
    Dots,
    Input,
    None,
}

/// Page navigation (`Pagination`): previous and next buttons with page
/// numbers, a count, or dots.
pub struct Pagination<Msg> {
    base: Base,
    page: u32,
    on_change: Rc<dyn Fn(u32) -> Msg>,
    total_pages: Option<u32>,
    total_items: Option<u32>,
    page_size: u32,
    variant: PaginationVariant,
    sibling_count: u32,
    has_first_last: Option<bool>,
    size: ComponentSize,
    is_disabled: bool,
    label: Option<String>,
}

impl<Msg> Pagination<Msg> {
    pub fn new(page: u32, on_change: impl Fn(u32) -> Msg + 'static) -> Self {
        Pagination {
            base: Base::default(),
            page,
            on_change: Rc::new(on_change),
            total_pages: None,
            total_items: None,
            page_size: 10,
            variant: PaginationVariant::Pages,
            sibling_count: 1,
            has_first_last: None,
            size: ComponentSize::Md,
            is_disabled: false,
            label: None,
        }
    }
    pub fn total_pages(mut self, n: u32) -> Self {
        self.total_pages = Some(n);
        self
    }
    pub fn total_items(mut self, n: u32) -> Self {
        self.total_items = Some(n);
        self
    }
    pub fn page_size(mut self, n: u32) -> Self {
        self.page_size = n.max(1);
        self
    }
    pub fn variant(mut self, v: PaginationVariant) -> Self {
        self.variant = v;
        self
    }
    pub fn sibling_count(mut self, n: u32) -> Self {
        self.sibling_count = n;
        self
    }
    pub fn has_first_last(mut self, on: bool) -> Self {
        self.has_first_last = Some(on);
        self
    }
    pub fn size(mut self, s: ComponentSize) -> Self {
        self.size = s;
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }
}

component!(Pagination<Msg>);

/// The page numbers to show, with `None` for an ellipsis.
pub fn page_items(page: u32, total: u32, siblings: u32) -> Vec<Option<u32>> {
    let total = total.max(1);
    let page = page.clamp(1, total);
    // First, last, current, its siblings, and two ellipses at most.
    let slots = 2 * siblings + 5;
    if total <= slots {
        return (1..=total).map(Some).collect();
    }
    let left = page.saturating_sub(siblings).max(1);
    let right = (page + siblings).min(total);
    let show_left_gap = left > 3;
    let show_right_gap = right + 2 < total;
    let mut out = Vec::new();
    match (show_left_gap, show_right_gap) {
        (false, true) => {
            let end = 3 + 2 * siblings;
            out.extend((1..=end).map(Some));
            out.push(None);
            out.push(Some(total));
        }
        (true, false) => {
            out.push(Some(1));
            out.push(None);
            let start = total - (2 + 2 * siblings);
            out.extend((start..=total).map(Some));
        }
        _ => {
            out.push(Some(1));
            out.push(None);
            out.extend((left..=right).map(Some));
            out.push(None);
            out.push(Some(total));
        }
    }
    out
}

impl<Msg: Clone + 'static> Build<Msg> for Pagination<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let total = self
            .total_pages
            .or(self.total_items.map(|n| n.div_ceil(self.page_size)))
            .unwrap_or(1)
            .max(1);
        let page = self.page.clamp(1, total);
        let label = self
            .label
            .clone()
            .unwrap_or_else(|| cx.t("@astryx.pagination.label", &args! {}));
        cx.key(self.base.id.as_deref());
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                gap: taffy::Size {
                    width: len(t.space.s1),
                    height: len(0.0),
                },
                align_self: Some(AlignItems::FLEX_START),
                ..Style::default()
            })
            .sem(Semantics::new(Role::Navigation, label)),
        );
        let on = self.on_change.clone();
        let nav = |cx: &mut BuildCx<Msg>, icon: &str, words: String, to: u32, disabled: bool| {
            cx.child(
                Button::new(words)
                    .variant(ButtonVariant::Ghost)
                    .size(self.size)
                    .icon(Icon::new(icon))
                    .is_icon_only(true)
                    .is_disabled(disabled || self.is_disabled)
                    .on_click(on(to))
                    .into(),
            );
        };
        let first_last = self
            .has_first_last
            .unwrap_or(matches!(self.variant, PaginationVariant::Input));
        if first_last {
            let w = cx.t("@astryx.pagination.first", &args! {});
            nav(cx, "chevronsLeft", w, 1, page == 1);
        }
        let prev = cx.t("@astryx.pagination.previous", &args! {});
        nav(
            cx,
            "chevronLeft",
            prev,
            page.saturating_sub(1).max(1),
            page == 1,
        );
        match self.variant {
            PaginationVariant::Pages => {
                for item in page_items(page, total, self.sibling_count) {
                    match item {
                        Some(n) => {
                            let current = n == page;
                            let words = cx.t("@astryx.pagination.goToPage", &args! {"page" => n});
                            let mut b = Button::new(n.to_string())
                                .variant(if current {
                                    ButtonVariant::Secondary
                                } else {
                                    ButtonVariant::Ghost
                                })
                                .size(self.size)
                                .is_disabled(self.is_disabled)
                                .on_click(on(n));
                            if words != n.to_string() {
                                b = b.tooltip(words);
                            }
                            cx.child(b.into());
                        }
                        None => {
                            let mut e = text_node(
                                "\u{2026}",
                                type_style(t.font.label, FontFamily::Body),
                                color(c.text_secondary),
                            );
                            e.style.padding = taffy::Rect {
                                left: len(t.space.s1),
                                right: len(t.space.s1),
                                top: len(0.0),
                                bottom: len(0.0),
                            };
                            e.sem = None;
                            cx.leaf(e);
                        }
                    }
                }
            }
            PaginationVariant::Count | PaginationVariant::Compact | PaginationVariant::Input => {
                let words = format!("{page} / {total}");
                let mut n = text_node(
                    words,
                    type_style(t.font.label, FontFamily::Body),
                    color(c.text_primary),
                );
                text_draw(&mut n).max_lines = Some(1);
                n.style.padding = taffy::Rect {
                    left: len(t.space.s2),
                    right: len(t.space.s2),
                    top: len(0.0),
                    bottom: len(0.0),
                };
                cx.leaf(n);
            }
            PaginationVariant::Dots => {
                for n in 1..=total.min(12) {
                    let on_page = n == page;
                    let side = if on_page { 8.0 } else { 6.0 };
                    cx.leaf(
                        Node::new(Style {
                            size: taffy::Size {
                                width: Dimension::length(side),
                                height: Dimension::length(side),
                            },
                            ..Style::default()
                        })
                        .draw(Draw::Box(BoxDraw::fill(
                            if on_page {
                                color(c.text_primary)
                            } else {
                                color(c.border_emphasized)
                            },
                            radii(side / 2.0),
                        ))),
                    );
                }
            }
            PaginationVariant::None => {}
        }
        let next = cx.t("@astryx.pagination.next", &args! {});
        nav(
            cx,
            "chevronRight",
            next,
            (page + 1).min(total),
            page == total,
        );
        if first_last {
            let w = cx.t("@astryx.pagination.last", &args! {});
            nav(cx, "chevronsRight", w, total, page == total);
        }
        cx.end();
    }
}

/// A row that shows as many children as fit and counts the rest
/// (`OverflowList`). Without measurement before layout, it shows up to
/// `max_visible_items` and a "+N" token for the others.
pub struct OverflowList<Msg> {
    base: Base,
    gap: Space,
    max_visible_items: Option<usize>,
    children: Vec<Element<Msg>>,
}

impl<Msg> OverflowList<Msg> {
    pub fn new() -> Self {
        OverflowList {
            base: Base::default(),
            gap: Space::S2,
            max_visible_items: None,
            children: Vec::new(),
        }
    }
    pub fn gap(mut self, g: Space) -> Self {
        self.gap = g;
        self
    }
    pub fn max_visible_items(mut self, n: usize) -> Self {
        self.max_visible_items = Some(n);
        self
    }
    pub fn child(mut self, c: impl Into<Element<Msg>>) -> Self {
        self.children.push(c.into());
        self
    }
}

impl<Msg> Default for OverflowList<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

component!(OverflowList<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for OverflowList<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let gap = self.gap.points(t);
        cx.key(self.base.id.as_deref());
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            gap: taffy::Size {
                width: len(gap),
                height: len(0.0),
            },
            overflow: taffy::Point {
                x: taffy::Overflow::Clip,
                y: taffy::Overflow::Visible,
            },
            ..Style::default()
        }));
        let n = self.children.len();
        let shown = self.max_visible_items.unwrap_or(n).min(n);
        for ch in self.children.into_iter().take(shown) {
            cx.child(ch);
        }
        if shown < n {
            cx.child(crate::controls::Token::new(format!("+{}", n - shown)).into());
        }
        cx.end();
    }
}
