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

const PAGE: i32 = 1024;
/// Masks larger than this on either side get their own texture.
const LARGEST_PACKED: u32 = 256;
/// Pages kept across frames; more may be added within one frame.
const PAGES: usize = 4;
const LARGE: usize = 64;

struct Page {
    texture: glow::Texture,
    x: i32,
    y: i32,
    row: i32,
}

#[derive(Default)]
pub struct Atlas {
    pages: Vec<Page>,
    /// A page refilled every frame, for masks that are unlikely to repeat.
    transient: Option<Page>,
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
        if let Some(t) = &mut self.transient {
            (t.x, t.y, t.row) = (0, 0, 0);
        }
        let over = self.pages.len() > PAGES || self.large.len() > LARGE;
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
            if let Some(t) = self.transient.take() {
                gl.delete_texture(t.texture);
            }
            for texture in self.large.drain(..) {
                gl.delete_texture(texture);
            }
        }
        self.regions.clear();
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
        let page = self.transient.get_or_insert_with(|| {
            let zero = vec![0u8; (PAGE * PAGE) as usize];
            // SAFETY: the context is current while rendering.
            let texture = unsafe { mask_texture(gl, dialect, PAGE, PAGE, Some(&zero)) };
            Page {
                texture,
                x: 0,
                y: 0,
                row: 0,
            }
        });
        let (x, y) = place(page, w, h)?;
        Some(upload(gl, dialect, page.texture, x, y, w, h, data))
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
        let region = if w > LARGEST_PACKED || h > LARGEST_PACKED {
            // SAFETY: the context is current while rendering.
            let texture = unsafe { mask_texture(gl, dialect, w as i32, h as i32, Some(data)) };
            self.large.push(texture);
            Region {
                texture,
                uv: [0.0, 0.0, 1.0, 1.0],
            }
        } else {
            let index = match self.pages.iter_mut().position(|p| fits(p, w, h)) {
                Some(i) => i,
                None => {
                    let zero = vec![0u8; (PAGE * PAGE) as usize];
                    // SAFETY: as above.
                    let texture = unsafe { mask_texture(gl, dialect, PAGE, PAGE, Some(&zero)) };
                    self.pages.push(Page {
                        texture,
                        x: 0,
                        y: 0,
                        row: 0,
                    });
                    self.pages.len() - 1
                }
            };
            let page = &mut self.pages[index];
            let (x, y) = place(page, w, h).expect("a page with room");
            upload(gl, dialect, page.texture, x, y, w, h, data)
        };
        self.regions.insert(key, region);
        region
    }
}

fn fits(p: &Page, w: u32, h: u32) -> bool {
    let (pw, ph) = (w as i32 + 1, h as i32 + 1);
    (p.x + pw <= PAGE && p.y + ph.max(p.row) <= PAGE) || p.y + p.row + ph <= PAGE
}

/// Reserves room for a `w`×`h` mask with a one-pixel gap.
fn place(page: &mut Page, w: u32, h: u32) -> Option<(i32, i32)> {
    if !fits(page, w, h) {
        return None;
    }
    let (pw, ph) = (w as i32 + 1, h as i32 + 1);
    if page.x + pw > PAGE {
        page.x = 0;
        page.y += page.row;
        page.row = 0;
    }
    let at = (page.x, page.y);
    page.x += pw;
    page.row = page.row.max(ph);
    Some(at)
}

#[allow(clippy::too_many_arguments)]
fn upload(
    gl: &glow::Context,
    dialect: Dialect,
    texture: glow::Texture,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    data: &[u8],
) -> Region {
    let (_, format) = mask_format(dialect);
    // SAFETY: the context is current while rendering; the data matches the region.
    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_sub_image_2d(
            glow::TEXTURE_2D,
            0,
            x,
            y,
            w as i32,
            h as i32,
            format,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(data)),
        );
    }
    let s = 1.0 / PAGE as f32;
    Region {
        texture,
        uv: [
            x as f32 * s,
            y as f32 * s,
            (x + w as i32) as f32 * s,
            (y + h as i32) as f32 * s,
        ],
    }
}
