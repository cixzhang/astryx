//! Astryx CPU renderer. Draws a [`DisplayList`] into its own pixel buffer
//! with `tiny-skia`, deterministically, for headless tests, screenshots, and
//! screens without a GPU.
//!
//! Text, pixel snapping, and shadow masks come from [`astryx_raster`], the
//! same code the GPU renderer uses.

use astryx_core::{
    Color, DisplayList, DrawCmd, FrameInfo, ImageData, ImageId, Renderer, TextMeasure,
};
use std::collections::HashMap;
use std::rc::Rc;
use tiny_skia::{IntSize, Mask, PathBuilder, Pixmap};

pub use astryx_raster;
pub use tiny_skia;

use astryx_raster::geom::{fit_radii, rect_px, snap_rect, Affine, PxRect, Seg};
use astryx_raster::light::{light_map, LightMap, PlacedLight};
use astryx_raster::shadow::{ShadowCache, ShadowKey, ShadowMask};
use astryx_raster::text::{FontData, FontError, FontSet, GlyphDraw, TextEngine};
use astryx_raster::transition::{turn, Moment};

mod damage;
mod paint;
mod pool;

fn to_skia_path(segs: &[Seg]) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    for seg in segs {
        match *seg {
            Seg::Move(x, y) => pb.move_to(x, y),
            Seg::Line(x, y) => pb.line_to(x, y),
            Seg::Quad(a, b, x, y) => pb.quad_to(a, b, x, y),
            Seg::Cubic(a, b, c, d, x, y) => pb.cubic_to(a, b, c, d, x, y),
            Seg::Close => pb.close(),
        }
    }
    pb.finish()
}

/// A color as premultiplied 8-bit channels.
fn premultiplied(c: Color) -> [u8; 4] {
    let a = c.a.clamp(0.0, 1.0);
    let ch = |v: f32| (v.clamp(0.0, 1.0) * a * 255.0).round() as u8;
    [ch(c.r), ch(c.g), ch(c.b), (a * 255.0).round() as u8]
}

fn skia_color(c: Color) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba(
        c.r.clamp(0.0, 1.0),
        c.g.clamp(0.0, 1.0),
        c.b.clamp(0.0, 1.0),
        c.a.clamp(0.0, 1.0),
    )
    .unwrap_or(tiny_skia::Color::TRANSPARENT)
}

fn div255(v: u32) -> u32 {
    let v = v + 128;
    (v + (v >> 8)) >> 8
}

/// Source-over blends `color` into `pixels` (RGBA rows `dw` x `dh`)
/// through an 8-bit coverage bitmap whose top left lands at (`x`, `y`),
/// gated by the clip mask.
#[allow(clippy::too_many_arguments)]
fn blend_coverage(
    pixels: &mut [u8],
    dw: u32,
    dh: u32,
    clip: Option<&Mask>,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    coverage: &[u8],
    color: [u8; 4],
) {
    if color[3] == 0 {
        return;
    }
    let (dw, dh) = (dw as i32, dh as i32);
    let (x0, y0) = (x.max(0), y.max(0));
    let (x1, y1) = ((x + w as i32).min(dw), (y + h as i32).min(dh));
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    for py in y0..y1 {
        for px in x0..x1 {
            let mut c = coverage[((py - y) * w as i32 + (px - x)) as usize] as u32;
            if let Some(m) = clip {
                c = div255(c * m.data()[(py * dw + px) as usize] as u32);
            }
            if c == 0 {
                continue;
            }
            let src_a = div255(color[3] as u32 * c);
            let i = ((py * dw + px) * 4) as usize;
            for k in 0..4 {
                let s = div255(color[k] as u32 * c);
                let d = div255(pixels[i + k] as u32 * (255 - src_a));
                pixels[i + k] = (s + d).min(255) as u8;
            }
        }
    }
}

/// The CPU renderer. Owns its pixel buffer; read it with [`Self::pixmap`].
///
/// It redraws only what changed since the frame it drew last, and paints in
/// bands on several threads; every frame is the same, pixel for pixel, as
/// the same frame drawn whole by a new renderer.
pub struct SoftRenderer {
    text: TextEngine,
    shadows: ShadowCache,
    base: Pixmap,
    /// Registered images, premultiplied.
    images: HashMap<ImageId, Pixmap>,
    transition: Option<Running>,
    /// The frame on screen, to tell what the next one changes.
    last: Option<LastFrame>,
    /// An image changed since the last frame.
    images_changed: bool,
    /// How many pixels the last frame drew again.
    painted: u64,
    /// Threads that paint bands, started with the first frame that uses
    /// them.
    painters: Option<pool::Pool>,
}

/// A transition in progress: the old frame and the time so far.
struct Running {
    transition: astryx_core::Transition,
    elapsed: f32,
    old: Pixmap,
}

/// What the last frame drew, to compare the next with.
struct LastFrame {
    list: DisplayList,
    bounds: Vec<Option<PxRect>>,
    lighting: bool,
    size: (u32, u32),
    scale: f32,
    text_generation: u64,
    /// Where its bands met.
    seams: Vec<u32>,
}

/// A group open while preparing a frame. A pop closes groups up to the
/// nearest of its kind.
#[derive(Clone, Copy, PartialEq)]
enum Open {
    Clip,
    Opacity,
    Transform,
}

/// Text and shadows of a frame, from the caches only the renderer's thread
/// may use, ready for the bands to paint.
struct Prepared {
    /// Per command: a text command's glyphs, top-left in frame pixels.
    glyphs: Vec<Vec<(i32, i32, Rc<astryx_raster::text::GlyphBitmap>)>>,
    /// Per command: a shadow command's masks, in drawing order.
    shadows: Vec<Vec<Option<Rc<ShadowMask>>>>,
    /// Each lighting pass's light map, by the index of the command closing
    /// it (`usize::MAX`: the end of the list).
    light_maps: HashMap<usize, LightMap>,
}

impl Default for SoftRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl SoftRenderer {
    /// A renderer with the embedded Roboto faces and, on demand, the embedded
    /// Arabic and Hebrew faces; with no faces when the `embedded-fonts`
    /// feature is off.
    pub fn new() -> Self {
        #[cfg(feature = "embedded-fonts")]
        let fonts = astryx_fonts::roboto();
        #[cfg(not(feature = "embedded-fonts"))]
        let fonts = FontSet::new();
        let renderer = Self::with_fonts(fonts);
        #[cfg(feature = "embedded-fonts")]
        renderer
            .text
            .set_fallback_provider(Box::new(astryx_fonts::fallback));
        renderer
    }

    /// A renderer that draws text with `fonts`, in their fallback order.
    pub fn with_fonts(fonts: FontSet) -> Self {
        Self {
            text: TextEngine::new(fonts),
            shadows: ShadowCache::default(),
            base: Pixmap::new(1, 1).expect("1x1 pixmap"),
            images: HashMap::new(),
            transition: None,
            last: None,
            images_changed: false,
            painted: 0,
            painters: None,
        }
    }

    /// Adds or replaces an image; `None` frees it. Pixels that do not match
    /// the size are ignored.
    pub fn set_image(&mut self, id: ImageId, data: Option<&ImageData>) {
        self.images_changed = true;
        let pixmap = data.and_then(|d| {
            let size = IntSize::from_wh(d.width, d.height)?;
            if d.rgba.len() != (d.width as usize) * (d.height as usize) * 4 {
                return None;
            }
            let mut px = d.rgba.to_vec();
            for p in px.as_chunks_mut::<4>().0 {
                let a = p[3] as u32;
                for c in &mut p[..3] {
                    *c = div255(*c as u32 * a) as u8;
                }
            }
            Pixmap::from_vec(px, size)
        });
        match pixmap {
            Some(p) => {
                self.images.insert(id, p);
            }
            None => {
                self.images.remove(&id);
            }
        }
    }

    /// Adds a face after creation. Layout picks it up on the next frame.
    pub fn add_fallback_font(&mut self, data: impl Into<FontData>) -> Result<(), FontError> {
        self.text.add_fallback_font(data)
    }

    /// The text engine used for drawing; hand it to layout.
    pub fn text(&self) -> &TextEngine {
        &self.text
    }

    /// The last rendered frame, premultiplied RGBA.
    pub fn pixmap(&self) -> &Pixmap {
        &self.base
    }

    /// The last rendered frame as straight-alpha RGBA bytes, row by row.
    pub fn rgba(&self) -> Vec<u8> {
        self.base.clone().take_demultiplied()
    }

    /// The last rendered frame as a PNG file.
    pub fn encode_png(&self) -> Vec<u8> {
        self.base
            .encode_png()
            .expect("PNG encoding of an in-memory pixmap")
    }

    /// Lays out the frame's text and finds its shadow masks and light
    /// maps, following its transforms and pixel grids as painting does.
    fn prepare(
        &mut self,
        list: &DisplayList,
        bounds: &[Option<PxRect>],
        regions: Option<&[(u32, u32, u32, u32)]>,
        s: f32,
        (w, h): (u32, u32),
    ) -> Prepared {
        // Text and shadows only where something is drawn again.
        let wanted = |i: usize| match (regions, &bounds[i]) {
            (Some(rs), Some(b)) => rs.iter().any(|&(x0, y0, x1, y1)| {
                b.x0 < x1 as f32 && x0 as f32 <= b.x1 && b.y0 < y1 as f32 && y0 as f32 <= b.y1
            }),
            _ => true,
        };
        let n = list.commands.len();
        let mut out = Prepared {
            glyphs: vec![Vec::new(); n],
            shadows: vec![Vec::new(); n],
            light_maps: HashMap::new(),
        };
        // Transforms open as groups of their own kind, closed by the next
        // pop of their kind; pops of other kinds leave them open.
        let mut transforms: Vec<Affine> = Vec::new();
        let mut kinds: Vec<Open> = Vec::new();
        let mut t = Affine::IDENTITY;
        let mut grids: Vec<f32> = Vec::new();
        let mut pass: Option<(astryx_core::Lighting, Vec<PlacedLight>)> = None;
        let mut placed: Vec<GlyphDraw> = Vec::new();
        let pop_until =
            |kinds: &mut Vec<Open>, transforms: &mut Vec<Affine>, t: &mut Affine, kind: Open| {
                while let Some(k) = kinds.pop() {
                    if k == Open::Transform {
                        *t = transforms.pop().expect("transform group");
                    }
                    if k == kind {
                        break;
                    }
                }
            };
        for (i, cmd) in list.commands.iter().enumerate() {
            match cmd {
                DrawCmd::PushTransform {
                    origin,
                    scale,
                    translate,
                } => {
                    transforms.push(t);
                    kinds.push(Open::Transform);
                    t = t.then(*origin, *scale, *translate);
                }
                DrawCmd::PopTransform => {
                    pop_until(&mut kinds, &mut transforms, &mut t, Open::Transform)
                }
                DrawCmd::PushClip { .. } => kinds.push(Open::Clip),
                DrawCmd::PopClip => pop_until(&mut kinds, &mut transforms, &mut t, Open::Clip),
                DrawCmd::PushOpacity(_) => kinds.push(Open::Opacity),
                DrawCmd::PopOpacity => {
                    pop_until(&mut kinds, &mut transforms, &mut t, Open::Opacity)
                }
                DrawCmd::PushPixelGrid { cell } => grids.push(*cell),
                DrawCmd::PopPixelGrid => {
                    grids.pop();
                }
                DrawCmd::PushLighting(lighting) => {
                    if pass.is_none() {
                        pass = Some((*lighting, Vec::new()));
                    }
                }
                DrawCmd::Light(_) => {
                    if let Some((l, lights)) = &mut pass {
                        let moved = if t.is_identity() {
                            cmd.clone()
                        } else {
                            t.apply(cmd)
                        };
                        if let DrawCmd::Light(light) = &moved {
                            lights.push(PlacedLight::new(light, l.time, s, l.strength));
                        }
                    }
                }
                DrawCmd::PopLighting => {
                    if let Some((l, lights)) = pass.take() {
                        if l.strength > 0.0 {
                            out.light_maps.insert(i, light_map(&l, &lights, s, w, h));
                        }
                    }
                }
                DrawCmd::Text {
                    rect,
                    text,
                    style,
                    align,
                    ..
                } if wanted(i) => {
                    let moved;
                    let (rect, text, style, align, snap) = if t.is_identity() {
                        (rect, text, style, align, true)
                    } else {
                        moved = t.apply(cmd);
                        let DrawCmd::Text {
                            rect,
                            text,
                            style,
                            align,
                            ..
                        } = &moved
                        else {
                            unreachable!("a moved text is text")
                        };
                        (rect, text, style, align, t.snaps())
                    };
                    let layout = self.text.layout(text, style, Some(rect.width), *align);
                    placed.clear();
                    match grids.last() {
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
                    out.glyphs[i] = placed
                        .iter()
                        .map(|g| {
                            (
                                g.pen_x + g.bitmap.left,
                                g.pen_y - g.bitmap.top,
                                g.bitmap.clone(),
                            )
                        })
                        .collect();
                }
                DrawCmd::Shadow {
                    rect,
                    radii,
                    layers,
                } if wanted(i) => {
                    let scaled = !t.is_identity() && !t.snaps();
                    let (r, radii, layers, scale_by) = if t.is_identity() || scaled {
                        let r = if scaled {
                            snap_rect(rect, s)
                        } else {
                            rect_px(rect, s, true)
                        };
                        (r, fit_radii(radii, s, &r), layers.clone(), s)
                    } else {
                        let moved = t.apply(cmd);
                        let DrawCmd::Shadow {
                            rect,
                            radii,
                            layers,
                        } = moved
                        else {
                            unreachable!("a moved shadow is a shadow")
                        };
                        let r = rect_px(&rect, s, t.snaps());
                        (r, fit_radii(&radii, s, &r), layers, s)
                    };
                    out.shadows[i] = layers
                        .iter()
                        .rev()
                        .map(|layer| {
                            self.shadows
                                .get(&ShadowKey::new(&r, radii, layer, scale_by))
                        })
                        .collect();
                }
                _ => {}
            }
        }
        if let Some((l, lights)) = pass.take() {
            if l.strength > 0.0 {
                out.light_maps
                    .insert(usize::MAX, light_map(&l, &lights, s, w, h));
            }
        }
        out
    }

    /// Draws one frame into the buffer, resizing it to the frame. Only the
    /// pixels that can differ from the frame drawn last are drawn again;
    /// the result is the same as drawing the whole frame.
    pub fn render_list(&mut self, list: &DisplayList, frame: &FrameInfo) {
        let (w, h) = (frame.width.max(1), frame.height.max(1));
        let scale = if frame.scale > 0.0 { frame.scale } else { 1.0 };
        let walk = damage::walk(list, scale, &self.text);
        let text_generation = TextMeasure::generation(&self.text);
        let seams = seams(h, &walk.no_seam);
        let regions = match &self.last {
            Some(last)
                if last.size == (w, h)
                    && last.scale == scale
                    && last.list.clear == list.clear
                    && last.text_generation == text_generation
                    && !last.lighting
                    && !walk.lighting
                    && !self.images_changed
                    && self.transition.is_none() =>
            {
                let changed = damage::changed(
                    &last.list.commands,
                    &last.bounds,
                    &list.commands,
                    &walk.bounds,
                );
                let mut regions = damage::regions(&changed, w, h);
                // A band's pixels depend on where it starts and ends: bands
                // that moved paint again.
                let old = bands(&last.seams, h);
                regions.extend(
                    bands(&seams, h)
                        .filter(|b| !old.clone().any(|o| o == *b))
                        .map(|(y0, y1)| (0, y0, w, y1)),
                );
                let area: u64 = regions
                    .iter()
                    .map(|&(x0, y0, x1, y1)| u64::from(x1 - x0) * u64::from(y1 - y0))
                    .sum();
                // Most of the frame: drawing it whole costs less.
                (area * 2 < u64::from(w) * u64::from(h)).then_some(regions)
            }
            _ => None,
        };
        if self.base.width() != w || self.base.height() != h {
            self.base = Pixmap::new(w, h).expect("frame pixmap");
        }
        self.shadows.begin_frame();
        // Nothing changed: the frame on screen stays.
        if !regions.as_ref().is_some_and(|r| r.is_empty()) {
            let prepared = self.prepare(list, &walk.bounds, regions.as_deref(), scale, (w, h));
            let glyphs: Vec<Vec<(i32, i32, &astryx_raster::text::GlyphBitmap)>> = prepared
                .glyphs
                .iter()
                .map(|g| g.iter().map(|(x, y, b)| (*x, *y, &**b)).collect())
                .collect();
            let shadows: Vec<Vec<Option<&ShadowMask>>> = prepared
                .shadows
                .iter()
                .map(|v| v.iter().map(|m| m.as_deref()).collect())
                .collect();
            let shared = paint::Shared {
                cmds: &list.commands,
                bounds: &walk.bounds,
                glyphs: &glyphs,
                shadows: &shadows,
                light_maps: &prepared.light_maps,
                images: &self.images,
                scale,
                width: w,
            };
            let clear = skia_color(list.clear).premultiply().to_color_u8();
            let clear = [clear.red(), clear.green(), clear.blue(), clear.alpha()];
            paint_frame(
                &shared,
                self.base.data_mut(),
                clear,
                regions.as_deref(),
                &seams,
                self.painters
                    .get_or_insert_with(|| pool::Pool::new(painters() - 1)),
            );
        }
        self.painted = match &regions {
            None => u64::from(w) * u64::from(h),
            Some(rs) => rs
                .iter()
                .map(|&(x0, y0, x1, y1)| u64::from(x1 - x0) * u64::from(y1 - y0))
                .sum(),
        };
        if regions.is_none() {
            self.compose_transition(frame, scale);
        }
        self.images_changed = false;
        self.last = Some(LastFrame {
            list: list.clone(),
            bounds: walk.bounds,
            lighting: walk.lighting,
            size: (w, h),
            scale,
            text_generation,
            seams,
        });
    }

    /// Starts a transition from `from` to the frames rendered next.
    pub fn begin_transition(
        &mut self,
        transition: &astryx_core::Transition,
        from: &DisplayList,
        frame: &FrameInfo,
    ) {
        self.transition = None;
        if transition.duration_ms == 0 {
            return;
        }
        self.render_list(from, frame);
        self.transition = Some(Running {
            transition: *transition,
            elapsed: 0.0,
            old: self.base.clone(),
        });
    }

    /// A transition is running.
    pub fn transitioning(&self) -> bool {
        self.transition.is_some()
    }

    /// Blends the old frame into the one just rendered, by the transition's
    /// progress after this frame's time.
    fn compose_transition(&mut self, frame: &FrameInfo, scale: f32) {
        let Some(run) = &mut self.transition else {
            return;
        };
        run.elapsed += frame.dt.max(0.0);
        let t = run.elapsed * 1000.0 / run.transition.duration_ms as f32;
        if t >= 1.0
            || run.old.width() != self.base.width()
            || run.old.height() != self.base.height()
        {
            self.transition = None;
            return;
        }
        let (w, h) = (self.base.width(), self.base.height());
        let moment = Moment::new(
            run.transition.kind,
            run.transition.easing,
            t,
            w as f32,
            h as f32,
            scale,
        );
        let lattice = astryx_raster::light::lattice();
        let new = self.base.data().to_vec();
        let old = run.old.data();
        let center = (w as f32 / 2.0, h as f32 / 2.0);
        let read = |data: &[u8], x: f32, y: f32| -> [f32; 4] {
            // Outside the frame, the nearest edge pixel, as the GPU clamps.
            let xi = (x.floor() as i64).clamp(0, w as i64 - 1) as u32;
            let yi = (y.floor() as i64).clamp(0, h as i64 - 1) as u32;
            let at = ((yi * w + xi) * 4) as usize;
            [0, 1, 2, 3].map(|c| f32::from(data[at + c]))
        };
        let out = self.base.data_mut();
        for y in 0..h {
            for x in 0..w {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let m = moment.at(px, py, &lattice);
                let (ox, oy) = turn(px, py, center, m.old_turn);
                let a = read(old, ox, oy);
                let b = if m.new > 0.0 {
                    read(&new, px + m.new_offset.0, py + m.new_offset.1)
                } else {
                    [0.0; 4]
                };
                let at = ((y * w + x) * 4) as usize;
                for c in 0..4 {
                    let mut v = a[c] + (b[c] - a[c]) * m.new;
                    v += (255.0 - v) * m.white;
                    let black = if c == 3 { 255.0 } else { 0.0 };
                    v += (black - v) * m.black;
                    out[at + c] = v.round().clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}

impl Renderer for SoftRenderer {
    fn measurer(&self) -> &dyn TextMeasure {
        &self.text
    }

    fn render(&mut self, list: &DisplayList, frame: &FrameInfo) {
        self.render_list(list, frame);
    }

    fn set_image(&mut self, id: ImageId, data: Option<&ImageData>) {
        SoftRenderer::set_image(self, id, data);
    }

    fn begin_transition(
        &mut self,
        transition: &astryx_core::Transition,
        from: &DisplayList,
        frame: &FrameInfo,
    ) {
        SoftRenderer::begin_transition(self, transition, from, frame);
    }

    fn transitioning(&self) -> bool {
        SoftRenderer::transitioning(self)
    }
}

/// The family a pixel theme's text is set in, which draws on the grid.
fn own_family(style: &astryx_core::TextStyle) -> Option<&'static str> {
    match style.family {
        astryx_core::FontFamily::Theme(font) => Some(font.family),
        _ => None,
    }
}

/// How many threads paint a frame.
fn painters() -> usize {
    if cfg!(target_arch = "wasm32") {
        1
    } else {
        std::thread::available_parallelism()
            .map_or(1, |n| n.get())
            .min(8)
    }
}

/// About how tall a band is, in pixels. Fixed, not taken from the number
/// of threads, so a frame comes out the same on every machine.
const BAND: u32 = 128;

/// The bands, (first row, end row), that `seams` cut `h` rows into.
fn bands(seams: &[u32], h: u32) -> impl Iterator<Item = (u32, u32)> + Clone + '_ {
    let starts = std::iter::once(0).chain(seams.iter().copied());
    let ends = seams.iter().copied().chain(std::iter::once(h));
    starts.zip(ends)
}

/// Where bands meet: rows, increasing, that split a frame `h` rows tall
/// into bands about [`BAND`] rows tall, none of them inside `no_seam`.
fn seams(h: u32, no_seam: &[(f32, f32)]) -> Vec<u32> {
    let count = h.div_ceil(BAND) as usize;
    if count <= 1 {
        return Vec::new();
    }
    // Rows r where a seam would cut a pixel row a shape crosses: from
    // floor(a) + 1 to ceil(b) - 1 for each interval.
    let mut off: Vec<(i64, i64)> = no_seam
        .iter()
        .map(|&(a, b)| (a.floor() as i64 + 1, b.ceil() as i64 - 1))
        .filter(|(a, b)| a <= b)
        .collect();
    off.sort_unstable();
    let mut merged: Vec<(i64, i64)> = Vec::new();
    for (a, b) in off {
        match merged.last_mut() {
            Some(last) if a <= last.1 + 1 => last.1 = last.1.max(b),
            _ => merged.push((a, b)),
        }
    }
    let free = |r: i64| {
        let i = merged.partition_point(|&(a, _)| a <= r);
        i == 0 || merged[i - 1].1 < r
    };
    let step = h as f32 / count as f32;
    let mut out: Vec<u32> = Vec::new();
    for k in 1..count {
        let target = (k as f32 * step).round() as i64;
        let reach = (step / 2.0) as i64;
        let found = (0..=reach).find_map(|d| {
            [target - d, target + d]
                .into_iter()
                .find(|&r| r > 0 && r < h as i64 && free(r))
        });
        if let Some(r) = found {
            let r = r as u32;
            if out.last().is_none_or(|&last| r > last) {
                out.push(r);
            }
        }
    }
    out
}

/// Paints `shared`'s frame into `data`, band by band on the pool's threads:
/// every band, or with `regions` only the bands they touch, and in those
/// only the regions' pixels. Bands meet at `seams`.
fn paint_frame(
    shared: &paint::Shared,
    data: &mut [u8],
    clear: [u8; 4],
    regions: Option<&[(u32, u32, u32, u32)]>,
    seams: &[u32],
    pool: &pool::Pool,
) {
    let w = shared.width;
    let row_bytes = w as usize * 4;
    let regions: Option<Vec<PxRect>> = regions.map(|rs| {
        rs.iter()
            .map(|&(x0, y0, x1, y1)| PxRect {
                x0: x0 as f32,
                y0: y0 as f32,
                x1: x1 as f32,
                y1: y1 as f32,
            })
            .collect()
    });
    let mut jobs: Vec<(u32, &mut [u8])> = Vec::new();
    let mut rest = data;
    let mut y0 = 0u32;
    let total = (rest.len() / row_bytes) as u32;
    for end in seams.iter().copied().chain([total]) {
        let (chunk, tail) = rest.split_at_mut((end - y0) as usize * row_bytes);
        rest = tail;
        let band = PxRect {
            x0: 0.0,
            y0: y0 as f32,
            x1: w as f32,
            y1: end as f32,
        };
        if regions
            .as_ref()
            .is_none_or(|rs| rs.iter().any(|r| paint::overlaps(r, &band)))
        {
            jobs.push((y0, chunk));
        }
        y0 = end;
    }
    let paint_job = |(y0, chunk): (u32, &mut [u8])| {
        let bh = (chunk.len() / (w as usize * 4)) as u32;
        let band = PxRect {
            x0: 0.0,
            y0: y0 as f32,
            x1: w as f32,
            y1: (y0 + bh) as f32,
        };
        match &regions {
            None => {
                for px in chunk.as_chunks_mut::<4>().0 {
                    *px = clear;
                }
                paint::paint_band(shared, chunk, y0, bh, &[band]);
            }
            Some(rs) => {
                // The band painted whole into a buffer of its own, from
                // which only the regions' pixels are kept: painting goes
                // past the regions.
                let cull: Vec<PxRect> = rs
                    .iter()
                    .filter(|r| paint::overlaps(r, &band))
                    .map(|r| PxRect {
                        x0: r.x0,
                        y0: r.y0.max(band.y0),
                        x1: r.x1,
                        y1: r.y1.min(band.y1),
                    })
                    .collect();
                let mut own = vec![0u8; chunk.len()];
                let row = w as usize * 4;
                for r in &cull {
                    for y in (r.y0 as u32 - y0) as usize..(r.y1 as u32 - y0) as usize {
                        for px in own[y * row + r.x0 as usize * 4..y * row + r.x1 as usize * 4]
                            .as_chunks_mut::<4>()
                            .0
                        {
                            *px = clear;
                        }
                    }
                }
                paint::paint_band(shared, &mut own, y0, bh, &cull);
                for r in &cull {
                    let (a, b) = (r.x0 as usize * 4, r.x1 as usize * 4);
                    for y in (r.y0 as u32 - y0) as usize..(r.y1 as u32 - y0) as usize {
                        chunk[y * row + a..y * row + b]
                            .copy_from_slice(&own[y * row + a..y * row + b]);
                    }
                }
            }
        }
    };
    if jobs.len() <= 1 {
        jobs.into_iter().for_each(paint_job);
        return;
    }
    // Each band once, to whichever thread takes it.
    let jobs: Vec<_> = jobs
        .into_iter()
        .map(|j| std::sync::Mutex::new(Some(j)))
        .collect();
    pool.run(jobs.len(), &|i| {
        if let Some(job) = jobs[i].lock().expect("band").take() {
            paint_job(job);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use astryx_core::{ImageFit, Radii, Rect};

    fn frame(w: u32, h: u32, scale: f32) -> FrameInfo {
        FrameInfo {
            width: w,
            height: h,
            scale,
            dt: 0.0,
        }
    }

    /// A page of rows like a settings page: a card behind them, a label and a
    /// switch in each.
    fn rows_page(on: usize) -> DisplayList {
        let mut commands = vec![
            DrawCmd::Shadow {
                rect: Rect::new(16.0, 16.0, 688.0, 868.0),
                radii: Radii::all(12.0),
                layers: vec![astryx_core::ShadowLayer {
                    x: 0.0,
                    y: 2.0,
                    blur: 8.0,
                    spread: 0.0,
                    inset: false,
                    color: Color::rgba(0.0, 0.0, 0.0, 0.2),
                }],
            },
            DrawCmd::Rect {
                rect: Rect::new(16.0, 16.0, 688.0, 868.0),
                radii: Radii::all(12.0),
                fill: Color::rgba(1.0, 1.0, 1.0, 1.0),
                stroke: Some((Color::rgba(0.8, 0.8, 0.85, 1.0), 1.0)),
            },
            DrawCmd::PushClip {
                rect: Rect::new(16.0, 16.0, 688.0, 868.0),
                radii: Radii::all(12.0),
            },
        ];
        for i in 0..20 {
            let y = 32.0 + i as f32 * 42.0;
            commands.push(DrawCmd::Text {
                rect: Rect::new(40.0, y + 8.0, 400.0, 22.0),
                text: format!("Setting number {i}"),
                style: astryx_core::TextStyle {
                    direction: None,
                    family: astryx_core::FontFamily::Body,
                    size: 15.0,
                    weight: 400,
                    leading: 1.4,
                },
                color: Color::rgba(0.1, 0.1, 0.12, 1.0),
                align: astryx_core::TextAlign::Start,
            });
            commands.push(DrawCmd::Rect {
                rect: Rect::new(620.0, y + 6.0, 52.0, 30.0),
                radii: Radii::all(15.0),
                fill: if i == on {
                    Color::rgba(0.2, 0.45, 0.95, 1.0)
                } else {
                    Color::rgba(0.75, 0.75, 0.78, 1.0)
                },
                stroke: None,
            });
            commands.push(DrawCmd::Rect {
                rect: Rect::new(if i == on { 644.0 } else { 624.0 }, y + 10.0, 24.0, 22.0),
                radii: Radii::all(11.0),
                fill: Color::rgba(1.0, 1.0, 1.0, 1.0),
                stroke: None,
            });
        }
        commands.push(DrawCmd::PopClip);
        DisplayList {
            clear: Color::rgba(0.95, 0.95, 0.96, 1.0),
            commands,
        }
    }

    #[test]
    fn a_frame_draws_again_only_what_changed() {
        let frame = frame(1440, 1800, 2.0);
        let mut r = SoftRenderer::new();
        r.render_list(&rows_page(3), &frame);
        assert_eq!(r.painted, 1440 * 1800, "the first frame draws whole");
        r.render_list(&rows_page(3), &frame);
        assert_eq!(r.painted, 0, "an unchanged frame draws nothing");
        // Two switches change: the one turned off and the one turned on.
        r.render_list(&rows_page(4), &frame);
        assert!(
            r.painted < 1440 * 1800 / 10,
            "a toggle draws a tenth of the frame at most, drew {} pixels",
            r.painted
        );
        let mut whole = SoftRenderer::new();
        whole.render_list(&rows_page(4), &frame);
        assert!(r.pixmap().data() == whole.pixmap().data());
    }

    #[test]
    fn dropping_the_renderer_ends_its_painting_threads() {
        let mut renderer = SoftRenderer::new();
        renderer.render_list(&rows_page(0), &frame(1440, 1800, 2.0));
        let watch = renderer
            .painters
            .as_ref()
            .expect("painting threads")
            .watch();
        drop(renderer);
        // Each thread holds the pool's shared state until it has ended.
        assert!(
            watch.upgrade().is_none(),
            "a painting thread outlived the renderer"
        );
    }

    fn px(r: &SoftRenderer, x: u32, y: u32) -> [u8; 4] {
        let p = r.pixmap().pixel(x, y).unwrap();
        [p.red(), p.green(), p.blue(), p.alpha()]
    }

    const RED: Color = Color::rgba(1.0, 0.0, 0.0, 1.0);
    const WHITE: Color = Color::rgba(1.0, 1.0, 1.0, 1.0);

    #[test]
    fn rects_snap_to_pixels_at_any_scale() {
        let mut r = SoftRenderer::new();
        let list = DisplayList {
            clear: WHITE,
            commands: vec![DrawCmd::Rect {
                rect: Rect::new(2.0, 2.0, 4.0, 4.0),
                radii: Radii::all(0.0),
                fill: RED,
                stroke: None,
            }],
        };
        r.render(&list, &frame(20, 20, 1.5));
        assert_eq!(px(&r, 3, 3), [255, 0, 0, 255]);
        assert_eq!(px(&r, 8, 8), [255, 0, 0, 255]);
        assert_eq!(px(&r, 9, 9), [255, 255, 255, 255]);
    }

    #[test]
    fn border_is_drawn_inside_the_box() {
        let mut r = SoftRenderer::new();
        let list = DisplayList {
            clear: WHITE,
            commands: vec![DrawCmd::Rect {
                rect: Rect::new(2.0, 2.0, 10.0, 10.0),
                radii: Radii::all(0.0),
                fill: Color::TRANSPARENT,
                stroke: Some((RED, 1.0)),
            }],
        };
        r.render(&list, &frame(16, 16, 1.0));
        assert_eq!(px(&r, 2, 5), [255, 0, 0, 255]);
        assert_eq!(px(&r, 1, 5), [255, 255, 255, 255]);
        assert_eq!(px(&r, 3, 5), [255, 255, 255, 255]);
        assert_eq!(px(&r, 11, 5), [255, 0, 0, 255]);
    }

    #[test]
    fn group_opacity_does_not_double_overlaps() {
        let mut r = SoftRenderer::new();
        let rect = |x| DrawCmd::Rect {
            rect: Rect::new(x, 0.0, 10.0, 10.0),
            radii: Radii::all(0.0),
            fill: RED,
            stroke: None,
        };
        let list = DisplayList {
            clear: WHITE,
            commands: vec![
                DrawCmd::PushOpacity(0.5),
                rect(0.0),
                rect(5.0),
                DrawCmd::PopOpacity,
            ],
        };
        r.render(&list, &frame(20, 10, 1.0));
        assert_eq!(px(&r, 2, 5), px(&r, 7, 5));
        let [_, g, _, _] = px(&r, 7, 5);
        assert!((126..=129).contains(&g), "{g}");
    }

    #[test]
    fn rounded_clip_cuts_corners() {
        let mut r = SoftRenderer::new();
        let list = DisplayList {
            clear: WHITE,
            commands: vec![
                DrawCmd::PushClip {
                    rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                    radii: Radii::all(10.0),
                },
                DrawCmd::Rect {
                    rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                    radii: Radii::all(0.0),
                    fill: RED,
                    stroke: None,
                },
                DrawCmd::PopClip,
                DrawCmd::Rect {
                    rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                    radii: Radii::all(0.0),
                    fill: RED,
                    stroke: None,
                },
            ],
        };
        r.render(&list, &frame(20, 20, 1.0));
        assert_eq!(px(&r, 10, 10), [255, 0, 0, 255]);
        assert_eq!(px(&r, 1, 1), [255, 255, 255, 255]);
        assert_eq!(px(&r, 0, 0), [255, 0, 0, 255]);
    }

    #[test]
    fn unbalanced_groups_are_closed() {
        let mut r = SoftRenderer::new();
        let list = DisplayList {
            clear: WHITE,
            commands: vec![
                DrawCmd::PushOpacity(0.5),
                DrawCmd::PushClip {
                    rect: Rect::new(0.0, 0.0, 5.0, 5.0),
                    radii: Radii::all(0.0),
                },
                DrawCmd::Rect {
                    rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                    radii: Radii::all(0.0),
                    fill: RED,
                    stroke: None,
                },
                DrawCmd::PopOpacity,
                DrawCmd::PopClip,
            ],
        };
        r.render(&list, &frame(10, 10, 1.0));
        assert_eq!(px(&r, 8, 8), [255, 255, 255, 255]);
        assert_ne!(px(&r, 2, 2), [255, 255, 255, 255]);
    }

    #[cfg(feature = "embedded-fonts")]
    #[test]
    fn text_appears_on_the_next_frame_after_a_face_arrives() {
        let list = DisplayList {
            clear: WHITE,
            commands: vec![DrawCmd::Text {
                rect: Rect::new(0.0, 0.0, 100.0, 20.0),
                text: "Late".into(),
                style: astryx_core::TextStyle {
                    direction: None,
                    family: astryx_core::FontFamily::Body,
                    size: 14.0,
                    weight: 400,
                    leading: 1.4,
                },
                color: RED,
                align: astryx_core::TextAlign::Start,
            }],
        };
        let inked = |r: &SoftRenderer| r.pixmap().pixels().iter().any(|p| p.green() < 128);
        let mut r = SoftRenderer::with_fonts(FontSet::new());
        r.render(&list, &frame(100, 20, 1.0));
        assert!(!inked(&r));
        r.add_fallback_font(astryx_fonts::ROBOTO_REGULAR).unwrap();
        r.render(&list, &frame(100, 20, 1.0));
        assert!(inked(&r));
    }

    #[test]
    fn rendering_is_deterministic() {
        let list = DisplayList {
            clear: WHITE,
            commands: vec![DrawCmd::Text {
                rect: Rect::new(4.0, 4.0, 120.0, 40.0),
                text: "Deterministic text".into(),
                style: astryx_core::TextStyle {
                    direction: None,
                    family: astryx_core::FontFamily::Body,
                    size: 14.0,
                    weight: 400,
                    leading: 1.4,
                },
                color: RED,
                align: astryx_core::TextAlign::Start,
            }],
        };
        let mut a = SoftRenderer::new();
        let mut b = SoftRenderer::new();
        a.render(&list, &frame(128, 48, 2.0));
        b.render(&list, &frame(128, 48, 2.0));
        assert_eq!(a.pixmap().data(), b.pixmap().data());
        assert!(a.pixmap().pixels().iter().any(|p| p.green() < 50));
    }

    /// An 8x4 image: red on the left half, blue on the right.
    fn red_blue() -> ImageData {
        let mut rgba = Vec::new();
        for _ in 0..4 {
            for x in 0..8 {
                rgba.extend_from_slice(if x < 4 {
                    &[255, 0, 0, 255]
                } else {
                    &[0, 0, 255, 255]
                });
            }
        }
        ImageData {
            width: 8,
            height: 4,
            rgba: std::sync::Arc::from(rgba),
        }
    }

    fn image_frame(fit: ImageFit, radii: Radii) -> SoftRenderer {
        let mut r = SoftRenderer::new();
        r.set_image(ImageId(7), Some(&red_blue()));
        let list = DisplayList {
            clear: WHITE,
            commands: vec![DrawCmd::Image {
                rect: Rect::new(10.0, 10.0, 40.0, 40.0),
                radii,
                image: ImageId(7),
                fit,
            }],
        };
        r.render_list(&list, &frame(60, 60, 1.0));
        r
    }

    #[test]
    fn cover_fills_the_box_and_crops() {
        let r = image_frame(ImageFit::Cover, Radii::default());
        // The middle of a 2x1 image fills a square: red left of center, blue right.
        assert_eq!(px(&r, 12, 30), [255, 0, 0, 255]);
        assert_eq!(px(&r, 48, 30), [0, 0, 255, 255]);
        assert_eq!(px(&r, 30, 11)[3], 255);
        assert_eq!(
            px(&r, 5, 30),
            [255, 255, 255, 255],
            "nothing outside the box"
        );
    }

    #[test]
    fn contain_letterboxes() {
        let r = image_frame(ImageFit::Contain, Radii::default());
        // A 40x20 band through the middle; above and below stay clear.
        assert_eq!(px(&r, 12, 30), [255, 0, 0, 255]);
        assert_eq!(px(&r, 30, 15), [255, 255, 255, 255]);
        assert_eq!(px(&r, 30, 45), [255, 255, 255, 255]);
    }

    #[test]
    fn radii_clip_the_corners_and_missing_images_draw_nothing() {
        let r = image_frame(ImageFit::Fill, Radii::all(20.0));
        assert_eq!(px(&r, 11, 11), [255, 255, 255, 255]);
        assert_eq!(px(&r, 12, 30), [255, 0, 0, 255]);
        let mut r = image_frame(ImageFit::Fill, Radii::default());
        r.set_image(ImageId(7), None);
        let list = DisplayList {
            clear: WHITE,
            commands: vec![DrawCmd::Image {
                rect: Rect::new(10.0, 10.0, 40.0, 40.0),
                radii: Radii::default(),
                image: ImageId(7),
                fit: ImageFit::Fill,
            }],
        };
        r.render_list(&list, &frame(60, 60, 1.0));
        assert_eq!(px(&r, 30, 30), [255, 255, 255, 255]);
    }

    #[test]
    fn straight_alpha_is_premultiplied_on_upload() {
        let mut r = SoftRenderer::new();
        let half = ImageData {
            width: 1,
            height: 1,
            rgba: std::sync::Arc::from(vec![255, 0, 0, 128]),
        };
        r.set_image(ImageId(1), Some(&half));
        let list = DisplayList {
            clear: WHITE,
            commands: vec![DrawCmd::Image {
                rect: Rect::new(0.0, 0.0, 4.0, 4.0),
                radii: Radii::default(),
                image: ImageId(1),
                fit: ImageFit::Fill,
            }],
        };
        r.render_list(&list, &frame(4, 4, 1.0));
        let [red, green, _, _] = px(&r, 2, 2);
        assert_eq!(red, 255);
        assert!((green as i32 - 127).abs() <= 1, "{green}");
    }
}
