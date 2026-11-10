//! Wave 8: the app frame — AppShell, TopNav, SideNav (+ SideNavItem,
//! SideNavSection), MobileNav and TreeList.

mod tree;

pub use tree::{TreeList, TreeListItem, TreeListVariant};

use crate::controls::{Button, ButtonVariant};
use crate::layout::Padding;
use crate::primitives::Icon;
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

/// The width under which AppShell shows its mobile top bar and puts the
/// side navigation in a drawer (Astryx's `md` breakpoint).
pub const MOBILE_BREAKPOINT: f32 = 768.0;

/// AppShell `variant`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AppShellVariant {
    Wash,
    Surface,
    Section,
    #[default]
    Elevated,
}

/// AppShell `height`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AppShellHeight {
    #[default]
    Fill,
    Auto,
}

/// The app frame (`AppShell`): an optional banner and top navigation, a
/// side navigation, and the main content; below the mobile breakpoint the
/// side navigation moves into a drawer opened from a top bar.
pub struct AppShell<Msg> {
    base: Base,
    top_nav: Option<Element<Msg>>,
    side_nav: Option<Rc<dyn Fn() -> Element<Msg>>>,
    banner: Option<Element<Msg>>,
    content_padding: Option<Space>,
    height: AppShellHeight,
    variant: AppShellVariant,
    has_mobile_nav: bool,
    children: Vec<Element<Msg>>,
}

impl<Msg> Default for AppShell<Msg> {
    fn default() -> Self {
        AppShell {
            base: Base::default(),
            top_nav: None,
            side_nav: None,
            banner: None,
            content_padding: None,
            height: AppShellHeight::Fill,
            variant: AppShellVariant::Elevated,
            has_mobile_nav: true,
            children: Vec::new(),
        }
    }
}

impl<Msg> AppShell<Msg> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn top_nav(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.top_nav = Some(e.into());
        self
    }
    /// The side navigation. It is built where it shows: beside the content,
    /// or in the mobile drawer.
    pub fn side_nav(mut self, f: impl Fn() -> Element<Msg> + 'static) -> Self {
        self.side_nav = Some(Rc::new(f));
        self
    }
    pub fn banner(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.banner = Some(e.into());
        self
    }
    pub fn content_padding(mut self, s: Space) -> Self {
        self.content_padding = Some(s);
        self
    }
    pub fn height(mut self, h: AppShellHeight) -> Self {
        self.height = h;
        self
    }
    pub fn variant(mut self, v: AppShellVariant) -> Self {
        self.variant = v;
        self
    }
    /// `mobileNav={false}`: keep the side navigation at every width.
    pub fn has_mobile_nav(mut self, on: bool) -> Self {
        self.has_mobile_nav = on;
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

component!(AppShell<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for AppShell<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let (vw, vh) = cx.viewport;
        let mobile =
            self.has_mobile_nav && self.side_nav.is_some() && vw > 0.0 && vw < MOBILE_BREAKPOINT;
        let shell_ground = match self.variant {
            AppShellVariant::Wash | AppShellVariant::Elevated => color(c.background_body),
            AppShellVariant::Surface | AppShellVariant::Section => color(c.background_surface),
        };
        let id = self.base.id.clone().unwrap_or_else(|| "app-shell".into());
        cx.key(Some(&id));
        let drawer = cx.local_handle::<bool>();
        let fill = self.height == AppShellHeight::Fill;
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Column,
                size: taffy::Size {
                    width: Dimension::percent(1.0),
                    height: if fill && vh > 0.0 {
                        Dimension::length(vh)
                    } else {
                        Dimension::auto()
                    },
                },
                min_size: taffy::Size {
                    width: LengthPercentageAuto::auto(),
                    height: if !fill && vh > 0.0 {
                        LengthPercentageAuto::length(vh)
                    } else {
                        LengthPercentageAuto::auto()
                    },
                },
                overflow: taffy::Point {
                    x: taffy::Overflow::Clip,
                    y: taffy::Overflow::Clip,
                },
                ..Style::default()
            })
            .draw(Draw::Box(BoxDraw::fill(shell_ground, radii(0.0)))),
        );
        if let Some(b) = self.banner {
            cx.child(b);
        }
        if let Some(top) = self.top_nav {
            cx.child(top);
        }
        if mobile {
            // The mobile top bar: a menu button that opens the drawer.
            cx.begin(Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                size: taffy::Size {
                    width: Dimension::auto(),
                    height: Dimension::length(t.space.s12),
                },
                padding: taffy::Rect {
                    left: len(t.space.s2),
                    right: len(t.space.s2),
                    top: len(0.0),
                    bottom: len(0.0),
                },
                flex_shrink: 0.0,
                ..Style::default()
            }));
            let label = cx.t("@astryx.appShell.openNavigation", &args! {});
            let open = drawer.get();
            let h = drawer.clone();
            cx.child(
                Button::new(if label.starts_with('@') {
                    "Open navigation".into()
                } else {
                    label
                })
                .variant(ButtonVariant::Ghost)
                .icon(Icon::new("menu"))
                .is_icon_only(true)
                .effect(Rc::new(move || h.set(!open)))
                .into(),
            );
            cx.end();
        }
        // Navigation and main content.
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Row,
            flex_grow: 1.0,
            min_size: taffy::Size {
                width: len_auto(0.0),
                height: len_auto(0.0),
            },
            ..Style::default()
        }));
        if let (Some(side), false) = (&self.side_nav, mobile) {
            cx.child(side());
        }
        let elevated = self.variant == AppShellVariant::Elevated;
        let page = t.radius.page;
        let corner = if elevated && self.side_nav.is_some() && !mobile {
            let mut r = radii(0.0);
            if cx.rtl() {
                r.0[1] = page;
            } else {
                r.0[0] = page;
            }
            r
        } else {
            radii(0.0)
        };
        let content_ground = match self.variant {
            AppShellVariant::Wash => color(c.background_body),
            _ => color(c.background_surface),
        };
        let pad = self
            .content_padding
            .map_or(0.0, |s| Padding::all(s).resolve(t).0);
        let mut main = Node::new(Style {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_size: taffy::Size {
                width: len_auto(0.0),
                height: len_auto(0.0),
            },
            padding: crate::util::rect(len(pad)),
            overflow: taffy::Point {
                x: taffy::Overflow::Visible,
                y: taffy::Overflow::Scroll,
            },
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw::fill(content_ground, corner)))
        .sem(Semantics::role(Role::Main));
        main.scroll = Some(astryx_core::authoring::Axis::Vertical);
        main.clip = Some(corner);
        cx.key(Some("main"));
        cx.begin(main);
        cx.container_padding.push(pad);
        for ch in self.children {
            cx.child(ch);
        }
        cx.container_padding.pop();
        cx.end();
        cx.end();
        if mobile && drawer.get() {
            if let Some(side) = &self.side_nav {
                let h = drawer.clone();
                cx.child(
                    MobileNav::new(true, move |_| None::<Msg>)
                        .on_close_effect(Rc::new(move || h.set(false)))
                        .child(side())
                        .into(),
                );
            }
        }
        cx.end();
    }
}

/// The bar across the top of an app (`TopNav`): a heading, start,
/// center and end regions.
pub struct TopNav<Msg> {
    base: Base,
    heading: Option<Element<Msg>>,
    start: Vec<Element<Msg>>,
    center: Vec<Element<Msg>>,
    end: Vec<Element<Msg>>,
    label: String,
}

impl<Msg> Default for TopNav<Msg> {
    fn default() -> Self {
        TopNav {
            base: Base::default(),
            heading: None,
            start: Vec::new(),
            center: Vec::new(),
            end: Vec::new(),
            label: "Top navigation".into(),
        }
    }
}

impl<Msg> TopNav<Msg> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn heading(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.heading = Some(e.into());
        self
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
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = l.into();
        self
    }
}

component!(TopNav<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for TopNav<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let gap = |g: f32| taffy::Size {
            width: len(g),
            height: len(g),
        };
        cx.key(self.base.id.as_deref());
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Row,
                flex_wrap: taffy::FlexWrap::Wrap,
                align_items: AlignItems::CENTER,
                padding: crate::util::rect(len(t.space.s2)),
                flex_shrink: 0.0,
                ..Style::default()
            })
            .sem(Semantics::new(Role::Navigation, self.label)),
        );
        // Start: heading and start content.
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Row,
            flex_wrap: taffy::FlexWrap::Wrap,
            align_items: AlignItems::CENTER,
            gap: gap(t.space.s4),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::length(0.0),
            ..Style::default()
        }));
        if let Some(h) = self.heading {
            cx.child(h);
        }
        if !self.start.is_empty() {
            cx.begin(Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                gap: gap(t.space.s1),
                ..Style::default()
            }));
            for e in self.start {
                cx.child(e);
            }
            cx.end();
        }
        cx.end();
        if !self.center.is_empty() {
            cx.begin(Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                justify_content: JustifyContent::CENTER,
                gap: gap(t.space.s1),
                flex_grow: 1000.0,
                flex_shrink: 1.0,
                min_size: taffy::Size {
                    width: len_auto(0.0),
                    height: LengthPercentageAuto::auto(),
                },
                ..Style::default()
            }));
            for e in self.center {
                cx.child(e);
            }
            cx.end();
        }
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Row,
            flex_wrap: taffy::FlexWrap::Wrap,
            align_items: AlignItems::CENTER,
            justify_content: JustifyContent::FLEX_END,
            gap: gap(t.space.s1),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::length(0.0),
            ..Style::default()
        }));
        for e in self.end {
            cx.child(e);
        }
        cx.end();
        cx.end();
    }
}

/// Side navigation (`SideNav`): a sticky header and top content, scrolling
/// items, and a sticky footer; collapsible to an icon rail.
pub struct SideNav<Msg> {
    base: Base,
    header: Option<Element<Msg>>,
    top_content: Option<Element<Msg>>,
    footer: Option<Element<Msg>>,
    collapsible: bool,
    is_collapsed: Option<bool>,
    width: f32,
    resizable: Option<(f32, f32)>,
    label: String,
    children: Vec<Element<Msg>>,
}

impl<Msg> Default for SideNav<Msg> {
    fn default() -> Self {
        SideNav {
            base: Base::default(),
            header: None,
            top_content: None,
            footer: None,
            collapsible: false,
            is_collapsed: None,
            width: 260.0,
            resizable: None,
            label: "Side navigation".into(),
            children: Vec::new(),
        }
    }
}

impl<Msg> SideNav<Msg> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn header(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.header = Some(e.into());
        self
    }
    pub fn top_content(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.top_content = Some(e.into());
        self
    }
    pub fn footer(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.footer = Some(e.into());
        self
    }
    /// Adds a button that collapses the navigation to an icon rail.
    pub fn collapsible(mut self, on: bool) -> Self {
        self.collapsible = on;
        self
    }
    pub fn is_collapsed(mut self, on: bool) -> Self {
        self.is_collapsed = Some(on);
        self
    }
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// A drag handle on the end edge resizes it between `min` and `max`;
    /// `width` is where it starts.
    pub fn resizable(mut self, min: f32, max: f32) -> Self {
        self.resizable = Some((min, max));
        self
    }
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = l.into();
        self
    }
    pub fn child(mut self, c: impl Into<Element<Msg>>) -> Self {
        self.children.push(c.into());
        self
    }
}

component!(SideNav<Msg>);

#[derive(Clone, Debug, Default, PartialEq)]
struct NavWidth {
    width: Option<f32>,
    drag_from: Option<f32>,
}

/// What SideNav tells its items.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SideNavScope {
    pub collapsed: bool,
}

impl<Msg: Clone + 'static> Build<Msg> for SideNav<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let id = self.base.id.clone().unwrap_or_else(|| "side-nav".into());
        cx.key(Some(&id));
        let h = cx.local_handle::<Option<bool>>();
        let collapsed = self.is_collapsed.or(h.get()).unwrap_or(false);
        let scope = Rc::new(SideNavScope { collapsed });
        // The resize state lives on a wrapper around the navigation.
        let resize = self.resizable;
        let wkey = format!("{id}:width");
        let width_state = resize.map(|_| {
            cx.key(Some(&wkey));
            cx.local_handle::<NavWidth>()
        });
        let width = width_state
            .as_ref()
            .and_then(|w| w.get().width)
            .unwrap_or(self.width);
        if width_state.is_some() {
            cx.begin(Node::new(Style {
                flex_direction: FlexDirection::Row,
                flex_shrink: 0.0,
                ..Style::default()
            }));
        }
        let col = |gap: f32| Style {
            flex_direction: FlexDirection::Column,
            gap: taffy::Size {
                width: len(0.0),
                height: len(gap),
            },
            align_items: if collapsed {
                AlignItems::CENTER
            } else {
                AlignItems::STRETCH
            },
            ..Style::default()
        };
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Column,
                size: taffy::Size {
                    width: Dimension::length(if collapsed { t.space.s12 } else { width }),
                    height: Dimension::percent(1.0),
                },
                flex_shrink: 0.0,
                overflow: taffy::Point {
                    x: taffy::Overflow::Clip,
                    y: taffy::Overflow::Clip,
                },
                ..Style::default()
            })
            .sem(Semantics::new(Role::Navigation, self.label)),
        );
        let has_top = self.header.is_some() || self.top_content.is_some() || self.collapsible;
        if has_top {
            cx.begin(Node::new(Style {
                padding: crate::util::rect(len(t.space.s2)),
                flex_shrink: 0.0,
                ..col(t.space.s2)
            }));
            cx.begin(Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                justify_content: JustifyContent::SPACE_BETWEEN,
                ..Style::default()
            }));
            if let (Some(hd), false) = (self.header, collapsed) {
                cx.child(hd);
            }
            if self.collapsible {
                let label = if collapsed {
                    "Expand navigation"
                } else {
                    "Collapse navigation"
                };
                let hh = h.clone();
                cx.child(
                    Button::new(label)
                        .variant(ButtonVariant::Ghost)
                        .size(ComponentSize::Sm)
                        .icon(Icon::new("sidebar"))
                        .is_icon_only(true)
                        .effect(Rc::new(move || hh.set(Some(!collapsed))))
                        .into(),
                );
            }
            cx.end();
            if let (Some(tc), false) = (self.top_content, collapsed) {
                cx.child(tc);
            }
            cx.end();
        }
        let mut body = Node::new(Style {
            flex_grow: 1.0,
            min_size: taffy::Size {
                width: LengthPercentageAuto::auto(),
                height: len_auto(0.0),
            },
            padding: taffy::Rect {
                left: len(t.space.s2),
                right: len(t.space.s2),
                top: len(if has_top { t.space.s1 } else { t.space.s2 }),
                bottom: len(if self.footer.is_some() {
                    t.space.s1
                } else {
                    t.space.s2
                }),
            },
            overflow: taffy::Point {
                x: taffy::Overflow::Clip,
                y: taffy::Overflow::Scroll,
            },
            ..col(t.space.s0_5)
        });
        body.scroll = Some(astryx_core::authoring::Axis::Vertical);
        cx.key(Some("items"));
        cx.begin(body);
        for ch in self.children {
            cx.child(ch.provide(scope.clone()));
        }
        cx.end();
        if let Some(f) = self.footer {
            cx.begin(Node::new(Style {
                padding: crate::util::rect(len(t.space.s2)),
                flex_shrink: 0.0,
                ..col(t.space.s2)
            }));
            cx.child(f.provide(scope.clone()));
            cx.end();
        }
        cx.end();
        if let (Some(wh), Some((min, max))) = (width_state, resize) {
            if !collapsed {
                let hh = wh.clone();
                let grip = astryx_core::authoring::element(EdgeHandle);
                let sign = if cx.rtl() { -1.0 } else { 1.0 };
                cx.child(
                    astryx_spike_gestures::runtime::Drag::local(
                        grip,
                        astryx_spike_gestures::Axis::X,
                        move |e| {
                            let mut s = hh.get();
                            let from = *s.drag_from.get_or_insert(s.width.unwrap_or(width));
                            s.width = Some((from + sign * e.translation.x).clamp(min, max));
                            if matches!(
                                e.phase,
                                astryx_spike_gestures::PanPhase::Ended
                                    | astryx_spike_gestures::PanPhase::Cancelled
                            ) {
                                s.drag_from = None;
                            }
                            hh.set(s);
                        },
                    )
                    .id(format!("{id}:resize"))
                    .into(),
                );
            }
            cx.end();
        }
    }
}

/// The resize handle on a side navigation's edge: a one-point rule with a
/// wider hit area.
struct EdgeHandle;

impl<Msg: 'static> Build<Msg> for EdgeHandle {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        cx.begin(
            Node::new(Style {
                size: taffy::Size {
                    width: Dimension::length(8.0),
                    height: Dimension::percent(1.0),
                },
                margin: taffy::Rect {
                    left: len_auto(-4.0),
                    right: len_auto(-4.0),
                    top: len_auto(0.0),
                    bottom: len_auto(0.0),
                },
                justify_content: JustifyContent::CENTER,
                ..Style::default()
            })
            .sem(Semantics::new(Role::Separator, "Resize navigation")),
        );
        cx.leaf(
            Node::new(Style {
                size: taffy::Size {
                    width: Dimension::length(t.size.border_width),
                    height: Dimension::percent(1.0),
                },
                ..Style::default()
            })
            .draw(Draw::Box(BoxDraw::fill(color(t.color.border), radii(0.0)))),
        );
        cx.end();
    }
}

/// A titled group of side navigation items (`SideNavSection`).
pub struct SideNavSection<Msg> {
    base: Base,
    title: String,
    subtitle: Option<String>,
    end_content: Option<Element<Msg>>,
    is_header_hidden: bool,
    children: Vec<Element<Msg>>,
}

impl<Msg> SideNavSection<Msg> {
    pub fn new(title: impl Into<String>) -> Self {
        SideNavSection {
            base: Base::default(),
            title: title.into(),
            subtitle: None,
            end_content: None,
            is_header_hidden: false,
            children: Vec::new(),
        }
    }
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = Some(s.into());
        self
    }
    pub fn end_content(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.end_content = Some(e.into());
        self
    }
    pub fn is_header_hidden(mut self, on: bool) -> Self {
        self.is_header_hidden = on;
        self
    }
    pub fn child(mut self, c: impl Into<Element<Msg>>) -> Self {
        self.children.push(c.into());
        self
    }
}

component!(SideNavSection<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for SideNavSection<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let collapsed = cx.context::<SideNavScope>().is_some_and(|s| s.collapsed);
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Column,
                padding: taffy::Rect {
                    left: len(0.0),
                    right: len(0.0),
                    top: len(t.space.s1),
                    bottom: len(t.space.s1),
                },
                ..Style::default()
            })
            .sem(Semantics::new(Role::Group, self.title.clone())),
        );
        if !self.is_header_hidden && !collapsed {
            cx.begin(Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                gap: taffy::Size {
                    width: len(t.space.s2),
                    height: len(0.0),
                },
                padding: taffy::Rect {
                    left: len(t.space.s2),
                    right: len(t.space.s2),
                    top: len(t.space.s1),
                    bottom: len(t.space.s1),
                },
                ..Style::default()
            }));
            cx.begin(Node::new(Style {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_size: taffy::Size {
                    width: len_auto(0.0),
                    height: LengthPercentageAuto::auto(),
                },
                ..Style::default()
            }));
            let mut s = type_style(t.font.supporting, FontFamily::Body);
            s.weight = t.font.weight.semibold;
            let mut title = text_node(self.title, s, color(c.text_secondary));
            text_draw(&mut title).max_lines = Some(1);
            title.sem = None;
            cx.leaf(title);
            if let Some(sub) = self.subtitle {
                let mut n = text_node(
                    sub,
                    type_style(t.font.supporting, FontFamily::Body),
                    color(c.text_secondary),
                );
                text_draw(&mut n).max_lines = Some(1);
                cx.leaf(n);
            }
            cx.end();
            if let Some(e) = self.end_content {
                cx.child(e);
            }
            cx.end();
        }
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            gap: taffy::Size {
                width: len(0.0),
                height: len(t.space.s0_5),
            },
            align_items: if collapsed {
                AlignItems::CENTER
            } else {
                AlignItems::STRETCH
            },
            ..Style::default()
        }));
        for ch in self.children {
            cx.child(ch);
        }
        cx.end();
        cx.end();
    }
}

/// One side navigation link (`SideNavItem`): icon, label, end content,
/// selected and disabled states, and nested items that expand.
pub struct SideNavItem<Msg> {
    base: Base,
    label: String,
    icon: Option<Icon>,
    selected_icon: Option<Icon>,
    is_selected: bool,
    is_disabled: bool,
    on_click: Option<Msg>,
    href: Option<String>,
    end_content: Option<Element<Msg>>,
    size: ComponentSize,
    children: Vec<Element<Msg>>,
    default_is_collapsed: bool,
}

impl<Msg> SideNavItem<Msg> {
    pub fn new(label: impl Into<String>) -> Self {
        SideNavItem {
            base: Base::default(),
            label: label.into(),
            icon: None,
            selected_icon: None,
            is_selected: false,
            is_disabled: false,
            on_click: None,
            href: None,
            end_content: None,
            size: ComponentSize::Md,
            children: Vec::new(),
            default_is_collapsed: false,
        }
    }
    pub fn icon(mut self, i: Icon) -> Self {
        self.icon = Some(i);
        self
    }
    pub fn selected_icon(mut self, i: Icon) -> Self {
        self.selected_icon = Some(i);
        self
    }
    pub fn is_selected(mut self, on: bool) -> Self {
        self.is_selected = on;
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    pub fn on_click(mut self, m: Msg) -> Self {
        self.on_click = Some(m);
        self
    }
    pub fn href(mut self, h: impl Into<String>) -> Self {
        self.href = Some(h.into());
        self
    }
    pub fn end_content(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.end_content = Some(e.into());
        self
    }
    pub fn size(mut self, s: ComponentSize) -> Self {
        self.size = s;
        self
    }
    pub fn child(mut self, c: impl Into<Element<Msg>>) -> Self {
        self.children.push(c.into());
        self
    }
    /// Nested items start hidden.
    pub fn default_is_collapsed(mut self, on: bool) -> Self {
        self.default_is_collapsed = on;
        self
    }
}

component!(SideNavItem<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for SideNavItem<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let rail = cx.context::<SideNavScope>().is_some_and(|s| s.collapsed);
        let id = self
            .base
            .id
            .clone()
            .unwrap_or_else(|| format!("nav:{}", self.label));
        cx.key(Some(&id));
        let h = cx.local_handle::<Option<bool>>();
        let has_children = !self.children.is_empty();
        let open = !h.get().unwrap_or(self.default_is_collapsed);
        let (min_h, pad) = match self.size {
            ComponentSize::Sm => (t.size.element_sm, t.space.s1),
            ComponentSize::Md => (t.size.element_md, t.space.s2),
            ComponentSize::Lg => (t.size.element_lg, t.space.s2),
        };
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            ..Style::default()
        }));
        let r = radii(t.radius.element);
        let mut sem = if self.href.is_some() {
            Semantics::new(Role::Link, self.label.clone())
        } else {
            Semantics::new(Role::Button, self.label.clone())
        };
        sem.disabled = self.is_disabled;
        if self.is_selected {
            sem.state.current = Some(astryx_core::contract::Current::Page);
        }
        if has_children {
            sem.state.expanded = Some(open);
        }
        let mut interact =
            Interact::new(self.on_click, self.is_disabled).href(self.href.as_deref());
        if has_children {
            let hh = h.clone();
            interact = interact.effect(move || hh.set(Some(open)));
        }
        let ground = if self.is_selected {
            color(c.neutral)
        } else {
            astryx_core::contract::Color::TRANSPARENT
        };
        let ink = if self.is_disabled {
            color(c.text_disabled)
        } else {
            color(c.text_primary)
        };
        let node = Node::new(Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            justify_content: if rail {
                JustifyContent::CENTER
            } else {
                JustifyContent::FLEX_START
            },
            gap: taffy::Size {
                width: len(t.space.s2),
                height: len(0.0),
            },
            size: if rail {
                taffy::Size {
                    width: Dimension::length(min_h),
                    height: Dimension::length(min_h),
                }
            } else {
                taffy::Size {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                }
            },
            min_size: taffy::Size {
                width: LengthPercentageAuto::auto(),
                height: len_auto(min_h),
            },
            padding: taffy::Rect {
                left: len(if rail { 0.0 } else { pad }),
                right: len(if rail { 0.0 } else { pad }),
                top: len(0.0),
                bottom: len(0.0),
            },
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw::fill(ground, r)))
        .interact(interact)
        .indicator(Indicator::ring(
            color(c.accent),
            r,
            Some(color(c.overlay_pressed)),
        ))
        .sem(sem)
        .tooltip(rail.then(|| self.label.clone()));
        cx.begin(node);
        let icon = if self.is_selected {
            self.selected_icon.or(self.icon)
        } else {
            self.icon
        };
        if let Some(i) = icon {
            cx.child(i.size(crate::primitives::IconSize::Sm).into());
        }
        if !rail {
            let mut s = type_style(t.font.label, FontFamily::Body);
            s.weight = if self.is_selected {
                t.font.weight.medium
            } else {
                t.font.weight.normal
            };
            let mut l = text_node(self.label, s, ink);
            text_draw(&mut l).max_lines = Some(1);
            l.style.flex_grow = 1.0;
            l.style.min_size.width = len_auto(0.0);
            l.sem = None;
            cx.leaf(l);
            if let Some(e) = self.end_content {
                cx.child(e);
            }
            if has_children {
                cx.child(
                    Icon::new(if open { "chevronUp" } else { "chevronDown" })
                        .size(crate::primitives::IconSize::Xsm)
                        .color(crate::primitives::IconColor::Secondary)
                        .into(),
                );
            }
        }
        cx.end();
        if has_children && open && !rail {
            cx.begin(Node::new(Style {
                flex_direction: FlexDirection::Column,
                gap: taffy::Size {
                    width: len(0.0),
                    height: len(t.space.s0_5),
                },
                padding: taffy::Rect {
                    left: len(t.space.s4),
                    right: len(0.0),
                    top: len(t.space.s0_5),
                    bottom: len(0.0),
                },
                ..Style::default()
            }));
            for ch in self.children {
                cx.child(ch);
            }
            cx.end();
        }
        cx.end();
    }
}

/// MobileNav `side`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MobileNavSide {
    #[default]
    Start,
    End,
}

/// A navigation drawer over the page (`MobileNav`): modal, from the start
/// or end edge, closed by Escape, a press outside, or the close button.
pub struct MobileNav<Msg> {
    is_open: bool,
    on_open_change: Rc<dyn Fn(bool) -> Option<Msg>>,
    close_effect: Option<Rc<dyn Fn()>>,
    header: Option<Element<Msg>>,
    width: f32,
    side: MobileNavSide,
    children: Vec<Element<Msg>>,
}

impl<Msg> MobileNav<Msg> {
    /// `on_open_change` may send nothing, for a drawer its owner closes
    /// through its own state.
    pub fn new(is_open: bool, on_open_change: impl Fn(bool) -> Option<Msg> + 'static) -> Self {
        MobileNav {
            is_open,
            on_open_change: Rc::new(on_open_change),
            close_effect: None,
            header: None,
            width: 320.0,
            side: MobileNavSide::Start,
            children: Vec::new(),
        }
    }
    pub(crate) fn on_close_effect(mut self, f: Rc<dyn Fn()>) -> Self {
        self.close_effect = Some(f);
        self
    }
    pub fn header(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.header = Some(e.into());
        self
    }
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    pub fn side(mut self, s: MobileNavSide) -> Self {
        self.side = s;
        self
    }
    pub fn child(mut self, c: impl Into<Element<Msg>>) -> Self {
        self.children.push(c.into());
        self
    }
}

impl<Msg: Clone + 'static> From<MobileNav<Msg>> for Element<Msg> {
    fn from(m: MobileNav<Msg>) -> Self {
        astryx_core::authoring::element(m)
    }
}

struct Drawer<Msg> {
    width: f32,
    end: bool,
    header: Option<Element<Msg>>,
    close: Rc<dyn Fn()>,
    close_msg: Option<Msg>,
    children: Vec<Element<Msg>>,
}

impl<Msg: Clone + 'static> Build<Msg> for Drawer<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let c = &t.color;
        let (vw, vh) = cx.viewport;
        let vh = if vh > 0.0 { vh } else { 800.0 };
        let w = if vw > 0.0 {
            self.width.min(vw)
        } else {
            self.width
        };
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Column,
                size: taffy::Size {
                    width: Dimension::length(w),
                    height: Dimension::length(vh),
                },
                ..Style::default()
            })
            .draw(Draw::Box(BoxDraw {
                border: Some((color(c.border), t.size.border_width)),
                ..BoxDraw::fill(color(c.background_surface), radii(0.0))
            })),
        );
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            justify_content: if self.end {
                JustifyContent::FLEX_START
            } else {
                JustifyContent::SPACE_BETWEEN
            },
            padding: crate::util::rect(len(t.space.s2)),
            ..Style::default()
        }));
        if let Some(h) = self.header {
            cx.child(h);
        }
        let close = self.close.clone();
        let mut b = Button::new("Close navigation")
            .variant(ButtonVariant::Ghost)
            .icon(Icon::new("close"))
            .is_icon_only(true)
            .effect(Rc::new(move || close()));
        if let Some(m) = self.close_msg {
            b = b.on_click(m);
        }
        cx.child(b.into());
        cx.end();
        for ch in self.children {
            cx.child(ch);
        }
        cx.end();
    }
}

impl<Msg: Clone + 'static> Build<Msg> for MobileNav<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if !self.is_open {
            return;
        }
        let t = cx.tokens;
        let end = (self.side == MobileNavSide::End) != cx.rtl();
        let close = self.close_effect.clone().unwrap_or_else(|| Rc::new(|| {}));
        let drawer = astryx_core::authoring::element(Drawer {
            width: self.width,
            end,
            header: self.header,
            close: close.clone(),
            close_msg: (self.on_open_change)(false),
            children: self.children,
        });
        use astryx_core::layer::{DismissReason, LayerAnchor, LayerSpec, PlaceInput};
        let mut spec = LayerSpec::new(
            LayerAnchor::None,
            Rc::new(move |input: &PlaceInput| astryx_core::Point {
                x: if end {
                    input.viewport.x + input.viewport.width - input.size.width
                } else {
                    input.viewport.x
                },
                y: input.viewport.y,
            }),
        );
        spec.modal = true;
        spec.trap_focus = true;
        spec.restore_focus = true;
        spec.escape_dismiss = true;
        spec.light_dismiss = true;
        spec.backdrop = Some(color(t.color.overlay));
        let close2 = close.clone();
        spec.dismiss_effect = Some(Rc::new(move |_: DismissReason| close2()));
        if (self.on_open_change)(false).is_some() {
            let on = self.on_open_change.clone();
            spec.on_dismiss = Some(Rc::new(move |_: DismissReason| {
                on(false).expect("close message")
            }));
        }
        cx.layer(spec, move |cx| {
            cx.begin(Node::new(Style::default()).sem(Semantics::new(Role::Dialog, "Navigation")));
            cx.child(drawer);
            cx.end();
        });
    }
}

#[allow(dead_code)]
fn _unused<Msg>(cx: &BuildCx<Msg>) -> Element<Msg> {
    let _ = cx;
    unreachable!()
}
