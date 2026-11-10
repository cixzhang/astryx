//! Frame cost of a fling through 10,000 windowed rows, and of a sheet drag:
//! `cargo run --release -p astryx-spike-infra-gallery --example fling`.
//! Prints the p50 and p95 of the runtime's frame (build, layout, a11y and
//! paint; rasterizing is the renderer's and is measured by the QA harness).

use astryx_core::contract::{Point, PointerEvent, PointerKind, PointerPhase, Surface};
use astryx_core::Runtime;
use astryx_spike_infra_gallery::Gallery;
use std::time::Instant;

fn percentiles(mut ms: Vec<f64>) -> (f64, f64) {
    ms.sort_by(f64::total_cmp);
    let at = |p: f64| ms[((ms.len() as f64 - 1.0) * p).round() as usize];
    (at(0.5), at(0.95))
}

fn main() {
    let renderer = astryx_render_soft::SoftRenderer::new();
    let frame = astryx_headless::frame_info(640, 560, 1.0);
    let mut step = frame;
    step.dt = 1.0 / 60.0;

    let mut rt = Runtime::new(Gallery::case("infra-virtual-10k").expect("case"));
    let _ = rt.frame(&frame, renderer.text());
    // A touch fling: a fast drag up, then the coast.
    let mut t = 0.0;
    let down = |phase, y: f32, t: f64| PointerEvent {
        id: 1,
        kind: PointerKind::Touch,
        phase,
        at: Point { x: 100.0, y },
        button: 0,
        time_ms: t,
    };
    rt.pointer(down(PointerPhase::Down, 500.0, t));
    let mut times = Vec::new();
    for i in 1..=10 {
        t += 16.0;
        rt.pointer(down(PointerPhase::Move, 500.0 - 40.0 * i as f32, t));
        let s = Instant::now();
        let _ = rt.frame(&step, renderer.text());
        times.push(s.elapsed().as_secs_f64() * 1000.0);
    }
    rt.pointer(down(PointerPhase::Up, 100.0, t + 4.0));
    let mut frames = 0;
    while rt.wants_frames() && frames < 600 {
        let s = Instant::now();
        let _ = rt.frame(&step, renderer.text());
        times.push(s.elapsed().as_secs_f64() * 1000.0);
        frames += 1;
    }
    let (p50, p95) = percentiles(times.clone());
    println!(
        "fling_10k frames={} p50_ms={p50:.2} p95_ms={p95:.2}",
        times.len()
    );

    let mut rt = Runtime::new(Gallery::case("infra-gestures-sheet").expect("case"));
    let _ = rt.frame(&frame, renderer.text());
    let handle_y = |rt: &Runtime<Gallery>| {
        fn find(n: &astryx_core::A11yNode) -> Option<f32> {
            if n.label.as_deref() == Some("Drag the sheet") {
                return Some(n.bounds.y);
            }
            n.children.iter().find_map(find)
        }
        rt.accessibility()
            .as_ref()
            .and_then(find)
            .unwrap_or(f32::NAN)
    };
    let before = handle_y(&rt);
    let mut times = Vec::new();
    let mut t = 0.0;
    rt.pointer(down(PointerPhase::Down, 280.0, t));
    for i in 1..=60 {
        t += 16.0;
        let y = 280.0 + 3.0 * i as f32;
        rt.pointer(PointerEvent {
            at: Point { x: 100.0, y },
            ..down(PointerPhase::Move, y, t)
        });
        let s = Instant::now();
        let _ = rt.frame(&step, renderer.text());
        times.push(s.elapsed().as_secs_f64() * 1000.0);
    }
    let (p50, p95) = percentiles(times);
    let after = handle_y(&rt);
    println!(
        "sheet_drag frames=60 moved_pt={:.0} p50_ms={p50:.2} p95_ms={p95:.2}",
        after - before
    );
}
