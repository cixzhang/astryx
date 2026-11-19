//! The window behaves the same whichever way it is drawn: OpenGL on the main
//! thread (`ASTRYX_RENDERER=gl`), the CPU renderer first and then OpenGL on
//! its own thread (`cpu-first`, the default on an X server with a GPU), and
//! the CPU renderer only (`cpu`). Each way runs in a child process (one
//! event loop per process) and must show:
//! - a first picture, then never anything but that picture within
//!   antialiasing (no blank or partial frame, no flash at the switch from
//!   the CPU renderer to OpenGL); with `cpu-first`, the first picture is the
//!   CPU renderer's exactly, drawn before any GL library loads, and OpenGL
//!   takes over;
//! - no frames while idle; exactly the frames a surface asks for; a
//!   transition drawn over several frames, then none;
//! - the accessibility tree read once after each presented frame;
//! - on quit, no thread left behind (OpenGL's, or the CPU renderer's
//!   painting threads).
//!
//! Needs an X display with OpenGL (Xvfb has llvmpipe). A plain `main`
//! because winit wants the main thread.

#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "ios", target_os = "android"))
))]
mod x11 {
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use astryx_core::{
        A11yNode, Command, DisplayList, Easing, FrameInfo, Input, InputPhase, Surface, TextMeasure,
        Transition, TransitionKind,
    };
    use astryx_desktop::{run_with, Options};
    use astryx_headless::fixtures::{FixtureSurface, FIXTURE_SIZE};
    use astryx_headless::{astryx_render_soft::SoftRenderer, frame_info};
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{ConnectionExt, ImageFormat};

    const MODES: [&str; 3] = ["gl", "cpu-first", "cpu"];
    const ANIMATION: u32 = 12;

    /// What the test asks of the surface, after nothing (0).
    const TRANSITION: u8 = 1;
    const QUIT: u8 = 2;

    #[derive(Default)]
    struct Shared {
        ask: AtomicU8,
        frames: AtomicU64,
        /// Accessibility reads that followed a frame.
        after_frame: AtomicU64,
        drawn: AtomicBool,
        /// Frames still wanted by an animation.
        animate: AtomicU32,
        /// A GL library was loaded when the first frame was presented.
        gl_at_first: AtomicU8,
    }

    struct Probe {
        inner: FixtureSurface,
        shared: Arc<Shared>,
        transition_sent: bool,
    }

    fn gl_loaded() -> bool {
        let maps = std::fs::read_to_string("/proc/self/maps").unwrap_or_default();
        maps.lines().any(|l| {
            let name = l.rsplit('/').next().unwrap_or("");
            name.starts_with("libGL") || name.starts_with("libEGL")
        })
    }

    impl Surface for Probe {
        fn input(&mut self, input: Input, phase: InputPhase) -> Command {
            self.inner.input(input, phase)
        }
        fn frame(&mut self, frame: &FrameInfo, text: &dyn TextMeasure) -> DisplayList {
            let s = &self.shared;
            s.frames.fetch_add(1, Ordering::SeqCst);
            s.drawn.store(true, Ordering::SeqCst);
            // Only this thread counts down.
            if s.animate.load(Ordering::SeqCst) > 0 {
                s.animate.fetch_sub(1, Ordering::SeqCst);
            }
            self.inner.frame(frame, text)
        }
        // The shell reads the tree after each frame is presented.
        fn accessibility(&self) -> Option<A11yNode> {
            let s = &self.shared;
            if s.drawn.swap(false, Ordering::SeqCst)
                && s.after_frame.fetch_add(1, Ordering::SeqCst) == 0
            {
                s.gl_at_first
                    .store(1 + u8::from(gl_loaded()), Ordering::SeqCst);
            }
            self.inner.accessibility()
        }
        fn wants_frames(&self) -> bool {
            self.shared.animate.load(Ordering::SeqCst) > 0
        }
        fn take_command(&mut self) -> Command {
            match self.shared.ask.load(Ordering::SeqCst) {
                TRANSITION if !self.transition_sent => {
                    self.transition_sent = true;
                    Command::Transition(Transition {
                        kind: TransitionKind::Fade,
                        duration_ms: 250,
                        easing: Easing::Linear,
                        sound: None,
                    })
                }
                QUIT => Command::Quit,
                _ => Command::None,
            }
        }
    }

    fn fail(mode: &str, message: String) -> ! {
        println!("window_paths[{mode}]: FAILED, {message}");
        std::process::exit(1);
    }

    fn grab(conn: &impl Connection, root: u32) -> Vec<[u8; 3]> {
        let (w, h) = FIXTURE_SIZE;
        let image = conn
            .get_image(ImageFormat::Z_PIXMAP, root, 0, 0, w as u16, h as u16, !0)
            .expect("get image")
            .reply()
            .expect("image");
        image
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| [p[2], p[1], p[0]])
            .collect()
    }

    /// Pixels differing by more than 16 levels.
    fn over(a: &[[u8; 3]], b: &[[u8; 3]]) -> usize {
        a.iter()
            .zip(b)
            .filter(|(p, q)| (0..3).any(|k| p[k].abs_diff(q[k]) > 16))
            .count()
    }

    fn cpu_picture() -> Vec<[u8; 3]> {
        let (w, h) = FIXTURE_SIZE;
        let frame = frame_info(w, h, 1.0);
        let mut soft = SoftRenderer::new();
        let list = FixtureSurface::new("gallery")
            .expect("fixture")
            .frame(&frame, soft.text());
        soft.render_list(&list, &frame);
        soft.pixmap()
            .data()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| [p[0], p[1], p[2]])
            .collect()
    }

    fn frames_over(shared: &Shared, ms: u64) -> u64 {
        let before = shared.frames.load(Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(ms));
        shared.frames.load(Ordering::SeqCst) - before
    }

    /// One way of drawing, in this process.
    fn child(mode: &str) {
        let mode = mode.to_owned();
        let shared = Arc::new(Shared::default());
        let (conn, screen) = x11rb::connect(None).expect("X connection");
        let root = conn.setup().roots[screen].root;
        let expected = cpu_picture();
        // Tolerance as window_smoke: GL and CPU agree within antialiasing.
        let allowed = expected.len() / 200;
        let blank = grab(&conn, root);
        let watch = shared.clone();
        let m = mode.clone();
        let driver = std::thread::spawn(move || {
            let mode = m;
            let start = Instant::now();
            // The first picture, then only that picture within
            // antialiasing until the switch has happened.
            let mut first: Option<Vec<[u8; 3]>> = None;
            let mut grabs = 0;
            let mut switched = false;
            while start.elapsed() < Duration::from_millis(2500) {
                let got = grab(&conn, root);
                if first.is_none() {
                    if got == blank {
                        continue;
                    }
                    // The window may be mapped before its first frame:
                    // wait until it shows the page.
                    if over(&got, &expected) > allowed {
                        continue;
                    }
                    first = Some(got.clone());
                }
                grabs += 1;
                let n = over(&got, &expected);
                if n > allowed {
                    fail(
                        &mode,
                        format!(
                            "grab {grabs} at {:?}: {n} pixels off the page (flash?)",
                            start.elapsed()
                        ),
                    );
                }
                if got != expected {
                    switched = true;
                }
                if mode != "cpu-first" && grabs > 20 {
                    break;
                }
            }
            let Some(first) = first else {
                fail(&mode, "the page never showed".into());
            };
            match mode.as_str() {
                "cpu-first" => {
                    if first != expected {
                        fail(&mode, "the first picture is not the CPU renderer's".into());
                    }
                    if !switched {
                        fail(&mode, "OpenGL never took over".into());
                    }
                }
                "cpu" if switched => fail(&mode, "the CPU picture changed".into()),
                "gl" if first == expected => {
                    fail(&mode, "the GL picture is the CPU renderer's".into())
                }
                _ => {}
            }
            let idle = frames_over(&watch, 600);
            if idle != 0 {
                fail(&mode, format!("{idle} frames while idle"));
            }
            watch.animate.store(ANIMATION, Ordering::SeqCst);
            // Wake the idle shell: it asks wants_frames() at least every
            // 100 ms.
            std::thread::sleep(Duration::from_millis(900));
            let animated = watch.animate.load(Ordering::SeqCst);
            if animated != 0 {
                fail(&mode, format!("{animated} animation frames never drawn"));
            }
            let idle_after = frames_over(&watch, 400);
            watch.ask.store(TRANSITION, Ordering::SeqCst);
            let transition = frames_over(&watch, 700);
            let idle_after_transition = frames_over(&watch, 400);
            if idle_after != 0 || transition < 3 || idle_after_transition != 0 {
                fail(&mode, format!("idle {idle_after}, transition {transition}, then {idle_after_transition} frames"));
            }
            watch.ask.store(QUIT, Ordering::SeqCst);
            format!("{grabs} grabs on the page, {transition} transition frames")
        });
        let surface = Probe {
            inner: FixtureSurface::new("gallery").expect("fixture"),
            shared: shared.clone(),
            transition_sent: false,
        };
        let options = Options {
            title: "Astryx paths test".into(),
            size: FIXTURE_SIZE,
            ..Options::default()
        };
        run_with(surface, options).expect("desktop shell runs");
        let note = driver.join().expect("driver");
        let (frames, after) = (
            shared.frames.load(Ordering::SeqCst),
            shared.after_frame.load(Ordering::SeqCst),
        );
        if frames != after {
            fail(
                &mode,
                format!("{frames} frames but {after} accessibility reads after a frame"),
            );
        }
        let gl_first = shared.gl_at_first.load(Ordering::SeqCst) == 2;
        if mode != "gl" && gl_first {
            fail(
                &mode,
                "a GL library was loaded before the first frame was on screen".into(),
            );
        }
        // Quit leaves no thread of the shell's behind.
        let threads: Vec<String> = std::fs::read_dir("/proc/self/task")
            .map(|d| {
                d.flatten()
                    .filter_map(|t| std::fs::read_to_string(t.path().join("comm")).ok())
                    .map(|c| c.trim().to_owned())
                    .collect()
            })
            .unwrap_or_default();
        // The OpenGL thread and the CPU renderer's painting threads.
        if threads
            .iter()
            .any(|t| t == "astryx-gl" || t.starts_with("astryx-paint"))
        {
            fail(
                &mode,
                format!("a shell thread outlived the window: {threads:?}"),
            );
        }
        println!(
            "window_paths[{mode}]: ok, {note}; idle 0, animation {ANIMATION}, transition then 0, \
             tree read after each of {frames} frames, first frame before GL: {}, no shell thread left",
            !gl_first
        );
    }

    pub fn main() {
        if std::env::var_os("DISPLAY").is_none() {
            println!("window_paths: skipped, no X display");
            return;
        }
        if let Ok(mode) = std::env::var("ASTRYX_PATHS_CHILD") {
            child(&mode);
            return;
        }
        let me = std::env::current_exe().expect("test binary");
        let mut failed = false;
        for mode in MODES {
            let out = std::process::Command::new(&me)
                .env("ASTRYX_PATHS_CHILD", mode)
                .env("ASTRYX_RENDERER", mode)
                .env_remove("LIBGL_ALWAYS_SOFTWARE")
                .output()
                .expect("child runs");
            let text = String::from_utf8_lossy(&out.stdout);
            let line = text
                .lines()
                .find(|l| l.starts_with("window_paths["))
                .unwrap_or("no report");
            println!("{line}");
            if !out.status.success() || !line.contains(": ok") {
                failed = true;
                eprintln!("{}", String::from_utf8_lossy(&out.stderr));
            }
        }
        if failed {
            println!("window_paths: FAILED");
            std::process::exit(1);
        }
        println!("window_paths: ok");
    }
}

fn main() {
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    ))]
    x11::main();
}
