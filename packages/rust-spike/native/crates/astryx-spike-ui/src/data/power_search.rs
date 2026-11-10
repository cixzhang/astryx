//! PowerSearch: a search field of structured filter tokens (stretch).
//!
//! A subset of Astryx's operator value types: empty, string, integer,
//! float, enum and date. Typing suggests fields to filter by, enum values
//! and a contains-match on the content field; picking a field then offers
//! its operators, and the typed text becomes the value.

use super::calendar::CivilDate;
use super::search::{Listbox, OptionRow};
use crate::controls::{Button, ButtonVariant, ComboHooks, TextInput};
use crate::primitives::{Icon, IconSize};
use crate::util::{color, len, radii};
use astryx_core::authoring::{BoxDraw, Build, BuildCx, Draw, KeyResponse, LocalHandle, Node};
use astryx_core::{Element, InputPhase, Key, KeyEvent};
use astryx_spike_foundations::ComponentSize;
use astryx_spike_layers::placement::{Align, Placement, Side};
use astryx_spike_layers::Layer;
use std::rc::Rc;
use taffy::{AlignItems, FlexDirection, Style};

/// What an operator takes (`OperatorValue`).
#[derive(Clone, Debug, PartialEq)]
pub enum OperatorValue {
    Empty,
    String,
    Integer,
    Float,
    Enum(Vec<(String, String)>),
    Date,
}

/// One operator of a field (`PowerSearchOperator`).
#[derive(Clone, Debug, PartialEq)]
pub struct PowerSearchOperator {
    pub key: String,
    pub label: String,
    pub value: OperatorValue,
}

impl PowerSearchOperator {
    pub fn new(key: impl Into<String>, label: impl Into<String>, value: OperatorValue) -> Self {
        PowerSearchOperator {
            key: key.into(),
            label: label.into(),
            value,
        }
    }
}

/// A field filters can use (`PowerSearchField`).
#[derive(Clone, Debug, PartialEq)]
pub struct PowerSearchField {
    pub key: String,
    pub label: String,
    pub operators: Vec<PowerSearchOperator>,
}

impl PowerSearchField {
    pub fn new(
        key: impl Into<String>,
        label: impl Into<String>,
        operators: Vec<PowerSearchOperator>,
    ) -> Self {
        PowerSearchField {
            key: key.into(),
            label: label.into(),
            operators,
        }
    }
}

/// The fields, and which one free text searches (`PowerSearchConfig`).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PowerSearchConfig {
    pub fields: Vec<PowerSearchField>,
    pub content_search_field_key: Option<String>,
}

/// A filter's value (`FilterValue`).
#[derive(Clone, Debug, PartialEq)]
pub enum FilterValue {
    Empty,
    String(String),
    Integer(i64),
    Float(f64),
    Enum(String),
    Date(CivilDate),
}

/// One filter (`PowerSearchFilter`).
#[derive(Clone, Debug, PartialEq)]
pub struct PowerSearchFilter {
    pub field: String,
    pub operator: String,
    pub value: FilterValue,
}

/// How the filters changed (`changeType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterChange {
    Add,
    Edit,
    Remove,
}

type OnFilters<Msg> = Rc<dyn Fn(Vec<PowerSearchFilter>, FilterChange, usize) -> Msg>;

#[derive(Clone, Debug, Default, PartialEq)]
struct SearchState {
    query: String,
    /// The field, then the operator, picked for the filter being made.
    field: Option<String>,
    operator: Option<String>,
    open: bool,
    active: Option<usize>,
}

/// A suggestion and what picking it does.
#[derive(Clone, Debug, PartialEq)]
enum Pick {
    Field(String),
    Operator(String),
    Filter(PowerSearchFilter),
}

/// A search field of filter tokens (`PowerSearch`).
pub struct PowerSearch<Msg> {
    id: String,
    label: String,
    config: PowerSearchConfig,
    filters: Vec<PowerSearchFilter>,
    on_change: OnFilters<Msg>,
    placeholder: String,
    is_disabled: bool,
    is_read_only: bool,
    max_results: usize,
    size: Option<ComponentSize>,
}

impl<Msg> PowerSearch<Msg> {
    pub fn new(
        id: impl Into<String>,
        config: PowerSearchConfig,
        filters: Vec<PowerSearchFilter>,
        on_change: impl Fn(Vec<PowerSearchFilter>, FilterChange, usize) -> Msg + 'static,
    ) -> Self {
        PowerSearch {
            id: id.into(),
            label: "Search".into(),
            config,
            filters,
            on_change: Rc::new(on_change),
            placeholder: "Search...".into(),
            is_disabled: false,
            is_read_only: false,
            max_results: 10,
            size: None,
        }
    }
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = l.into();
        self
    }
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    pub fn is_read_only(mut self, on: bool) -> Self {
        self.is_read_only = on;
        self
    }
    pub fn max_search_results(mut self, n: usize) -> Self {
        self.max_results = n;
        self
    }
    pub fn size(mut self, s: ComponentSize) -> Self {
        self.size = Some(s);
        self
    }
}

impl<Msg: Clone + 'static> From<PowerSearch<Msg>> for Element<Msg> {
    fn from(p: PowerSearch<Msg>) -> Self {
        astryx_core::authoring::element(p)
    }
}

/// The value `text` gives an operator, when it is one.
pub fn parse_filter_value(op: &OperatorValue, text: &str) -> Option<FilterValue> {
    let t = text.trim();
    match op {
        OperatorValue::Empty => Some(FilterValue::Empty),
        OperatorValue::String => (!t.is_empty()).then(|| FilterValue::String(t.to_string())),
        OperatorValue::Integer => t.parse().ok().map(FilterValue::Integer),
        OperatorValue::Float => t.parse().ok().map(FilterValue::Float),
        OperatorValue::Enum(items) => items
            .iter()
            .find(|(k, l)| k.eq_ignore_ascii_case(t) || l.eq_ignore_ascii_case(t))
            .map(|(k, _)| FilterValue::Enum(k.clone())),
        OperatorValue::Date => CivilDate::parse(t).map(FilterValue::Date),
    }
}

impl PowerSearchConfig {
    fn field(&self, key: &str) -> Option<&PowerSearchField> {
        self.fields.iter().find(|f| f.key == key)
    }

    /// A filter in words: `Status is Open`.
    pub fn describe(&self, f: &PowerSearchFilter) -> String {
        let Some(field) = self.field(&f.field) else {
            return f.field.clone();
        };
        let op = field.operators.iter().find(|o| o.key == f.operator);
        let value = match (&f.value, op.map(|o| &o.value)) {
            (FilterValue::Empty, _) => String::new(),
            (FilterValue::String(s), _) => format!("\u{201c}{s}\u{201d}"),
            (FilterValue::Integer(n), _) => n.to_string(),
            (FilterValue::Float(n), _) => n.to_string(),
            (FilterValue::Date(d), _) => d.iso(),
            (FilterValue::Enum(k), Some(OperatorValue::Enum(items))) => items
                .iter()
                .find(|(kk, _)| kk == k)
                .map_or(k.clone(), |(_, l)| l.clone()),
            (FilterValue::Enum(k), _) => k.clone(),
        };
        let op_label = op.map_or(f.operator.clone(), |o| o.label.clone());
        format!("{} {} {}", field.label, op_label, value)
            .trim()
            .to_string()
    }

    fn suggestions(&self, s: &SearchState) -> Vec<(String, Pick)> {
        let q = s.query.trim().to_lowercase();
        match (&s.field, &s.operator) {
            (Some(fk), None) => self
                .field(fk)
                .map(|f| {
                    f.operators
                        .iter()
                        .filter(|o| q.is_empty() || o.label.to_lowercase().contains(&q))
                        .map(|o| {
                            (
                                format!("{} {}", f.label, o.label),
                                Pick::Operator(o.key.clone()),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
            (Some(fk), Some(ok)) => {
                let Some(f) = self.field(fk) else {
                    return Vec::new();
                };
                let Some(op) = f.operators.iter().find(|o| &o.key == ok) else {
                    return Vec::new();
                };
                match &op.value {
                    OperatorValue::Enum(items) => items
                        .iter()
                        .filter(|(_, l)| q.is_empty() || l.to_lowercase().contains(&q))
                        .map(|(k, l)| {
                            (
                                format!("{} {} {l}", f.label, op.label),
                                Pick::Filter(PowerSearchFilter {
                                    field: f.key.clone(),
                                    operator: op.key.clone(),
                                    value: FilterValue::Enum(k.clone()),
                                }),
                            )
                        })
                        .collect(),
                    v => parse_filter_value(v, &s.query)
                        .map(|value| {
                            let filter = PowerSearchFilter {
                                field: f.key.clone(),
                                operator: op.key.clone(),
                                value,
                            };
                            vec![(self.describe(&filter), Pick::Filter(filter))]
                        })
                        .unwrap_or_default(),
                }
            }
            _ => {
                if q.is_empty() {
                    return Vec::new();
                }
                let mut out = Vec::new();
                for f in &self.fields {
                    if f.label.to_lowercase().starts_with(&q)
                        || f.key.to_lowercase().starts_with(&q)
                    {
                        out.push((format!("Filter by {}", f.label), Pick::Field(f.key.clone())));
                    }
                    for op in &f.operators {
                        if let OperatorValue::Enum(items) = &op.value {
                            for (k, l) in items {
                                if l.to_lowercase().starts_with(&q) {
                                    out.push((
                                        format!("{} {} {l}", f.label, op.label),
                                        Pick::Filter(PowerSearchFilter {
                                            field: f.key.clone(),
                                            operator: op.key.clone(),
                                            value: FilterValue::Enum(k.clone()),
                                        }),
                                    ));
                                }
                            }
                        }
                    }
                }
                if let Some(ck) = &self.content_search_field_key {
                    if let Some(f) = self.field(ck) {
                        if let Some(op) = f
                            .operators
                            .iter()
                            .find(|o| o.value == OperatorValue::String)
                        {
                            let filter = PowerSearchFilter {
                                field: f.key.clone(),
                                operator: op.key.clone(),
                                value: FilterValue::String(s.query.trim().to_string()),
                            };
                            out.push((self.describe(&filter), Pick::Filter(filter)));
                        }
                    }
                }
                out
            }
        }
    }
}

impl<Msg: Clone + 'static> Build<Msg> for PowerSearch<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let c = &t.color;
        cx.key(Some(&format!("{}:power", self.id)));
        let h: LocalHandle<SearchState> = cx.local_handle();
        let s = h.get();
        let config = Rc::new(self.config.clone());
        let mut results = config.suggestions(&s);
        results.truncate(self.max_results);
        let active = s
            .active
            .filter(|a| *a < results.len())
            .or((!results.is_empty()).then_some(0));
        let editable = !self.is_disabled && !self.is_read_only;
        let list_id = format!("{}:suggestions", self.id);
        let option_id = |i: usize| format!("{}:suggestion-{i}", self.id);
        // Picking a suggestion: a field, then an operator, then a filter.
        let pick = {
            let h = h.clone();
            let filters = self.filters.clone();
            let on_change = self.on_change.clone();
            let config = config.clone();
            Rc::new(move |p: &Pick| -> Option<Msg> {
                let mut st = h.get();
                match p {
                    Pick::Field(k) => {
                        st.field = Some(k.clone());
                        st.operator = config
                            .field(k)
                            .filter(|f| f.operators.len() == 1)
                            .map(|f| f.operators[0].key.clone());
                        st.query.clear();
                        st.active = None;
                        st.open = true;
                        h.set(st);
                        None
                    }
                    Pick::Operator(k) => {
                        st.operator = Some(k.clone());
                        st.query.clear();
                        st.active = None;
                        h.set(st);
                        None
                    }
                    Pick::Filter(f) => {
                        h.set(SearchState::default());
                        let mut next = filters.clone();
                        next.push(f.clone());
                        let at = next.len() - 1;
                        Some(on_change(next, FilterChange::Add, at))
                    }
                }
            })
        };
        let on_key = {
            let h = h.clone();
            let results: Vec<Pick> = results.iter().map(|(_, p)| p.clone()).collect();
            let pick = pick.clone();
            let filters = self.filters.clone();
            let on_change = self.on_change.clone();
            Rc::new(move |e: &KeyEvent| -> KeyResponse<Msg> {
                if e.phase != InputPhase::Pressed || !editable {
                    return KeyResponse::Pass;
                }
                let mut st = h.get();
                let n = results.len();
                match e.key {
                    Key::Down | Key::Up if n > 0 => {
                        let a = active.unwrap_or(0);
                        st.active = Some(if e.key == Key::Down {
                            (a + 1) % n
                        } else {
                            (a + n - 1) % n
                        });
                        st.open = true;
                        h.set(st);
                        KeyResponse::Handled
                    }
                    Key::Enter => match active.and_then(|a| results.get(a)) {
                        Some(p) => match pick(p) {
                            Some(m) => KeyResponse::Send(m),
                            None => KeyResponse::Handled,
                        },
                        None => KeyResponse::Handled,
                    },
                    Key::Escape if st.open || st.field.is_some() => {
                        h.set(SearchState::default());
                        KeyResponse::Handled
                    }
                    Key::Backspace if st.query.is_empty() => {
                        if st.operator.is_some() {
                            st.operator = None;
                            h.set(st);
                            KeyResponse::Handled
                        } else if st.field.is_some() {
                            st.field = None;
                            h.set(st);
                            KeyResponse::Handled
                        } else if let Some(last) = filters.len().checked_sub(1) {
                            let mut next = filters.clone();
                            next.pop();
                            KeyResponse::Send(on_change(next, FilterChange::Remove, last))
                        } else {
                            KeyResponse::Pass
                        }
                    }
                    _ => KeyResponse::Pass,
                }
            })
        };
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            ..Style::default()
        }));
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Row,
            flex_wrap: taffy::FlexWrap::Wrap,
            align_items: AlignItems::CENTER,
            gap: taffy::Size {
                width: len(t.space.s1),
                height: len(t.space.s1),
            },
            ..Style::default()
        }));
        // The filter tokens, each removable.
        for (i, f) in self.filters.iter().enumerate() {
            let mut rest = self.filters.clone();
            rest.remove(i);
            let words = config.describe(f);
            cx.begin(
                Node::new(Style {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::CENTER,
                    padding: taffy::Rect {
                        left: len(t.space.s2),
                        right: len(0.0),
                        top: len(0.0),
                        bottom: len(0.0),
                    },
                    ..Style::default()
                })
                .draw(Draw::Box(BoxDraw::fill(
                    color(c.background_muted),
                    radii(t.radius.element),
                ))),
            );
            cx.child(crate::primitives::Text::new(words.clone()).into());
            if editable {
                cx.child(
                    Button::new(format!("Remove {words}"))
                        .variant(ButtonVariant::Ghost)
                        .size(ComponentSize::Sm)
                        .icon(Icon::new("close").size(IconSize::Xsm))
                        .is_icon_only(true)
                        .on_click((self.on_change)(rest, FilterChange::Remove, i))
                        .into(),
                );
            }
            cx.end();
        }
        // The step being built: field, then operator.
        if let Some(fk) = &s.field {
            let f = config.field(fk);
            let mut words = f.map_or(fk.clone(), |f| f.label.clone());
            if let (Some(ok), Some(f)) = (&s.operator, f) {
                if let Some(o) = f.operators.iter().find(|o| &o.key == ok) {
                    words = format!("{words} {}", o.label);
                }
            }
            cx.child(crate::controls::Token::new(words).into());
        }
        let edit = {
            let h = h.clone();
            Rc::new(move |text: String| {
                let mut st = h.get();
                st.query = text;
                st.open = true;
                st.active = None;
                h.set(st);
            }) as Rc<dyn Fn(String)>
        };
        let mut input = TextInput::new(self.label.clone(), s.query.clone())
            .is_label_hidden(true)
            .is_disabled(self.is_disabled)
            .is_read_only(self.is_read_only)
            .start_icon(Icon::new("search"))
            .placeholder(self.placeholder.clone())
            .id(self.id.clone())
            .on_edit(edit)
            .combo(ComboHooks {
                expanded: !results.is_empty(),
                active_descendant: active.map(option_id),
                controls: Some(list_id.clone()),
                on_key: Some(on_key),
                has_popup_listbox: true,
            });
        if let Some(sz) = self.size {
            input = input.size(sz);
        }
        cx.child(input.into());
        cx.end();
        if !results.is_empty() && editable {
            let rows: Vec<OptionRow<Msg>> = results
                .iter()
                .enumerate()
                .map(|(i, (label, p))| {
                    let pick = pick.clone();
                    let p2 = p.clone();
                    let msg = match p {
                        Pick::Filter(f) => {
                            let mut next = self.filters.clone();
                            next.push(f.clone());
                            let at = next.len() - 1;
                            Some((self.on_change)(next, FilterChange::Add, at))
                        }
                        _ => None,
                    };
                    OptionRow {
                        id: option_id(i),
                        label: label.clone(),
                        description: None,
                        selected: false,
                        active: active == Some(i),
                        disabled: false,
                        check: false,
                        msg,
                        effect: Some(Rc::new(move || {
                            let _ = pick(&p2);
                        })),
                    }
                })
                .collect();
            let list = Listbox {
                id: list_id,
                label: self.label.clone(),
                rows,
                empty_text: None,
                width: None,
                max_height: 320.0,
                multiselectable: false,
                header: None,
                bare: false,
            };
            let hh = h.clone();
            cx.child(
                Layer::popover(&self.id, astryx_core::authoring::element(list))
                    .placement(Placement::new(Side::Below, Align::Start).offset(t.space.s1))
                    .match_anchor_width(true)
                    .trap_focus(false)
                    .restore_focus(false)
                    .no_role()
                    .on_dismiss_effect(move |_| {
                        let mut st = hh.get();
                        st.open = false;
                        hh.set(st);
                    })
                    .into(),
            );
        }
        cx.end();
    }
}
