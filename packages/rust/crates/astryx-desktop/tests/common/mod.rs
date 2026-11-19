//! What a window test sees on screen: the window's pixels, read back from
//! the display server.
//!
//! On X11 (Xvfb, no window manager) the window opens at the top left of the
//! screen and is read with GetImage. On Wayland (weston, started with
//! `--debug` so its screenshooter is offered) the whole output is captured
//! and the window is found by a marker: a magenta square that the test's
//! surface draws at its top left ([`with_marker`]).

#![allow(dead_code)]

use astryx_core::{Color, DisplayList, DrawCmd, Radii, Rect};

/// The marker's side in pixels.
pub const MARKER: u32 = 6;
const MAGENTA: [u8; 3] = [255, 0, 255];

/// `list` with the marker on top, at the top left.
pub fn with_marker(mut list: DisplayList) -> DisplayList {
    list.commands.push(DrawCmd::Rect {
        rect: Rect::new(0.0, 0.0, MARKER as f32, MARKER as f32),
        radii: Radii::default(),
        fill: Color::rgba(1.0, 0.0, 1.0, 1.0),
        stroke: None,
    });
    list
}

/// Where window tests run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    X11,
    Wayland,
}

/// The display server the shell's windows open on, as winit chooses it:
/// Wayland when `WAYLAND_DISPLAY` is set, else X11 when `DISPLAY` is.
pub fn kind() -> Option<Kind> {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        Some(Kind::Wayland)
    } else if std::env::var_os("DISPLAY").is_some() {
        Some(Kind::X11)
    } else {
        None
    }
}

/// Reads the window's pixels.
pub enum Screen {
    X11 {
        conn: x11rb::rust_connection::RustConnection,
        root: u32,
    },
    Wayland(wl::Shooter),
}

impl Screen {
    pub fn connect() -> Screen {
        match kind().expect("a display") {
            Kind::X11 => {
                use x11rb::connection::Connection;
                let (conn, screen) = x11rb::connect(None).expect("X connection");
                let root = conn.setup().roots[screen].root;
                Screen::X11 { conn, root }
            }
            Kind::Wayland => Screen::Wayland(wl::Shooter::connect()),
        }
    }

    /// The `w` x `h` pixels of the window, as RGB; `None` while it is not
    /// on screen (on Wayland: no marker yet).
    pub fn window(&mut self, w: u32, h: u32) -> Option<Vec<[u8; 3]>> {
        match self {
            Screen::X11 { conn, root } => {
                use x11rb::protocol::xproto::{ConnectionExt, ImageFormat};
                let image = conn
                    .get_image(ImageFormat::Z_PIXMAP, *root, 0, 0, w as u16, h as u16, !0)
                    .ok()?
                    .reply()
                    .ok()?;
                Some(
                    image
                        .data
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|p| [p[2], p[1], p[0]])
                        .collect(),
                )
            }
            Screen::Wayland(shooter) => {
                let (sw, sh, pixels) = shooter.shoot();
                let at = find_marker(sw, sh, &pixels)?;
                if at.0 + w > sw || at.1 + h > sh {
                    return None;
                }
                let mut out = Vec::with_capacity((w * h) as usize);
                for y in 0..h {
                    let row = ((at.1 + y) * sw + at.0) as usize;
                    out.extend_from_slice(&pixels[row..row + w as usize]);
                }
                Some(out)
            }
        }
    }
}

/// The top left of the first full marker square in a `w`-wide capture.
fn find_marker(w: u32, h: u32, pixels: &[[u8; 3]]) -> Option<(u32, u32)> {
    let at = |x: u32, y: u32| pixels[(y * w + x) as usize];
    for y in 0..h.saturating_sub(MARKER) {
        for x in 0..w.saturating_sub(MARKER) {
            if at(x, y) != MAGENTA {
                continue;
            }
            let full = (0..MARKER).all(|dy| (0..MARKER).all(|dx| at(x + dx, y + dy) == MAGENTA));
            if full {
                return Some((x, y));
            }
        }
    }
    None
}

pub mod wl {
    //! weston's screenshooter.

    use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};

    use wayland_client::protocol::{wl_buffer, wl_output, wl_registry, wl_shm, wl_shm_pool};
    use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle};

    #[allow(non_upper_case_globals, non_camel_case_types, unused)]
    pub mod protocol {
        use wayland_client;
        use wayland_client::protocol::*;

        pub mod __interfaces {
            use wayland_client::protocol::__interfaces::*;
            wayland_scanner::generate_interfaces!("tests/common/weston-screenshooter.xml");
        }
        use self::__interfaces::*;

        wayland_scanner::generate_client_code!("tests/common/weston-screenshooter.xml");
    }
    use protocol::weston_screenshooter::{self, WestonScreenshooter};

    #[derive(Default)]
    struct State {
        shm: Option<wl_shm::WlShm>,
        output: Option<wl_output::WlOutput>,
        shooter: Option<WestonScreenshooter>,
        mode: Option<(u32, u32)>,
        done: bool,
    }

    pub struct Shooter {
        _conn: Connection,
        queue: EventQueue<State>,
        state: State,
        /// The capture's size and its buffer, reused.
        size: (u32, u32),
        memory: *mut u8,
        buffer: wl_buffer::WlBuffer,
    }

    // SAFETY: the mapping belongs to the shooter alone, and only `shoot`,
    // through `&mut self`, reads it.
    unsafe impl Send for Shooter {}

    impl Shooter {
        pub fn connect() -> Shooter {
            let conn = Connection::connect_to_env().expect("Wayland connection");
            let mut queue = conn.new_event_queue();
            let qh = queue.handle();
            conn.display().get_registry(&qh, ());
            let mut state = State::default();
            queue.roundtrip(&mut state).expect("globals");
            queue.roundtrip(&mut state).expect("output mode");
            let shm = state.shm.clone().expect("wl_shm");
            assert!(
                state.shooter.is_some(),
                "no weston_screenshooter: start weston with --debug"
            );
            let (w, h) = state.mode.expect("an output mode");
            let len = (w * h * 4) as usize;
            // SAFETY: a new memory file of this process's own, mapped for
            // as long as the shooter lives.
            let (fd, memory) = unsafe {
                let fd = libc::memfd_create(c"window-test-shot".as_ptr(), libc::MFD_CLOEXEC);
                assert!(fd >= 0, "memfd");
                let fd = OwnedFd::from_raw_fd(fd);
                assert_eq!(libc::ftruncate(fd.as_raw_fd(), len as libc::off_t), 0);
                let memory = libc::mmap(
                    std::ptr::null_mut(),
                    len,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_SHARED,
                    fd.as_raw_fd(),
                    0,
                );
                assert_ne!(memory, libc::MAP_FAILED);
                (fd, memory.cast::<u8>())
            };
            let pool = shm.create_pool(fd.as_fd(), len as i32, &qh, ());
            let buffer = pool.create_buffer(
                0,
                w as i32,
                h as i32,
                (w * 4) as i32,
                wl_shm::Format::Argb8888,
                &qh,
                (),
            );
            pool.destroy();
            Shooter {
                _conn: conn,
                queue,
                state,
                size: (w, h),
                memory,
                buffer,
            }
        }

        /// The whole output, as RGB, with its width and height.
        pub fn shoot(&mut self) -> (u32, u32, Vec<[u8; 3]>) {
            let (w, h) = self.size;
            self.state.done = false;
            let shooter = self.state.shooter.clone().expect("shooter");
            let output = self.state.output.clone().expect("output");
            shooter.shoot(&output, &self.buffer);
            while !self.state.done {
                self.queue
                    .blocking_dispatch(&mut self.state)
                    .expect("dispatch");
            }
            // SAFETY: mapped in `connect`; the compositor wrote it before
            // `done`.
            let bytes = unsafe { std::slice::from_raw_parts(self.memory, (w * h * 4) as usize) };
            let pixels = bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| [p[2], p[1], p[0]])
                .collect();
            (w, h, pixels)
        }
    }

    /// Whether the compositor takes GPU buffers from its clients
    /// (`zwp_linux_dmabuf_v1`, `wl_drm` or `wl_eglstream_display`).
    pub fn gpu_buffers() -> bool {
        struct Names(Vec<String>);
        impl Dispatch<wl_registry::WlRegistry, ()> for Names {
            fn event(
                names: &mut Self,
                _: &wl_registry::WlRegistry,
                event: wl_registry::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
                if let wl_registry::Event::Global { interface, .. } = event {
                    names.0.push(interface);
                }
            }
        }
        let conn = Connection::connect_to_env().expect("Wayland connection");
        let mut queue = conn.new_event_queue();
        conn.display().get_registry(&queue.handle(), ());
        let mut names = Names(Vec::new());
        queue.roundtrip(&mut names).expect("globals");
        names.0.iter().any(|n| {
            ["zwp_linux_dmabuf_v1", "wl_drm", "wl_eglstream_display"].contains(&n.as_str())
        })
    }

    impl Dispatch<wl_registry::WlRegistry, ()> for State {
        fn event(
            state: &mut Self,
            registry: &wl_registry::WlRegistry,
            event: wl_registry::Event,
            _: &(),
            _: &Connection,
            qh: &QueueHandle<Self>,
        ) {
            if let wl_registry::Event::Global {
                name, interface, ..
            } = event
            {
                match interface.as_str() {
                    "wl_shm" => state.shm = Some(registry.bind(name, 1, qh, ())),
                    "wl_output" if state.output.is_none() => {
                        state.output = Some(registry.bind(name, 1, qh, ()))
                    }
                    "weston_screenshooter" => state.shooter = Some(registry.bind(name, 1, qh, ())),
                    _ => {}
                }
            }
        }
    }

    impl Dispatch<wl_output::WlOutput, ()> for State {
        fn event(
            state: &mut Self,
            _: &wl_output::WlOutput,
            event: wl_output::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            if let wl_output::Event::Mode {
                flags,
                width,
                height,
                ..
            } = event
            {
                let current = flags
                    .into_result()
                    .is_ok_and(|f| f.contains(wl_output::Mode::Current));
                if current || state.mode.is_none() {
                    state.mode = Some((width as u32, height as u32));
                }
            }
        }
    }

    impl Dispatch<WestonScreenshooter, ()> for State {
        fn event(
            state: &mut Self,
            _: &WestonScreenshooter,
            event: weston_screenshooter::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            let weston_screenshooter::Event::Done = event;
            state.done = true;
        }
    }

    wayland_client::delegate_noop!(State: ignore wl_shm::WlShm);
    wayland_client::delegate_noop!(State: ignore wl_shm_pool::WlShmPool);
    wayland_client::delegate_noop!(State: ignore wl_buffer::WlBuffer);
}
