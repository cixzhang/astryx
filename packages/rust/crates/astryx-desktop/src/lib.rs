//! Desktop Astryx shell: one window drawn by the GPU renderer, with keyboard
//! and gamepad input and an AccessKit bridge for screen readers.
//!
//! ```no_run
//! # struct App;
//! # impl astryx_core::Surface for App {
//! #     fn input(&mut self, _: astryx_core::Input, _: astryx_core::InputPhase) -> astryx_core::Command { Default::default() }
//! #     fn frame(&mut self, _: &astryx_core::FrameInfo, _: &dyn astryx_core::TextMeasure) -> astryx_core::DisplayList { Default::default() }
//! #     fn accessibility(&self) -> Option<astryx_core::A11yNode> { None }
//! #     fn wants_frames(&self) -> bool { false }
//! # }
//! astryx_desktop::run(App).unwrap();
//! ```
//!
//! Where OpenGL runs only on the CPU, the CPU renderer draws the window
//! instead: on an X server with neither DRI3 nor NV-GLX (Xvfb, most VMs), on
//! a Wayland compositor that takes no GPU buffers or a machine without a
//! GPU render node, with `LIBGL_ALWAYS_SOFTWARE` set, or when no OpenGL
//! context can be made.
//!
//! On an X server with a GPU, the CPU renderer draws the first frames while
//! OpenGL starts on a thread of its own, which then draws every frame.
//! `ASTRYX_RENDERER=gl` (OpenGL on the main thread from the first frame),
//! `cpu-first` or `cpu` chooses outright.

pub mod a11y;
#[cfg(feature = "audio")]
mod audio;
mod backend;
#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "ios", target_os = "android"))
))]
mod gl_thread;
pub mod input;
pub mod platform;
pub mod pointer;
mod present;

use std::error::Error;
use std::ffi::CString;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use astryx_core::{
    A11yAction, A11yNode, Command, FrameInfo, Input, InputPhase, Point, Renderer, Surface,
};
use astryx_render_gl::GlRenderer;
use astryx_render_soft::tiny_skia::{IntSize, Pixmap};
use astryx_render_soft::SoftRenderer;
use glutin::config::{Config, ConfigTemplateBuilder, GlConfig};
use glutin::context::{
    ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext, Version,
};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::{GlSurface, Surface as GlutinSurface, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::ModifiersState;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle};
use winit::window::{CursorIcon, Window, WindowAttributes, WindowId};

/// Window and run settings.
#[derive(Clone, Debug)]
pub struct Options {
    pub title: String,
    /// Initial inner size in logical points.
    pub size: (u32, u32),
    /// Writes the first frame, read back from the renderer, to this PNG file.
    pub screenshot: Option<PathBuf>,
    /// Exits after the screenshot is written.
    pub exit_after_screenshot: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            title: "Astryx".into(),
            size: (960, 640),
            screenshot: None,
            exit_after_screenshot: false,
        }
    }
}

/// Opens a window and runs `surface` until it quits or the window closes.
pub fn run<S: Surface>(surface: S) -> Result<(), Box<dyn Error>> {
    run_with(surface, Options::default())
}

/// [`run`] with explicit [`Options`].
pub fn run_with<S: Surface>(surface: S, options: Options) -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::<accesskit_winit::Event>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let mut shell = Shell {
        surface,
        options,
        proxy,
        screen: None,
        error: None,
        modifiers: ModifiersState::empty(),
        text_field: None,
        last_frame: None,
        last_tree: None,
        frames: 0,
        pointer: None,
        pointer_target: None,
        cursor: CursorIcon::Default,
        #[cfg(feature = "gamepad")]
        gamepads: None,
        #[cfg(feature = "gamepad")]
        preferences: astryx_core::Preferences::default(),
        pending_transition: None,
        last_list: None,
        #[cfg(feature = "audio")]
        audio: None,
        devices_open: false,
        #[cfg(feature = "audio")]
        early_sounds: Vec::new(),
        gl_images: std::collections::HashMap::new(),
        shot_pending: false,
    };
    shell.shot_pending = shell.options.screenshot.is_some();
    event_loop.run_app(&mut shell)?;
    match shell.error.take() {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// The window and what draws it.
struct Screen {
    // Field order is drop order: the drawing and OpenGL starting go before
    // the window.
    draw: Draw,
    /// OpenGL starting on its own thread while the CPU renderer draws.
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    ))]
    gl_thread: Option<gl_thread::GlThread>,
    /// OpenGL starts after the first frame is on screen: loading its
    /// driver would hold up the first frame's threads.
    gl_wanted: bool,
    adapter: accesskit_winit::Adapter,
    window: Arc<Window>,
}

enum Draw {
    Gl {
        // Field order is drop order: the renderer's GL objects go before
        // the context.
        renderer: Box<GlRenderer>,
        gl_surface: GlutinSurface<WindowSurface>,
        context: PossiblyCurrentContext,
    },
    Cpu {
        renderer: Box<SoftRenderer>,
        present: present::Present,
    },
    /// OpenGL on its own thread. The CPU renderer that drew the first
    /// frames stays for measuring text, so its caches stay warm.
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    ))]
    Thread {
        gl: gl_thread::GlThread,
        measurer: Box<SoftRenderer>,
    },
}

impl Draw {
    fn transitioning(&self) -> bool {
        match self {
            Draw::Gl { renderer, .. } => renderer.transitioning(),
            Draw::Cpu { renderer, .. } => renderer.transitioning(),
            #[cfg(all(
                unix,
                not(any(target_os = "macos", target_os = "ios", target_os = "android"))
            ))]
            Draw::Thread { gl, .. } => gl.transitioning(),
        }
    }
}

struct Shell<S: Surface> {
    surface: S,
    options: Options,
    proxy: winit::event_loop::EventLoopProxy<accesskit_winit::Event>,
    screen: Option<Screen>,
    error: Option<Box<dyn Error>>,
    modifiers: ModifiersState,
    /// Bounds of the focused text field, when one has focus.
    text_field: Option<astryx_core::Rect>,
    last_frame: Option<Instant>,
    last_tree: Option<Option<A11yNode>>,
    frames: u64,
    /// The pointer position in logical points, while it is over the window.
    pointer: Option<Point>,
    /// The node a press landed on, activated if the release lands on it too.
    pointer_target: Option<u64>,
    cursor: CursorIcon,
    #[cfg(feature = "gamepad")]
    gamepads: Option<gilrs::Gilrs>,
    /// What the shell last passed to `Surface::set_preferences`.
    #[cfg(feature = "gamepad")]
    preferences: astryx_core::Preferences,
    /// A transition the app asked for, started with the next frame.
    pending_transition: Option<astryx_core::Transition>,
    /// The frame on screen, where a transition starts from.
    last_list: Option<astryx_core::DisplayList>,
    #[cfg(feature = "audio")]
    audio: Option<audio::Audio>,
    /// Gamepads and the audio output open once the first frame is on
    /// screen, so they do not delay it.
    devices_open: bool,
    /// Sounds played before the audio output opened.
    #[cfg(feature = "audio")]
    early_sounds: Vec<astryx_core::SoundId>,
    /// While OpenGL starts: the images the app gave the CPU renderer, for
    /// the GPU renderer that takes over.
    gl_images: std::collections::HashMap<astryx_core::ImageId, astryx_core::ImageData>,
    /// The screenshot is still to be written: from the first frame of the
    /// renderer that keeps drawing.
    shot_pending: bool,
}

/// How often an idle shell asks the surface whether it wants a frame, so a
/// surface that changes by itself is drawn within this time: about ten
/// wakeups a second and no frames while nothing changes.
const IDLE_POLL: std::time::Duration = std::time::Duration::from_millis(100);

fn pick_config(configs: Box<dyn Iterator<Item = Config> + '_>) -> Config {
    // Prefer no multisampling: the renderer antialiases itself.
    configs
        .reduce(|best, c| {
            if c.num_samples() < best.num_samples() {
                c
            } else {
                best
            }
        })
        .expect("at least one GL config")
}

/// A context for `config`: OpenGL 3.3, else OpenGL ES 3.0, else ES 2.0.
fn make_context(
    display: &glutin::display::Display,
    config: &Config,
    handle: winit::raw_window_handle::RawWindowHandle,
) -> Result<glutin::context::NotCurrentContext, Box<dyn Error>> {
    for api in [
        ContextApi::OpenGl(Some(Version::new(3, 3))),
        ContextApi::Gles(Some(Version::new(3, 0))),
        ContextApi::Gles(Some(Version::new(2, 0))),
    ] {
        let attrs = ContextAttributesBuilder::new()
            .with_context_api(api)
            .build(Some(handle));
        // SAFETY: the config belongs to this display and the handle to a live window.
        if let Ok(c) = unsafe { display.create_context(config, &attrs) } {
            return Ok(c);
        }
    }
    Err("no OpenGL or OpenGL ES context available".into())
}

impl<S: Surface> Shell<S> {
    fn window_attributes(&self) -> WindowAttributes {
        let (w, h) = self.options.size;
        WindowAttributes::default()
            .with_title(self.options.title.clone())
            .with_inner_size(LogicalSize::new(w, h))
            // AccessKit must attach before the window is first shown.
            .with_visible(false)
    }

    /// Opens the window with the renderer for this machine
    /// ([`backend::choose`]); without OpenGL, with the CPU renderer.
    fn create(&mut self, event_loop: &ActiveEventLoop) -> Result<Screen, Box<dyn Error>> {
        let display = match event_loop.display_handle()?.as_raw() {
            RawDisplayHandle::Xlib(_) | RawDisplayHandle::Xcb(_) => backend::Display::X11,
            RawDisplayHandle::Wayland(_) => backend::Display::Wayland,
            _ => backend::Display::Other,
        };
        match backend::choose(&backend::facts(display)) {
            backend::Backend::Gl => match self.create_gl(event_loop) {
                Ok(screen) => return Ok(screen),
                Err(e) => {
                    eprintln!("astryx-desktop: no OpenGL ({e}); drawing with the CPU renderer")
                }
            },
            backend::Backend::CpuFirst => {
                let mut screen = self.create_cpu(event_loop)?;
                screen.gl_wanted = true;
                return Ok(screen);
            }
            backend::Backend::Cpu => {}
        }
        self.create_cpu(event_loop)
    }

    fn create_cpu(&mut self, event_loop: &ActiveEventLoop) -> Result<Screen, Box<dyn Error>> {
        let window = Arc::new(event_loop.create_window(self.window_attributes())?);
        let present = present::Present::new(&window)?;
        self.show(
            event_loop,
            window,
            Draw::Cpu {
                renderer: Box::new(SoftRenderer::new()),
                present,
            },
        )
    }

    fn create_gl(&mut self, event_loop: &ActiveEventLoop) -> Result<Screen, Box<dyn Error>> {
        let attributes = self.window_attributes();
        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_stencil_size(8);
        let (window, config) = DisplayBuilder::new()
            .with_window_attributes(Some(attributes.clone()))
            .build(event_loop, template, pick_config)?;
        let window = match window {
            Some(w) => w,
            None => glutin_winit::finalize_window(event_loop, attributes, &config)?,
        };
        let handle = window.window_handle()?.as_raw();
        let display = config.display();
        let context = make_context(&display, &config, handle)?;
        let surface_attrs = window.build_surface_attributes(Default::default())?;
        // SAFETY: as above.
        let gl_surface = unsafe { display.create_window_surface(&config, &surface_attrs)? };
        let context = context.make_current(&gl_surface)?;
        let _ = gl_surface.set_swap_interval(&context, SwapInterval::Wait(NonZeroU32::MIN));
        // SAFETY: the context is current on this thread.
        let renderer = unsafe {
            GlRenderer::from_loader(|name| {
                let name = CString::from(name);
                display.get_proc_address(&name)
            })
        }
        .map_err(|e| format!("GPU renderer: {e:?}"))?;
        self.show(
            event_loop,
            Arc::new(window),
            Draw::Gl {
                renderer: Box::new(renderer),
                gl_surface,
                context,
            },
        )
    }

    /// Attaches AccessKit, shows the window and asks for its first frame.
    fn show(
        &mut self,
        event_loop: &ActiveEventLoop,
        window: Arc<Window>,
        draw: Draw,
    ) -> Result<Screen, Box<dyn Error>> {
        let adapter = accesskit_winit::Adapter::with_event_loop_proxy(
            event_loop,
            &window,
            self.proxy.clone(),
        );
        window.set_visible(true);
        window.request_redraw();
        Ok(Screen {
            draw,
            #[cfg(all(
                unix,
                not(any(target_os = "macos", target_os = "ios", target_os = "android"))
            ))]
            gl_thread: None,
            gl_wanted: false,
            adapter,
            window,
        })
    }

    /// Asks for one more frame (a no-op when one is already pending).
    fn request_redraw(&self) {
        if let Some(screen) = &self.screen {
            screen.window.request_redraw();
        }
    }

    /// Carries out `command`, and asks for a frame only when it changes
    /// what is shown. A frame for any other reason comes from what changed
    /// it: an input ([`Self::after_input`]), a resize, or the surface
    /// wanting frames while something moves.
    fn handle(&mut self, event_loop: &ActiveEventLoop, command: Command) {
        if needs_frame(&command) {
            self.request_redraw();
        }
        self.carry_out(event_loop, command);
    }

    /// An input reached the surface: draw its effect. The surface may have
    /// changed state even when it returned no command (focus moved, a
    /// press began), and a surface without a runtime cannot say so.
    fn after_input(&mut self, event_loop: &ActiveEventLoop, command: Command) {
        self.request_redraw();
        self.carry_out(event_loop, command);
    }

    fn carry_out(&mut self, event_loop: &ActiveEventLoop, command: Command) {
        match command {
            // The runtime carries out focus requests; none reach the shell.
            Command::None | Command::Handled | Command::Focus(_) => {}
            Command::Batch(commands) => {
                for command in commands {
                    self.carry_out(event_loop, command);
                }
            }
            Command::Quit => event_loop.exit(),
            Command::OpenUrl(url) => {
                if let Err(e) = platform::open_url(&url) {
                    eprintln!("astryx-desktop: cannot open {url:?}: {e}");
                }
            }
            Command::Transition(t) => {
                if let Some(id) = t.sound {
                    self.play(id);
                }
                self.pending_transition = Some(t);
            }
            Command::PlaySound(id) => self.play(id),
        }
    }

    fn play(&mut self, id: astryx_core::SoundId) {
        #[cfg(feature = "audio")]
        if let Some(a) = &self.audio {
            a.control.play(id);
        } else if !self.devices_open {
            self.early_sounds.push(id);
        }
        #[cfg(not(feature = "audio"))]
        let _ = id;
    }

    /// Opens gamepads and the audio output, after the first frame is on
    /// screen; hands the audio output the sounds registered and played
    /// before.
    fn open_devices(&mut self) {
        if !std::mem::replace(&mut self.devices_open, true) {
            #[cfg(feature = "gamepad")]
            {
                self.gamepads = gilrs::Gilrs::new().ok();
            }
            #[cfg(feature = "audio")]
            {
                self.audio = audio::Audio::open()
                    .map_err(|e| eprintln!("astryx-desktop: no sound: {e}"))
                    .ok();
                self.take_sounds();
                for id in std::mem::take(&mut self.early_sounds) {
                    self.play(id);
                }
            }
        }
    }

    /// Hands sounds the app registered to the audio output. Until it opens
    /// they wait in the surface.
    fn take_sounds(&mut self) {
        if !self.devices_open {
            return;
        }
        let updates = self.surface.take_sound_updates();
        #[cfg(feature = "audio")]
        if let Some(a) = &self.audio {
            for (id, wav) in updates {
                if let Err(e) = a.control.set_sound(id, wav.as_deref()) {
                    eprintln!("astryx-desktop: sound {id:?}: {e}");
                }
            }
        }
        #[cfg(not(feature = "audio"))]
        drop(updates);
    }

    fn send(&mut self, event_loop: &ActiveEventLoop, input: Input, phase: InputPhase) {
        let command = self.surface.input(input, phase);
        self.after_input(event_loop, command);
    }

    fn type_text(&mut self, event_loop: &ActiveEventLoop, text: &str) {
        let command = self.surface.text_input(text);
        self.after_input(event_loop, command);
    }

    /// The node under the pointer in the last frame's tree.
    fn pointer_hit(&self) -> Option<&A11yNode> {
        let at = self.pointer?;
        let tree = self.last_tree.as_ref()?.as_ref()?;
        pointer::hit_test(tree, at.x, at.y)
    }

    /// Tells the surface where the mouse is, for hover, and repaints.
    fn hover(&mut self) {
        self.surface.pointer_move(self.pointer);
        if let Some(screen) = &self.screen {
            screen.window.request_redraw();
        }
    }

    fn update_cursor(&mut self) {
        let cursor = pointer::cursor_for(self.pointer_hit());
        if cursor != self.cursor {
            self.cursor = cursor;
            if let Some(screen) = &self.screen {
                screen.window.set_cursor(cursor);
            }
        }
    }

    /// Turns the input method on while a text field has focus, off when
    /// focus leaves it, and keeps its candidate window at that field.
    fn update_text_field(&mut self, tree: Option<&A11yNode>) {
        let field = input::ime_area(tree);
        if field == self.text_field {
            return;
        }
        let Some(screen) = &self.screen else { return };
        if field.is_some() != self.text_field.is_some() {
            screen.window.set_ime_allowed(field.is_some());
        }
        if let Some(r) = field {
            screen.window.set_ime_cursor_area(
                LogicalPosition::new(r.x, r.y),
                LogicalSize::new(r.width, r.height),
            );
        }
        self.text_field = field;
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        self.take_sounds();
        let Some(screen) = &mut self.screen else {
            return;
        };
        let size = screen.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        let now = Instant::now();
        let dt = self.last_frame.map_or(0.0, |t| (now - t).as_secs_f32());
        self.last_frame = Some(now);
        let frame = FrameInfo {
            width: size.width,
            height: size.height,
            scale: screen.window.scale_factor() as f32,
            dt,
        };
        take_gl(screen, &self.gl_images);
        let starting_gl = screen.starting_gl();
        if !starting_gl {
            self.gl_images.clear();
        }
        let updates = self.surface.take_image_updates();
        // The screenshot comes from the renderer that keeps drawing.
        let shot = if self.shot_pending && !starting_gl {
            self.options.screenshot.clone()
        } else {
            None
        };
        let drawn: Result<(), Box<dyn Error>> = match &mut screen.draw {
            #[cfg(all(
                unix,
                not(any(target_os = "macos", target_os = "ios", target_os = "android"))
            ))]
            Draw::Thread { gl, measurer } => {
                for (id, data) in updates {
                    gl.set_image(id, data);
                }
                let list = self.surface.frame(&frame, measurer.text());
                if let (Some(t), Some(from)) = (self.pending_transition.take(), &self.last_list) {
                    gl.begin_transition(&t, from);
                }
                screen.window.pre_present_notify();
                gl.draw(list, frame, shot.clone()).map(|list| {
                    self.last_list = Some(list);
                })
            }
            draw => {
                let renderer: &mut dyn Renderer = match draw {
                    Draw::Gl { renderer, .. } => &mut **renderer,
                    Draw::Cpu { renderer, .. } => &mut **renderer,
                    #[cfg(all(
                        unix,
                        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
                    ))]
                    Draw::Thread { .. } => unreachable!("handled above"),
                };
                for (id, data) in updates {
                    renderer.set_image(id, data.as_ref());
                    if starting_gl {
                        match data {
                            Some(d) => self.gl_images.insert(id, d),
                            None => self.gl_images.remove(&id),
                        };
                    }
                }
                let list = self.surface.frame(&frame, renderer.measurer());
                if let (Some(t), Some(from)) = (self.pending_transition.take(), &self.last_list) {
                    renderer.begin_transition(&t, from, &frame);
                }
                renderer.render(&list, &frame);
                self.last_list = Some(list);
                let saved = match &shot {
                    Some(path) => save_screenshot(draw, frame.width, frame.height, path),
                    None => Ok(()),
                };
                saved.and_then(|()| {
                    screen.window.pre_present_notify();
                    match draw {
                        Draw::Gl {
                            gl_surface,
                            context,
                            ..
                        } => gl_surface.swap_buffers(context).map_err(Into::into),
                        Draw::Cpu { renderer, present } => present.show(renderer.pixmap()),
                        #[cfg(all(
                            unix,
                            not(any(
                                target_os = "macos",
                                target_os = "ios",
                                target_os = "android"
                            ))
                        ))]
                        Draw::Thread { .. } => Ok(()),
                    }
                })
            }
        };
        if let Err(e) = drawn {
            self.error = Some(e);
            event_loop.exit();
            return;
        }
        self.frames += 1;
        if shot.is_some() {
            self.shot_pending = false;
            if self.options.exit_after_screenshot {
                event_loop.exit();
            }
        }
        if self.frames == 1 {
            self.open_devices();
            #[cfg(all(
                unix,
                not(any(target_os = "macos", target_os = "ios", target_os = "android"))
            ))]
            if let Some(screen) = &mut self.screen {
                if std::mem::take(&mut screen.gl_wanted) {
                    screen.gl_thread = Some(gl_thread::GlThread::spawn(screen.window.clone()));
                }
            }
        }
        let tree = self.surface.accessibility();
        self.update_text_field(tree.as_ref());
        let Some(screen) = &mut self.screen else {
            return;
        };
        if self.last_tree.as_ref() != Some(&tree) {
            let (title, scale) = (self.options.title.clone(), screen.window.scale_factor());
            screen
                .adapter
                .update_if_active(|| a11y::tree_update(tree.as_ref(), &title, scale));
            self.last_tree = Some(tree);
            self.update_cursor();
        }
        let Some(screen) = &mut self.screen else {
            return;
        };
        // One more frame right after the first, which carries the GPU
        // renderer's warm-up, so it is done before the first input.
        if self.surface.wants_frames() || screen.draw.transitioning() || self.frames == 1 {
            screen.window.request_redraw();
        } else {
            self.last_frame = None;
        }
    }
}

impl Screen {
    /// Whether OpenGL is still to start or starting, while the CPU renderer
    /// draws.
    fn starting_gl(&self) -> bool {
        #[cfg(all(
            unix,
            not(any(target_os = "macos", target_os = "ios", target_os = "android"))
        ))]
        if self.gl_wanted || self.gl_thread.is_some() {
            return true;
        }
        false
    }
}

/// Follows OpenGL starting, between frames: once it has started, has it
/// draw the frame on screen ahead, off screen and off this thread. Returns
/// whether a frame is needed: to switch, or for the screenshot when OpenGL
/// failed and the CPU renderer stays.
fn gl_step(
    screen: &mut Screen,
    images: &std::collections::HashMap<astryx_core::ImageId, astryx_core::ImageData>,
    on_screen: Option<&astryx_core::DisplayList>,
) -> bool {
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    ))]
    {
        let Some(gl) = &mut screen.gl_thread else {
            return false;
        };
        if gl.prepared() {
            // Switch in a frame once no transition runs.
            return !screen.draw.transitioning();
        }
        match gl.poll() {
            None => false,
            Some(Err(e)) => {
                eprintln!("astryx-desktop: no OpenGL ({e}); drawing with the CPU renderer");
                screen.gl_thread = None;
                true
            }
            Some(Ok(gl_thread::Started::Ready)) => {
                for (id, data) in images {
                    gl.set_image(*id, Some(data.clone()));
                }
                let size = screen.window.inner_size();
                let frame = FrameInfo {
                    width: size.width.max(1),
                    height: size.height.max(1),
                    scale: screen.window.scale_factor() as f32,
                    dt: 0.0,
                };
                gl.prepare(on_screen.cloned().unwrap_or_default(), frame);
                false
            }
            Some(Ok(gl_thread::Started::Prepared)) => {
                gl.again();
                !screen.draw.transitioning()
            }
        }
    }
    #[cfg(not(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    )))]
    {
        let _ = (screen, images, on_screen);
        false
    }
}

/// Switches to OpenGL on its thread once it has drawn its frame ahead and no
/// transition runs. This frame is then the GPU renderer's: the same picture
/// within antialiasing, so the switch does not show.
fn take_gl(
    screen: &mut Screen,
    images: &std::collections::HashMap<astryx_core::ImageId, astryx_core::ImageData>,
) {
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    ))]
    {
        let Some(gl) = &mut screen.gl_thread else {
            return;
        };
        if screen.draw.transitioning() || !gl.prepared() {
            return;
        }
        let mut gl = screen.gl_thread.take().expect("started");
        // Images given since the frame ahead; setting one twice is harmless.
        for (id, data) in images {
            gl.set_image(*id, Some(data.clone()));
        }
        let Draw::Cpu { renderer, .. } = &mut screen.draw else {
            unreachable!("the CPU renderer draws while OpenGL starts")
        };
        // An empty stand-in, gone with the CPU path below.
        let stand_in =
            SoftRenderer::with_fonts(astryx_render_soft::astryx_raster::text::FontSet::new());
        let mut renderer = std::mem::replace(renderer, Box::new(stand_in));
        // Kept for measuring text only: let its frame buffer go.
        let tiny = FrameInfo {
            width: 1,
            height: 1,
            scale: 1.0,
            dt: 0.0,
        };
        renderer.render_list(&astryx_core::DisplayList::default(), &tiny);
        screen.draw = Draw::Thread {
            gl,
            measurer: renderer,
        };
    }
    #[cfg(not(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    )))]
    {
        let _ = (screen, images);
    }
}

fn save_screenshot(
    draw: &mut Draw,
    width: u32,
    height: u32,
    path: &std::path::Path,
) -> Result<(), Box<dyn Error>> {
    match draw {
        Draw::Gl { renderer, .. } => save_gl_screenshot(renderer, width, height, path),
        Draw::Cpu { renderer, .. } => {
            renderer.pixmap().save_png(path)?;
            Ok(())
        }
        #[cfg(all(
            unix,
            not(any(target_os = "macos", target_os = "ios", target_os = "android"))
        ))]
        Draw::Thread { .. } => Err("the OpenGL thread writes its own screenshots".into()),
    }
}

/// Writes the GPU renderer's frame, read back from the framebuffer.
fn save_gl_screenshot(
    renderer: &mut GlRenderer,
    width: u32,
    height: u32,
    path: &std::path::Path,
) -> Result<(), Box<dyn Error>> {
    let mut rgba = renderer
        .read_pixels()
        .map_err(|e| format!("read pixels: {e:?}"))?;
    for px in rgba.as_chunks_mut::<4>().0 {
        let a = px[3] as u32;
        for c in &mut px[..3] {
            *c = ((*c as u32 * a + 127) / 255) as u8;
        }
    }
    let size = IntSize::from_wh(width, height).ok_or("empty frame")?;
    let pixmap = Pixmap::from_vec(rgba, size).ok_or("frame size mismatch")?;
    pixmap.save_png(path)?;
    Ok(())
}

impl<S: Surface> ApplicationHandler<accesskit_winit::Event> for Shell<S> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.screen.is_some() {
            return;
        }
        match self.create(event_loop) {
            Ok(screen) => self.screen = Some(screen),
            Err(e) => {
                self.error = Some(e);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if let Some(screen) = &mut self.screen {
            screen.adapter.process_event(&screen.window, &event);
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let (Some(screen), Some(w), Some(h)) = (
                    &self.screen,
                    NonZeroU32::new(size.width),
                    NonZeroU32::new(size.height),
                ) {
                    // The CPU path sizes its buffer to each frame.
                    if let Draw::Gl {
                        gl_surface,
                        context,
                        ..
                    } = &screen.draw
                    {
                        gl_surface.resize(context, w, h);
                    }
                    screen.window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(screen) = &self.screen {
                    screen.window.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(m) => self.modifiers = m.state(),
            WindowEvent::KeyboardInput { event, .. } => {
                if self.text_field.is_some() && input::is_text_key(&event.logical_key) {
                    if event.state == ElementState::Pressed {
                        if let Some(text) = input::typed_text(event.text.as_deref(), self.modifiers)
                        {
                            self.type_text(event_loop, &text);
                        }
                    }
                    return;
                }
                if let Some(input) =
                    input::key_input(&event.logical_key, self.modifiers.shift_key())
                {
                    let phase = match event.state {
                        ElementState::Pressed => InputPhase::Pressed,
                        ElementState::Released => InputPhase::Released,
                    };
                    self.send(event_loop, input, phase);
                }
            }
            // Composing text shows inline in the focused field; a commit
            // ends it and types the text (as the web shell's composition
            // events do).
            WindowEvent::Ime(ime) => {
                let steps = input::ime_steps(&ime);
                if !steps.is_empty() {
                    let command = input::apply_ime(&mut self.surface, steps);
                    self.after_input(event_loop, command);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self
                    .screen
                    .as_ref()
                    .map_or(1.0, |g| g.window.scale_factor());
                let logical = position.to_logical::<f32>(scale);
                self.pointer = Some(Point {
                    x: logical.x,
                    y: logical.y,
                });
                self.update_cursor();
                self.hover();
            }
            WindowEvent::CursorLeft { .. } => {
                self.pointer = None;
                self.update_cursor();
                self.hover();
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                let hit = self.pointer_hit().map(|n| n.id);
                match state {
                    ElementState::Pressed => {
                        self.pointer_target = hit;
                        if let Some(id) = hit {
                            let command = self.surface.a11y_action(id, A11yAction::PressStart);
                            self.after_input(event_loop, command);
                        }
                    }
                    ElementState::Released => {
                        if let Some(id) = self.pointer_target.take() {
                            // Lifting over the pressed node ends the press as a
                            // click; lifting elsewhere cancels it.
                            let action = if hit == Some(id) {
                                A11yAction::PressEnd
                            } else {
                                A11yAction::PressCancel
                            };
                            let command = self.surface.a11y_action(id, action);
                            self.after_input(event_loop, command);
                        }
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let (Some(at), Some(screen)) = (self.pointer, &self.screen) {
                    let delta = pointer::scroll_delta(delta, screen.window.scale_factor());
                    if self.surface.scroll(at, delta) {
                        screen.window.request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: accesskit_winit::Event) {
        match event.window_event {
            accesskit_winit::WindowEvent::InitialTreeRequested => {
                let tree = self.surface.accessibility();
                if let Some(screen) = &mut self.screen {
                    let (title, scale) = (self.options.title.clone(), screen.window.scale_factor());
                    screen
                        .adapter
                        .update_if_active(|| a11y::tree_update(tree.as_ref(), &title, scale));
                }
                self.last_tree = Some(tree);
            }
            accesskit_winit::WindowEvent::ActionRequested(request) => {
                if let Some((id, action)) = a11y::action(&request) {
                    let command = self.surface.a11y_action(id, action);
                    self.after_input(event_loop, command);
                }
            }
            accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Commands the app returned during frames, such as a hold completing.
        // This runs on every turn of the event loop, so it draws only for a
        // command that changes what is shown; otherwise the loop waits.
        let deferred = self.surface.take_command();
        self.handle(event_loop, deferred);
        // A surface that changes by itself (an update from another thread or
        // a stream) says so through wants_frames(); every wakeup asks again,
        // including the idle re-poll below.
        if self.surface.wants_frames() {
            self.request_redraw();
        }
        #[cfg(feature = "gamepad")]
        {
            let mut events = Vec::new();
            if let Some(gilrs) = &mut self.gamepads {
                while let Some(ev) = gilrs.next_event() {
                    let nintendo = gilrs.gamepad(ev.id).vendor_id() == Some(input::NINTENDO_VENDOR);
                    events.push((ev.event, nintendo));
                }
            }
            // The pad that sends input sets the face-button layout.
            if let Some(&(_, nintendo)) = events.last() {
                let layout = if nintendo {
                    astryx_core::FaceLayout::Nintendo
                } else {
                    astryx_core::FaceLayout::Xbox
                };
                if layout != self.preferences.face_layout {
                    self.preferences.face_layout = layout;
                    self.surface.set_preferences(&self.preferences);
                    if let Some(screen) = &self.screen {
                        screen.window.request_redraw();
                    }
                }
            }
            for (ev, nintendo) in events {
                let (button, phase) = match ev {
                    gilrs::EventType::ButtonPressed(b, _) => (b, InputPhase::Pressed),
                    gilrs::EventType::ButtonRepeated(b, _) => (b, InputPhase::Pressed),
                    gilrs::EventType::ButtonReleased(b, _) => (b, InputPhase::Released),
                    _ => continue,
                };
                if let Some(input) = input::gamepad_input(button, nintendo) {
                    self.send(event_loop, input, phase);
                }
            }
            // gilrs has no wakeup source, so poll while a gamepad is connected.
            let connected = self
                .gamepads
                .as_ref()
                .is_some_and(|g| g.gamepads().next().is_some());
            if connected {
                let next = Instant::now() + std::time::Duration::from_millis(8);
                event_loop.set_control_flow(ControlFlow::WaitUntil(next));
                return;
            }
        }
        // While OpenGL starts, look for its news often.
        if let Some(screen) = &mut self.screen {
            if screen.starting_gl() {
                if gl_step(screen, &self.gl_images, self.last_list.as_ref()) {
                    screen.window.request_redraw();
                }
                let next = Instant::now() + std::time::Duration::from_millis(8);
                event_loop.set_control_flow(ControlFlow::WaitUntil(next));
                return;
            }
        }
        // Idle: wake every IDLE_POLL to ask wants_frames() again, drawing
        // nothing unless it says so.
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + IDLE_POLL));
    }
}

/// Whether carrying out `command` changes what is shown, so a frame is
/// needed. Nothing, a press the app handled itself, and a focus request
/// (the runtime already wants a frame for the rebuild it waits on) need
/// none; a transition needs one to begin.
fn needs_frame(command: &Command) -> bool {
    match command {
        Command::None | Command::Handled | Command::Focus(_) => false,
        Command::Batch(commands) => commands.iter().any(needs_frame),
        Command::Quit | Command::OpenUrl(_) | Command::PlaySound(_) => false,
        Command::Transition(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::needs_frame;
    use astryx_core::{Command, Easing, SoundId, Transition, TransitionKind};

    #[test]
    fn only_commands_that_change_the_screen_ask_for_a_frame() {
        let fade = Command::Transition(Transition {
            kind: TransitionKind::Fade,
            duration_ms: 200,
            easing: Easing::Linear,
            sound: None,
        });
        // The idle path: the loop asks for the deferred command on every
        // turn, and an empty answer must not draw.
        assert!(!needs_frame(&Command::None));
        assert!(!needs_frame(&Command::Batch(vec![])));
        assert!(!needs_frame(&Command::Handled));
        assert!(!needs_frame(&Command::Focus("row".into())));
        assert!(!needs_frame(&Command::PlaySound(SoundId(1))));
        assert!(!needs_frame(&Command::OpenUrl(
            "https://example.com".into()
        )));
        assert!(needs_frame(&fade));
        assert!(needs_frame(&Command::Batch(vec![Command::None, fade])));
    }
}
