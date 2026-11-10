//! CheckboxInput, CheckboxList and RadioList.

use super::field::{field_methods, label_column, status_box, FieldParts};
use crate::layout::{size_dim, SizeValue};
use crate::primitives::{CheckboxIndicator, CheckboxState, IndicatorSize, RadioIndicator};
use crate::util::{color, component, len, len_auto, radii};
use astryx_core::authoring::{
    text_node, type_style, Base, BoxDraw, Build, BuildCx, Draw, Indicator, Interact, Node,
    Semantics,
};
use astryx_core::contract::{FontFamily, Role};
use astryx_core::Element;
use std::rc::Rc;
use taffy::{AlignItems, FlexDirection, Style};

/// The size of a choice control's mark.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ChoiceSize {
    Sm,
    #[default]
    Md,
}

impl From<ChoiceSize> for IndicatorSize {
    fn from(s: ChoiceSize) -> Self {
        match s {
            ChoiceSize::Sm => IndicatorSize::Sm,
            ChoiceSize::Md => IndicatorSize::Md,
        }
    }
}

/// How a row's mark aligns with multi-line text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ChoiceAlign {
    #[default]
    Center,
    Start,
    FirstLine,
}

/// List `density`: the block padding of each row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ListDensity {
    Compact,
    #[default]
    Balanced,
    Spacious,
}

/// One labeled choice row: a mark, a label and a description.
#[allow(clippy::too_many_arguments)]
fn choice_row<Msg>(
    cx: &mut BuildCx<Msg>,
    mark: Element<Msg>,
    label: &str,
    description: Option<&str>,
    align: ChoiceAlign,
    disabled: bool,
    pad_block: f32,
    end: Option<Element<Msg>>,
) {
    let t = cx.tokens;
    let c = &t.color;
    cx.begin(Node::new(Style {
        flex_direction: FlexDirection::Row,
        align_items: match align {
            ChoiceAlign::Center => AlignItems::CENTER,
            _ => AlignItems::FLEX_START,
        },
        gap: taffy::Size {
            width: len(t.space.s2),
            height: len(0.0),
        },
        padding: taffy::Rect {
            left: len(0.0),
            right: len(0.0),
            top: len(pad_block),
            bottom: len(pad_block),
        },
        ..Style::default()
    }));
    cx.child(mark);
    cx.begin(Node::new(Style {
        flex_direction: FlexDirection::Column,
        flex_grow: 1.0,
        flex_shrink: 1.0,
        min_size: taffy::Size {
            width: len_auto(0.0),
            height: taffy::LengthPercentageAuto::auto(),
        },
        ..Style::default()
    }));
    let style = type_style(t.font.body, FontFamily::Body);
    let ink = if disabled {
        color(c.text_disabled)
    } else {
        color(c.text_primary)
    };
    let mut l = text_node(label.to_string(), style, ink);
    l.sem = None;
    cx.leaf(l);
    if let Some(d) = description {
        let mut d = text_node(
            d.to_string(),
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

/// A checkbox with a label (`CheckboxInput`).
pub struct CheckboxInput<Msg> {
    base: Base,
    field: FieldParts,
    value: CheckboxState,
    on_change: Option<Rc<dyn Fn(bool) -> Msg>>,
    is_loading: bool,
    is_read_only: bool,
    size: ChoiceSize,
    width: Option<SizeValue>,
}

impl<Msg> CheckboxInput<Msg> {
    pub fn new(label: impl Into<String>, value: CheckboxState) -> Self {
        CheckboxInput {
            base: Base::default(),
            field: FieldParts::new(label),
            value,
            on_change: None,
            is_loading: false,
            is_read_only: false,
            size: ChoiceSize::Md,
            width: None,
        }
    }
    pub fn on_change(mut self, f: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
    pub fn is_loading(mut self, on: bool) -> Self {
        self.is_loading = on;
        self
    }
    pub fn is_read_only(mut self, on: bool) -> Self {
        self.is_read_only = on;
        self
    }
    pub fn size(mut self, s: ChoiceSize) -> Self {
        self.size = s;
        self
    }
    pub fn width(mut self, w: SizeValue) -> Self {
        self.width = Some(w);
        self
    }
    field_methods!();
}

component!(CheckboxInput<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for CheckboxInput<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let f = &self.field;
        let checked = self.value == CheckboxState::Checked;
        let mut sem = Semantics::new(Role::Checkbox, f.name(cx));
        sem.checked = Some(checked);
        sem.disabled = f.is_disabled;
        sem.description = f.described();
        let sem = sem.with_state(|s| {
            s.read_only = self.is_read_only;
            s.busy = self.is_loading;
            s.required = f.is_required;
            s.invalid = f
                .status
                .as_ref()
                .is_some_and(|s| s.status_type == super::StatusType::Error);
        });
        let mut outer = Style {
            flex_direction: FlexDirection::Column,
            ..Style::default()
        };
        if let Some(w) = self.width {
            outer.size.width = size_dim(w);
        }
        cx.key(self.base.id.as_deref());
        cx.begin(Node::new(outer));
        let inert = f.is_disabled || self.is_read_only;
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FLEX_START,
                gap: taffy::Size {
                    width: len(t.space.s2),
                    height: len(0.0),
                },
                ..Style::default()
            })
            .interact(
                Interact::new(self.on_change.as_ref().map(|on| on(!checked)), inert)
                    .busy(self.is_loading),
            )
            .sem(sem),
        );
        let r = radii(t.radius.inner);
        cx.begin(Node::new(Style::default()).indicator(Indicator::ring(
            color(t.color.accent),
            r,
            None,
        )));
        cx.child(
            CheckboxIndicator::new(self.value)
                .size(self.size.into())
                .is_disabled(f.is_disabled)
                .into(),
        );
        cx.end();
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            justify_content: taffy::JustifyContent::CENTER,
            min_size: taffy::Size {
                width: len_auto(0.0),
                height: len_auto(if self.size == ChoiceSize::Sm {
                    20.0
                } else {
                    24.0
                }),
            },
            ..Style::default()
        }));
        label_column(cx, f);
        cx.end();
        cx.end();
        status_box(cx, f);
        cx.end();
    }
}

/// One option in a [`CheckboxList`] (`CheckboxListItem`).
pub struct CheckboxListItem<Msg> {
    pub label: String,
    pub value: String,
    pub description: Option<String>,
    pub align: ChoiceAlign,
    pub is_disabled: bool,
    pub end_content: Option<Element<Msg>>,
}

impl<Msg> CheckboxListItem<Msg> {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        CheckboxListItem {
            label: label.into(),
            value: value.into(),
            description: None,
            align: ChoiceAlign::Center,
            is_disabled: false,
            end_content: None,
        }
    }
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
    pub fn align(mut self, a: ChoiceAlign) -> Self {
        self.align = a;
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    pub fn end_content(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.end_content = Some(e.into());
        self
    }
}

/// A labeled group of checkboxes over a set of values (`CheckboxList`).
pub struct CheckboxList<Msg> {
    base: Base,
    field: FieldParts,
    value: Vec<String>,
    items: Vec<CheckboxListItem<Msg>>,
    on_change: Option<Rc<dyn Fn(Vec<String>) -> Msg>>,
    density: ListDensity,
    has_dividers: bool,
    is_read_only: bool,
    width: Option<SizeValue>,
}

impl<Msg> CheckboxList<Msg> {
    pub fn new(label: impl Into<String>, value: Vec<String>) -> Self {
        CheckboxList {
            base: Base::default(),
            field: FieldParts::new(label),
            value,
            items: Vec::new(),
            on_change: None,
            density: ListDensity::Balanced,
            has_dividers: false,
            is_read_only: false,
            width: None,
        }
    }
    pub fn item(mut self, item: CheckboxListItem<Msg>) -> Self {
        self.items.push(item);
        self
    }
    pub fn on_change(mut self, f: impl Fn(Vec<String>) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
    pub fn density(mut self, d: ListDensity) -> Self {
        self.density = d;
        self
    }
    pub fn has_dividers(mut self, on: bool) -> Self {
        self.has_dividers = on;
        self
    }
    pub fn is_read_only(mut self, on: bool) -> Self {
        self.is_read_only = on;
        self
    }
    pub fn width(mut self, w: SizeValue) -> Self {
        self.width = Some(w);
        self
    }
    field_methods!();
}

component!(CheckboxList<Msg>);

pub(crate) fn density_pad(t: &astryx_core::Tokens, d: ListDensity) -> f32 {
    match d {
        ListDensity::Compact => t.space.s1,
        ListDensity::Balanced => t.space.s2,
        ListDensity::Spacious => t.space.s3,
    }
}

impl<Msg: Clone + 'static> Build<Msg> for CheckboxList<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let f = &self.field;
        let mut outer = Style {
            flex_direction: FlexDirection::Column,
            gap: taffy::Size {
                width: len(0.0),
                height: len(t.space.s1),
            },
            ..Style::default()
        };
        if let Some(w) = self.width {
            outer.size.width = size_dim(w);
        }
        cx.key(self.base.id.as_deref());
        cx.begin(Node::new(outer).sem(Semantics::new(Role::Group, f.name(cx))));
        label_column(cx, f);
        let pad = density_pad(t, self.density);
        let n = self.items.len();
        for (i, item) in self.items.into_iter().enumerate() {
            let on = self.value.contains(&item.value);
            let disabled = f.is_disabled || item.is_disabled;
            let next: Vec<String> = if on {
                self.value
                    .iter()
                    .filter(|v| **v != item.value)
                    .cloned()
                    .collect()
            } else {
                let mut v = self.value.clone();
                v.push(item.value.clone());
                v
            };
            let mut sem = Semantics::new(Role::Checkbox, item.label.clone());
            sem.checked = Some(on);
            sem.disabled = disabled;
            sem.description = item.description.clone();
            cx.key_slot(i as u64);
            cx.begin(
                Node::new(Style {
                    flex_direction: FlexDirection::Column,
                    ..Style::default()
                })
                .interact(Interact::new(
                    self.on_change.as_ref().map(|c| c(next)),
                    disabled || self.is_read_only,
                ))
                .indicator(Indicator::ring(
                    color(t.color.accent),
                    radii(t.radius.inner),
                    None,
                ))
                .sem(sem),
            );
            let mark = CheckboxIndicator::new(if on {
                CheckboxState::Checked
            } else {
                CheckboxState::Unchecked
            })
            .size(IndicatorSize::Sm)
            .is_disabled(disabled)
            .into();
            choice_row(
                cx,
                mark,
                &item.label,
                item.description.as_deref(),
                item.align,
                disabled,
                pad,
                item.end_content,
            );
            if self.has_dividers && i + 1 < n {
                cx.leaf(
                    Node::new(Style {
                        size: taffy::Size {
                            width: taffy::Dimension::percent(1.0),
                            height: taffy::Dimension::length(t.size.border_width),
                        },
                        ..Style::default()
                    })
                    .draw(Draw::Box(BoxDraw::fill(color(t.color.border), radii(0.0)))),
                );
            }
            cx.end();
        }
        status_box(cx, f);
        cx.end();
    }
}

/// One option in a [`RadioList`] (`RadioListItem`).
pub struct RadioListItem<Msg> {
    pub label: String,
    pub value: String,
    pub description: Option<String>,
    pub align: ChoiceAlign,
    pub is_disabled: bool,
    pub end_content: Option<Element<Msg>>,
}

impl<Msg> RadioListItem<Msg> {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        RadioListItem {
            label: label.into(),
            value: value.into(),
            description: None,
            align: ChoiceAlign::Center,
            is_disabled: false,
            end_content: None,
        }
    }
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
    pub fn align(mut self, a: ChoiceAlign) -> Self {
        self.align = a;
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
}

/// RadioList `orientation`.
pub use super::button::Orientation as RadioOrientation;

/// One choice among options (`RadioList`): one tab stop; arrow keys move
/// the choice.
pub struct RadioList<Msg> {
    base: Base,
    field: FieldParts,
    value: String,
    items: Vec<RadioListItem<Msg>>,
    on_change: Option<Rc<dyn Fn(String) -> Msg>>,
    orientation: RadioOrientation,
    size: ChoiceSize,
    width: Option<SizeValue>,
}

impl<Msg> RadioList<Msg> {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        RadioList {
            base: Base::default(),
            field: FieldParts::new(label),
            value: value.into(),
            items: Vec::new(),
            on_change: None,
            orientation: RadioOrientation::Vertical,
            size: ChoiceSize::Md,
            width: None,
        }
    }
    pub fn item(mut self, item: RadioListItem<Msg>) -> Self {
        self.items.push(item);
        self
    }
    pub fn on_change(mut self, f: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
    pub fn orientation(mut self, o: RadioOrientation) -> Self {
        self.orientation = o;
        self
    }
    pub fn size(mut self, s: ChoiceSize) -> Self {
        self.size = s;
        self
    }
    pub fn width(mut self, w: SizeValue) -> Self {
        self.width = Some(w);
        self
    }
    field_methods!();
}

component!(RadioList<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for RadioList<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let f = &self.field;
        let mut outer = Style {
            flex_direction: FlexDirection::Column,
            gap: taffy::Size {
                width: len(0.0),
                height: len(t.space.s1),
            },
            ..Style::default()
        };
        if let Some(w) = self.width {
            outer.size.width = size_dim(w);
        }
        cx.key(self.base.id.as_deref());
        let root = cx.begin(Node::new(outer).sem(
            Semantics::new(Role::RadioGroup, f.name(cx)).with_state(|s| {
                s.required = f.is_required;
            }),
        ));
        let group = cx.id_of(root);
        label_column(cx, f);
        cx.begin(Node::new(Style {
            flex_direction: match self.orientation {
                RadioOrientation::Horizontal => FlexDirection::Row,
                RadioOrientation::Vertical => FlexDirection::Column,
            },
            gap: taffy::Size {
                width: len(t.space.s4),
                height: len(0.0),
            },
            flex_wrap: taffy::FlexWrap::Wrap,
            ..Style::default()
        }));
        let any_selected = self.items.iter().any(|i| i.value == self.value);
        for (i, item) in self.items.into_iter().enumerate() {
            let on = item.value == self.value;
            let disabled = f.is_disabled || item.is_disabled;
            let mut sem = Semantics::new(Role::Radio, item.label.clone());
            sem.checked = Some(on);
            sem.disabled = disabled;
            sem.description = item.description.clone();
            let mut node = Node::new(Style {
                flex_direction: FlexDirection::Column,
                ..Style::default()
            })
            .interact(Interact::new(
                self.on_change.as_ref().map(|c| c(item.value.clone())),
                disabled,
            ))
            .indicator(Indicator::ring(
                color(t.color.accent),
                radii(t.radius.full),
                None,
            ))
            .sem(sem);
            node.group = Some(group);
            node.group_entry = on || (!any_selected && i == 0);
            cx.key_slot(i as u64);
            cx.begin(node);
            let mark = RadioIndicator::new(on)
                .size(self.size.into())
                .is_disabled(disabled)
                .into();
            choice_row(
                cx,
                mark,
                &item.label,
                item.description.as_deref(),
                item.align,
                disabled,
                t.space.s1,
                item.end_content,
            );
            cx.end();
        }
        cx.end();
        status_box(cx, f);
        cx.end();
    }
}
