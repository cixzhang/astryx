//! Gamepad to [`Input`] mapping with direction repeat.
//!
//! The Gamepad API has no button events, so the shell polls each animation
//! frame and this module turns snapshots into pressed and released inputs.

use astryx_core::{Input, InputPhase};

/// Time a direction is held before it repeats, in milliseconds.
const REPEAT_DELAY_MS: f64 = 400.0;
/// Time between repeats while a direction stays held, in milliseconds.
const REPEAT_INTERVAL_MS: f64 = 90.0;
/// Stick deflection that presses a direction.
const AXIS_PRESS: f64 = 0.6;
/// Stick deflection below which a pressed direction releases.
const AXIS_RELEASE: f64 = 0.4;

/// Button indices of the W3C "standard" gamepad mapping.
const STANDARD_BUTTONS: [(usize, Input); 15] = [
    (0, Input::A),
    (1, Input::B),
    (2, Input::X),
    (3, Input::Y),
    (4, Input::L1),
    (5, Input::R1),
    (6, Input::L2),
    (7, Input::R2),
    (8, Input::Select),
    (9, Input::Start),
    (12, Input::Up),
    (13, Input::Down),
    (14, Input::Left),
    (15, Input::Right),
    (16, Input::Menu),
];

/// Face buttons only, for pads without the standard mapping.
const FALLBACK_BUTTONS: [(usize, Input); 4] =
    [(0, Input::A), (1, Input::B), (2, Input::X), (3, Input::Y)];

/// One gamepad snapshot.
pub(crate) struct Snapshot<'a> {
    pub standard: bool,
    /// A Nintendo-style pad: its face buttons are named by position in the
    /// mapping, so A and B, and X and Y, swap to follow the printed labels.
    pub nintendo: bool,
    pub buttons: &'a [bool],
    pub axes: &'a [f64],
}

fn is_direction(input: Input) -> bool {
    matches!(input, Input::Up | Input::Down | Input::Left | Input::Right)
}

/// The tracked state of one gamepad.
#[derive(Default)]
pub(crate) struct PadTracker {
    /// Held inputs and, for directions, the time of the next repeat.
    held: Vec<(Input, f64)>,
}

impl PadTracker {
    fn is_held(&self, input: Input) -> bool {
        self.held.iter().any(|(i, _)| *i == input)
    }

    fn held_now(&self, pad: &Snapshot) -> Vec<Input> {
        let table: &[(usize, Input)] = if pad.standard {
            &STANDARD_BUTTONS
        } else {
            &FALLBACK_BUTTONS
        };
        let mut now: Vec<Input> = table
            .iter()
            .filter(|(index, _)| pad.buttons.get(*index).copied().unwrap_or(false))
            .map(|(_, input)| match (pad.nintendo, *input) {
                (true, Input::A) => Input::B,
                (true, Input::B) => Input::A,
                (true, Input::X) => Input::Y,
                (true, Input::Y) => Input::X,
                (_, input) => input,
            })
            .collect();
        let x = pad.axes.first().copied().unwrap_or(0.0);
        let y = pad.axes.get(1).copied().unwrap_or(0.0);
        for (value, negative, positive) in
            [(x, Input::Left, Input::Right), (y, Input::Up, Input::Down)]
        {
            for (input, deflection) in [(negative, -value), (positive, value)] {
                let threshold = if self.is_held(input) {
                    AXIS_RELEASE
                } else {
                    AXIS_PRESS
                };
                if deflection > threshold && !now.contains(&input) {
                    now.push(input);
                }
            }
        }
        now
    }

    /// Compares a snapshot with the previous one and returns the inputs to send.
    pub(crate) fn update(&mut self, now_ms: f64, pad: &Snapshot) -> Vec<(Input, InputPhase)> {
        let current = self.held_now(pad);
        let mut out = Vec::new();
        self.held.retain(|(input, _)| {
            let keep = current.contains(input);
            if !keep {
                out.push((*input, InputPhase::Released));
            }
            keep
        });
        for (input, next_repeat) in &mut self.held {
            if is_direction(*input) && now_ms >= *next_repeat {
                out.push((*input, InputPhase::Pressed));
                *next_repeat = now_ms + REPEAT_INTERVAL_MS;
            }
        }
        for input in current {
            if !self.is_held(input) {
                out.push((input, InputPhase::Pressed));
                self.held.push((input, now_ms + REPEAT_DELAY_MS));
            }
        }
        out
    }

    /// Releases everything, for a disconnected pad.
    pub(crate) fn release_all(&mut self) -> Vec<(Input, InputPhase)> {
        self.held
            .drain(..)
            .map(|(input, _)| (input, InputPhase::Released))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use InputPhase::{Pressed, Released};

    fn buttons(pressed: &[usize]) -> Vec<bool> {
        (0..17).map(|i| pressed.contains(&i)).collect()
    }

    fn snap<'a>(b: &'a [bool], axes: &'a [f64]) -> Snapshot<'a> {
        Snapshot {
            standard: true,
            nintendo: false,
            buttons: b,
            axes,
        }
    }

    #[test]
    fn standard_buttons_press_and_release() {
        let mut pad = PadTracker::default();
        let down = buttons(&[0, 13]);
        assert_eq!(
            pad.update(0.0, &snap(&down, &[])),
            vec![(Input::A, Pressed), (Input::Down, Pressed)]
        );
        assert_eq!(pad.update(16.0, &snap(&down, &[])), vec![]);
        let none = buttons(&[]);
        assert_eq!(
            pad.update(32.0, &snap(&none, &[])),
            vec![(Input::A, Released), (Input::Down, Released)]
        );
    }

    #[test]
    fn held_direction_repeats_but_buttons_do_not() {
        let mut pad = PadTracker::default();
        let held = buttons(&[0, 15]);
        pad.update(0.0, &snap(&held, &[]));
        assert_eq!(pad.update(399.0, &snap(&held, &[])), vec![]);
        assert_eq!(
            pad.update(400.0, &snap(&held, &[])),
            vec![(Input::Right, Pressed)]
        );
        assert_eq!(pad.update(450.0, &snap(&held, &[])), vec![]);
        assert_eq!(
            pad.update(490.0, &snap(&held, &[])),
            vec![(Input::Right, Pressed)]
        );
    }

    #[test]
    fn stick_uses_hysteresis() {
        let mut pad = PadTracker::default();
        let none = buttons(&[]);
        assert_eq!(pad.update(0.0, &snap(&none, &[0.0, 0.5])), vec![]);
        assert_eq!(
            pad.update(1.0, &snap(&none, &[0.0, 0.7])),
            vec![(Input::Down, Pressed)]
        );
        assert_eq!(pad.update(2.0, &snap(&none, &[0.0, 0.5])), vec![]);
        assert_eq!(
            pad.update(3.0, &snap(&none, &[0.0, 0.3])),
            vec![(Input::Down, Released)]
        );
        assert_eq!(
            pad.update(4.0, &snap(&none, &[-0.9, 0.0])),
            vec![(Input::Left, Pressed)]
        );
    }

    #[test]
    fn nonstandard_pads_map_face_buttons_only() {
        let mut pad = PadTracker::default();
        let b = buttons(&[1, 13]);
        let out = pad.update(
            0.0,
            &Snapshot {
                standard: false,
                nintendo: false,
                buttons: &b,
                axes: &[],
            },
        );
        assert_eq!(out, vec![(Input::B, Pressed)]);
    }

    #[test]
    fn release_all_clears_state() {
        let mut pad = PadTracker::default();
        let b = buttons(&[0]);
        pad.update(0.0, &snap(&b, &[]));
        assert_eq!(pad.release_all(), vec![(Input::A, Released)]);
        assert_eq!(pad.update(1.0, &snap(&b, &[])), vec![(Input::A, Pressed)]);
    }
}
