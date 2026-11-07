//! An offscreen GL context for tests: EGL on a device display with no
//! window (Mesa's software rasterizer works), rendering into a framebuffer
//! object.

use std::ffi::CStr;

use astryx_core::{DisplayList, FrameInfo};
use astryx_render_gl::glow::{self, HasContext};
use astryx_render_gl::GlRenderer;
use glutin::api::egl::context::PossiblyCurrentContext;
use glutin::api::egl::device::Device;
use glutin::api::egl::display::Display;
use glutin::config::{ConfigSurfaceTypes, ConfigTemplateBuilder};
use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
use glutin::prelude::*;

pub struct OffscreenGl {
    pub renderer: GlRenderer,
    gl: glow::Context,
    fbo: glow::NativeFramebuffer,
    color: glow::NativeRenderbuffer,
    stencil: glow::NativeRenderbuffer,
    size: (u32, u32),
    pub description: String,
    _context: PossiblyCurrentContext,
}

impl OffscreenGl {
    /// Returns `None` with a reason when no EGL device can make a context.
    pub fn new() -> Result<Self, String> {
        let devices = Device::query_devices().map_err(|e| format!("no EGL devices: {e}"))?;
        let mut errors = Vec::new();
        for device in devices {
            match Self::on_device(&device) {
                Ok(gl) => return Ok(gl),
                Err(e) => errors.push(e),
            }
        }
        Err(format!("no usable EGL device: {errors:?}"))
    }

    fn on_device(device: &Device) -> Result<Self, String> {
        // SAFETY: no native display is passed; EGL owns the device display.
        let display = unsafe { Display::with_device(device, None) }.map_err(|e| e.to_string())?;
        let template = ConfigTemplateBuilder::new()
            .with_surface_type(ConfigSurfaceTypes::empty())
            .build();
        // SAFETY: the template is valid and the display is alive.
        let config = unsafe { display.find_configs(template) }
            .map_err(|e| e.to_string())?
            .next()
            .ok_or("no config")?;
        let mut last = String::new();
        for api in [
            ContextApi::Gles(Some(Version::new(3, 0))),
            ContextApi::Gles(Some(Version::new(2, 0))),
        ] {
            let attrs = ContextAttributesBuilder::new()
                .with_context_api(api)
                .build(None);
            // SAFETY: config belongs to display.
            let context = match unsafe { display.create_context(&config, &attrs) } {
                Ok(c) => c,
                Err(e) => {
                    last = e.to_string();
                    continue;
                }
            };
            let context = context
                .make_current_surfaceless()
                .map_err(|e| e.to_string())?;
            let load = |s: &CStr| display.get_proc_address(s);
            // SAFETY: the context is current on this thread.
            let gl = unsafe { glow::Context::from_loader_function_cstr(load) };
            let description = unsafe {
                format!(
                    "{} / {}",
                    gl.get_parameter_string(glow::RENDERER),
                    gl.get_parameter_string(glow::VERSION)
                )
            };
            // SAFETY: as above.
            let renderer =
                unsafe { GlRenderer::from_loader(load) }.map_err(|e| format!("{e:?}"))?;
            let (fbo, color, stencil) = unsafe {
                (
                    gl.create_framebuffer().map_err(|e| e.to_string())?,
                    gl.create_renderbuffer().map_err(|e| e.to_string())?,
                    gl.create_renderbuffer().map_err(|e| e.to_string())?,
                )
            };
            let mut out = Self {
                renderer,
                gl,
                fbo,
                color,
                stencil,
                size: (0, 0),
                description,
                _context: context,
            };
            out.renderer.set_framebuffer(Some(fbo));
            return Ok(out);
        }
        Err(last)
    }

    fn resize(&mut self, w: u32, h: u32) {
        if self.size == (w, h) {
            return;
        }
        self.size = (w, h);
        let gl = &self.gl;
        // SAFETY: the objects belong to the current context.
        unsafe {
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(self.color));
            gl.renderbuffer_storage(glow::RENDERBUFFER, glow::RGBA8, w as i32, h as i32);
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(self.stencil));
            gl.renderbuffer_storage(
                glow::RENDERBUFFER,
                glow::DEPTH24_STENCIL8,
                w as i32,
                h as i32,
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::RENDERBUFFER,
                Some(self.color),
            );
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::DEPTH_STENCIL_ATTACHMENT,
                glow::RENDERBUFFER,
                Some(self.stencil),
            );
            assert_eq!(
                gl.check_framebuffer_status(glow::FRAMEBUFFER),
                glow::FRAMEBUFFER_COMPLETE
            );
        }
    }

    /// Renders one frame and returns straight-alpha RGBA rows, top first.
    pub fn render(&mut self, list: &DisplayList, frame: &FrameInfo) -> Vec<u8> {
        self.resize(frame.width, frame.height);
        // SAFETY: the framebuffer belongs to the current context.
        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
            self.gl
                .viewport(0, 0, frame.width as i32, frame.height as i32);
        }
        self.renderer.render_list(list, frame);
        self.renderer.read_pixels().expect("read pixels")
    }

    /// Average milliseconds per frame over `n` frames, waiting for the GPU.
    pub fn time_frames(&mut self, list: &DisplayList, frame: &FrameInfo, n: u32) -> f64 {
        self.render(list, frame);
        let start = std::time::Instant::now();
        for _ in 0..n {
            self.renderer.render_list(list, frame);
            // SAFETY: the context is current.
            unsafe { self.gl.finish() };
        }
        start.elapsed().as_secs_f64() * 1000.0 / n as f64
    }
}
