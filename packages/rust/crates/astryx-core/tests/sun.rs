//! Sunrise and sunset from the date and place (NOAA formulas), and the
//! day's light placed by them.

use astryx_core::{DayPhase, Lighting, SunTimes};

/// San Francisco Bay Area, as the Brick shell defaults to.
const LAT: f64 = 37.8;
const LON: f64 = -122.3;
const PST: f64 = -8.0 * 60.0;
const PDT: f64 = -7.0 * 60.0;

fn hm(minutes: f32) -> String {
    let m = minutes.round() as i32;
    format!("{:02}:{:02}", m / 60, m % 60)
}

/// Within `tol` minutes of `h:m`.
fn near(minutes: f32, h: i32, m: i32, tol: f32, what: &str) {
    let want = (h * 60 + m) as f32;
    assert!(
        (minutes - want).abs() <= tol,
        "{what}: {} not near {h:02}:{m:02}",
        hm(minutes)
    );
}

#[test]
fn bay_area_sun_times_match_published_tables() {
    // Published times for San Francisco, to the minute; the formula is
    // good to about two.
    let nov8 = SunTimes::compute(2026, 11, 8, LAT, LON, PST).unwrap();
    near(nov8.sunrise, 6, 41, 3.0, "Nov 8 sunrise");
    near(nov8.sunset, 17, 5, 3.0, "Nov 8 sunset");
    near(nov8.dawn, 6, 14, 3.0, "Nov 8 civil dawn");
    near(nov8.dusk, 17, 32, 3.0, "Nov 8 civil dusk");

    let june = SunTimes::compute(2026, 6, 21, LAT, LON, PDT).unwrap();
    near(june.sunrise, 5, 48, 3.0, "June solstice sunrise");
    near(june.sunset, 20, 35, 3.0, "June solstice sunset");

    let december = SunTimes::compute(2026, 12, 21, LAT, LON, PST).unwrap();
    near(december.sunrise, 7, 21, 3.0, "December solstice sunrise");
    near(december.sunset, 16, 54, 3.0, "December solstice sunset");
}

#[test]
fn the_daylight_saving_change_moves_local_times_by_an_hour() {
    // Clocks fell back on Sunday 2026-11-01: the day before is PDT.
    let before = SunTimes::compute(2026, 10, 31, LAT, LON, PDT).unwrap();
    let after = SunTimes::compute(2026, 11, 1, LAT, LON, PST).unwrap();
    near(before.sunrise, 7, 32, 3.0, "Oct 31 sunrise (PDT)");
    near(after.sunrise, 6, 33, 3.0, "Nov 1 sunrise (PST)");
    assert!((before.sunrise - after.sunrise - 60.0).abs() < 3.0);
}

#[test]
fn no_sunrise_in_polar_night() {
    assert_eq!(SunTimes::compute(2026, 12, 21, 78.2, 15.6, 60.0), None);
}

#[test]
fn the_light_follows_the_sun() {
    let nov8 = SunTimes::compute(2026, 11, 8, LAT, LON, PST).unwrap();
    // 05:00 is night before a 06:14 dawn: dark.
    let five = Lighting::at_sun(5.0 * 60.0, &nov8, 0.0).unwrap();
    assert!(five.strength >= 0.69, "05:00: {}", five.strength);
    // At sunrise the morning tint is on its way out.
    let rise = Lighting::at_sun(nov8.sunrise, &nov8, 0.0).unwrap();
    assert!(
        (rise.strength - 0.18).abs() < 0.02,
        "sunrise: {}",
        rise.strength
    );
    // Full day half an hour after sunrise, and through the afternoon.
    assert!(Lighting::at_sun(nov8.sunrise + 31.0, &nov8, 0.0).is_none());
    assert!(Lighting::at_sun(14.0 * 60.0, &nov8, 0.0).is_none());
    // After a 17:05 sunset it is getting dark well before the old 18:15.
    let six = Lighting::at_sun(18.0 * 60.0, &nov8, 0.0).unwrap();
    assert!(six.strength > 0.5, "18:00: {}", six.strength);
    assert_eq!(nov8.phase(5.0 * 60.0), DayPhase::Night);
    assert_eq!(nov8.phase(6.0 * 60.0 + 30.0), DayPhase::Dusk);
    assert_eq!(nov8.phase(12.0 * 60.0), DayPhase::Day);
    assert_eq!(nov8.phase(17.0 * 60.0 + 20.0), DayPhase::Dusk);
    assert_eq!(nov8.phase(19.0 * 60.0), DayPhase::Night);
    // Lamps: on before dawn, off by day, on again after dusk.
    assert_eq!(Lighting::lamp_level_sun(5.0 * 60.0, &nov8), 1.0);
    assert_eq!(Lighting::lamp_level_sun(12.0 * 60.0, &nov8), 0.0);
    assert_eq!(Lighting::lamp_level_sun(18.5 * 60.0, &nov8), 1.0);
}

#[test]
fn the_default_day_keeps_the_old_schedule() {
    // Lighting::at is at_sun on SunTimes::DEFAULT: 04:45 night, 05:45
    // dawn, 06:30 morning, 07:00 day, 18:15 sunset, 20:30 night.
    let s = |h: f32, m: f32| Lighting::at(h * 60.0 + m, 0.0).map_or(0.0, |l| l.strength);
    assert!((s(4.0, 45.0) - 0.70).abs() < 1e-4);
    assert!((s(5.0, 45.0) - 0.35).abs() < 1e-4);
    assert!((s(6.0, 30.0) - 0.18).abs() < 1e-4);
    assert_eq!(s(7.0, 0.0), 0.0);
    assert!((s(18.0, 15.0) - 0.38).abs() < 1e-4);
    assert!((s(20.0, 30.0) - 0.70).abs() < 1e-4);
    assert!((s(2.0, 0.0) - 0.70).abs() < 1e-4);
}
