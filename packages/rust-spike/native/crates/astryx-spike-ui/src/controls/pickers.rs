//! Token, SelectableCard and SegmentedControl.

use super::button::element_height;
use crate::layout::{card_ground, elevation_layers, size_dim, CardVariant, SizeValue};
use crate::primitives::{icon_node, Icon, IconSize};
use crate::util::{color, component, len, len_auto, radii, rect};
use astryx_core::authoring::{
    text_draw, text_node, type_style, Base, BoxDraw, Build, BuildCx, Draw, Indicator, Interact,
    Node, Semantics,
};
use astryx_core::contract::{Color, FontFamily, Role};
use astryx_core::{Element, Elevation, Space};
use astryx_spike_foundations::{args, ComponentSize, CxExt};
use std::rc::Rc;
use taffy::{AlignItems, Dimension, FlexDirection, JustifyContent, LengthPercentageAuto, Style};

/// Token `color`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TokenColor {
    #[default]
    Default,
    Red,
    Orange,
    Yellow,
    Green,
    Teal,
    Cyan,
    Blue,
    Purple,
    Pink,
    Gray,
}

impl TokenColor {
    pub const ALL: [TokenColor; 11] = [
        TokenColor::Default,
        TokenColor::Red,
        TokenColor::Orange,
        TokenColor::Yellow,
        TokenColor::Green,
        TokenColor::Teal,
        TokenColor::Cyan,
        TokenColor::Blue,
        TokenColor::Purple,
        TokenColor::Pink,
        TokenColor::Gray,
    ];
}

fn token_colors<Msg>(cx: &BuildCx<Msg>, tc: TokenColor) -> (Color, Color) {
    let c = &cx.tokens.color;
    let (bg, fg) = match tc {
        TokenColor::Default => (c.neutral, c.text_primary),
        TokenColor::Red => (c.background_red, c.text_red),
        TokenColor::Orange => (c.background_orange, c.text_orange),
        TokenColor::Yellow => (c.background_yellow, c.text_yellow),
        TokenColor::Green => (c.background_green, c.text_green),
        TokenColor::Teal => (c.background_teal, c.text_teal),
        TokenColor::Cyan => (c.background_cyan, c.text_cyan),
        TokenColor::Blue => (c.background_blue, c.text_blue),
        TokenColor::Purple => (c.background_purple, c.text_purple),
        TokenColor::Pink => (c.background_pink, c.text_pink),
        TokenColor::Gray => (c.background_gray, c.text_gray),
    };
    (color(bg), color(fg))
}

/// A compact label for a value, tag or filter (`Token`), optionally
/// clickable and removable.
pub struct Token<Msg> {
    base: Base,
    label: String,
    size: Option<ComponentSize>,
    color: TokenColor,
    icon: Option<Icon>,
    is_disabled: bool,
    on_remove: Option<Msg>,
    on_click: Option<Msg>,
    description: Option<String>,
    end_content: Option<Element<Msg>>,
    is_label_hidden: bool,
}

impl<Msg> Token<Msg> {
    pub fn new(label: impl Into<String>) -> Self {
        Token {
            base: Base::default(),
            label: label.into(),
            size: None,
            color: TokenColor::Default,
            icon: None,
            is_disabled: false,
            on_remove: None,
            on_click: None,
            description: None,
            end_content: None,
            is_label_hidden: false,
        }
    }
    pub fn size(mut self, s: ComponentSize) -> Self {
        self.size = Some(s);
        self
    }
    pub fn color(mut self, c: TokenColor) -> Self {
        self.color = c;
        self
    }
    pub fn icon(mut self, i: Icon) -> Self {
        self.icon = Some(i);
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    /// Shows a remove (×) button that sends `msg`.
    pub fn on_remove(mut self, msg: Msg) -> Self {
        self.on_remove = Some(msg);
        self
    }
    pub fn on_click(mut self, msg: Msg) -> Self {
        self.on_click = Some(msg);
        self
    }
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
    pub fn end_content(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.end_content = Some(e.into());
        self
    }
    pub fn is_label_hidden(mut self, on: bool) -> Self {
        self.is_label_hidden = on;
        self
    }
}

component!(Token<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for Token<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let size = cx.size(self.size, ComponentSize::Md);
        let height = element_height(t, size) - 8.0;
        let (bg, fg) = token_colors(cx, self.color);
        let r = radii(t.radius.inner);
        let mut node = Node::new(Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            gap: taffy::Size {
                width: len(t.space.s1),
                height: len(0.0),
            },
            padding: taffy::Rect {
                left: len(t.space.s2),
                right: len(t.space.s2),
                top: len(0.0),
                bottom: len(0.0),
            },
            size: taffy::Size {
                width: Dimension::auto(),
                height: Dimension::length(height),
            },
            max_size: taffy::Size {
                width: LengthPercentageAuto::percent(1.0),
                height: LengthPercentageAuto::auto(),
            },
            align_self: Some(AlignItems::FLEX_START),
            flex_shrink: 1.0,
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw::fill(bg, r)))
        .opacity(if self.is_disabled { 0.5 } else { 1.0 });
        let mut sem = Semantics::new(
            if self.on_click.is_some() {
                Role::Button
            } else {
                Role::Text
            },
            self.label.clone(),
        );
        sem.description = self.description.clone();
        sem.disabled = self.is_disabled;
        if let Some(msg) = self.on_click {
            node = node
                .interact(Interact::new(Some(msg), self.is_disabled))
                .indicator(Indicator::ring(
                    color(t.color.accent),
                    r,
                    Some(color(t.color.overlay_pressed)),
                ));
        }
        node = node.sem(sem);
        cx.key(self.base.id.as_deref());
        cx.begin(node);
        if let Some(icon) = self.icon {
            let pushed = astryx_core::authoring::Inherited {
                color: fg,
                ..cx.inherited()
            };
            cx.inherit.push(pushed);
            cx.child(icon.default_size(IconSize::Xsm).into());
            cx.inherit.pop();
        }
        if !self.is_label_hidden {
            let mut style = type_style(t.font.supporting, FontFamily::Body);
            style.weight = t.font.weight.medium;
            let mut label = text_node(self.label.clone(), style, fg);
            text_draw(&mut label).max_lines = Some(1);
            label.style.min_size.width = len_auto(0.0);
            label.style.flex_shrink = 1.0;
            label.sem = None;
            cx.leaf(label);
        }
        if let Some(e) = self.end_content {
            cx.child(e);
        }
        if let Some(msg) = self.on_remove {
            let name = cx.t(
                "@astryx.token.remove",
                &args! {"label" => self.label.clone()},
            );
            let pill = radii(t.radius.full);
            cx.begin(
                Node::new(Style {
                    size: crate::util::square(16.0),
                    flex_shrink: 0.0,
                    align_items: AlignItems::CENTER,
                    justify_content: JustifyContent::CENTER,
                    margin: taffy::Rect {
                        left: len_auto(0.0),
                        right: len_auto(-t.space.s1),
                        top: len_auto(0.0),
                        bottom: len_auto(0.0),
                    },
                    ..Style::default()
                })
                .interact(Interact::new(Some(msg), self.is_disabled))
                .indicator(Indicator::ring(
                    color(t.color.accent),
                    pill,
                    Some(color(t.color.overlay_pressed)),
                ))
                .sem(Semantics::new(Role::Button, name)),
            );
            if let Some(n) = icon_node::<Msg>("close", 12.0, fg) {
                cx.leaf(n);
            }
            cx.end();
        }
        cx.end();
    }
}

/// A card that toggles selected, with an accent ring when selected
/// (`SelectableCard`). It is a checkbox for assistive technology.
pub struct SelectableCard<Msg> {
    base: Base,
    label: String,
    is_selected: bool,
    on_change: Option<Rc<dyn Fn(bool) -> Msg>>,
    is_disabled: bool,
    padding: Space,
    variant: CardVariant,
    elevation: Elevation,
    width: Option<SizeValue>,
    height: Option<SizeValue>,
    max_width: Option<SizeValue>,
    children: Vec<Element<Msg>>,
}

impl<Msg> SelectableCard<Msg> {
    pub fn new(label: impl Into<String>, is_selected: bool) -> Self {
        SelectableCard {
            base: Base::default(),
            label: label.into(),
            is_selected,
            on_change: None,
            is_disabled: false,
            padding: Space::S4,
            variant: CardVariant::Default,
            elevation: Elevation::None,
            width: None,
            height: None,
            max_width: None,
            children: Vec::new(),
        }
    }
    pub fn on_change(mut self, f: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    pub fn padding(mut self, p: Space) -> Self {
        self.padding = p;
        self
    }
    pub fn variant(mut self, v: CardVariant) -> Self {
        self.variant = v;
        self
    }
    pub fn elevation(mut self, e: Elevation) -> Self {
        self.elevation = e;
        self
    }
    pub fn width(mut self, w: SizeValue) -> Self {
        self.width = Some(w);
        self
    }
    pub fn height(mut self, h: SizeValue) -> Self {
        self.height = Some(h);
        self
    }
    pub fn max_width(mut self, w: SizeValue) -> Self {
        self.max_width = Some(w);
        self
    }
    pub fn child(mut self, c: impl Into<Element<Msg>>) -> Self {
        self.children.push(c.into());
        self
    }
}

component!(SelectableCard<Msg>);

/// The ring color a selected card of `variant` takes.
fn ring_color<Msg>(cx: &BuildCx<Msg>, v: CardVariant) -> Color {
    let c = &cx.tokens.color;
    color(match v {
        CardVariant::Blue => c.border_blue,
        CardVariant::Cyan => c.border_cyan,
        CardVariant::Gray => c.border_gray,
        CardVariant::Green => c.border_green,
        CardVariant::Orange => c.border_orange,
        CardVariant::Pink => c.border_pink,
        CardVariant::Purple => c.border_purple,
        CardVariant::Red => c.border_red,
        CardVariant::Teal => c.border_teal,
        CardVariant::Yellow => c.border_yellow,
        _ => c.accent,
    })
}

impl<Msg: Clone + 'static> Build<Msg> for SelectableCard<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let (ground, bordered) = card_ground(cx, self.variant);
        let bw = t.size.border_width;
        let ring = ring_color(cx, self.variant);
        // Selected: a 2 point inset ring in the variant's border color; the
        // border takes the same color, so the ring reads as one stroke.
        let (border, stroke) = if self.is_selected {
            (Some((ring, 2.0)), 2.0)
        } else if bordered {
            (Some((color(t.color.border), bw)), bw)
        } else {
            (None, 0.0)
        };
        let inset = (self.padding.points(t) - stroke).max(0.0);
        let r = radii(t.radius.container);
        let mut style = Style {
            flex_direction: FlexDirection::Column,
            padding: rect(len(inset)),
            border: rect(len(stroke)),
            min_size: taffy::Size {
                width: len_auto(0.0),
                height: LengthPercentageAuto::auto(),
            },
            ..Style::default()
        };
        if let Some(w) = self.width {
            style.size.width = size_dim(w);
        }
        if let Some(h) = self.height {
            style.size.height = size_dim(h);
        }
        if let Some(w) = self.max_width {
            style.max_size.width = crate::layout::size_auto(w);
        }
        let mut sem = Semantics::new(Role::Checkbox, self.label.clone());
        sem.checked = Some(self.is_selected);
        sem.disabled = self.is_disabled;
        let mut node = Node::new(style)
            .draw(Draw::Box(BoxDraw {
                fill: ground,
                radii: r,
                border,
                shadow: elevation_layers(self.elevation, t),
            }))
            .interact(Interact::new(
                self.on_change.as_ref().map(|f| f(!self.is_selected)),
                self.is_disabled,
            ))
            .indicator(Indicator::ring(
                color(t.color.accent),
                r,
                Some(color(t.color.overlay_pressed)),
            ))
            .sem(sem)
            .opacity(if self.is_disabled { 0.5 } else { 1.0 });
        node.clip = Some(r);
        cx.key(self.base.id.as_deref());
        cx.begin(node);
        cx.container_padding.push(inset + stroke);
        for c in self.children {
            cx.child(c);
        }
        cx.container_padding.pop();
        cx.end();
    }
}

/// SegmentedControl `layout`: hug the items, or fill the width.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SegmentedLayout {
    #[default]
    Hug,
    Fill,
}

/// One option of a [`SegmentedControl`] (`SegmentedControlItem`).
pub struct SegmentedControlItem {
    pub value: String,
    pub label: String,
    pub is_label_hidden: bool,
    pub icon: Option<Icon>,
    pub is_disabled: bool,
}

impl SegmentedControlItem {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        SegmentedControlItem {
            value: value.into(),
            label: label.into(),
            is_label_hidden: false,
            icon: None,
            is_disabled: false,
        }
    }
    pub fn is_label_hidden(mut self, on: bool) -> Self {
        self.is_label_hidden = on;
        self
    }
    pub fn icon(mut self, i: Icon) -> Self {
        self.icon = Some(i);
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
}

/// A single choice among a few options shown as connected segments
/// (`SegmentedControl`). One tab stop; arrow keys move the selection.
pub struct SegmentedControl<Msg> {
    base: Base,
    label: String,
    value: String,
    items: Vec<SegmentedControlItem>,
    on_change: Option<Rc<dyn Fn(String) -> Msg>>,
    size: Option<ComponentSize>,
    layout: SegmentedLayout,
    is_disabled: bool,
    disabled_message: Option<String>,
}

impl<Msg> SegmentedControl<Msg> {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        SegmentedControl {
            base: Base::default(),
            label: label.into(),
            value: value.into(),
            items: Vec::new(),
            on_change: None,
            size: None,
            layout: SegmentedLayout::Hug,
            is_disabled: false,
            disabled_message: None,
        }
    }
    pub fn item(mut self, item: SegmentedControlItem) -> Self {
        self.items.push(item);
        self
    }
    pub fn on_change(mut self, f: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
    pub fn size(mut self, s: ComponentSize) -> Self {
        self.size = Some(s);
        self
    }
    pub fn layout(mut self, l: SegmentedLayout) -> Self {
        self.layout = l;
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    pub fn disabled_message(mut self, m: impl Into<String>) -> Self {
        self.disabled_message = Some(m.into());
        self
    }
}

component!(SegmentedControl<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for SegmentedControl<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let size = cx.size(self.size, ComponentSize::Md);
        let pad = t.space.s0_5;
        let outer_r = t.radius.element;
        let inner_r = (outer_r - pad).max(0.0);
        let fill = self.layout == SegmentedLayout::Fill;
        cx.key(self.base.id.as_deref());
        let root = cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                gap: taffy::Size {
                    width: len(t.space.s0_5),
                    height: len(0.0),
                },
                padding: rect(len(pad)),
                align_self: if fill {
                    Some(AlignItems::STRETCH)
                } else {
                    Some(AlignItems::FLEX_START)
                },
                size: taffy::Size {
                    width: if fill {
                        Dimension::percent(1.0)
                    } else {
                        Dimension::auto()
                    },
                    height: Dimension::auto(),
                },
                ..Style::default()
            })
            .draw(Draw::Box(BoxDraw::fill(color(c.neutral), radii(outer_r))))
            .sem(Semantics::new(Role::RadioGroup, self.label.clone()))
            .tooltip(if self.is_disabled {
                self.disabled_message.clone()
            } else {
                None
            })
            .opacity(if self.is_disabled { 0.5 } else { 1.0 }),
        );
        let group = cx.id_of(root);
        let height = element_height(t, size) - 4.0;
        let pad_inline = if size == ComponentSize::Sm {
            t.space.s2
        } else {
            t.space.s3
        };
        let any = self.items.iter().any(|i| i.value == self.value);
        for (i, item) in self.items.into_iter().enumerate() {
            let on = item.value == self.value;
            let disabled = self.is_disabled || item.is_disabled;
            let r = radii(inner_r);
            let mut sem = Semantics::new(Role::Radio, item.label.clone());
            sem.checked = Some(on);
            sem.disabled = disabled;
            let mut node = Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                justify_content: JustifyContent::CENTER,
                gap: taffy::Size {
                    width: len(t.space.s1),
                    height: len(0.0),
                },
                padding: taffy::Rect {
                    left: len(pad_inline),
                    right: len(pad_inline),
                    top: len(0.0),
                    bottom: len(0.0),
                },
                size: taffy::Size {
                    width: Dimension::auto(),
                    height: Dimension::length(height),
                },
                flex_grow: if fill { 1.0 } else { 0.0 },
                flex_basis: if fill {
                    Dimension::length(0.0)
                } else {
                    Dimension::auto()
                },
                min_size: taffy::Size {
                    width: len_auto(0.0),
                    height: LengthPercentageAuto::auto(),
                },
                ..Style::default()
            })
            .interact(Interact::new(
                self.on_change.as_ref().map(|f| f(item.value.clone())),
                disabled,
            ))
            .indicator(Indicator::ring(color(c.accent), r, None))
            .sem(sem);
            if on {
                node = node.draw(Draw::Box(BoxDraw {
                    shadow: t.shadow.low,
                    ..BoxDraw::fill(color(c.background_surface), r)
                }));
            }
            node.group = Some(group);
            node.group_entry = on || (!any && i == 0);
            cx.key_slot(i as u64);
            cx.begin(node);
            let ink = if disabled {
                color(c.text_disabled)
            } else if on {
                color(c.text_primary)
            } else {
                color(c.text_secondary)
            };
            if let Some(icon) = item.icon {
                let pushed = astryx_core::authoring::Inherited {
                    color: ink,
                    ..cx.inherited()
                };
                cx.inherit.push(pushed);
                cx.child(
                    icon.default_size(if size == ComponentSize::Sm {
                        IconSize::Xsm
                    } else {
                        IconSize::Sm
                    })
                    .into(),
                );
                cx.inherit.pop();
            }
            if !item.is_label_hidden {
                let mut style = if size == ComponentSize::Sm {
                    type_style(t.font.supporting, FontFamily::Body)
                } else {
                    type_style(t.font.label, FontFamily::Body)
                };
                style.weight = if on {
                    t.font.weight.semibold
                } else {
                    t.font.weight.medium
                };
                let mut l = text_node(item.label, style, ink);
                text_draw(&mut l).max_lines = Some(1);
                l.style.min_size.width = len_auto(0.0);
                l.style.flex_shrink = 1.0;
                l.sem = None;
                cx.leaf(l);
            }
            cx.end();
        }
        cx.end();
    }
}
