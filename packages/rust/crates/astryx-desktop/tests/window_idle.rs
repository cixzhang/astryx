//! The shell draws only when something changed: an idle window draws no
//! frames, a key draws its effect and stops, and a surface that wants
//! frames (an animation) gets them until it stops wanting them. Needs a
//! display and `xdotool`; run it under Xvfb on headless machines. A plain
//! `main` because winit wants the main thread.

use std::process::Command as Process;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use astryx_core::{
    A11yNode, Command, DisplayList, FrameInfo, Input, InputPhase, Surface, TextMeasure,
};
use astryx_desktop::{run_with, Options};
use astryx_headless::fixtures::{FixtureSurface, FIXTURE_SIZE};

const TITLE: &str = "Astryx idle test";
/// Frames an animation started by Home asks for.
const ANIMATION: u32 = 12;

struct Counting {
    inner: FixtureSurface,
    frames: Arc<AtomicU64>,
    /// Frames still wanted by the animation.
    animating: Arc<AtomicU32>,
}

impl Surface for Counting {
    fn input(&mut self, input: Input, phase: InputPhase) -> Command {
        if phase == InputPhase::Pressed {
            match input {
                Input::B => return Command::Quit,
                Input::Home => self.animating.store(ANIMATION, Ordering::SeqCst),
                _ => {}
            }
        }
        self.inner.input(input, phase)
    }
    fn frame(&mut self, frame: &FrameInfo, text: &dyn TextMeasure) -> DisplayList {
        self.frames.fetch_add(1, Ordering::SeqCst);
        let left = self.animating.load(Ordering::SeqCst);
        self.animating
            .store(left.saturating_sub(1), Ordering::SeqCst);
        self.inner.frame(frame, text)
    }
    fn accessibility(&self) -> Option<A11yNode> {
        self.inner.accessibility()
    }
    fn wants_frames(&self) -> bool {
        self.animating.load(Ordering::SeqCst) > 0
    }
}

fn xdotool(args: &[&str]) -> Option<String> {
    let out = Process::new("xdotool").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// Frames drawn over `ms` of no input.
fn idle_frames(frames: &AtomicU64, ms: u64) -> u64 {
    let before = frames.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(ms));
    frames.load(Ordering::SeqCst) - before
}

fn main() {
    if std::env::var_os("DISPLAY").is_none() {
        println!("window_idle: skipped, no X display");
        return;
    }
    if xdotool(&["version"]).is_none() {
        println!("window_idle: skipped, xdotool not installed");
        return;
    }
    let frames = Arc::new(AtomicU64::new(0));
    let animating = Arc::new(AtomicU32::new(0));
    let report = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    {
        let (frames, report) = (frames.clone(), report.clone());
        std::thread::spawn(move || {
            let id = xdotool(&["search", "--sync", "--onlyvisible", "--name", TITLE])
                .and_then(|ids| ids.lines().next().map(str::to_owned))
                .expect("window appears");
            let _ = xdotool(&["mousemove", "--window", &id, "5", "5"]);
            let _ = xdotool(&["windowfocus", "--sync", &id]);
            // Let the first frames (map, expose, focus) settle.
            std::thread::sleep(Duration::from_millis(700));
            let mut log = report.lock().unwrap();
            let idle = idle_frames(&frames, 1500);
            log.push(format!("idle 1.5 s: {idle} frames"));
            assert_eq!(idle, 0, "an idle window draws nothing");

            let before = frames.load(Ordering::SeqCst);
            xdotool(&["key", "Tab"]).expect("xdotool sends a key");
            std::thread::sleep(Duration::from_millis(400));
            let after_key = frames.load(Ordering::SeqCst) - before;
            log.push(format!("one key: {after_key} frames"));
            assert!(
                (1..=4).contains(&after_key),
                "a key draws its effect once or twice"
            );
            let idle = idle_frames(&frames, 1000);
            log.push(format!("idle after the key: {idle} frames"));
            assert_eq!(idle, 0);

            let before = frames.load(Ordering::SeqCst);
            xdotool(&["key", "Home"]).expect("xdotool sends a key");
            std::thread::sleep(Duration::from_millis(1200));
            let animated = frames.load(Ordering::SeqCst) - before;
            log.push(format!("animation of {ANIMATION}: {animated} frames"));
            assert!(
                animated >= u64::from(ANIMATION) && animated <= u64::from(ANIMATION) + 4,
                "frames while the surface wants them, then none"
            );
            let idle = idle_frames(&frames, 1000);
            log.push(format!("idle after the animation: {idle} frames"));
            assert_eq!(idle, 0);
            drop(log);
            xdotool(&["key", "Escape"]).expect("xdotool sends a key");
        });
    }
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(60));
        eprintln!("window_idle: timed out");
        std::process::exit(1);
    });
    let surface = Counting {
        inner: FixtureSurface::new("gallery").unwrap(),
        frames: frames.clone(),
        animating,
    };
    let options = Options {
        title: TITLE.into(),
        size: FIXTURE_SIZE,
        ..Options::default()
    };
    run_with(surface, options).expect("desktop shell runs");
    let log = report.lock().unwrap();
    assert_eq!(log.len(), 5, "every step ran: {log:?}");
    println!("window_idle: ok, {}", log.join("; "));
}
