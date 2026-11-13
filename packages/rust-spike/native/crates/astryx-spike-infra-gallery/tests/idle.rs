//! Shells draw only while a surface wants frames. Every infra and UI
//! gallery screen settles and then wants none; each animation and timer
//! wants frames until it has played, then stops.

use astryx_core::contract::{FrameInfo, Surface};
use astryx_core::testing::FixedMeasure;
use astryx_core::Runtime;

const DT: f32 = 1.0 / 60.0;

fn frame(rt: &mut dyn Surface, dt: f32) {
    let _ = rt.frame(
        &FrameInfo {
            width: 640,
            height: 560,
            scale: 1.0,
            dt,
        },
        &FixedMeasure::default(),
    );
}

/// Frames until the surface stops wanting them; how many it took.
fn settle(rt: &mut dyn Surface, what: &str) -> usize {
    for n in 0..1200 {
        if !rt.wants_frames() {
            return n;
        }
        frame(rt, DT);
    }
    panic!("{what}: still wants frames after 20 s");
}

/// Settled, it stays settled with no input.
fn assert_idle(rt: &mut dyn Surface, what: &str) {
    settle(rt, what);
    for _ in 0..30 {
        frame(rt, DT);
        assert!(!rt.wants_frames(), "{what}: wants frames while idle");
    }
}

#[test]
fn every_infra_screen_settles_and_stays_idle() {
    for id in astryx_spike_infra_gallery::CASES {
        let mut rt = Runtime::new(astryx_spike_infra_gallery::Gallery::case(id).unwrap());
        frame(&mut rt, DT);
        assert_idle(&mut rt, id);
    }
}

/// A screen that may show a spinner (the Spinner family, or a control
/// with `isLoading`) animates for as long as it shows; every other screen
/// settles.
fn may_show_a_spinner(case: &astryx_spike_gallery::families::Case) -> bool {
    case.family == "Spinner" || case.props.get("isLoading") == Some(&serde_json::Value::Bool(true))
}

#[test]
fn every_ui_screen_settles_and_stays_idle_unless_it_shows_a_spinner() {
    let mut wrong = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let (mut idle, mut spinning) = (0, 0);
    for family in astryx_spike_gallery::families::all() {
        for case in astryx_spike_gallery::families::cases(&family) {
            if !seen.insert(case.id.clone()) {
                continue;
            }
            let mut rt = Runtime::new(astryx_spike_gallery::Gallery::case(&case.id).unwrap());
            frame(&mut rt, DT);
            for _ in 0..600 {
                if !rt.wants_frames() {
                    break;
                }
                frame(&mut rt, DT);
            }
            let busy = rt.wants_frames();
            if busy && !may_show_a_spinner(&case) {
                wrong.push((case.id.clone(), busy));
            }
            if busy {
                spinning += 1;
                continue;
            }
            idle += 1;
            for _ in 0..30 {
                frame(&mut rt, DT);
                if rt.wants_frames() {
                    wrong.push((case.id.clone(), true));
                    break;
                }
            }
        }
    }
    eprintln!("UI screens: {idle} settle and stay idle, {spinning} spin");
    assert!(wrong.is_empty(), "screens that never settle: {wrong:?}");
}

// ---- Each animation and timer wants frames until it has played ----

use astryx_core::contract::{
    A11yAction, A11yNode, InputPhase, Key, KeyEvent, Modifiers, Point, PointerEvent, PointerKind,
    PointerPhase, Preferences, Role,
};
use astryx_spike_infra_gallery::Gallery;

fn screen(id: &str) -> Runtime<Gallery> {
    let mut rt = Runtime::new(Gallery::case(id).unwrap());
    frame(&mut rt, DT);
    settle(&mut rt, id);
    rt
}

fn find<'a>(n: &'a A11yNode, test: &dyn Fn(&A11yNode) -> bool) -> Option<&'a A11yNode> {
    if test(n) {
        return Some(n);
    }
    n.children.iter().find_map(|c| find(c, test))
}

fn labeled(rt: &Runtime<Gallery>, label: &str) -> Option<A11yNode> {
    find(&rt.accessibility()?, &|n| n.label.as_deref() == Some(label)).cloned()
}

fn press(rt: &mut Runtime<Gallery>, label: &str) {
    let id = labeled(rt, label)
        .unwrap_or_else(|| panic!("no {label}"))
        .id;
    rt.a11y_action(id, A11yAction::PressStart);
    rt.a11y_action(id, A11yAction::PressEnd);
}

/// Plays the animation an input started: it wants frames now, and stops
/// within the bound. Returns how many frames it took.
fn plays(rt: &mut Runtime<Gallery>, what: &str) -> usize {
    assert!(rt.wants_frames(), "{what}: wants a frame when it starts");
    let n = settle(rt, what);
    assert_idle(rt, what);
    n
}

fn touch(rt: &mut Runtime<Gallery>, phase: PointerPhase, at: Point, t: f64) {
    rt.pointer(PointerEvent {
        id: 1,
        kind: PointerKind::Touch,
        phase,
        at,
        button: 0,
        time_ms: t,
    });
}

/// A finger drag from `from` by `dy` over `steps` 16 ms moves, released
/// at once (a flick).
fn flick(rt: &mut Runtime<Gallery>, from: Point, dy: f32, steps: u32) {
    let mut t = 1000.0;
    touch(rt, PointerPhase::Down, from, t);
    for i in 1..=steps {
        t += 16.0;
        let at = Point {
            x: from.x,
            y: from.y + dy * i as f32 / steps as f32,
        };
        touch(rt, PointerPhase::Move, at, t);
        frame(rt, DT);
    }
    let end = Point {
        x: from.x,
        y: from.y + dy,
    };
    touch(rt, PointerPhase::Up, end, t + 4.0);
}

fn center(n: &A11yNode) -> Point {
    Point {
        x: n.bounds.x + n.bounds.width / 2.0,
        y: n.bounds.y + n.bounds.height / 2.0,
    }
}

#[test]
fn layer_entrances_play_on_the_first_frames_then_stop() {
    for id in ["infra-layers-popover-dialog", "infra-layers-context-menu"] {
        let mut rt = Runtime::new(Gallery::case(id).unwrap());
        frame(&mut rt, DT);
        let n = settle(&mut rt, id);
        assert!(n >= 5, "{id}: the entrance plays over frames ({n})");
        assert_idle(&mut rt, id);
    }
}

#[test]
fn a_popover_exit_plays_once_then_stops() {
    let mut rt = screen("infra-layers-popover-dialog");
    let key = |phase| KeyEvent {
        key: Key::Escape,
        phase,
        modifiers: Modifiers::default(),
        repeat: false,
    };
    rt.key(key(InputPhase::Pressed));
    rt.input(astryx_core::contract::Input::B, InputPhase::Pressed);
    rt.input(astryx_core::contract::Input::B, InputPhase::Released);
    rt.key(key(InputPhase::Released));
    frame(&mut rt, DT);
    assert!(labeled(&rt, "Duplicate").is_none(), "the popover closed");
    let n = plays(&mut rt, "popover exit");
    assert!(n >= 3, "the exit copy fades over frames ({n})");
}

#[test]
fn a_spring_toggle_plays_retargets_and_stops() {
    let mut rt = screen("infra-motion-switch");
    press(&mut rt, "Turn on");
    let full = plays(&mut rt, "toggle on");
    assert!(
        full >= 10,
        "the stateChange spring plays over frames ({full})"
    );
    // Reverse mid-way: it keeps wanting frames from where it is.
    press(&mut rt, "Turn off");
    for _ in 0..4 {
        frame(&mut rt, DT);
    }
    press(&mut rt, "Turn on");
    let n = plays(&mut rt, "retarget");
    assert!(n >= 5, "the retarget plays ({n})");
    // Reduce Motion: travel lands at once, so far fewer frames.
    rt.set_preferences(&Preferences {
        reduced_motion: true,
        ..Preferences::default()
    });
    settle(&mut rt, "preferences");
    press(&mut rt, "Turn off");
    let n = plays(&mut rt, "reduced toggle");
    assert!(n < full, "Reduce Motion: {n} frames, full motion {full}");
}

#[test]
fn a_collapsible_height_plays_and_reflows_then_stops() {
    let mut rt = screen("infra-motion-collapsible");
    let below = |rt: &Runtime<Gallery>| labeled(rt, "Content below reflows").unwrap().bounds.y;
    // The screen starts expanded.
    press(&mut rt, "Collapse");
    plays(&mut rt, "collapse");
    let start = below(&rt);
    press(&mut rt, "Expand");
    frame(&mut rt, DT);
    frame(&mut rt, DT);
    let mid = below(&rt);
    let n = plays(&mut rt, "expand");
    let end = below(&rt);
    assert!(n >= 10, "the height animates over frames ({n})");
    assert!(
        start < mid && mid < end,
        "content below reflows: {start} {mid} {end}"
    );
}

/// A button whose tooltip opens on hover intent (Astryx's delays).
struct Tip {
    shown: bool,
}

impl astryx_core::App for Tip {
    type Msg = bool;
    fn view(&self) -> astryx_core::Element<bool> {
        let mut page = astryx_core::VStack::new()
            .padding(astryx_core::Space::S10)
            .child(astryx_spike_layers::Hover::new(
                astryx_core::Button::new("Save").id("save"),
                |on| on,
            ));
        if self.shown {
            page = page.child(astryx_spike_layers::Layer::tooltip(
                "save",
                astryx_core::Text::new("Save your changes").into(),
            ));
        }
        page.into()
    }
    fn update(&mut self, on: bool) -> astryx_core::Command {
        self.shown = on;
        astryx_core::Command::None
    }
}

#[test]
fn hover_intent_and_tooltip_wait_with_frames_then_stop() {
    // Keyboard focus would show the tooltip at once: test the pointer.
    let mut rt = Runtime::new(Tip { shown: false }).hide_focus_until_keyboard();
    frame(&mut rt, DT);
    assert_idle(&mut rt, "tooltip screen");
    let save = find(&rt.accessibility().unwrap(), &|n| {
        n.label.as_deref() == Some("Save")
    })
    .cloned()
    .unwrap();
    let tip = |rt: &Runtime<Tip>| {
        rt.accessibility()
            .and_then(|t| find(&t, &|n| n.role == Role::Tooltip).cloned())
    };
    rt.pointer_move(Some(center(&save)));
    assert!(
        rt.wants_frames(),
        "the hover delay is a timer: it wants frames"
    );
    let n = settle(&mut rt, "hover open");
    assert!(
        n >= 10,
        "the open delay and entrance play over frames ({n})"
    );
    assert!(tip(&rt).is_some(), "the tooltip opened after the delay");
    assert_idle(&mut rt, "tooltip open");
    rt.pointer_move(None);
    assert!(rt.wants_frames(), "the leave delay is a timer too");
    settle(&mut rt, "hover close");
    assert!(
        tip(&rt).is_none(),
        "the tooltip closed after the leave delay"
    );
    assert_idle(&mut rt, "tooltip closed");
}

#[test]
fn a_sheet_flick_settles_on_the_spring_rubber_bands_and_stops() {
    let mut rt = screen("infra-gestures-sheet");
    let handle = |rt: &Runtime<Gallery>| labeled(rt, "Drag the sheet").unwrap();
    let start = handle(&rt).bounds.y;
    let from = center(&handle(&rt));
    flick(&mut rt, from, 120.0, 6);
    frame(&mut rt, DT);
    let released = handle(&rt).bounds.y;
    let n = plays(&mut rt, "sheet settle");
    let rest = handle(&rt).bounds.y;
    assert!(n >= 10, "the settle spring plays over frames ({n})");
    assert!(
        rest > start,
        "a downward flick moves the sheet down: {start} -> {rest}"
    );
    assert!(
        (released - rest).abs() > 1.0,
        "it eases onto the snap point, no jump: released at {released}, rests at {rest}"
    );
    // The sheet now rests 344 below its top snap point: a finger 480 up
    // overshoots the top by 136, which rubber bands.
    let top = handle(&rt).bounds.y;
    let from = center(&handle(&rt));
    let mut t = 5000.0;
    touch(&mut rt, PointerPhase::Down, from, t);
    for i in 1..=12 {
        t += 16.0;
        let at = Point {
            x: from.x,
            y: from.y - 40.0 * i as f32,
        };
        touch(&mut rt, PointerPhase::Move, at, t);
        frame(&mut rt, DT);
    }
    let pulled = handle(&rt).bounds.y;
    assert!(
        pulled > top - 480.0 && pulled < top - 344.0,
        "past the end it moves less than the finger: {top} -> {pulled}"
    );
    t += 200.0;
    let end = Point {
        x: from.x,
        y: from.y - 480.0,
    };
    touch(&mut rt, PointerPhase::Up, end, t);
    let n = plays(&mut rt, "rubber band back");
    assert!(n >= 10, "it springs back over frames ({n})");
}

#[test]
fn a_fling_through_10k_rows_coasts_then_stops() {
    let mut rt = screen("infra-virtual-10k");
    let first_row = |rt: &Runtime<Gallery>| {
        rt.accessibility()
            .and_then(|t| find(&t, &|n| n.role == Role::Row).map(|n| n.state.row_index))
            .flatten()
            .unwrap_or(0)
    };
    let grid = rt
        .accessibility()
        .and_then(|t| find(&t, &|n| n.role == Role::Grid).cloned())
        .unwrap();
    flick(&mut rt, center(&grid), -240.0, 6);
    let released = first_row(&rt);
    let n = plays(&mut rt, "fling coast");
    assert!(n >= 10, "the fling coasts over frames ({n})");
    assert!(
        first_row(&rt) > released,
        "rows move on after the release: {released} -> {}",
        first_row(&rt)
    );
}
