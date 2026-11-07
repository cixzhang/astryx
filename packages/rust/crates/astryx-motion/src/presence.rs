//! Enter and exit of a surface (`spec:AST-090`).
//!
//! A surface opens with its entrance and, when its kind's exit is on, leaves
//! by retracing it on `--ease-exit`. While it leaves it keeps its content,
//! takes no input, and a reopen returns it to open from where it is.

use crate::animated::{Animated, Motion};
use crate::reduced::{Change, CrossFade, FadeShape, MotionPolicy, Plan, Pointer};
use crate::spring::SpringSpec;
use crate::tokens::MotionTokens;

/// The two kinds of surface whose exit a theme switches (`spec:AST-090` FR2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceKind {
    /// A dialog: its exit duration is `--duration-exit-dialog`.
    Dialog,
    /// An anchored layer (popover, tooltip, menu, selector): `--duration-exit-layer`.
    Layer,
}

/// How a component's touch motion plays (`spec:AST-094` FR45).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Interaction {
    /// A component whose main interaction is a tap: curve tokens.
    Tap,
    /// A component whose main interaction is a gesture: its spring.
    Gesture(SpringSpec),
}

/// Where a surface is in its life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Closed,
    Entering,
    Open,
    Leaving,
}

/// What a caller must act on after `close` or `tick`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresenceEvent {
    /// Return focus now: a layer when its close begins, a dialog when it is
    /// hidden (`spec:AST-090` FR6).
    ReturnFocus,
    /// The surface is hidden: stop drawing it (FR1, FR9).
    Hidden,
}

/// How one entrance or exit plays.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Play {
    Instant,
    Move(Motion),
    Fade(CrossFade),
}

/// FR9: a leaving surface is hidden no later than 100 ms after its exit
/// duration, even if its exit never reports an end.
pub const EXIT_GRACE_MS: f32 = 100.0;

/// One surface's enter and exit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Presence {
    kind: SurfaceKind,
    phase: Phase,
    /// 0 = closed position, 1 = in place.
    travel: Animated<f32>,
    fade: Option<(CrossFade, f32)>,
    leave_elapsed_ms: f32,
    leave_limit_ms: f32,
}

impl Presence {
    pub fn new(kind: SurfaceKind) -> Self {
        Presence {
            kind,
            phase: Phase::Closed,
            travel: Animated::new(0.0),
            fade: None,
            leave_elapsed_ms: 0.0,
            leave_limit_ms: 0.0,
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Drawn at all (entering, open or leaving).
    pub fn is_visible(&self) -> bool {
        self.phase != Phase::Closed
    }

    /// FR4: no press or key activates anything inside a leaving surface.
    pub fn accepts_input(&self) -> bool {
        matches!(self.phase, Phase::Entering | Phase::Open)
    }

    /// FR6: a leaving layer is hidden from assistive technology; a leaving
    /// dialog stays until it is hidden.
    pub fn in_accessibility_tree(&self) -> bool {
        match self.phase {
            Phase::Closed => false,
            Phase::Leaving => self.kind == SurfaceKind::Dialog,
            _ => true,
        }
    }

    /// FR11: while leaving, keep drawing the content shown when the close began.
    pub fn is_leaving(&self) -> bool {
        self.phase == Phase::Leaving
    }

    /// How far along its entrance path the surface is: 0 at the closed
    /// position, 1 in place. Map it to the entrance's scale and offset; an
    /// exit runs it back, so the exit retraces the entrance (FR3).
    pub fn travel(&self) -> f32 {
        if self.fade.is_some() {
            1.0
        } else {
            self.travel.value()
        }
    }

    /// The opacity multiplier.
    pub fn opacity(&self) -> f32 {
        match self.fade {
            Some((fade, elapsed)) => fade.opacity(elapsed),
            None => self.travel.value().clamp(0.0, 1.0),
        }
    }

    /// Opens with an entrance of `duration_ms` on `--ease-entrance`, or the
    /// component's spring for a gesture component. Reopening a leaving
    /// surface returns it to open from where it is, with no hide (FR8).
    pub fn open(
        &mut self,
        duration_ms: f32,
        interaction: Interaction,
        tokens: &MotionTokens,
        policy: &MotionPolicy,
    ) {
        if matches!(self.phase, Phase::Open | Phase::Entering) {
            return;
        }
        let designed = match interaction {
            Interaction::Tap => Motion::curve(duration_ms, tokens.ease_entrance),
            Interaction::Gesture(spring) => Motion::Spring(tokens.spring(spring).spring()),
        };
        let play = plan(designed, FadeShape::Enter, tokens, policy);
        self.phase = if play == Play::Instant {
            Phase::Open
        } else {
            Phase::Entering
        };
        self.start(1.0, play);
    }

    /// Closes. Returns the events to act on now.
    ///
    /// Pointer presentation (FR1–FR10): the exit plays only when its kind's
    /// exit duration is above zero, on `--ease-exit`; reduced motion closes
    /// at once (FR10). Touch presentation (FR14): the exit always plays: a
    /// tap component's on `--duration-exit` and `--ease-exit`, a gesture
    /// component's on its spring; reduced motion cross-fades it.
    pub fn close(
        &mut self,
        interaction: Interaction,
        tokens: &MotionTokens,
        policy: &MotionPolicy,
    ) -> Vec<PresenceEvent> {
        if matches!(self.phase, Phase::Closed | Phase::Leaving) {
            return Vec::new();
        }
        let mut events = Vec::new();
        if self.kind == SurfaceKind::Layer {
            events.push(PresenceEvent::ReturnFocus);
        }
        let play = match policy.pointer {
            Pointer::Fine => {
                let ms = match self.kind {
                    SurfaceKind::Dialog => tokens.durations.exit_dialog,
                    SurfaceKind::Layer => tokens.durations.exit_layer,
                };
                if ms <= 0.0 || policy.reduces(tokens) {
                    Play::Instant
                } else {
                    Play::Move(Motion::curve(ms, tokens.ease_exit))
                }
            }
            Pointer::Coarse => {
                let designed = match interaction {
                    Interaction::Tap => Motion::curve(tokens.durations.exit, tokens.ease_exit),
                    Interaction::Gesture(spring) => Motion::Spring(tokens.spring(spring).spring()),
                };
                plan(designed, FadeShape::Exit, tokens, policy)
            }
        };
        if play == Play::Instant {
            self.hide(&mut events);
            return events;
        }
        self.phase = Phase::Leaving;
        self.leave_elapsed_ms = 0.0;
        self.start(0.0, play);
        self.leave_limit_ms = self.remaining_ms() + EXIT_GRACE_MS;
        events
    }

    /// Advances by `dt` seconds. Returns the events to act on.
    pub fn tick(&mut self, dt: f32) -> Vec<PresenceEvent> {
        let mut events = Vec::new();
        let moving = match &mut self.fade {
            Some((fade, elapsed)) => {
                *elapsed += dt * 1000.0;
                !fade.is_done(*elapsed)
            }
            None => self.travel.tick(dt),
        };
        match self.phase {
            Phase::Entering if !moving => {
                self.fade = None;
                self.travel.set(1.0);
                self.phase = Phase::Open;
            }
            Phase::Leaving => {
                self.leave_elapsed_ms += dt * 1000.0;
                if !moving || self.leave_elapsed_ms >= self.leave_limit_ms {
                    self.hide(&mut events);
                }
            }
            _ => {}
        }
        events
    }

    /// True while the surface still animates.
    pub fn wants_frames(&self) -> bool {
        matches!(self.phase, Phase::Entering | Phase::Leaving)
    }

    fn start(&mut self, to: f32, play: Play) {
        match play {
            Play::Instant => {
                self.fade = None;
                self.travel.set(to);
            }
            Play::Move(motion) => {
                self.fade = None;
                self.travel.animate_to(to, motion);
            }
            Play::Fade(fade) => {
                self.travel.set(to);
                self.fade = Some((fade, 0.0));
            }
        }
    }

    fn remaining_ms(&self) -> f32 {
        match self.fade {
            Some((fade, elapsed)) => (fade.duration_ms - elapsed).max(0.0),
            None => self.travel.remaining() * 1000.0,
        }
    }

    fn hide(&mut self, events: &mut Vec<PresenceEvent>) {
        self.phase = Phase::Closed;
        self.fade = None;
        self.travel.set(0.0);
        if self.kind == SurfaceKind::Dialog {
            events.push(PresenceEvent::ReturnFocus);
        }
        events.push(PresenceEvent::Hidden);
    }
}

fn plan(designed: Motion, shape: FadeShape, tokens: &MotionTokens, policy: &MotionPolicy) -> Play {
    match policy.plan(Change::Transition, tokens) {
        Plan::Animate if designed == Motion::Instant => Play::Instant,
        Plan::Animate => Play::Move(designed),
        Plan::Instant => Play::Instant,
        Plan::CrossFade {
            duration_ms,
            easing,
        } => Play::Fade(CrossFade {
            shape,
            duration_ms,
            easing,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::{springs, ASTRYX};

    fn exits_on() -> MotionTokens {
        let mut t = ASTRYX;
        t.durations.exit_dialog = t.durations.exit;
        t.durations.exit_layer = t.durations.exit;
        t
    }

    fn run(p: &mut Presence, seconds: f32) -> Vec<PresenceEvent> {
        let mut all = Vec::new();
        for _ in 0..(seconds * 120.0) as usize {
            all.extend(p.tick(1.0 / 120.0));
        }
        all
    }

    const FINE: MotionPolicy = MotionPolicy {
        prefers_reduced_motion: false,
        pointer: Pointer::Fine,
    };
    const TOUCH: MotionPolicy = MotionPolicy {
        prefers_reduced_motion: false,
        pointer: Pointer::Coarse,
    };

    #[test]
    fn exits_are_off_by_default_and_close_at_once() {
        let mut p = Presence::new(SurfaceKind::Dialog);
        p.open(ASTRYX.durations.medium, Interaction::Tap, &ASTRYX, &FINE);
        run(&mut p, 1.0);
        assert_eq!(p.phase(), Phase::Open);
        let events = p.close(Interaction::Tap, &ASTRYX, &FINE);
        assert_eq!(
            events,
            vec![PresenceEvent::ReturnFocus, PresenceEvent::Hidden]
        );
        assert!(!p.is_visible());
    }

    #[test]
    fn a_theme_turns_an_exit_on_and_it_retraces_on_ease_exit() {
        let t = exits_on();
        let mut p = Presence::new(SurfaceKind::Layer);
        p.open(t.durations.fast, Interaction::Tap, &t, &FINE);
        run(&mut p, 0.5);
        let events = p.close(Interaction::Tap, &t, &FINE);
        // A layer hands focus back when its close begins.
        assert_eq!(events, vec![PresenceEvent::ReturnFocus]);
        assert!(p.is_leaving());
        assert!(!p.accepts_input());
        assert!(!p.in_accessibility_tree());
        p.tick(0.05);
        let want = 1.0 - t.ease_exit.ease(0.05 / (t.durations.exit_layer / 1000.0));
        assert!((p.travel() - want).abs() < 1e-4);
        let events = run(&mut p, 0.5);
        assert_eq!(events, vec![PresenceEvent::Hidden]);
    }

    #[test]
    fn a_leaving_dialog_keeps_its_place_and_returns_focus_when_hidden() {
        let t = exits_on();
        let mut p = Presence::new(SurfaceKind::Dialog);
        p.open(t.durations.medium, Interaction::Tap, &t, &FINE);
        run(&mut p, 1.0);
        assert!(p.close(Interaction::Tap, &t, &FINE).is_empty());
        assert!(p.in_accessibility_tree());
        let events = run(&mut p, 1.0);
        assert_eq!(
            events,
            vec![PresenceEvent::ReturnFocus, PresenceEvent::Hidden]
        );
    }

    #[test]
    fn reopening_a_leaving_surface_never_hides_it() {
        let t = exits_on();
        let mut p = Presence::new(SurfaceKind::Layer);
        p.open(t.durations.fast, Interaction::Tap, &t, &FINE);
        run(&mut p, 0.5);
        p.close(Interaction::Tap, &t, &FINE);
        p.tick(0.05);
        let mid = p.travel();
        p.open(t.durations.fast, Interaction::Tap, &t, &FINE);
        assert_eq!(p.travel(), mid);
        assert!(p.is_visible());
        let events = run(&mut p, 0.5);
        assert!(events.is_empty());
        assert_eq!(p.phase(), Phase::Open);
    }

    #[test]
    fn reduced_motion_closes_at_once_in_a_pointer_presentation() {
        let t = exits_on();
        let policy = MotionPolicy {
            prefers_reduced_motion: true,
            pointer: Pointer::Fine,
        };
        let mut p = Presence::new(SurfaceKind::Dialog);
        p.open(t.durations.medium, Interaction::Tap, &t, &policy);
        assert_eq!(p.phase(), Phase::Open);
        let events = p.close(Interaction::Tap, &t, &policy);
        assert!(events.contains(&PresenceEvent::Hidden));
    }

    #[test]
    fn touch_exits_play_even_with_the_switches_off() {
        let mut p = Presence::new(SurfaceKind::Layer);
        p.open(ASTRYX.durations.fast, Interaction::Tap, &ASTRYX, &TOUCH);
        run(&mut p, 0.5);
        p.close(Interaction::Tap, &ASTRYX, &TOUCH);
        assert!(p.is_leaving());
        // A tap component's touch exit plays --duration-exit.
        let events = run(&mut p, ASTRYX.durations.exit / 1000.0 + 0.02);
        assert!(events.contains(&PresenceEvent::Hidden));
    }

    #[test]
    fn a_gesture_component_springs_its_touch_exit() {
        let mut p = Presence::new(SurfaceKind::Dialog);
        let sheet = Interaction::Gesture(springs::SETTLE);
        p.open(0.0, sheet, &ASTRYX, &TOUCH);
        run(&mut p, 2.0);
        assert_eq!(p.phase(), Phase::Open);
        p.close(sheet, &ASTRYX, &TOUCH);
        p.tick(0.1);
        assert!(p.travel() < 1.0 && p.travel() > 0.0);
    }

    #[test]
    fn reduced_motion_on_touch_cross_fades_without_travel() {
        let policy = MotionPolicy {
            prefers_reduced_motion: true,
            pointer: Pointer::Coarse,
        };
        let mut p = Presence::new(SurfaceKind::Layer);
        p.open(ASTRYX.durations.fast, Interaction::Tap, &ASTRYX, &policy);
        assert_eq!(p.travel(), 1.0);
        assert_eq!(p.opacity(), 0.0);
        p.tick(0.1);
        assert!(p.opacity() > 0.0 && p.opacity() < 1.0);
        run(&mut p, 0.2);
        assert_eq!(p.phase(), Phase::Open);
        p.close(Interaction::Tap, &ASTRYX, &policy);
        assert_eq!(p.travel(), 1.0);
        let events = run(&mut p, 0.25);
        assert!(events.contains(&PresenceEvent::Hidden));
    }
}
