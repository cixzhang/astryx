//! The CPU path's window: the CPU renderer's pixels, copied to the window.
//!
//! On X11 the rows that changed go to the server as images, a few rows at a
//! time, over an X connection of the presenter's own; nothing else is kept
//! but a hash of each row on screen. Elsewhere, and when the X server takes
//! no such images, `softbuffer` keeps a frame-sized buffer (shared memory on
//! X11) and sends what changed from it.
//!
//! An X server without a compositor does not keep a window's pixels: when
//! another window uncovers part of it, the server sends Expose and that part
//! must be sent again. Only rows that changed are sent otherwise, so the
//! presenter watches for Expose itself.

use std::error::Error;
use std::num::NonZeroU32;
use std::sync::Arc;

use astryx_render_soft::tiny_skia::Pixmap;
use winit::window::Window;

/// Shows frames in a window.
pub(crate) enum Present {
    /// Changed rows as X images: no frame-sized buffer.
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    ))]
    Images(Box<x11::Images>),
    Buffer(Box<Buffered>),
}

impl Present {
    pub(crate) fn new(window: &Arc<Window>) -> Result<Self, Box<dyn Error>> {
        #[cfg(all(
            unix,
            not(any(target_os = "macos", target_os = "ios", target_os = "android"))
        ))]
        if let Some(images) = x11::Images::open(window) {
            return Ok(Present::Images(Box::new(images)));
        }
        Ok(Present::Buffer(Box::new(Buffered::new(window)?)))
    }

    /// Shows `frame`, premultiplied RGBA. The window is opaque, so alpha is
    /// dropped. When the server still has the window's pixels, only rows
    /// that changed are sent; an unchanged frame sends nothing.
    pub(crate) fn show(&mut self, frame: &Pixmap) -> Result<(), Box<dyn Error>> {
        match self {
            #[cfg(all(
                unix,
                not(any(target_os = "macos", target_os = "ios", target_os = "android"))
            ))]
            Present::Images(images) => images.show(frame),
            Present::Buffer(buffered) => buffered.show(frame).map_err(Into::into),
        }
    }
}

/// Frames through `softbuffer`'s frame-sized buffer.
pub(crate) struct Buffered {
    // Field order is drop order: the surface goes before its context.
    surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    _context: softbuffer::Context<Arc<Window>>,
    /// On X11: where the server asks for the window's pixels again.
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    ))]
    exposes: Option<x11::Exposes>,
    /// The rows (first, end) each recent present wrote into its buffer,
    /// newest last; `None` for a whole frame. A buffer `age` frames old
    /// misses the rows the last `age - 1` presents wrote, so those are sent
    /// again with the rows that change now (Wayland's two buffers give age
    /// 2).
    written: std::collections::VecDeque<Option<(usize, usize)>>,
}

/// How many presents [`Buffered::written`] keeps.
const KEPT: usize = 3;

impl Buffered {
    fn new(window: &Arc<Window>) -> Result<Self, Box<dyn Error>> {
        let context = softbuffer::Context::new(window.clone())?;
        let surface = softbuffer::Surface::new(&context, window.clone())?;
        Ok(Self {
            surface,
            _context: context,
            #[cfg(all(
                unix,
                not(any(target_os = "macos", target_os = "ios", target_os = "android"))
            ))]
            exposes: x11::Exposes::watch(window),
            written: std::collections::VecDeque::new(),
        })
    }

    /// As [`Present::show`]. Every pixel that differs from the buffer's is
    /// written, so the buffer then holds the frame whatever its age; the
    /// rows sent are those written now and those the buffer's age says the
    /// screen has newer than the buffer had.
    fn show(&mut self, frame: &Pixmap) -> Result<(), softbuffer::SoftBufferError> {
        let (Some(w), Some(h)) = (
            NonZeroU32::new(frame.width()),
            NonZeroU32::new(frame.height()),
        ) else {
            return Ok(());
        };
        let exposed = self.exposed();
        self.surface.resize(w, h)?;
        let mut buffer = self.surface.buffer_mut()?;
        // A new buffer (age 0), one older than the presents kept, a new
        // size, or a server that dropped part of the window: every row goes.
        let age = usize::from(buffer.age());
        let newer: Option<Vec<Option<(usize, usize)>>> = (age >= 1
            && age - 1 <= self.written.len())
        .then(|| self.written.iter().rev().take(age - 1).copied().collect());
        let whole = exposed || newer.as_ref().is_none_or(|n| n.iter().any(Option::is_none));
        let width = w.get() as usize;
        let (mut first, mut last) = (usize::MAX, 0);
        for (y, (out, row)) in buffer
            .chunks_exact_mut(width)
            .zip(frame.data().chunks_exact(width * 4))
            .enumerate()
        {
            let mut changed = false;
            for (o, p) in out.iter_mut().zip(row.as_chunks::<4>().0) {
                let v = (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32;
                if *o != v {
                    *o = v;
                    changed = true;
                }
            }
            if changed || whole {
                first = first.min(y);
                last = y;
            }
        }
        let mut keep = |entry| {
            self.written.push_back(entry);
            if self.written.len() > KEPT {
                self.written.pop_front();
            }
        };
        if whole {
            keep(None);
            return buffer.present();
        }
        let now = (first <= last).then_some((first, last + 1));
        // The rows to send: written now, or newer on screen than in this
        // buffer.
        let send = newer
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .chain(now)
            .reduce(|(a0, a1), (b0, b1)| (a0.min(b0), a1.max(b1)));
        let Some((y0, y1)) = send else {
            // Nothing changed: the frame on screen stays.
            return Ok(());
        };
        keep(now);
        let rows = NonZeroU32::new((y1 - y0) as u32).expect("at least one row");
        buffer.present_with_damage(&[softbuffer::Rect {
            x: 0,
            y: y0 as u32,
            width: w,
            height: rows,
        }])
    }

    /// Whether the server has asked for any of the window's pixels again
    /// since the last frame.
    fn exposed(&self) -> bool {
        #[cfg(all(
            unix,
            not(any(target_os = "macos", target_os = "ios", target_os = "android"))
        ))]
        if let Some(e) = &self.exposes {
            return e.exposed();
        }
        false
    }
}

#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "ios", target_os = "android"))
))]
mod x11 {
    use std::error::Error;

    use astryx_render_soft::tiny_skia::Pixmap;
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use winit::window::Window;
    use x11rb::connection::{Connection, RequestConnection};
    use x11rb::protocol::shm::ConnectionExt as _;
    use x11rb::protocol::xproto::{
        ChangeWindowAttributesAux, ConnectionExt, CreateGCAux, EventMask, ImageFormat, ImageOrder,
        VisualClass,
    };
    use x11rb::protocol::Event;
    use x11rb::rust_connection::RustConnection;

    fn xid(window: &Window) -> Option<u32> {
        match window.window_handle().ok()?.as_raw() {
            RawWindowHandle::Xlib(w) => u32::try_from(w.window).ok(),
            RawWindowHandle::Xcb(w) => Some(w.window.get()),
            _ => None,
        }
    }

    /// A connection of the presenter's own, with Expose chosen for `xid`.
    fn connect(xid: u32) -> Option<RustConnection> {
        let (conn, _) = x11rb::connect(None).ok()?;
        let mask = ChangeWindowAttributesAux::new().event_mask(EventMask::EXPOSURE);
        conn.change_window_attributes(xid, &mask)
            .ok()?
            .check()
            .ok()?;
        Some(conn)
    }

    /// Whether the server asked for any of the window's pixels again since
    /// the last call. A round trip first: the server sends the connection
    /// every Expose it made before answering, so none that woke the frame is
    /// still on its way.
    fn exposed(conn: &RustConnection) -> bool {
        let synced = conn
            .get_input_focus()
            .ok()
            .and_then(|c| c.reply().ok())
            .is_some();
        let mut exposed = !synced;
        while let Ok(Some(event)) = conn.poll_for_event() {
            exposed |= matches!(event, Event::Expose(_));
        }
        exposed
    }

    /// The most rows one image carries without shared memory: enough to keep
    /// requests few, small enough to keep the buffer that builds them small.
    const MOST_ROWS: usize = 32;

    /// About how many bytes each shared-memory strip holds.
    const STRIP_BYTES: usize = 256 * 1024;

    /// Frames as X images of the rows that changed.
    pub(in super::super) struct Images {
        conn: RustConnection,
        window: u32,
        gc: u32,
        depth: u8,
        /// The frame size on screen.
        size: (u32, u32),
        /// A hash of each row on screen; empty when the window's pixels are
        /// not known (first frame, a new size, an Expose).
        rows: Vec<u64>,
        /// The server asked for the window's pixels again while this
        /// presenter waited for a strip: the next frame goes whole.
        exposed: bool,
        /// Two strips of shared memory the server reads images from, one
        /// filled while the server reads the other; `None` where the server
        /// has no MIT-SHM 1.2 or is not on this machine.
        strips: Option<shm::Strips>,
        /// Without shared memory: rows converted for the server, reused.
        strip: Vec<u8>,
    }

    impl Images {
        /// `None` when the window is not on X11, or its server does not take
        /// images as 32-bit little-endian XRGB for its visual.
        pub(in super::super) fn open(window: &Window) -> Option<Self> {
            let window = xid(window)?;
            let conn = connect(window)?;
            let setup = conn.setup();
            if setup.image_byte_order != ImageOrder::LSB_FIRST {
                return None;
            }
            let geometry = conn.get_geometry(window).ok()?.reply().ok()?;
            let attributes = conn.get_window_attributes(window).ok()?.reply().ok()?;
            let format = setup
                .pixmap_formats
                .iter()
                .find(|f| f.depth == geometry.depth)?;
            if format.bits_per_pixel != 32 {
                return None;
            }
            let visual = setup
                .roots
                .iter()
                .flat_map(|r| &r.allowed_depths)
                .flat_map(|d| &d.visuals)
                .find(|v| v.visual_id == attributes.visual)?;
            let xrgb = visual.class == VisualClass::TRUE_COLOR
                && (visual.red_mask, visual.green_mask, visual.blue_mask)
                    == (0x00ff_0000, 0x0000_ff00, 0x0000_00ff);
            if !xrgb {
                return None;
            }
            let gc = conn.generate_id().ok()?;
            conn.create_gc(gc, window, &CreateGCAux::new())
                .ok()?
                .check()
                .ok()?;
            Some(Self {
                conn,
                window,
                gc,
                depth: geometry.depth,
                size: (0, 0),
                rows: Vec::new(),
                exposed: false,
                strips: None,
                strip: Vec::new(),
            })
        }

        pub(in super::super) fn show(&mut self, frame: &Pixmap) -> Result<(), Box<dyn Error>> {
            let (w, h) = (frame.width(), frame.height());
            if w == 0 || h == 0 {
                return Ok(());
            }
            let row_bytes = w as usize * 4;
            if self.size.0 != w {
                // Strips fit the width: new ones, once the server has read
                // the old.
                if let Some(old) = self.strips.take() {
                    old.release(&self.conn);
                }
                self.strips = shm::Strips::new(&self.conn, row_bytes, STRIP_BYTES);
            }
            if self.take_events()? || std::mem::take(&mut self.exposed) || self.size != (w, h) {
                self.rows.clear();
            }
            self.size = (w, h);
            let known = !self.rows.is_empty();
            self.rows.resize(h as usize, 0);
            // Rows per image: what a strip holds, or what one request
            // carries, at most MOST_ROWS.
            let per = match &self.strips {
                Some(s) => s.rows(),
                None => (self.conn.maximum_request_bytes().saturating_sub(64) / row_bytes)
                    .clamp(1, MOST_ROWS),
            };
            let mut run: Option<usize> = None;
            let data = frame.data();
            for y in 0..=h as usize {
                let changed = y < h as usize && {
                    let hash = row_hash(&data[y * row_bytes..(y + 1) * row_bytes]);
                    let changed = !known || self.rows[y] != hash;
                    self.rows[y] = hash;
                    changed
                };
                match (changed, run) {
                    (true, None) => run = Some(y),
                    (false, Some(first)) => {
                        self.send(data, w, first, y)?;
                        run = None;
                    }
                    _ => {}
                }
                if let Some(first) = run {
                    if y + 1 - first == per {
                        self.send(data, w, first, y + 1)?;
                        run = None;
                    }
                }
            }
            self.conn.flush()?;
            Ok(())
        }

        /// Takes the events that arrived; whether one was an Expose.
        fn take_events(&mut self) -> Result<bool, Box<dyn Error>> {
            // A round trip first: the server sends the connection every
            // Expose it made before answering, so none that woke the frame
            // is still on its way.
            self.conn.get_input_focus()?.reply()?;
            let mut exposed = false;
            while let Some(event) = self.conn.poll_for_event()? {
                exposed |= self.note(event);
            }
            Ok(exposed)
        }

        /// Notes one event; whether it was an Expose.
        fn note(&mut self, event: Event) -> bool {
            match event {
                Event::Expose(_) => true,
                Event::ShmCompletion(done) => {
                    if let Some(strips) = &mut self.strips {
                        strips.done(done.shmseg);
                    }
                    false
                }
                _ => false,
            }
        }

        /// Sends rows `first..end` of `data`, a frame `w` pixels wide.
        fn send(
            &mut self,
            data: &[u8],
            w: u32,
            first: usize,
            end: usize,
        ) -> Result<(), Box<dyn Error>> {
            let row_bytes = w as usize * 4;
            let src = &data[first * row_bytes..end * row_bytes];
            let rows = (end - first) as u16;
            if self.strips.is_some() {
                // A strip the server has finished reading.
                while !self.strips.as_ref().expect("strips").free() {
                    let event = self.conn.wait_for_event()?;
                    self.exposed |= self.note(event);
                }
                let strips = self.strips.as_mut().expect("strips");
                let (seg, dst) = strips.take();
                convert(src, dst);
                self.conn.shm_put_image(
                    self.window,
                    self.gc,
                    w as u16,
                    rows,
                    0,
                    0,
                    w as u16,
                    rows,
                    0,
                    first as i16,
                    self.depth,
                    ImageFormat::Z_PIXMAP.into(),
                    true,
                    seg,
                    0,
                )?;
                // Send now: the server reads this strip while the next fills.
                self.conn.flush()?;
                return Ok(());
            }
            self.strip.resize(src.len(), 0);
            convert(src, &mut self.strip);
            self.conn.put_image(
                ImageFormat::Z_PIXMAP,
                self.window,
                self.gc,
                w as u16,
                rows,
                0,
                first as i16,
                0,
                self.depth,
                &self.strip,
            )?;
            Ok(())
        }
    }

    impl Drop for Images {
        fn drop(&mut self) {
            if let Some(strips) = self.strips.take() {
                strips.release(&self.conn);
            }
        }
    }

    /// Premultiplied RGBA to the server's XRGB, little-endian.
    fn convert(src: &[u8], dst: &mut [u8]) {
        for (d, s) in dst
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(src.as_chunks::<4>().0)
        {
            let p = u32::from_le_bytes(*s);
            let xrgb = (p & 0x0000_ff00) | ((p >> 16) & 0xff) | ((p & 0xff) << 16);
            *d = (0xff00_0000 | xrgb).to_le_bytes();
        }
    }

    /// A 64-bit hash of one row's bytes, for telling changed rows. Four
    /// lanes, so the multiplies do not wait on each other.
    fn row_hash(row: &[u8]) -> u64 {
        const K: u64 = 0x9e37_79b9_7f4a_7c15;
        let (blocks, rest) = row.as_chunks::<32>();
        let mut h = [
            0x243f_6a88_85a3_08d3u64,
            0x1319_8a2e_0370_7344,
            0xa409_3822_299f_31d0,
            0x082e_fa98_ec4e_6c89,
        ];
        for b in blocks {
            for (lane, w) in h.iter_mut().zip(b.as_chunks::<8>().0) {
                *lane = (*lane ^ u64::from_le_bytes(*w))
                    .wrapping_mul(K)
                    .rotate_left(29);
            }
        }
        let mut out = h.iter().fold(row.len() as u64, |acc, &l| {
            (acc ^ l).wrapping_mul(K).rotate_left(31)
        });
        for &b in rest {
            out = (out ^ u64::from(b)).wrapping_mul(K);
        }
        out
    }

    /// Expose events for one window, on a connection of their own: each X
    /// client chooses its own events, so this leaves winit's alone.
    pub(in super::super) struct Exposes {
        conn: RustConnection,
    }

    impl Exposes {
        /// `None` when the window is not on X11 or the server cannot be
        /// reached; frames then go as before.
        pub(in super::super) fn watch(window: &Window) -> Option<Self> {
            Some(Self {
                conn: connect(xid(window)?)?,
            })
        }

        pub(in super::super) fn exposed(&self) -> bool {
            exposed(&self.conn)
        }
    }

    /// Strips of shared memory for X images: the server reads the image
    /// from memory this process maps too, so its bytes are not copied
    /// through the connection.
    mod shm {
        use std::os::fd::{FromRawFd, OwnedFd};
        use std::ptr::NonNull;

        use x11rb::connection::Connection;
        use x11rb::protocol::shm::ConnectionExt;
        use x11rb::protocol::xproto::ConnectionExt as _;
        use x11rb::rust_connection::RustConnection;

        /// One mapped segment, attached on the server.
        struct Strip {
            seg: u32,
            ptr: NonNull<u8>,
            len: usize,
            /// The server may still be reading it.
            busy: bool,
        }

        /// Two strips, used in turn.
        pub(super) struct Strips {
            strips: [Strip; 2],
            rows: usize,
            next: usize,
        }

        // SAFETY, for every `unsafe` in this module: a strip's memory is
        // mapped from `new` until `release`, and this process writes it only
        // while the strip is not busy: after the server's ShmCompletion for
        // its last image, or before its first. The server only reads it.

        impl Strips {
            /// Strips of about `bytes` each, whole rows of `row_bytes`;
            /// `None` without MIT-SHM 1.2 (fd passing) or when the server
            /// cannot attach this process's memory (another machine).
            pub(super) fn new(
                conn: &RustConnection,
                row_bytes: usize,
                bytes: usize,
            ) -> Option<Self> {
                if !cfg!(target_os = "linux") {
                    return None;
                }
                let version = conn.shm_query_version().ok()?.reply().ok()?;
                if (version.major_version, version.minor_version) < (1, 2) {
                    return None;
                }
                let rows = (bytes / row_bytes).clamp(1, 128);
                let len = rows * row_bytes;
                let a = Strip::new(conn, len)?;
                let b = match Strip::new(conn, len) {
                    Some(b) => b,
                    None => {
                        a.release(conn);
                        return None;
                    }
                };
                Some(Self {
                    strips: [a, b],
                    rows,
                    next: 0,
                })
            }

            /// Rows one strip holds.
            pub(super) fn rows(&self) -> usize {
                self.rows
            }

            /// Whether the strip to use next is free.
            pub(super) fn free(&self) -> bool {
                !self.strips[self.next].busy
            }

            /// The next strip, marked busy: its segment, and its memory to
            /// fill before the image is sent. Only when [`Self::free`].
            pub(super) fn take(&mut self) -> (u32, &mut [u8]) {
                let strip = &mut self.strips[self.next];
                assert!(!strip.busy, "strip still read by the server");
                strip.busy = true;
                self.next ^= 1;
                // SAFETY: see the top of the module; the strip was not busy.
                let memory =
                    unsafe { std::slice::from_raw_parts_mut(strip.ptr.as_ptr(), strip.len) };
                (strip.seg, memory)
            }

            /// The server finished reading segment `seg`.
            pub(super) fn done(&mut self, seg: u32) {
                for strip in &mut self.strips {
                    if strip.seg == seg {
                        strip.busy = false;
                    }
                }
            }

            /// Detaches and unmaps both, after the server has read them.
            pub(super) fn release(self, conn: &RustConnection) {
                // After this round trip the server has handled every image
                // sent before it.
                let _ = conn.get_input_focus().map(|c| c.reply());
                for strip in self.strips {
                    strip.release(conn);
                }
            }
        }

        impl Strip {
            fn new(conn: &RustConnection, len: usize) -> Option<Self> {
                #[cfg(target_os = "linux")]
                {
                    // SAFETY: see the top of the module. The file and the
                    // mapping are this process's own; the fd moves to the
                    // server, and the mapping stays until `release`.
                    unsafe {
                        let fd = libc::memfd_create(c"astryx-strip".as_ptr(), libc::MFD_CLOEXEC);
                        if fd < 0 {
                            return None;
                        }
                        let fd = OwnedFd::from_raw_fd(fd);
                        if libc::ftruncate(std::os::fd::AsRawFd::as_raw_fd(&fd), len as libc::off_t)
                            != 0
                        {
                            return None;
                        }
                        let ptr = libc::mmap(
                            std::ptr::null_mut(),
                            len,
                            libc::PROT_READ | libc::PROT_WRITE,
                            libc::MAP_SHARED,
                            std::os::fd::AsRawFd::as_raw_fd(&fd),
                            0,
                        );
                        if ptr == libc::MAP_FAILED {
                            return None;
                        }
                        let ptr = NonNull::new(ptr.cast::<u8>())?;
                        let strip = Self {
                            seg: 0,
                            ptr,
                            len,
                            busy: false,
                        };
                        let seg = match conn.generate_id() {
                            Ok(id) => id,
                            Err(_) => {
                                strip.unmap();
                                return None;
                            }
                        };
                        // Read-only for the server: it only reads images.
                        let attached = conn
                            .shm_attach_fd(seg, fd, true)
                            .ok()
                            .and_then(|c| c.check().ok());
                        if attached.is_none() {
                            strip.unmap();
                            return None;
                        }
                        Some(Self { seg, ..strip })
                    }
                }
                #[cfg(not(target_os = "linux"))]
                {
                    let _ = (conn, len);
                    None
                }
            }

            fn release(self, conn: &RustConnection) {
                let _ = conn.shm_detach(self.seg);
                let _ = conn.flush();
                self.unmap();
            }

            fn unmap(self) {
                // SAFETY: see the top of the module; mapped in `new`, and
                // nothing uses it after this.
                unsafe {
                    libc::munmap(self.ptr.as_ptr().cast(), self.len);
                }
            }
        }
    }
}
