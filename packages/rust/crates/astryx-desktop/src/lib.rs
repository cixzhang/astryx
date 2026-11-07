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

pub mod a11y;
pub mod input;
pub mod platform;

use std::error::Error;
use std::ffi::CString;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::time::Instant;

use astryx_core::{A11yNode, Command, FrameInfo, Input, InputPhase, Surface};
use astryx_render_gl::GlRenderer;
use astryx_render_soft::tiny_skia::{IntSize, Pixmap};
use glutin::config::{Config, ConfigTemplateBuilder, GlConfig};
use glutin::context::{
    ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext, Version,
};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::{GlSurface, Surface as GlutinSurface, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::{ElementState, Ime, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::ModifiersState;
use winit::raw_window_handle::HasWindowHandle;
use winit::window::{Window, WindowAttributes, WindowId};

/// Window and run settings.
#[derive(Clone, Debug)]
pub struct Options {
    pub title: String,
    /// Initial inner size in logical points.
    pub size: (u32, u32),
    /// Writes the first frame, read back from the GL framebuffer, to this PNG file.
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
        gpu: None,
        error: None,
        modifiers: ModifiersState::empty(),
        text_field: None,
        last_frame: None,
        last_tree: None,
        frames: 0,
        #[cfg(feature = "gamepad")]
        gamepads: gilrs::Gilrs::new().ok(),
    };
    event_loop.run_app(&mut shell)?;
    match shell.error.take() {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

struct Gpu {
    // Field order is drop order: the renderer's GL objects go before the context.
    renderer: GlRenderer,
    gl_surface: GlutinSurface<WindowSurface>,
    context: PossiblyCurrentContext,
    adapter: accesskit_winit::Adapter,
    window: Window,
}

struct Shell<S: Surface> {
    surface: S,
    options: Options,
    proxy: winit::event_loop::EventLoopProxy<accesskit_winit::Event>,
    gpu: Option<Gpu>,
    error: Option<Box<dyn Error>>,
    modifiers: ModifiersState,
    /// Bounds of the focused text field, when one has focus.
    text_field: Option<astryx_core::Rect>,
    last_frame: Option<Instant>,
    last_tree: Option<Option<A11yNode>>,
    frames: u64,
    #[cfg(feature = "gamepad")]
    gamepads: Option<gilrs::Gilrs>,
}

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

impl<S: Surface> Shell<S> {
    fn create(&mut self, event_loop: &ActiveEventLoop) -> Result<Gpu, Box<dyn Error>> {
        let (w, h) = self.options.size;
        let attributes = WindowAttributes::default()
            .with_title(self.options.title.clone())
            .with_inner_size(LogicalSize::new(w, h))
            // AccessKit must attach before the window is first shown.
            .with_visible(false);
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
        let mut context = None;
        for api in [
            ContextApi::OpenGl(Some(Version::new(3, 3))),
            ContextApi::Gles(Some(Version::new(3, 0))),
            ContextApi::Gles(Some(Version::new(2, 0))),
        ] {
            let attrs = ContextAttributesBuilder::new()
                .with_context_api(api)
                .build(Some(handle));
            // SAFETY: the config belongs to this display and the handle to a live window.
            if let Ok(c) = unsafe { display.create_context(&config, &attrs) } {
                context = Some(c);
                break;
            }
        }
        let context = context.ok_or("no OpenGL or OpenGL ES context available")?;
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
        let adapter = accesskit_winit::Adapter::with_event_loop_proxy(
            event_loop,
            &window,
            self.proxy.clone(),
        );
        window.set_visible(true);
        window.request_redraw();
        Ok(Gpu {
            renderer,
            gl_surface,
            context,
            adapter,
            window,
        })
    }

    fn handle(&mut self, event_loop: &ActiveEventLoop, command: Command) {
        match command {
            Command::None => {}
            Command::Quit => event_loop.exit(),
            Command::OpenUrl(url) => {
                if let Err(e) = platform::open_url(&url) {
                    eprintln!("astryx-desktop: cannot open {url:?}: {e}");
                }
            }
        }
        if let Some(gpu) = &self.gpu {
            gpu.window.request_redraw();
        }
    }

    fn send(&mut self, event_loop: &ActiveEventLoop, input: Input, phase: InputPhase) {
        let command = self.surface.input(input, phase);
        self.handle(event_loop, command);
    }

    fn type_text(&mut self, event_loop: &ActiveEventLoop, text: &str) {
        let command = self.surface.text_input(text);
        self.handle(event_loop, command);
    }

    /// Turns the input method on while a text field has focus and keeps its
    /// candidate window next to that field.
    fn update_text_field(&mut self, tree: Option<&A11yNode>) {
        let field = tree.and_then(find_focused_text_field);
        if field == self.text_field {
            return;
        }
        if let Some(gpu) = &self.gpu {
            gpu.window.set_ime_allowed(field.is_some());
            if let Some(r) = field {
                gpu.window.set_ime_cursor_area(
                    LogicalPosition::new(r.x, r.y),
                    LogicalSize::new(r.width, r.height),
                );
            }
        }
        self.text_field = field;
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let Some(gpu) = &mut self.gpu else { return };
        let size = gpu.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        let now = Instant::now();
        let dt = self.last_frame.map_or(0.0, |t| (now - t).as_secs_f32());
        self.last_frame = Some(now);
        let frame = FrameInfo {
            width: size.width,
            height: size.height,
            scale: gpu.window.scale_factor() as f32,
            dt,
        };
        let list = self.surface.frame(&frame, gpu.renderer.text());
        gpu.renderer.render_list(&list, &frame);
        self.frames += 1;
        if self.frames == 1 {
            if let Some(path) = self.options.screenshot.clone() {
                if let Err(e) = save_screenshot(&mut gpu.renderer, frame.width, frame.height, &path)
                {
                    self.error = Some(e);
                    event_loop.exit();
                    return;
                }
                if self.options.exit_after_screenshot {
                    event_loop.exit();
                }
            }
        }
        gpu.window.pre_present_notify();
        if let Err(e) = gpu.gl_surface.swap_buffers(&gpu.context) {
            self.error = Some(e.into());
            event_loop.exit();
            return;
        }
        let tree = self.surface.accessibility();
        self.update_text_field(tree.as_ref());
        let Some(gpu) = &mut self.gpu else { return };
        if self.last_tree.as_ref() != Some(&tree) {
            let (title, scale) = (self.options.title.clone(), gpu.window.scale_factor());
            gpu.adapter
                .update_if_active(|| a11y::tree_update(tree.as_ref(), &title, scale));
            self.last_tree = Some(tree);
        }
        if self.surface.wants_frames() {
            gpu.window.request_redraw();
        } else {
            self.last_frame = None;
        }
    }
}

fn save_screenshot(
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
        if self.gpu.is_some() {
            return;
        }
        match self.create(event_loop) {
            Ok(gpu) => self.gpu = Some(gpu),
            Err(e) => {
                self.error = Some(e);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if let Some(gpu) = &mut self.gpu {
            gpu.adapter.process_event(&gpu.window, &event);
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let (Some(gpu), Some(w), Some(h)) = (
                    &self.gpu,
                    NonZeroU32::new(size.width),
                    NonZeroU32::new(size.height),
                ) {
                    gpu.gl_surface.resize(&gpu.context, w, h);
                    gpu.window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(gpu) = &self.gpu {
                    gpu.window.request_redraw();
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
            WindowEvent::Ime(Ime::Commit(text)) => {
                if !text.is_empty() {
                    self.type_text(event_loop, &text);
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
                if let Some(gpu) = &mut self.gpu {
                    let (title, scale) = (self.options.title.clone(), gpu.window.scale_factor());
                    gpu.adapter
                        .update_if_active(|| a11y::tree_update(tree.as_ref(), &title, scale));
                }
                self.last_tree = Some(tree);
            }
            accesskit_winit::WindowEvent::ActionRequested(request) => {
                if let Some((id, action)) = a11y::action(&request) {
                    let command = self.surface.a11y_action(id, action);
                    self.handle(event_loop, command);
                }
            }
            accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(feature = "gamepad")]
        {
            let mut events = Vec::new();
            if let Some(gilrs) = &mut self.gamepads {
                while let Some(ev) = gilrs.next_event() {
                    events.push(ev.event);
                }
            }
            for ev in events {
                let (button, phase) = match ev {
                    gilrs::EventType::ButtonPressed(b, _) => (b, InputPhase::Pressed),
                    gilrs::EventType::ButtonRepeated(b, _) => (b, InputPhase::Pressed),
                    gilrs::EventType::ButtonReleased(b, _) => (b, InputPhase::Released),
                    _ => continue,
                };
                if let Some(input) = input::gamepad_input(button) {
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
        event_loop.set_control_flow(ControlFlow::Wait);
    }
}

fn find_focused_text_field(node: &A11yNode) -> Option<astryx_core::Rect> {
    if node.focused && node.role == astryx_core::Role::TextInput {
        return Some(node.bounds);
    }
    node.children.iter().find_map(find_focused_text_field)
}
