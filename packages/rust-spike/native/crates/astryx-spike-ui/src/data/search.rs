//! The combobox family: Typeahead, Selector, MultiSelector, Tokenizer and
//! CommandPalette, on one listbox popup.

use crate::controls::{Button, ButtonVariant, ComboHooks, TextInput, Token};
use crate::primitives::{CheckIndicator, Icon, IconSize};
use crate::util::{color, len, len_auto, radii};
use astryx_core::authoring::{
    text_draw, text_node, type_style, Base, BoxDraw, Build, BuildCx, Draw, Interact, KeyResponse,
    LocalHandle, Node, Semantics,
};
use astryx_core::contract::{FontFamily, Role};
use astryx_core::{Element, InputPhase, Key, KeyEvent};
use astryx_spike_foundations::{args, ComponentSize, CxExt};
use astryx_spike_layers::placement::{Align, Placement, Side};
use astryx_spike_layers::Layer;
use std::rc::Rc;
use taffy::{AlignItems, Dimension, FlexDirection, LengthPercentageAuto, Style};

/// What a search source returns: anything with a stable key and a label.
pub trait SearchItem: Clone + 'static {
    fn key(&self) -> String;
    fn label(&self) -> String;
    fn description(&self) -> Option<String> {
        None
    }
}

/// A plain entry (`{id, label}`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Entry {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
}

impl Entry {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Entry {
            id: id.into(),
            label: label.into(),
            description: None,
        }
    }
}

impl SearchItem for Entry {
    fn key(&self) -> String {
        self.id.clone()
    }
    fn label(&self) -> String {
        self.label.clone()
    }
    fn description(&self) -> Option<String> {
        self.description.clone()
    }
}

/// Finds items for a query (`SearchSource`). Synchronous: an app that
/// searches a server keeps results in its model and returns them here.
pub type SearchSource<T> = Rc<dyn Fn(&str) -> Vec<T>>;

/// A source over a fixed list (`createStaticSource`): items whose label
/// has a word starting with the query, case-insensitive, then items that
/// contain it.
pub fn static_source<T: SearchItem>(items: Vec<T>) -> SearchSource<T> {
    Rc::new(move |q: &str| {
        let q = q.trim().to_lowercase();
        if q.is_empty() {
            return items.clone();
        }
        let mut starts = Vec::new();
        let mut contains = Vec::new();
        for it in &items {
            let l = it.label().to_lowercase();
            if l.split(|ch: char| !ch.is_alphanumeric())
                .any(|w| w.starts_with(&q))
            {
                starts.push(it.clone());
            } else if l.contains(&q) {
                contains.push(it.clone());
            }
        }
        starts.extend(contains);
        starts
    })
}

/// One option row of a listbox.
pub(crate) struct OptionRow<Msg> {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub selected: bool,
    pub active: bool,
    pub disabled: bool,
    pub check: bool,
    pub msg: Option<Msg>,
    pub effect: Option<Rc<dyn Fn()>>,
}

/// The popup list of options (role listbox). Options are not focus stops:
/// the field keeps focus and points at the active option.
pub(crate) struct Listbox<Msg> {
    pub id: String,
    pub label: String,
    pub rows: Vec<OptionRow<Msg>>,
    pub empty_text: Option<String>,
    pub width: Option<f32>,
    pub max_height: f32,
    pub multiselectable: bool,
    pub header: Option<Element<Msg>>,
    /// Drawn inside another surface (a palette): no ground of its own.
    pub bare: bool,
}

impl<Msg: Clone + 'static> Build<Msg> for Listbox<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let c = &t.color;
        let r = radii(t.radius.container.min(t.space.s2 + t.space.s1 * 2.0));
        let mut node = Node::new(Style {
            flex_direction: FlexDirection::Column,
            gap: taffy::Size {
                width: len(0.0),
                height: len(t.space.s0_5),
            },
            padding: crate::util::rect(len(t.space.s1)),
            size: taffy::Size {
                width: self.width.map_or(Dimension::auto(), Dimension::length),
                height: Dimension::auto(),
            },
            min_size: taffy::Size {
                width: LengthPercentageAuto::length(160.0),
                height: LengthPercentageAuto::auto(),
            },
            max_size: taffy::Size {
                width: LengthPercentageAuto::auto(),
                height: LengthPercentageAuto::length(self.max_height),
            },
            overflow: taffy::Point {
                x: taffy::Overflow::Visible,
                y: taffy::Overflow::Scroll,
            },
            ..Style::default()
        })
        .draw(Draw::Box(if self.bare {
            BoxDraw::fill(astryx_core::contract::Color::TRANSPARENT, r)
        } else {
            BoxDraw {
                shadow: t.shadow.med,
                border: Some((color(c.border), t.size.border_width)),
                ..BoxDraw::fill(color(c.background_popover), r)
            }
        }));
        node.scroll = Some(astryx_core::authoring::Axis::Vertical);
        node.clip = Some(r);
        cx.begin(node);
        if let Some(h) = self.header {
            cx.child(h);
        }
        let mut sem = Semantics::new(Role::ListBox, self.label);
        sem.state.multiselectable = self.multiselectable;
        cx.key(Some(&self.id));
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
        if self.rows.is_empty() {
            if let Some(e) = self.empty_text {
                let mut n = text_node(
                    e,
                    type_style(t.font.label, FontFamily::Body),
                    color(c.text_secondary),
                );
                n.style.padding = crate::util::rect(len(t.space.s2));
                cx.leaf(n);
            }
        }
        let rr = radii((t.space.s2 - t.space.s1).max(0.0) + 4.0);
        for row in self.rows {
            let mut sem = Semantics::new(Role::ListBoxOption, row.label.clone());
            sem.selected = Some(row.selected);
            sem.disabled = row.disabled;
            sem.description = row.description.clone();
            let ground = if row.active {
                color(c.overlay_hover)
            } else {
                astryx_core::contract::Color::TRANSPARENT
            };
            // Pointer only: the field keeps keyboard focus and points at
            // the active option.
            let mut interact = Interact::new(row.msg, row.disabled).pointer_only();
            if let Some(e) = row.effect {
                interact = interact.effect(move || e());
            }
            let n = Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                gap: taffy::Size {
                    width: len(t.space.s2),
                    height: len(0.0),
                },
                padding: taffy::Rect {
                    left: len(t.space.s2),
                    right: len(t.space.s2),
                    top: len(t.space.s1_5),
                    bottom: len(t.space.s1_5),
                },
                ..Style::default()
            })
            .draw(Draw::Box(BoxDraw::fill(ground, rr)))
            .variant(
                astryx_core::layer::When::Hovered,
                Draw::Box(BoxDraw::fill(color(c.overlay_hover), rr)),
            )
            .interact(interact)
            .sem(sem)
            .opacity(if row.disabled { 0.5 } else { 1.0 });
            cx.key(Some(&row.id));
            cx.begin(n);
            if row.check {
                cx.child(CheckIndicator::new(row.selected).into());
            }
            cx.begin(Node::new(Style {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_size: taffy::Size {
                    width: len_auto(0.0),
                    height: LengthPercentageAuto::auto(),
                },
                ..Style::default()
            }));
            let mut l = text_node(
                row.label,
                type_style(t.font.label, FontFamily::Body),
                color(c.text_primary),
            );
            text_draw(&mut l).max_lines = Some(1);
            l.sem = None;
            cx.leaf(l);
            if let Some(d) = row.description {
                let mut dn = text_node(
                    d,
                    type_style(t.font.supporting, FontFamily::Body),
                    color(c.text_secondary),
                );
                dn.sem = None;
                cx.leaf(dn);
            }
            cx.end();
            if row.selected && !row.check {
                if let Some(ic) =
                    crate::primitives::icon_node::<Msg>("check", 16.0, color(c.icon_primary))
                {
                    cx.leaf(ic);
                }
            }
            cx.end();
        }
        cx.end();
        cx.end();
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct ComboState {
    /// The typed text; `None` shows the value's label.
    query: Option<String>,
    open: bool,
    active: Option<usize>,
}

/// Moves the active option with the keys a combobox uses; `None` when the
/// key is not one of them.
fn step_active(key: &Key, active: Option<usize>, len: usize) -> Option<Option<usize>> {
    if len == 0 {
        return match key {
            Key::Down | Key::Up => Some(None),
            _ => None,
        };
    }
    Some(match key {
        Key::Down => Some(active.map_or(0, |a| (a + 1) % len)),
        Key::Up => Some(active.map_or(len - 1, |a| (a + len - 1) % len)),
        Key::Home => Some(0),
        Key::End => Some(len - 1),
        _ => return None,
    })
}

/// A text field that suggests items as the person types and picks one
/// (`Typeahead`).
pub struct Typeahead<T, Msg> {
    base: Base,
    id: String,
    label: String,
    source: SearchSource<T>,
    value: Option<T>,
    on_change: Option<Rc<dyn Fn(Option<T>) -> Msg>>,
    placeholder: Option<String>,
    has_entries_on_focus: bool,
    has_clear: bool,
    is_disabled: bool,
    max_menu_items: usize,
    min_query_length: usize,
    empty_search_text: Option<String>,
    size: Option<ComponentSize>,
    is_label_hidden: bool,
    description: Option<String>,
    status: Option<crate::controls::Status>,
    status_variant: crate::controls::StatusVariant,
    start_icon: Option<Icon>,
    width: Option<crate::layout::SizeValue>,
}

impl<T: SearchItem, Msg> Typeahead<T, Msg> {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        source: SearchSource<T>,
        value: Option<T>,
    ) -> Self {
        Typeahead {
            base: Base::default(),
            id: id.into(),
            label: label.into(),
            source,
            value,
            on_change: None,
            placeholder: None,
            has_entries_on_focus: false,
            has_clear: true,
            is_disabled: false,
            max_menu_items: 10,
            min_query_length: 1,
            empty_search_text: None,
            size: None,
            is_label_hidden: false,
            description: None,
            status: None,
            status_variant: crate::controls::StatusVariant::Attached,
            start_icon: None,
            width: None,
        }
    }
    pub fn on_change(mut self, f: impl Fn(Option<T>) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = Some(p.into());
        self
    }
    pub fn has_entries_on_focus(mut self, on: bool) -> Self {
        self.has_entries_on_focus = on;
        self
    }
    pub fn has_clear(mut self, on: bool) -> Self {
        self.has_clear = on;
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    pub fn max_menu_items(mut self, n: usize) -> Self {
        self.max_menu_items = n;
        self
    }
    pub fn min_query_length(mut self, n: usize) -> Self {
        self.min_query_length = n;
        self
    }
    pub fn empty_search_text(mut self, s: impl Into<String>) -> Self {
        self.empty_search_text = Some(s.into());
        self
    }
    pub fn size(mut self, s: ComponentSize) -> Self {
        self.size = Some(s);
        self
    }
    pub fn is_label_hidden(mut self, on: bool) -> Self {
        self.is_label_hidden = on;
        self
    }
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
    pub fn status(mut self, s: crate::controls::Status) -> Self {
        self.status = Some(s);
        self
    }
    pub fn status_variant(mut self, v: crate::controls::StatusVariant) -> Self {
        self.status_variant = v;
        self
    }
    pub fn start_icon(mut self, i: Icon) -> Self {
        self.start_icon = Some(i);
        self
    }
    pub fn width(mut self, w: crate::layout::SizeValue) -> Self {
        self.width = Some(w);
        self
    }
}

impl<T: SearchItem, Msg: Clone + 'static> From<Typeahead<T, Msg>> for Element<Msg> {
    fn from(t: Typeahead<T, Msg>) -> Self {
        astryx_core::authoring::element(t)
    }
}

impl<T: SearchItem, Msg: Clone + 'static> Build<Msg> for Typeahead<T, Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        cx.key(Some(&format!("{}:combo", self.id)));
        let h: LocalHandle<ComboState> = cx.local_handle();
        let s = h.get();
        let shown = s
            .query
            .clone()
            .unwrap_or_else(|| self.value.as_ref().map(|v| v.label()).unwrap_or_default());
        let query = s.query.clone().unwrap_or_default();
        let searching =
            s.open && (query.chars().count() >= self.min_query_length || self.has_entries_on_focus);
        let results: Vec<T> = if searching {
            (self.source)(&query)
                .into_iter()
                .take(self.max_menu_items)
                .collect()
        } else {
            Vec::new()
        };
        let list_id = format!("{}:listbox", self.id);
        let option_id = |i: usize| format!("{}:option-{i}", self.id);
        let active = s.active.filter(|a| *a < results.len());
        // Keys on the field.
        let on_key = {
            let h = h.clone();
            let results = results.clone();
            let on_change = self.on_change.clone();
            Rc::new(move |e: &KeyEvent| -> KeyResponse<Msg> {
                if e.phase != InputPhase::Pressed {
                    return KeyResponse::Pass;
                }
                let mut st = h.get();
                if let Some(next) = step_active(&e.key, active, results.len()) {
                    st.open = true;
                    st.active = next;
                    h.set(st);
                    return KeyResponse::Handled;
                }
                match e.key {
                    Key::Enter if st.open => {
                        let Some(item) = active.and_then(|a| results.get(a)).cloned() else {
                            return KeyResponse::Pass;
                        };
                        h.set(ComboState::default());
                        match &on_change {
                            Some(f) => KeyResponse::Send(f(Some(item))),
                            None => KeyResponse::Handled,
                        }
                    }
                    Key::Escape if st.open => {
                        st.open = false;
                        st.active = None;
                        h.set(st);
                        KeyResponse::Handled
                    }
                    _ => KeyResponse::Pass,
                }
            })
        };
        let edit = {
            let h = h.clone();
            Rc::new(move |text: String| {
                h.set(ComboState {
                    query: Some(text),
                    open: true,
                    active: None,
                })
            }) as Rc<dyn Fn(String)>
        };
        let mut input = TextInput::new(self.label.clone(), shown)
            .is_label_hidden(self.is_label_hidden)
            .is_disabled(self.is_disabled)
            .has_clear(self.has_clear)
            .id(self.id.clone())
            .on_edit(edit)
            .combo(ComboHooks {
                expanded: searching,
                active_descendant: active.map(option_id),
                controls: Some(list_id.clone()),
                on_key: Some(on_key),
                has_popup_listbox: true,
            });
        if let Some(p) = &self.placeholder {
            input = input.placeholder(p.clone());
        }
        if let Some(sz) = self.size {
            input = input.size(sz);
        }
        if let Some(d) = &self.description {
            input = input.description(d.clone());
        }
        if let Some(st) = &self.status {
            input = input.status(st.clone()).status_variant(self.status_variant);
        }
        if let Some(i) = self.start_icon.clone() {
            input = input.start_icon(i);
        }
        if let Some(w) = self.width {
            input = input.width(w);
        }
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            ..Style::default()
        }));
        cx.child(input.into());
        if searching {
            let value_key = self.value.as_ref().map(|v| v.key());
            let rows = results
                .iter()
                .enumerate()
                .map(|(i, it)| {
                    let h = h.clone();
                    OptionRow {
                        id: option_id(i),
                        label: it.label(),
                        description: it.description(),
                        selected: value_key.as_deref() == Some(&it.key()),
                        active: active == Some(i),
                        disabled: false,
                        check: false,
                        msg: self.on_change.as_ref().map(|f| f(Some(it.clone()))),
                        effect: Some(Rc::new(move || h.set(ComboState::default()))),
                    }
                })
                .collect();
            let empty = self
                .empty_search_text
                .clone()
                .unwrap_or_else(|| cx.t("@astryx.typeahead.emptySearch", &args! {}));
            let list = Listbox {
                id: list_id,
                label: self.label.clone(),
                rows,
                empty_text: Some(if empty.starts_with('@') {
                    "No results found".into()
                } else {
                    empty
                }),
                width: None,
                max_height: 320.0,
                multiselectable: false,
                header: None,
                bare: false,
            };
            let t = cx.tokens;
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

/// One Selector option (`SelectorOption`).
#[derive(Clone, Debug, PartialEq)]
pub struct SelectorOption {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub is_disabled: bool,
}

impl SelectorOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        SelectorOption {
            value: value.into(),
            label: label.into(),
            description: None,
            icon: None,
            is_disabled: false,
        }
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
}

/// Selector `variant`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SelectorVariant {
    #[default]
    Input,
    Secondary,
    Ghost,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct SelectState {
    open: bool,
    active: Option<usize>,
}

/// A button that opens a list of options and shows the chosen ones:
/// `Selector` (one value) and `MultiSelector` (several).
pub struct Selector<Msg> {
    base: Base,
    id: String,
    label: String,
    options: Vec<SelectorOption>,
    values: Vec<String>,
    multiple: bool,
    on_change: Option<Rc<dyn Fn(Vec<String>) -> Msg>>,
    placeholder: String,
    size: Option<ComponentSize>,
    variant: SelectorVariant,
    is_disabled: bool,
    is_label_hidden: bool,
    has_select_all: bool,
    max_badges: usize,
    trigger_display: TriggerDisplay,
    width: Option<crate::layout::SizeValue>,
}

/// MultiSelector `triggerDisplay`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TriggerDisplay {
    #[default]
    Count,
    Labels,
    Badges,
}

impl<Msg> Selector<Msg> {
    /// One value (`Selector`).
    pub fn single(
        id: impl Into<String>,
        label: impl Into<String>,
        options: Vec<SelectorOption>,
        value: Option<String>,
    ) -> Self {
        Self::make(id, label, options, value.into_iter().collect(), false)
    }
    /// Several values (`MultiSelector`).
    pub fn multiple(
        id: impl Into<String>,
        label: impl Into<String>,
        options: Vec<SelectorOption>,
        values: Vec<String>,
    ) -> Self {
        Self::make(id, label, options, values, true)
    }
    fn make(
        id: impl Into<String>,
        label: impl Into<String>,
        options: Vec<SelectorOption>,
        values: Vec<String>,
        multiple: bool,
    ) -> Self {
        Selector {
            base: Base::default(),
            id: id.into(),
            label: label.into(),
            options,
            values,
            multiple,
            on_change: None,
            placeholder: "Select...".into(),
            size: None,
            variant: SelectorVariant::Input,
            is_disabled: false,
            is_label_hidden: false,
            has_select_all: false,
            max_badges: 3,
            trigger_display: TriggerDisplay::Count,
            width: None,
        }
    }
    /// Gets the new values (one at most for a single Selector).
    pub fn on_change(mut self, f: impl Fn(Vec<String>) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }
    pub fn size(mut self, s: ComponentSize) -> Self {
        self.size = Some(s);
        self
    }
    pub fn variant(mut self, v: SelectorVariant) -> Self {
        self.variant = v;
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    pub fn is_label_hidden(mut self, on: bool) -> Self {
        self.is_label_hidden = on;
        self
    }
    pub fn has_select_all(mut self, on: bool) -> Self {
        self.has_select_all = on;
        self
    }
    pub fn trigger_display(mut self, d: TriggerDisplay) -> Self {
        self.trigger_display = d;
        self
    }
    pub fn max_badges(mut self, n: usize) -> Self {
        self.max_badges = n;
        self
    }
    pub fn width(mut self, w: crate::layout::SizeValue) -> Self {
        self.width = Some(w);
        self
    }
}

crate::util::component!(Selector<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for Selector<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        cx.key(Some(&format!("{}:select", self.id)));
        let h: LocalHandle<SelectState> = cx.local_handle();
        let s = h.get();
        let size = cx.size(self.size, ComponentSize::Md);
        let chosen: Vec<&SelectorOption> = self
            .options
            .iter()
            .filter(|o| self.values.contains(&o.value))
            .collect();
        let summary = if chosen.is_empty() {
            None
        } else if !self.multiple || self.trigger_display == TriggerDisplay::Labels {
            Some(
                chosen
                    .iter()
                    .map(|o| o.label.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        } else {
            Some(format!("{} selected", chosen.len()))
        };
        let list_id = format!("{}:listbox", self.id);
        let option_id = |i: usize| format!("{}:option-{i}", self.id);
        let n = self.options.len();
        let active = s.active.filter(|a| *a < n);
        let toggle = |v: &str, values: &[String], multiple: bool| -> Vec<String> {
            if multiple {
                let mut out = values.to_vec();
                if let Some(p) = out.iter().position(|x| x == v) {
                    out.remove(p);
                } else {
                    out.push(v.to_string());
                }
                out
            } else {
                vec![v.to_string()]
            }
        };
        let on_key = {
            let h = h.clone();
            let opts = self.options.clone();
            let values = self.values.clone();
            let multiple = self.multiple;
            let on_change = self.on_change.clone();
            Rc::new(move |e: &KeyEvent| -> KeyResponse<Msg> {
                if e.phase != InputPhase::Pressed {
                    return KeyResponse::Pass;
                }
                let mut st = h.get();
                if let Some(next) = step_active(&e.key, active, opts.len()) {
                    st.open = true;
                    st.active = next.or(Some(0));
                    h.set(st);
                    return KeyResponse::Handled;
                }
                match e.key {
                    Key::Enter | Key::Space if st.open => {
                        let Some(o) = active.and_then(|a| opts.get(a)) else {
                            return KeyResponse::Handled;
                        };
                        if o.is_disabled {
                            return KeyResponse::Handled;
                        }
                        if !multiple {
                            h.set(SelectState::default());
                        }
                        match &on_change {
                            Some(f) => KeyResponse::Send(f(toggle(&o.value, &values, multiple))),
                            None => KeyResponse::Handled,
                        }
                    }
                    Key::Escape if st.open => {
                        h.set(SelectState::default());
                        KeyResponse::Handled
                    }
                    _ => KeyResponse::Pass,
                }
            })
        };
        // The trigger: a field-like button with the summary and a chevron.
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            gap: taffy::Size {
                width: len(0.0),
                height: len(t.space.s1),
            },
            align_self: Some(AlignItems::FLEX_START),
            ..Style::default()
        }));
        if !self.is_label_hidden {
            let mut l = text_node(
                self.label.clone(),
                type_style(t.font.label, FontFamily::Body),
                color(c.text_secondary),
            );
            l.sem = None;
            cx.leaf(l);
        }
        let r = radii(t.radius.element);
        let (ground, border) = match self.variant {
            SelectorVariant::Input => (
                color(c.background_surface),
                Some((color(c.border_emphasized), t.size.border_width)),
            ),
            SelectorVariant::Secondary => (color(c.background_muted), None),
            SelectorVariant::Ghost => (astryx_core::contract::Color::TRANSPARENT, None),
        };
        let mut sem = Semantics::new(Role::ComboBox, self.label.clone());
        sem.value = Some(summary.clone().unwrap_or_default());
        sem.disabled = self.is_disabled;
        sem.state.expanded = Some(s.open);
        sem.state.has_popup = Some(astryx_core::contract::HasPopup::ListBox);
        sem.refs.controls = vec![list_id.clone()];
        sem.refs.active_descendant = if s.open { active.map(option_id) } else { None };
        let open = s.open;
        let hh = h.clone();
        let mut node = Node::new(Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            gap: taffy::Size {
                width: len(t.space.s2),
                height: len(0.0),
            },
            padding: taffy::Rect {
                left: len(t.space.s3),
                right: len(t.space.s2),
                top: len(0.0),
                bottom: len(0.0),
            },
            border: crate::util::rect(len(if border.is_some() {
                t.size.border_width
            } else {
                0.0
            })),
            min_size: taffy::Size {
                width: LengthPercentageAuto::length(160.0),
                height: len_auto(crate::controls::element_height(t, size)),
            },
            size: taffy::Size {
                width: self
                    .width
                    .map_or(Dimension::auto(), crate::layout::size_dim),
                height: Dimension::auto(),
            },
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw {
            border,
            ..BoxDraw::fill(ground, r)
        }))
        .interact(Interact::new(None, self.is_disabled).effect(move || {
            hh.set(SelectState {
                open: !open,
                active: None,
            })
        }))
        .indicator(astryx_core::authoring::Indicator::ring(
            color(c.accent),
            r,
            Some(color(c.overlay_pressed)),
        ))
        .sem(sem)
        .opacity(if self.is_disabled { 0.5 } else { 1.0 });
        node.on_key = Some(on_key);
        cx.key(Some(&self.id));
        cx.begin(node);
        if self.multiple && self.trigger_display == TriggerDisplay::Badges && !chosen.is_empty() {
            cx.begin(Node::new(Style {
                flex_direction: FlexDirection::Row,
                gap: taffy::Size {
                    width: len(t.space.s1),
                    height: len(0.0),
                },
                flex_grow: 1.0,
                ..Style::default()
            }));
            for o in chosen.iter().take(self.max_badges) {
                cx.child(Token::new(o.label.clone()).size(ComponentSize::Sm).into());
            }
            if chosen.len() > self.max_badges {
                cx.child(
                    Token::new(format!("+{}", chosen.len() - self.max_badges))
                        .size(ComponentSize::Sm)
                        .into(),
                );
            }
            cx.end();
        } else {
            let (words, ink) = match &summary {
                Some(w) => (w.clone(), color(c.text_primary)),
                None => (self.placeholder.clone(), color(c.text_secondary)),
            };
            let mut v = text_node(words, type_style(t.font.body, FontFamily::Body), ink);
            text_draw(&mut v).max_lines = Some(1);
            v.style.flex_grow = 1.0;
            v.style.min_size.width = len_auto(0.0);
            v.sem = None;
            cx.leaf(v);
        }
        if let Some(i) =
            crate::primitives::icon_node::<Msg>("chevronDown", 16.0, color(c.icon_secondary))
        {
            cx.leaf(i);
        }
        cx.end();
        cx.end();
        if s.open {
            let mut rows: Vec<OptionRow<Msg>> = Vec::new();
            for (i, o) in self.options.iter().enumerate() {
                let hh = h.clone();
                let multiple = self.multiple;
                rows.push(OptionRow {
                    id: option_id(i),
                    label: o.label.clone(),
                    description: o.description.clone(),
                    selected: self.values.contains(&o.value),
                    active: active == Some(i),
                    disabled: o.is_disabled,
                    check: self.multiple,
                    msg: self
                        .on_change
                        .as_ref()
                        .map(|f| f(toggle(&o.value, &self.values, self.multiple))),
                    effect: Some(Rc::new(move || {
                        if !multiple {
                            hh.set(SelectState::default());
                        }
                    })),
                });
            }
            let header = (self.multiple && self.has_select_all).then(|| {
                let all: Vec<String> = self
                    .options
                    .iter()
                    .filter(|o| !o.is_disabled)
                    .map(|o| o.value.clone())
                    .collect();
                let every = all.iter().all(|v| self.values.contains(v));
                let mut b = Button::new("Select all")
                    .variant(ButtonVariant::Ghost)
                    .size(ComponentSize::Sm);
                if let Some(f) = &self.on_change {
                    b = b.on_click(f(if every { Vec::new() } else { all }));
                }
                b.into()
            });
            let list = Listbox {
                id: list_id,
                label: self.label.clone(),
                rows,
                empty_text: Some("No options".into()),
                width: None,
                max_height: 320.0,
                multiselectable: self.multiple,
                header,
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
                    .on_dismiss_effect(move |_| hh.set(SelectState::default()))
                    .into(),
            );
        }
    }
}

/// A field of tokens with a typeahead for adding more (`Tokenizer`).
pub struct Tokenizer<T, Msg> {
    id: String,
    label: String,
    source: SearchSource<T>,
    value: Vec<T>,
    on_change: Rc<dyn Fn(Vec<T>) -> Msg>,
    placeholder: Option<String>,
    max_entries: Option<usize>,
    is_disabled: bool,
}

impl<T: SearchItem, Msg> Tokenizer<T, Msg> {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        source: SearchSource<T>,
        value: Vec<T>,
        on_change: impl Fn(Vec<T>) -> Msg + 'static,
    ) -> Self {
        Tokenizer {
            id: id.into(),
            label: label.into(),
            source,
            value,
            on_change: Rc::new(on_change),
            placeholder: None,
            max_entries: None,
            is_disabled: false,
        }
    }
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = Some(p.into());
        self
    }
    pub fn max_entries(mut self, n: usize) -> Self {
        self.max_entries = Some(n);
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
}

impl<T: SearchItem, Msg: Clone + 'static> From<Tokenizer<T, Msg>> for Element<Msg> {
    fn from(t: Tokenizer<T, Msg>) -> Self {
        astryx_core::authoring::element(t)
    }
}

impl<T: SearchItem, Msg: Clone + 'static> Build<Msg> for Tokenizer<T, Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let full = self.max_entries.is_some_and(|m| self.value.len() >= m);
        let taken: Vec<String> = self.value.iter().map(|v| v.key()).collect();
        let base = self.source.clone();
        let source: SearchSource<T> = Rc::new(move |q| {
            base(q)
                .into_iter()
                .filter(|i| !taken.contains(&i.key()))
                .collect()
        });
        let value = self.value.clone();
        let on = self.on_change.clone();
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            gap: taffy::Size {
                width: len(0.0),
                height: len(t.space.s1),
            },
            ..Style::default()
        }));
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Row,
            flex_wrap: taffy::FlexWrap::Wrap,
            gap: taffy::Size {
                width: len(t.space.s1),
                height: len(t.space.s1),
            },
            ..Style::default()
        }));
        for (i, v) in self.value.iter().enumerate() {
            let mut rest = self.value.clone();
            rest.remove(i);
            let remove_label = format!("Remove {}", v.label());
            cx.begin(Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                ..Style::default()
            }));
            cx.child(Token::new(v.label()).into());
            cx.child(
                Button::new(remove_label)
                    .variant(ButtonVariant::Ghost)
                    .size(ComponentSize::Sm)
                    .icon(Icon::new("close").size(IconSize::Xsm))
                    .is_icon_only(true)
                    .is_disabled(self.is_disabled)
                    .on_click((self.on_change)(rest))
                    .into(),
            );
            cx.end();
        }
        cx.end();
        let mut ta = Typeahead::new(self.id.clone(), self.label.clone(), source, None)
            .is_disabled(self.is_disabled || full)
            .has_clear(false)
            .on_change(move |item: Option<T>| {
                let mut next = value.clone();
                if let Some(i) = item {
                    next.push(i);
                }
                on(next)
            });
        if let Some(p) = &self.placeholder {
            ta = ta.placeholder(p.clone());
        }
        cx.child(ta.into());
        cx.end();
    }
}

/// A search dialog over commands and places (`CommandPalette`): type,
/// move with arrows, Enter runs the active result.
pub struct CommandPalette<T, Msg> {
    id: String,
    is_open: bool,
    on_open_change: Rc<dyn Fn(bool) -> Msg>,
    source: SearchSource<T>,
    on_value_change: Option<Rc<dyn Fn(T) -> Msg>>,
    label: String,
    width: f32,
    max_height: f32,
    is_inline: bool,
    empty_search_text: String,
    empty_bootstrap_text: String,
}

impl<T: SearchItem, Msg> CommandPalette<T, Msg> {
    pub fn new(
        id: impl Into<String>,
        is_open: bool,
        on_open_change: impl Fn(bool) -> Msg + 'static,
        source: SearchSource<T>,
    ) -> Self {
        CommandPalette {
            id: id.into(),
            is_open,
            on_open_change: Rc::new(on_open_change),
            source,
            on_value_change: None,
            label: "Command palette".into(),
            width: 640.0,
            max_height: 480.0,
            is_inline: false,
            empty_search_text: "No results".into(),
            empty_bootstrap_text: "Type to search".into(),
        }
    }
    pub fn on_value_change(mut self, f: impl Fn(T) -> Msg + 'static) -> Self {
        self.on_value_change = Some(Rc::new(f));
        self
    }
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = l.into();
        self
    }
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    pub fn max_height(mut self, h: f32) -> Self {
        self.max_height = h;
        self
    }
    pub fn is_inline(mut self, on: bool) -> Self {
        self.is_inline = on;
        self
    }
}

impl<T: SearchItem, Msg: Clone + 'static> From<CommandPalette<T, Msg>> for Element<Msg> {
    fn from(t: CommandPalette<T, Msg>) -> Self {
        astryx_core::authoring::element(t)
    }
}

impl<T: SearchItem, Msg: Clone + 'static> Build<Msg> for CommandPalette<T, Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if !self.is_open && !self.is_inline {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        cx.key(Some(&format!("{}:palette", self.id)));
        let h: LocalHandle<ComboState> = cx.local_handle();
        let s = h.get();
        let query = s.query.clone().unwrap_or_default();
        let results: Vec<T> = if query.is_empty() {
            Vec::new()
        } else {
            (self.source)(&query)
        };
        let active = s
            .active
            .filter(|a| *a < results.len())
            .or((!results.is_empty()).then_some(0));
        let list_id = format!("{}:results", self.id);
        let option_id = |i: usize| format!("{}:result-{i}", self.id);
        let on_key = {
            let h = h.clone();
            let results = results.clone();
            let pick = self.on_value_change.clone();
            Rc::new(move |e: &KeyEvent| -> KeyResponse<Msg> {
                if e.phase != InputPhase::Pressed {
                    return KeyResponse::Pass;
                }
                let mut st = h.get();
                if let Some(next) = step_active(&e.key, active, results.len()) {
                    st.active = next;
                    h.set(st);
                    return KeyResponse::Handled;
                }
                if e.key == Key::Enter {
                    if let (Some(item), Some(f)) =
                        (active.and_then(|a| results.get(a)).cloned(), &pick)
                    {
                        h.set(ComboState::default());
                        return KeyResponse::Send(f(item));
                    }
                }
                KeyResponse::Pass
            })
        };
        let edit = {
            let h = h.clone();
            Rc::new(move |text: String| {
                h.set(ComboState {
                    query: Some(text),
                    open: true,
                    active: None,
                })
            }) as Rc<dyn Fn(String)>
        };
        let input = TextInput::new(self.label.clone(), query.clone())
            .is_label_hidden(true)
            .start_icon(Icon::new("search"))
            .placeholder("Search…")
            .has_auto_focus(true)
            .on_edit(edit)
            .combo(ComboHooks {
                expanded: !results.is_empty(),
                active_descendant: active.map(option_id),
                controls: Some(list_id.clone()),
                on_key: Some(on_key),
                has_popup_listbox: true,
            });
        let rows: Vec<OptionRow<Msg>> = results
            .iter()
            .enumerate()
            .map(|(i, it)| OptionRow {
                id: option_id(i),
                label: it.label(),
                description: it.description(),
                selected: false,
                active: active == Some(i),
                disabled: false,
                check: false,
                msg: self.on_value_change.as_ref().map(|f| f(it.clone())),
                effect: None,
            })
            .collect();
        let empty = if query.is_empty() {
            self.empty_bootstrap_text.clone()
        } else {
            self.empty_search_text.clone()
        };
        let body = astryx_core::authoring::element(PaletteBody {
            width: self.width,
            max_height: self.max_height,
            input: input.into(),
            list: astryx_core::authoring::element(Listbox {
                id: list_id,
                label: self.label.clone(),
                rows,
                empty_text: Some(empty),
                width: None,
                max_height: self.max_height,
                multiselectable: false,
                header: None,
                bare: true,
            }),
        });
        if self.is_inline {
            cx.begin(
                Node::new(Style::default()).sem(Semantics::new(Role::Dialog, self.label.clone())),
            );
            cx.child(body);
            cx.end();
            return;
        }
        let on = self.on_open_change.clone();
        let hh = h.clone();
        cx.child(
            Layer::dialog(body)
                .backdrop(color(c.overlay))
                .label(self.label.clone())
                .on_dismiss_effect(move |_| hh.set(ComboState::default()))
                .on_dismiss(move |_| on(false))
                .into(),
        );
        let _ = t;
    }
}

struct PaletteBody<Msg> {
    width: f32,
    max_height: f32,
    input: Element<Msg>,
    list: Element<Msg>,
}

impl<Msg: Clone + 'static> Build<Msg> for PaletteBody<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let c = &t.color;
        let (vw, _) = cx.viewport;
        let w = if vw > 0.0 {
            self.width.min(vw - 2.0 * t.space.s4)
        } else {
            self.width
        };
        let r = radii(t.radius.container);
        let mut n = Node::new(Style {
            flex_direction: FlexDirection::Column,
            gap: taffy::Size {
                width: len(0.0),
                height: len(t.space.s2),
            },
            padding: crate::util::rect(len(t.space.s2)),
            size: taffy::Size {
                width: Dimension::length(w),
                height: Dimension::auto(),
            },
            max_size: taffy::Size {
                width: LengthPercentageAuto::auto(),
                height: LengthPercentageAuto::length(self.max_height),
            },
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw {
            shadow: t.shadow.high,
            ..BoxDraw::fill(color(c.background_surface), r)
        }));
        n.clip = Some(r);
        cx.begin(n);
        cx.child(self.input);
        cx.child(self.list);
        cx.end();
    }
}
