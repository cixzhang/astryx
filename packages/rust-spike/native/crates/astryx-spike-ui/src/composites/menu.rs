//! DropdownMenu and ContextMenu, and the menu surface and rows they share.

use crate::controls::{Button, ButtonVariant};
use crate::primitives::{icon_node, CheckIndicator, Icon, IconSize};
use crate::util::{color, component, len, len_auto, radii};
use astryx_core::authoring::{
    text_draw, text_node, type_style, Base, BoxDraw, Build, BuildCx, Draw, Indicator, Interact,
    Node, Semantics,
};
use astryx_core::contract::{FontFamily, Role};
use astryx_core::{Element, Point};
use astryx_spike_foundations::ComponentSize;
use astryx_spike_layers::placement::{Align, Placement, Side};
use astryx_spike_layers::Layer;
use std::rc::Rc;
use taffy::{AlignItems, Dimension, FlexDirection, LengthPercentageAuto, Style};

/// DropdownMenuItem `variant`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MenuItemVariant {
    #[default]
    Default,
    Destructive,
}

/// One entry of a menu (`DropdownMenuOption` / `ContextMenuOption`).
pub enum MenuOption<Msg> {
    Item {
        label: String,
        icon: Option<Icon>,
        description: Option<String>,
        on_click: Option<Msg>,
        is_disabled: bool,
        variant: MenuItemVariant,
        end_content: Option<Element<Msg>>,
    },
    Checkbox {
        label: String,
        is_checked: bool,
        on_change: Option<Rc<dyn Fn(bool) -> Msg>>,
        is_disabled: bool,
    },
    /// One choice of a radio group, by its group's value.
    Radio {
        label: String,
        is_checked: bool,
        on_select: Option<Msg>,
        is_disabled: bool,
    },
    Group {
        label: Option<String>,
        options: Vec<MenuOption<Msg>>,
    },
    Divider,
}

impl<Msg> MenuOption<Msg> {
    pub fn item(label: impl Into<String>) -> Self {
        MenuOption::Item {
            label: label.into(),
            icon: None,
            description: None,
            on_click: None,
            is_disabled: false,
            variant: MenuItemVariant::Default,
            end_content: None,
        }
    }
    pub fn checkbox(label: impl Into<String>, is_checked: bool) -> Self {
        MenuOption::Checkbox {
            label: label.into(),
            is_checked,
            on_change: None,
            is_disabled: false,
        }
    }
    pub fn radio(label: impl Into<String>, is_checked: bool) -> Self {
        MenuOption::Radio {
            label: label.into(),
            is_checked,
            on_select: None,
            is_disabled: false,
        }
    }
    pub fn group(label: Option<String>, options: Vec<MenuOption<Msg>>) -> Self {
        MenuOption::Group { label, options }
    }
    pub fn on_click(mut self, msg: Msg) -> Self {
        match &mut self {
            MenuOption::Item { on_click, .. } => *on_click = Some(msg),
            MenuOption::Radio { on_select, .. } => *on_select = Some(msg),
            _ => {}
        }
        self
    }
    pub fn on_change(mut self, f: impl Fn(bool) -> Msg + 'static) -> Self {
        if let MenuOption::Checkbox { on_change, .. } = &mut self {
            *on_change = Some(Rc::new(f));
        }
        self
    }
    pub fn icon(mut self, i: Icon) -> Self {
        if let MenuOption::Item { icon, .. } = &mut self {
            *icon = Some(i);
        }
        self
    }
    pub fn description(mut self, d: impl Into<String>) -> Self {
        if let MenuOption::Item { description, .. } = &mut self {
            *description = Some(d.into());
        }
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        match &mut self {
            MenuOption::Item { is_disabled, .. }
            | MenuOption::Checkbox { is_disabled, .. }
            | MenuOption::Radio { is_disabled, .. } => *is_disabled = on,
            _ => {}
        }
        self
    }
    pub fn variant(mut self, v: MenuItemVariant) -> Self {
        if let MenuOption::Item { variant, .. } = &mut self {
            *variant = v;
        }
        self
    }
    pub fn end_content(mut self, e: impl Into<Element<Msg>>) -> Self {
        if let MenuOption::Item { end_content, .. } = &mut self {
            *end_content = Some(e.into());
        }
        self
    }
}

/// The menu surface with its rows.
pub(crate) struct MenuList<Msg> {
    pub options: Vec<MenuOption<Msg>>,
    pub size: ComponentSize,
    pub width: Option<f32>,
    pub max_height: Option<f32>,
    pub header: Option<Element<Msg>>,
    pub footer: Option<Element<Msg>>,
    /// Sent after a row acts, so the menu closes.
    pub on_close: Option<Msg>,
    /// Runs after a row acts, for a menu that keeps its own open state.
    pub close_effect: Option<Rc<dyn Fn()>>,
}

fn row_pad<Msg>(cx: &BuildCx<Msg>, size: ComponentSize) -> (f32, f32) {
    let s = &cx.tokens.space;
    match size {
        ComponentSize::Sm => (s.s2, s.s1),
        ComponentSize::Md => (s.s2, s.s1_5),
        ComponentSize::Lg => (s.s2, s.s2),
    }
}

impl<Msg: Clone + 'static> MenuList<Msg> {
    #[allow(clippy::too_many_arguments)]
    fn row(
        cx: &mut BuildCx<Msg>,
        size: ComponentSize,
        role: Role,
        label: String,
        description: Option<String>,
        checked: Option<bool>,
        mark: Option<Element<Msg>>,
        msg: Option<Msg>,
        disabled: bool,
        destructive: bool,
        end: Option<Element<Msg>>,
        effect: Option<Rc<dyn Fn()>>,
    ) {
        let t = cx.tokens;
        let c = &t.color;
        let (pi, pb) = row_pad(cx, size);
        let r = radii((t.space.s2 - t.space.s1).max(0.0));
        let mut sem = Semantics::new(role, label.clone());
        sem.checked = checked;
        sem.disabled = disabled;
        sem.description = description.clone();
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                gap: taffy::Size {
                    width: len(t.space.s2),
                    height: len(0.0),
                },
                padding: taffy::Rect {
                    left: len(pi),
                    right: len(pi),
                    top: len(pb),
                    bottom: len(pb),
                },
                size: taffy::Size {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Style::default()
            })
            .interact({
                let mut i = Interact::new(msg, disabled);
                if let Some(e) = effect {
                    i = i.effect(move || e());
                }
                i
            })
            .indicator(Indicator::ring(
                color(c.accent),
                r,
                Some(color(c.overlay_pressed)),
            ))
            .variant(
                astryx_core::layer::When::Focused,
                Draw::Box(BoxDraw::fill(color(c.overlay_hover), r)),
            )
            .sem(sem)
            .opacity(if disabled { 0.5 } else { 1.0 }),
        );
        if let Some(m) = mark {
            cx.child(m);
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
        let ink = if destructive {
            color(c.text_error)
        } else {
            color(c.text_primary)
        };
        let mut l = text_node(label, type_style(t.font.label, FontFamily::Body), ink);
        text_draw(&mut l).max_lines = Some(1);
        l.sem = None;
        cx.leaf(l);
        if let Some(d) = description {
            let mut d = text_node(
                d,
                type_style(t.font.supporting, FontFamily::Body),
                color(c.text_secondary),
            );
            d.sem = None;
            cx.leaf(d);
        }
        cx.end();
        if let Some(e) = end {
            cx.child(e);
        }
        cx.end();
    }

    fn options(
        cx: &mut BuildCx<Msg>,
        size: ComponentSize,
        options: Vec<MenuOption<Msg>>,
        close: &Option<Msg>,
        effect: &Option<Rc<dyn Fn()>>,
    ) {
        let t = cx.tokens;
        let c = &t.color;
        for o in options {
            match o {
                MenuOption::Item {
                    label,
                    icon,
                    description,
                    on_click,
                    is_disabled,
                    variant,
                    end_content,
                } => {
                    let destructive = variant == MenuItemVariant::Destructive;
                    let mark = icon.map(|i| {
                        i.color(if destructive {
                            crate::primitives::IconColor::Error
                        } else {
                            crate::primitives::IconColor::Secondary
                        })
                        .size(IconSize::Sm)
                        .into()
                    });
                    Self::row(
                        cx,
                        size,
                        Role::MenuItem,
                        label,
                        description,
                        None,
                        mark,
                        on_click.or(close.clone()),
                        is_disabled,
                        destructive,
                        end_content,
                        effect.clone(),
                    );
                }
                MenuOption::Checkbox {
                    label,
                    is_checked,
                    on_change,
                    is_disabled,
                } => {
                    let msg = on_change.map(|f| f(!is_checked));
                    Self::row(
                        cx,
                        size,
                        Role::MenuItemCheckbox,
                        label,
                        None,
                        Some(is_checked),
                        Some(CheckIndicator::new(is_checked).into()),
                        msg,
                        is_disabled,
                        false,
                        None,
                        None,
                    );
                }
                MenuOption::Radio {
                    label,
                    is_checked,
                    on_select,
                    is_disabled,
                } => {
                    Self::row(
                        cx,
                        size,
                        Role::MenuItemRadio,
                        label,
                        None,
                        Some(is_checked),
                        Some(CheckIndicator::new(is_checked).into()),
                        on_select,
                        is_disabled,
                        false,
                        None,
                        effect.clone(),
                    );
                }
                MenuOption::Group { label, options } => {
                    let sem = match &label {
                        Some(l) => Semantics::new(Role::Group, l.clone()),
                        None => Semantics::role(Role::Group),
                    };
                    cx.begin(
                        Node::new(Style {
                            flex_direction: FlexDirection::Column,
                            gap: taffy::Size {
                                width: len(0.0),
                                height: len(t.space.s0_5),
                            },
                            ..Style::default()
                        })
                        .sem(sem),
                    );
                    if let Some(l) = label {
                        let mut h = text_node(
                            l,
                            type_style(t.font.supporting, FontFamily::Body),
                            color(c.text_secondary),
                        );
                        h.style.padding = taffy::Rect {
                            left: len(t.space.s2),
                            right: len(t.space.s2),
                            top: len(t.space.s1),
                            bottom: len(t.space.s1),
                        };
                        h.sem = None;
                        cx.leaf(h);
                    }
                    Self::options(cx, size, options, close, effect);
                    cx.end();
                }
                MenuOption::Divider => {
                    cx.leaf(
                        Node::new(Style {
                            size: taffy::Size {
                                width: Dimension::percent(1.0),
                                height: Dimension::length(t.size.border_width),
                            },
                            margin: taffy::Rect {
                                left: len_auto(0.0),
                                right: len_auto(0.0),
                                top: len_auto(t.space.s0_5),
                                bottom: len_auto(t.space.s0_5),
                            },
                            ..Style::default()
                        })
                        .draw(Draw::Box(BoxDraw::fill(color(c.border), radii(0.0))))
                        .sem(Semantics::role(Role::Separator)),
                    );
                }
            }
        }
    }
}

impl<Msg: Clone + 'static> Build<Msg> for MenuList<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let c = &t.color;
        let r = radii(t.radius.container.min(t.space.s2 + t.space.s1 * 2.0));
        let mut style = Style {
            flex_direction: FlexDirection::Column,
            gap: taffy::Size {
                width: len(0.0),
                height: len(t.space.s0_5),
            },
            padding: crate::util::rect(len(t.space.s1)),
            min_size: taffy::Size {
                width: LengthPercentageAuto::length(160.0),
                height: LengthPercentageAuto::auto(),
            },
            ..Style::default()
        };
        if let Some(w) = self.width {
            style.size.width = Dimension::length(w);
        }
        if let Some(h) = self.max_height {
            style.max_size.height = LengthPercentageAuto::length(h);
            style.overflow = taffy::Point {
                x: taffy::Overflow::Visible,
                y: taffy::Overflow::Scroll,
            };
        }
        let mut node = Node::new(style).draw(Draw::Box(BoxDraw {
            shadow: t.shadow.med,
            border: Some((color(c.border), t.size.border_width)),
            ..BoxDraw::fill(color(c.background_popover), r)
        }));
        if self.max_height.is_some() {
            node.scroll = Some(astryx_core::authoring::Axis::Vertical);
        }
        node.clip = Some(r);
        cx.begin(node);
        if let Some(h) = self.header {
            cx.child(h);
        }
        Self::options(
            cx,
            self.size,
            self.options,
            &self.on_close,
            &self.close_effect,
        );
        if let Some(f) = self.footer {
            cx.child(f);
        }
        cx.end();
    }
}

/// A button that opens a menu of actions (`DropdownMenu`). Controlled:
/// `is_menu_open` and `on_open_change`.
pub struct DropdownMenu<Msg> {
    base: Base,
    id: String,
    label: String,
    variant: ButtonVariant,
    icon: Option<Icon>,
    is_icon_only: bool,
    items: Vec<MenuOption<Msg>>,
    is_menu_open: Option<bool>,
    on_open_change: Option<Rc<dyn Fn(bool) -> Msg>>,
    menu_width: Option<f32>,
    menu_max_height: Option<f32>,
    header: Option<Element<Msg>>,
    footer: Option<Element<Msg>>,
    placement: Side,
    alignment: Align,
    has_chevron: bool,
    size: Option<ComponentSize>,
}

impl<Msg> DropdownMenu<Msg> {
    /// `id` names the trigger so the menu can anchor to it.
    pub fn new(id: impl Into<String>, items: Vec<MenuOption<Msg>>) -> Self {
        DropdownMenu {
            base: Base::default(),
            id: id.into(),
            label: "Menu".into(),
            variant: ButtonVariant::Secondary,
            icon: None,
            is_icon_only: false,
            items,
            is_menu_open: None,
            on_open_change: None,
            menu_width: None,
            menu_max_height: None,
            header: None,
            footer: None,
            placement: Side::Below,
            alignment: Align::Start,
            has_chevron: true,
            size: None,
        }
    }
    /// The trigger button's label (`button.label`).
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = l.into();
        self
    }
    pub fn variant(mut self, v: ButtonVariant) -> Self {
        self.variant = v;
        self
    }
    pub fn icon(mut self, i: Icon) -> Self {
        self.icon = Some(i);
        self
    }
    pub fn is_icon_only(mut self, on: bool) -> Self {
        self.is_icon_only = on;
        self
    }
    /// Controls it; without this, the trigger opens and closes it.
    pub fn is_menu_open(mut self, open: bool) -> Self {
        self.is_menu_open = Some(open);
        self
    }
    pub fn on_open_change(mut self, f: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_open_change = Some(Rc::new(f));
        self
    }
    pub fn menu_width(mut self, w: f32) -> Self {
        self.menu_width = Some(w);
        self
    }
    pub fn menu_max_height(mut self, h: f32) -> Self {
        self.menu_max_height = Some(h);
        self
    }
    pub fn header(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.header = Some(e.into());
        self
    }
    pub fn footer(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.footer = Some(e.into());
        self
    }
    pub fn placement(mut self, s: Side) -> Self {
        self.placement = s;
        self
    }
    pub fn alignment(mut self, a: Align) -> Self {
        self.alignment = a;
        self
    }
    pub fn has_chevron(mut self, on: bool) -> Self {
        self.has_chevron = on;
        self
    }
    pub fn size(mut self, s: ComponentSize) -> Self {
        self.size = Some(s);
        self
    }
}

component!(DropdownMenu<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for DropdownMenu<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let owner = format!("{}:menu", self.id);
        let (open, h) = crate::overlays::open_state(cx, &owner, self.is_menu_open, false);
        cx.begin(crate::overlays::owner_node());
        let uncontrolled = self.is_menu_open.is_none();
        let mut b = Button::new(self.label)
            .variant(self.variant)
            .is_icon_only(self.is_icon_only)
            .id(self.id.clone());
        if let Some(s) = self.size {
            b = b.size(s);
        }
        if let Some(i) = self.icon {
            b = b.icon(i);
        }
        if self.has_chevron && !self.is_icon_only {
            b = b.end_content(Icon::new("chevronDown"));
        }
        if let Some(f) = &self.on_open_change {
            b = b.on_click(f(!open));
        }
        if uncontrolled {
            let h = h.clone();
            b = b.effect(Rc::new(move || h.set(Some(!open))));
        }
        cx.child(b.into());
        if open {
            let list = MenuList {
                options: self.items,
                size: self.size.unwrap_or(ComponentSize::Md),
                width: self.menu_width,
                max_height: self.menu_max_height,
                header: self.header,
                footer: self.footer,
                on_close: self.on_open_change.as_ref().map(|f| f(false)),
                close_effect: uncontrolled.then(|| {
                    let h = h.clone();
                    Rc::new(move || h.set(Some(false))) as Rc<dyn Fn()>
                }),
            };
            let mut layer = Layer::menu(&self.id, astryx_core::authoring::element(list))
                .placement(Placement::new(self.placement, self.alignment).offset(t.space.s1))
                .role(Role::Menu)
                .on_dismiss_effect(move |_| h.set(Some(false)));
            if let Some(f) = &self.on_open_change {
                let f = f.clone();
                layer = layer.on_dismiss(move |_| f(false));
            }
            cx.child(layer.into());
        }
        cx.end();
    }
}

/// A menu opened at the pointer by a secondary press, or at its target by
/// Shift+F10 or the Menu key (`ContextMenu`). Controlled: `open_at`.
pub struct ContextMenu<Msg> {
    base: Base,
    child: Element<Msg>,
    items: Vec<MenuOption<Msg>>,
    menu_width: f32,
    size: ComponentSize,
    label: String,
    is_disabled: bool,
    open_at: Option<Point>,
    on_open_change: Option<Rc<dyn Fn(Option<Point>) -> Msg>>,
}

impl<Msg> ContextMenu<Msg> {
    pub fn new(child: impl Into<Element<Msg>>, items: Vec<MenuOption<Msg>>) -> Self {
        ContextMenu {
            base: Base::default(),
            child: child.into(),
            items,
            menu_width: 160.0,
            size: ComponentSize::Md,
            label: "Context menu".into(),
            is_disabled: false,
            open_at: None,
            on_open_change: None,
        }
    }
    pub fn menu_width(mut self, w: f32) -> Self {
        self.menu_width = w;
        self
    }
    pub fn size(mut self, s: ComponentSize) -> Self {
        self.size = s;
        self
    }
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = l.into();
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    /// Where the menu is open, in viewport points, or `None` when closed.
    pub fn open_at(mut self, at: Option<Point>) -> Self {
        self.open_at = at;
        self
    }
    /// Called with the point to open at (pointer position, or the target's
    /// corner for the keyboard), or `None` to close.
    pub fn on_open_change(mut self, f: impl Fn(Option<Point>) -> Msg + 'static) -> Self {
        self.on_open_change = Some(Rc::new(f));
        self
    }
}

component!(ContextMenu<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for ContextMenu<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        cx.key(self.base.id.as_deref());
        let mut node = Node::new(Style {
            flex_direction: FlexDirection::Column,
            ..Style::default()
        });
        // The Menu key opens the menu at the target (keyboard parity for a
        // secondary press). Secondary presses arrive with pointer support.
        if let (Some(f), false) = (&self.on_open_change, self.is_disabled) {
            let f = f.clone();
            let cap: astryx_core::authoring::Capture<Msg> = Rc::new(move |input, phase, _| {
                use astryx_core::contract::{Input, InputPhase};
                if input == Input::Menu && phase == InputPhase::Pressed {
                    (Some(f(Some(Point { x: 0.0, y: 0.0 }))), None)
                } else {
                    (None, None)
                }
            });
            node.capture = Some(cap);
        }
        cx.begin(node);
        cx.child(self.child);
        cx.end();
        if let (Some(at), false) = (self.open_at, self.is_disabled) {
            let list = MenuList {
                options: self.items,
                size: self.size,
                width: Some(self.menu_width),
                max_height: None,
                header: None,
                footer: None,
                on_close: self.on_open_change.as_ref().map(|f| f(None)),
                close_effect: None,
            };
            let mut layer = Layer::menu_at(at, astryx_core::authoring::element(list))
                .role(Role::Menu)
                .label(self.label);
            if let Some(f) = &self.on_open_change {
                let f = f.clone();
                layer = layer.on_dismiss(move |_| f(None));
            }
            cx.child(layer.into());
        }
    }
}

#[allow(dead_code)]
fn _unused<Msg>() -> Option<Node<Msg>> {
    icon_node("check", 16.0, astryx_core::contract::Color::TRANSPARENT)
}
