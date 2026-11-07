//! The one text engine every Astryx renderer uses: font selection with
//! fallback, shaping, wrapping, and glyph rasterization. Layout and pixels
//! agree because every renderer measures and draws through this module.
//!
//! Fonts arrive at runtime in a [`FontSet`]. Faces can be added later with
//! [`TextEngine::add_fallback_font`]; text measured after that uses them, so
//! layout is correct again on the next frame.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use astryx_core::{FontFamily, Size, TextAlign, TextDirection, TextMeasure, TextStyle, ThemeFont};
use swash::scale::{Render, ScaleContext, Source};
use swash::shape::{Direction, ShapeContext};
use swash::text::cluster::Boundary;
use swash::text::Codepoint;
use unicode_bidi::{BidiInfo, Level};

pub use swash::text::Script;

/// Supplies faces for a script no face in the set covers, the first time
/// text in that script is shaped. Returns no faces when it has none.
pub type FallbackProvider = Box<dyn FnMut(Script) -> Vec<FontData>>;
use swash::zeno::{Format, Vector};
use swash::{CacheKey, FontRef, StringId};

/// Font file bytes, borrowed for the program's lifetime or shared.
#[derive(Clone, Debug)]
pub enum FontData {
    Static(&'static [u8]),
    Shared(Arc<[u8]>),
}

impl FontData {
    pub fn bytes(&self) -> &[u8] {
        match self {
            FontData::Static(b) => b,
            FontData::Shared(b) => b,
        }
    }
}

impl From<&'static [u8]> for FontData {
    fn from(bytes: &'static [u8]) -> Self {
        FontData::Static(bytes)
    }
}

impl From<Vec<u8>> for FontData {
    fn from(bytes: Vec<u8>) -> Self {
        FontData::Shared(bytes.into())
    }
}

/// The bytes are not a font this engine can read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontError;

impl std::fmt::Display for FontError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("not a readable OpenType or TrueType font")
    }
}

impl std::error::Error for FontError {}

#[derive(Clone, Debug)]
struct Face {
    data: FontData,
    offset: u32,
    key: CacheKey,
    weight: u16,
}

impl Face {
    fn font(&self) -> FontRef<'_> {
        FontRef {
            data: self.data.bytes(),
            offset: self.offset,
            key: self.key,
        }
    }
}

/// Faces grouped by family. Families are tried in the order they were
/// first added: the first is the primary font, the rest are fallbacks for
/// characters it lacks. Within a family the face is picked by weight, as CSS
/// font matching does.
#[derive(Clone, Debug, Default)]
pub struct FontSet {
    faces: Vec<Face>,
    families: Vec<(String, Vec<usize>)>,
}

impl FontSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds the first face of a font file. A face whose family is already
    /// in the set joins it as another weight; otherwise its family becomes
    /// the last fallback.
    pub fn add(&mut self, data: impl Into<FontData>) -> Result<(), FontError> {
        let data = data.into();
        let font = FontRef::from_index(data.bytes(), 0).ok_or(FontError)?;
        let strings = font.localized_strings();
        let family = strings
            .find_by_id(StringId::TypographicFamily, None)
            .or_else(|| strings.find_by_id(StringId::Family, None))
            .map(|s| s.chars().collect::<String>())
            .unwrap_or_default();
        let face = Face {
            offset: font.offset,
            key: font.key,
            weight: font.attributes().weight().0,
            data,
        };
        self.faces.push(face);
        let index = self.faces.len() - 1;
        match self.families.iter_mut().find(|(name, _)| *name == family) {
            Some((_, faces)) => faces.push(index),
            None => self.families.push((family, vec![index])),
        }
        Ok(())
    }

    /// Builder form of [`Self::add`].
    pub fn with(mut self, data: impl Into<FontData>) -> Result<Self, FontError> {
        self.add(data)?;
        Ok(self)
    }

    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }

    /// Family names in fallback order.
    pub fn families(&self) -> impl Iterator<Item = &str> {
        self.families.iter().map(|(name, _)| name.as_str())
    }

    fn has_family(&self, name: &str) -> bool {
        self.families.iter().any(|(family, _)| family == name)
    }

    /// One face per family for a CSS weight, in fallback order, with the
    /// families named in `first` moved to the front in that order.
    fn candidates(&self, weight: u16, first: &[&str]) -> Vec<usize> {
        let pick = |faces: &Vec<usize>| {
            match_weight(faces.iter().map(|&i| (i, self.faces[i].weight)), weight)
        };
        let lead = first
            .iter()
            .filter_map(|want| self.families.iter().find(|(name, _)| name == want));
        let rest = self
            .families
            .iter()
            .filter(|(name, _)| !first.contains(&name.as_str()));
        lead.chain(rest)
            .filter_map(|(_, faces)| pick(faces))
            .collect()
    }
}

/// CSS font-weight matching: the closest face, searching heavier first for
/// bold requests and lighter first for light ones.
fn match_weight(faces: impl Iterator<Item = (usize, u16)>, want: u16) -> Option<usize> {
    let faces: Vec<(usize, u16)> = faces.collect();
    let rank = |w: u16| -> (u8, u16) {
        if w == want {
            (0, 0)
        } else if (400..=500).contains(&want) {
            if w > want && w <= 500 {
                (1, w - want)
            } else if w < want {
                (2, want - w)
            } else {
                (3, w - want)
            }
        } else if want < 400 {
            if w < want {
                (1, want - w)
            } else {
                (2, w - want)
            }
        } else if w > want {
            (1, w - want)
        } else {
            (2, want - w)
        }
    };
    faces
        .into_iter()
        .min_by_key(|&(_, w)| rank(w))
        .map(|(i, _)| i)
}

/// Width differences smaller than this do not wrap, so text measured
/// unwrapped and then drawn in a box of exactly that width stays on one line.
const WRAP_EPSILON: f32 = 0.01;

/// One positioned glyph, in points from the left of its line and its baseline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedGlyph {
    /// Index of the face in the engine's font set. Stable once added.
    pub face: u16,
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
    pub size: f32,
    pub lines: Vec<TextLine>,
    pub bounds: Size,
}

#[derive(Clone, Copy, Debug)]
struct Cluster {
    advance: f32,
    /// Bidirectional embedding level: odd reads right to left.
    level: u8,
    first_glyph: usize,
    glyph_count: usize,
    whitespace: bool,
    break_before: bool,
}

#[derive(Clone, Copy, Debug)]
struct Glyph {
    face: u16,
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
    weight: u16,
    size: u32,
    family: Option<&'static str>,
    direction: Option<TextDirection>,
}

const SHAPE_CACHE_LIMIT: usize = 4096;
const GLYPH_CACHE_LIMIT: usize = 8192;

/// Shapes, measures, lays out, and rasterizes text. Implements
/// [`TextMeasure`], so a renderer hands this to layout and draws with the
/// same instance.
pub struct TextEngine {
    inner: RefCell<EngineInner>,
}

struct EngineInner {
    fonts: FontSet,
    generation: u64,
    shaper: ShapeContext,
    scaler: ScaleContext,
    shapes: HashMap<ShapeKey, Rc<ShapedText>>,
    glyphs: HashMap<GlyphKey, Rc<GlyphBitmap>>,
    provider: Option<FallbackProvider>,
    /// Scripts already asked for, whether or not a face came back.
    requested: Vec<Script>,
    /// Scripts asked for that no provider answered, for shells that load
    /// faces asynchronously.
    missing: Vec<Script>,
}

impl Default for TextEngine {
    fn default() -> Self {
        Self::new(FontSet::new())
    }
}

impl TextEngine {
    pub fn new(fonts: FontSet) -> Self {
        Self {
            inner: RefCell::new(EngineInner {
                fonts,
                generation: 0,
                shaper: ShapeContext::new(),
                scaler: ScaleContext::new(),
                shapes: HashMap::new(),
                glyphs: HashMap::new(),
                provider: None,
                requested: Vec::new(),
                missing: Vec::new(),
            }),
        }
    }

    /// Adds a face after creation, for example one the web shell loaded.
    /// Measurements made from now on use it; ask for a new frame so layout
    /// runs again.
    pub fn add_fallback_font(&self, data: impl Into<FontData>) -> Result<(), FontError> {
        let mut inner = self.inner.borrow_mut();
        inner.fonts.add(data)?;
        // Shapes picked faces from the old set. Glyph bitmaps stay valid
        // because face indices never change.
        inner.shapes.clear();
        inner.generation += 1;
        Ok(())
    }

    /// Sets where faces for uncovered scripts come from. It is asked once per
    /// script, while text is shaped.
    pub fn set_fallback_provider(&self, provider: FallbackProvider) {
        self.inner.borrow_mut().provider = Some(provider);
    }

    /// Scripts that appeared in text with no face to draw them and no
    /// provider to supply one. Each script is reported once. A shell that
    /// loads faces asynchronously fetches them and calls
    /// [`Self::add_fallback_font`].
    pub fn take_missing_scripts(&self) -> Vec<Script> {
        std::mem::take(&mut self.inner.borrow_mut().missing)
    }

    /// Changes whenever the font set changes, so callers that cache
    /// measurements know to drop them.
    pub fn generation(&self) -> u64 {
        self.inner.borrow().generation
    }

    pub fn has_fonts(&self) -> bool {
        !self.inner.borrow().fonts.is_empty()
    }

    /// Adds the files of a theme's font the first time it is drawn.
    fn load_theme_font(&self, font: &'static ThemeFont) {
        if self.inner.borrow().fonts.has_family(font.family) {
            return;
        }
        let mut inner = self.inner.borrow_mut();
        let mut added = false;
        for &file in font.files {
            added |= inner.fonts.add(file).is_ok();
        }
        if added {
            inner.shapes.clear();
            inner.generation += 1;
        }
    }

    fn shaped(&self, text: &str, style: &TextStyle, size: f32) -> Rc<ShapedText> {
        let font = match style.family {
            FontFamily::Theme(font) => {
                self.load_theme_font(font);
                Some(font)
            }
            _ => None,
        };
        let weight = style.weight;
        let direction = style.direction;
        let key = ShapeKey {
            text: text.to_owned(),
            weight,
            size: size.to_bits(),
            family: font.map(|f| f.family),
            direction,
        };
        let mut inner = self.inner.borrow_mut();
        if let Some(found) = inner.shapes.get(&key) {
            return found.clone();
        }
        let inner = &mut *inner;
        if inner.request_faces(text) {
            // Earlier shapes drew these characters as missing glyphs.
            inner.shapes.clear();
            inner.generation += 1;
        }
        let mut first: Vec<&str> = font.map_or(Vec::new(), |f| f.fallbacks.to_vec());
        if let Some(f) = font {
            first.insert(0, f.family);
        }
        let candidates = inner.fonts.candidates(weight, &first);
        let (ascent, descent) = match candidates.first() {
            Some(&primary) => {
                let m = inner.fonts.faces[primary].font().metrics(&[]).scale(size);
                (m.ascent, m.descent)
            }
            // Typical proportions until a face arrives.
            None => (size * 0.8, size * 0.2),
        };
        let paragraphs = text
            .split('\n')
            .map(|p| {
                shape_paragraph(
                    &mut inner.shaper,
                    &inner.fonts,
                    &candidates,
                    size,
                    direction,
                    p,
                )
            })
            .collect();
        let shaped = Rc::new(ShapedText {
            paragraphs,
            ascent,
            descent,
        });
        if inner.shapes.len() >= SHAPE_CACHE_LIMIT {
            inner.shapes.clear();
        }
        inner.shapes.insert(key, shaped.clone());
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
        let size = style.size.max(0.0);
        let line_height = size * style.leading;
        if text.is_empty() || size == 0.0 {
            return TextLayout {
                size,
                lines: Vec::new(),
                bounds: Size::default(),
            };
        }
        let shaped = self.shaped(text, style, size);
        let half_leading = (line_height - (shaped.ascent + shaped.descent)) / 2.0;
        let mut lines = Vec::new();
        let mut max_width: f32 = 0.0;
        for paragraph in &shaped.paragraphs {
            for (start, end, line_width) in wrap(paragraph, width) {
                let baseline = lines.len() as f32 * line_height + half_leading + shaped.ascent;
                let mut pen = 0.0;
                let mut glyphs = Vec::new();
                for ix in visual_order(&paragraph.clusters, start, end) {
                    let cluster = &paragraph.clusters[ix];
                    for g in &paragraph.glyphs
                        [cluster.first_glyph..cluster.first_glyph + cluster.glyph_count]
                    {
                        glyphs.push(PlacedGlyph {
                            face: g.face,
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
            size,
            lines,
            bounds,
        }
    }

    /// The coverage bitmap of one glyph, rasterized without hinting so its
    /// shape only depends on pixel size and subpixel offset.
    pub fn glyph(&self, key: GlyphKey) -> Rc<GlyphBitmap> {
        let mut inner = self.inner.borrow_mut();
        if let Some(found) = inner.glyphs.get(&key) {
            return found.clone();
        }
        let inner = &mut *inner;
        let face = &inner.fonts.faces[key.face as usize];
        let mut scaler = inner
            .scaler
            .builder(face.font())
            .size(f32::from_bits(key.size))
            .hint(false)
            .build();
        // Low four bits step right, high four bits step down; font space points up.
        let steps = SUBPIXEL_STEPS as f32;
        let offset = Vector::new(
            (key.subpixel & 15) as f32 / steps,
            -((key.subpixel >> 4) as f32) / steps,
        );
        let mut bitmap = match Render::new(&[Source::Outline])
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
        if key.cell > 0 {
            for v in &mut bitmap.data {
                *v = if *v >= 128 { 255 } else { 0 };
            }
        }
        if key.cell > 1 {
            bitmap = bitmap.upscaled(u32::from(key.cell));
        }
        if inner.glyphs.len() >= GLYPH_CACHE_LIMIT {
            inner.glyphs.clear();
        }
        let bitmap = Rc::new(bitmap);
        inner.glyphs.insert(key, bitmap.clone());
        bitmap
    }

    /// Places every glyph of a layout drawn with its box at `origin` (logical
    /// points) and `scale` physical pixels per point. Horizontal positions
    /// keep subpixel precision. Baselines snap to whole pixels when `snap` is
    /// set, else they keep subpixel precision too, so moving text glides.
    pub fn place(
        &self,
        layout: &TextLayout,
        origin: (f32, f32),
        scale: f32,
        snap: bool,
        out: &mut Vec<GlyphDraw>,
    ) {
        let size = (layout.size * scale).to_bits();
        let steps = SUBPIXEL_STEPS as f32;
        // Splits a position into a whole pixel and a quarter-pixel step.
        let split = |v: f32| {
            let mut whole = v.floor();
            let mut step = ((v - whole) * steps).round() as u32;
            if step == SUBPIXEL_STEPS {
                whole += 1.0;
                step = 0;
            }
            (whole as i32, step as u8)
        };
        for line in &layout.lines {
            let baseline = (origin.1 + line.baseline) * scale;
            let baseline = if snap { baseline.round() } else { baseline };
            for g in &line.glyphs {
                let (pen_x, sx) = split((origin.0 + line.x + g.x) * scale);
                let (pen_y, sy) = split(baseline - g.y * scale);
                let key = GlyphKey {
                    face: g.face,
                    id: g.id,
                    size,
                    subpixel: sx | (sy << 4),
                    cell: 0,
                };
                let bitmap = self.glyph(key);
                if bitmap.width == 0 || bitmap.height == 0 {
                    continue;
                }
                out.push(GlyphDraw {
                    key,
                    bitmap,
                    pen_x,
                    pen_y,
                });
            }
        }
    }
}

impl TextEngine {
    /// [`Self::place`] on a pixel grid of `cell` points: line origins and
    /// baselines snap to whole cells and glyphs draw without antialiasing.
    /// When a cell is a whole number of device pixels, glyphs are sampled
    /// once per cell and scaled up, so a pixel font's design pixels fill
    /// whole cells even where its outlines leave its grid.
    pub fn place_on_grid(
        &self,
        layout: &TextLayout,
        origin: (f32, f32),
        scale: f32,
        cell: f32,
        out: &mut Vec<GlyphDraw>,
    ) {
        let snap = |v: f32| (v / cell).round() * cell;
        let px = cell * scale;
        let k = if (px - px.round()).abs() < 0.01 && (2.0..=64.0).contains(&px.round()) {
            px.round()
        } else {
            1.0
        };
        let size = (layout.size * scale / k).to_bits();
        for line in &layout.lines {
            let baseline = (snap(origin.1 + line.baseline) * scale).round();
            let line_x = snap(origin.0 + line.x);
            for g in &line.glyphs {
                let pen_x = ((line_x + g.x) * scale / k).round() * k;
                let pen_y = ((baseline - g.y * scale) / k).round() * k;
                let key = GlyphKey {
                    face: g.face,
                    id: g.id,
                    size,
                    subpixel: 0,
                    cell: k as u8,
                };
                let bitmap = self.glyph(key);
                if bitmap.width == 0 || bitmap.height == 0 {
                    continue;
                }
                out.push(GlyphDraw {
                    key,
                    bitmap,
                    pen_x: pen_x as i32,
                    pen_y: pen_y as i32,
                });
            }
        }
    }
}

impl EngineInner {
    /// Asks for faces for the scripts in `text` no face covers. Returns true
    /// when faces were added.
    fn request_faces(&mut self, text: &str) -> bool {
        let mut added = false;
        for c in text.chars() {
            if c.is_whitespace() || c.is_control() {
                continue;
            }
            let script = c.script();
            if matches!(script, Script::Common | Script::Inherited | Script::Unknown)
                || self.requested.contains(&script)
            {
                continue;
            }
            let covered = self
                .fonts
                .faces
                .iter()
                .any(|f| f.font().charmap().map(c) != 0);
            if covered {
                continue;
            }
            self.requested.push(script);
            let faces = self
                .provider
                .as_mut()
                .map(|p| p(script))
                .unwrap_or_default();
            if faces.is_empty() {
                self.missing.push(script);
            }
            for face in faces {
                added |= self.fonts.add(face).is_ok();
            }
        }
        added
    }
}

impl TextMeasure for TextEngine {
    fn measure(&self, text: &str, style: &TextStyle, max_width: Option<f32>) -> Size {
        self.layout(text, style, max_width, TextAlign::Start).bounds
    }

    fn generation(&self) -> u64 {
        TextEngine::generation(self)
    }
}

/// Picks the face for each character: the current run's face when it has
/// the glyph (spaces and combining marks always stay), else the first
/// candidate that has it, else the primary face.
fn pick_face(
    c: char,
    current: Option<usize>,
    candidates: &[usize],
    covers: impl Fn(usize, char) -> bool,
) -> usize {
    if let Some(face) = current {
        if c.is_whitespace() || c.script() == Script::Inherited || covers(face, c) {
            return face;
        }
    }
    candidates
        .iter()
        .copied()
        .find(|&f| covers(f, c))
        .unwrap_or(candidates[0])
}

fn shape_paragraph(
    ctx: &mut ShapeContext,
    fonts: &FontSet,
    candidates: &[usize],
    size: f32,
    direction: Option<TextDirection>,
    text: &str,
) -> Shaped {
    let mut out = Shaped::default();
    if text.is_empty() || candidates.is_empty() {
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
    let covers = |face: usize, c: char| fonts.faces[face].font().charmap().map(c) != 0;
    // Without a set direction, the paragraph takes it from its first strong letter.
    let base = direction.map(|d| match d {
        TextDirection::Ltr => Level::ltr(),
        TextDirection::Rtl => Level::rtl(),
    });
    let bidi = BidiInfo::new(text, base);
    // Runs that share one face, one embedding level, and one script.
    let mut runs: Vec<Run> = Vec::new();
    for (offset, ch) in text.char_indices() {
        let current = runs.last().map(|r| r.face);
        let face = pick_face(ch, current, candidates, covers);
        let level = bidi.levels[offset].number();
        let script = Some(ch.script())
            .filter(|s| !matches!(s, Script::Common | Script::Inherited | Script::Unknown));
        let end = offset + ch.len_utf8();
        match runs.last_mut() {
            Some(run)
                if run.face == face
                    && run.level == level
                    && (script.is_none() || run.script.is_none() || run.script == script) =>
            {
                run.end = end;
                run.script = run.script.or(script);
            }
            _ => runs.push(Run {
                start: offset,
                end,
                face,
                level,
                script,
            }),
        }
    }
    for Run {
        start,
        end,
        face,
        level,
        script,
    } in runs
    {
        let run = &text[start..end];
        let direction = if level % 2 == 1 {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        };
        let mut shaper = ctx
            .builder(fonts.faces[face].font())
            .script(script.unwrap_or(Script::Latin))
            .direction(direction)
            .size(size)
            .build();
        shaper.add_str(run);
        shaper.shape_with(|cluster| {
            let cs = start + cluster.source.start as usize;
            let ce = (start + cluster.source.end as usize).min(end);
            let first_glyph = out.glyphs.len();
            let mut advance = 0.0;
            for g in cluster.glyphs {
                out.glyphs.push(Glyph {
                    face: face as u16,
                    id: g.id,
                    x: advance + g.x,
                    y: g.y,
                });
                advance += g.advance;
            }
            out.clusters.push(Cluster {
                advance,
                level,
                first_glyph,
                glyph_count: cluster.glyphs.len(),
                whitespace: (cs..ce).all(|i| !text.is_char_boundary(i) || whitespace[i]),
                break_before: breaks[cs],
            });
        });
    }
    out
}

struct Run {
    start: usize,
    end: usize,
    face: usize,
    level: u8,
    script: Option<Script>,
}

/// The clusters of one line in left-to-right display order, as the Unicode
/// bidirectional algorithm reorders them (rule L2). Trailing white space is
/// left out; it takes no room at the line end.
fn visual_order(clusters: &[Cluster], start: usize, end: usize) -> Vec<usize> {
    let mut end = end;
    while end > start && clusters[end - 1].whitespace {
        end -= 1;
    }
    let mut order: Vec<usize> = (start..end).collect();
    let max = order.iter().map(|&i| clusters[i].level).max().unwrap_or(0);
    let lowest_odd = order
        .iter()
        .map(|&i| clusters[i].level)
        .filter(|l| l % 2 == 1)
        .min()
        .unwrap_or(max + 1);
    for level in (lowest_odd..=max).rev() {
        let mut i = 0;
        while i < order.len() {
            if clusters[order[i]].level >= level {
                let from = i;
                while i < order.len() && clusters[order[i]].level >= level {
                    i += 1;
                }
                order[from..i].reverse();
            } else {
                i += 1;
            }
        }
    }
    order
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

impl GlyphBitmap {
    /// Each pixel as a `k` by `k` block, placed relative to the same pen.
    fn upscaled(&self, k: u32) -> GlyphBitmap {
        let (w, h) = (self.width * k, self.height * k);
        let mut data = vec![0; (w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                data[(y * w + x) as usize] = self.data[((y / k) * self.width + x / k) as usize];
            }
        }
        GlyphBitmap {
            left: self.left * k as i32,
            top: self.top * k as i32,
            width: w,
            height: h,
            data,
        }
    }
}

/// Horizontal subpixel positions per pixel. Four keeps text spacing even
/// while bounding the cache.
pub const SUBPIXEL_STEPS: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub face: u16,
    pub id: u16,
    /// Pixel size as `f32` bits.
    pub size: u32,
    /// Quarter-pixel offsets: right in the low four bits, down in the high four.
    pub subpixel: u8,
    /// On a pixel grid, device pixels per cell: coverage is all or nothing,
    /// and above 1 the glyph is drawn at `size` and each pixel scaled up to
    /// a cell. 0 off the grid.
    pub cell: u8,
}

/// A glyph ready to draw: its bitmap and the integer pixel of its pen.
pub struct GlyphDraw {
    pub key: GlyphKey,
    pub bitmap: Rc<GlyphBitmap>,
    pub pen_x: i32,
    pub pen_y: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use astryx_core::FontFamily;

    static REGULAR: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Regular.ttf");
    static MEDIUM: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Medium.ttf");
    static BOLD: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Bold.ttf");
    const HEBREW: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../astryx-fonts/fonts/NotoSansHebrew-Regular.ttf"
    );

    fn roboto() -> FontSet {
        FontSet::new()
            .with(REGULAR)
            .unwrap()
            .with(MEDIUM)
            .unwrap()
            .with(BOLD)
            .unwrap()
    }

    fn engine() -> TextEngine {
        TextEngine::new(roboto())
    }

    fn style(size: f32, weight: u16) -> TextStyle {
        TextStyle {
            direction: None,
            family: FontFamily::Body,
            size,
            weight,
            leading: 1.5,
        }
    }

    #[test]
    fn faces_group_into_one_family() {
        let set = roboto();
        assert_eq!(set.families().collect::<Vec<_>>(), vec!["Roboto"]);
        assert_eq!(set.candidates(400, &[]).len(), 1);
    }

    #[test]
    fn weights_match_like_css() {
        let faces = [(0, 400), (1, 500), (2, 700)];
        let pick = |w| match_weight(faces.iter().copied(), w).unwrap();
        assert_eq!(pick(400), 0);
        assert_eq!(pick(500), 1);
        assert_eq!(pick(600), 2);
        assert_eq!(pick(900), 2);
        assert_eq!(pick(300), 0);
        assert_eq!(pick(450), 1);
        assert!(match_weight(std::iter::empty(), 400).is_none());
    }

    #[test]
    fn fallback_uses_the_first_face_with_the_glyph() {
        // Face 0 has only ASCII letters, face 1 has everything.
        let covers = |f: usize, c: char| f == 1 || c.is_ascii_alphabetic();
        assert_eq!(pick_face('a', None, &[0, 1], covers), 0);
        assert_eq!(pick_face('Ж', Some(0), &[0, 1], covers), 1);
        assert_eq!(pick_face(' ', Some(1), &[0, 1], covers), 1);
        // A combining mark stays with its base character's face.
        assert_eq!(pick_face('\u{301}', Some(0), &[0, 1], covers), 0);
        // Nothing covers it: the primary face draws its missing-glyph box.
        assert_eq!(pick_face('Ж', None, &[0], covers), 0);
    }

    #[test]
    fn cyrillic_greek_and_vietnamese_have_real_glyphs() {
        let layout = engine().layout(
            "Привет мир · Γειά σου · Tiếng Việt",
            &style(16.0, 400),
            None,
            TextAlign::Start,
        );
        let glyphs = &layout.lines[0].glyphs;
        assert!(!glyphs.is_empty());
        assert!(glyphs.iter().all(|g| g.id != 0), "missing glyphs");
    }

    #[test]
    fn layout_updates_when_a_face_arrives_later() {
        let engine = TextEngine::default();
        let st = style(16.0, 400);
        let before = engine.measure("Hello", &st, None);
        assert_eq!(
            before,
            Size {
                width: 0.0,
                height: 24.0
            }
        );
        let measurer: &dyn TextMeasure = &engine;
        let generation = measurer.generation();
        engine.add_fallback_font(REGULAR.to_vec()).unwrap();
        assert_ne!(measurer.generation(), generation);
        let after = engine.measure("Hello", &st, None);
        assert!(after.width > 30.0, "{after:?}");
        assert!(engine.add_fallback_font(vec![1, 2, 3]).is_err());
    }

    #[test]
    fn line_height_is_size_times_leading() {
        let engine = engine();
        let s = engine.measure("Hello", &style(16.0, 400), None);
        assert_eq!(s.height, 24.0);
        assert!(s.width > 30.0 && s.width < 45.0, "{s:?}");
        let two = engine.measure("Hello\nWorld", &style(16.0, 400), None);
        assert_eq!(two.height, 48.0);
    }

    #[test]
    fn empty_text_has_no_size() {
        assert_eq!(
            engine().measure("", &style(16.0, 400), None),
            Size::default()
        );
    }

    #[test]
    fn wraps_at_width_and_never_wraps_at_its_own_width() {
        let engine = engine();
        let st = style(14.0, 400);
        let text = "The quick brown fox jumps over the lazy dog";
        let one = engine.measure(text, &st, None);
        assert_eq!(engine.measure(text, &st, Some(one.width)), one);
        let narrow = engine.measure(text, &st, Some(one.width / 2.5));
        assert!(narrow.height >= 3.0 * 21.0, "{narrow:?}");
        assert!(narrow.width <= one.width / 2.5 + WRAP_EPSILON);
        let layout = engine.layout(text, &st, Some(one.width / 2.5), TextAlign::Start);
        let glyphs: usize = layout.lines.iter().map(|l| l.glyphs.len()).sum();
        let unwrapped: usize = engine.layout(text, &st, None, TextAlign::Start).lines[0]
            .glyphs
            .len();
        // Each break drops the space at the end of its line.
        assert_eq!(glyphs + layout.lines.len() - 1, unwrapped);
    }

    fn shaped_order(engine: &TextEngine, text: &str) -> Vec<(u16, u16)> {
        engine
            .layout(text, &style(16.0, 400), None, TextAlign::Start)
            .lines[0]
            .glyphs
            .iter()
            .map(|g| (g.face, g.id))
            .collect()
    }

    #[test]
    fn right_to_left_runs_display_reversed() {
        let engine = engine();
        engine
            .add_fallback_font(std::fs::read(HEBREW).unwrap())
            .unwrap();
        // Two Hebrew words read right to left, so the second word is drawn first.
        let both = shaped_order(&engine, "שלום עולם");
        let first = shaped_order(&engine, "שלום");
        let second = shaped_order(&engine, "עולם");
        assert_eq!(&both[..second.len()], &second[..]);
        assert_eq!(&both[both.len() - first.len()..], &first[..]);
        // Latin inside Hebrew keeps its own left-to-right order.
        let mixed = shaped_order(&engine, "שלום abc עולם");
        let abc = shaped_order(&engine, "abc");
        let at = mixed.windows(3).position(|w| w == &abc[..]);
        assert!(at.is_some(), "abc stays in order inside right-to-left text");
        // A Hebrew word inside a Latin paragraph flips, the Latin does not.
        let latin = shaped_order(&engine, "abc שלום def");
        assert_eq!(&latin[..3], &abc[..]);
        // "abc", a space, the word, a space, "def".
        assert_eq!(&latin[4..4 + first.len()], &first[..]);
    }

    #[test]
    fn a_set_direction_overrides_the_first_strong_letter() {
        let engine = engine();
        engine
            .add_fallback_font(std::fs::read(HEBREW).unwrap())
            .unwrap();
        let order = |text: &str, direction| {
            let st = TextStyle {
                direction,
                ..style(16.0, 400)
            };
            engine.layout(text, &st, None, TextAlign::Start).lines[0]
                .glyphs
                .iter()
                .map(|g| (g.face, g.id))
                .collect::<Vec<_>>()
        };
        let abc = order("abc", None);
        let word = order("שלום", None);
        // Detected: the Latin word starts the paragraph and stays on the left.
        let auto = order("abc שלום", None);
        assert_eq!(&auto[..3], &abc[..]);
        // Right to left: the Latin word starts the line at the right.
        let rtl = order("abc שלום", Some(TextDirection::Rtl));
        assert_eq!(&rtl[..word.len()], &word[..]);
        assert_eq!(&rtl[rtl.len() - 3..], &abc[..]);
        // Left to right on a Hebrew-first paragraph puts the Latin word last.
        let ltr = order("שלום abc", Some(TextDirection::Ltr));
        assert_eq!(&ltr[..word.len()], &word[..]);
        assert_eq!(&ltr[ltr.len() - 3..], &abc[..]);
        // Measurement agrees with drawing and does not depend on the order.
        let st = |direction| TextStyle {
            direction,
            ..style(16.0, 400)
        };
        let a = engine.measure("abc שלום", &st(None), None);
        let b = engine.measure("abc שלום", &st(Some(TextDirection::Rtl)), None);
        assert!((a.width - b.width).abs() < 0.01 && a.height == b.height);
    }

    #[test]
    fn missing_scripts_are_reported_once_without_a_provider() {
        let engine = engine();
        engine.measure("שלום", &style(16.0, 400), None);
        engine.measure("עולם", &style(16.0, 400), None);
        assert_eq!(engine.take_missing_scripts(), vec![Script::Hebrew]);
        assert!(engine.take_missing_scripts().is_empty());
    }

    #[test]
    fn a_provider_supplies_faces_on_first_use() {
        let engine = engine();
        engine.set_fallback_provider(Box::new(|script| match script {
            Script::Hebrew => vec![std::fs::read(HEBREW).unwrap().into()],
            _ => Vec::new(),
        }));
        let g = shaped_order(&engine, "שלום");
        assert!(g.iter().all(|&(face, id)| face == 3 && id != 0), "{g:?}");
    }

    #[test]
    fn long_words_overflow_like_css() {
        let s = engine().measure("Supercalifragilistic", &style(14.0, 400), Some(10.0));
        assert_eq!(s.height, 21.0);
        assert!(s.width > 10.0);
    }

    #[test]
    fn alignment_offsets_lines() {
        let engine = engine();
        let st = style(14.0, 400);
        let w = engine.measure("Hi", &st, None).width;
        let c = engine.layout("Hi", &st, Some(100.0), TextAlign::Center);
        assert!((c.lines[0].x - (100.0 - w) / 2.0).abs() < 1e-4);
        let e = engine.layout("Hi", &st, Some(100.0), TextAlign::End);
        assert!((e.lines[0].x - (100.0 - w)).abs() < 1e-4);
    }

    #[test]
    fn bolder_text_is_wider() {
        let engine = engine();
        let r = engine.measure("Settings", &style(14.0, 400), None).width;
        let b = engine.measure("Settings", &style(14.0, 700), None).width;
        assert!(b > r);
    }

    #[test]
    fn unsnapped_baselines_move_ink_by_subpixels() {
        let engine = engine();
        let layout = engine.layout("H", &style(16.0, 400), None, TextAlign::Start);
        // Vertical center of ink, in frame pixels.
        let ink_y = |y: f32| {
            let mut out = Vec::new();
            engine.place(&layout, (0.0, y), 2.0, false, &mut out);
            let g = &out[0];
            let (mut sum, mut weight) = (0.0f64, 0.0f64);
            for (i, &a) in g.bitmap.data.iter().enumerate() {
                let row = (i as u32 / g.bitmap.width) as f64;
                sum += (g.pen_y - g.bitmap.top) as f64 * a as f64 + row * a as f64;
                weight += a as f64;
            }
            sum / weight
        };
        let moved = ink_y(0.25) - ink_y(0.0);
        assert!((moved - 0.5).abs() < 0.1, "moved {moved} px");
    }

    #[test]
    fn glyphs_rasterize() {
        let engine = engine();
        let layout = engine.layout("Ag", &style(16.0, 400), None, TextAlign::Start);
        let mut out = Vec::new();
        engine.place(&layout, (0.0, 0.0), 2.0, true, &mut out);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|g| g.bitmap.data.iter().any(|&a| a > 200)));
    }
}
