//! A surface that changes by itself, such as one fed by a network stream on
//! another thread, says so through `wants_frames()`. The shell asks again
//! while idle, so such a change is drawn within the idle interval, without
//! any input; and an idle window still draws nothing and wakes rarely.
//! Needs a display; run it under Xvfb on headless machines. A plain `main`
//! because winit wants the main thread.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use astryx_core::{
    A11yNode, Command, DisplayList, FrameInfo, Input, InputPhase, Surface, TextMeasure,
};
use astryx_desktop::{run_with, Options};
use astryx_headless::fixtures::{FixtureSurface, FIXTURE_SIZE};

/// Updates from the other thread.
const UPDATES: usize = 5;
/// The longest an update may wait for its frame: the shell's idle interval
/// (100 ms) plus a frame, with room for a loaded test machine.
const MAX_WAIT_MS: f64 = 250.0;

#[derive(Default)]
struct Shared {
    /// An update waits to be drawn.
    pending: AtomicBool,
    frames: AtomicU64,
    /// Calls to wants_frames: one per wakeup of an idle shell.
    asked: AtomicU64,
}

struct Live {
    inner: FixtureSurface,
    shared: Arc<Shared>,
}

impl Surface for Live {
    fn input(&mut self, input: Input, phase: InputPhase) -> Command {
        self.inner.input(input, phase)
    }
    fn frame(&mut self, frame: &FrameInfo, text: &dyn TextMeasure) -> DisplayList {
        self.shared.frames.fetch_add(1, Ordering::SeqCst);
        self.shared.pending.store(false, Ordering::SeqCst);
        self.inner.frame(frame, text)
    }
    fn accessibility(&self) -> Option<A11yNode> {
        self.inner.accessibility()
    }
    fn wants_frames(&self) -> bool {
        self.shared.asked.fetch_add(1, Ordering::SeqCst);
        self.shared.pending.load(Ordering::SeqCst)
    }
}

fn fail(message: String) -> ! {
    println!("window_async: FAILED, {message}");
    std::process::exit(1);
}

fn main() {
    if std::env::var_os("DISPLAY").is_none() {
        println!("window_async: skipped, no X display");
        return;
    }
    let shared = Arc::new(Shared::default());
    let probe = shared.clone();
    std::thread::spawn(move || {
        // The first frames settle.
        std::thread::sleep(Duration::from_millis(1500));
        let (frames, asked) = (
            probe.frames.load(Ordering::SeqCst),
            probe.asked.load(Ordering::SeqCst),
        );
        std::thread::sleep(Duration::from_secs(2));
        let idle_frames = probe.frames.load(Ordering::SeqCst) - frames;
        let wakeups = probe.asked.load(Ordering::SeqCst) - asked;
        if idle_frames != 0 {
            fail(format!("{idle_frames} frames over 2 s of idle"));
        }
        // About 10 a second; more would mean the shell spins.
        if wakeups > 25 {
            fail(format!("{wakeups} wakeups over 2 s of idle"));
        }
        let mut waits = Vec::new();
        for i in 0..UPDATES {
            std::thread::sleep(Duration::from_millis(300 + 70 * i as u64));
            let before = probe.frames.load(Ordering::SeqCst);
            let at = Instant::now();
            probe.pending.store(true, Ordering::SeqCst);
            while probe.frames.load(Ordering::SeqCst) == before
                && at.elapsed() < Duration::from_secs(2)
            {
                std::thread::sleep(Duration::from_millis(1));
            }
            let ms = at.elapsed().as_secs_f64() * 1000.0;
            if probe.frames.load(Ordering::SeqCst) == before {
                fail(format!(
                    "update {} of {UPDATES} was not drawn within 2 s without input",
                    i + 1
                ));
            }
            waits.push(ms);
        }
        let max = waits.iter().cloned().fold(0.0, f64::max);
        if max > MAX_WAIT_MS {
            fail(format!(
                "an update waited {max:.0} ms for its frame ({waits:.0?})"
            ));
        }
        // Drawn, and idle again.
        std::thread::sleep(Duration::from_millis(300));
        let after = probe.frames.load(Ordering::SeqCst);
        std::thread::sleep(Duration::from_secs(1));
        let late = probe.frames.load(Ordering::SeqCst) - after;
        if late != 0 {
            fail(format!("{late} frames over 1 s of idle after the updates"));
        }
        println!(
            "window_async: ok, idle 2 s: 0 frames, {wakeups} wakeups; {UPDATES}/{UPDATES} background updates drawn, waits {waits:.0?} ms (max {max:.0})"
        );
        std::process::exit(0);
    });
    let surface = Live {
        inner: FixtureSurface::new("gallery").unwrap(),
        shared,
    };
    let options = Options {
        title: "Astryx async test".into(),
        size: FIXTURE_SIZE,
        ..Options::default()
    };
    if let Err(e) = run_with(surface, options) {
        fail(format!("the shell failed: {e}"));
    }
}
