//! OpenGL on a thread of its own (X11). The CPU renderer draws the first
//! frames while this thread loads the GL driver and makes a config that fits
//! the window, the context, the window surface and the GPU renderer: about
//! 150 ms on NVIDIA. Its driver also charges each thread 50 to 100 ms the
//! first time a context is made current on it, so the context stays on this
//! thread: from the switch on, it draws every frame. The main thread hands
//! over each frame's display list and waits until the frame is swapped, so
//! frame timing, transitions and the accessibility update after each
//! present are the same as when the main thread draws.
//!
//! The thread talks to the X server over a connection of its own: GLX
//! holds its connection's lock across driver calls, and sharing the event
//! loop's connection would stall input and presents for up to a quarter of
//! a second while OpenGL starts.

use std::error::Error;
use std::ffi::CString;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::JoinHandle;

use astryx_core::{DisplayList, FrameInfo, ImageData, ImageId, Transition};
use astryx_render_gl::GlRenderer;
use glutin::config::{ConfigTemplateBuilder, GlConfig};
use glutin::context::{NotCurrentGlContext, PossiblyCurrentContext};
use glutin::display::{Display, DisplayApiPreference, GlDisplay};
use glutin::platform::x11::X11GlConfigExt;
use glutin::surface::{GlSurface, Surface, SwapInterval, WindowSurface};
use glutin_winit::GlWindow;
use winit::raw_window_handle::{
    HasWindowHandle, RawDisplayHandle, RawWindowHandle, XlibDisplayHandle,
};
use winit::window::Window;

/// One frame for the thread to draw and swap.
struct Job {
    list: DisplayList,
    frame: FrameInfo,
    /// Image changes since the last frame.
    images: Vec<(ImageId, Option<ImageData>)>,
    /// A transition from this list to the frame.
    transition: Option<(Transition, DisplayList)>,
    /// Writes the frame to this PNG file.
    screenshot: Option<PathBuf>,
    /// Shows the frame. A frame drawn but not shown warms the text and
    /// shadow caches before the switch.
    swap: bool,
}

enum Reply {
    /// OpenGL started; the thread waits for frames.
    Ready,
    /// OpenGL did not start; the thread has ended.
    Failed(String),
    /// A frame is drawn, not shown, ahead of the switch.
    Prepared,
    /// A frame is drawn and swapped. The list comes back for the shell to
    /// keep as the frame on screen.
    Drawn {
        list: DisplayList,
        transitioning: bool,
        result: Result<(), String>,
    },
}

/// What [`GlThread::poll`] found.
pub(crate) enum Started {
    /// OpenGL has started: draw a frame ahead with [`GlThread::prepare`].
    Ready,
    /// The frame ahead is drawn: the thread can take over.
    Prepared,
}

/// The thread, starting or drawing.
pub(crate) struct GlThread {
    // Dropping the job channel ends the thread; dropping the struct then
    // waits for it.
    jobs: Option<mpsc::Sender<Job>>,
    replies: mpsc::Receiver<Reply>,
    thread: Option<JoinHandle<()>>,
    images: Vec<(ImageId, Option<ImageData>)>,
    transition: Option<(Transition, DisplayList)>,
    transitioning: bool,
    /// A `Prepared` answer the shell asked to see again.
    prepared_again: bool,
}

impl GlThread {
    /// Starts OpenGL for `window`. The shell follows it with
    /// [`Self::poll`] between frames; the thread never wakes the event loop
    /// itself, as the loop may have ended.
    pub(crate) fn spawn(window: Arc<Window>) -> Self {
        let (jobs, inbox) = mpsc::channel::<Job>();
        let (outbox, replies) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("astryx-gl".into())
            .spawn(move || run(window, inbox, outbox));
        let (thread, jobs) = match spawned {
            Ok(t) => (Some(t), Some(jobs)),
            Err(e) => {
                // No thread: report the failure on the first poll.
                let (outbox, failed) = mpsc::channel();
                let _ = outbox.send(Reply::Failed(format!("no thread for OpenGL: {e}")));
                return Self {
                    jobs: None,
                    replies: failed,
                    thread: None,
                    images: Vec::new(),
                    transition: None,
                    transitioning: false,
                    prepared_again: false,
                };
            }
        };
        Self {
            jobs,
            replies,
            thread,
            images: Vec::new(),
            transition: None,
            transitioning: false,
            prepared_again: false,
        }
    }

    /// Whether OpenGL has started or drawn its frame ahead (`Some(Ok)`),
    /// failed (`Some(Err)`), or neither yet (`None`).
    pub(crate) fn poll(&mut self) -> Option<Result<Started, String>> {
        match self.replies.try_recv() {
            Ok(Reply::Ready) => Some(Ok(Started::Ready)),
            Ok(Reply::Prepared) => Some(Ok(Started::Prepared)),
            Ok(Reply::Failed(e)) => {
                self.finish();
                Some(Err(e))
            }
            Ok(Reply::Drawn { .. }) => Some(Err("OpenGL thread drew before it started".into())),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.finish();
                Some(Err("OpenGL thread ended".into()))
            }
        }
    }

    pub(crate) fn set_image(&mut self, id: ImageId, data: Option<ImageData>) {
        self.images.push((id, data));
    }

    pub(crate) fn begin_transition(&mut self, transition: &Transition, from: &DisplayList) {
        self.transition = Some((*transition, from.clone()));
    }

    pub(crate) fn transitioning(&self) -> bool {
        self.transitioning
    }

    /// Keeps a `Prepared` answer for [`Self::prepared`].
    pub(crate) fn again(&mut self) {
        self.prepared_again = true;
    }

    /// Whether the frame ahead is drawn, as [`Self::poll`] answered.
    pub(crate) fn prepared(&self) -> bool {
        self.prepared_again
    }

    /// Draws `list` without showing it, so the frames after the switch find
    /// their text and shadows ready; does not wait. [`Self::poll`] then
    /// answers `Prepared`.
    pub(crate) fn prepare(&mut self, list: DisplayList, frame: FrameInfo) {
        let job = Job {
            list,
            frame,
            images: std::mem::take(&mut self.images),
            transition: None,
            screenshot: None,
            swap: false,
        };
        if let Some(jobs) = &self.jobs {
            let _ = jobs.send(job);
        }
    }

    /// Draws and swaps `list`, waiting until the frame is swapped; gives the
    /// list back.
    pub(crate) fn draw(
        &mut self,
        list: DisplayList,
        frame: FrameInfo,
        screenshot: Option<PathBuf>,
    ) -> Result<DisplayList, Box<dyn Error>> {
        let job = Job {
            list,
            frame,
            images: std::mem::take(&mut self.images),
            transition: self.transition.take(),
            screenshot,
            swap: true,
        };
        let sent = self.jobs.as_ref().is_some_and(|j| j.send(job).is_ok());
        if !sent {
            return Err("OpenGL thread ended".into());
        }
        match self.replies.recv() {
            Ok(Reply::Drawn {
                list,
                transitioning,
                result,
            }) => {
                self.transitioning = transitioning;
                result.map(|()| list).map_err(Into::into)
            }
            Ok(_) | Err(_) => Err("OpenGL thread ended".into()),
        }
    }

    /// Ends the thread and waits for it.
    fn finish(&mut self) {
        self.jobs = None;
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for GlThread {
    // The thread holds the window and GL objects on its display connection:
    // it finishes before the window goes.
    fn drop(&mut self) {
        self.finish();
    }
}

/// The thread: start OpenGL, then draw frames until the job channel closes.
fn run(window: Arc<Window>, inbox: mpsc::Receiver<Job>, outbox: mpsc::Sender<Reply>) {
    let connection = match XConnection::open() {
        Ok(c) => c,
        Err(e) => {
            let _ = outbox.send(Reply::Failed(e.to_string()));
            return;
        }
    };
    let started = start(&window, &connection);
    let (mut renderer, surface, context, display) = match started {
        Ok(gl) => gl,
        Err(e) => {
            let _ = outbox.send(Reply::Failed(e.to_string()));
            return;
        }
    };
    let _ = outbox.send(Reply::Ready);
    let mut size = window.inner_size();
    while let Ok(job) = inbox.recv() {
        for (id, data) in &job.images {
            renderer.set_image(*id, data.as_ref());
        }
        if (job.frame.width, job.frame.height) != (size.width, size.height) {
            if let (Some(w), Some(h)) = (
                NonZeroU32::new(job.frame.width),
                NonZeroU32::new(job.frame.height),
            ) {
                surface.resize(&context, w, h);
            }
            size = winit::dpi::PhysicalSize::new(job.frame.width, job.frame.height);
        }
        if let Some((t, from)) = &job.transition {
            renderer.begin_transition(t, from, &job.frame);
        }
        renderer.render_list(&job.list, &job.frame);
        if !job.swap {
            if outbox.send(Reply::Prepared).is_err() {
                break;
            }
            continue;
        }
        let mut result = Ok(());
        if let Some(path) = &job.screenshot {
            result =
                crate::save_gl_screenshot(&mut renderer, job.frame.width, job.frame.height, path)
                    .map_err(|e| e.to_string());
        }
        if result.is_ok() {
            result = surface.swap_buffers(&context).map_err(|e| e.to_string());
        }
        let reply = Reply::Drawn {
            list: job.list,
            transitioning: renderer.transitioning(),
            result,
        };
        if outbox.send(reply).is_err() {
            break;
        }
    }
    // The renderer's GL objects go while the context is current, then the
    // surface, the context, and the connection last.
    drop(renderer);
    drop(surface);
    drop(context);
    drop(display);
    drop(connection);
}

/// An Xlib connection of the thread's own to the window's X server.
struct XConnection {
    xlib: x11_dl::xlib::Xlib,
    display: std::ptr::NonNull<x11_dl::xlib::Display>,
}

impl XConnection {
    fn open() -> Result<Self, Box<dyn Error>> {
        let xlib = x11_dl::xlib::Xlib::open()?;
        // The display the event loop opened: both follow DISPLAY.
        // SAFETY: a null name opens the display named by DISPLAY.
        let display = unsafe { (xlib.XOpenDisplay)(std::ptr::null()) };
        let display = std::ptr::NonNull::new(display).ok_or("cannot open the X display")?;
        Ok(Self { xlib, display })
    }

    fn handle(&self) -> RawDisplayHandle {
        // SAFETY: the connection is open.
        let screen = unsafe { (self.xlib.XDefaultScreen)(self.display.as_ptr()) };
        RawDisplayHandle::Xlib(XlibDisplayHandle::new(Some(self.display.cast()), screen))
    }
}

impl Drop for XConnection {
    fn drop(&mut self) {
        // SAFETY: opened in `open`; nothing uses it after this.
        unsafe { (self.xlib.XCloseDisplay)(self.display.as_ptr()) };
    }
}

type Gl = (
    GlRenderer,
    Surface<WindowSurface>,
    PossiblyCurrentContext,
    Display,
);

/// Starts OpenGL on this thread for `window`, over `connection`.
fn start(window: &Window, connection: &XConnection) -> Result<Gl, Box<dyn Error>> {
    let raw_display = connection.handle();
    let raw_window = window.window_handle()?.as_raw();
    // As glutin-winit does on X11: GLX first, then EGL.
    let preference =
        DisplayApiPreference::GlxThenEgl(Box::new(winit::platform::x11::register_xlib_error_hook));
    // SAFETY: the connection is open for as long as the display is used.
    let display = unsafe { Display::new(raw_display, preference)? };
    // A config whose visual is the window's own: the window is already on
    // screen, with the default visual. glutin does not filter GLX configs by
    // the window, so match the visual here. The window is opaque, as it is
    // on the GL path, so alpha in the window buffer is optional.
    let visual = match raw_window {
        RawWindowHandle::Xlib(w) => Some(w.visual_id),
        RawWindowHandle::Xcb(w) => w.visual_id.map(|v| u64::from(v.get())),
        _ => None,
    };
    // glutin asks for 8 bits of alpha unless told otherwise.
    let template = ConfigTemplateBuilder::new()
        .with_alpha_size(0)
        .with_stencil_size(8)
        .compatible_with_native_window(raw_window)
        .build();
    // SAFETY: as above.
    let config = unsafe { display.find_configs(template)? }
        .filter(|c| visual.is_none() || c.x11_visual().map(|v| v.visual_id()) == visual)
        // Alpha when offered, then fewer samples: the renderer antialiases
        // itself.
        .min_by_key(|c| (c.alpha_size() < 8, c.num_samples()))
        .ok_or("no GL config fits the window")?;
    let context = crate::make_context(&display, &config, raw_window)?;
    let attributes = window.build_surface_attributes(Default::default())?;
    // SAFETY: the config belongs to this display and the window is live.
    let surface = unsafe { display.create_window_surface(&config, &attributes)? };
    let context = context.make_current(&surface)?;
    let _ = surface.set_swap_interval(&context, SwapInterval::Wait(NonZeroU32::MIN));
    // SAFETY: the context is current on this thread.
    let mut renderer = unsafe {
        GlRenderer::from_loader(|name| {
            let name = CString::from(name);
            display.get_proc_address(&name)
        })
    }
    .map_err(|e| format!("GPU renderer: {e:?}"))?;
    // The renderer warms the driver up on its second frame. Two empty
    // frames, never swapped, do that here instead of in the frames that
    // follow the switch.
    let size = window.inner_size();
    let frame = FrameInfo {
        width: size.width.max(1),
        height: size.height.max(1),
        scale: window.scale_factor() as f32,
        dt: 0.0,
    };
    let empty = DisplayList::default();
    renderer.render_list(&empty, &frame);
    renderer.render_list(&empty, &frame);
    Ok((renderer, surface, context, display))
}
