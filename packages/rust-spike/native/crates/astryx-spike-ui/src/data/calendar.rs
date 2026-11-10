//! Calendar: a month grid for picking a date or a range (`Calendar`).
//!
//! Gregorian only. The core has no clock that works on every target, so
//! the caller passes `today` when it wants today marked.

use crate::controls::{Button, ButtonVariant};
use crate::primitives::Icon;
use crate::util::{color, len, radii};
use astryx_core::authoring::{
    text_node, type_style, Base, BoxDraw, Build, BuildCx, Draw, Indicator, Interact, KeyResponse,
    Node, Semantics,
};
use astryx_core::contract::{FontFamily, Role};
use astryx_core::{Element, InputPhase, Key, KeyEvent};
use astryx_spike_foundations::{args, ComponentSize, CxExt};
use std::rc::Rc;
use taffy::{AlignItems, Dimension, FlexDirection, JustifyContent, Style};

/// A calendar date (`ISODateString`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CivilDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl CivilDate {
    pub fn new(year: i32, month: u32, day: u32) -> Self {
        CivilDate { year, month, day }
    }
    /// Reads `YYYY-MM-DD`.
    pub fn parse(s: &str) -> Option<Self> {
        let mut it = s.trim().splitn(3, '-');
        let y = it.next()?.parse().ok()?;
        let m: u32 = it.next()?.parse().ok()?;
        let d: u32 = it.next()?.parse().ok()?;
        (1..=12).contains(&m).then_some(())?;
        (1..=days_in_month(y, m)).contains(&d).then_some(())?;
        Some(CivilDate::new(y, m, d))
    }
    pub fn iso(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
    /// Days since 1970-01-01.
    pub fn days(&self) -> i64 {
        let y = i64::from(self.year) - i64::from(self.month <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = i64::from(self.month);
        let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }
    pub fn from_days(z: i64) -> Self {
        let z = z + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let y = (yoe + era * 400 + i64::from(m <= 2)) as i32;
        CivilDate::new(y, m, d)
    }
    pub fn add_days(&self, n: i64) -> Self {
        Self::from_days(self.days() + n)
    }
    /// 0 is Sunday.
    pub fn weekday(&self) -> u32 {
        (self.days() + 4).rem_euclid(7) as u32
    }
    /// The same day `n` months away, clamped to the month's length.
    pub fn add_months(&self, n: i32) -> Self {
        let total = self.year * 12 + self.month as i32 - 1 + n;
        let (y, m) = (total.div_euclid(12), total.rem_euclid(12) as u32 + 1);
        CivilDate::new(y, m, self.day.min(days_in_month(y, m)))
    }
    pub fn first_of_month(&self) -> Self {
        CivilDate::new(self.year, self.month, 1)
    }
    /// The ISO 8601 week number.
    pub fn iso_week(&self) -> u32 {
        let wd = (self.weekday() + 6) % 7; // Monday 0
        let thursday = self.add_days(3 - i64::from(wd));
        let jan1 = CivilDate::new(thursday.year, 1, 1);
        ((thursday.days() - jan1.days()) / 7 + 1) as u32
    }
}

pub fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
    }
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
const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// A date in words, as assistive technology reads it.
pub fn spoken_date(d: CivilDate) -> String {
    format!(
        "{}, {} {}, {}",
        WEEKDAYS[d.weekday() as usize],
        MONTHS[d.month as usize - 1],
        d.day,
        d.year
    )
}

/// A selected range (`DateRange`); `end` is empty while picking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateRange {
    pub start: CivilDate,
    pub end: Option<CivilDate>,
}

/// Calendar `value`: one date, or a range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalendarValue {
    Single(Option<CivilDate>),
    Range(Option<DateRange>),
}

type DateFn = Rc<dyn Fn(CivilDate) -> bool>;

#[derive(Clone, Debug, Default, PartialEq)]
struct CalendarState {
    /// The day with keyboard focus; sets the shown month.
    focus: Option<CivilDate>,
}

/// A month grid (`Calendar`). Arrows move a day or a week, Page Up/Down a
/// month, Home/End to the week's ends; Enter or Space picks.
pub struct Calendar<Msg> {
    base: Base,
    id: String,
    value: CalendarValue,
    on_change: Option<Rc<dyn Fn(CalendarValue) -> Msg>>,
    number_of_months: u32,
    min: Option<CivilDate>,
    max: Option<CivilDate>,
    constraints: Vec<DateFn>,
    has_outside_days: bool,
    has_week_numbers: bool,
    has_variable_row_count: bool,
    week_starts_on: u32,
    today: Option<CivilDate>,
    focus_date: Option<CivilDate>,
}

impl<Msg> Calendar<Msg> {
    pub fn new(id: impl Into<String>, value: CalendarValue) -> Self {
        Calendar {
            base: Base::default(),
            id: id.into(),
            value,
            on_change: None,
            number_of_months: 1,
            min: None,
            max: None,
            constraints: Vec::new(),
            has_outside_days: true,
            has_week_numbers: false,
            has_variable_row_count: false,
            week_starts_on: 0,
            today: None,
            focus_date: None,
        }
    }
    pub fn on_change(mut self, f: impl Fn(CalendarValue) -> Msg + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
    pub fn number_of_months(mut self, n: u32) -> Self {
        self.number_of_months = n.clamp(1, 2);
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
    /// A rule that disables the days it returns true for.
    pub fn date_constraint(mut self, f: impl Fn(CivilDate) -> bool + 'static) -> Self {
        self.constraints.push(Rc::new(f));
        self
    }
    pub fn has_outside_days(mut self, on: bool) -> Self {
        self.has_outside_days = on;
        self
    }
    pub fn has_week_numbers(mut self, on: bool) -> Self {
        self.has_week_numbers = on;
        self
    }
    pub fn has_variable_row_count(mut self, on: bool) -> Self {
        self.has_variable_row_count = on;
        self
    }
    /// 0 is Sunday.
    pub fn week_starts_on(mut self, d: u32) -> Self {
        self.week_starts_on = d % 7;
        self
    }
    pub fn today(mut self, d: CivilDate) -> Self {
        self.today = Some(d);
        self
    }
    /// The month to show first, when nothing is selected.
    pub fn focus_date(mut self, d: CivilDate) -> Self {
        self.focus_date = Some(d);
        self
    }
}

crate::util::component!(Calendar<Msg>);

impl<Msg: Clone + 'static> Calendar<Msg> {
    fn disabled(&self, d: CivilDate) -> bool {
        self.min.is_some_and(|m| d < m)
            || self.max.is_some_and(|m| d > m)
            || self.constraints.iter().any(|f| f(d))
    }

    fn pick(&self, d: CivilDate) -> CalendarValue {
        match self.value {
            CalendarValue::Single(_) => CalendarValue::Single(Some(d)),
            CalendarValue::Range(r) => CalendarValue::Range(Some(match r {
                Some(DateRange { start, end: None }) if d >= start => DateRange {
                    start,
                    end: Some(d),
                },
                _ => DateRange {
                    start: d,
                    end: None,
                },
            })),
        }
    }
}

fn day_id(cal: &str, d: CivilDate) -> String {
    format!("{cal}:{}", d.iso())
}

impl<Msg: Clone + 'static> Build<Msg> for Calendar<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        cx.key(Some(&self.id));
        let h = cx.local_handle::<CalendarState>();
        let selected_anchor = match self.value {
            CalendarValue::Single(d) => d,
            CalendarValue::Range(r) => r.map(|r| r.start),
        };
        let focus = h
            .get()
            .focus
            .or(selected_anchor)
            .or(self.focus_date)
            .or(self.today)
            .unwrap_or(CivilDate::new(2026, 1, 1));
        let first_month = focus.first_of_month();
        // Keys move the focused day; the shown month follows.
        let requests = cx.focus_requests();
        let id = self.id.clone();
        let wso = self.week_starts_on;
        let on_key = {
            let h = h.clone();
            Rc::new(move |e: &KeyEvent| -> KeyResponse<Msg> {
                if e.phase != InputPhase::Pressed {
                    return KeyResponse::Pass;
                }
                let next = match e.key {
                    Key::Left => focus.add_days(-1),
                    Key::Right => focus.add_days(1),
                    Key::Up => focus.add_days(-7),
                    Key::Down => focus.add_days(7),
                    Key::PageUp => focus.add_months(if e.modifiers.shift { -12 } else { -1 }),
                    Key::PageDown => focus.add_months(if e.modifiers.shift { 12 } else { 1 }),
                    Key::Home => focus.add_days(-i64::from((focus.weekday() + 7 - wso) % 7)),
                    Key::End => focus.add_days(6 - i64::from((focus.weekday() + 7 - wso) % 7)),
                    _ => return KeyResponse::Pass,
                };
                h.set(CalendarState { focus: Some(next) });
                requests.request(day_id(&id, next));
                KeyResponse::Handled
            })
        };
        let mut root = Node::new(Style {
            flex_direction: FlexDirection::Column,
            padding: crate::util::rect(len(t.space.s3)),
            min_size: taffy::Size {
                width: taffy::LengthPercentageAuto::length(220.0),
                height: taffy::LengthPercentageAuto::auto(),
            },
            align_self: Some(AlignItems::FLEX_START),
            ..Style::default()
        });
        root.on_key = Some(on_key);
        cx.begin(root);
        // Header: previous, month and year, next.
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            justify_content: JustifyContent::SPACE_BETWEEN,
            gap: taffy::Size {
                width: len(t.space.s2),
                height: len(0.0),
            },
            margin: taffy::Rect {
                left: taffy::LengthPercentageAuto::length(0.0),
                right: taffy::LengthPercentageAuto::length(0.0),
                top: taffy::LengthPercentageAuto::length(0.0),
                bottom: taffy::LengthPercentageAuto::length(t.space.s2),
            },
            ..Style::default()
        }));
        let prev_label = cx.t("@astryx.calendar.previousMonth", &args! {});
        let next_label = cx.t("@astryx.calendar.nextMonth", &args! {});
        let nav = |label: String, icon: &str, to: CivilDate| {
            let h = h.clone();
            Button::new(label)
                .variant(ButtonVariant::Ghost)
                .size(ComponentSize::Sm)
                .icon(Icon::new(icon))
                .is_icon_only(true)
                .effect(Rc::new(move || h.set(CalendarState { focus: Some(to) })))
        };
        let rtl = cx.rtl();
        cx.child(
            nav(
                prev_label,
                if rtl { "chevronRight" } else { "chevronLeft" },
                focus.add_months(-1),
            )
            .into(),
        );
        let n = self.number_of_months;
        let mut title_style = type_style(t.font.label, FontFamily::Body);
        title_style.weight = t.font.weight.semibold;
        let mut titles = Vec::new();
        for k in 0..n {
            let m = first_month.add_months(k as i32);
            titles.push(format!("{} {}", MONTHS[m.month as usize - 1], m.year));
        }
        cx.begin(Node::new(Style {
            flex_grow: 1.0,
            justify_content: JustifyContent::CENTER,
            ..Style::default()
        }));
        let mut title = text_node(titles.join("    "), title_style, color(c.text_primary));
        title.sem = Some(Semantics::new(Role::Heading(2), titles.join(", ")));
        cx.leaf(title);
        cx.end();
        cx.child(
            nav(
                next_label,
                if rtl { "chevronLeft" } else { "chevronRight" },
                focus.add_months(1),
            )
            .into(),
        );
        cx.end();
        // Months.
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Row,
            flex_wrap: taffy::FlexWrap::Wrap,
            gap: taffy::Size {
                width: len(t.space.s4),
                height: len(t.space.s4),
            },
            ..Style::default()
        }));
        let cell = t.size.element_md;
        let group = self.id.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
        });
        let day_side = t.size.element_sm;
        for k in 0..n {
            let month = first_month.add_months(k as i32);
            let lead = (month.weekday() + 7 - self.week_starts_on) % 7;
            let grid_start = month.add_days(-i64::from(lead));
            let dim = days_in_month(month.year, month.month);
            let rows = if self.has_variable_row_count {
                (lead + dim).div_ceil(7)
            } else {
                6
            };
            let cols = 7 + u32::from(self.has_week_numbers);
            let tpl: Vec<taffy::GridTemplateComponent<String>> = (0..cols)
                .map(|i| {
                    if self.has_week_numbers && i == 0 {
                        taffy::GridTemplateComponent::Single(taffy::prelude::length(cell))
                    } else {
                        taffy::GridTemplateComponent::Single(taffy::prelude::fr(1.0))
                    }
                })
                .collect();
            cx.begin(
                Node::new(Style {
                    display: taffy::Display::Grid,
                    grid_template_columns: tpl,
                    flex_grow: 1.0,
                    flex_basis: Dimension::length(0.0),
                    ..Style::default()
                })
                .sem(Semantics::new(
                    Role::Grid,
                    format!("{} {}", MONTHS[month.month as usize - 1], month.year),
                )),
            );
            // Weekday names.
            let names = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
            if self.has_week_numbers {
                cx.leaf(Node::new(Style::default()));
            }
            for i in 0..7 {
                let wd = ((self.week_starts_on + i) % 7) as usize;
                let mut s = type_style(t.font.supporting, FontFamily::Body);
                s.weight = t.font.weight.normal;
                let mut n = text_node(names[wd], s, color(c.text_secondary));
                n.style.size = taffy::Size {
                    width: Dimension::auto(),
                    height: Dimension::length(cell + t.space.s1),
                };
                n.style.padding.bottom = len(t.space.s1);
                n.sem = Some(Semantics::new(Role::ColumnHeader, WEEKDAYS[wd]));
                n.style.justify_self = Some(AlignItems::CENTER);
                n.style.align_self = Some(AlignItems::CENTER);
                cx.leaf(n);
            }
            for r in 0..rows {
                if self.has_week_numbers {
                    let wk = grid_start.add_days(i64::from(r * 7)).iso_week();
                    let mut w = text_node(
                        wk.to_string(),
                        type_style(t.font.supporting, FontFamily::Body),
                        color(c.text_secondary),
                    );
                    w.style.justify_self = Some(AlignItems::CENTER);
                    w.style.align_self = Some(AlignItems::CENTER);
                    cx.leaf(w);
                }
                for col in 0..7 {
                    let d = grid_start.add_days(i64::from(r * 7 + col));
                    let outside = d.month != month.month;
                    if outside && !self.has_outside_days {
                        cx.leaf(Node::new(Style {
                            min_size: taffy::Size {
                                width: taffy::LengthPercentageAuto::auto(),
                                height: taffy::LengthPercentageAuto::length(cell),
                            },
                            ..Style::default()
                        }));
                        continue;
                    }
                    let disabled = self.disabled(d);
                    let (selected, in_range, start, end) = match self.value {
                        CalendarValue::Single(v) => (v == Some(d), false, false, false),
                        CalendarValue::Range(Some(r)) => {
                            let end_d = r.end.unwrap_or(r.start);
                            let inr = d >= r.start && d <= end_d && r.end.is_some();
                            (
                                d == r.start || Some(d) == r.end,
                                inr,
                                d == r.start,
                                Some(d) == r.end,
                            )
                        }
                        CalendarValue::Range(None) => (false, false, false, false),
                    };
                    let spoken = spoken_date(d);
                    let label = if start && end {
                        cx.t(
                            "@astryx.calendar.dayRangeStartAndEnd",
                            &args! {"date" => spoken.clone()},
                        )
                    } else if start && in_range {
                        cx.t(
                            "@astryx.calendar.dayRangeStart",
                            &args! {"date" => spoken.clone()},
                        )
                    } else if end {
                        cx.t(
                            "@astryx.calendar.dayRangeEnd",
                            &args! {"date" => spoken.clone()},
                        )
                    } else if in_range {
                        cx.t(
                            "@astryx.calendar.dayInRange",
                            &args! {"date" => spoken.clone()},
                        )
                    } else if selected {
                        cx.t(
                            "@astryx.calendar.daySelected",
                            &args! {"date" => spoken.clone()},
                        )
                    } else {
                        spoken
                    };
                    // The cell, with the range band behind the day.
                    cx.begin(Node::new(Style {
                        align_items: AlignItems::CENTER,
                        justify_content: JustifyContent::CENTER,
                        min_size: taffy::Size {
                            width: taffy::LengthPercentageAuto::auto(),
                            height: taffy::LengthPercentageAuto::length(cell),
                        },
                        padding: crate::util::rect(len(2.0)),
                        ..Style::default()
                    }));
                    if in_range {
                        // Round the band's start and end, in reading order.
                        let full = day_side / 2.0;
                        let (lead, trail) = if rtl { (end, start) } else { (start, end) };
                        let l = if lead { full } else { 0.0 };
                        let tr = if trail { full } else { 0.0 };
                        let r = astryx_core::contract::Radii([l, tr, tr, l]);
                        let a = taffy::LengthPercentageAuto::length(0.0);
                        let inset = taffy::LengthPercentageAuto::length(2.0);
                        cx.leaf(
                            Node::new(Style {
                                position: taffy::Position::Absolute,
                                inset: taffy::Rect {
                                    left: if (start && !rtl) || (end && rtl) {
                                        inset
                                    } else {
                                        a
                                    },
                                    right: if (end && !rtl) || (start && rtl) {
                                        inset
                                    } else {
                                        a
                                    },
                                    top: inset,
                                    bottom: inset,
                                },
                                ..Style::default()
                            })
                            .draw(Draw::Box(BoxDraw::fill(color(c.accent_muted), r))),
                        );
                    }
                    let pill = radii(day_side / 2.0);
                    let (ground, ink) = if selected {
                        (color(c.accent), color(c.on_accent))
                    } else if outside {
                        (
                            astryx_core::contract::Color::TRANSPARENT,
                            color(c.text_secondary),
                        )
                    } else {
                        (
                            astryx_core::contract::Color::TRANSPARENT,
                            color(c.text_primary),
                        )
                    };
                    let today = self.today == Some(d) && !selected;
                    let mut sem = Semantics::new(Role::GridCell, label);
                    sem.selected = Some(selected);
                    sem.disabled = disabled;
                    if self.today == Some(d) {
                        sem.state.current = Some(astryx_core::contract::Current::Date);
                    }
                    let msg = self.on_change.as_ref().map(|f| f(self.pick(d)));
                    let h2 = h.clone();
                    let mut node = Node::new(Style {
                        align_items: AlignItems::CENTER,
                        justify_content: JustifyContent::CENTER,
                        min_size: taffy::Size {
                            width: taffy::LengthPercentageAuto::length(day_side),
                            height: taffy::LengthPercentageAuto::length(day_side),
                        },
                        ..Style::default()
                    })
                    .draw(Draw::Box(BoxDraw {
                        border: today.then(|| {
                            (
                                if in_range {
                                    color(c.text_primary)
                                } else {
                                    color(c.border_emphasized)
                                },
                                t.size.border_width,
                            )
                        }),
                        ..BoxDraw::fill(ground, pill)
                    }))
                    .interact(
                        Interact::new(msg, disabled)
                            .effect(move || h2.set(CalendarState { focus: Some(d) })),
                    )
                    .indicator(Indicator::ring(
                        color(c.accent),
                        pill,
                        Some(color(c.overlay_pressed)),
                    ))
                    .sem(sem)
                    .opacity(if disabled {
                        0.3
                    } else if outside {
                        0.5
                    } else {
                        1.0
                    });
                    // One tab stop: only the focused day is in the tab order.
                    node.group = Some(group);
                    if d == focus {
                        node.group_entry = true;
                    }
                    cx.key(Some(&day_id(&self.id, d)));
                    cx.begin(node);
                    let mut num = text_node(
                        d.day.to_string(),
                        type_style(t.font.body, FontFamily::Body),
                        ink,
                    );
                    num.sem = None;
                    cx.leaf(num);
                    cx.end();
                    cx.end();
                }
            }
            cx.end();
        }
        cx.end();
        cx.end();
    }
}

#[allow(dead_code)]
fn _unused(_: Element<()>) {}
