//! The CPU renderer's frames come at the display's rate when no input asks
//! for them. While the surface wants frames and nothing changes, the window
//! is drawn at most about 60 times a second (Xvfb and weston report 60 Hz),
//! not as fast as the CPU allows; an update from another thread that lands
//! while the surface wants frames is drawn by the next frame, within one
//! frame time and a margin (on Wayland the compositor's frame callbacks set
//! the pace: up to two frame times); and an input is still drawn at once. Needs an X display (Xvfb) or weston. A plain `main` because winit
//! wants the main thread.

#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "ios", target_os = "android"))
))]
mod common;

#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "ios", target_os = "android"))
))]
mod paced {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use astryx_core::{
        A11yNode, Color, Command, DisplayList, DrawCmd, FrameInfo, Input, InputPhase, Radii, Rect,
        Surface, TextMeasure,
    };
    use astryx_desktop::{run_with, Options};
    use astryx_headless::fixtures::{FixtureSurface, FIXTURE_SIZE};

    use super::common::{self, Kind};

    /// The most frames a second while nothing changes: 60 Hz and a margin.
    const MOST_PER_SECOND: f64 = 66.0;
    /// An update is drawn within this time: one 60 Hz frame and a margin;
    /// on Wayland, two.
    const UPDATE_DRAWN: Duration = Duration::from_millis(20);
    const UPDATE_DRAWN_WAYLAND: Duration = Duration::from_millis(40);
    /// An input is drawn within this time.
    const INPUT_DRAWN: Duration = Duration::from_millis(8);
    const UPDATES: usize = 6;
    const TITLE: &str = "Astryx paced frames test";

    #[derive(Default)]
    struct Log {
        /// Frames begun, with when.
        frames: Vec<Instant>,
        /// Per update: when it landed, when the frame drawing it began, and
        /// when that frame was on screen (the tree is read after it).
        updates: Vec<(Instant, Option<Instant>, Option<Instant>)>,
        /// Per input: when it reached the surface, and the next frame.
        inputs: Vec<(Instant, Option<Instant>)>,
        quit: bool,
    }

    /// The gallery fixture with a small square that each update moves, as
    /// a value that arrives replaces its placeholder. It always wants
    /// frames, like a surface waiting on the network.
    struct Waiting {
        page: FixtureSurface,
        landed: usize,
        log: Arc<Mutex<Log>>,
    }

    impl Surface for Waiting {
        fn input(&mut self, input: Input, phase: InputPhase) -> Command {
            if phase == InputPhase::Pressed {
                self.log.lock().unwrap().inputs.push((Instant::now(), None));
            }
            self.page.input(input, phase)
        }
        fn frame(&mut self, frame: &FrameInfo, text: &dyn TextMeasure) -> DisplayList {
            let now = Instant::now();
            let mut log = self.log.lock().unwrap();
            log.frames.push(now);
            for input in &mut log.inputs {
                input.1.get_or_insert(now);
            }
            // Draw every update that has landed.
            let landed = log.updates.len();
            for update in &mut log.updates {
                update.1.get_or_insert(now);
            }
            self.landed = landed;
            drop(log);
            let mut list = self.page.frame(frame, text);
            list.commands.push(DrawCmd::Rect {
                rect: Rect::new(40.0 + 30.0 * self.landed as f32, 60.0, 24.0, 24.0),
                radii: Radii::all(4.0),
                fill: Color::rgba(0.9, 0.1, 0.1, 1.0),
                stroke: None,
            });
            list
        }
        fn accessibility(&self) -> Option<A11yNode> {
            let now = Instant::now();
            for update in &mut self.log.lock().unwrap().updates {
                if update.1.is_some() {
                    update.2.get_or_insert(now);
                }
            }
            self.page.accessibility()
        }
        fn wants_frames(&self) -> bool {
            true
        }
        fn take_command(&mut self) -> Command {
            if self.log.lock().unwrap().quit {
                Command::Quit
            } else {
                Command::None
            }
        }
    }

    fn fail(message: String) -> ! {
        println!("window_paced: FAILED, {message}");
        std::process::exit(1);
    }

    /// This process's CPU time.
    fn cpu_time() -> Duration {
        let stat = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
        // Fields after the command name, which ends with ')'.
        let rest = stat.rsplit(')').next().unwrap_or("");
        let fields: Vec<&str> = rest.split_whitespace().collect();
        let ticks: u64 = fields
            .get(11..13)
            .map_or(0, |f| f.iter().filter_map(|v| v.parse::<u64>().ok()).sum());
        Duration::from_millis(ticks * 10)
    }

    pub fn main() {
        let Some(kind) = common::kind() else {
            println!("window_paced: skipped, no display");
            return;
        };
        std::env::set_var("ASTRYX_RENDERER", "cpu");
        let log = Arc::new(Mutex::new(Log::default()));
        let watch = log.clone();
        let driver = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            // Nothing changes for a second.
            let (count, cpu) = {
                let before = watch.lock().unwrap().frames.len();
                let cpu = cpu_time();
                std::thread::sleep(Duration::from_secs(1));
                (
                    watch.lock().unwrap().frames.len() - before,
                    cpu_time() - cpu,
                )
            };
            // Updates land at times unrelated to the frames.
            for i in 0..UPDATES {
                std::thread::sleep(Duration::from_millis(97 + 13 * i as u64));
                watch
                    .lock()
                    .unwrap()
                    .updates
                    .push((Instant::now(), None, None));
            }
            std::thread::sleep(Duration::from_millis(100));
            // An input is drawn at once (X11: xdotool sends a key).
            let mut keyed = false;
            if kind == Kind::X11 {
                keyed = std::process::Command::new("xdotool")
                    .args([
                        "search",
                        "--name",
                        TITLE,
                        "windowfocus",
                        "--sync",
                        "key",
                        "Tab",
                    ])
                    .status()
                    .is_ok_and(|s| s.success());
                std::thread::sleep(Duration::from_millis(100));
            }
            watch.lock().unwrap().quit = true;
            (count, cpu, keyed)
        });
        let surface = Waiting {
            page: FixtureSurface::new("gallery").expect("fixture"),
            landed: 0,
            log: log.clone(),
        };
        let options = Options {
            title: TITLE.into(),
            size: FIXTURE_SIZE,
            ..Options::default()
        };
        run_with(surface, options).expect("desktop shell runs");
        let (count, cpu, keyed) = driver.join().expect("driver");
        let log = log.lock().unwrap();
        let rate = count as f64;
        let ms = |d: Duration| d.as_secs_f64() * 1e3;
        let drawn: Vec<f64> = log
            .updates
            .iter()
            .map(|(at, drawn, _)| drawn.map_or(f64::INFINITY, |d| ms(d - *at)))
            .collect();
        let shown: Vec<f64> = log
            .updates
            .iter()
            .map(|(at, _, on)| on.map_or(f64::INFINITY, |on| ms(on - *at)))
            .collect();
        let input = log
            .inputs
            .iter()
            .filter_map(|(at, drawn)| drawn.map(|d| ms(d - *at)))
            .fold(0.0f64, f64::max);
        println!(
            "window_paced: {kind:?}: {rate:.0} frames/s while nothing changes, CPU {:.0}% of a core; \
             updates drawn after {drawn:.1?} ms, on screen after {shown:.1?} ms; \
             input drawn after {input:.1} ms{}",
            ms(cpu) / 10.0,
            if keyed { "" } else { " (no input sent)" }
        );
        if rate > MOST_PER_SECOND {
            fail(format!(
                "{rate:.0} frames a second while nothing changes, more than {MOST_PER_SECOND}"
            ));
        }
        let most = match kind {
            Kind::X11 => UPDATE_DRAWN,
            Kind::Wayland => UPDATE_DRAWN_WAYLAND,
        };
        if drawn.len() != UPDATES || drawn.iter().any(|&d| d > ms(most)) {
            fail(format!(
                "an update waited more than {} ms for a frame: {drawn:.1?}",
                ms(most)
            ));
        }
        if shown.iter().any(|s| !s.is_finite()) {
            fail(format!("an update never reached the screen: {shown:.1?}"));
        }
        if keyed && (log.inputs.is_empty() || input > ms(INPUT_DRAWN)) {
            fail(format!(
                "the input was drawn after {input:.1} ms, more than {} ms",
                ms(INPUT_DRAWN)
            ));
        }
        println!("window_paced: ok");
    }
}

fn main() {
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    ))]
    paced::main();
}
