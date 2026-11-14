//! The renderer's warm-up (every draw kind, offscreen) runs after the first
//! frame, not while it is made, so an app's first frame shows sooner; and a
//! renderer whose warm-up drew layers and a transition keeps drawing frames
//! with layers and transitions. Debug builds check the frame arithmetic for
//! overflow.

mod common;

use astryx_core::{
    Color, DisplayList, DrawCmd, Easing, FontFamily, FrameInfo, Radii, Rect, ShadowLayer,
    TextAlign, TextStyle, Transition, TransitionKind,
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

fn small_frame() -> FrameInfo {
    FrameInfo {
        width: 64,
        height: 64,
        scale: 1.0,
        dt: 0.0,
    }
}

/// The shadow the warm-up draws.
fn warm_up_shadow() -> DisplayList {
    let red = Color::rgba(1.0, 0.0, 0.0, 1.0);
    DisplayList {
        clear: Color::rgba(1.0, 1.0, 1.0, 1.0),
        commands: vec![DrawCmd::Shadow {
            rect: Rect::new(10.0, 10.0, 24.0, 24.0),
            radii: Radii::all(6.0),
            layers: vec![ShadowLayer {
                x: 0.0,
                y: 2.0,
                blur: 4.0,
                spread: 0.0,
                inset: false,
                color: red,
            }],
        }],
    }
}

fn label() -> DisplayList {
    DisplayList {
        clear: Color::rgba(1.0, 1.0, 1.0, 1.0),
        commands: vec![DrawCmd::Text {
            rect: Rect::new(4.0, 4.0, 56.0, 20.0),
            text: "Hi".into(),
            style: TextStyle {
                family: FontFamily::Body,
                size: 16.0,
                weight: 500,
                leading: 1.4,
                direction: None,
            },
            color: Color::rgba(0.1, 0.1, 0.1, 1.0),
            align: TextAlign::Start,
        }],
    }
}

#[test]
fn the_first_frame_is_drawn_before_the_warm_up_and_the_second_after_it() {
    let mut gl = match common::OffscreenGl::new() {
        Ok(gl) => gl,
        Err(e) => {
            println!("warm_up: skipped, no EGL device: {e}");
            return;
        }
    };
    // First frame: the warm-up has not drawn its shadow yet, so this frame
    // computes it.
    gl.render(&warm_up_shadow(), &small_frame());
    assert_eq!(
        gl.renderer.last_stats().shadows,
        1,
        "the first frame comes before the warm-up"
    );
    // The second frame runs the warm-up first, and still draws exactly
    // what a renderer that never warmed up draws.
    let after = gl.render(&label(), &small_frame());
    gl.reset_renderer();
    let fresh = gl.render(&label(), &small_frame());
    assert_eq!(
        after, fresh,
        "the warm-up leaves no trace in the frame after it"
    );
}
