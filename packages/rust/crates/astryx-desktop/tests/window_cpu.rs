//! The CPU path end to end: with `ASTRYX_RENDERER=cpu`, the window shows,
//! pixel for pixel, what the CPU renderer draws: the first frame, then
//! frames where only part changed, including a part that changes back
//! (on Wayland each buffer is two frames old, so rows sent for the frame
//! before must go again). The pixels are read back from the display
//! server, so this checks what reaches the screen. On X11 it also keeps
//! less shared memory than one frame, and sends images through it where the
//! server takes them. Needs an X display (Xvfb) or weston with `--debug`;
//! a plain `main` because winit wants the main thread.

#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "ios", target_os = "android"))
))]
mod common;

#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "ios", target_os = "android"))
))]
mod cpu {
    use std::sync::atomic::{AtomicU8, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use astryx_core::{
        A11yNode, Color, Command, DisplayList, DrawCmd, FrameInfo, Input, InputPhase, Radii, Rect,
        Surface, TextMeasure,
    };
    use astryx_desktop::{run_with, Options};
    use astryx_headless::fixtures::{FixtureSurface, FIXTURE_SIZE};
    use astryx_headless::{astryx_render_soft::SoftRenderer, frame_info};

    use super::common::{self, Kind, Screen};

    /// Shared memory mapped once the frames showed.
    static SHARED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    /// The server takes images from shared memory.
    static SHM: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

    /// What the test has asked of the window so far: the page, then a red
    /// square on it, moved, moved back.
    const FIRST: u8 = 0;
    const SQUARE: u8 = 1;
    const MOVED: u8 = 2;
    const BACK: u8 = 3;
    const DONE: u8 = 4;

    /// The gallery fixture with the marker, and the square once the test
    /// asks.
    struct Steps {
        inner: FixtureSurface,
        step: Arc<AtomicU8>,
        drawn: u8,
    }

    fn with_square(list: DisplayList, step: u8) -> DisplayList {
        let mut list = common::with_marker(list);
        let x = match step {
            SQUARE | BACK => 40.0,
            MOVED => 200.0,
            _ => return list,
        };
        list.commands.push(DrawCmd::Rect {
            rect: Rect::new(x, 60.0, 48.0, 32.0),
            radii: Radii::all(6.0),
            fill: Color::rgba(0.9, 0.1, 0.1, 1.0),
            stroke: None,
        });
        list
    }

    impl Surface for Steps {
        fn input(&mut self, input: Input, phase: InputPhase) -> Command {
            self.inner.input(input, phase)
        }
        fn frame(&mut self, frame: &FrameInfo, text: &dyn TextMeasure) -> DisplayList {
            self.drawn = self.step.load(Ordering::SeqCst);
            with_square(self.inner.frame(frame, text), self.drawn)
        }
        fn accessibility(&self) -> Option<A11yNode> {
            self.inner.accessibility()
        }
        fn wants_frames(&self) -> bool {
            self.step.load(Ordering::SeqCst) != self.drawn
        }
        fn take_command(&mut self) -> Command {
            if self.step.load(Ordering::SeqCst) == DONE {
                Command::Quit
            } else {
                Command::None
            }
        }
    }

    fn fail(message: String) -> ! {
        println!("window_cpu: FAILED, {message}");
        std::process::exit(1);
    }

    /// What the CPU renderer draws for the fixture at `step`, as RGB.
    fn expected(step: u8) -> Vec<[u8; 3]> {
        let (w, h) = FIXTURE_SIZE;
        let frame = frame_info(w, h, 1.0);
        let mut soft = SoftRenderer::new();
        let list = FixtureSurface::new("gallery")
            .expect("fixture")
            .frame(&frame, soft.text());
        soft.render_list(&with_square(list, step), &frame);
        soft.pixmap()
            .data()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| [p[0], p[1], p[2]])
            .collect()
    }

    /// Waits until the screen shows `want`, up to two seconds.
    fn wait_for(screen: &mut Screen, want: &[[u8; 3]], what: &str) {
        let (w, h) = FIXTURE_SIZE;
        let start = Instant::now();
        loop {
            let got = screen.window(w, h);
            if got.as_deref() == Some(want) {
                return;
            }
            if start.elapsed() > Duration::from_secs(2) {
                let Some(got) = got else {
                    fail(format!("{what}: the window never showed"));
                };
                let differ = got.iter().zip(want).filter(|(a, b)| a != b).count();
                let first = got.iter().zip(want).position(|(a, b)| a != b);
                fail(format!(
                    "{what}: {differ} pixels differ from the CPU renderer's; first at {:?}",
                    first.map(|i| (i as u32 % w, i as u32 / w))
                ));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// On X11: less shared memory than one frame, and images through it
    /// where the server takes them.
    fn check_x11_memory() {
        let maps = std::fs::read_to_string("/proc/self/maps").unwrap_or_default();
        // Shared memory shows as a SysV segment, a file in /dev/shm
        // (softbuffer's buffer is /dev/shm/softbuffer-x11-*) or a memfd.
        let shared: Vec<&str> = maps
            .lines()
            .filter(|l| l.contains("SYSV") || l.contains("/dev/shm/") || l.contains("/memfd:"))
            .collect();
        let bytes: u64 = shared
            .iter()
            .filter_map(|l| {
                let (a, b) = l.split_whitespace().next()?.split_once('-')?;
                Some(u64::from_str_radix(b, 16).ok()? - u64::from_str_radix(a, 16).ok()?)
            })
            .sum();
        let frame_bytes = u64::from(FIXTURE_SIZE.0) * u64::from(FIXTURE_SIZE.1) * 4;
        if bytes >= frame_bytes {
            fail(format!(
                "{bytes} bytes of shared memory mapped, a frame is {frame_bytes}: {shared:?}"
            ));
        }
        // Where the server takes images from shared memory (MIT-SHM 1.2, on
        // this machine), they go through it: their bytes are not copied
        // through the connection.
        use x11rb::protocol::shm::ConnectionExt as _;
        let (conn, _) = x11rb::connect(None).expect("X connection");
        let shm = conn
            .shm_query_version()
            .ok()
            .and_then(|c| c.reply().ok())
            .is_some_and(|v| (v.major_version, v.minor_version) >= (1, 2));
        if shm && !shared.iter().any(|l| l.contains("astryx-strip")) {
            fail(format!(
                "the server has MIT-SHM, but no shared-memory strip is mapped: {shared:?}"
            ));
        }
        SHARED.store(bytes, Ordering::SeqCst);
        SHM.store(shm, Ordering::SeqCst);
    }

    pub fn main() {
        let Some(kind) = common::kind() else {
            println!("window_cpu: skipped, no display");
            return;
        };
        std::env::set_var("ASTRYX_RENDERER", "cpu");
        let step = Arc::new(AtomicU8::new(FIRST));
        let watch = step.clone();
        std::thread::spawn(move || {
            let mut screen = Screen::connect();
            wait_for(&mut screen, &expected(FIRST), "first frame");
            // Only the square changes: the window gets just those rows.
            for (next, what) in [
                (SQUARE, "square drawn"),
                (MOVED, "square moved"),
                (BACK, "square moved back"),
            ] {
                watch.store(next, Ordering::SeqCst);
                wait_for(&mut screen, &expected(next), what);
            }
            if kind == Kind::X11 {
                check_x11_memory();
            }
            watch.store(DONE, Ordering::SeqCst);
        });
        let surface = Steps {
            inner: FixtureSurface::new("gallery").expect("fixture"),
            step: step.clone(),
            drawn: FIRST,
        };
        let options = Options {
            title: "Astryx CPU path test".into(),
            size: FIXTURE_SIZE,
            ..Options::default()
        };
        run_with(surface, options).expect("desktop shell runs");
        if step.load(Ordering::SeqCst) != DONE {
            fail("the window closed before every frame showed".into());
        }
        match kind {
            Kind::X11 => println!(
                "window_cpu: X11: first and changed frames on screen equal the CPU renderer's; \
                 {} bytes of shared memory, less than a frame; images through shared memory: {}",
                SHARED.load(Ordering::SeqCst),
                SHM.load(Ordering::SeqCst)
            ),
            Kind::Wayland => println!(
                "window_cpu: Wayland: first and changed frames on screen equal the CPU \
                 renderer's, including a change back"
            ),
        }
        println!("window_cpu: ok");
    }
}

fn main() {
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    ))]
    cpu::main();
}
