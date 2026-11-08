//! Light elements: Lit wraps its subtree in one light pass, lights resolve
//! from layout and are left out when off screen, and time comes from the
//! runtime, still under reduced motion.

mod common;

use astryx_core::paged::CameraFollow;
use astryx_core::world::{Board, Emissive, LightSource, Lit};
use astryx_core::*;
use common::*;

fn lamp(flicker: f32) -> LightSource {
    LightSource::new(Light {
        center: Point { x: 0.0, y: 0.0 },
        radius_x: 20.0,
        radius_y: 20.0,
        color: Color::rgba(1.0, 0.8, 0.5, 1.0),
        intensity: 1.0,
        flicker: Flicker {
            amplitude: flicker,
            hz: 2.0,
        },
        pulse: Pulse::default(),
    })
}

fn night(drift: f32) -> Lighting {
    Lighting {
        ambient: Color::rgba(0.1, 0.1, 0.3, 1.0),
        strength: 0.8,
        time: 123.0,
        dapple: Some(Dapple {
            cell: 20.0,
            velocity: Point { x: drift, y: 0.0 },
            bands: 3,
            depth: 0.3,
        }),
    }
}

fn frame(rt: &mut Runtime<View<()>>, dt: f32) -> DisplayList {
    rt.frame(
        &FrameInfo {
            width: 400,
            height: 300,
            scale: 1.0,
            dt,
        },
        &measure(),
    )
}

#[test]
fn lit_wraps_its_children_and_places_lights_from_layout() {
    let list = paint(|| {
        VStack::<()>::new()
            .child(
                Lit::new(night(0.0)).child(
                    Board::new(400.0, 200.0)
                        .child_at(50.0, 60.0, lamp(0.0))
                        .child_at(
                            10.0,
                            20.0,
                            Emissive::new(Color::rgba(1.0, 1.0, 0.0, 1.0)).size(6.0, 4.0),
                        ),
                ),
            )
            .child(Text::new("Unlit label"))
            .into()
    });
    let at = |p: &dyn Fn(&DrawCmd) -> bool| list.commands.iter().position(p).unwrap();
    let push = at(&|c| matches!(c, DrawCmd::PushLighting(_)));
    let pop = at(&|c| matches!(c, DrawCmd::PopLighting));
    let light = at(&|c| matches!(c, DrawCmd::Light(_)));
    let glow = at(&|c| matches!(c, DrawCmd::Emissive { .. }));
    let label = at(&|c| matches!(c, DrawCmd::Text { text, .. } if text == "Unlit label"));
    assert!(push < light && light < pop && push < glow && glow < pop && pop < label);
    let DrawCmd::Light(l) = &list.commands[light] else {
        unreachable!()
    };
    assert_eq!((l.center.x, l.center.y), (50.0, 60.0));
    let DrawCmd::Emissive { rect, .. } = &list.commands[glow] else {
        unreachable!()
    };
    assert_eq!(*rect, Rect::new(10.0, 20.0, 6.0, 4.0));
    let DrawCmd::PushLighting(lit) = &list.commands[push] else {
        unreachable!()
    };
    assert_eq!(lit.time, 0.0, "nothing drifts, so time holds still");
}

#[test]
fn lights_off_screen_cost_nothing() {
    let mut rt = runtime(|| {
        let mut board = Board::new(4000.0, 300.0);
        for i in 0..40 {
            board = board.child_at(i as f32 * 100.0 + 50.0, 150.0, lamp(0.0));
        }
        Lit::new(night(0.0)).child(CameraFollow::new(board)).into()
    });
    let list = frame(&mut rt, 0.0);
    let lights = list
        .commands
        .iter()
        .filter(|c| matches!(c, DrawCmd::Light(_)))
        .count();
    // A 400-point view with lights every 100 points, each reaching 20.
    assert!((4..=5).contains(&lights), "{lights} lights painted");
}

#[test]
fn time_comes_from_the_runtime_and_stops_under_reduced_motion() {
    let mut rt = runtime(|| {
        Lit::new(night(5.0))
            .child(Board::new(100.0, 100.0).child_at(10.0, 10.0, lamp(0.1)))
            .into()
    });
    frame(&mut rt, 0.0);
    let list = frame(&mut rt, 0.5);
    assert!(
        rt.wants_frames(),
        "drifting shade and a flickering lamp animate"
    );
    let t = list.commands.iter().find_map(|c| match c {
        DrawCmd::PushLighting(l) => Some(l.time),
        _ => None,
    });
    assert!(t.is_some_and(|t| t > 0.0 && t != 123.0), "{t:?}");
    rt.set_preferences(&Preferences {
        reduced_motion: true,
        ..Preferences::default()
    });
    let list = frame(&mut rt, 0.5);
    assert!(!rt.wants_frames());
    for c in &list.commands {
        match c {
            DrawCmd::PushLighting(l) => assert_eq!(l.time, 0.0),
            DrawCmd::Light(l) => assert_eq!(l.flicker, Flicker::default()),
            _ => {}
        }
    }
}
