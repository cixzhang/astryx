//! Astryx GPU renderer: draws a [`DisplayList`] with one small shader into
//! the current OpenGL ES 2/3, WebGL2, or desktop OpenGL 3.3 context.
//!
//! Every draw command becomes quads in one vertex buffer, drawn in a few
//! batched calls:
//! - Rounded rectangles and borders are signed-distance shapes in the shader.
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

use astryx_core::{Color, DisplayList, DrawCmd, FrameInfo, Renderer, TextAlign, TextMeasure};
use astryx_raster::geom::{
    fit_radii, line_rect, path_segs, rect_px, snap_rect, width_px, Affine, PxRect, Seg,
};
use astryx_raster::path::{self, PathKey, PathStyle};
use astryx_raster::shadow::{ShadowCache, ShadowKey};
use astryx_raster::text::{FontData, FontError, FontSet, GlyphDraw, TextEngine};
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
    vbo: glow::Buffer,
    ibo: glow::Buffer,
    white: glow::Texture,
    framebuffer: Option<glow::Framebuffer>,
    text: TextEngine,
    shadows: ShadowCache,
    paths: HashMap<PathKey, Option<(i32, i32, u32, u32)>>,
    /// Paths drawn once, by the frame they were last drawn in. A path seen
    /// again in a later frame earns a place in the atlas; animated paths
    /// that never repeat use the per-frame page instead.
    paths_seen: HashMap<PathKey, u64>,
    atlas: Atlas,
    layers: Vec<Layer>,
    layers_in_use: Vec<Layer>,
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
}

impl GlRenderer {
    /// Takes over a GL context. Uses the embedded Roboto faces, or no faces
    /// when the `embedded-fonts` feature is off; add them later with
    /// [`Self::add_fallback_font`].
    pub fn new(gl: glow::Context) -> Result<Self, GlError> {
        #[cfg(feature = "embedded-fonts")]
        let fonts = astryx_fonts::roboto();
        #[cfg(not(feature = "embedded-fonts"))]
        let fonts = FontSet::new();
        Self::with_fonts(gl, fonts)
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
            Ok(GlRenderer {
                gl: Rc::new(gl),
                dialect,
                program,
                vao,
                vbo,
                ibo,
                white,
                framebuffer: None,
                text: TextEngine::new(fonts),
                shadows: ShadowCache::default(),
                paths: HashMap::new(),
                paths_seen: HashMap::new(),
                atlas: Atlas::default(),
                layers: Vec::new(),
                layers_in_use: Vec::new(),
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
            })
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
        });
    }

    fn visible(&self, r: IRect) -> bool {
        r.overlaps(self.clip.scissor)
    }

    fn draw_path(&mut self, style: PathStyle, color: Color) {
        let Some(prepared) = path::prepare(&self.segs, style) else {
            return;
        };
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
        let map = |x: f32, y: f32| (x * t.scale + t.offset.x * s, y * t.scale + t.offset.y * s);
        for layer in layers.iter().rev() {
            let key = ShadowKey::new(&r, radii, layer, s);
            let mk = MaskKey::Shadow(key);
            let Some(mask) = self.shadows.get(&key) else {
                continue;
            };
            let (x0, y0) = map(r.x0 + mask.x as f32, r.y0 + mask.y as f32);
            let (x1, y1) = map(
                r.x0 + (mask.x + mask.width as i32) as f32,
                r.y0 + (mask.y + mask.height as i32) as f32,
            );
            let area = PxRect { x0, y0, x1, y1 };
            if !self.visible(IRect::outer(&area)) {
                continue;
            }
            let region = match self.atlas.get(&mk) {
                Some(region) => region,
                None => self.atlas.insert(
                    &self.gl,
                    self.dialect,
                    mk,
                    mask.width,
                    mask.height,
                    &mask.data,
                ),
            };
            self.mask_area(area, region, layer.color);
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
                    let mk = MaskKey::Shadow(key);
                    let Some(mask) = self.shadows.get(&key) else {
                        continue;
                    };
                    let region = match self.atlas.get(&mk) {
                        Some(region) => region,
                        None => self.atlas.insert(
                            &self.gl,
                            self.dialect,
                            mk,
                            mask.width,
                            mask.height,
                            &mask.data,
                        ),
                    };
                    let (x, y) = (r.x0 as i32 + mask.x, r.y0 as i32 + mask.y);
                    self.mask(x, y, mask.width, mask.height, region, layer.color);
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
                self.text
                    .place(&layout, (rect.x, rect.y), s, snap, &mut placed);
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
            DrawCmd::PopClip
            | DrawCmd::PopOpacity
            | DrawCmd::PushTransform { .. }
            | DrawCmd::PopTransform => {}
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
        let layer = self.layer((bounds.x1 - bounds.x0, bounds.y1 - bounds.y0));
        let origin = (bounds.x0, bounds.y0);
        let t = now();
        self.draw_items(
            &target.items,
            Some(layer.framebuffer),
            layer.size,
            origin,
            Some([0.0; 4]),
        );
        self.stats.flush += since(t);
        self.stats.layers += 1;
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
        };
        self.layers_in_use.push(layer);
        let saved = self.clip;
        self.clip = composite_clip;
        self.push(quad);
        self.clip = saved;
        if let Some(parent) = self.targets.last_mut() {
            parent.composited = true;
        }
    }

    /// A pooled offscreen texture at least `size` large.
    fn layer(&mut self, size: (i32, i32)) -> Layer {
        let round = |v: i32| ((v + 63) / 64) * 64;
        let size = (round(size.0.max(1)), round(size.1.max(1)));
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
            let mut batches: Vec<(usize, usize, Option<glow::Texture>, IRect)> = Vec::new();
            for item in items {
                let q = &item.quad;
                let start = vertices.len() / 4;
                let joins = match batches.last_mut() {
                    Some((first, count, texture, scissor))
                        if *scissor == item.clip.scissor
                            && *count < BATCH_QUADS
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
                    batches.push((start, 1, q.texture, item.clip.scissor));
                }
                push_vertices(&mut vertices, item);
            }
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, as_bytes(&vertices), glow::STREAM_DRAW);
            for (i, _) in ATTRIBUTES.iter().enumerate() {
                gl.enable_vertex_attrib_array(i as u32);
            }
            for (first, count, texture, scissor) in &batches {
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

    /// Draws one frame into the current framebuffer.
    pub fn render_list(&mut self, list: &DisplayList, frame: &FrameInfo) {
        let start = now();
        let (w, h) = (frame.width.max(1) as i32, frame.height.max(1) as i32);
        let scale = if frame.scale > 0.0 { frame.scale } else { 1.0 };
        self.frame += 1;
        self.frame_size = (w, h);
        self.stats = RenderStats {
            commands: list.commands.len(),
            ..RenderStats::default()
        };
        self.shadows.begin_frame();
        if self.atlas.begin_frame(&self.gl) {
            self.paths.clear();
            self.paths_seen.clear();
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
        for cmd in &list.commands {
            self.draw(cmd, scale);
        }
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
        let flushed = since(encoded);
        self.stats.encode = since(start)
            .saturating_sub(flushed)
            .saturating_sub(self.stats.flush);
        self.stats.flush += flushed;
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
            for l in self.layers.drain(..).chain(self.layers_in_use.drain(..)) {
                gl.delete_texture(l.texture);
                gl.delete_framebuffer(l.framebuffer);
            }
            gl.delete_texture(self.white);
            gl.delete_buffer(self.vbo);
            gl.delete_buffer(self.ibo);
            if let Some(vao) = self.vao {
                gl.delete_vertex_array(vao);
            }
            gl.delete_program(self.program.program);
        }
    }
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
            params: [q.mode, q.border, 0.0, 0.0],
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
}
