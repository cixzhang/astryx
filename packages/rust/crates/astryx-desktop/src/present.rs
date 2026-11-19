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
}

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
        })
    }

    /// As [`Present::show`]; when the buffer holds the frame on screen, only
    /// rows that changed are written into it.
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
        // Age 1: the buffer holds the frame on screen. Otherwise its pixels
        // are new or older, and every row goes; so it does when the server
        // has dropped part of the window.
        let whole = buffer.age() != 1 || exposed;
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
        if whole {
            return buffer.present();
        }
        if first > last {
            // Nothing changed: the frame on screen stays.
            return Ok(());
        }
        let rows = NonZeroU32::new((last - first + 1) as u32).expect("at least one row");
        buffer.present_with_damage(&[softbuffer::Rect {
            x: 0,
            y: first as u32,
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

    /// The most rows one image carries: enough to keep requests few, small
    /// enough to keep the buffer that builds them small.
    const MOST_ROWS: usize = 32;

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
        /// Rows converted for the server, reused.
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
                strip: Vec::new(),
            })
        }

        pub(in super::super) fn show(&mut self, frame: &Pixmap) -> Result<(), Box<dyn Error>> {
            let (w, h) = (frame.width(), frame.height());
            if w == 0 || h == 0 {
                return Ok(());
            }
            if exposed(&self.conn) || self.size != (w, h) {
                self.rows.clear();
            }
            self.size = (w, h);
            let known = !self.rows.is_empty();
            self.rows.resize(h as usize, 0);
            let row_bytes = w as usize * 4;
            // Rows per image: what one request carries, at most MOST_ROWS.
            let fit = (self.conn.maximum_request_bytes().saturating_sub(64) / row_bytes).max(1);
            let per = fit.min(MOST_ROWS);
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
            self.strip.clear();
            self.strip.extend(
                src.as_chunks::<4>()
                    .0
                    .iter()
                    .flat_map(|p| [p[2], p[1], p[0], 0xff]),
            );
            self.conn.put_image(
                ImageFormat::Z_PIXMAP,
                self.window,
                self.gc,
                w as u16,
                (end - first) as u16,
                0,
                first as i16,
                0,
                self.depth,
                &self.strip,
            )?;
            Ok(())
        }
    }

    /// A 64-bit hash of one row's bytes, for telling changed rows.
    fn row_hash(row: &[u8]) -> u64 {
        let (words, rest) = row.as_chunks::<8>();
        let mut h: u64 = 0x243f_6a88_85a3_08d3;
        for w in words {
            h = (h ^ u64::from_le_bytes(*w))
                .wrapping_mul(0x9e37_79b9_7f4a_7c15)
                .rotate_left(29);
        }
        for &b in rest {
            h = (h ^ u64::from(b)).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        }
        h
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
}
