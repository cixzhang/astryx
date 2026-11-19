//! The CPU path end to end: with `ASTRYX_RENDERER=cpu`, the window shows,
//! pixel for pixel, what the CPU renderer draws, both the first frame and a
//! later frame where only part changed. The pixels are read back from the X
//! server, so this checks what reaches the screen. Needs an X display; run
//! it under Xvfb on headless machines. A plain `main` because winit wants
//! the main thread.

#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "ios", target_os = "android"))
))]
mod x11 {
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
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{ConnectionExt, ImageFormat};

    /// What the test has asked of the window so far.
    const FIRST: u8 = 0;
    const CHANGED: u8 = 1;
    const DONE: u8 = 2;

    /// The gallery fixture, with a red square added once the test asks.
    struct Steps {
        inner: FixtureSurface,
        step: Arc<AtomicU8>,
        drawn: u8,
    }

    fn with_square(mut list: DisplayList, step: u8) -> DisplayList {
        if step >= CHANGED {
            list.commands.push(DrawCmd::Rect {
                rect: Rect::new(40.0, 60.0, 48.0, 32.0),
                radii: Radii::all(6.0),
                fill: Color::rgba(0.9, 0.1, 0.1, 1.0),
                stroke: None,
            });
        }
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

    /// The window's pixels as RGB, read from the top left of the screen,
    /// where a window opens on an X server with no window manager.
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
    fn wait_for(conn: &impl Connection, root: u32, want: &[[u8; 3]], what: &str) {
        let start = Instant::now();
        loop {
            let got = grab(conn, root);
            if got == want {
                return;
            }
            if start.elapsed() > Duration::from_secs(2) {
                let differ = got.iter().zip(want).filter(|(a, b)| a != b).count();
                let first = got.iter().zip(want).position(|(a, b)| a != b);
                fail(format!(
                    "{what}: {differ} pixels differ from the CPU renderer's; first at {:?}",
                    first.map(|i| (i as u32 % FIXTURE_SIZE.0, i as u32 / FIXTURE_SIZE.0))
                ));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn main() {
        if std::env::var_os("DISPLAY").is_none() {
            println!("window_cpu: skipped, no X display");
            return;
        }
        std::env::set_var("ASTRYX_RENDERER", "cpu");
        let (conn, screen) = x11rb::connect(None).expect("X connection");
        let root = conn.setup().roots[screen].root;
        let step = Arc::new(AtomicU8::new(FIRST));
        let watch = step.clone();
        std::thread::spawn(move || {
            wait_for(&conn, root, &expected(FIRST), "first frame");
            // Only the square changes: the window gets just those rows.
            watch.store(CHANGED, Ordering::SeqCst);
            wait_for(&conn, root, &expected(CHANGED), "changed frame");
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
            fail("the window closed before both frames showed".into());
        }
        println!("window_cpu: first and changed frames on screen equal the CPU renderer's");
        println!("window_cpu: ok");
    }
}

fn main() {
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    ))]
    x11::main();
}
