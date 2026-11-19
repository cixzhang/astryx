//! The shell picks its renderer by what the X server offers: an X server
//! with neither DRI3 nor NV-GLX, such as Xvfb, has only software GL, and the
//! window draws with the CPU renderer without loading any GL library; on
//! one with either, the CPU renderer draws the first frame, before any GL
//! library loads, and OpenGL starts after it. Needs an X display; run it
//! under Xvfb on headless machines. A plain `main` because winit wants the
//! main thread.

#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "ios", target_os = "android"))
))]
mod x11 {
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    use astryx_core::{
        A11yNode, Command, DisplayList, FrameInfo, Input, InputPhase, Surface, TextMeasure,
    };
    use astryx_desktop::{run_with, Options};
    use astryx_headless::fixtures::{FixtureSurface, FIXTURE_SIZE};
    use x11rb::connection::RequestConnection;

    /// Whether a GL library is loaded into this process.
    fn gl_loaded() -> bool {
        let maps = std::fs::read_to_string("/proc/self/maps").unwrap_or_default();
        maps.lines().any(|l| {
            let name = l.rsplit('/').next().unwrap_or("");
            name.starts_with("libGL") || name.starts_with("libEGL")
        })
    }

    /// The fixture; notes whether GL is loaded when its first frame is on
    /// screen, and quits after it, or once GL has loaded when it should.
    struct Probe {
        inner: FixtureSurface,
        frames: Cell<u32>,
        gl: Rc<Cell<Option<bool>>>,
        want_gl: bool,
        start: Instant,
    }

    impl Surface for Probe {
        fn input(&mut self, input: Input, phase: InputPhase) -> Command {
            self.inner.input(input, phase)
        }
        fn frame(&mut self, frame: &FrameInfo, text: &dyn TextMeasure) -> DisplayList {
            self.frames.set(self.frames.get() + 1);
            self.inner.frame(frame, text)
        }
        // The shell asks for the tree after each frame is presented.
        fn accessibility(&self) -> Option<A11yNode> {
            if self.frames.get() >= 1 && self.gl.get().is_none() {
                self.gl.set(Some(gl_loaded()));
            }
            self.inner.accessibility()
        }
        fn wants_frames(&self) -> bool {
            false
        }
        fn take_command(&mut self) -> Command {
            let waited = self.start.elapsed() > Duration::from_secs(3);
            match self.gl.get() {
                Some(_) if !self.want_gl || gl_loaded() || waited => Command::Quit,
                _ => Command::None,
            }
        }
    }

    pub fn main() {
        if std::env::var_os("DISPLAY").is_none() {
            println!("window_backend: skipped, no X display");
            return;
        }
        std::env::remove_var("ASTRYX_RENDERER");
        let (conn, _) = x11rb::connect(None).expect("X connection");
        let has = |name: &'static str| conn.extension_information(name).ok().flatten().is_some();
        let (dri3, nv_glx) = (has("DRI3"), has("NV-GLX"));
        let software =
            std::env::var_os("LIBGL_ALWAYS_SOFTWARE").is_some_and(|v| v == "1" || v == "true");
        let want_gl = (dri3 || nv_glx) && !software;
        let seen = Rc::new(Cell::new(None));
        let probe = Probe {
            inner: FixtureSurface::new("gallery").expect("fixture"),
            frames: Cell::new(0),
            gl: seen.clone(),
            want_gl,
            start: Instant::now(),
        };
        let options = Options {
            title: "Astryx renderer choice test".into(),
            size: FIXTURE_SIZE,
            ..Options::default()
        };
        run_with(probe, options).expect("desktop shell runs");
        let Some(gl_first) = seen.get() else {
            println!("window_backend: FAILED, no frame was presented");
            std::process::exit(1);
        };
        let gl_later = gl_loaded();
        println!(
            "window_backend: DRI3 {dri3}, NV-GLX {nv_glx}, LIBGL_ALWAYS_SOFTWARE {software}: \
             first frame {}, then {}",
            if gl_first {
                "with GL loaded"
            } else {
                "before any GL library"
            },
            if gl_later {
                "GL loaded"
            } else {
                "no GL library loaded"
            }
        );
        if gl_first || gl_later != want_gl {
            println!(
                "window_backend: FAILED, expected the CPU renderer first, then {}",
                if want_gl { "OpenGL" } else { "no GL library" }
            );
            std::process::exit(1);
        }
        println!("window_backend: ok");
    }
}

fn main() {
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    ))]
    x11::main();
}
