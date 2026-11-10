//! NumberInput and the date and time inputs (DateInput, DateRangeInput,
//! TimeInput, DateTimeInput).

use super::calendar::{spoken_date, Calendar, CalendarValue, CivilDate, DateRange};
use super::search::{Selector, SelectorOption};
use crate::controls::{Button, ButtonVariant, ComboHooks, TextInput};
use crate::primitives::Icon;
use crate::util::{color, len, len_auto, radii};
use astryx_core::authoring::{
    text_draw, text_node, type_style, Base, BoxDraw, Build, BuildCx, Interact, KeyResponse,
    LocalHandle, Node, Semantics,
};
use astryx_core::contract::{FontFamily, Role};
use astryx_core::{Element, InputPhase, Key, KeyEvent};
use astryx_spike_foundations::ComponentSize;
use astryx_spike_layers::placement::{Align, Placement, Side};
use astryx_spike_layers::Layer;
use std::rc::Rc;
use taffy::{AlignItems, Dimension, FlexDirection, LengthPercentageAuto, Style};

#[derive(Clone, Debug, Default, PartialEq)]
struct Draft {
    text: Option<String>,
}

/// A number field (`NumberInput`): typing keeps a draft; Enter, the arrow
/// keys and the steppers commit a value within min and max.
pub struct NumberInput<Msg> {
    base: Base,
    id: String,
    label: String,
    value: Option<f64>,
    on_change: Option<Rc<dyn Fn(f64) -> Msg>>,
    min: Option<f64>,
    max: Option<f64>,
    step: f64,
    units: Option<String>,
    is_integer_only: bool,
    has_number_steppers: bool,
    is_disabled: bool,
    is_read_only: bool,
    is_label_hidden: bool,
    placeholder: Option<String>,
    size: Option<ComponentSize>,
    width: Option<crate::layout::SizeValue>,
    format: Option<Rc<dyn Fn(f64) -> String>>,
}

impl<Msg> NumberInput<Msg> {
    pub fn new(id: impl Into<String>, label: impl Into<String>, value: Option<f64>) -> Self {
        NumberInput {
            base: Base::default(),
            id: id.into(),
            label: label.into(),
            value,
            on_change: None,
            min: None,
            max: None,
            step: 1.0,
            units: None,
            is_integer_only: false,
            has_number_steppers: false,
            is_disabled: false,
            is_read_only: false,
            is_label_hidden: false,
            placeholder: None,
            size: None,
            width: None,
            format: None,
        }
    }
    pub fn on_change(mut self, f: impl Fn(f64) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
    pub fn min(mut self, v: f64) -> Self {
        self.min = Some(v);
        self
    }
    pub fn max(mut self, v: f64) -> Self {
        self.max = Some(v);
        self
    }
    pub fn step(mut self, s: f64) -> Self {
        self.step = s;
        self
    }
    pub fn units(mut self, u: impl Into<String>) -> Self {
        self.units = Some(u.into());
        self
    }
    pub fn is_integer_only(mut self, on: bool) -> Self {
        self.is_integer_only = on;
        self
    }
    pub fn has_number_steppers(mut self, on: bool) -> Self {
        self.has_number_steppers = on;
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
    pub fn is_label_hidden(mut self, on: bool) -> Self {
        self.is_label_hidden = on;
        self
    }
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = Some(p.into());
        self
    }
    pub fn size(mut self, s: ComponentSize) -> Self {
        self.size = Some(s);
        self
    }
    pub fn width(mut self, w: crate::layout::SizeValue) -> Self {
        self.width = Some(w);
        self
    }
    pub fn format_value(mut self, f: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Some(Rc::new(f));
        self
    }
}

crate::util::component!(NumberInput<Msg>);

/// The value `text` asks for, within the limits; `None` when it is not a
/// number.
pub fn parse_number(text: &str, min: Option<f64>, max: Option<f64>, integer: bool) -> Option<f64> {
    let cleaned: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ',')
        .collect();
    let mut v: f64 = cleaned.parse().ok()?;
    if !v.is_finite() {
        return None;
    }
    if integer {
        v = v.round();
    }
    if let Some(m) = min {
        v = v.max(m);
    }
    if let Some(m) = max {
        v = v.min(m);
    }
    Some(v)
}

fn plain(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

impl<Msg: Clone + 'static> Build<Msg> for NumberInput<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        cx.key(Some(&format!("{}:number", self.id)));
        let h: LocalHandle<Draft> = cx.local_handle();
        let draft = h.get().text;
        let shown = draft.clone().unwrap_or_else(|| match self.value {
            Some(v) => self.format.as_ref().map_or_else(|| plain(v), |f| f(v)),
            None => String::new(),
        });
        let (min, max, int, step) = (self.min, self.max, self.is_integer_only, self.step);
        let current = draft
            .as_deref()
            .and_then(|d| parse_number(d, min, max, int))
            .or(self.value);
        let stepped = move |dir: f64| {
            let base = current.unwrap_or(min.unwrap_or(0.0));
            let mut v = base + dir * step;
            if let Some(m) = min {
                v = v.max(m);
            }
            if let Some(m) = max {
                v = v.min(m);
            }
            v
        };
        let editable = !self.is_disabled && !self.is_read_only;
        let on_key = {
            let h = h.clone();
            let on_change = self.on_change.clone();
            Rc::new(move |e: &KeyEvent| -> KeyResponse<Msg> {
                if e.phase != InputPhase::Pressed || !editable {
                    return KeyResponse::Pass;
                }
                let value = match e.key {
                    Key::Up => Some(stepped(1.0)),
                    Key::Down => Some(stepped(-1.0)),
                    Key::Enter => current,
                    _ => return KeyResponse::Pass,
                };
                h.set(Draft::default());
                match (value, &on_change) {
                    (Some(v), Some(f)) => KeyResponse::Send(f(v)),
                    _ => KeyResponse::Handled,
                }
            })
        };
        let edit = {
            let h = h.clone();
            Rc::new(move |text: String| h.set(Draft { text: Some(text) })) as Rc<dyn Fn(String)>
        };
        let mut input = TextInput::new(self.label.clone(), shown)
            .is_label_hidden(self.is_label_hidden)
            .is_disabled(self.is_disabled)
            .is_read_only(self.is_read_only)
            .id(self.id.clone())
            .on_edit(edit)
            .combo(ComboHooks {
                expanded: false,
                active_descendant: None,
                controls: None,
                on_key: Some(on_key),
                has_popup_listbox: false,
            })
            .role(Role::SpinButton);
        if let Some(p) = &self.placeholder {
            input = input.placeholder(p.clone());
        }
        if let Some(s) = self.size {
            input = input.size(s);
        }
        if let Some(w) = self.width {
            input = input.width(w);
        }
        let mut end: Vec<Element<Msg>> = Vec::new();
        if let Some(u) = &self.units {
            end.push(
                crate::primitives::Text::new(u.clone())
                    .color(crate::primitives::TextColor::Secondary)
                    .into(),
            );
        }
        if self.has_number_steppers {
            for (glyph, words, dir) in [("minus", "Decrease", -1.0), ("plus", "Increase", 1.0)] {
                let hh = h.clone();
                let mut b = Button::new(words)
                    .variant(ButtonVariant::Ghost)
                    .size(ComponentSize::Sm)
                    .icon(Icon::new(glyph))
                    .is_icon_only(true)
                    .is_disabled(!editable)
                    .effect(Rc::new(move || hh.set(Draft::default())));
                if let Some(f) = &self.on_change {
                    b = b.on_click(f(stepped(dir)));
                }
                end.push(b.into());
            }
        }
        cx.child(input.end_content(end).into());
    }
}

/// How DateInput shows its date (`format`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DateFormat {
    /// `Nov 9, 2026`.
    Date,
    /// `November 9, 2026`.
    #[default]
    DateLong,
    /// `Monday, November 9, 2026`.
    DateWeekday,
    /// `2026-11-09`.
    SystemDate,
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

pub fn format_date(d: CivilDate, f: DateFormat) -> String {
    let month = MONTHS[d.month as usize - 1];
    match f {
        DateFormat::Date => format!("{} {}, {}", &month[..3], d.day, d.year),
        DateFormat::DateLong => format!("{month} {}, {}", d.day, d.year),
        DateFormat::DateWeekday => spoken_date(d),
        DateFormat::SystemDate => d.iso(),
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct PickerState {
    open: bool,
}

/// The field-like button that opens a date picker.
struct Trigger<'a, Msg> {
    id: &'a str,
    label: &'a str,
    shown: Option<String>,
    placeholder: &'a str,
    open: bool,
    disabled: bool,
    toggle: Rc<dyn Fn()>,
    clear: Option<Msg>,
    icon: &'a str,
}

fn picker_trigger<Msg: Clone + 'static>(cx: &mut BuildCx<Msg>, tr: Trigger<Msg>) {
    let Trigger {
        id,
        label,
        shown,
        placeholder,
        open,
        disabled,
        toggle,
        clear,
        icon,
    } = tr;
    let t = cx.tokens;
    let c = &t.color;
    cx.begin(Node::new(Style {
        flex_direction: FlexDirection::Column,
        gap: taffy::Size {
            width: len(0.0),
            height: len(t.space.s1),
        },
        align_self: Some(AlignItems::FLEX_START),
        ..Style::default()
    }));
    let mut l = text_node(
        label.to_string(),
        type_style(t.font.label, FontFamily::Body),
        color(c.text_secondary),
    );
    l.sem = None;
    cx.leaf(l);
    let r = radii(t.radius.element);
    let mut sem = Semantics::new(Role::Button, label.to_string());
    sem.value = shown.clone();
    sem.disabled = disabled;
    sem.state.expanded = Some(open);
    sem.state.has_popup = Some(astryx_core::contract::HasPopup::Dialog);
    cx.key(Some(id));
    cx.begin(
        Node::new(Style {
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
            border: crate::util::rect(len(t.size.border_width)),
            min_size: taffy::Size {
                width: LengthPercentageAuto::length(200.0),
                height: len_auto(t.size.element_md),
            },
            ..Style::default()
        })
        .draw(astryx_core::authoring::Draw::Box(BoxDraw {
            border: Some((color(c.border_emphasized), t.size.border_width)),
            ..BoxDraw::fill(color(c.background_surface), r)
        }))
        .interact(Interact::new(None, disabled).effect(move || toggle()))
        .indicator(astryx_core::authoring::Indicator::ring(
            color(c.accent),
            r,
            Some(color(c.overlay_pressed)),
        ))
        .sem(sem)
        .opacity(if disabled { 0.5 } else { 1.0 }),
    );
    if let Some(n) = crate::primitives::icon_node::<Msg>(icon, 16.0, color(c.icon_secondary)) {
        cx.leaf(n);
    }
    let (words, ink) = match shown {
        Some(s) => (s, color(c.text_primary)),
        None => (placeholder.to_string(), color(c.text_secondary)),
    };
    let mut v = text_node(words, type_style(t.font.body, FontFamily::Body), ink);
    text_draw(&mut v).max_lines = Some(1);
    v.style.flex_grow = 1.0;
    v.sem = None;
    cx.leaf(v);
    cx.end();
    if let Some(m) = clear {
        cx.child(
            Button::new(format!("Clear {label}"))
                .variant(ButtonVariant::Ghost)
                .size(ComponentSize::Sm)
                .icon(Icon::new("close"))
                .is_icon_only(true)
                .on_click(m)
                .into(),
        );
    }
    cx.end();
}

/// A date field that opens a calendar (`DateInput`), or a range field
/// (`DateRangeInput`) with two months.
pub struct DateInput<Msg> {
    base: Base,
    id: String,
    label: String,
    value: CalendarValue,
    on_change: Option<Rc<dyn Fn(CalendarValue) -> Msg>>,
    min: Option<CivilDate>,
    max: Option<CivilDate>,
    placeholder: String,
    format: DateFormat,
    number_of_months: u32,
    week_starts_on: u32,
    is_disabled: bool,
    has_clear: bool,
    today: Option<CivilDate>,
}

impl<Msg> DateInput<Msg> {
    /// One date (`DateInput`).
    pub fn single(
        id: impl Into<String>,
        label: impl Into<String>,
        value: Option<CivilDate>,
    ) -> Self {
        Self::make(id, label, CalendarValue::Single(value), "Select a date", 1)
    }
    /// A start and end (`DateRangeInput`).
    pub fn range(
        id: impl Into<String>,
        label: impl Into<String>,
        value: Option<DateRange>,
    ) -> Self {
        Self::make(
            id,
            label,
            CalendarValue::Range(value),
            "Select date range",
            2,
        )
    }
    fn make(
        id: impl Into<String>,
        label: impl Into<String>,
        value: CalendarValue,
        placeholder: &str,
        months: u32,
    ) -> Self {
        DateInput {
            base: Base::default(),
            id: id.into(),
            label: label.into(),
            value,
            on_change: None,
            min: None,
            max: None,
            placeholder: placeholder.into(),
            format: DateFormat::DateLong,
            number_of_months: months,
            week_starts_on: 0,
            is_disabled: false,
            has_clear: matches!(value, CalendarValue::Range(_)),
            today: None,
        }
    }
    pub fn on_change(mut self, f: impl Fn(CalendarValue) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
    pub fn min(mut self, d: CivilDate) -> Self {
        self.min = Some(d);
        self
    }
    pub fn max(mut self, d: CivilDate) -> Self {
        self.max = Some(d);
        self
    }
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }
    pub fn format(mut self, f: DateFormat) -> Self {
        self.format = f;
        self
    }
    pub fn number_of_months(mut self, n: u32) -> Self {
        self.number_of_months = n.clamp(1, 2);
        self
    }
    pub fn week_starts_on(mut self, d: u32) -> Self {
        self.week_starts_on = d % 7;
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    pub fn has_clear(mut self, on: bool) -> Self {
        self.has_clear = on;
        self
    }
    pub fn today(mut self, d: CivilDate) -> Self {
        self.today = Some(d);
        self
    }
}

crate::util::component!(DateInput<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for DateInput<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        cx.key(Some(&format!("{}:picker", self.id)));
        let h: LocalHandle<PickerState> = cx.local_handle();
        let open = h.get().open;
        let shown = match self.value {
            CalendarValue::Single(d) => d.map(|d| format_date(d, self.format)),
            CalendarValue::Range(r) => r.map(|r| match r.end {
                Some(e) => format!(
                    "{} – {}",
                    format_date(r.start, DateFormat::Date),
                    format_date(e, DateFormat::Date)
                ),
                None => format_date(r.start, DateFormat::Date),
            }),
        };
        let empty = match self.value {
            CalendarValue::Single(_) => CalendarValue::Single(None),
            CalendarValue::Range(_) => CalendarValue::Range(None),
        };
        let clear = (self.has_clear && shown.is_some())
            .then(|| self.on_change.as_ref().map(|f| f(empty)))
            .flatten();
        let toggle = {
            let h = h.clone();
            Rc::new(move || h.set(PickerState { open: !open })) as Rc<dyn Fn()>
        };
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            ..Style::default()
        }));
        picker_trigger(
            cx,
            Trigger {
                id: &self.id,
                label: &self.label,
                shown,
                placeholder: &self.placeholder,
                open,
                disabled: self.is_disabled,
                toggle,
                clear,
                icon: "calendar",
            },
        );
        if open {
            let hh = h.clone();
            let on = self.on_change.clone();
            let mut cal = Calendar::new(format!("{}:calendar", self.id), self.value)
                .number_of_months(self.number_of_months)
                .week_starts_on(self.week_starts_on);
            if let Some(d) = self.min {
                cal = cal.min(d);
            }
            if let Some(d) = self.max {
                cal = cal.max(d);
            }
            if let Some(d) = self.today {
                cal = cal.today(d);
            }
            if let Some(f) = on {
                cal = cal.on_change(move |v| f(v));
            }
            let close_after = match self.value {
                CalendarValue::Single(_) => true,
                CalendarValue::Range(r) => matches!(r, Some(DateRange { end: None, .. })),
            };
            let _ = close_after;
            let surface = astryx_core::authoring::element(PickerSurface { child: cal.into() });
            cx.child(
                Layer::popover(&self.id, surface)
                    .placement(Placement::new(Side::Below, Align::Start).offset(t.space.s1))
                    .label(self.label.clone())
                    .on_dismiss_effect(move |_| hh.set(PickerState { open: false }))
                    .into(),
            );
        }
        cx.end();
    }
}

struct PickerSurface<Msg> {
    child: Element<Msg>,
}

impl<Msg: 'static> Build<Msg> for PickerSurface<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let c = &t.color;
        let r = radii(t.radius.container);
        cx.begin(
            Node::new(Style::default()).draw(astryx_core::authoring::Draw::Box(BoxDraw {
                shadow: t.shadow.med,
                border: Some((color(c.border), t.size.border_width)),
                ..BoxDraw::fill(color(c.background_popover), r)
            })),
        );
        cx.child(self.child);
        cx.end();
    }
}

/// A clock time (`ISOTimeString`), minutes since midnight.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClockTime {
    pub minutes: u32,
}

impl ClockTime {
    pub fn new(h: u32, m: u32) -> Self {
        ClockTime {
            minutes: (h % 24) * 60 + m % 60,
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        let mut it = s.split(':');
        let h: u32 = it.next()?.trim().parse().ok()?;
        let m: u32 = it.next()?.trim().parse().ok()?;
        (h < 24 && m < 60).then(|| ClockTime::new(h, m))
    }
    pub fn iso(&self) -> String {
        format!("{:02}:{:02}", self.minutes / 60, self.minutes % 60)
    }
    pub fn label(&self, twelve: bool) -> String {
        let (h, m) = (self.minutes / 60, self.minutes % 60);
        if twelve {
            let h12 = if h % 12 == 0 { 12 } else { h % 12 };
            format!("{h12}:{m:02} {}", if h < 12 { "AM" } else { "PM" })
        } else {
            self.iso()
        }
    }
}

/// A time field with a list of times (`TimeInput`). The list steps by the
/// larger of `increment` and 15 minutes.
pub struct TimeInput<Msg> {
    id: String,
    label: String,
    value: Option<ClockTime>,
    on_change: Option<Rc<dyn Fn(Option<ClockTime>) -> Msg>>,
    increment: u32,
    twelve_hour: bool,
    min: Option<ClockTime>,
    max: Option<ClockTime>,
    is_disabled: bool,
    placeholder: String,
}

impl<Msg> TimeInput<Msg> {
    pub fn new(id: impl Into<String>, label: impl Into<String>, value: Option<ClockTime>) -> Self {
        TimeInput {
            id: id.into(),
            label: label.into(),
            value,
            on_change: None,
            increment: 1,
            twelve_hour: true,
            min: None,
            max: None,
            is_disabled: false,
            placeholder: "Select a time".into(),
        }
    }
    pub fn on_change(mut self, f: impl Fn(Option<ClockTime>) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
    pub fn increment(mut self, m: u32) -> Self {
        self.increment = m.max(1);
        self
    }
    /// `hourFormat`: true for 12-hour.
    pub fn twelve_hour(mut self, on: bool) -> Self {
        self.twelve_hour = on;
        self
    }
    pub fn min(mut self, t: ClockTime) -> Self {
        self.min = Some(t);
        self
    }
    pub fn max(mut self, t: ClockTime) -> Self {
        self.max = Some(t);
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
}

impl<Msg: Clone + 'static> From<TimeInput<Msg>> for Element<Msg> {
    fn from(t: TimeInput<Msg>) -> Self {
        let step = t.increment.max(15);
        let options: Vec<SelectorOption> = (0..24 * 60)
            .step_by(step as usize)
            .map(|m| ClockTime { minutes: m })
            .filter(|c| t.min.is_none_or(|lo| *c >= lo) && t.max.is_none_or(|hi| *c <= hi))
            .map(|c| SelectorOption::new(c.iso(), c.label(t.twelve_hour)))
            .collect();
        let mut s = Selector::single(t.id, t.label, options, t.value.map(|v| v.iso()))
            .placeholder(t.placeholder)
            .is_disabled(t.is_disabled);
        if let Some(f) = t.on_change {
            s = s.on_change(move |v: Vec<String>| f(v.first().and_then(|x| ClockTime::parse(x))));
        }
        s.into()
    }
}

/// Gets a new date and time.
pub type DateTimeChange<Msg> = Rc<dyn Fn(Option<CivilDate>, Option<ClockTime>) -> Msg>;

/// A date and a time side by side (`DateTimeInput`).
pub struct DateTimeInput<Msg> {
    id: String,
    label: String,
    date: Option<CivilDate>,
    time: Option<ClockTime>,
    on_change: Option<DateTimeChange<Msg>>,
    time_label: String,
}

impl<Msg> DateTimeInput<Msg> {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        date: Option<CivilDate>,
        time: Option<ClockTime>,
    ) -> Self {
        DateTimeInput {
            id: id.into(),
            label: label.into(),
            date,
            time,
            on_change: None,
            time_label: "Time".into(),
        }
    }
    pub fn on_change(
        mut self,
        f: impl Fn(Option<CivilDate>, Option<ClockTime>) -> Msg + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
    pub fn time_label(mut self, l: impl Into<String>) -> Self {
        self.time_label = l.into();
        self
    }
}

impl<Msg: Clone + 'static> From<DateTimeInput<Msg>> for Element<Msg> {
    fn from(d: DateTimeInput<Msg>) -> Self {
        let mut date = DateInput::single(format!("{}:date", d.id), d.label, d.date);
        let mut time = TimeInput::new(format!("{}:time", d.id), d.time_label, d.time);
        if let Some(f) = d.on_change {
            let (t0, d0) = (d.time, d.date);
            let f2 = f.clone();
            date = date.on_change(move |v| match v {
                CalendarValue::Single(x) => f(x, t0),
                CalendarValue::Range(_) => f(None, t0),
            });
            time = time.on_change(move |x| f2(d0, x));
        }
        crate::layout::HStack::new()
            .gap(astryx_core::Space::S2)
            .v_align(crate::layout::StackAxisAlign::End)
            .child(date)
            .child(time)
            .into()
    }
}

#[allow(dead_code)]
fn _unused() -> Dimension {
    Dimension::auto()
}
