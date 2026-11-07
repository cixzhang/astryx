//! The day's light changes smoothly: no step is visible from one minute to
//! the next, day is unlit, night is dark blue, and lamps fade in and out.

use astryx_core::Lighting;

/// What a mid-gray pixel becomes under the light at `minutes`.
fn tinted(minutes: f32) -> [f32; 3] {
    match Lighting::at(minutes, 0.0) {
        None => [0.5; 3],
        Some(l) => {
            let a = [l.ambient.r, l.ambient.g, l.ambient.b];
            a.map(|c| 0.5 * (1.0 - l.strength * (1.0 - c)))
        }
    }
}

#[test]
fn no_minute_to_minute_jumps() {
    let mut worst = 0.0f32;
    for m in 0..(24 * 60 * 4) {
        let (a, b) = (tinted(m as f32 / 4.0), tinted(m as f32 / 4.0 + 1.0));
        for c in 0..3 {
            worst = worst.max((a[c] - b[c]).abs());
        }
    }
    // Under 2% of full scale per minute: one or two 8-bit levels.
    assert!(worst < 0.02, "largest change in a minute {worst}");
}

#[test]
fn day_is_unlit_and_night_is_dark_blue() {
    assert!(Lighting::at(12.0 * 60.0, 0.0).is_none());
    for m in [23.0 * 60.0, 2.0 * 60.0, 24.0 * 60.0 + 60.0] {
        let night = Lighting::at(m, 0.0).unwrap();
        assert!((night.strength - 0.7).abs() < 1e-6, "{m}");
        assert!(night.ambient.b > night.ambient.r, "{m}");
    }
    // Golden hour and sunset are warm.
    let golden = Lighting::at(17.0 * 60.0 + 20.0, 0.0).unwrap();
    assert!(golden.ambient.r > golden.ambient.b);
}

#[test]
fn lamps_fade_in_at_dusk_and_out_at_dawn() {
    assert_eq!(Lighting::lamp_level(12.0 * 60.0), 0.0);
    assert_eq!(Lighting::lamp_level(23.0 * 60.0), 1.0);
    assert_eq!(Lighting::lamp_level(3.0 * 60.0), 1.0);
    let dusk: Vec<f32> = (0..=12)
        .map(|i| Lighting::lamp_level(17.5 * 60.0 + i as f32 * 10.0))
        .collect();
    assert!(dusk.windows(2).all(|w| w[1] >= w[0]));
    assert!(dusk[6] > 0.2 && dusk[6] < 0.8, "{dusk:?}");
}
