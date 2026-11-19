//! An X server without a compositor drops the pixels of a window that
//! another window covers, and asks for them again when it moves away. With
//! the CPU renderer, which otherwise sends only rows that changed, the
//! uncovered part must come back, pixel for pixel; and an idle window still
//! draws and sends nothing. Needs an X display; run it under Xvfb on
//! headless machines. A plain `main` because winit wants the main thread.

#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "ios", target_os = "android"))
))]
mod x11 {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use astryx_core::{
        A11yNode, Command, DisplayList, FrameInfo, Input, InputPhase, Surface, TextMeasure,
    };
    use astryx_desktop::{run_with, Options};
    use astryx_headless::fixtures::{FixtureSurface, FIXTURE_SIZE};
    use astryx_headless::{astryx_render_soft::SoftRenderer, frame_info};
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{ConnectionExt, CreateWindowAux, ImageFormat, WindowClass};
    use x11rb::COPY_DEPTH_FROM_PARENT;

    /// The part another window covers, as in the bug report.
    const COVER: u16 = 300;

    struct Probe {
        inner: FixtureSurface,
        frames: Arc<AtomicU64>,
        quit: Arc<AtomicBool>,
    }

    impl Surface for Probe {
        fn input(&mut self, input: Input, phase: InputPhase) -> Command {
            self.inner.input(input, phase)
        }
        fn frame(&mut self, frame: &FrameInfo, text: &dyn TextMeasure) -> DisplayList {
            self.frames.fetch_add(1, Ordering::SeqCst);
            self.inner.frame(frame, text)
        }
        fn accessibility(&self) -> Option<A11yNode> {
            self.inner.accessibility()
        }
        fn wants_frames(&self) -> bool {
            false
        }
        fn take_command(&mut self) -> Command {
            if self.quit.load(Ordering::SeqCst) {
                Command::Quit
            } else {
                Command::None
            }
        }
    }

    fn fail(message: String) -> ! {
        println!("window_expose: FAILED, {message}");
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

    fn expected() -> Vec<[u8; 3]> {
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

    /// Waits up to a second for the window to show `want`; the pixels that
    /// differ at the end.
    fn settle(conn: &impl Connection, root: u32, want: &[[u8; 3]]) -> usize {
        let start = Instant::now();
        loop {
            let got = grab(conn, root);
            let wrong = got.iter().zip(want).filter(|(a, b)| a != b).count();
            if wrong == 0 || start.elapsed() > Duration::from_secs(1) {
                return wrong;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn main() {
        if std::env::var_os("DISPLAY").is_none() {
            println!("window_expose: skipped, no X display");
            return;
        }
        std::env::set_var("ASTRYX_RENDERER", "cpu");
        let frames = Arc::new(AtomicU64::new(0));
        let quit = Arc::new(AtomicBool::new(false));
        let (seen, stop) = (frames.clone(), quit.clone());
        let driver = std::thread::spawn(move || {
            let (conn, screen) = x11rb::connect(None).expect("X connection");
            let root = conn.setup().roots[screen].root;
            let want = expected();
            let wrong = settle(&conn, root, &want);
            if wrong != 0 {
                fail(format!(
                    "first frame: {wrong} pixels differ from the CPU renderer's"
                ));
            }
            // Another window over the top left, then away.
            let cover = conn.generate_id().expect("id");
            let aux = CreateWindowAux::new()
                .background_pixel(0x00ff_0000)
                .override_redirect(1);
            conn.create_window(
                COPY_DEPTH_FROM_PARENT,
                cover,
                root,
                0,
                0,
                COVER,
                COVER,
                0,
                WindowClass::INPUT_OUTPUT,
                0,
                &aux,
            )
            .expect("cover window");
            conn.map_window(cover).expect("map");
            conn.flush().expect("flush");
            std::thread::sleep(Duration::from_millis(300));
            let covered = grab(&conn, root)
                .iter()
                .zip(&want)
                .filter(|(a, b)| a != b)
                .count();
            conn.unmap_window(cover).expect("unmap");
            conn.flush().expect("flush");
            let wrong = settle(&conn, root, &want);
            if covered == 0 {
                fail("the cover window did not cover the page".into());
            }
            if wrong != 0 {
                fail(format!(
                    "after the cover went away, {wrong} of {} pixels are wrong",
                    u32::from(COVER) * u32::from(COVER)
                ));
            }
            // Idle again: no frames, so nothing sent.
            std::thread::sleep(Duration::from_millis(200));
            let before = seen.load(Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(800));
            let idle = seen.load(Ordering::SeqCst) - before;
            if idle != 0 {
                fail(format!("{idle} frames while idle after the expose"));
            }
            stop.store(true, Ordering::SeqCst);
            covered
        });
        let surface = Probe {
            inner: FixtureSurface::new("gallery").expect("fixture"),
            frames,
            quit,
        };
        let options = Options {
            title: "Astryx expose test".into(),
            size: FIXTURE_SIZE,
            ..Options::default()
        };
        run_with(surface, options).expect("desktop shell runs");
        let covered = driver.join().expect("driver");
        println!(
            "window_expose: covered {covered} pixels; after uncovering, 0 wrong; idle after: 0 frames"
        );
        println!("window_expose: ok");
    }
}

fn main() {
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    ))]
    x11::main();
}
