//! Astryx CPU renderer. Draws a [`DisplayList`] into its own pixel buffer
//! with `tiny-skia`, deterministically, for headless tests, screenshots, and
//! screens without a GPU.
//!
//! Text, pixel snapping, and shadow masks come from [`astryx_raster`], the
//! same code the GPU renderer uses.

use astryx_core::{Color, DisplayList, DrawCmd, FrameInfo, Renderer, TextMeasure};
use tiny_skia::{
    FillRule, LineCap, LineJoin, Mask, Paint, PathBuilder, Pixmap, PixmapPaint, Stroke, Transform,
};

pub use astryx_raster;
pub use tiny_skia;

use astryx_raster::geom::{
    border_ring, fit_radii, line_rect, path_segs, rect_px, rounded_rect, snap_rect, width_px,
    Affine, PxRect, Seg,
};
use astryx_raster::shadow::{ShadowCache, ShadowKey};
use astryx_raster::text::{FontData, FontError, FontSet, GlyphDraw, TextEngine};

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

/// Source-over blends `color` through an 8-bit coverage bitmap whose top left
/// lands at (`x`, `y`), gated by the clip mask.
#[allow(clippy::too_many_arguments)]
fn blend_coverage(
    dst: &mut Pixmap,
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
    let (dw, dh) = (dst.width() as i32, dst.height() as i32);
    let (x0, y0) = (x.max(0), y.max(0));
    let (x1, y1) = ((x + w as i32).min(dw), (y + h as i32).min(dh));
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    let pixels = dst.data_mut();
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

enum Group {
    Clip,
    /// An opacity group drawn into its own buffer.
    Layer,
    /// An opacity group that needs no buffer.
    Pass,
    /// A transform, holding the one around it.
    Transform(Affine),
}

/// The CPU renderer. Owns its pixel buffer; read it with [`Self::pixmap`].
pub struct SoftRenderer {
    text: TextEngine,
    shadows: ShadowCache,
    base: Pixmap,
    layers: Vec<(Pixmap, f32)>,
    pool: Vec<Pixmap>,
    clips: Vec<Mask>,
    groups: Vec<Group>,
    transform: Affine,
    segs: Vec<Seg>,
    placed: Vec<GlyphDraw>,
}

impl Default for SoftRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl SoftRenderer {
    /// A renderer with the embedded Roboto faces, or with no faces when the
    /// `embedded-fonts` feature is off.
    pub fn new() -> Self {
        #[cfg(feature = "embedded-fonts")]
        let fonts = astryx_fonts::roboto();
        #[cfg(not(feature = "embedded-fonts"))]
        let fonts = FontSet::new();
        Self::with_fonts(fonts)
    }

    /// A renderer that draws text with `fonts`, in their fallback order.
    pub fn with_fonts(fonts: FontSet) -> Self {
        Self {
            text: TextEngine::new(fonts),
            shadows: ShadowCache::default(),
            base: Pixmap::new(1, 1).expect("1x1 pixmap"),
            layers: Vec::new(),
            pool: Vec::new(),
            clips: Vec::new(),
            groups: Vec::new(),
            transform: Affine::IDENTITY,
            segs: Vec::new(),
            placed: Vec::new(),
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

    fn target(&mut self) -> (&mut Pixmap, Option<&Mask>) {
        let target = match self.layers.last_mut() {
            Some((layer, _)) => layer,
            None => &mut self.base,
        };
        (target, self.clips.last())
    }

    fn fill(&mut self, rule: FillRule, color: Color) {
        let Some(path) = to_skia_path(&self.segs) else {
            return;
        };
        let mut paint = Paint::default();
        paint.set_color(skia_color(color));
        paint.anti_alias = true;
        let (target, clip) = self.target();
        target.fill_path(&path, &paint, rule, Transform::identity(), clip);
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
        let mut paint = Paint::default();
        paint.set_color(skia_color(color));
        paint.anti_alias = true;
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
        let (target, clip) = self.target();
        target.stroke_path(&path, &paint, &stroke, Transform::identity(), clip);
    }

    fn push_clip(&mut self, rect: &PxRect, radii: [f32; 4]) {
        let (w, h) = (self.base.width(), self.base.height());
        let mut mask = Mask::new(w, h).expect("frame-sized mask");
        self.segs.clear();
        rounded_rect(rect, radii, &mut self.segs);
        if let (Some(path), false) = (to_skia_path(&self.segs), rect.is_empty()) {
            mask.fill_path(&path, FillRule::Winding, true, Transform::identity());
        }
        if let Some(parent) = self.clips.last() {
            for (m, &p) in mask.data_mut().iter_mut().zip(parent.data()) {
                *m = div255(*m as u32 * p as u32) as u8;
            }
        }
        self.clips.push(mask);
        self.groups.push(Group::Clip);
    }

    fn push_opacity(&mut self, opacity: f32) {
        if opacity >= 1.0 {
            self.groups.push(Group::Pass);
            return;
        }
        let (w, h) = (self.base.width(), self.base.height());
        let mut layer = match self.pool.pop() {
            Some(p) if p.width() == w && p.height() == h => p,
            _ => Pixmap::new(w, h).expect("frame-sized layer"),
        };
        layer.fill(tiny_skia::Color::TRANSPARENT);
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
                let (target, _) = self.target();
                target.draw_pixmap(0, 0, layer.as_ref(), &paint, Transform::identity(), None);
                self.pool.push(layer);
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
            DrawCmd::PopClip => return self.pop_until(|g| matches!(g, Group::Clip)),
            DrawCmd::PopOpacity => {
                return self.pop_until(|g| matches!(g, Group::Layer | Group::Pass))
            }
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

    /// A shadow under a scaling transform: the untransformed mask, scaled.
    /// Blurred edges scale without visible loss, and the mask stays cached
    /// while the scale animates.
    fn draw_scaled_shadow(
        &mut self,
        rect: &astryx_core::Rect,
        radii: &astryx_core::Radii,
        layers: &[astryx_core::ShadowLayer],
        s: f32,
    ) {
        let r = snap_rect(rect, s);
        let radii = fit_radii(radii, s, &r);
        let t = self.transform;
        for layer in layers.iter().rev() {
            let Some(mask) = self.shadows.get(&ShadowKey::new(&r, radii, layer, s)) else {
                continue;
            };
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
                y * t.scale + t.offset.y * s,
            );
            let paint = PixmapPaint {
                quality: tiny_skia::FilterQuality::Bilinear,
                ..PixmapPaint::default()
            };
            let (target, clip) = self.target();
            target.draw_pixmap(0, 0, tinted.as_ref(), &paint, transform, clip);
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
            DrawCmd::Shadow {
                rect,
                radii,
                layers,
            } => {
                let r = rect_px(rect, s, snap);
                let radii = fit_radii(radii, s, &r);
                // CSS paints the first layer on top.
                for layer in layers.iter().rev() {
                    let Some(mask) = self.shadows.get(&ShadowKey::new(&r, radii, layer, s)) else {
                        continue;
                    };
                    let color = premultiplied(layer.color);
                    let (target, clip) = self.target();
                    let (x, y) = (r.x0 as i32 + mask.x, r.y0 as i32 + mask.y);
                    blend_coverage(
                        target,
                        clip,
                        x,
                        y,
                        mask.width,
                        mask.height,
                        &mask.data,
                        color,
                    );
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
                self.text
                    .place(&layout, (rect.x, rect.y), s, snap, &mut placed);
                let color = premultiplied(*color);
                let (target, clip) = self.target();
                for g in &placed {
                    let b = &g.bitmap;
                    blend_coverage(
                        target,
                        clip,
                        g.pen_x + b.left,
                        g.pen_y - b.top,
                        b.width,
                        b.height,
                        &b.data,
                        color,
                    );
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
            DrawCmd::PushClip { rect, radii } => {
                let r = rect_px(rect, s, snap);
                let radii = fit_radii(radii, s, &r);
                self.push_clip(&r, radii);
            }
            DrawCmd::PushOpacity(opacity) => self.push_opacity(*opacity),
            DrawCmd::PopClip
            | DrawCmd::PopOpacity
            | DrawCmd::PushTransform { .. }
            | DrawCmd::PopTransform => {}
        }
    }

    /// Draws one frame into the buffer, resizing it to the frame.
    pub fn render_list(&mut self, list: &DisplayList, frame: &FrameInfo) {
        let (w, h) = (frame.width.max(1), frame.height.max(1));
        if self.base.width() != w || self.base.height() != h {
            self.base = Pixmap::new(w, h).expect("frame pixmap");
            self.pool.clear();
        }
        self.base.fill(skia_color(list.clear));
        self.shadows.begin_frame();
        self.transform = Affine::IDENTITY;
        let scale = if frame.scale > 0.0 { frame.scale } else { 1.0 };
        for cmd in &list.commands {
            self.draw(cmd, scale);
        }
        while self.pop_group().is_some() {}
    }
}

impl Renderer for SoftRenderer {
    fn measurer(&self) -> &dyn TextMeasure {
        &self.text
    }

    fn render(&mut self, list: &DisplayList, frame: &FrameInfo) {
        self.render_list(list, frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astryx_core::{Radii, Rect};

    fn frame(w: u32, h: u32, scale: f32) -> FrameInfo {
        FrameInfo {
            width: w,
            height: h,
            scale,
            dt: 0.0,
        }
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
}
