use super::*;

fn s(x: f32, y: f32, t: f64) -> Sample {
    Sample { x, y, time_ms: t }
}

#[test]
fn velocity_blends_the_newest_moves_and_rests_to_zero() {
    let mut v = VelocityTracker::new();
    v.add(s(0.0, 0.0, 0.0));
    v.add(s(0.0, 10.0, 10.0));
    v.add(s(0.0, 30.0, 20.0));
    // 0.2 × 2000 + 0.8 × 1000.
    assert!((v.velocity(25.0).y - 1200.0).abs() < 1e-3);
    assert_eq!(v.velocity(80.0).y, 0.0, "rested 60 ms before lifting");
    // Scroll release: 0.25 × newest pan + 0.75 × the pan before it.
    assert!((v.scroll_velocity(25.0).y - (0.25 * 1200.0 + 0.75 * 1000.0)).abs() < 1e-3);
}

#[test]
fn pan_begins_after_hysteresis_and_fails_across_its_axis() {
    let mut p = Pan::new(Axis::Y);
    p.down(s(0.0, 0.0, 0.0));
    assert!(p.moved(s(0.0, 5.0, 8.0)).is_none());
    let e = p.moved(s(0.0, 12.0, 16.0)).unwrap();
    assert_eq!(e.phase, PanPhase::Began);
    assert_eq!(e.translation.y, 12.0);
    assert_eq!(
        p.moved(s(0.0, 40.0, 24.0)).unwrap().phase,
        PanPhase::Changed
    );
    let end = p.up(s(0.0, 60.0, 32.0)).unwrap();
    assert_eq!(end.phase, PanPhase::Ended);
    assert!(end.velocity.y > 1000.0);
    let mut sideways = Pan::new(Axis::Y);
    sideways.down(s(0.0, 0.0, 0.0));
    assert!(sideways.moved(s(15.0, 3.0, 8.0)).is_none());
    assert_eq!(sideways.phase(), PanPhase::Failed);
}

#[test]
fn arena_lets_the_inner_list_scroll_first() {
    let list = |back| Member {
        axis: Axis::Y,
        scroll_room: Some((back, 500.0)),
    };
    let sheet = Member {
        axis: Axis::Y,
        scroll_room: None,
    };
    let down = Point { x: 0.0, y: 12.0 };
    let up = Point { x: 0.0, y: -12.0 };
    assert_eq!(
        arbitrate(&[list(200.0), sheet], down),
        Some(0),
        "list scrolled: it scrolls back"
    );
    assert_eq!(
        arbitrate(&[list(0.0), sheet], down),
        Some(1),
        "list at top: the sheet drags"
    );
    assert_eq!(
        arbitrate(&[list(0.0), sheet], up),
        Some(0),
        "pushing up scrolls the list"
    );
    let row = Member {
        axis: Axis::X,
        scroll_room: None,
    };
    assert_eq!(
        arbitrate(&[row, list(0.0)], Point { x: 14.0, y: 2.0 }),
        Some(0),
        "swipe row"
    );
}

#[test]
fn momentum_coasts_to_its_projection() {
    let m = Momentum::new(0.0, 2000.0);
    let rest = m.rest();
    assert!((rest - 998.0).abs() < 1.0, "{rest}");
    let mut run = m;
    let mut frames = 0;
    while run.step(16.0) {
        frames += 1;
    }
    assert!((run.position - rest).abs() < 1.0);
    assert!(frames > 100 && frames < 400);
}

#[test]
fn rubber_band_resists_and_never_passes_the_dimension() {
    assert_eq!(rubber_band(0.0, 600.0), 0.0);
    assert!(rubber_band(100.0, 600.0) < 100.0 && rubber_band(100.0, 600.0) > 40.0);
    assert!(rubber_band(1e6, 600.0) < 600.0);
    assert_eq!(rubber_band(-50.0, 600.0), -rubber_band(50.0, 600.0));
}

#[test]
fn snap_points_follow_astryx_release_rules() {
    let points = [0.0, 240.0, 480.0];
    assert_eq!(resolve_snap(&points, 100.0, 0.0, None), 0, "nearest");
    assert_eq!(
        resolve_snap(&points, 100.0, 1200.0, None),
        1,
        "a flick carries on"
    );
    assert_eq!(
        resolve_snap(&points, 230.0, -1500.0, None),
        0,
        "a flick back"
    );
    assert_eq!(
        resolve_snap(&points, 200.0, -300.0, Some(0.0)),
        0,
        "moving back returns to the start"
    );
    assert_eq!(resolve_snap(&points, 200.0, 300.0, Some(0.0)), 1);
}

#[test]
fn reorder_by_keyboard_announces_and_by_pointer_drops_in_place() {
    let mut r = Reorder::new(4);
    assert_eq!(r.toggle_grab(2), "Grabbed item 3 of 4.");
    assert_eq!(r.step(true).as_deref(), Some("Moved to position 2 of 4."));
    assert_eq!(r.step(true).as_deref(), Some("Moved to position 1 of 4."));
    assert_eq!(r.step(true), None, "at the top");
    assert_eq!(r.toggle_grab(0), "Dropped at position 1 of 4.");
    assert_eq!(r.order, vec![2, 0, 1, 3]);
    let mut p = Reorder::new(4);
    assert_eq!(p.drag_to(2, 5.0, 0.0, 40.0), 0, "row 3 to the top");
    assert_eq!(p.order, vec![2, 0, 1, 3]);
}

// The ledger's Gestures exercise through the core runtime: a sheet with a
// scrolling list.

mod sheet {
    use crate::runtime::{Drag, SnapDrag};
    use crate::{Axis, PanEvent, PanPhase};
    use astryx_core::testing::{frame, FixedMeasure};
    use astryx_core::{
        A11yNode, App, Command, Element, Point, PointerEvent, PointerKind, PointerPhase, Runtime,
        SizeValue, Surface, Text, VStack,
    };

    struct Sheet {
        snap: SnapDrag,
        events: Vec<PanPhase>,
    }

    #[derive(Clone)]
    enum Msg {
        Pan(PanEvent),
    }

    impl App for Sheet {
        type Msg = Msg;
        fn view(&self) -> Element<Msg> {
            let list = VStack::new()
                .id("list")
                .is_scrollable(true)
                .height(SizeValue::Points(200.0))
                .children((0..30).map(|i| Text::new(format!("Row {i}"))));
            VStack::new()
                .child(
                    Drag::new(
                        VStack::new().child(Text::new("Handle")).child(list),
                        Axis::Y,
                        |e| Some(Msg::Pan(e)),
                    )
                    .id("sheet"),
                )
                .into()
        }
        fn update(&mut self, msg: Msg) -> Command {
            let Msg::Pan(e) = msg;
            self.events.push(e.phase);
            self.snap.pan(&e);
            Command::None
        }
    }

    fn rt() -> Runtime<Sheet> {
        let mut rt = Runtime::new(Sheet {
            snap: SnapDrag::new(vec![0.0, 240.0, 480.0], 0),
            events: vec![],
        });
        step(&mut rt, 0.0);
        rt
    }

    fn step(rt: &mut Runtime<Sheet>, ms: f32) {
        let mut f = frame(400, 600);
        f.dt = ms / 1000.0;
        rt.frame(&f, &FixedMeasure::default());
    }

    fn row_y(rt: &Runtime<Sheet>, label: &str) -> f32 {
        fn find<'a>(n: &'a A11yNode, l: &str) -> Option<&'a A11yNode> {
            if n.label.as_deref() == Some(l) {
                return Some(n);
            }
            n.children.iter().find_map(|c| find(c, l))
        }
        find(&rt.accessibility().unwrap(), label).unwrap().bounds.y
    }

    /// Drags from (x, y0) to (x, y1) in `steps` moves of 16 ms, then lifts.
    fn drag(
        rt: &mut Runtime<Sheet>,
        kind: PointerKind,
        y0: f32,
        y1: f32,
        steps: u32,
        t0: f64,
    ) -> f64 {
        let e = |phase, y: f32, t: f64| PointerEvent {
            id: 7,
            kind,
            phase,
            at: Point { x: 40.0, y },
            button: 0,
            time_ms: t,
        };
        rt.pointer(e(PointerPhase::Down, y0, t0));
        let mut t = t0;
        for i in 1..=steps {
            t += 16.0;
            let y = y0 + (y1 - y0) * i as f32 / steps as f32;
            rt.pointer(e(PointerPhase::Move, y, t));
            step(rt, 16.0);
        }
        rt.pointer(e(PointerPhase::Up, y1, t + 4.0));
        step(rt, 16.0);
        t + 4.0
    }

    #[test]
    fn the_list_scrolls_first_and_the_sheet_drags_from_the_list_top() {
        let mut r = rt();
        let top = row_y(&r, "Row 0");
        // Push up slowly inside the list: the list scrolls; the sheet does not move.
        drag(&mut r, PointerKind::Touch, 200.0, 100.0, 10, 0.0);
        let scrolled = row_y(&r, "Row 0");
        // 100 points under the finger, plus a little coast after the lift.
        assert!(
            top - scrolled >= 99.0 && top - scrolled < 160.0,
            "list scrolled: {top} -> {scrolled}"
        );
        assert!(r.app().events.is_empty(), "the sheet got nothing");
        // Pull down: the list scrolls back first, to its top.
        drag(&mut r, PointerKind::Touch, 100.0, 300.0, 10, 1000.0);
        assert!((row_y(&r, "Row 0") - top).abs() < 1.0, "back at the top");
        assert!(r.app().events.is_empty());
        // At the top, pulling down drags the sheet; a fast release flicks on.
        drag(&mut r, PointerKind::Touch, 100.0, 180.0, 4, 2000.0);
        assert_eq!(r.app().events.first(), Some(&PanPhase::Began));
        assert_eq!(r.app().events.last(), Some(&PanPhase::Ended));
        assert_eq!(
            r.app().snap.index,
            1,
            "release velocity picked the next snap point"
        );
    }

    #[test]
    fn a_slow_release_settles_on_the_nearest_snap_point() {
        let mut r = rt();
        drag(&mut r, PointerKind::Touch, 100.0, 140.0, 20, 0.0);
        assert_eq!(r.app().snap.index, 0, "40 points of a slow drag returns");
    }

    #[test]
    fn a_throw_coasts_with_momentum() {
        let mut r = rt();
        let top = row_y(&r, "Row 0");
        drag(&mut r, PointerKind::Touch, 400.0, 340.0, 3, 0.0);
        let released = row_y(&r, "Row 0");
        for _ in 0..30 {
            step(&mut r, 16.0);
        }
        let coasted = row_y(&r, "Row 0");
        assert!(top - released >= 59.0);
        assert!(
            coasted < released - 20.0,
            "kept scrolling after the lift: {released} -> {coasted}"
        );
    }

    #[test]
    fn a_mouse_drag_does_not_scroll_the_list_but_drags_the_sheet() {
        let mut r = rt();
        let top = row_y(&r, "Row 0");
        drag(&mut r, PointerKind::Mouse, 200.0, 120.0, 8, 0.0);
        assert_eq!(row_y(&r, "Row 0"), top);
        assert_eq!(r.app().events.first(), Some(&PanPhase::Began));
    }
}
