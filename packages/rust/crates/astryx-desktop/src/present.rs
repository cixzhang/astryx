//! The CPU path's window: the CPU renderer's pixels, copied to the window
//! through `softbuffer` (shared memory on X11 where the server has it).

use std::error::Error;
use std::num::NonZeroU32;
use std::rc::Rc;

use astryx_render_soft::tiny_skia::Pixmap;
use winit::window::Window;

/// Shows frames in a window.
pub(crate) struct Present {
    // Field order is drop order: the surface goes before its context.
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    _context: softbuffer::Context<Rc<Window>>,
}

impl Present {
    pub(crate) fn new(window: &Rc<Window>) -> Result<Self, Box<dyn Error>> {
        let context = softbuffer::Context::new(window.clone())?;
        let surface = softbuffer::Surface::new(&context, window.clone())?;
        Ok(Self {
            surface,
            _context: context,
        })
    }

    /// Shows `frame`, premultiplied RGBA. The window is opaque, so alpha is
    /// dropped. When the buffer holds the frame on screen, only rows that
    /// changed are written and sent; an unchanged frame sends nothing.
    pub(crate) fn show(&mut self, frame: &Pixmap) -> Result<(), softbuffer::SoftBufferError> {
        let (Some(w), Some(h)) = (
            NonZeroU32::new(frame.width()),
            NonZeroU32::new(frame.height()),
        ) else {
            return Ok(());
        };
        self.surface.resize(w, h)?;
        let mut buffer = self.surface.buffer_mut()?;
        // Age 1: the buffer holds the frame on screen. Otherwise its pixels
        // are new or older, and every row goes.
        let whole = buffer.age() != 1;
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
