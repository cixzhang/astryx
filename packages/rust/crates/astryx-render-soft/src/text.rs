//! The one text engine every Astryx renderer uses: font selection, shaping,
//! wrapping, and glyph rasterization. Layout and pixels agree because every
//! renderer measures and draws through this module with the same font data.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use astryx_core::{Size, TextAlign, TextMeasure, TextStyle};
use swash::scale::{Render, ScaleContext, Source};
use swash::shape::ShapeContext;
use swash::text::cluster::Boundary;
use swash::text::Script;
use swash::zeno::{Format, Vector};
use swash::FontRef;

static ROBOTO_REGULAR: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Regular.ttf");
static ROBOTO_MEDIUM: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Medium.ttf");
static ROBOTO_BOLD: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Bold.ttf");

/// One of the bundled faces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    Regular,
    Medium,
    Bold,
}

impl Face {
    /// Picks the face for a style. Body, heading, and code share Roboto.
    pub fn for_style(style: &TextStyle) -> Face {
        match style.weight {
            0..=499 => Face::Regular,
            500..=599 => Face::Medium,
            _ => Face::Bold,
        }
    }

    pub fn data(self) -> &'static [u8] {
        match self {
            Face::Regular => ROBOTO_REGULAR,
            Face::Medium => ROBOTO_MEDIUM,
            Face::Bold => ROBOTO_BOLD,
        }
    }

    fn font(self) -> FontRef<'static> {
        FontRef::from_index(self.data(), 0).expect("bundled font parses")
    }
}

/// Width differences smaller than this do not wrap, so text measured
/// unwrapped and then drawn in a box of exactly that width stays on one line.
const WRAP_EPSILON: f32 = 0.01;

/// One positioned glyph, in points from the left of its line and its baseline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedGlyph {
    pub id: u16,
    pub x: f32,
    pub y: f32,
}

/// One laid out line, in points relative to the top left of the text box.
#[derive(Clone, Debug, PartialEq)]
pub struct TextLine {
    /// Left edge of the line after alignment.
    pub x: f32,
    /// Baseline, measured from the top of the box.
    pub baseline: f32,
    /// Width without trailing white space.
    pub width: f32,
    pub glyphs: Vec<PlacedGlyph>,
}

/// A paragraph laid out for drawing.
#[derive(Clone, Debug, PartialEq)]
pub struct TextLayout {
    pub face: Face,
    pub size: f32,
    pub lines: Vec<TextLine>,
    pub bounds: Size,
}

#[derive(Clone, Copy, Debug)]
struct Cluster {
    advance: f32,
    first_glyph: usize,
    glyph_count: usize,
    whitespace: bool,
    break_before: bool,
}

#[derive(Clone, Copy, Debug)]
struct Glyph {
    id: u16,
    x: f32,
    y: f32,
}

/// One hard line of text, shaped once and wrapped as often as needed.
#[derive(Debug, Default)]
struct Shaped {
    clusters: Vec<Cluster>,
    glyphs: Vec<Glyph>,
}

#[derive(Debug)]
struct ShapedText {
    paragraphs: Vec<Shaped>,
    ascent: f32,
    descent: f32,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct ShapeKey {
    text: String,
    face: Face,
    size: u32,
}

const SHAPE_CACHE_LIMIT: usize = 4096;

/// Shapes, measures, and lays out text. Implements [`TextMeasure`], so a
/// renderer hands this to layout and draws with the same instance.
pub struct TextEngine {
    inner: RefCell<EngineInner>,
}

struct EngineInner {
    shaper: ShapeContext,
    cache: HashMap<ShapeKey, Rc<ShapedText>>,
}

impl Default for TextEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TextEngine {
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(EngineInner {
                shaper: ShapeContext::new(),
                cache: HashMap::new(),
            }),
        }
    }

    fn shaped(&self, text: &str, face: Face, size: f32) -> Rc<ShapedText> {
        let key = ShapeKey {
            text: text.to_owned(),
            face,
            size: size.to_bits(),
        };
        let mut inner = self.inner.borrow_mut();
        if let Some(found) = inner.cache.get(&key) {
            return found.clone();
        }
        let font = face.font();
        let metrics = font.metrics(&[]).scale(size);
        let paragraphs = text
            .split('\n')
            .map(|p| shape_paragraph(&mut inner.shaper, font, size, p))
            .collect();
        let shaped = Rc::new(ShapedText {
            paragraphs,
            ascent: metrics.ascent,
            descent: metrics.descent,
        });
        if inner.cache.len() >= SHAPE_CACHE_LIMIT {
            inner.cache.clear();
        }
        inner.cache.insert(key, shaped.clone());
        shaped
    }

    /// Lays out `text` in a box `width` points wide (unbounded when `None`),
    /// aligning each line inside that width.
    pub fn layout(
        &self,
        text: &str,
        style: &TextStyle,
        width: Option<f32>,
        align: TextAlign,
    ) -> TextLayout {
        let face = Face::for_style(style);
        let size = style.size.max(0.0);
        let line_height = size * style.leading;
        if text.is_empty() || size == 0.0 {
            return TextLayout {
                face,
                size,
                lines: Vec::new(),
                bounds: Size::default(),
            };
        }
        let shaped = self.shaped(text, face, size);
        let half_leading = (line_height - (shaped.ascent + shaped.descent)) / 2.0;
        let mut lines = Vec::new();
        let mut max_width: f32 = 0.0;
        for paragraph in &shaped.paragraphs {
            for (start, end, line_width) in wrap(paragraph, width) {
                let baseline = lines.len() as f32 * line_height + half_leading + shaped.ascent;
                let mut pen = 0.0;
                let mut glyphs = Vec::new();
                for cluster in &paragraph.clusters[start..end] {
                    for g in &paragraph.glyphs
                        [cluster.first_glyph..cluster.first_glyph + cluster.glyph_count]
                    {
                        glyphs.push(PlacedGlyph {
                            id: g.id,
                            x: pen + g.x,
                            y: g.y,
                        });
                    }
                    pen += cluster.advance;
                }
                max_width = max_width.max(line_width);
                lines.push(TextLine {
                    x: 0.0,
                    baseline,
                    width: line_width,
                    glyphs,
                });
            }
        }
        let box_width = width.unwrap_or(max_width);
        for line in &mut lines {
            line.x = match align {
                TextAlign::Start => 0.0,
                TextAlign::Center => (box_width - line.width) / 2.0,
                TextAlign::End => box_width - line.width,
            };
        }
        let bounds = Size {
            width: max_width,
            height: lines.len() as f32 * line_height,
        };
        TextLayout {
            face,
            size,
            lines,
            bounds,
        }
    }
}

impl TextMeasure for TextEngine {
    fn measure(&self, text: &str, style: &TextStyle, max_width: Option<f32>) -> Size {
        self.layout(text, style, max_width, TextAlign::Start).bounds
    }
}

fn shape_paragraph(ctx: &mut ShapeContext, font: FontRef<'_>, size: f32, text: &str) -> Shaped {
    let mut out = Shaped::default();
    if text.is_empty() {
        return out;
    }
    // Byte offsets where a line may break before the character.
    let mut breaks = vec![false; text.len() + 1];
    let mut whitespace = vec![false; text.len() + 1];
    for ((offset, ch), (_, boundary)) in text.char_indices().zip(swash::text::analyze(text.chars()))
    {
        breaks[offset] = matches!(boundary, Boundary::Line | Boundary::Mandatory) && offset > 0;
        whitespace[offset] = ch.is_whitespace();
    }
    let script = text
        .chars()
        .map(swash::text::Codepoint::script)
        .find(|s| !matches!(s, Script::Common | Script::Inherited | Script::Unknown))
        .unwrap_or(Script::Latin);
    let mut shaper = ctx.builder(font).script(script).size(size).build();
    shaper.add_str(text);
    shaper.shape_with(|cluster| {
        let start = cluster.source.start as usize;
        let end = (cluster.source.end as usize).min(text.len());
        let first_glyph = out.glyphs.len();
        let mut advance = 0.0;
        for g in cluster.glyphs {
            out.glyphs.push(Glyph {
                id: g.id,
                x: advance + g.x,
                y: g.y,
            });
            advance += g.advance;
        }
        out.clusters.push(Cluster {
            advance,
            first_glyph,
            glyph_count: cluster.glyphs.len(),
            whitespace: (start..end).all(|i| !text.is_char_boundary(i) || whitespace[i]),
            break_before: breaks[start],
        });
    });
    out
}

/// Greedy line breaking at Unicode line break opportunities. Returns cluster
/// ranges with their widths. A word longer than the box overflows, as in CSS.
fn wrap(p: &Shaped, max_width: Option<f32>) -> Vec<(usize, usize, f32)> {
    if p.clusters.is_empty() {
        return vec![(0, 0, 0.0)];
    }
    let mut lines = Vec::new();
    let mut start = 0;
    let mut pen = 0.0f32;
    // Pen position after the last visible cluster of the current line.
    let mut visible = 0.0f32;
    // Last break opportunity on this line: cluster index and the visible width before it.
    let mut last_break: Option<(usize, f32)> = None;
    let mut i = 0;
    while i < p.clusters.len() {
        let c = p.clusters[i];
        if i > start && c.break_before {
            last_break = Some((i, visible));
        }
        if !c.whitespace {
            let end = pen + c.advance;
            if let (Some(limit), Some((b, b_visible))) = (max_width, last_break) {
                if end > limit + WRAP_EPSILON {
                    lines.push((start, b, b_visible));
                    // Re-scan from the new line start so its break opportunities are found again.
                    start = b;
                    pen = 0.0;
                    visible = 0.0;
                    last_break = None;
                    i = b;
                    continue;
                }
            }
            visible = end;
        }
        pen += c.advance;
        i += 1;
    }
    lines.push((start, p.clusters.len(), visible));
    lines
}

/// An 8-bit coverage bitmap of one glyph, placed relative to its pen position.
#[derive(Debug)]
pub struct GlyphBitmap {
    /// Offset of the bitmap's left edge from the integer pen position.
    pub left: i32,
    /// Offset of the bitmap's top edge above the baseline.
    pub top: i32,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// Horizontal subpixel positions per pixel. Four keeps text spacing even
/// while bounding the cache.
pub const SUBPIXEL_STEPS: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub face: Face,
    pub id: u16,
    /// Pixel size as `f32` bits.
    pub size: u32,
    pub subpixel: u8,
}

/// A glyph ready to draw: its bitmap and the integer pixel of its pen.
pub struct GlyphDraw {
    pub key: GlyphKey,
    pub bitmap: Rc<GlyphBitmap>,
    pub pen_x: i32,
    pub pen_y: i32,
}

const GLYPH_CACHE_LIMIT: usize = 8192;

/// Rasterizes glyphs without hinting, so shapes only depend on pixel size and
/// subpixel offset. Shared by the CPU and GPU renderers for identical text.
pub struct GlyphRasterizer {
    scaler: ScaleContext,
    cache: HashMap<GlyphKey, Rc<GlyphBitmap>>,
}

impl Default for GlyphRasterizer {
    fn default() -> Self {
        Self::new()
    }
}

impl GlyphRasterizer {
    pub fn new() -> Self {
        Self {
            scaler: ScaleContext::new(),
            cache: HashMap::new(),
        }
    }

    pub fn glyph(&mut self, key: GlyphKey) -> Rc<GlyphBitmap> {
        if let Some(found) = self.cache.get(&key) {
            return found.clone();
        }
        let size = f32::from_bits(key.size);
        let mut scaler = self
            .scaler
            .builder(key.face.font())
            .size(size)
            .hint(false)
            .build();
        let offset = Vector::new(key.subpixel as f32 / SUBPIXEL_STEPS as f32, 0.0);
        let bitmap = match Render::new(&[Source::Outline])
            .format(Format::Alpha)
            .offset(offset)
            .render(&mut scaler, key.id)
        {
            Some(image) => GlyphBitmap {
                left: image.placement.left,
                top: image.placement.top,
                width: image.placement.width,
                height: image.placement.height,
                data: image.data,
            },
            None => GlyphBitmap {
                left: 0,
                top: 0,
                width: 0,
                height: 0,
                data: Vec::new(),
            },
        };
        if self.cache.len() >= GLYPH_CACHE_LIMIT {
            self.cache.clear();
        }
        let bitmap = Rc::new(bitmap);
        self.cache.insert(key, bitmap.clone());
        bitmap
    }

    /// Places every glyph of a layout drawn with its box at `origin` (logical
    /// points) and `scale` physical pixels per point. Baselines snap to whole
    /// pixels; horizontal positions keep subpixel precision.
    pub fn place(
        &mut self,
        layout: &TextLayout,
        origin: (f32, f32),
        scale: f32,
        out: &mut Vec<GlyphDraw>,
    ) {
        let size = (layout.size * scale).to_bits();
        for line in &layout.lines {
            let baseline = ((origin.1 + line.baseline) * scale).round();
            for g in &line.glyphs {
                let x = (origin.0 + line.x + g.x) * scale;
                let mut pen_x = x.floor();
                let mut subpixel = ((x - pen_x) * SUBPIXEL_STEPS as f32).round() as u32;
                if subpixel == SUBPIXEL_STEPS {
                    pen_x += 1.0;
                    subpixel = 0;
                }
                let key = GlyphKey {
                    face: layout.face,
                    id: g.id,
                    size,
                    subpixel: subpixel as u8,
                };
                let bitmap = self.glyph(key);
                if bitmap.width == 0 || bitmap.height == 0 {
                    continue;
                }
                out.push(GlyphDraw {
                    key,
                    bitmap,
                    pen_x: pen_x as i32,
                    pen_y: (baseline - g.y * scale).round() as i32,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astryx_core::FontFamily;

    fn style(size: f32, weight: u16) -> TextStyle {
        TextStyle {
            family: FontFamily::Body,
            size,
            weight,
            leading: 1.5,
        }
    }

    #[test]
    fn weights_map_to_faces() {
        assert_eq!(Face::for_style(&style(14.0, 400)), Face::Regular);
        assert_eq!(Face::for_style(&style(14.0, 500)), Face::Medium);
        assert_eq!(Face::for_style(&style(14.0, 600)), Face::Bold);
        assert_eq!(Face::for_style(&style(14.0, 900)), Face::Bold);
    }

    #[test]
    fn line_height_is_size_times_leading() {
        let engine = TextEngine::new();
        let s = engine.measure("Hello", &style(16.0, 400), None);
        assert_eq!(s.height, 24.0);
        assert!(s.width > 30.0 && s.width < 45.0, "{s:?}");
        let two = engine.measure("Hello\nWorld", &style(16.0, 400), None);
        assert_eq!(two.height, 48.0);
    }

    #[test]
    fn empty_text_has_no_size() {
        assert_eq!(
            TextEngine::new().measure("", &style(16.0, 400), None),
            Size::default()
        );
    }

    #[test]
    fn wraps_at_width_and_never_wraps_at_its_own_width() {
        let engine = TextEngine::new();
        let st = style(14.0, 400);
        let text = "The quick brown fox jumps over the lazy dog";
        let one = engine.measure(text, &st, None);
        assert_eq!(engine.measure(text, &st, Some(one.width)), one);
        let narrow = engine.measure(text, &st, Some(one.width / 2.5));
        assert!(narrow.height >= 3.0 * 21.0, "{narrow:?}");
        assert!(narrow.width <= one.width / 2.5 + WRAP_EPSILON);
        // Each wrapped line fits and lays out the same words.
        let layout = engine.layout(text, &st, Some(one.width / 2.5), TextAlign::Start);
        let glyphs: usize = layout.lines.iter().map(|l| l.glyphs.len()).sum();
        let unwrapped: usize = engine.layout(text, &st, None, TextAlign::Start).lines[0]
            .glyphs
            .len();
        assert_eq!(glyphs, unwrapped);
    }

    #[test]
    fn long_words_overflow_like_css() {
        let engine = TextEngine::new();
        let s = engine.measure("Supercalifragilistic", &style(14.0, 400), Some(10.0));
        assert_eq!(s.height, 21.0);
        assert!(s.width > 10.0);
    }

    #[test]
    fn alignment_offsets_lines() {
        let engine = TextEngine::new();
        let st = style(14.0, 400);
        let w = engine.measure("Hi", &st, None).width;
        let c = engine.layout("Hi", &st, Some(100.0), TextAlign::Center);
        assert!((c.lines[0].x - (100.0 - w) / 2.0).abs() < 1e-4);
        let e = engine.layout("Hi", &st, Some(100.0), TextAlign::End);
        assert!((e.lines[0].x - (100.0 - w)).abs() < 1e-4);
    }

    #[test]
    fn bolder_text_is_wider() {
        let engine = TextEngine::new();
        let r = engine.measure("Settings", &style(14.0, 400), None).width;
        let b = engine.measure("Settings", &style(14.0, 700), None).width;
        assert!(b > r);
    }

    #[test]
    fn glyphs_rasterize() {
        let engine = TextEngine::new();
        let layout = engine.layout("Ag", &style(16.0, 400), None, TextAlign::Start);
        let mut raster = GlyphRasterizer::new();
        let mut out = Vec::new();
        raster.place(&layout, (0.0, 0.0), 2.0, &mut out);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|g| g.bitmap.data.iter().any(|&a| a > 200)));
    }
}
