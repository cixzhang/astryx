//! A new renderer, whose warm-up frame drew layers and a transition, keeps
//! drawing frames with layers and transitions. Debug builds check the frame
//! arithmetic for overflow.

mod common;

use astryx_core::{
    Color, DisplayList, DrawCmd, Easing, FrameInfo, Radii, Rect, Transition, TransitionKind,
};

fn list(shift: f32) -> DisplayList {
    let rect = |x: f32, y: f32| DrawCmd::Rect {
        rect: Rect::new(x + shift, y, 40.0, 30.0),
        radii: Radii::all(6.0),
        fill: Color::rgba(0.2, 0.4, 0.8, 1.0),
        stroke: None,
    };
    DisplayList {
        clear: Color::rgba(1.0, 1.0, 1.0, 1.0),
        commands: vec![
            DrawCmd::PushOpacity(0.5),
            rect(10.0, 10.0),
            rect(20.0, 20.0),
            rect(30.0, 15.0),
            rect(25.0, 25.0),
            DrawCmd::PopOpacity,
        ],
    }
}

#[test]
fn frames_after_the_warm_up_count_on() {
    let mut gl = match common::OffscreenGl::new() {
        Ok(gl) => gl,
        Err(e) => {
            eprintln!("skipping: {e}");
            return;
        }
    };
    let frame = FrameInfo {
        width: 200,
        height: 150,
        scale: 1.0,
        dt: 1.0 / 60.0,
    };
    gl.renderer.prepare_transitions(200, 150);
    for i in 0..200 {
        if i % 50 == 10 {
            let t = Transition {
                kind: TransitionKind::Fade,
                duration_ms: 200,
                easing: Easing::Linear,
                sound: None,
            };
            gl.renderer.begin_transition(&t, &list(0.0), &frame);
        }
        gl.render(&list((i % 7) as f32), &frame);
    }
}
