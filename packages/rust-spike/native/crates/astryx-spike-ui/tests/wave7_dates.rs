//! Wave 7: Calendar and date math.

use astryx_core::contract::Role;
use astryx_core::Key;
use astryx_spike_ui::testkit::{all, find, Rig};
use astryx_spike_ui::*;

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Pick(CalendarValue),
}

fn d(y: i32, m: u32, day: u32) -> CivilDate {
    CivilDate::new(y, m, day)
}

#[test]
fn civil_date_math() {
    assert_eq!(d(1970, 1, 1).days(), 0);
    assert_eq!(CivilDate::from_days(d(2026, 11, 9).days()), d(2026, 11, 9));
    assert_eq!(d(2026, 11, 9).weekday(), 1, "a Monday");
    assert_eq!(d(2024, 1, 31).add_months(1), d(2024, 2, 29));
    assert_eq!(CivilDate::parse("2026-02-30"), None);
    assert_eq!(
        CivilDate::parse("2026-02-28").map(|x| x.iso()).as_deref(),
        Some("2026-02-28")
    );
    assert_eq!(d(2026, 1, 1).iso_week(), 1);
    assert_eq!(d(2021, 1, 3).iso_week(), 53);
}

#[test]
fn calendar_grid_shows_the_selected_month_and_picks() {
    let mut rig = Rig::<Msg>::new(|| {
        Calendar::new("cal", CalendarValue::Single(Some(d(2026, 11, 9))))
            .today(d(2026, 11, 9))
            .on_change(Msg::Pick)
            .into()
    });
    let tree = rig.tree();
    assert!(find(&tree, Role::Grid, "November 2026").is_some());
    assert_eq!(all(&tree, Role::GridCell).len(), 42, "six weeks");
    let sel =
        find(&tree, Role::GridCell, "Monday, November 9, 2026, selected").expect("selected day");
    assert_eq!(sel.selected, Some(true));
    // The day grid is one tab stop: focus enters on the selected day after
    // the two month buttons; arrows move and Enter picks.
    rig.press(astryx_core::contract::Input::Next);
    rig.press(astryx_core::contract::Input::Next);
    rig.key(Key::Right, Default::default());
    rig.key(Key::Enter, Default::default());
    assert_eq!(
        rig.messages().last(),
        Some(&Msg::Pick(CalendarValue::Single(Some(d(2026, 11, 10)))))
    );
}

#[test]
fn page_down_moves_a_month_and_range_marks_the_band() {
    let mut rig = Rig::<Msg>::new(|| {
        Calendar::new(
            "range",
            CalendarValue::Range(Some(DateRange {
                start: d(2026, 3, 3),
                end: Some(d(2026, 3, 6)),
            })),
        )
        .on_change(Msg::Pick)
        .into()
    });
    let tree = rig.tree();
    assert!(find(&tree, Role::GridCell, "Tuesday, March 3, 2026, range start").is_some());
    assert!(find(&tree, Role::GridCell, "Wednesday, March 4, 2026, in range").is_some());
    assert!(find(&tree, Role::GridCell, "Friday, March 6, 2026, range end").is_some());
    rig.press(astryx_core::contract::Input::Next);
    rig.press(astryx_core::contract::Input::Next);
    rig.key(Key::PageDown, Default::default());
    assert!(find(&rig.tree(), Role::Grid, "April 2026").is_some());
}

#[derive(Clone, Debug, PartialEq)]
enum NMsg {
    Num(f64),
    Date(CalendarValue),
    Time(Option<ClockTime>),
}

#[test]
fn number_input_commits_on_enter_and_steps_with_arrows() {
    assert_eq!(parse_number("1,200", None, None, false), Some(1200.0));
    assert_eq!(parse_number("7.6", Some(0.0), Some(5.0), true), Some(5.0));
    assert_eq!(parse_number("-", None, None, false), None);
    let mut rig = Rig::<NMsg>::new(|| {
        NumberInput::new("qty", "Quantity", Some(3.0))
            .min(0.0)
            .max(10.0)
            .has_number_steppers(true)
            .on_change(NMsg::Num)
            .into()
    });
    let field = find(&rig.tree(), Role::SpinButton, "Quantity")
        .cloned()
        .unwrap();
    assert_eq!(field.value.as_deref(), Some("3"));
    rig.key(Key::Up, Default::default());
    assert_eq!(rig.messages(), vec![NMsg::Num(4.0)]);
    rig.type_text("9");
    rig.key(Key::Enter, Default::default());
    assert_eq!(
        rig.messages().last(),
        Some(&NMsg::Num(10.0)),
        "39 clamps to the max"
    );
    assert!(find(&rig.tree(), Role::Button, "Increase").is_some());
}

#[test]
fn date_input_opens_a_calendar_and_formats_the_value() {
    let mut rig = Rig::<NMsg>::sized(
        || {
            DateInput::single("due", "Due date", Some(d(2026, 11, 9)))
                .on_change(NMsg::Date)
                .into()
        },
        800,
        600,
    );
    let b = find(&rig.tree(), Role::Button, "Due date")
        .cloned()
        .unwrap();
    assert_eq!(b.value.as_deref(), Some("November 9, 2026"));
    rig.press(astryx_core::contract::Input::A);
    assert!(find(&rig.tree(), Role::Grid, "November 2026").is_some());
    assert_eq!(format_date(d(2026, 11, 9), DateFormat::Date), "Nov 9, 2026");
    let range = Rig::<NMsg>::new(|| {
        DateInput::range(
            "trip",
            "Trip",
            Some(DateRange {
                start: d(2026, 3, 3),
                end: Some(d(2026, 3, 6)),
            }),
        )
        .into()
    });
    let b = find(&range.tree(), Role::Button, "Trip").cloned().unwrap();
    assert_eq!(b.value.as_deref(), Some("Mar 3, 2026 – Mar 6, 2026"));
}

#[test]
fn time_input_lists_times() {
    assert_eq!(ClockTime::new(13, 5).label(true), "1:05 PM");
    assert_eq!(ClockTime::parse("09:30"), Some(ClockTime::new(9, 30)));
    let mut rig = Rig::<NMsg>::new(|| {
        TimeInput::new("t", "Start", Some(ClockTime::new(9, 0)))
            .min(ClockTime::new(9, 0))
            .max(ClockTime::new(10, 0))
            .on_change(NMsg::Time)
            .into()
    });
    rig.key(Key::Down, Default::default());
    let opts: Vec<String> = all(&rig.tree(), Role::ListBoxOption)
        .iter()
        .filter_map(|o| o.label.clone())
        .collect();
    assert_eq!(
        opts,
        ["9:00 AM", "9:15 AM", "9:30 AM", "9:45 AM", "10:00 AM"]
    );
    rig.key(Key::Down, Default::default());
    rig.key(Key::Enter, Default::default());
    assert_eq!(
        rig.messages(),
        vec![NMsg::Time(Some(ClockTime::new(9, 15)))]
    );
}
