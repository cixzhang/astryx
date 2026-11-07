//! Astryx GPU renderer: draws a [`DisplayList`] with one small shader into
//! the current OpenGL ES 2/3, WebGL2, or desktop OpenGL 3.3 context.
//!
//! Every draw command becomes quads in one vertex buffer, drawn in a few
//! batched calls:
//! - Rounded rectangles and borders are signed-distance shapes in the shader.
//! - Images are premultiplied RGBA textures, one per registered image,
//!   sampled inside their rounded box.
//! - Glyphs, shadows, and icon paths are 8-bit coverage masks from
//!   [`astryx_raster`], the same code the CPU renderer uses, packed into
//!   shared textures and cached across frames.
//! - Clips are a scissor rectangle plus one rounded shape in the shader.
//!   Overlapping rounded clips and group opacity render into small offscreen
//!   layers sized to their content.
//! - Commands outside the current clip are skipped.
//!
//! Layout is identical to the CPU renderer and pixels match it closely.

mod atlas;
mod shader;

use std::collections::HashMap;
use std::rc::Rc;

use astryx_core::{
    Color, DisplayList, DrawCmd, FrameInfo, ImageData, ImageFit, ImageId, Renderer, TextAlign,
    TextMeasure,
};
use astryx_raster::geom::{
    fit_radii, line_rect, path_segs, rect_px, snap_rect, width_px, Affine, PxRect, Seg,
};
use astryx_raster::light::{self, PlacedLight};
use astryx_raster::path::{self, PathKey, PathStyle};
use astryx_raster::shadow::{shadow_mask, ShadowKey};
use astryx_raster::text::{FontData, FontError, FontSet, GlyphDraw, TextEngine};
use astryx_raster::transition::Moment;
use glow::HasContext;

use atlas::{Atlas, MaskKey, Region};
use shader::{Dialect, Program, ATTRIBUTES};

pub use astryx_raster;
pub use glow;

/// The renderer could not start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlError(pub String);

impl std::fmt::Display for GlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GlError {}

/// How long the last frame took inside the renderer, for profiling shells.
/// Times are zero on targets without a clock (WebAssembly).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RenderStats {
    /// Turning draw commands into quads, rasterizing new masks, uploading them.
    pub encode: std::time::Duration,
    /// Submitting the frame to OpenGL.
    pub flush: std::time::Duration,
    pub commands: usize,
    /// Commands skipped because they lie outside their clip.
    pub culled: usize,
    pub quads: usize,
    pub draw_calls: usize,
    pub layers: usize,
    /// Glyphs rasterized during the frame (not prewarmed).
    pub glyphs: usize,
    /// Shadow masks computed during the frame.
    pub shadows: usize,
    /// Masks uploaded to textures during the frame.
    pub uploads: usize,
    /// Textures created during the frame.
    pub textures: usize,
    /// Time spent in texture uploads, part of `encode`.
    pub upload_time: std::time::Duration,
    /// Uploads by kind: glyphs, shadows, paths.
    pub upload_kinds: [usize; 3],
}

#[cfg(not(target_arch = "wasm32"))]
type Clock = std::time::Instant;
#[cfg(not(target_arch = "wasm32"))]
fn now() -> Option<Clock> {
    Some(std::time::Instant::now())
}
#[cfg(target_arch = "wasm32")]
type Clock = ();
#[cfg(target_arch = "wasm32")]
fn now() -> Option<Clock> {
    None
}
#[cfg(not(target_arch = "wasm32"))]
fn since(t: Option<Clock>) -> std::time::Duration {
    t.map(|t| t.elapsed()).unwrap_or_default()
}
#[cfg(target_arch = "wasm32")]
fn since(_: Option<Clock>) -> std::time::Duration {
    std::time::Duration::ZERO
}

/// The texture size a layer of `size` pixels gets: rounded up to 64, so
/// layers of nearby sizes share textures.
fn layer_size(size: (i32, i32)) -> (i32, i32) {
    let round = |v: i32| ((v.max(1) + 63) / 64) * 64;
    (round(size.0), round(size.1))
}

/// A transition in progress: the old frame and the time so far.
struct Running {
    transition: astryx_core::Transition,
    elapsed: f32,
    old: Layer,
    size: (i32, i32),
}

/// A lighting pass collecting its lights and emissive rects until it closes.
struct LightingPass {
    lighting: astryx_core::Lighting,
    lights: Vec<PlacedLight>,
    /// Emissive rects, already moved by the transform, and whether they snap.
    emissive: Vec<(astryx_core::Rect, Color, bool)>,
}

/// Where a shadow tile's mask sits relative to its box, and the center
/// column and row a stretched shadow repeats.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ShadowPlace {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    cx: i32,
    cy: i32,
}

#[cfg(not(target_arch = "wasm32"))]
type ShadowPrewarm =
    std::sync::mpsc::Receiver<Vec<(ShadowKey, Option<astryx_raster::shadow::ShadowMask>)>>;

/// A rectangle in whole pixels: left, top, right, bottom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct IRect {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

impl IRect {
    const EVERYTHING: IRect = IRect {
        x0: -1 << 20,
        y0: -1 << 20,
        x1: 1 << 20,
        y1: 1 << 20,
    };

    fn outer(r: &PxRect) -> IRect {
        IRect {
            x0: r.x0.floor() as i32,
            y0: r.y0.floor() as i32,
            x1: r.x1.ceil() as i32,
            y1: r.y1.ceil() as i32,
        }
    }

    fn intersect(self, o: IRect) -> IRect {
        IRect {
            x0: self.x0.max(o.x0),
            y0: self.y0.max(o.y0),
            x1: self.x1.min(o.x1),
            y1: self.y1.min(o.y1),
        }
    }

    fn union(self, o: IRect) -> IRect {
        IRect {
            x0: self.x0.min(o.x0),
            y0: self.y0.min(o.y0),
            x1: self.x1.max(o.x1),
            y1: self.y1.max(o.y1),
        }
    }

    fn is_empty(self) -> bool {
        self.x1 <= self.x0 || self.y1 <= self.y0
    }

    fn overlaps(self, o: IRect) -> bool {
        !self.intersect(o).is_empty()
    }
}

/// A rounded shape clip the shader applies, in frame pixels: center, half
/// size, corner radii.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ShapeClip {
    rect: PxRect,
    radii: [f32; 4],
}

// A clip shape so large it never cuts, inside the precision of mediump.
const NO_SHAPE: ([f32; 4], [f32; 4]) = ([8192.0, 8192.0, 8192.0, 8192.0], [0.0; 4]);

impl ShapeClip {
    fn encode(&self) -> ([f32; 4], [f32; 4]) {
        let r = &self.rect;
        (
            [
                (r.x0 + r.x1) / 2.0,
                (r.y0 + r.y1) / 2.0,
                r.width() / 2.0,
                r.height() / 2.0,
            ],
            self.radii,
        )
    }

    /// Signed distance to the shape: negative inside.
    fn distance(&self, x: f32, y: f32) -> f32 {
        let ([cx, cy, hx, hy], r) = self.encode();
        let (px, py) = (x - cx, y - cy);
        let rad = match (px > 0.0, py > 0.0) {
            (false, false) => r[0],
            (true, false) => r[1],
            (true, true) => r[2],
            (false, true) => r[3],
        };
        let (qx, qy) = (px.abs() - hx + rad, py.abs() - hy + rad);
        qx.max(qy).min(0.0) + (qx.max(0.0).hypot(qy.max(0.0))) - rad
    }

    /// True when `inner` lies inside this shape.
    fn contains(&self, inner: &ShapeClip) -> bool {
        let (o, i) = (&self.rect, &inner.rect);
        if i.x0 < o.x0 || i.y0 < o.y0 || i.x1 > o.x1 || i.y1 > o.y1 {
            return false;
        }
        let k = 1.0 - std::f32::consts::FRAC_1_SQRT_2;
        let [tl, tr, br, bl] = inner.radii;
        [
            (i.x0 + tl * k, i.y0 + tl * k),
            (i.x1 - tr * k, i.y0 + tr * k),
            (i.x1 - br * k, i.y1 - br * k),
            (i.x0 + bl * k, i.y1 - bl * k),
        ]
        .iter()
        .all(|&(x, y)| self.distance(x, y) <= 0.01)
    }
}

/// The clip in force: a whole-pixel scissor and at most one rounded shape.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ClipState {
    scissor: IRect,
    shape: Option<ShapeClip>,
}

impl ClipState {
    const NONE: ClipState = ClipState {
        scissor: IRect::EVERYTHING,
        shape: None,
    };
}

const MODE_FILL: f32 = 0.0;
const MODE_BORDER: f32 = 1.0;
const MODE_MASK: f32 = 2.0;
const MODE_LAYER: f32 = 3.0;
const MODE_IMAGE: f32 = 4.0;

/// One quad to draw.
#[derive(Clone, Copy, Debug)]
struct Quad {
    /// The area the quad covers, in frame pixels.
    area: PxRect,
    /// For shapes: the shape rectangle and radii.
    shape: PxRect,
    radii: [f32; 4],
    uv: [f32; 4],
    /// Premultiplied.
    color: [u8; 4],
    mode: f32,
    border: f32,
    texture: Option<glow::Texture>,
    /// Multiplies the target by the quad's color instead of blending over it.
    multiply: bool,
}

#[derive(Clone, Copy, Debug)]
struct Item {
    quad: Quad,
    clip: ClipState,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Vertex {
    pos_local: [f32; 4],
    half_uv: [f32; 4],
    radii: [f32; 4],
    color: [u8; 4],
    params: [f32; 4],
    clip: [f32; 4],
    clip_radii: [f32; 4],
}

const STRIDE: i32 = std::mem::size_of::<Vertex>() as i32;
/// Quads per draw call, bounded by 16-bit indices.
const BATCH_QUADS: usize = 16000;

enum Kind {
    /// The frame itself.
    Frame,
    /// Group opacity.
    Opacity(f32),
    /// A rounded clip that overlaps the rounded clip around it; composited
    /// through the outer one.
    Clip { outer: ClipState },
}

struct Target {
    kind: Kind,
    items: Vec<Item>,
    bounds: Option<IRect>,
    /// True once a layer was composited into this target.
    composited: bool,
}

impl Target {
    fn new(kind: Kind) -> Self {
        Target {
            kind,
            items: Vec::new(),
            bounds: None,
            composited: false,
        }
    }
}

struct Layer {
    texture: glow::Texture,
    framebuffer: glow::Framebuffer,
    size: (i32, i32),
    last_used: u64,
}

enum Group {
    /// A transform, holding the one around it.
    Transform(Affine),
    Clip {
        previous: ClipState,
        layer: bool,
    },
    Opacity {
        layer: bool,
    },
}

/// Frames an unused layer texture stays in the pool.
const LAYER_FRAMES: u64 = 120;
/// Opacity groups with at most this many non-overlapping quads skip the
/// layer and fade each quad instead, which looks the same.
const FADE_QUADS: usize = 16;

/// The GPU renderer. Draws into the GL context that is current when
/// [`Renderer::render`] runs, at the frame's pixel size.
pub struct GlRenderer {
    gl: Rc<glow::Context>,
    dialect: Dialect,
    program: Program,
    vao: Option<glow::VertexArray>,
    light_program: shader::LightProgram,
    light_vbo: glow::Buffer,
    /// The dapple noise lattice, repeating.
    lattice: glow::Texture,
    /// The open lighting pass, if any.
    lighting: Option<LightingPass>,
    transition_program: shader::TransitionProgram,
    transition: Option<Running>,
    /// Frame-sized textures kept for transitions, so starting one creates
    /// none.
    frame_layers: Vec<Layer>,
    vbo: glow::Buffer,
    ibo: glow::Buffer,
    white: glow::Texture,
    framebuffer: Option<glow::Framebuffer>,
    text: TextEngine,
    /// Where each cached shadow mask sits relative to its box, so a mask
    /// already in the atlas is never computed again.
    shadow_places: HashMap<ShadowKey, Option<ShadowPlace>>,
    /// Masks computed ahead of use on another thread, not yet uploaded.
    prewarmed_shadows: HashMap<ShadowKey, Option<astryx_raster::shadow::ShadowMask>>,
    #[cfg(not(target_arch = "wasm32"))]
    shadow_prewarm: Option<ShadowPrewarm>,
    paths: HashMap<PathKey, Option<(i32, i32, u32, u32)>>,
    /// Paths drawn once, by the frame they were last drawn in. A path seen
    /// again in a later frame earns a place in the atlas; animated paths
    /// that never repeat use the per-frame page instead.
    paths_seen: HashMap<PathKey, u64>,
    atlas: Atlas,
    layers: Vec<Layer>,
    layers_in_use: Vec<Layer>,
    /// Layers drawn last frame, by a hash of their content. A layer whose
    /// content did not change is composited again without redrawing it, so
    /// steady frames switch framebuffers no more than needed; on tiled GPUs
    /// each switch costs a full tile flush.
    layer_cache: HashMap<u64, Layer>,
    layer_cache_next: HashMap<u64, Layer>,
    frame: u64,
    frame_size: (i32, i32),
    targets: Vec<Target>,
    groups: Vec<Group>,
    clip: ClipState,
    transform: Affine,
    segs: Vec<Seg>,
    placed: Vec<GlyphDraw>,
    vertices: Vec<Vertex>,
    stats: RenderStats,
    /// Open pixel grids, innermost last, in points.
    grids: Vec<f32>,
    /// Images whose textures sample nearest, last drawn on a pixel grid.
    nearest: std::collections::HashSet<ImageId>,
    /// Registered images: the texture and its size in pixels.
    images: HashMap<ImageId, (glow::Texture, u32, u32)>,
}

impl GlRenderer {
    /// Takes over a GL context. Uses the embedded Roboto faces and, on demand,
    /// the embedded Arabic and Hebrew faces; no faces when the
    /// `embedded-fonts` feature is off, so add them with
    /// [`Self::add_fallback_font`].
    pub fn new(gl: glow::Context) -> Result<Self, GlError> {
        #[cfg(feature = "embedded-fonts")]
        let fonts = astryx_fonts::roboto();
        #[cfg(not(feature = "embedded-fonts"))]
        let fonts = FontSet::new();
        let renderer = Self::with_fonts(gl, fonts);
        #[cfg(feature = "embedded-fonts")]
        let renderer = renderer.inspect(|r| {
            r.text()
                .set_fallback_provider(Box::new(astryx_fonts::fallback))
        });
        renderer
    }

    /// Takes over a GL context and draws text with `fonts`, in their
    /// fallback order.
    pub fn with_fonts(gl: glow::Context, fonts: FontSet) -> Result<Self, GlError> {
        let dialect = Dialect::detect(&gl);
        // SAFETY: the caller made the context current on this thread.
        unsafe {
            let program = shader::compile(&gl, dialect).map_err(GlError)?;
            let vao = match dialect {
                Dialect::Es2 => None,
                _ => Some(gl.create_vertex_array().map_err(GlError)?),
            };
            let vbo = gl.create_buffer().map_err(GlError)?;
            let ibo = gl.create_buffer().map_err(GlError)?;
            let mut indices: Vec<u16> = Vec::with_capacity(BATCH_QUADS * 6);
            for q in 0..BATCH_QUADS as u16 {
                let v = q * 4;
                indices.extend_from_slice(&[v, v + 1, v + 2, v, v + 2, v + 3]);
            }
            gl.bind_vertex_array(vao);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ibo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                as_bytes(&indices),
                glow::STATIC_DRAW,
            );
            let white = atlas::mask_texture(&gl, dialect, 1, 1, Some(&[255]));
            let light_program = shader::compile_light(&gl, dialect).map_err(GlError)?;
            let transition_program = shader::compile_transition(&gl, dialect).map_err(GlError)?;
            let light_vbo = gl.create_buffer().map_err(GlError)?;
            let lattice = atlas::mask_texture(
                &gl,
                dialect,
                light::LATTICE as i32,
                light::LATTICE as i32,
                Some(&light::lattice()),
            );
            for wrap in [glow::TEXTURE_WRAP_S, glow::TEXTURE_WRAP_T] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, wrap, glow::REPEAT as i32);
            }
            let mut renderer = GlRenderer {
                gl: Rc::new(gl),
                dialect,
                program,
                vao,
                light_program,
                light_vbo,
                lattice,
                lighting: None,
                transition_program,
                transition: None,
                frame_layers: Vec::new(),
                vbo,
                ibo,
                white,
                framebuffer: None,
                text: TextEngine::new(fonts),
                shadow_places: HashMap::new(),
                prewarmed_shadows: HashMap::new(),
                #[cfg(not(target_arch = "wasm32"))]
                shadow_prewarm: None,
                paths: HashMap::new(),
                paths_seen: HashMap::new(),
                atlas: Atlas::default(),
                layers: Vec::new(),
                layers_in_use: Vec::new(),
                layer_cache: HashMap::new(),
                layer_cache_next: HashMap::new(),
                frame: 0,
                frame_size: (1, 1),
                targets: Vec::new(),
                groups: Vec::new(),
                clip: ClipState::NONE,
                transform: Affine::IDENTITY,
                segs: Vec::new(),
                placed: Vec::new(),
                vertices: Vec::new(),
                stats: RenderStats::default(),
                grids: Vec::new(),
                nearest: std::collections::HashSet::new(),
                images: HashMap::new(),
            };
            renderer.warm_up();
            Ok(renderer)
        }
    }

    /// Creates the renderer from a GL function loader for the current context.
    ///
    /// # Safety
    /// A GL context must be current on this thread and `load` must return
    /// that context's function pointers.
    #[cfg(not(target_arch = "wasm32"))]
    pub unsafe fn from_loader(
        load: impl FnMut(&std::ffi::CStr) -> *const std::ffi::c_void,
    ) -> Result<Self, GlError> {
        Self::new(glow::Context::from_loader_function_cstr(load))
    }

    /// Creates the renderer on a canvas's WebGL2 context, drawing text with
    /// `fonts`.
    #[cfg(target_arch = "wasm32")]
    pub fn from_html_canvas(
        canvas: &web_sys::HtmlCanvasElement,
        fonts: FontSet,
    ) -> Result<Self, GlError> {
        use wasm_bindgen::JsCast;
        let attrs = web_sys::WebGlContextAttributes::new();
        attrs.set_antialias(false);
        attrs.set_stencil(false);
        attrs.set_depth(false);
        let context = canvas
            .get_context_with_context_options("webgl2", &attrs)
            .ok()
            .flatten()
            .and_then(|c| c.dyn_into::<web_sys::WebGl2RenderingContext>().ok())
            .ok_or_else(|| GlError("no WebGL2 context".into()))?;
        Self::with_fonts(glow::Context::from_webgl2_context(context), fonts)
    }

    /// Draws into this framebuffer instead of the default one, for offscreen
    /// rendering. `None` restores the default framebuffer.
    pub fn set_framebuffer(&mut self, framebuffer: Option<glow::Framebuffer>) {
        self.framebuffer = framebuffer;
    }

    /// Adds a face after creation, for example one loaded over the network.
    /// Layout picks it up on the next frame; request one.
    pub fn add_fallback_font(&mut self, data: impl Into<FontData>) -> Result<(), FontError> {
        self.text.add_fallback_font(data)
    }

    /// Adds or replaces an image; `None` frees it. Pixels that do not match
    /// the size are ignored.
    pub fn set_image(&mut self, id: ImageId, data: Option<&ImageData>) {
        // A new texture starts with linear filtering.
        self.nearest.remove(&id);
        let gl = self.gl.clone();
        if let Some((old, _, _)) = self.images.remove(&id) {
            // SAFETY: the texture belongs to this context.
            unsafe { gl.delete_texture(old) };
        }
        let Some(d) = data else { return };
        let (w, h) = (d.width as usize, d.height as usize);
        if w == 0 || h == 0 || d.rgba.len() != w * h * 4 {
            return;
        }
        // Premultiplied, so filtering does not bleed the color of transparent
        // pixels into their neighbours.
        let mut px = d.rgba.to_vec();
        for p in px.as_chunks_mut::<4>().0 {
            let a = p[3] as u32;
            for c in &mut p[..3] {
                *c = ((*c as u32 * a + 127) / 255) as u8;
            }
        }
        // SAFETY: the context is current while the shell drives the
        // renderer; the pixel buffer matches the size and format.
        unsafe {
            let Ok(texture) = gl.create_texture() else {
                return;
            };
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            // Clamped, linear, no mipmaps: what every GL ES 2 device allows
            // for any size.
            for (k, v) in [
                (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
                (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
                (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
            ] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, k, v as i32);
            }
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                d.width as i32,
                d.height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&px)),
            );
            self.images.insert(id, (texture, d.width, d.height));
        }
    }

    /// Uploads every glyph the text engine holds that is not on the GPU yet,
    /// for example after a prewarm, so text first shown later costs no
    /// upload. Returns the number of glyph pages in use.
    /// Only glyphs of these text styles (each with its pixel grid, if any)
    /// are uploaded.
    pub fn preload_glyphs(
        &mut self,
        styles: &[(astryx_core::TextStyle, Option<f32>)],
        scale: f32,
    ) -> usize {
        let sizes: Vec<f32> = styles
            .iter()
            .map(|(s, grid)| astryx_raster::text::glyph_size(s.size, scale, *grid))
            .collect();
        let mut glyphs = self.text.cached_glyphs();
        glyphs.retain(|(k, _)| sizes.contains(&f32::from_bits(k.size)));
        // By height, so shelves pack tightly; a run of one size stays close.
        glyphs.sort_by_key(|(k, b)| (b.height, k.size, k.face, k.id, k.subpixel));
        for (key, b) in glyphs {
            let mk = MaskKey::Glyph(key);
            if b.width == 0 || b.height == 0 || self.atlas.get(&mk).is_some() {
                continue;
            }
            self.atlas
                .preload(&self.gl, self.dialect, mk, b.width, b.height, &b.data);
        }
        self.atlas.flush(&self.gl, self.dialect);
        self.atlas.pages()
    }

    /// Timings and counts of the last frame.
    pub fn last_stats(&self) -> RenderStats {
        self.stats
    }

    pub fn text(&self) -> &TextEngine {
        &self.text
    }

    /// Adds a quad to the current target unless the clip hides it.
    fn push(&mut self, quad: Quad) -> bool {
        let area = IRect::outer(&quad.area).intersect(self.clip.scissor);
        if area.is_empty() || quad.color[3] == 0 {
            return false;
        }
        let target = self.targets.last_mut().expect("frame target");
        target.bounds = Some(target.bounds.map_or(area, |b| b.union(area)));
        target.items.push(Item {
            quad,
            clip: self.clip,
        });
        true
    }

    fn shape(&mut self, rect: &PxRect, radii: [f32; 4], color: Color, border: Option<f32>) {
        let (mode, border) = match border {
            Some(w) => (MODE_BORDER, w),
            None => (MODE_FILL, 0.0),
        };
        self.push(Quad {
            area: rect.outset(1.0),
            shape: *rect,
            radii,
            uv: [0.0; 4],
            color: premultiplied(color),
            mode,
            border,
            texture: None,
            multiply: false,
        });
    }

    fn mask(&mut self, x: i32, y: i32, w: u32, h: u32, region: Region, color: Color) {
        let area = PxRect {
            x0: x as f32,
            y0: y as f32,
            x1: (x + w as i32) as f32,
            y1: (y + h as i32) as f32,
        };
        self.mask_area(area, region, color);
    }

    /// The atlas region and placement of a shadow mask, computing and
    /// uploading it on first use.
    fn shadow(&mut self, key: ShadowKey) -> Option<(Region, ShadowPlace)> {
        let mk = MaskKey::Shadow(key);
        if let (Some(place), Some(region)) = (self.shadow_places.get(&key), self.atlas.get(&mk)) {
            return place.map(|p| (region, p));
        }
        let mask = match self.prewarmed_shadows.remove(&key) {
            Some(mask) => mask,
            None => {
                self.stats.shadows += 1;
                shadow_mask(&key)
            }
        };
        let Some(mask) = mask else {
            self.shadow_places.insert(key, None);
            return None;
        };
        let (cx, cy) = key.center(&mask);
        let place = ShadowPlace {
            x: mask.x,
            y: mask.y,
            w: mask.width,
            h: mask.height,
            cx,
            cy,
        };
        let region = self.atlas.insert(
            &self.gl,
            self.dialect,
            mk,
            mask.width,
            mask.height,
            &mask.data,
        );
        self.shadow_places.insert(key, Some(place));
        Some((region, place))
    }

    /// Draws one shadow layer of the box `r` (frame pixels before the
    /// transform `k`, `off`) from its shared tile: the tile's corners as
    /// they are and its center line stretched along the box.
    #[allow(clippy::too_many_arguments)]
    fn draw_shadow(
        &mut self,
        key: ShadowKey,
        r: &PxRect,
        k: f32,
        off: (f32, f32),
        inset: bool,
        radii: [f32; 4],
        color: Color,
    ) {
        let tiling = key.tiling();
        let Some((region, p)) = self.shadow(tiling.tile) else {
            return;
        };
        let (ew, eh) = (tiling.extra.0 as i32, tiling.extra.1 as i32);
        let map = |x: f32, y: f32| (x * k + off.0, y * k + off.1);
        let (o0, o1) = map(r.x0 + p.x as f32, r.y0 + p.y as f32);
        let reach = PxRect {
            x0: o0,
            y0: o1,
            x1: o0 + (p.w as i32 + ew) as f32 * k,
            y1: o1 + (p.h as i32 + eh) as f32 * k,
        };
        if !self.visible(IRect::outer(&reach)) {
            return;
        }
        let hole = (!inset).then(|| {
            let h = r.outset(-(rmax(radii) + 1.0));
            let (x0, y0) = map(h.x0, h.y0);
            let (x1, y1) = map(h.x1, h.y1);
            PxRect { x0, y0, x1, y1 }
        });
        let [u0, v0, u1, v1] = region.uv;
        let (du, dv) = ((u1 - u0) / p.w as f32, (v1 - v0) / p.h as f32);
        // Spans along one axis: start and end in pixels of the full shadow,
        // and the texels they show; the stretched span shows one texel.
        let spans = |center: i32, extra: i32, size: i32| {
            [
                (0, center, 0.0, center as f32),
                (
                    center,
                    center + extra + 1,
                    center as f32 + 0.5,
                    center as f32 + 0.5,
                ),
                (
                    center + extra + 1,
                    size + extra,
                    center as f32 + 1.0,
                    size as f32,
                ),
            ]
        };
        for (y0, y1, ty0, ty1) in spans(p.cy, eh, p.h as i32) {
            if y1 <= y0 {
                continue;
            }
            for (x0, x1, tx0, tx1) in spans(p.cx, ew, p.w as i32) {
                if x1 <= x0 {
                    continue;
                }
                let area = PxRect {
                    x0: o0 + x0 as f32 * k,
                    y0: o1 + y0 as f32 * k,
                    x1: o0 + x1 as f32 * k,
                    y1: o1 + y1 as f32 * k,
                };
                let uv = [u0 + tx0 * du, v0 + ty0 * dv, u0 + tx1 * du, v0 + ty1 * dv];
                self.mask_hollow(area, Region { uv, ..region }, color, hole);
            }
        }
    }

    /// Uploads the shadow tiles of these box styles that the prewarm has
    /// computed so far, so boxes first shown later cost no upload. Returns
    /// how many tiles it uploaded.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn preload_shadows(
        &mut self,
        styles: &[([f32; 4], Vec<astryx_core::ShadowLayer>)],
        scale: f32,
    ) -> usize {
        if let Some(rx) = &self.shadow_prewarm {
            while let Ok(batch) = rx.try_recv() {
                self.prewarmed_shadows.extend(batch);
            }
        }
        let big = PxRect {
            x0: 0.0,
            y0: 0.0,
            x1: 4096.0,
            y1: 4096.0,
        };
        let mut done = 0;
        for (radii, layers) in styles {
            let radii = radii.map(|r| r * scale);
            for layer in layers {
                let tile = ShadowKey::new(&big, radii, layer, scale).tiling().tile;
                let mk = MaskKey::Shadow(tile);
                if self.atlas.get(&mk).is_some() {
                    continue;
                }
                let Some(Some(mask)) = self.prewarmed_shadows.remove(&tile) else {
                    continue;
                };
                let (cx, cy) = tile.center(&mask);
                let place = ShadowPlace {
                    x: mask.x,
                    y: mask.y,
                    w: mask.width,
                    h: mask.height,
                    cx,
                    cy,
                };
                self.atlas.preload(
                    &self.gl,
                    self.dialect,
                    mk,
                    mask.width,
                    mask.height,
                    &mask.data,
                );
                self.shadow_places.insert(tile, Some(place));
                done += 1;
            }
        }
        self.atlas.flush(&self.gl, self.dialect);
        done
    }

    /// Ends preloading: the preload pages free their CPU copies and take
    /// no more masks.
    pub fn finish_preload(&mut self) {
        self.atlas.seal_preloaded(&self.gl, self.dialect);
    }

    /// Starts computing the shadow tiles of these box styles on another
    /// thread: corner radii in points and the layers drawn under them.
    /// Tiles do not depend on box size, so any large box with these styles
    /// later draws without computing a mask.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn prewarm_shadows(
        &mut self,
        styles: &[([f32; 4], Vec<astryx_core::ShadowLayer>)],
        scale: f32,
    ) {
        let big = PxRect {
            x0: 0.0,
            y0: 0.0,
            x1: 4096.0,
            y1: 4096.0,
        };
        let mut keys: Vec<ShadowKey> = Vec::new();
        for (radii, layers) in styles {
            let radii = radii.map(|r| r * scale);
            for layer in layers {
                let tile = ShadowKey::new(&big, radii, layer, scale).tiling().tile;
                if !keys.contains(&tile) {
                    keys.push(tile);
                }
            }
        }
        self.shadow_prewarm = Some(astryx_raster::shadow::prewarm(keys));
    }

    /// A mask quad with a hole where the mask is known to be empty, so the
    /// GPU shades only the ring around it. Outer shadows are empty under
    /// their box, which is most of their area.
    fn mask_hollow(&mut self, area: PxRect, region: Region, color: Color, hole: Option<PxRect>) {
        let hole = hole.map(|h| PxRect {
            x0: h.x0.ceil(),
            y0: h.y0.ceil(),
            x1: h.x1.floor(),
            y1: h.y1.floor(),
        });
        let Some(h) = hole.filter(|h| {
            !h.is_empty()
                && h.x0 >= area.x0
                && h.y0 >= area.y0
                && h.x1 <= area.x1
                && h.y1 <= area.y1
        }) else {
            return self.mask_area(area, region, color);
        };
        let [u0, v0, u1, v1] = region.uv;
        let u = |x: f32| u0 + (x - area.x0) / area.width() * (u1 - u0);
        let v = |y: f32| v0 + (y - area.y0) / area.height() * (v1 - v0);
        for strip in [
            PxRect {
                x0: area.x0,
                y0: area.y0,
                x1: area.x1,
                y1: h.y0,
            },
            PxRect {
                x0: area.x0,
                y0: h.y1,
                x1: area.x1,
                y1: area.y1,
            },
            PxRect {
                x0: area.x0,
                y0: h.y0,
                x1: h.x0,
                y1: h.y1,
            },
            PxRect {
                x0: h.x1,
                y0: h.y0,
                x1: area.x1,
                y1: h.y1,
            },
        ] {
            if strip.is_empty() {
                continue;
            }
            let uv = [u(strip.x0), v(strip.y0), u(strip.x1), v(strip.y1)];
            self.mask_area(strip, Region { uv, ..region }, color);
        }
    }

    fn mask_area(&mut self, area: PxRect, region: Region, color: Color) {
        self.push(Quad {
            area,
            shape: area,
            radii: [0.0; 4],
            uv: region.uv,
            color: premultiplied(color),
            mode: MODE_MASK,
            border: 0.0,
            texture: Some(region.texture),
            multiply: false,
        });
    }

    fn visible(&self, r: IRect) -> bool {
        r.overlaps(self.clip.scissor)
    }

    fn draw_path(&mut self, style: PathStyle, color: Color) {
        let Some(prepared) = path::prepare(&self.segs, style) else {
            return;
        };
        if !self.grids.is_empty() {
            // On a pixel grid, coverage is all or nothing. Such masks are
            // few, so they skip the path cache.
            let Some(mut m) = prepared.rasterize() else {
                return;
            };
            for v in &mut m.data {
                *v = if *v >= 128 { 255 } else { 0 };
            }
            if let Some(region) =
                self.atlas
                    .insert_transient(&self.gl, self.dialect, m.width, m.height, &m.data)
            {
                self.mask(m.x, m.y, m.width, m.height, region, color);
            }
            return;
        }
        let key = MaskKey::Path(prepared.key.clone());
        if let Some(placed) = self.paths.get(&prepared.key) {
            // The atlas and this cache are cleared together, so a cached mask is in the atlas.
            if let (Some((dx, dy, w, h)), Some(region)) = (*placed, self.atlas.get(&key)) {
                self.mask(
                    prepared.origin.0 + dx,
                    prepared.origin.1 + dy,
                    w,
                    h,
                    region,
                    color,
                );
            }
            return;
        }
        let repeated = self
            .paths_seen
            .get(&prepared.key)
            .is_some_and(|&f| f < self.frame);
        let mask = prepared.rasterize();
        let Some(m) = mask else {
            self.paths.insert(prepared.key, None);
            return;
        };
        let transient = if repeated {
            None
        } else {
            if self.paths_seen.len() > 4096 {
                self.paths_seen.clear();
            }
            self.paths_seen.insert(prepared.key.clone(), self.frame);
            self.atlas
                .insert_transient(&self.gl, self.dialect, m.width, m.height, &m.data)
        };
        let region = match transient {
            Some(region) => region,
            None => {
                self.paths_seen.remove(&prepared.key);
                let offset = (
                    m.x - prepared.origin.0,
                    m.y - prepared.origin.1,
                    m.width,
                    m.height,
                );
                self.paths.insert(prepared.key, Some(offset));
                self.atlas
                    .insert(&self.gl, self.dialect, key, m.width, m.height, &m.data)
            }
        };
        self.mask(m.x, m.y, m.width, m.height, region, color);
    }

    /// Closes groups until one that `done` accepts is closed. Groups opened
    /// inside it and never closed close with it.
    fn pop_until(&mut self, done: impl Fn(&Group) -> bool) {
        while let Some(group) = self.pop_group() {
            if done(&group) {
                break;
            }
        }
    }

    fn draw(&mut self, cmd: &DrawCmd, s: f32) {
        match cmd {
            DrawCmd::PushTransform {
                origin,
                scale,
                translate,
            } => {
                let previous = self.transform;
                self.transform = previous.then(*origin, *scale, *translate);
                self.groups.push(Group::Transform(previous));
                return;
            }
            DrawCmd::PopTransform => return self.pop_until(|g| matches!(g, Group::Transform(_))),
            DrawCmd::PushPixelGrid { cell } => {
                self.grids.push(*cell);
                return;
            }
            DrawCmd::PopPixelGrid => {
                self.grids.pop();
                return;
            }
            DrawCmd::PushLighting(lighting) => {
                if self.lighting.is_none() {
                    self.lighting = Some(LightingPass {
                        lighting: *lighting,
                        lights: Vec::new(),
                        emissive: Vec::new(),
                    });
                }
                return;
            }
            DrawCmd::PopLighting => return self.finish_lighting(s),
            DrawCmd::PopClip => return self.pop_until(|g| matches!(g, Group::Clip { .. })),
            DrawCmd::PopOpacity => return self.pop_until(|g| matches!(g, Group::Opacity { .. })),
            _ => {}
        }
        let t = self.transform;
        if t.is_identity() {
            self.draw_moved(cmd, s, true);
        } else if let (
            DrawCmd::Shadow {
                rect,
                radii,
                layers,
            },
            false,
        ) = (cmd, t.snaps())
        {
            self.draw_scaled_shadow(rect, radii, layers, s);
        } else {
            self.draw_moved(&t.apply(cmd), s, t.snaps());
        }
    }

    /// A shadow under a scaling transform: the untransformed mask on a
    /// scaled quad. Blurred edges scale without visible loss, and the mask
    /// stays cached while the scale animates.
    fn draw_scaled_shadow(
        &mut self,
        rect: &astryx_core::Rect,
        radii: &astryx_core::Radii,
        layers: &[astryx_core::ShadowLayer],
        s: f32,
    ) {
        let r = snap_rect(rect, s);
        if r.is_empty() {
            return;
        }
        let radii = fit_radii(radii, s, &r);
        let t = self.transform;
        for layer in layers.iter().rev() {
            let key = ShadowKey::new(&r, radii, layer, s);
            let off = (t.offset.x * s, t.offset.y * s);
            self.draw_shadow(key, &r, t.scale, off, layer.inset, radii, layer.color);
        }
    }

    /// Draws a command already moved by the transform. Edges snap to whole
    /// pixels only when `snap` is set.
    fn draw_moved(&mut self, cmd: &DrawCmd, s: f32, snap: bool) {
        match cmd {
            DrawCmd::Rect {
                rect,
                radii,
                fill,
                stroke,
            } => {
                let r = rect_px(rect, s, snap);
                if r.is_empty() || !self.visible(IRect::outer(&r)) {
                    self.stats.culled += 1;
                    return;
                }
                let radii = fit_radii(radii, s, &r);
                if fill.a > 0.0 {
                    self.shape(&r, radii, *fill, None);
                }
                if let Some((color, width)) = stroke {
                    let w = width_px(*width, s, snap);
                    if w > 0.0 && color.a > 0.0 {
                        let inner_empty = r.width() <= 2.0 * w || r.height() <= 2.0 * w;
                        self.shape(&r, radii, *color, if inner_empty { None } else { Some(w) });
                    }
                }
            }
            DrawCmd::Shadow {
                rect,
                radii,
                layers,
            } => {
                let r = rect_px(rect, s, snap);
                let reach = layers
                    .iter()
                    .map(|l| {
                        (l.x.abs().max(l.y.abs()) + l.spread.max(0.0) + l.blur * 1.5) * s + 2.0
                    })
                    .fold(0.0, f32::max);
                if r.is_empty() || !self.visible(IRect::outer(&r.outset(reach))) {
                    self.stats.culled += 1;
                    return;
                }
                let radii = fit_radii(radii, s, &r);
                // CSS paints the first layer on top.
                for layer in layers.iter().rev() {
                    let key = ShadowKey::new(&r, radii, layer, s);
                    self.draw_shadow(key, &r, 1.0, (0.0, 0.0), layer.inset, radii, layer.color);
                }
            }
            DrawCmd::Text {
                rect,
                text,
                style,
                color,
                align,
            } => {
                // Text starts at its box and flows right and down; centered or
                // end-aligned overflow can also reach left.
                let lh = style.size * style.leading;
                let left = if *align == TextAlign::Start {
                    rect.x
                } else {
                    f32::NEG_INFINITY
                };
                let reach = PxRect {
                    x0: (left * s).max(-1e6),
                    y0: rect.y * s,
                    x1: 1e6,
                    y1: (rect.y + rect.height + lh) * s,
                };
                if !self.visible(IRect::outer(&reach)) || color.a <= 0.0 {
                    self.stats.culled += 1;
                    return;
                }
                let layout = self.text.layout(text, style, Some(rect.width), *align);
                let mut placed = std::mem::take(&mut self.placed);
                placed.clear();
                match self.grids.last() {
                    Some(&cell) if snap => self.text.place_on_grid(
                        &layout,
                        (rect.x, rect.y),
                        s,
                        cell,
                        own_family(style),
                        &mut placed,
                    ),
                    _ => self
                        .text
                        .place(&layout, (rect.x, rect.y), s, snap, &mut placed),
                }
                for g in &placed {
                    let b = &g.bitmap;
                    let (x, y) = (g.pen_x + b.left, g.pen_y - b.top);
                    let area = IRect {
                        x0: x,
                        y0: y,
                        x1: x + b.width as i32,
                        y1: y + b.height as i32,
                    };
                    if !self.visible(area) {
                        continue;
                    }
                    let key = MaskKey::Glyph(g.key);
                    let region = match self.atlas.get(&key) {
                        Some(region) => region,
                        // Sizes under a scaling transform change every frame of
                        // an animation, so they stay out of the lasting atlas.
                        None if !snap => match self.atlas.insert_transient(
                            &self.gl,
                            self.dialect,
                            b.width,
                            b.height,
                            &b.data,
                        ) {
                            Some(region) => region,
                            None => self.atlas.insert(
                                &self.gl,
                                self.dialect,
                                key,
                                b.width,
                                b.height,
                                &b.data,
                            ),
                        },
                        None => self.atlas.insert(
                            &self.gl,
                            self.dialect,
                            key,
                            b.width,
                            b.height,
                            &b.data,
                        ),
                    };
                    self.mask(x, y, b.width, b.height, region, *color);
                }
                self.placed = placed;
            }
            DrawCmd::Line {
                from,
                to,
                color,
                width,
            } => {
                if let Some(r) = line_rect(*from, *to, *width, s, snap) {
                    if self.visible(IRect::outer(&r)) {
                        self.shape(&r, [0.0; 4], *color, None);
                    } else {
                        self.stats.culled += 1;
                    }
                } else {
                    self.segs.clear();
                    self.segs.push(Seg::Move(from.x * s, from.y * s));
                    self.segs.push(Seg::Line(to.x * s, to.y * s));
                    if self.segs_visible(width * s) {
                        self.draw_path(
                            PathStyle::Stroke {
                                width: width * s,
                                round: false,
                            },
                            *color,
                        );
                    }
                }
            }
            DrawCmd::Path {
                commands,
                fill,
                stroke,
            } => {
                self.segs.clear();
                path_segs(commands, s, &mut self.segs);
                let margin = stroke.map_or(0.0, |(_, w)| w * s);
                if !self.segs_visible(margin) {
                    return;
                }
                if let Some(fill) = fill {
                    self.draw_path(PathStyle::Fill, *fill);
                }
                if let Some((color, width)) = stroke {
                    self.draw_path(
                        PathStyle::Stroke {
                            width: width * s,
                            round: true,
                        },
                        *color,
                    );
                }
            }
            DrawCmd::Image {
                rect,
                radii,
                image,
                fit,
            } => {
                let r = rect_px(rect, s, snap);
                let Some(&(texture, w, h)) = self.images.get(image) else {
                    return;
                };
                // On a pixel grid, image pixels stay square. The filter is
                // the texture's, so it follows where the image was drawn last.
                let on_grid = !self.grids.is_empty();
                if on_grid != self.nearest.contains(image) {
                    let filter = if on_grid { glow::NEAREST } else { glow::LINEAR };
                    // SAFETY: the context is current while the shell drives
                    // the renderer, and the texture is alive.
                    unsafe {
                        let gl = &self.gl;
                        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                        gl.tex_parameter_i32(
                            glow::TEXTURE_2D,
                            glow::TEXTURE_MIN_FILTER,
                            filter as i32,
                        );
                        gl.tex_parameter_i32(
                            glow::TEXTURE_2D,
                            glow::TEXTURE_MAG_FILTER,
                            filter as i32,
                        );
                    }
                    if on_grid {
                        self.nearest.insert(*image);
                    } else {
                        self.nearest.remove(image);
                    }
                }
                if r.is_empty() || !self.visible(IRect::outer(&r)) {
                    self.stats.culled += 1;
                    return;
                }
                let radii = fit_radii(radii, s, &r);
                let box_ = astryx_core::Rect::new(r.x0, r.y0, r.width(), r.height());
                let dest = fit.place(box_, w, h);
                // Cover and fill reach past the box, so the quad covers the
                // box and the shape cuts the corners. Contain draws only where
                // the image lands, on whole pixels while snapping, like the
                // CPU renderer.
                let area = if *fit == ImageFit::Contain {
                    let edge = |v: f32| if snap { v.round() } else { v };
                    PxRect {
                        x0: edge(dest.x.max(r.x0)),
                        y0: edge(dest.y.max(r.y0)),
                        x1: edge((dest.x + dest.width).min(r.x1)),
                        y1: edge((dest.y + dest.height).min(r.y1)),
                    }
                } else {
                    r.outset(1.0)
                };
                let uv = |x: f32, y: f32| ((x - dest.x) / dest.width, (y - dest.y) / dest.height);
                let (u0, v0) = uv(area.x0, area.y0);
                let (u1, v1) = uv(area.x1, area.y1);
                self.push(Quad {
                    area,
                    shape: r,
                    radii,
                    uv: [u0, v0, u1, v1],
                    color: [255; 4],
                    mode: MODE_IMAGE,
                    border: 0.0,
                    texture: Some(texture),
                    multiply: false,
                });
            }
            DrawCmd::PushClip { rect, radii } => {
                let r = rect_px(rect, s, snap);
                let radii = fit_radii(radii, s, &r);
                self.push_clip(r, radii);
            }
            DrawCmd::PushOpacity(opacity) => {
                let layer = *opacity < 1.0;
                if layer {
                    self.targets
                        .push(Target::new(Kind::Opacity(opacity.max(0.0))));
                }
                self.groups.push(Group::Opacity { layer });
            }
            DrawCmd::Light(light) => {
                if let Some(pass) = &mut self.lighting {
                    let l = &pass.lighting;
                    pass.lights
                        .push(PlacedLight::new(light, l.time, s, l.strength));
                }
            }
            DrawCmd::Emissive { rect, color } => match &mut self.lighting {
                Some(pass) => pass.emissive.push((*rect, *color, snap)),
                None => self.draw_emissive(rect, *color, s, snap),
            },
            DrawCmd::PopClip
            | DrawCmd::PopOpacity
            | DrawCmd::PushTransform { .. }
            | DrawCmd::PopTransform
            | DrawCmd::PushPixelGrid { .. }
            | DrawCmd::PopPixelGrid
            | DrawCmd::PushLighting(_)
            | DrawCmd::PopLighting => {}
        }
    }

    /// True when the bounding box of the current path, grown by `margin`,
    /// touches the clip. Counts a cull otherwise.
    fn segs_visible(&mut self, margin: f32) -> bool {
        let mut b = PxRect {
            x0: f32::INFINITY,
            y0: f32::INFINITY,
            x1: f32::NEG_INFINITY,
            y1: f32::NEG_INFINITY,
        };
        let mut see = |x: f32, y: f32| {
            b.x0 = b.x0.min(x);
            b.y0 = b.y0.min(y);
            b.x1 = b.x1.max(x);
            b.y1 = b.y1.max(y);
        };
        for seg in &self.segs {
            match *seg {
                Seg::Move(x, y) | Seg::Line(x, y) => see(x, y),
                Seg::Quad(a, c, x, y) => {
                    see(a, c);
                    see(x, y);
                }
                Seg::Cubic(a, c, d, e, x, y) => {
                    see(a, c);
                    see(d, e);
                    see(x, y);
                }
                Seg::Close => {}
            }
        }
        let visible = b.x0.is_finite() && self.visible(IRect::outer(&b.outset(margin + 1.0)));
        if !visible {
            self.stats.culled += 1;
        }
        visible
    }

    fn push_clip(&mut self, rect: PxRect, radii: [f32; 4]) {
        let previous = self.clip;
        let scissor = previous.scissor.intersect(IRect::outer(&rect));
        let shape = ShapeClip { rect, radii };
        let rounded = radii.iter().any(|&r| r > 0.0);
        let mut layer = false;
        let next = match (rounded, previous.shape) {
            (false, _) => ClipState {
                scissor,
                shape: previous.shape,
            },
            (true, None) => ClipState {
                scissor,
                shape: Some(shape),
            },
            (true, Some(outer)) if outer.contains(&shape) => ClipState {
                scissor,
                shape: Some(shape),
            },
            (true, Some(outer)) if shape.contains(&outer) => ClipState {
                scissor,
                shape: Some(outer),
            },
            (true, Some(_)) => {
                layer = true;
                self.targets
                    .push(Target::new(Kind::Clip { outer: previous }));
                ClipState {
                    scissor,
                    shape: Some(shape),
                }
            }
        };
        self.clip = next;
        self.groups.push(Group::Clip { previous, layer });
    }

    fn pop_group(&mut self) -> Option<Group> {
        let group = self.groups.pop()?;
        match group {
            Group::Clip { previous, layer } => {
                self.clip = previous;
                if layer {
                    self.finish_layer();
                }
            }
            Group::Opacity { layer } => {
                if layer {
                    self.finish_layer();
                }
            }
            Group::Transform(previous) => self.transform = previous,
        }
        Some(group)
    }

    /// Closes the innermost layer target and composites it into its parent.
    fn draw_emissive(&mut self, rect: &astryx_core::Rect, color: Color, s: f32, snap: bool) {
        let cmd = DrawCmd::Rect {
            rect: *rect,
            radii: astryx_core::Radii::default(),
            fill: color,
            stroke: None,
        };
        self.draw_moved(&cmd, s, snap);
    }

    /// Renders the light map, multiplies it over the target, then draws
    /// the emissive rects.
    fn finish_lighting(&mut self, s: f32) {
        let Some(pass) = self.lighting.take() else {
            return;
        };
        if pass.lighting.strength > 0.0 {
            let (w, h) = self.frame_size;
            let d = light::MAP_DIVISOR as i32;
            let (mw, mh) = ((w + d - 1) / d, (h + d - 1) / d);
            let target = self.layer((mw, mh));
            let t = now();
            self.draw_light_map(&pass, s, &target, (mw, mh));
            self.stats.flush += since(t);
            let (tw, th) = (target.size.0 as f32, target.size.1 as f32);
            let area = PxRect {
                x0: 0.0,
                y0: 0.0,
                x1: w as f32,
                y1: h as f32,
            };
            // The map's first row is at the top of the texture.
            let (sw, sh) = (w as f32 / d as f32, h as f32 / d as f32);
            let quad = Quad {
                area,
                shape: area,
                radii: [0.0; 4],
                uv: [0.0, 1.0, sw / tw, 1.0 - sh / th],
                color: [255; 4],
                mode: MODE_LAYER,
                border: 0.0,
                texture: Some(target.texture),
                multiply: true,
            };
            self.layers_in_use.push(target);
            let saved = self.clip;
            self.clip = ClipState {
                scissor: IRect {
                    x0: 0,
                    y0: 0,
                    x1: w,
                    y1: h,
                },
                shape: None,
            };
            self.push(quad);
            self.clip = saved;
        }
        for (rect, color, snap) in pass.emissive {
            self.draw_emissive(&rect, color, s, snap);
        }
    }

    /// Draws the light map of `pass` into `target`, `size` map pixels.
    fn draw_light_map(&mut self, pass: &LightingPass, s: f32, target: &Layer, size: (i32, i32)) {
        let lit = &pass.lighting;
        let (mw, mh) = (size.0 as f32, size.1 as f32);
        let d = light::MAP_DIVISOR as f32;
        // Per vertex, three vec4s: position and center (target pixels),
        // radii and kind (0 base, 1 light), color.
        let mut data: Vec<f32> = Vec::with_capacity(48 * (pass.lights.len() + 1));
        let mut quad =
            |r: [f32; 4], center: (f32, f32), radii: (f32, f32), kind: f32, rgb: [f32; 3]| {
                for (x, y) in [(r[0], r[1]), (r[2], r[1]), (r[2], r[3]), (r[0], r[3])] {
                    data.extend_from_slice(&[x, y, center.0, center.1]);
                    data.extend_from_slice(&[radii.0, radii.1, kind, 0.0]);
                    data.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 0.0]);
                }
            };
        quad(
            [0.0, 0.0, mw, mh],
            (0.0, 0.0),
            (1.0, 1.0),
            0.0,
            light::base(lit),
        );
        for l in &pass.lights {
            let (cx, cy) = l.center;
            let (rx, ry) = l.radius;
            if rx <= 0.0 || ry <= 0.0 {
                continue;
            }
            let r = [
                ((cx - rx) / d).floor().max(0.0),
                ((cy - ry) / d).floor().max(0.0),
                ((cx + rx) / d).ceil().min(mw),
                ((cy + ry) / d).ceil().min(mh),
            ];
            if r[0] < r[2] && r[1] < r[3] {
                quad(r, l.center, l.radius, 1.0, l.rgb);
            }
        }
        let quads = data.len() / 48;
        let lattice_on = lit.dapple.filter(|d| d.cell > 0.0 && d.bands >= 2);
        let gl = self.gl.clone();
        let p = &self.light_program;
        // SAFETY: the context is current while rendering; the objects belong to it.
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(target.framebuffer));
            gl.viewport(0, 0, target.size.0, target.size.1);
            gl.disable(glow::SCISSOR_TEST);
            gl.use_program(Some(p.program));
            gl.uniform_2_f32(p.size.as_ref(), target.size.0 as f32, target.size.1 as f32);
            gl.uniform_1_f32(p.divisor.as_ref(), d);
            gl.uniform_1_i32(p.lattice.as_ref(), 0);
            match lattice_on {
                Some(dp) => {
                    gl.uniform_4_f32(
                        p.dapple.as_ref(),
                        1.0 / (dp.cell * s),
                        dp.velocity.x * lit.time / dp.cell,
                        dp.velocity.y * lit.time / dp.cell,
                        f32::from(dp.bands),
                    );
                    gl.uniform_1_f32(
                        p.dapple_depth.as_ref(),
                        dp.depth.clamp(0.0, 1.0) * lit.strength.clamp(0.0, 1.0),
                    );
                }
                None => gl.uniform_1_f32(p.dapple_depth.as_ref(), 0.0),
            }
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.lattice));
            gl.bind_vertex_array(self.vao);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ibo));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.light_vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, as_bytes(&data), glow::STREAM_DRAW);
            for i in 0..7u32 {
                if i < 3 {
                    gl.enable_vertex_attrib_array(i);
                    gl.vertex_attrib_pointer_f32(i, 4, glow::FLOAT, false, 48, i as i32 * 16);
                } else {
                    gl.disable_vertex_attrib_array(i);
                }
            }
            gl.disable(glow::BLEND);
            gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_SHORT, 0);
            if quads > 1 {
                gl.enable(glow::BLEND);
                gl.blend_func(glow::ONE, glow::ONE);
                gl.draw_elements(
                    glow::TRIANGLES,
                    ((quads - 1) * 6) as i32,
                    glow::UNSIGNED_SHORT,
                    12,
                );
            }
            self.stats.draw_calls += 1 + usize::from(quads > 1);
        }
    }

    fn finish_layer(&mut self) {
        let target = self.targets.pop().expect("layer target");
        let Some(bounds) = target.bounds else { return };
        let (alpha, composite_clip) = match target.kind {
            Kind::Opacity(a) => (
                a,
                ClipState {
                    scissor: self.clip.scissor,
                    shape: None,
                },
            ),
            Kind::Clip { outer } => (1.0, outer),
            Kind::Frame => unreachable!("the frame target is never a layer"),
        };
        if alpha <= 0.0 {
            return;
        }
        // A few separate quads fade the same with or without a layer.
        if let Kind::Opacity(a) = target.kind {
            if !target.composited && target.items.len() <= FADE_QUADS && !overlapping(&target.items)
            {
                let parent = self.targets.last_mut().expect("parent target");
                for mut item in target.items {
                    item.quad.color = item.quad.color.map(|c| (c as f32 * a).round() as u8);
                    parent.items.push(item);
                }
                parent.bounds = Some(parent.bounds.map_or(bounds, |b| b.union(bounds)));
                return;
            }
        }
        let key = self.layer_key(&target.items, bounds);
        let layer = match key.and_then(|k| self.layer_cache.remove(&k)) {
            Some(layer) => layer,
            None => {
                let layer = self.layer((bounds.x1 - bounds.x0, bounds.y1 - bounds.y0));
                let t = now();
                self.draw_items(
                    &target.items,
                    Some(layer.framebuffer),
                    layer.size,
                    (bounds.x0, bounds.y0),
                    Some([0.0; 4]),
                );
                self.stats.flush += since(t);
                self.stats.layers += 1;
                layer
            }
        };
        let (w, h) = (bounds.x1 - bounds.x0, bounds.y1 - bounds.y0);
        let (tw, th) = (layer.size.0 as f32, layer.size.1 as f32);
        let area = PxRect {
            x0: bounds.x0 as f32,
            y0: bounds.y0 as f32,
            x1: bounds.x1 as f32,
            y1: bounds.y1 as f32,
        };
        let a = (alpha * 255.0).round() as u8;
        let quad = Quad {
            area,
            shape: area,
            radii: [0.0; 4],
            // Layer rows are stored bottom up.
            uv: [0.0, 1.0, w as f32 / tw, 1.0 - h as f32 / th],
            color: [a, a, a, a],
            mode: MODE_LAYER,
            border: 0.0,
            texture: Some(layer.texture),
            multiply: false,
        };
        match key {
            Some(k) => {
                if let Some(old) = self.layer_cache_next.insert(k, layer) {
                    self.layers_in_use.push(old);
                }
            }
            None => self.layers_in_use.push(layer),
        }
        let saved = self.clip;
        self.clip = composite_clip;
        self.push(quad);
        self.clip = saved;
        if let Some(parent) = self.targets.last_mut() {
            parent.composited = true;
        }
    }

    /// A hash of everything a layer's pixels depend on, or `None` when they
    /// use the per-frame mask page, whose content changes every frame.
    fn layer_key(&self, items: &[Item], bounds: IRect) -> Option<u64> {
        use std::hash::{Hash, Hasher};

        let mut h = std::collections::hash_map::DefaultHasher::new();
        (bounds.x0, bounds.y0, bounds.x1, bounds.y1).hash(&mut h);
        let bits = |h: &mut std::collections::hash_map::DefaultHasher, v: &[f32]| {
            v.iter().for_each(|f| f.to_bits().hash(h))
        };
        for item in items {
            let q = &item.quad;
            if q.multiply || q.texture.is_some_and(|t| self.atlas.is_transient(t)) {
                return None;
            }
            bits(
                &mut h,
                &[
                    q.area.x0, q.area.y0, q.area.x1, q.area.y1, q.shape.x0, q.shape.y0, q.shape.x1,
                    q.shape.y1,
                ],
            );
            bits(&mut h, &q.radii);
            bits(&mut h, &q.uv);
            bits(&mut h, &[q.mode, q.border]);
            q.color.hash(&mut h);
            q.texture.hash(&mut h);
            let c = &item.clip;
            (c.scissor.x0, c.scissor.y0, c.scissor.x1, c.scissor.y1).hash(&mut h);
            if let Some(shape) = c.shape {
                let (a, b) = shape.encode();
                bits(&mut h, &a);
                bits(&mut h, &b);
            }
        }
        Some(h.finish())
    }

    /// A pooled offscreen texture at least `size` large.
    fn layer(&mut self, size: (i32, i32)) -> Layer {
        let size = layer_size(size);
        if let Some(i) = self.layers.iter().position(|l| l.size == size) {
            let mut layer = self.layers.swap_remove(i);
            layer.last_used = self.frame;
            return layer;
        }
        let gl = &self.gl;
        // SAFETY: the context is current while rendering.
        unsafe {
            let texture = gl.create_texture().expect("layer texture");
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            for (k, v) in [
                (glow::TEXTURE_MIN_FILTER, glow::NEAREST),
                (glow::TEXTURE_MAG_FILTER, glow::NEAREST),
                (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
            ] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, k, v as i32);
            }
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                size.0,
                size.1,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );
            self.stats.textures += 1;
            let framebuffer = gl.create_framebuffer().expect("layer framebuffer");
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture),
                0,
            );
            Layer {
                texture,
                framebuffer,
                size,
                last_used: self.frame,
            }
        }
    }

    /// Draws items into a framebuffer whose top left sits at `origin` in
    /// frame pixels.
    fn draw_items(
        &mut self,
        items: &[Item],
        framebuffer: Option<glow::Framebuffer>,
        size: (i32, i32),
        origin: (i32, i32),
        clear: Option<[f32; 4]>,
    ) {
        let gl = self.gl.clone();
        self.atlas.flush(&gl, self.dialect);
        // SAFETY: the context is current while rendering; buffers and
        // textures belong to it.
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, framebuffer);
            gl.viewport(0, 0, size.0, size.1);
            gl.disable(glow::SCISSOR_TEST);
            if let Some([r, g, b, a]) = clear {
                gl.clear_color(r, g, b, a);
                gl.clear(glow::COLOR_BUFFER_BIT);
            }
            if items.is_empty() {
                return;
            }
            gl.use_program(Some(self.program.program));
            gl.uniform_2_f32(self.program.size.as_ref(), size.0 as f32, size.1 as f32);
            gl.uniform_2_f32(
                self.program.origin.as_ref(),
                origin.0 as f32,
                origin.1 as f32,
            );
            gl.uniform_1_i32(self.program.tex.as_ref(), 0);
            gl.active_texture(glow::TEXTURE0);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::STENCIL_TEST);
            gl.disable(glow::CULL_FACE);
            gl.enable(glow::SCISSOR_TEST);
            gl.bind_vertex_array(self.vao);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ibo));

            // Batches: runs of items that share a texture and scissor.
            let mut vertices = std::mem::take(&mut self.vertices);
            vertices.clear();
            let mut batches: Vec<(usize, usize, Option<glow::Texture>, IRect, bool)> = Vec::new();
            for item in items {
                let q = &item.quad;
                let start = vertices.len() / 4;
                let joins = match batches.last_mut() {
                    Some((first, count, texture, scissor, multiply))
                        if *scissor == item.clip.scissor
                            && *count < BATCH_QUADS
                            && *multiply == q.multiply
                            && (q.texture.is_none()
                                || texture.is_none()
                                || *texture == q.texture) =>
                    {
                        if texture.is_none() {
                            *texture = q.texture;
                        }
                        let _ = first;
                        *count += 1;
                        true
                    }
                    _ => false,
                };
                if !joins {
                    batches.push((start, 1, q.texture, item.clip.scissor, q.multiply));
                }
                push_vertices(&mut vertices, item);
            }
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, as_bytes(&vertices), glow::STREAM_DRAW);
            for (i, _) in ATTRIBUTES.iter().enumerate() {
                gl.enable_vertex_attrib_array(i as u32);
            }
            let mut multiplying = false;
            for (first, count, texture, scissor, multiply) in &batches {
                if *multiply != multiplying {
                    multiplying = *multiply;
                    if multiplying {
                        gl.blend_func(glow::DST_COLOR, glow::ZERO);
                    } else {
                        gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
                    }
                }
                let s = scissor.intersect(IRect {
                    x0: origin.0,
                    y0: origin.1,
                    x1: origin.0 + size.0,
                    y1: origin.1 + size.1,
                });
                if s.is_empty() {
                    continue;
                }
                // GL scissor counts rows from the bottom.
                gl.scissor(
                    s.x0 - origin.0,
                    size.1 - (s.y1 - origin.1),
                    s.x1 - s.x0,
                    s.y1 - s.y0,
                );
                if let Some(t) = texture {
                    self.atlas.mark_used(*t);
                }
                gl.bind_texture(glow::TEXTURE_2D, Some(texture.unwrap_or(self.white)));
                let base = (*first * 4) as i32 * STRIDE;
                let float = |index: u32, offset: i32| {
                    gl.vertex_attrib_pointer_f32(
                        index,
                        4,
                        glow::FLOAT,
                        false,
                        STRIDE,
                        base + offset,
                    );
                };
                float(0, 0);
                float(1, 16);
                float(2, 32);
                gl.vertex_attrib_pointer_f32(3, 4, glow::UNSIGNED_BYTE, true, STRIDE, base + 48);
                float(4, 52);
                float(5, 68);
                float(6, 84);
                gl.draw_elements(
                    glow::TRIANGLES,
                    (*count * 6) as i32,
                    glow::UNSIGNED_SHORT,
                    0,
                );
                self.stats.draw_calls += 1;
            }
            self.stats.quads += vertices.len() / 4;
            self.vertices = vertices;
        }
    }

    /// Draws one frame into the current framebuffer, through the running
    /// transition, if any.
    pub fn render_list(&mut self, list: &DisplayList, frame: &FrameInfo) {
        let Some(mut run) = self.transition.take() else {
            return self.render_frame(list, frame);
        };
        run.elapsed += frame.dt.max(0.0);
        let t = run.elapsed * 1000.0 / run.transition.duration_ms as f32;
        let size = (frame.width.max(1) as i32, frame.height.max(1) as i32);
        if t >= 1.0 || run.size != size {
            self.keep_frame_layer(run.old);
            return self.render_frame(list, frame);
        }
        let new = self.frame_layer(size);
        let saved = self.framebuffer;
        self.framebuffer = Some(new.framebuffer);
        self.render_frame(list, frame);
        self.framebuffer = saved;
        let start = now();
        let scale = if frame.scale > 0.0 { frame.scale } else { 1.0 };
        self.draw_transition(&run, &new, t, scale);
        self.stats.flush += since(start);
        self.keep_frame_layer(new);
        self.transition = Some(run);
    }

    /// Starts a transition from `from`, the frame on screen now, to the
    /// frames rendered next.
    pub fn begin_transition(
        &mut self,
        transition: &astryx_core::Transition,
        from: &DisplayList,
        frame: &FrameInfo,
    ) {
        if let Some(run) = self.transition.take() {
            self.keep_frame_layer(run.old);
        }
        if transition.duration_ms == 0 {
            return;
        }
        let size = (frame.width.max(1) as i32, frame.height.max(1) as i32);
        let old = self.frame_layer(size);
        let saved = self.framebuffer;
        self.framebuffer = Some(old.framebuffer);
        self.render_frame(from, frame);
        self.framebuffer = saved;
        self.transition = Some(Running {
            transition: *transition,
            elapsed: 0.0,
            old,
            size,
        });
    }

    /// A transition is running.
    pub fn transitioning(&self) -> bool {
        self.transition.is_some()
    }

    /// Creates the two textures a transition draws through, for frames of
    /// this size, so the first transition does not wait for them.
    pub fn prepare_transitions(&mut self, width: u32, height: u32) {
        let size = (width.max(1) as i32, height.max(1) as i32);
        let (keep, other): (Vec<Layer>, Vec<Layer>) = self
            .frame_layers
            .drain(..)
            .partition(|l| l.size == layer_size(size));
        self.frame_layers = keep;
        self.layers.extend(other);
        while self.frame_layers.len() < 2 {
            let l = self.layer(size);
            // SAFETY: the context is current; the framebuffer belongs to it.
            unsafe {
                self.gl
                    .bind_framebuffer(glow::FRAMEBUFFER, Some(l.framebuffer));
                self.gl.viewport(0, 0, l.size.0, l.size.1);
                self.gl.disable(glow::SCISSOR_TEST);
                self.gl.clear_color(0.0, 0.0, 0.0, 0.0);
                self.gl.clear(glow::COLOR_BUFFER_BIT);
            }
            self.frame_layers.push(l);
        }
    }

    fn frame_layer(&mut self, size: (i32, i32)) -> Layer {
        match self
            .frame_layers
            .iter()
            .position(|l| l.size == layer_size(size))
        {
            Some(i) => self.frame_layers.swap_remove(i),
            None => self.layer(size),
        }
    }

    fn keep_frame_layer(&mut self, layer: Layer) {
        if self.frame_layers.len() < 2 {
            self.frame_layers.push(layer);
        } else {
            self.layers_in_use.push(layer);
        }
    }

    /// Composites the old and new frames of `run` at time `t` into the
    /// framebuffer.
    fn draw_transition(&mut self, run: &Running, new: &Layer, t: f32, scale: f32) {
        use astryx_core::{Edge, TransitionKind as K};
        let (w, h) = (run.size.0 as f32, run.size.1 as f32);
        let m = Moment::new(run.transition.kind, run.transition.easing, t, w, h, scale);
        let p = m.p;
        let (kind, param) = match run.transition.kind {
            K::Fade => (0.0, [0.0; 4]),
            K::Wipe(edge) => {
                let (e, l) = match edge {
                    Edge::Left => (0.0, p * w),
                    Edge::Right => (1.0, (1.0 - p) * w),
                    Edge::Top => (2.0, p * h),
                    Edge::Bottom => (3.0, (1.0 - p) * h),
                };
                (1.0, [e, l, 0.0, 0.0])
            }
            K::Door => {
                let inset = 0.5 * (1.0 - p);
                (
                    2.0,
                    [inset * w, inset * h, (1.0 - inset) * w, (1.0 - inset) * h],
                )
            }
            K::Iris(c) => {
                let (cx, cy) = (c.x * scale, c.y * scale);
                let far = [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)]
                    .iter()
                    .map(|(x, y)| ((x - cx).powi(2) + (y - cy).powi(2)).sqrt())
                    .fold(0.0, f32::max);
                (3.0, [cx, cy, p * far, 0.0])
            }
            K::Slide(o) => (
                4.0,
                [o.x * scale * (1.0 - p), o.y * scale * (1.0 - p), 0.0, 0.0],
            ),
            K::PixelDissolve(cell) => (5.0, [(cell * scale).max(1.0), 0.0, 0.0, 0.0]),
            K::Swirl => {
                let (cx, cy) = (w / 2.0, h / 2.0);
                (6.0, [cx, cy, (cx * cx + cy * cy).sqrt(), 0.0])
            }
            K::Encounter => {
                let (white, bars) = m.encounter();
                (7.0, [white, bars, 0.0, 0.0])
            }
        };
        let data: [f32; 16] = [
            0.0, 0.0, 0.0, 0.0, w, 0.0, 0.0, 0.0, w, h, 0.0, 0.0, 0.0, h, 0.0, 0.0,
        ];
        let gl = self.gl.clone();
        let tp = &self.transition_program;
        // SAFETY: the context is current while rendering; the objects belong to it.
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, self.framebuffer);
            gl.viewport(0, 0, run.size.0, run.size.1);
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::BLEND);
            gl.use_program(Some(tp.program));
            gl.uniform_2_f32(tp.size.as_ref(), w, h);
            gl.uniform_2_f32(
                tp.extent.as_ref(),
                w / run.old.size.0 as f32,
                h / run.old.size.1 as f32,
            );
            gl.uniform_1_f32(tp.kind.as_ref(), kind);
            gl.uniform_1_f32(tp.p.as_ref(), p);
            gl.uniform_4_f32(tp.param.as_ref(), param[0], param[1], param[2], param[3]);
            for (unit, texture, at) in [
                (0, run.old.texture, &tp.old),
                (1, new.texture, &tp.new),
                (2, self.lattice, &tp.lattice),
            ] {
                gl.active_texture(glow::TEXTURE0 + unit);
                gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                gl.uniform_1_i32(at.as_ref(), unit as i32);
            }
            gl.active_texture(glow::TEXTURE0);
            gl.bind_vertex_array(self.vao);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ibo));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.light_vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, as_bytes(&data), glow::STREAM_DRAW);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 4, glow::FLOAT, false, 16, 0);
            for i in 1..7u32 {
                gl.disable_vertex_attrib_array(i);
            }
            gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_SHORT, 0);
            self.stats.draw_calls += 1;
        }
    }

    /// Draws one frame into the current framebuffer.
    fn render_frame(&mut self, list: &DisplayList, frame: &FrameInfo) {
        let start = now();
        let (w, h) = (frame.width.max(1) as i32, frame.height.max(1) as i32);
        let scale = if frame.scale > 0.0 { frame.scale } else { 1.0 };
        self.frame += 1;
        self.grids.clear();
        self.frame_size = (w, h);
        self.stats = RenderStats {
            commands: list.commands.len(),
            ..RenderStats::default()
        };

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(rx) = &self.shadow_prewarm {
            loop {
                match rx.try_recv() {
                    Ok(batch) => self.prewarmed_shadows.extend(batch),
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.shadow_prewarm = None;
                        break;
                    }
                }
            }
        }
        if self.atlas.begin_frame(&self.gl) {
            self.paths.clear();
            self.paths_seen.clear();
            // New mask textures may reuse old names, so cached layers could match wrongly.
            self.layers.extend(self.layer_cache.drain().map(|(_, l)| l));
            self.shadow_places.clear();
        }
        let frame_no = self.frame;
        let gl = self.gl.clone();
        self.layers.retain(|l| {
            let keep = frame_no - l.last_used <= LAYER_FRAMES;
            if !keep {
                // SAFETY: the objects belong to this context.
                unsafe {
                    gl.delete_texture(l.texture);
                    gl.delete_framebuffer(l.framebuffer);
                }
            }
            keep
        });

        self.clip = ClipState {
            scissor: IRect {
                x0: 0,
                y0: 0,
                x1: w,
                y1: h,
            },
            shape: None,
        };
        self.targets.clear();
        self.groups.clear();
        self.transform = Affine::IDENTITY;
        self.targets.push(Target::new(Kind::Frame));
        self.lighting = None;
        for cmd in &list.commands {
            self.draw(cmd, scale);
        }
        self.finish_lighting(scale);
        while self.pop_group().is_some() {}
        let frame_target = self.targets.pop().expect("frame target");
        let encoded = now();
        let c = list.clear;
        let a = c.a.clamp(0.0, 1.0);
        let clear = [c.r * a, c.g * a, c.b * a, a];
        self.draw_items(
            &frame_target.items,
            self.framebuffer,
            (w, h),
            (0, 0),
            Some(clear),
        );
        self.layers.append(&mut self.layers_in_use);
        // Cached layers not used this frame go back to the pool.
        self.layers.extend(self.layer_cache.drain().map(|(_, l)| l));
        std::mem::swap(&mut self.layer_cache, &mut self.layer_cache_next);
        let frame_no = self.frame;
        for l in self.layer_cache.values_mut() {
            l.last_used = frame_no;
        }
        let flushed = since(encoded);
        self.stats.encode = since(start)
            .saturating_sub(flushed)
            .saturating_sub(self.stats.flush);
        self.stats.flush += flushed;
        self.stats.glyphs = self.text.take_rasterized();
        self.stats.uploads = self.atlas.uploads;
        self.stats.upload_time = self.atlas.upload_time;
        self.stats.upload_kinds = self.atlas.upload_kinds;
        self.stats.textures += self.atlas.textures;
    }

    /// Draws one small frame with every kind of draw offscreen, so the
    /// driver compiles its shader variants and allocates its first textures
    /// before the first real frame.
    fn warm_up(&mut self) {
        use astryx_core::{FontFamily, Point, Radii, Rect, ShadowLayer, TextStyle};
        self.atlas.prepare(&self.gl, self.dialect);
        let r = |x, y| Rect::new(x, y, 24.0, 24.0);
        let red = Color::rgba(1.0, 0.0, 0.0, 1.0);
        let list = DisplayList {
            clear: Color::rgba(1.0, 1.0, 1.0, 1.0),
            commands: vec![
                DrawCmd::PushLighting(astryx_core::Lighting {
                    ambient: red,
                    strength: 0.5,
                    time: 0.0,
                    dapple: Some(astryx_core::Dapple {
                        cell: 8.0,
                        velocity: Point { x: 1.0, y: 0.0 },
                        bands: 3,
                        depth: 0.5,
                    }),
                }),
                DrawCmd::Light(astryx_core::Light {
                    center: Point { x: 20.0, y: 20.0 },
                    radius_x: 16.0,
                    radius_y: 12.0,
                    color: red,
                    intensity: 1.0,
                    flicker: Default::default(),
                    pulse: Default::default(),
                }),
                DrawCmd::Shadow {
                    rect: r(4.0, 4.0),
                    radii: Radii::all(6.0),
                    layers: vec![ShadowLayer {
                        x: 0.0,
                        y: 2.0,
                        blur: 4.0,
                        spread: 0.0,
                        inset: false,
                        color: red,
                    }],
                },
                DrawCmd::Rect {
                    rect: r(4.0, 4.0),
                    radii: Radii::all(6.0),
                    fill: red,
                    stroke: Some((red, 1.0)),
                },
                DrawCmd::Text {
                    rect: r(0.0, 0.0),
                    text: "A".into(),
                    style: TextStyle {
                        family: FontFamily::Body,
                        size: 12.0,
                        weight: 400,
                        leading: 1.4,
                        direction: None,
                    },
                    color: red,
                    align: TextAlign::Start,
                },
                DrawCmd::PushClip {
                    rect: r(2.0, 2.0),
                    radii: Radii::all(8.0),
                },
                DrawCmd::PushClip {
                    rect: r(12.0, 12.0),
                    radii: Radii::all(8.0),
                },
                DrawCmd::PushOpacity(0.5),
                DrawCmd::Rect {
                    rect: r(8.0, 8.0),
                    radii: Radii::all(0.0),
                    fill: red,
                    stroke: None,
                },
                DrawCmd::Rect {
                    rect: r(10.0, 10.0),
                    radii: Radii::all(0.0),
                    fill: red,
                    stroke: None,
                },
                DrawCmd::PopOpacity,
                DrawCmd::PopClip,
                DrawCmd::PopClip,
                DrawCmd::Line {
                    from: Point { x: 0.0, y: 0.0 },
                    to: Point { x: 30.0, y: 20.0 },
                    color: red,
                    width: 1.0,
                },
                DrawCmd::PopLighting,
            ],
        };
        let target = self.layer((64, 64));
        let saved = self.framebuffer;
        self.framebuffer = Some(target.framebuffer);
        let small = FrameInfo {
            width: 64,
            height: 64,
            scale: 1.0,
            dt: 0.0,
        };
        self.render_list(&list, &small);
        // One transition frame, so its program is ready too.
        let fade = astryx_core::Transition {
            kind: astryx_core::TransitionKind::Swirl,
            duration_ms: 1000,
            easing: astryx_core::Easing::Linear,
            sound: None,
        };
        self.begin_transition(&fade, &list, &small);
        self.render_list(&list, &FrameInfo { dt: 0.5, ..small });
        if let Some(run) = self.transition.take() {
            self.layers.push(run.old);
        }
        self.layers.append(&mut self.frame_layers);
        self.framebuffer = saved;
        // SAFETY: the context is current while the renderer is created.
        unsafe { self.gl.finish() };
        self.layers.push(target);
        // Layers cached from the warm-up go back to the pool. The frame
        // counter keeps running: pooled layers carry the frames they were
        // last used in.
        let cached: Vec<Layer> = self.layer_cache.drain().map(|(_, l)| l).collect();
        self.layers.extend(cached);
    }

    /// Reads back the frame just drawn as straight-alpha RGBA rows, top row
    /// first. Call after [`Renderer::render`] and before swapping buffers.
    pub fn read_pixels(&mut self) -> Result<Vec<u8>, GlError> {
        let (w, h) = self.frame_size;
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        // SAFETY: the buffer matches the requested size and format.
        unsafe {
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, self.framebuffer);
            self.gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            self.gl.read_pixels(
                0,
                0,
                w,
                h,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
        }
        let stride = (w * 4) as usize;
        let mut out = Vec::with_capacity(pixels.len());
        for row in pixels.chunks_exact(stride).rev() {
            out.extend_from_slice(row);
        }
        for px in out.as_chunks_mut::<4>().0 {
            let a = px[3] as u32;
            if a > 0 && a < 255 {
                for c in &mut px[..3] {
                    *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
                }
            }
        }
        Ok(out)
    }
}

impl Drop for GlRenderer {
    fn drop(&mut self) {
        let gl = &self.gl;
        self.atlas.clear(gl);
        // SAFETY: the objects belong to this context; the shell keeps it
        // alive until the renderer is gone.
        unsafe {
            let cached: Vec<Layer> = self
                .layer_cache
                .drain()
                .chain(self.layer_cache_next.drain())
                .map(|(_, l)| l)
                .collect();
            for l in self
                .layers
                .drain(..)
                .chain(self.layers_in_use.drain(..))
                .chain(cached)
            {
                gl.delete_texture(l.texture);
                gl.delete_framebuffer(l.framebuffer);
            }
            for (_, (texture, _, _)) in self.images.drain() {
                gl.delete_texture(texture);
            }
            gl.delete_texture(self.white);
            gl.delete_buffer(self.vbo);
            gl.delete_buffer(self.ibo);
            if let Some(vao) = self.vao {
                gl.delete_vertex_array(vao);
            }
            gl.delete_program(self.program.program);
            gl.delete_program(self.light_program.program);
            gl.delete_program(self.transition_program.program);
            let running = self.transition.take().map(|r| r.old);
            for l in running.into_iter().chain(self.frame_layers.drain(..)) {
                gl.delete_texture(l.texture);
                gl.delete_framebuffer(l.framebuffer);
            }
            gl.delete_buffer(self.light_vbo);
            gl.delete_texture(self.lattice);
        }
    }
}

fn rmax(radii: [f32; 4]) -> f32 {
    radii.iter().fold(0.0f32, |a, &r| a.max(r))
}

fn overlapping(items: &[Item]) -> bool {
    let rects: Vec<IRect> = items.iter().map(|i| IRect::outer(&i.quad.area)).collect();
    rects
        .iter()
        .enumerate()
        .any(|(i, a)| rects[i + 1..].iter().any(|b| a.overlaps(*b)))
}

fn push_vertices(out: &mut Vec<Vertex>, item: &Item) {
    let q = &item.quad;
    let (clip, clip_radii) = item.clip.shape.map_or(NO_SHAPE, |s| s.encode());
    let (cx, cy) = (
        (q.shape.x0 + q.shape.x1) / 2.0,
        (q.shape.y0 + q.shape.y1) / 2.0,
    );
    let half = [q.shape.width() / 2.0, q.shape.height() / 2.0];
    let a = &q.area;
    let [u0, v0, u1, v1] = q.uv;
    for (x, y, u, v) in [
        (a.x0, a.y0, u0, v0),
        (a.x1, a.y0, u1, v0),
        (a.x1, a.y1, u1, v1),
        (a.x0, a.y1, u0, v1),
    ] {
        out.push(Vertex {
            pos_local: [x, y, x - cx, y - cy],
            half_uv: [half[0], half[1], u, v],
            radii: q.radii,
            color: q.color,
            params: [
                q.mode,
                q.border,
                item.clip.shape.is_some() as u8 as f32,
                0.0,
            ],
            clip,
            clip_radii,
        });
    }
}

fn premultiplied(c: Color) -> [u8; 4] {
    let a = c.a.clamp(0.0, 1.0);
    let ch = |v: f32| (v.clamp(0.0, 1.0) * a * 255.0).round() as u8;
    [ch(c.r), ch(c.g), ch(c.b), (a * 255.0).round() as u8]
}

fn as_bytes<T: Copy>(data: &[T]) -> &[u8] {
    // SAFETY: T is plain old data (u16 or the repr(C) vertex without padding).
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, std::mem::size_of_val(data)) }
}

impl Renderer for GlRenderer {
    fn measurer(&self) -> &dyn TextMeasure {
        &self.text
    }

    fn render(&mut self, list: &DisplayList, frame: &FrameInfo) {
        self.render_list(list, frame);
    }

    fn set_image(&mut self, id: ImageId, data: Option<&ImageData>) {
        GlRenderer::set_image(self, id, data);
    }

    fn begin_transition(
        &mut self,
        transition: &astryx_core::Transition,
        from: &DisplayList,
        frame: &FrameInfo,
    ) {
        GlRenderer::begin_transition(self, transition, from, frame);
    }

    fn transitioning(&self) -> bool {
        GlRenderer::transitioning(self)
    }
}

/// The family a pixel theme's text is set in, which draws on the grid.
fn own_family(style: &astryx_core::TextStyle) -> Option<&'static str> {
    match style.family {
        astryx_core::FontFamily::Theme(font) => Some(font.family),
        _ => None,
    }
}
