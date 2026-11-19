//! Painting a frame in horizontal bands, on several threads.
//!
//! Each band paints every command that touches it into its own rows. A band
//! clips shapes at its top and bottom edges, and clipping a curve or a
//! slanted edge rounds it differently from drawing it whole, so bands meet
//! only at rows that just straight vertical edges cross: there clipping
//! changes nothing, and a band paints exactly the pixels the whole frame
//! would. Text and shadows are prepared before (they come from caches that
//! only the renderer's thread may use), so painting reads only shared,
//! unchanging data.

use std::collections::HashMap;

use astryx_core::{Color, DrawCmd, ImageFit, ImageId};
use astryx_raster::geom::{
    border_ring, fit_radii, line_rect, path_segs, rect_px, rounded_rect, snap_rect, width_px,
    Affine, PxRect, Seg,
};
use astryx_raster::light::{LightMap, MAP_DIVISOR};
use astryx_raster::shadow::ShadowMask;
use astryx_raster::text::GlyphBitmap;
use tiny_skia::{
    FillRule, FilterQuality, LineCap, LineJoin, Mask, Paint, Pattern, Pixmap, PixmapMut,
    PixmapPaint, SpreadMode, Stroke, Transform,
};

use crate::{blend_coverage, div255, premultiplied, skia_color, to_skia_path};

/// What every band reads: the frame's commands and what was prepared for
/// them.
pub(crate) struct Shared<'a> {
    pub cmds: &'a [DrawCmd],
    pub bounds: &'a [Option<PxRect>],
    /// Per text command: each glyph's top-left in frame pixels.
    pub glyphs: &'a [Vec<(i32, i32, &'a GlyphBitmap)>],
    /// Per shadow command: each layer's mask, in drawing order.
    pub shadows: &'a [Vec<Option<&'a ShadowMask>>],
    /// The light map each lighting pass multiplies the frame by, by the
    /// index of the command that closes the pass (`usize::MAX`: the end of
    /// the list).
    pub light_maps: &'a HashMap<usize, LightMap>,
    pub images: &'a HashMap<ImageId, Pixmap>,
    pub scale: f32,
    pub width: u32,
}

enum Group {
    Clip,
    /// An opacity group drawn into its own buffer.
    Layer,
    /// An opacity group that needs no buffer.
    Pass,
    /// A transform, holding the one around it.
    Transform(Affine),
}

/// An open clip. Its mask is made only when something drawn inside it
/// crosses its edge: most content lies well inside its clips.
struct Clip {
    /// The clip in frame pixels.
    rect: PxRect,
    radii: [f32; 4],
    /// Where the mask is fully on: inside the rect, clear of its rounded
    /// corners and antialiased edge.
    inner: PxRect,
    /// Whether its edge is antialiased: not on a pixel grid.
    anti_alias: bool,
    /// This clip's coverage times its parents', in band pixels.
    mask: Option<Mask>,
}

/// An open lighting pass: emissive rects wait for its light map.
struct LightingPass {
    emissive: Vec<(astryx_core::Rect, Color, bool)>,
}

/// Paint for a solid color. A translucent color blends in floating point,
/// as the GPU blends it: the 8-bit pipeline rounds the blend down by a level
/// in some channels (a Save button's fill at 10% over the page came out 230
/// where the exact blend is 230.9).
fn solid(color: Color) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(skia_color(color));
    paint.force_hq_pipeline = color.a < 1.0;
    paint
}

pub(crate) fn inside(a: &PxRect, b: &PxRect) -> bool {
    a.x0 >= b.x0 && a.y0 >= b.y0 && a.x1 <= b.x1 && a.y1 <= b.y1
}

pub(crate) fn overlaps(a: &PxRect, b: &PxRect) -> bool {
    a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1
}

/// Paints one band, rows `y0..y0 + h` of the frame, into `data` (those
/// rows, RGBA, already cleared). Only commands touching one of `cull` paint.
pub(crate) fn paint_band(shared: &Shared, data: &mut [u8], y0: u32, h: u32, cull: &[PxRect]) {
    if PixmapMut::from_bytes(data, shared.width, h).is_none() {
        return;
    }
    let mut p = Painter {
        shared,
        data,
        rows: h,
        y0: y0 as f32,
        cull,
        layers: Vec::new(),
        clips: Vec::new(),
        mask_level: None,
        groups: Vec::new(),
        transform: Affine::IDENTITY,
        grids: Vec::new(),
        segs: Vec::new(),
        lighting: None,
    };
    for i in 0..shared.cmds.len() {
        p.draw(i);
    }
    p.finish_lighting(usize::MAX);
    while p.pop_group().is_some() {}
}

struct Painter<'s, 'b> {
    shared: &'s Shared<'s>,
    /// The band's pixels, RGBA rows.
    data: &'b mut [u8],
    rows: u32,
    /// The band's first row in the frame.
    y0: f32,
    cull: &'s [PxRect],
    layers: Vec<(Pixmap, f32)>,
    clips: Vec<Clip>,
    /// The clip whose mask the command being drawn needs, if any.
    mask_level: Option<usize>,
    groups: Vec<Group>,
    transform: Affine,
    /// Open pixel grids, innermost last, in points.
    grids: Vec<f32>,
    segs: Vec<Seg>,
    lighting: Option<LightingPass>,
}

impl Painter<'_, '_> {
    fn width(&self) -> u32 {
        self.shared.width
    }

    fn height(&self) -> u32 {
        self.rows
    }

    fn band(data: &mut [u8], w: u32, h: u32) -> PixmapMut<'_> {
        PixmapMut::from_bytes(data, w, h).expect("band pixmap")
    }

    /// From frame pixels to band pixels.
    fn to_band(&self) -> Transform {
        Transform::from_translate(0.0, -self.y0)
    }

    fn target(&mut self) -> (PixmapMut<'_>, Option<&Mask>) {
        if let Some(level) = self.mask_level {
            self.make_mask(level);
        }
        let (w, h) = (self.shared.width, self.rows);
        let target = match self.layers.last_mut() {
            Some((layer, _)) => layer.as_mut(),
            None => Self::band(self.data, w, h),
        };
        (
            target,
            self.mask_level.and_then(|l| self.clips[l].mask.as_ref()),
        )
    }

    /// Makes the mask of clip `level`: its coverage, times its parent's
    /// where the parent's edge reaches into it.
    fn make_mask(&mut self, level: usize) {
        if self.clips[level].mask.is_some() {
            return;
        }
        let mut mask = Mask::new(self.width(), self.height()).expect("band-sized mask");
        let (rect, radii) = (self.clips[level].rect, self.clips[level].radii);
        let mut segs = Vec::new();
        rounded_rect(&rect, radii, &mut segs);
        if let (Some(path), false) = (to_skia_path(&segs), rect.is_empty()) {
            let anti_alias = self.clips[level].anti_alias;
            mask.fill_path(&path, FillRule::Winding, anti_alias, self.to_band());
        }
        // The nearest parent whose edge reaches into this clip.
        let parent = (0..level)
            .rev()
            .find(|&p| !inside(&rect, &self.clips[p].inner));
        if let Some(p) = parent {
            self.make_mask(p);
            let parent = self.clips[p].mask.as_ref().expect("parent mask");
            for (m, &p) in mask.data_mut().iter_mut().zip(parent.data()) {
                *m = div255(*m as u32 * p as u32) as u8;
            }
        }
        self.clips[level].mask = Some(mask);
    }

    /// Which clip mask drawing within `bounds` needs, or `None` when it
    /// lies inside every clip's fully covered area. `Err` when it lies
    /// outside a clip, the band or the regions being drawn, so it draws
    /// nothing.
    fn mask_for(&self, bounds: Option<&PxRect>) -> Result<Option<usize>, ()> {
        let Some(b) = bounds else {
            return Ok(self.clips.len().checked_sub(1));
        };
        if !self.cull.iter().any(|r| overlaps(b, r))
            || self.clips.iter().any(|c| !overlaps(b, &c.rect))
        {
            return Err(());
        }
        Ok((0..self.clips.len())
            .rev()
            .find(|&i| !inside(b, &self.clips[i].inner)))
    }

    fn fill(&mut self, rule: FillRule, color: Color) {
        let Some(path) = to_skia_path(&self.segs) else {
            return;
        };
        let mut paint = solid(color);
        paint.anti_alias = self.grids.is_empty();
        let to_band = self.to_band();
        let (mut target, clip) = self.target();
        target.fill_path(&path, &paint, rule, to_band, clip);
    }

    fn fill_px_rect(&mut self, rect: &PxRect, color: Color) {
        self.segs.clear();
        rounded_rect(rect, [0.0; 4], &mut self.segs);
        self.fill(FillRule::Winding, color);
    }

    fn stroke(&mut self, width: f32, color: Color, round: bool) {
        let Some(path) = to_skia_path(&self.segs) else {
            return;
        };
        let mut paint = solid(color);
        paint.anti_alias = self.grids.is_empty();
        let stroke = Stroke {
            width,
            line_cap: if round { LineCap::Round } else { LineCap::Butt },
            line_join: if round {
                LineJoin::Round
            } else {
                LineJoin::Miter
            },
            ..Stroke::default()
        };
        let to_band = self.to_band();
        let (mut target, clip) = self.target();
        target.stroke_path(&path, &paint, &stroke, to_band, clip);
    }

    /// Blends an 8-bit coverage bitmap whose top left lands at frame pixel
    /// (`x`, `y`).
    #[allow(clippy::too_many_arguments)]
    fn blend(&mut self, x: i32, y: i32, w: u32, h: u32, coverage: &[u8], color: [u8; 4]) {
        let y = y - self.y0 as i32;
        let (mut target, clip) = self.target();
        let (tw, th) = (target.width(), target.height());
        blend_coverage(target.data_mut(), tw, th, clip, x, y, w, h, coverage, color);
    }

    fn push_clip(&mut self, rect: &PxRect, radii: [f32; 4]) {
        let corner = radii.iter().fold(0.0f32, |a, &r| a.max(r));
        let inset = corner + 1.0;
        let inner = PxRect {
            x0: rect.x0 + inset,
            y0: rect.y0 + inset,
            x1: (rect.x1 - inset).max(rect.x0 + inset),
            y1: (rect.y1 - inset).max(rect.y0 + inset),
        };
        // A rect with no area clips everything away.
        let outer = if rect.is_empty() {
            PxRect {
                x0: 0.0,
                y0: 0.0,
                x1: 0.0,
                y1: 0.0,
            }
        } else {
            *rect
        };
        self.clips.push(Clip {
            rect: outer,
            radii,
            inner,
            anti_alias: self.grids.is_empty(),
            mask: None,
        });
        self.groups.push(Group::Clip);
    }

    fn push_opacity(&mut self, opacity: f32) {
        if opacity >= 1.0 {
            self.groups.push(Group::Pass);
            return;
        }
        let layer = Pixmap::new(self.width(), self.height()).expect("band-sized layer");
        self.layers.push((layer, opacity.max(0.0)));
        self.groups.push(Group::Layer);
    }

    /// Closes the innermost group and returns which kind it was.
    fn pop_group(&mut self) -> Option<Group> {
        let group = self.groups.pop()?;
        match group {
            Group::Clip => {
                self.clips.pop();
            }
            Group::Layer => {
                let (layer, opacity) = self.layers.pop().expect("layer for group");
                let paint = PixmapPaint {
                    opacity,
                    ..PixmapPaint::default()
                };
                let (w, h) = (self.shared.width, self.rows);
                let mut target = match self.layers.last_mut() {
                    Some((layer, _)) => layer.as_mut(),
                    None => Self::band(self.data, w, h),
                };
                target.draw_pixmap(0, 0, layer.as_ref(), &paint, Transform::identity(), None);
            }
            Group::Pass => {}
            Group::Transform(previous) => self.transform = previous,
        }
        Some(group)
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

    fn draw(&mut self, i: usize) {
        let shared = self.shared;
        let cmd = &shared.cmds[i];
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
            DrawCmd::PushLighting(_) => {
                if self.lighting.is_none() {
                    self.lighting = Some(LightingPass {
                        emissive: Vec::new(),
                    });
                }
                return;
            }
            DrawCmd::PopLighting => return self.finish_lighting(i),
            DrawCmd::PopClip => return self.pop_until(|g| matches!(g, Group::Clip)),
            DrawCmd::PopOpacity => {
                return self.pop_until(|g| matches!(g, Group::Layer | Group::Pass))
            }
            // Lights were placed when the frame was prepared.
            DrawCmd::Light(_) => return,
            _ => {}
        }
        if !matches!(cmd, DrawCmd::PushClip { .. } | DrawCmd::PushOpacity(_)) {
            match self.mask_for(shared.bounds[i].as_ref()) {
                Ok(level) => self.mask_level = level,
                // Outside a clip or the band: nothing to draw. Emissive
                // rects still join their lighting pass.
                Err(()) if !matches!(cmd, DrawCmd::Emissive { .. }) => return,
                Err(()) => self.mask_level = self.clips.len().checked_sub(1),
            }
        }
        let t = self.transform;
        if t.is_identity() {
            self.draw_moved(i, cmd, true);
        } else if let (
            DrawCmd::Shadow {
                rect,
                radii,
                layers,
            },
            false,
        ) = (cmd, t.snaps())
        {
            self.draw_scaled_shadow(i, rect, radii, layers);
        } else {
            self.draw_moved(i, &t.apply(cmd), t.snaps());
        }
    }

    /// A shadow under a scaling transform: the untransformed mask, scaled.
    fn draw_scaled_shadow(
        &mut self,
        i: usize,
        rect: &astryx_core::Rect,
        radii: &astryx_core::Radii,
        layers: &[astryx_core::ShadowLayer],
    ) {
        let s = self.shared.scale;
        let r = snap_rect(rect, s);
        let _ = fit_radii(radii, s, &r);
        let t = self.transform;
        let masks = &self.shared.shadows[i];
        for (layer, mask) in layers.iter().rev().zip(masks) {
            let Some(mask) = mask else { continue };
            let Some(mut tinted) = Pixmap::new(mask.width, mask.height) else {
                continue;
            };
            let color = premultiplied(layer.color);
            for (px, &c) in tinted
                .data_mut()
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(&mask.data)
            {
                *px = color.map(|v| div255(v as u32 * c as u32) as u8);
            }
            let (x, y) = (r.x0 + mask.x as f32, r.y0 + mask.y as f32);
            let transform = Transform::from_row(
                t.scale,
                0.0,
                0.0,
                t.scale,
                x * t.scale + t.offset.x * s,
                y * t.scale + t.offset.y * s - self.y0,
            );
            let paint = PixmapPaint {
                quality: tiny_skia::FilterQuality::Bilinear,
                ..PixmapPaint::default()
            };
            let (mut target, clip) = self.target();
            target.draw_pixmap(0, 0, tinted.as_ref(), &paint, transform, clip);
        }
    }

    /// Draws command `i`, already moved by the transform. Edges snap to
    /// whole pixels only when `snap` is set.
    fn draw_moved(&mut self, i: usize, cmd: &DrawCmd, snap: bool) {
        let s = self.shared.scale;
        match cmd {
            DrawCmd::Rect {
                rect,
                radii,
                fill,
                stroke,
            } => {
                let r = rect_px(rect, s, snap);
                if r.is_empty() {
                    return;
                }
                let radii = fit_radii(radii, s, &r);
                if fill.a > 0.0 {
                    self.segs.clear();
                    rounded_rect(&r, radii, &mut self.segs);
                    self.fill(FillRule::Winding, *fill);
                }
                if let Some((color, width)) = stroke {
                    let w = width_px(*width, s, snap);
                    if w > 0.0 && color.a > 0.0 {
                        self.segs.clear();
                        let ring = border_ring(&r, radii, w, &mut self.segs);
                        self.fill(
                            if ring {
                                FillRule::EvenOdd
                            } else {
                                FillRule::Winding
                            },
                            *color,
                        );
                    }
                }
            }
            DrawCmd::Shadow { rect, layers, .. } => {
                let r = rect_px(rect, s, snap);
                let masks = &self.shared.shadows[i];
                // CSS paints the first layer on top.
                for (layer, mask) in layers.iter().rev().zip(masks) {
                    let Some(mask) = mask else { continue };
                    let color = premultiplied(layer.color);
                    let (x, y) = (r.x0 as i32 + mask.x, r.y0 as i32 + mask.y);
                    self.blend(x, y, mask.width, mask.height, &mask.data, color);
                }
            }
            DrawCmd::Text { color, .. } => {
                let color = premultiplied(*color);
                let glyphs = &self.shared.glyphs[i];
                for &(x, y, b) in glyphs {
                    self.blend(x, y, b.width, b.height, &b.data, color);
                }
            }
            DrawCmd::Line {
                from,
                to,
                color,
                width,
            } => {
                if let Some(r) = line_rect(*from, *to, *width, s, snap) {
                    self.fill_px_rect(&r, *color);
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
                    self.fill(FillRule::Winding, *fill);
                }
                if let Some((color, width)) = stroke {
                    self.stroke(width * s, *color, true);
                }
            }
            DrawCmd::Image {
                rect,
                radii,
                image,
                fit,
            } => self.draw_image(rect, radii, *image, *fit, snap),
            DrawCmd::PushClip { rect, radii } => {
                let r = rect_px(rect, s, snap);
                let radii = fit_radii(radii, s, &r);
                self.push_clip(&r, radii);
            }
            DrawCmd::PushOpacity(opacity) => self.push_opacity(*opacity),
            DrawCmd::Emissive { rect, color } => match &mut self.lighting {
                Some(pass) => pass.emissive.push((*rect, *color, snap)),
                None => self.draw_emissive(rect, *color, snap),
            },
            DrawCmd::Light(_)
            | DrawCmd::PopClip
            | DrawCmd::PopOpacity
            | DrawCmd::PushTransform { .. }
            | DrawCmd::PopTransform
            | DrawCmd::PushPixelGrid { .. }
            | DrawCmd::PopPixelGrid
            | DrawCmd::PushLighting(_)
            | DrawCmd::PopLighting => {}
        }
    }

    fn draw_emissive(&mut self, rect: &astryx_core::Rect, color: Color, snap: bool) {
        let s = self.shared.scale;
        let r = rect_px(rect, s, snap);
        if r.is_empty() || color.a <= 0.0 {
            return;
        }
        self.segs.clear();
        rounded_rect(&r, [0.0; 4], &mut self.segs);
        self.fill(FillRule::Winding, color);
    }

    /// Multiplies the light map over the target, then draws the emissive
    /// rects. `at` is the index of the command closing the pass.
    fn finish_lighting(&mut self, at: usize) {
        let Some(pass) = self.lighting.take() else {
            return;
        };
        if let Some(map) = self.shared.light_maps.get(&at) {
            let y0 = self.y0 as u32;
            let (mut target, _) = {
                self.mask_level = None;
                self.target()
            };
            let (w, h) = (target.width(), target.height());
            let data = target.data_mut();
            for y in 0..h {
                let row = ((y0 + y) / MAP_DIVISOR * map.width) as usize;
                for x in 0..w {
                    let m = &map.rgb[(row + (x / MAP_DIVISOR) as usize) * 3..][..3];
                    let px = &mut data[((y * w + x) * 4) as usize..][..3];
                    for (c, &k) in px.iter_mut().zip(m) {
                        *c = div255(*c as u32 * k as u32) as u8;
                    }
                }
            }
        }
        self.mask_level = self.clips.len().checked_sub(1);
        for (rect, color, snap) in pass.emissive {
            self.draw_emissive(&rect, color, snap);
        }
    }

    fn draw_image(
        &mut self,
        rect: &astryx_core::Rect,
        radii: &astryx_core::Radii,
        image: ImageId,
        fit: ImageFit,
        snap: bool,
    ) {
        let s = self.shared.scale;
        let r = rect_px(rect, s, snap);
        let images = self.shared.images;
        let Some(pixmap) = images.get(&image) else {
            return;
        };
        let (pw, ph) = (pixmap.width(), pixmap.height());
        if r.is_empty() {
            return;
        }
        let radii = fit_radii(radii, s, &r);
        let area = astryx_core::Rect::new(r.x0, r.y0, r.width(), r.height());
        let dest = fit.place(area, pw, ph);
        // Contain can leave part of the box empty, so it clips to the box and
        // fills only where the image lands; the other fits cover the box.
        let contained = fit == ImageFit::Contain;
        if contained {
            self.push_clip(&r, radii);
            self.mask_level = Some(self.clips.len() - 1);
        }
        if let Some(level) = self.mask_level {
            self.make_mask(level);
        }
        self.segs.clear();
        if contained {
            // Whole pixels while snapping, so the letterbox edges are sharp.
            let edge = |v: f32| if snap { v.round() } else { v };
            let d = PxRect {
                x0: edge(dest.x.max(r.x0)),
                y0: edge(dest.y.max(r.y0)),
                x1: edge((dest.x + dest.width).min(r.x1)),
                y1: edge((dest.y + dest.height).min(r.y1)),
            };
            rounded_rect(&d, [0.0; 4], &mut self.segs);
        } else {
            rounded_rect(&r, radii, &mut self.segs);
        }
        if let Some(path) = to_skia_path(&self.segs) {
            let transform = Transform::from_row(
                dest.width / pw as f32,
                0.0,
                0.0,
                dest.height / ph as f32,
                dest.x,
                dest.y,
            );
            let paint = Paint {
                shader: Pattern::new(
                    pixmap.as_ref(),
                    SpreadMode::Pad,
                    // On a pixel grid, image pixels stay square.
                    if self.grids.is_empty() {
                        FilterQuality::Bilinear
                    } else {
                        FilterQuality::Nearest
                    },
                    1.0,
                    transform,
                ),
                anti_alias: self.grids.is_empty(),
                ..Paint::default()
            };
            let to_band = self.to_band();
            let (mut target, clip) = self.target();
            target.fill_path(&path, &paint, FillRule::Winding, to_band, clip);
        }
        if contained {
            self.pop_group();
        }
    }
}
