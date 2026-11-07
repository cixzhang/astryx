//! Focus-first gestures as plain state machines, fed by input, phase and time.
//!
//! - [`PressGesture`]: A down shows the pressed state, release activates; a
//!   hold past [`LONG_PRESS_MS`] is a long press instead.
//! - [`PressHighlight`]: the pressed state's look: on at once, fading out on
//!   the theme's exit timing (`component:Button` FR15).
//! - [`RepeatGesture`]: a held direction repeats after a delay.
//! - [`PagingGesture`]: L2 and R2 page back and forward.
//!
//! Time is milliseconds on any monotonic clock the caller keeps. The input
//! type is generic, so a shell or the runtime feeds its own `Input` values.
//!
//! Web gestures `spec:AST-107` defines, and where they stand here:
//!
//! | Gesture | Here |
//! |---|---|
//! | Press, activate on release | [`PressGesture`] |
//! | Long press (`useLongPress`) | [`PressGesture`], 500 ms, the same clock |
//! | Context request (long press, right click, Shift+F10; FR8) | Long press here; the menu is the runtime's |
//! | Pan (`usePanGesture`, FR1) | Not applicable yet: needs a pointer |
//! | Drag to snap points, flick threshold (FR2) | Not applicable yet: needs a pointer; `settleTo(index)`, its keyboard path, is an [`crate::Animated`] spring |
//! | Swipe, throw, momentum decay | Not applicable yet: needs a pointer |
//! | Animator `animate`, `set`, `stop`, `get`, `getVelocity` (FR3) | [`crate::Animated`] with `springs::STATE_CHANGE` |
//! | Animator `decay` | Not applicable yet: nothing throws without a pointer |

use crate::reduced::{Change, MotionPolicy, Plan};
use crate::tokens::{MotionTokens, LONG_PRESS_MS};
use crate::{Animated, Motion};

/// Whether a button went down or up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Pressed,
    Released,
}

/// What a press reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PressEvent {
    /// Show the pressed state.
    Down,
    /// Released before the long-press threshold: activate.
    Activate,
    /// Held past the threshold: a long press (a context request).
    LongPress,
    /// The pressed state ends (after Activate, a long press's release, or a cancel).
    Up,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PressState {
    Idle,
    Down { since_ms: f64 },
    Long,
}

/// A press on the focused element.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PressGesture {
    state: PressState,
    /// How long a hold takes to become a long press. Press clocks are fixed:
    /// `tempo` does not move them (`spec:AST-108` FR4).
    pub long_press_ms: f64,
}

impl Default for PressGesture {
    fn default() -> Self {
        PressGesture {
            state: PressState::Idle,
            long_press_ms: LONG_PRESS_MS as f64,
        }
    }
}

impl PressGesture {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_down(&self) -> bool {
        !matches!(self.state, PressState::Idle)
    }

    /// Feed the confirm button's phase.
    pub fn input(&mut self, phase: Phase, now_ms: f64) -> Vec<PressEvent> {
        match (phase, self.state) {
            (Phase::Pressed, PressState::Idle) => {
                self.state = PressState::Down { since_ms: now_ms };
                vec![PressEvent::Down]
            }
            // A key repeat while held changes nothing.
            (Phase::Pressed, _) => Vec::new(),
            (Phase::Released, PressState::Down { since_ms }) => {
                self.state = PressState::Idle;
                if now_ms - since_ms >= self.long_press_ms {
                    // The clock passed the threshold without a tick: a long press.
                    vec![PressEvent::LongPress, PressEvent::Up]
                } else {
                    vec![PressEvent::Activate, PressEvent::Up]
                }
            }
            (Phase::Released, PressState::Long) => {
                self.state = PressState::Idle;
                vec![PressEvent::Up]
            }
            (Phase::Released, PressState::Idle) => Vec::new(),
        }
    }

    /// Call every frame while down: reports the long press when the hold
    /// reaches the threshold.
    pub fn tick(&mut self, now_ms: f64) -> Vec<PressEvent> {
        if let PressState::Down { since_ms } = self.state {
            if now_ms - since_ms >= self.long_press_ms {
                self.state = PressState::Long;
                return vec![PressEvent::LongPress];
            }
        }
        Vec::new()
    }

    /// Focus moved or the element went away: end without activating.
    pub fn cancel(&mut self) -> Vec<PressEvent> {
        if self.is_down() {
            self.state = PressState::Idle;
            vec![PressEvent::Up]
        } else {
            Vec::new()
        }
    }

    /// When the next `tick` could report something, so a runtime knows it
    /// still needs frames (or a timer) while a press is held.
    pub fn wants_frames(&self) -> bool {
        matches!(self.state, PressState::Down { .. })
    }
}

/// The pressed state's strength, 0 to 1: it comes on in the frame the press
/// lands and fades out on `--duration-exit` and `--ease-exit` after release
/// (`component:Button` FR15). Under reduced motion the onset is already
/// instant, and a non-touch release is instant too (`design:reduced-motion`
/// DEC-4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PressHighlight {
    strength: Animated<f32>,
}

impl Default for PressHighlight {
    fn default() -> Self {
        PressHighlight {
            strength: Animated::new(0.0),
        }
    }
}

impl PressHighlight {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn strength(&self) -> f32 {
        self.strength.value()
    }

    /// Apply one press event.
    pub fn apply(
        &mut self,
        event: PressEvent,
        tokens: &MotionTokens,
        policy: &MotionPolicy,
        touch: bool,
    ) {
        match event {
            PressEvent::Down => self.strength.set(1.0),
            PressEvent::Up => {
                let motion = match policy.plan(Change::PressRelease { touch }, tokens) {
                    Plan::Animate => Motion::curve(tokens.durations.exit, tokens.ease_exit),
                    Plan::Instant => Motion::Instant,
                    Plan::CrossFade {
                        duration_ms,
                        easing,
                    } => Motion::curve(duration_ms, easing),
                };
                self.strength.animate_to(0.0, motion);
            }
            PressEvent::Activate | PressEvent::LongPress => {}
        }
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        self.strength.tick(dt)
    }

    pub fn wants_frames(&self) -> bool {
        self.strength.is_animating()
    }
}

/// Hold-to-repeat timing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RepeatTiming {
    /// Time a button is held before it repeats.
    pub delay_ms: f64,
    /// Time between repeats while it stays held.
    pub interval_ms: f64,
}

/// The D-pad's repeat timing. Astryx has no record for it; these are the
/// values the Rust web shell already uses, so every shell repeats alike.
pub const DPAD_REPEAT: RepeatTiming = RepeatTiming {
    delay_ms: 400.0,
    interval_ms: 90.0,
};

/// Repeats the most recently pressed held input. Inputs that should not
/// repeat are simply not fed to it.
#[derive(Clone, Debug, PartialEq)]
pub struct RepeatGesture<I: Copy + PartialEq> {
    timing: RepeatTiming,
    held: Option<(I, f64)>,
}

impl<I: Copy + PartialEq> RepeatGesture<I> {
    pub fn new(timing: RepeatTiming) -> Self {
        RepeatGesture { timing, held: None }
    }

    /// Feed a phase. Returns the input once when it goes down.
    pub fn input(&mut self, input: I, phase: Phase, now_ms: f64) -> Option<I> {
        match phase {
            Phase::Pressed => {
                // A platform key repeat for the input already held is ignored;
                // this machine owns the repeat clock.
                if self.held.is_some_and(|(held, _)| held == input) {
                    return None;
                }
                self.held = Some((input, now_ms + self.timing.delay_ms));
                Some(input)
            }
            Phase::Released => {
                if self.held.is_some_and(|(held, _)| held == input) {
                    self.held = None;
                }
                None
            }
        }
    }

    /// Call every frame: the repeat due now, if any. After a long frame it
    /// reports one repeat, not a burst, and restarts the interval from now.
    pub fn poll(&mut self, now_ms: f64) -> Option<I> {
        let (input, next) = self.held?;
        if now_ms >= next {
            self.held = Some((input, now_ms + self.timing.interval_ms));
            Some(input)
        } else {
            None
        }
    }

    /// Focus left or the app lost the input: stop repeating.
    pub fn cancel(&mut self) {
        self.held = None;
    }

    pub fn wants_frames(&self) -> bool {
        self.held.is_some()
    }
}

/// A page step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Previous,
    Next,
}

/// L2 pages back, R2 pages forward, one page per press. With a repeat
/// timing, a held trigger keeps paging.
#[derive(Clone, Debug, PartialEq)]
pub struct PagingGesture {
    repeat: Option<RepeatGesture<Page>>,
}

impl PagingGesture {
    /// One page per press.
    pub fn new() -> Self {
        PagingGesture { repeat: None }
    }

    /// Keeps paging while a trigger is held.
    pub fn repeating(timing: RepeatTiming) -> Self {
        PagingGesture {
            repeat: Some(RepeatGesture::new(timing)),
        }
    }

    /// Feed L2 (`Page::Previous`) or R2 (`Page::Next`).
    pub fn input(&mut self, page: Page, phase: Phase, now_ms: f64) -> Option<Page> {
        match &mut self.repeat {
            Some(r) => r.input(page, phase, now_ms),
            None => (phase == Phase::Pressed).then_some(page),
        }
    }

    pub fn poll(&mut self, now_ms: f64) -> Option<Page> {
        self.repeat.as_mut().and_then(|r| r.poll(now_ms))
    }

    pub fn wants_frames(&self) -> bool {
        self.repeat.as_ref().is_some_and(|r| r.wants_frames())
    }
}

impl Default for PagingGesture {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reduced::Pointer;
    use crate::tokens::ASTRYX;

    #[test]
    fn a_short_press_activates_on_release() {
        let mut p = PressGesture::new();
        assert_eq!(p.input(Phase::Pressed, 0.0), vec![PressEvent::Down]);
        assert!(p.tick(100.0).is_empty());
        assert_eq!(
            p.input(Phase::Released, 120.0),
            vec![PressEvent::Activate, PressEvent::Up]
        );
        assert!(!p.is_down());
    }

    #[test]
    fn a_hold_becomes_a_long_press_and_does_not_activate() {
        let mut p = PressGesture::new();
        p.input(Phase::Pressed, 0.0);
        assert!(p.tick(499.0).is_empty());
        assert_eq!(p.tick(500.0), vec![PressEvent::LongPress]);
        assert!(p.tick(600.0).is_empty());
        assert_eq!(p.input(Phase::Released, 900.0), vec![PressEvent::Up]);
    }

    #[test]
    fn a_long_press_is_seen_even_without_ticks() {
        let mut p = PressGesture::new();
        p.input(Phase::Pressed, 0.0);
        assert_eq!(
            p.input(Phase::Released, 800.0),
            vec![PressEvent::LongPress, PressEvent::Up]
        );
    }

    #[test]
    fn repeats_and_cancel() {
        let mut p = PressGesture::new();
        p.input(Phase::Pressed, 0.0);
        assert!(p.input(Phase::Pressed, 30.0).is_empty());
        assert_eq!(p.cancel(), vec![PressEvent::Up]);
        assert!(p.input(Phase::Released, 50.0).is_empty());
    }

    #[test]
    fn highlight_on_at_once_and_fades_on_exit_timing() {
        let mut h = PressHighlight::new();
        let policy = MotionPolicy::default();
        h.apply(PressEvent::Down, &ASTRYX, &policy, false);
        assert_eq!(h.strength(), 1.0);
        h.apply(PressEvent::Up, &ASTRYX, &policy, false);
        h.tick(0.05);
        let want = 1.0 - ASTRYX.ease_exit.ease(0.05 / 0.175);
        assert!((h.strength() - want).abs() < 1e-4);
        for _ in 0..30 {
            h.tick(1.0 / 60.0);
        }
        assert_eq!(h.strength(), 0.0);
    }

    #[test]
    fn highlight_release_is_instant_under_reduced_motion_without_touch() {
        let mut h = PressHighlight::new();
        let policy = MotionPolicy {
            prefers_reduced_motion: true,
            pointer: Pointer::Fine,
        };
        h.apply(PressEvent::Down, &ASTRYX, &policy, false);
        h.apply(PressEvent::Up, &ASTRYX, &policy, false);
        assert_eq!(h.strength(), 0.0);
        // A finger's release keeps a short fade.
        h.apply(PressEvent::Down, &ASTRYX, &policy, true);
        h.apply(PressEvent::Up, &ASTRYX, &policy, true);
        assert!(h.wants_frames());
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Dir {
        Up,
        Down,
    }

    #[test]
    fn dpad_repeats_after_delay_at_interval() {
        let mut r = RepeatGesture::new(DPAD_REPEAT);
        assert_eq!(r.input(Dir::Down, Phase::Pressed, 0.0), Some(Dir::Down));
        assert_eq!(r.poll(399.0), None);
        assert_eq!(r.poll(400.0), Some(Dir::Down));
        assert_eq!(r.poll(450.0), None);
        assert_eq!(r.poll(490.0), Some(Dir::Down));
        // A platform repeat for the held key is ignored.
        assert_eq!(r.input(Dir::Down, Phase::Pressed, 500.0), None);
        assert_eq!(r.input(Dir::Down, Phase::Released, 510.0), None);
        assert_eq!(r.poll(2000.0), None);
        assert!(!r.wants_frames());
    }

    #[test]
    fn newest_direction_wins_and_a_stall_gives_one_repeat() {
        let mut r = RepeatGesture::new(DPAD_REPEAT);
        r.input(Dir::Down, Phase::Pressed, 0.0);
        assert_eq!(r.input(Dir::Up, Phase::Pressed, 100.0), Some(Dir::Up));
        // Releasing the older one does not stop the newer.
        r.input(Dir::Down, Phase::Released, 150.0);
        assert_eq!(r.poll(500.0), Some(Dir::Up));
        assert_eq!(r.poll(5000.0), Some(Dir::Up));
        assert_eq!(r.poll(5001.0), None);
    }

    #[test]
    fn paging_one_page_per_press_or_repeating() {
        let mut p = PagingGesture::new();
        assert_eq!(p.input(Page::Next, Phase::Pressed, 0.0), Some(Page::Next));
        assert_eq!(p.poll(1000.0), None);
        assert_eq!(p.input(Page::Next, Phase::Released, 1000.0), None);
        let mut r = PagingGesture::repeating(DPAD_REPEAT);
        assert_eq!(
            r.input(Page::Previous, Phase::Pressed, 0.0),
            Some(Page::Previous)
        );
        assert_eq!(r.poll(400.0), Some(Page::Previous));
    }
}
