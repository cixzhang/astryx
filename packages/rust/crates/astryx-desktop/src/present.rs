//! The CPU path's window: the CPU renderer's pixels, copied to the window
//! through `softbuffer` (shared memory on X11 where the server has it).
//!
//! An X server without a compositor does not keep a window's pixels: when
//! another window uncovers part of it, the server sends Expose and that part
//! must be sent again. Only rows that changed are sent otherwise, so the
//! presenter watches for Expose itself, on an X connection of its own.

use std::error::Error;
use std::num::NonZeroU32;
use std::sync::Arc;

use astryx_render_soft::tiny_skia::Pixmap;
use winit::window::Window;

/// Shows frames in a window.
pub(crate) struct Present {
    // Field order is drop order: the surface goes before its context.
    surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    _context: softbuffer::Context<Arc<Window>>,
    /// On X11: where the server asks for the window's pixels again.
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    ))]
    exposes: Option<Box<x11::Exposes>>,
}

impl Present {
    pub(crate) fn new(window: &Arc<Window>) -> Result<Self, Box<dyn Error>> {
        let context = softbuffer::Context::new(window.clone())?;
        let surface = softbuffer::Surface::new(&context, window.clone())?;
        Ok(Self {
            surface,
            _context: context,
            #[cfg(all(
                unix,
                not(any(target_os = "macos", target_os = "ios", target_os = "android"))
            ))]
            exposes: x11::Exposes::watch(window).map(Box::new),
        })
    }

    /// Shows `frame`, premultiplied RGBA. The window is opaque, so alpha is
    /// dropped. When the buffer holds the frame on screen and the server
    /// still has the window's pixels, only rows that changed are written and
    /// sent; an unchanged frame sends nothing.
    pub(crate) fn show(&mut self, frame: &Pixmap) -> Result<(), softbuffer::SoftBufferError> {
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
}

impl Present {
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
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use winit::window::Window;
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{ChangeWindowAttributesAux, ConnectionExt, EventMask};
    use x11rb::protocol::Event;
    use x11rb::rust_connection::RustConnection;

    /// Expose events for one window, on a connection of their own: each X
    /// client chooses its own events, so this leaves winit's alone.
    pub(super) struct Exposes {
        conn: RustConnection,
    }

    impl Exposes {
        /// `None` when the window is not on X11 or the server cannot be
        /// reached; frames then go as before.
        pub(super) fn watch(window: &Window) -> Option<Self> {
            let xid = match window.window_handle().ok()?.as_raw() {
                RawWindowHandle::Xlib(w) => u32::try_from(w.window).ok()?,
                RawWindowHandle::Xcb(w) => w.window.get(),
                _ => return None,
            };
            let (conn, _) = x11rb::connect(None).ok()?;
            let mask = ChangeWindowAttributesAux::new().event_mask(EventMask::EXPOSURE);
            conn.change_window_attributes(xid, &mask)
                .ok()?
                .check()
                .ok()?;
            Some(Self { conn })
        }

        pub(super) fn exposed(&self) -> bool {
            // A round trip first: the server sends this connection every
            // Expose it made before answering, so none that woke the frame
            // is still on its way.
            let synced = self
                .conn
                .get_input_focus()
                .ok()
                .and_then(|c| c.reply().ok())
                .is_some();
            let mut exposed = !synced;
            while let Ok(Some(event)) = self.conn.poll_for_event() {
                exposed |= matches!(event, Event::Expose(_));
            }
            exposed
        }
    }
}
