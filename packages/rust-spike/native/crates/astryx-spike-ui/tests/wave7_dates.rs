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
