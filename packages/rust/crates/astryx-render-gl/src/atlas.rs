//! 8-bit coverage masks (glyphs, shadows, icon paths) packed into shared
//! textures, so most draws need no texture switch.

use std::collections::HashMap;

use astryx_raster::path::PathKey;
use astryx_raster::shadow::ShadowKey;
use astryx_raster::text::GlyphKey;
use glow::HasContext;

use crate::shader::Dialect;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MaskKey {
    Glyph(GlyphKey),
    Shadow(ShadowKey),
    Path(PathKey),
}

/// Where a mask lives: its texture and its texture coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    pub texture: glow::Texture,
    pub uv: [f32; 4],
}

/// Side of a page filled once ahead of use, for example with a theme's
/// glyphs. Large, so text draws from few textures in few calls.
const PRELOAD_PAGE: i32 = 2048;
/// Side of a page that takes masks as frames need them. Small, because
/// tiled GPUs convert the whole texture when part of it changes.
const PAGE: i32 = 512;
/// Side of a per-frame page.
const TRANSIENT_PAGE: i32 = 512;
/// Masks larger than this on either side get their own texture.
const LARGEST_PACKED: u32 = 256;
/// Pixels of lasting pages kept across frames; more may be added within
/// one frame.
const PAGE_BUDGET: i64 = 8 * (PRELOAD_PAGE as i64 * PRELOAD_PAGE as i64);
const LARGE: usize = 64;
const TRANSIENT_PAGES: usize = 3;

struct Page {
    size: i32,
    texture: glow::Texture,
    x: i32,
    y: i32,
    row: i32,
    /// A CPU copy of the page. New masks are written here and sent to the
    /// texture once per frame as one band of rows: on tiled GPUs every
    /// write to a texture the GPU may still read costs a copy or a wait,
    /// so one write per page per frame beats one per mask.
    pixels: Vec<u8>,
    /// Rows changed since the last upload: first and one past the last.
    dirty: Option<(i32, i32)>,
    /// The last frame that drew from this page; 0 when none has.
    last_used: u64,
    /// Filled and uploaded for good: no CPU copy, no more masks.
    sealed: bool,
}

impl Page {
    /// A blank page and its texture.
    ///
    /// # Safety
    /// `gl` must be current.
    unsafe fn new(gl: &glow::Context, dialect: Dialect, size: i32) -> Page {
        let pixels = vec![0u8; (size * size) as usize];
        let texture = mask_texture(gl, dialect, size, size, Some(&pixels));
        Page {
            size,
            texture,
            x: 0,
            y: 0,
            row: 0,
            pixels,
            dirty: None,
            last_used: 0,
            sealed: false,
        }
    }

    /// Writes a mask into the CPU copy and returns its region.
    fn write(&mut self, x: i32, y: i32, w: u32, h: u32, data: &[u8]) -> Region {
        for row in 0..h as usize {
            let at = (y as usize + row) * self.size as usize + x as usize;
            self.pixels[at..at + w as usize]
                .copy_from_slice(&data[row * w as usize..(row + 1) * w as usize]);
        }
        let (y0, y1) = (y, y + h as i32);
        self.dirty = Some(self.dirty.map_or((y0, y1), |(a, b)| (a.min(y0), b.max(y1))));
        let s = 1.0 / self.size as f32;
        Region {
            texture: self.texture,
            uv: [
                x as f32 * s,
                y as f32 * s,
                (x + w as i32) as f32 * s,
                (y + h as i32) as f32 * s,
            ],
        }
    }

    /// Sends the changed rows to the texture.
    fn flush(&mut self, gl: &glow::Context, dialect: Dialect) {
        let Some((y0, y1)) = self.dirty.take() else {
            return;
        };
        let (_, format) = mask_format(dialect);
        let rows = &self.pixels[(y0 * self.size) as usize..(y1 * self.size) as usize];
        // SAFETY: the context is current while rendering; the rows match the band.
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                y0,
                self.size,
                y1 - y0,
                format,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(rows)),
            );
        }
    }
}

#[derive(Default)]
pub struct Atlas {
    pages: Vec<Page>,
    /// Pages refilled every frame, for masks that are unlikely to repeat.
    /// They take turns, so the page written this frame is not one the GPU
    /// may still read from a frame in flight; a write to such a texture
    /// makes tiled GPUs copy or wait.
    transients: Vec<Page>,
    current: usize,
    frame: u64,
    /// Time spent uploading since the last frame started.
    pub upload_time: std::time::Duration,
    /// Masks uploaded since the last frame started.
    pub uploads: usize,
    /// Of those: glyphs, shadows, paths.
    pub upload_kinds: [usize; 3],
    /// Textures created since the last frame started.
    pub textures: usize,
    large: Vec<glow::Texture>,
    regions: HashMap<MaskKey, Region>,
}

/// Creates an 8-bit single-channel texture.
///
/// # Safety
/// `gl` must be current.
pub unsafe fn mask_texture(
    gl: &glow::Context,
    dialect: Dialect,
    w: i32,
    h: i32,
    data: Option<&[u8]>,
) -> glow::Texture {
    let texture = gl.create_texture().expect("texture");
    gl.bind_texture(glow::TEXTURE_2D, Some(texture));
    for (k, v) in [
        (glow::TEXTURE_MIN_FILTER, glow::NEAREST),
        (glow::TEXTURE_MAG_FILTER, glow::NEAREST),
        (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
        (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
    ] {
        gl.tex_parameter_i32(glow::TEXTURE_2D, k, v as i32);
    }
    let (internal, format) = mask_format(dialect);
    gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
    gl.tex_image_2d(
        glow::TEXTURE_2D,
        0,
        internal as i32,
        w,
        h,
        0,
        format,
        glow::UNSIGNED_BYTE,
        glow::PixelUnpackData::Slice(data),
    );
    texture
}

pub fn mask_format(dialect: Dialect) -> (u32, u32) {
    match dialect {
        Dialect::Es2 => (glow::LUMINANCE, glow::LUMINANCE),
        _ => (glow::R8, glow::RED),
    }
}

impl Atlas {
    /// Drops every mask when the atlas grew past its budget last frame.
    /// Returns true when it did.
    pub fn begin_frame(&mut self, gl: &glow::Context) -> bool {
        self.frame += 1;
        if !self.transients.is_empty() {
            self.current = (self.current + 1) % self.transients.len();
            let t = &mut self.transients[self.current];
            (t.x, t.y, t.row) = (0, 0, 0);
        }
        self.upload_time = std::time::Duration::ZERO;
        self.uploads = 0;
        self.upload_kinds = [0; 3];
        self.textures = 0;
        let area: i64 = self
            .pages
            .iter()
            .map(|p| p.size as i64 * p.size as i64)
            .sum();
        let over = area > PAGE_BUDGET || self.large.len() > LARGE;
        if over {
            self.clear(gl);
        }
        over
    }

    pub fn clear(&mut self, gl: &glow::Context) {
        // SAFETY: the textures belong to this context.
        unsafe {
            for page in self.pages.drain(..) {
                gl.delete_texture(page.texture);
            }
            for t in self.transients.drain(..) {
                gl.delete_texture(t.texture);
            }
            for texture in self.large.drain(..) {
                gl.delete_texture(texture);
            }
        }
        self.regions.clear();
    }

    /// Creates the first lasting page and the per-frame page ahead of use,
    /// so the first frames do not pay for them.
    pub fn prepare(&mut self, gl: &glow::Context, dialect: Dialect) {
        // Two pages, so the first frames that add masks find one no frame in
        // flight draws from without creating a texture.
        while self.pages.len() < 2 {
            // SAFETY: the context is current while the renderer is created.
            self.pages.push(unsafe { Page::new(gl, dialect, PAGE) });
        }
        while self.transients.len() < TRANSIENT_PAGES {
            // SAFETY: as above.
            self.transients
                .push(unsafe { Page::new(gl, dialect, TRANSIENT_PAGE) });
        }
    }

    /// Sends every mask written since the last call to its texture, one
    /// band of rows per page. Call before drawing.
    pub fn flush(&mut self, gl: &glow::Context, dialect: Dialect) {
        let t = crate::now();
        for page in self.pages.iter_mut().chain(self.transients.iter_mut()) {
            page.flush(gl, dialect);
        }
        self.upload_time += crate::since(t);
    }

    /// Uploads the preload pages and frees their CPU copies; they take no
    /// more masks.
    pub fn seal_preloaded(&mut self, gl: &glow::Context, dialect: Dialect) {
        self.flush(gl, dialect);
        for page in &mut self.pages {
            if page.size == PRELOAD_PAGE && !page.sealed {
                page.sealed = true;
                page.pixels = Vec::new();
            }
        }
    }

    /// True for the textures of the per-frame pages.
    pub fn is_transient(&self, texture: glow::Texture) -> bool {
        self.transients.iter().any(|p| p.texture == texture)
    }

    /// Records that the current frame draws from `texture`.
    pub fn mark_used(&mut self, texture: glow::Texture) {
        let frame = self.frame;
        if let Some(p) = self.pages.iter_mut().find(|p| p.texture == texture) {
            p.last_used = frame;
        }
    }

    /// The textures of every page, lasting and per-frame.
    pub fn page_textures(&self) -> Vec<glow::Texture> {
        self.pages
            .iter()
            .chain(&self.transients)
            .map(|p| p.texture)
            .collect()
    }

    /// Lasting pages in use.
    pub fn pages(&self) -> usize {
        self.pages.len()
    }

    pub fn get(&self, key: &MaskKey) -> Option<Region> {
        self.regions.get(key).copied()
    }

    /// Uploads a mask for this frame only. Returns `None` when it does not
    /// fit in the frame's page.
    pub fn insert_transient(
        &mut self,
        gl: &glow::Context,
        dialect: Dialect,
        w: u32,
        h: u32,
        data: &[u8],
    ) -> Option<Region> {
        if w > LARGEST_PACKED || h > LARGEST_PACKED {
            return None;
        }
        if self.transients.is_empty() {
            self.textures += TRANSIENT_PAGES;
            self.prepare(gl, dialect);
        }
        let page = &mut self.transients[self.current];
        let (x, y) = place(page, w, h)?;
        self.uploads += 1;
        Some(page.write(x, y, w, h, data))
    }

    /// Uploads a mask and remembers where it went.
    pub fn insert(
        &mut self,
        gl: &glow::Context,
        dialect: Dialect,
        key: MaskKey,
        w: u32,
        h: u32,
        data: &[u8],
    ) -> Region {
        self.insert_in(gl, dialect, key, w, h, data, PAGE)
    }

    /// Like [`Self::insert`], into large pages meant to be filled at once
    /// before use.
    pub fn preload(
        &mut self,
        gl: &glow::Context,
        dialect: Dialect,
        key: MaskKey,
        w: u32,
        h: u32,
        data: &[u8],
    ) -> Region {
        self.insert_in(gl, dialect, key, w, h, data, PRELOAD_PAGE)
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_in(
        &mut self,
        gl: &glow::Context,
        dialect: Dialect,
        key: MaskKey,
        w: u32,
        h: u32,
        data: &[u8],
        size: i32,
    ) -> Region {
        self.uploads += 1;
        self.upload_kinds[match key {
            MaskKey::Glyph(_) => 0,
            MaskKey::Shadow(_) => 1,
            MaskKey::Path(_) => 2,
        }] += 1;
        let t = crate::now();
        // Preload pages are big enough for shadow tiles too.
        let largest = if size == PRELOAD_PAGE {
            1024
        } else {
            LARGEST_PACKED
        };
        let region = if w > largest || h > largest {
            // SAFETY: the context is current while rendering.
            let texture = unsafe { mask_texture(gl, dialect, w as i32, h as i32, Some(data)) };
            self.large.push(texture);
            self.textures += 1;
            Region {
                texture,
                uv: [0.0, 0.0, 1.0, 1.0],
            }
        } else {
            // Only pages no frame in flight draws from: writing to one the
            // GPU may still read makes the driver wait for it.
            let frame = self.frame;
            let cold = |p: &Page| p.last_used == 0 || p.last_used + 2 <= frame || p.dirty.is_some();
            let index = match self
                .pages
                .iter_mut()
                .position(|p| p.size == size && cold(p) && fits(p, w, h))
            {
                Some(i) => i,
                None => {
                    self.textures += 1;
                    // SAFETY: as above.
                    self.pages.push(unsafe { Page::new(gl, dialect, size) });
                    self.pages.len() - 1
                }
            };
            let page = &mut self.pages[index];
            let (x, y) = place(page, w, h).expect("a page with room");
            page.write(x, y, w, h, data)
        };
        self.upload_time += crate::since(t);
        self.regions.insert(key, region);
        region
    }
}

fn fits(p: &Page, w: u32, h: u32) -> bool {
    if p.sealed {
        return false;
    }
    let (pw, ph) = (w as i32 + 1, h as i32 + 1);
    (p.x + pw <= p.size && p.y + ph.max(p.row) <= p.size) || p.y + p.row + ph <= p.size
}

/// Reserves room for a `w`×`h` mask with a one-pixel gap.
fn place(page: &mut Page, w: u32, h: u32) -> Option<(i32, i32)> {
    if !fits(page, w, h) {
        return None;
    }
    let (pw, ph) = (w as i32 + 1, h as i32 + 1);
    if page.x + pw > page.size {
        page.x = 0;
        page.y += page.row;
        page.row = 0;
    }
    let at = (page.x, page.y);
    page.x += pw;
    page.row = page.row.max(ph);
    Some(at)
}
