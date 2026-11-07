//! Astryx GPU renderer: draws a [`DisplayList`] with `femtovg` on `glow`,
//! into the current OpenGL ES 2/3 or WebGL2 context.
//!
//! Text and shadows go through the same code as the CPU renderer: glyphs are
//! shaped and rasterized by [`astryx_raster::text`] and shadow masks
//! come from [`astryx_raster::shadow`]; this renderer uploads them as
//! textures and caches them. Vector shapes, rounded clips, and opacity
//! groups use femtovg directly. So layout is identical and pixels match the
//! CPU renderer closely.

use std::collections::HashMap;

use astryx_core::{Color, DisplayList, DrawCmd, FrameInfo, Renderer, TextMeasure};
use astryx_raster::geom::{
    border_ring, fit_radii, path_segs, rounded_rect, snap_line, snap_rect, snap_width, PxRect, Seg,
};
use astryx_raster::shadow::{ShadowCache, ShadowKey};
use astryx_raster::text::{GlyphBitmap, GlyphDraw, GlyphKey, GlyphRasterizer, TextEngine};
use femtovg::imgref::Img;
use femtovg::renderer::OpenGl;
use femtovg::rgb::FromSlice;
use femtovg::{
    Canvas, DrawCommand, FillRule, GlyphDrawCommands, ImageFlags, ImageId, ImageSource,
    LayerEffects, LineCap, LineJoin, Paint, Path, Quad,
};

pub use femtovg;
pub use glow;

fn fv_color(c: Color) -> femtovg::Color {
    femtovg::Color::rgbaf(
        c.r.clamp(0.0, 1.0),
        c.g.clamp(0.0, 1.0),
        c.b.clamp(0.0, 1.0),
        c.a.clamp(0.0, 1.0),
    )
}

fn fv_path(segs: &[Seg]) -> Path {
    let mut path = Path::new();
    for seg in segs {
        match *seg {
            Seg::Move(x, y) => path.move_to(x, y),
            Seg::Line(x, y) => path.line_to(x, y),
            Seg::Quad(a, b, x, y) => path.quad_to(a, b, x, y),
            Seg::Cubic(a, b, c, d, x, y) => path.bezier_to(a, b, c, d, x, y),
            Seg::Close => path.close(),
        }
    }
    path
}

const ATLAS_SIZE: u32 = 1024;
/// Pages kept across frames. More may be added within a frame; the atlas
/// resets at the start of the next one.
const ATLAS_PAGES: usize = 4;

struct Page {
    image: ImageId,
    x: u32,
    y: u32,
    row: u32,
}

#[derive(Clone, Copy)]
struct Slot {
    page: usize,
    x: u32,
    y: u32,
}

/// Glyph bitmaps packed into 8-bit textures with a shelf packer.
#[derive(Default)]
struct Atlas {
    pages: Vec<Page>,
    slots: HashMap<GlyphKey, Slot>,
}

impl Atlas {
    fn begin_frame(&mut self, canvas: &mut Canvas<OpenGl>) {
        if self.pages.len() > ATLAS_PAGES {
            for page in self.pages.drain(..) {
                canvas.delete_image(page.image);
            }
            self.slots.clear();
        }
    }

    fn slot(
        &mut self,
        canvas: &mut Canvas<OpenGl>,
        key: GlyphKey,
        bitmap: &GlyphBitmap,
    ) -> Option<Slot> {
        if let Some(slot) = self.slots.get(&key) {
            return Some(*slot);
        }
        let (w, h) = (bitmap.width + 1, bitmap.height + 1);
        if w > ATLAS_SIZE || h > ATLAS_SIZE {
            return None;
        }
        let fits = |p: &Page| {
            (p.x + w <= ATLAS_SIZE && p.y + h.max(p.row) <= ATLAS_SIZE)
                || p.y + p.row + h <= ATLAS_SIZE
        };
        let index = match self.pages.iter().position(fits) {
            Some(i) => i,
            None => {
                let image = canvas
                    .create_image(
                        ImageSource::Gray(Img::new(
                            vec![0u8; (ATLAS_SIZE * ATLAS_SIZE) as usize].as_gray(),
                            ATLAS_SIZE as usize,
                            ATLAS_SIZE as usize,
                        )),
                        ImageFlags::NEAREST,
                    )
                    .ok()?;
                self.pages.push(Page {
                    image,
                    x: 0,
                    y: 0,
                    row: 0,
                });
                self.pages.len() - 1
            }
        };
        let page = &mut self.pages[index];
        if page.x + w > ATLAS_SIZE {
            page.x = 0;
            page.y += page.row;
            page.row = 0;
        }
        let slot = Slot {
            page: index,
            x: page.x,
            y: page.y,
        };
        page.x += w;
        page.row = page.row.max(h);
        let src = Img::new(
            bitmap.data.as_gray(),
            bitmap.width as usize,
            bitmap.height as usize,
        );
        canvas
            .update_image(
                page.image,
                ImageSource::Gray(src),
                slot.x as usize,
                slot.y as usize,
            )
            .ok()?;
        self.slots.insert(key, slot);
        Some(slot)
    }
}

/// Frames an unused shadow texture stays alive.
const SHADOW_TEXTURE_FRAMES: u64 = 120;

enum Group {
    Clip,
    Layer,
    Pass,
}

/// The GPU renderer. Draws into the GL context that is current when
/// [`Renderer::render`] runs, at the frame's pixel size.
pub struct GlRenderer {
    canvas: Canvas<OpenGl>,
    text: TextEngine,
    glyphs: GlyphRasterizer,
    atlas: Atlas,
    shadows: ShadowCache,
    shadow_textures: HashMap<ShadowKey, (ImageId, u64)>,
    frame: u64,
    groups: Vec<Group>,
    segs: Vec<Seg>,
    placed: Vec<GlyphDraw>,
}

impl GlRenderer {
    /// Wraps a femtovg OpenGL backend. On the web, create it with
    /// `OpenGl::new_from_html_canvas`.
    pub fn new(backend: OpenGl) -> Result<Self, femtovg::ErrorKind> {
        Ok(Self {
            canvas: Canvas::new(backend)?,
            text: TextEngine::new(),
            glyphs: GlyphRasterizer::new(),
            atlas: Atlas::default(),
            shadows: ShadowCache::default(),
            shadow_textures: HashMap::new(),
            frame: 0,
            groups: Vec::new(),
            segs: Vec::new(),
            placed: Vec::new(),
        })
    }

    /// Creates the renderer from a GL function loader for the current context.
    ///
    /// # Safety
    /// A GL context must be current on this thread and `load` must return
    /// that context's function pointers.
    #[cfg(not(target_arch = "wasm32"))]
    pub unsafe fn from_loader(
        load: impl FnMut(&std::ffi::CStr) -> *const std::ffi::c_void,
    ) -> Result<Self, femtovg::ErrorKind> {
        Self::new(OpenGl::new_from_function_cstr(load)?)
    }

    /// Draws into this framebuffer instead of the default one, for offscreen
    /// rendering. `None` restores the default framebuffer.
    pub fn set_framebuffer(
        &mut self,
        framebuffer: Option<<glow::Context as glow::HasContext>::Framebuffer>,
    ) {
        self.canvas.renderer_mut().set_screen_target(framebuffer);
    }

    pub fn text(&self) -> &TextEngine {
        &self.text
    }

    pub fn canvas_mut(&mut self) -> &mut Canvas<OpenGl> {
        &mut self.canvas
    }

    fn fill(&mut self, rule: FillRule, color: Color, anti_alias: bool) {
        let path = fv_path(&self.segs);
        let mut paint = Paint::color(fv_color(color));
        paint.set_fill_rule(rule);
        paint.set_anti_alias(anti_alias);
        self.canvas.fill_path(&path, &paint);
    }

    fn stroke(&mut self, width: f32, color: Color, round: bool) {
        let path = fv_path(&self.segs);
        let mut paint = Paint::color(fv_color(color));
        paint.set_line_width(width);
        paint.set_line_cap(if round { LineCap::Round } else { LineCap::Butt });
        paint.set_line_join(if round {
            LineJoin::Round
        } else {
            LineJoin::Miter
        });
        self.canvas.stroke_path(&path, &paint);
    }

    /// Fills a rounded rectangle. Square, pixel-aligned rectangles skip
    /// antialiasing: their edges already sit on pixel boundaries.
    fn fill_rounded(&mut self, rect: &PxRect, radii: [f32; 4], color: Color) {
        self.segs.clear();
        rounded_rect(rect, radii, &mut self.segs);
        self.fill(FillRule::NonZero, color, radii.iter().any(|&r| r > 0.0));
    }

    fn draw_mask(&mut self, image: ImageId, rect: PxRect, color: Color) {
        let quad = Quad {
            x0: rect.x0,
            y0: rect.y0,
            s0: 0.0,
            t0: 0.0,
            x1: rect.x1,
            y1: rect.y1,
            s1: 1.0,
            t1: 1.0,
        };
        let commands = GlyphDrawCommands {
            alpha_glyphs: vec![DrawCommand {
                image_id: image,
                quads: vec![quad],
            }],
            color_glyphs: Vec::new(),
        };
        self.canvas
            .draw_glyph_commands(commands, &Paint::color(fv_color(color)));
    }

    fn shadow_texture(&mut self, key: &ShadowKey) -> Option<(ImageId, i32, i32, u32, u32)> {
        let mask = self.shadows.get(key)?;
        let frame = self.frame;
        if let Some((image, used)) = self.shadow_textures.get_mut(key) {
            *used = frame;
            return Some((*image, mask.x, mask.y, mask.width, mask.height));
        }
        let src = Img::new(
            mask.data.as_gray(),
            mask.width as usize,
            mask.height as usize,
        );
        let image = self
            .canvas
            .create_image(ImageSource::Gray(src), ImageFlags::NEAREST)
            .ok()?;
        self.shadow_textures.insert(*key, (image, frame));
        Some((image, mask.x, mask.y, mask.width, mask.height))
    }

    fn pop_group(&mut self) -> Option<Group> {
        let group = self.groups.pop()?;
        match group {
            Group::Clip => self.canvas.restore(),
            Group::Layer => self.canvas.end_layer(),
            Group::Pass => {}
        }
        Some(group)
    }

    fn draw(&mut self, cmd: &DrawCmd, s: f32) {
        match cmd {
            DrawCmd::Rect {
                rect,
                radii,
                fill,
                stroke,
            } => {
                let r = snap_rect(rect, s);
                if r.is_empty() {
                    return;
                }
                let radii = fit_radii(radii, s, &r);
                if fill.a > 0.0 {
                    self.fill_rounded(&r, radii, *fill);
                }
                if let Some((color, width)) = stroke {
                    let w = snap_width(*width, s);
                    if w > 0.0 && color.a > 0.0 {
                        self.segs.clear();
                        let ring = border_ring(&r, radii, w, &mut self.segs);
                        let rule = if ring {
                            FillRule::EvenOdd
                        } else {
                            FillRule::NonZero
                        };
                        self.fill(rule, *color, radii.iter().any(|&r| r > 0.0));
                    }
                }
            }
            DrawCmd::Shadow {
                rect,
                radii,
                layers,
            } => {
                let r = snap_rect(rect, s);
                let radii = fit_radii(radii, s, &r);
                for layer in layers.iter().rev() {
                    let key = ShadowKey::new(&r, radii, layer, s);
                    let Some((image, x, y, w, h)) = self.shadow_texture(&key) else {
                        continue;
                    };
                    let (x0, y0) = (r.x0 + x as f32, r.y0 + y as f32);
                    let area = PxRect {
                        x0,
                        y0,
                        x1: x0 + w as f32,
                        y1: y0 + h as f32,
                    };
                    self.draw_mask(image, area, layer.color);
                }
            }
            DrawCmd::Text {
                rect,
                text,
                style,
                color,
                align,
            } => {
                let layout = self.text.layout(text, style, Some(rect.width), *align);
                let mut placed = std::mem::take(&mut self.placed);
                placed.clear();
                self.glyphs.place(&layout, (rect.x, rect.y), s, &mut placed);
                let mut per_page: Vec<Vec<Quad>> = Vec::new();
                let inv = 1.0 / ATLAS_SIZE as f32;
                for g in &placed {
                    let Some(slot) = self.atlas.slot(&mut self.canvas, g.key, &g.bitmap) else {
                        continue;
                    };
                    let b = &g.bitmap;
                    let (x0, y0) = ((g.pen_x + b.left) as f32, (g.pen_y - b.top) as f32);
                    if per_page.len() <= slot.page {
                        per_page.resize(slot.page + 1, Vec::new());
                    }
                    per_page[slot.page].push(Quad {
                        x0,
                        y0,
                        s0: slot.x as f32 * inv,
                        t0: slot.y as f32 * inv,
                        x1: x0 + b.width as f32,
                        y1: y0 + b.height as f32,
                        s1: (slot.x + b.width) as f32 * inv,
                        t1: (slot.y + b.height) as f32 * inv,
                    });
                }
                self.placed = placed;
                let alpha_glyphs = per_page
                    .into_iter()
                    .enumerate()
                    .filter(|(_, quads)| !quads.is_empty())
                    .map(|(page, quads)| DrawCommand {
                        image_id: self.atlas.pages[page].image,
                        quads,
                    })
                    .collect();
                let commands = GlyphDrawCommands {
                    alpha_glyphs,
                    color_glyphs: Vec::new(),
                };
                self.canvas
                    .draw_glyph_commands(commands, &Paint::color(fv_color(*color)));
            }
            DrawCmd::Line {
                from,
                to,
                color,
                width,
            } => {
                if let Some(r) = snap_line(*from, *to, *width, s) {
                    self.fill_rounded(&r, [0.0; 4], *color);
                } else {
                    self.segs.clear();
                    self.segs.push(Seg::Move(from.x * s, from.y * s));
                    self.segs.push(Seg::Line(to.x * s, to.y * s));
                    self.stroke(width * s, *color, false);
                }
            }
            DrawCmd::Path {
                commands,
                fill,
                stroke,
            } => {
                self.segs.clear();
                path_segs(commands, s, &mut self.segs);
                if let Some(fill) = fill {
                    self.fill(FillRule::NonZero, *fill, true);
                }
                if let Some((color, width)) = stroke {
                    self.stroke(width * s, *color, true);
                }
            }
            DrawCmd::PushClip { rect, radii } => {
                let r = snap_rect(rect, s);
                let radii = fit_radii(radii, s, &r);
                self.segs.clear();
                rounded_rect(&r, radii, &mut self.segs);
                self.canvas.save();
                self.canvas
                    .clip_path(&fv_path(&self.segs), FillRule::NonZero);
                self.groups.push(Group::Clip);
            }
            DrawCmd::PopClip => {
                while let Some(group) = self.pop_group() {
                    if matches!(group, Group::Clip) {
                        break;
                    }
                }
            }
            DrawCmd::PushOpacity(opacity) => {
                if *opacity >= 1.0 {
                    self.groups.push(Group::Pass);
                } else {
                    // A false return means femtovg could not capture the group and
                    // scales each draw's alpha instead; it still pairs with end_layer.
                    let _ = self
                        .canvas
                        .begin_layer(&LayerEffects::new().with_opacity(opacity.max(0.0)));
                    self.groups.push(Group::Layer);
                }
            }
            DrawCmd::PopOpacity => {
                while let Some(group) = self.pop_group() {
                    if !matches!(group, Group::Clip) {
                        break;
                    }
                }
            }
        }
    }

    /// Draws one frame into the current framebuffer.
    pub fn render_list(&mut self, list: &DisplayList, frame: &FrameInfo) {
        let (w, h) = (frame.width.max(1), frame.height.max(1));
        let scale = if frame.scale > 0.0 { frame.scale } else { 1.0 };
        self.frame += 1;
        let current = self.frame;
        let stale: Vec<ShadowKey> = self
            .shadow_textures
            .iter()
            .filter(|(_, (_, used))| current - used > SHADOW_TEXTURE_FRAMES)
            .map(|(key, _)| *key)
            .collect();
        for key in stale {
            if let Some((image, _)) = self.shadow_textures.remove(&key) {
                self.canvas.delete_image(image);
            }
        }
        self.shadows.begin_frame();
        self.atlas.begin_frame(&mut self.canvas);

        // Coordinates are physical pixels already, so the canvas runs at 1:1.
        self.canvas.set_size(w, h, 1.0);
        self.canvas.reset();
        self.canvas.clear_rect(0, 0, w, h, fv_color(list.clear));
        for cmd in &list.commands {
            self.draw(cmd, scale);
        }
        while self.pop_group().is_some() {}
        self.canvas.flush_to_output(());
    }

    /// Reads back the frame just drawn as straight-alpha RGBA rows, top row
    /// first. Call after [`Renderer::render`] and before swapping buffers.
    pub fn read_pixels(&mut self) -> Result<Vec<u8>, femtovg::ErrorKind> {
        let image = self.canvas.screenshot()?;
        let mut out = Vec::with_capacity(image.width() * image.height() * 4);
        for px in image.pixels() {
            let a = px.a as u32;
            // GL holds premultiplied color.
            let un = |c: u8| {
                if a == 0 || a == 255 {
                    c
                } else {
                    ((c as u32 * 255 + a / 2) / a).min(255) as u8
                }
            };
            out.extend_from_slice(&[un(px.r), un(px.g), un(px.b), px.a]);
        }
        Ok(out)
    }
}

impl Renderer for GlRenderer {
    fn measurer(&self) -> &dyn TextMeasure {
        &self.text
    }

    fn render(&mut self, list: &DisplayList, frame: &FrameInfo) {
        self.render_list(list, frame);
    }
}
