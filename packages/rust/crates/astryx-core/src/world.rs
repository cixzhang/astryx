//! Experimental: boards, sprites, and sprite sheets for top-down worlds on
//! focus-first devices.
//!
//! A [`Board`] places children at points on a fixed-size surface. They keep
//! their focus stops, accessibility, and `id`, and the arrows move between
//! them by geometry; inside a `paged::CameraFollow`, the view follows focus.
//! A [`Sprite`] draws a registered image with square, unblurred pixels, on
//! whole device pixels (or whole cells under a theme's pixel grid), and can
//! play a strip of frames. [`SpriteSheet`] cuts one image into frames.
//!
//! None of this is an Astryx component yet; names and behavior may change.

use crate::contract::{ImageData, ImageId, Rect, Role};
use crate::element::{
    base_props, Base, Build, BuildCx, Draw, Element, Indicator, Interact, Node, Semantics,
};
use std::sync::Arc;
use taffy::prelude::*;

/// Keys board children by their place in `child_at` order, so sorting them
/// by y never changes their identity.
const BOARD_SLOT: u64 = 0x5b0a_7d00_0000_0000;

/// A board child and its top left corner.
type Placed<Msg> = (f32, f32, Element<Msg>);

/// A fixed-size surface with children at points on it.
pub struct Board<Msg> {
    base: Base,
    width: f32,
    height: f32,
    children: Vec<Placed<Msg>>,
    sort_by_y: bool,
}

impl<Msg> Board<Msg> {
    pub fn new(width: f32, height: f32) -> Self {
        Board {
            base: Base::default(),
            width,
            height,
            children: Vec::new(),
            sort_by_y: false,
        }
    }

    /// Places `child` with its top left corner at (`x`, `y`). A child whose
    /// place changes between views glides there on the theme's motion.
    pub fn child_at(mut self, x: f32, y: f32, child: impl Into<Element<Msg>>) -> Self {
        self.children.push((x, y, child.into()));
        self
    }

    /// Draws children lower on the board later, so they overlap the ones
    /// above them, as a top-down scene does. Their order for Next and
    /// Previous follows.
    pub fn sort_by_y(mut self, on: bool) -> Self {
        self.sort_by_y = on;
        self
    }
}

base_props!(Board<Msg>);

impl<Msg> Build<Msg> for Board<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        cx.key(self.base.id.as_deref());
        cx.begin(Node::new(Style {
            size: Size {
                width: length(self.width),
                height: length(self.height),
            },
            flex_shrink: 0.0,
            ..Style::default()
        }));
        let mut children: Vec<(usize, Placed<Msg>)> =
            self.children.into_iter().enumerate().collect();
        if self.sort_by_y {
            children.sort_by(|a, b| a.1 .1.total_cmp(&b.1 .1));
        }
        for (slot, (x, y, child)) in children {
            cx.key_slot(BOARD_SLOT ^ slot as u64);
            cx.begin(
                Node::new(Style {
                    position: Position::Absolute,
                    inset: taffy::Rect {
                        left: length(x),
                        top: length(y),
                        right: auto(),
                        bottom: auto(),
                    },
                    ..Style::default()
                })
                .moves(),
            );
            cx.child(child);
            cx.end();
        }
        cx.end();
    }
}

/// A registered image drawn with square pixels: no smoothing, on whole
/// device pixels, or whole cells under a theme's pixel grid.
pub struct Sprite<Msg> {
    base: Base,
    image: ImageId,
    width: f32,
    height: f32,
    frames: u32,
    fps: f32,
    phase: f32,
    label: Option<String>,
    on_click: Option<Msg>,
}

impl<Msg> Sprite<Msg> {
    /// A sprite of `image`, 16 points square until [`Sprite::size`] sets it.
    pub fn new(image: ImageId) -> Self {
        Sprite {
            base: Base::default(),
            image,
            width: 16.0,
            height: 16.0,
            frames: 1,
            fps: 0.0,
            phase: 0.0,
            label: None,
            on_click: None,
        }
    }
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
    /// Plays `n` frames at `fps`: the image and the next `n - 1` ids, as a
    /// [`SpriteSheet`] numbers its cells. It holds its first frame under
    /// reduced motion.
    pub fn frames(mut self, n: u32, fps: f32) -> Self {
        self.frames = n.max(1);
        self.fps = fps.max(0.0);
        self
    }
    /// Starts the frame strip `seconds` in, so a field of sprites does not
    /// move in lockstep.
    pub fn phase(mut self, seconds: f32) -> Self {
        self.phase = seconds;
        self
    }
    /// What the sprite shows, for assistive technology.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    /// Makes the sprite a focus stop that sends `msg` when pressed.
    pub fn on_click(mut self, msg: Msg) -> Self {
        self.on_click = Some(msg);
        self
    }
}

base_props!(Sprite<Msg>);

impl<Msg> Build<Msg> for Sprite<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let animated = self.frames > 1 && self.fps > 0.0 && cx.animate();
        let this_phase = self.phase;
        let mut node = Node::new(Style {
            size: Size {
                width: length(self.width),
                height: length(self.height),
            },
            flex_shrink: 0.0,
            ..Style::default()
        })
        .draw(Draw::Sprite(SpriteDraw {
            first: self.image,
            frames: if animated { self.frames } else { 1 },
            fps: self.fps,
            phase: this_phase,
        }));
        let this = *self;
        let label = this.label.unwrap_or_default();
        node = match this.on_click {
            Some(msg) => node
                .interact(Interact::new(Some(msg), false))
                .indicator(Indicator::ring(
                    t.color.accent.into(),
                    Default::default(),
                    None,
                ))
                .sem(Semantics::new(Role::Button, label)),
            None if !label.is_empty() => node.sem(Semantics::new(Role::Image, label)),
            None => node,
        };
        cx.key(this.base.id.as_deref());
        cx.leaf(node);
    }
}

/// A sprite's image and frame strip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SpriteDraw {
    pub first: ImageId,
    pub frames: u32,
    pub fps: f32,
    /// Seconds into the strip at time zero, so sprites need not move in step.
    pub phase: f32,
}

impl SpriteDraw {
    /// The frame showing at `time` seconds.
    pub fn image_at(&self, time: f32) -> ImageId {
        if self.frames <= 1 {
            return self.first;
        }
        let frame =
            ((time + self.phase) * self.fps).floor().max(0.0) as u64 % u64::from(self.frames);
        ImageId(self.first.0.wrapping_add(frame))
    }
}

/// Cuts one image into sub-images registered under consecutive ids, so a
/// [`Sprite`] can play them as frames.
///
/// ```
/// use astryx_core::world::SpriteSheet;
/// use astryx_core::{ImageData, ImageId};
/// let strip = ImageData { width: 32, height: 16, rgba: vec![0; 32 * 16 * 4].into() };
/// let sheet = SpriteSheet::grid(ImageId::of("walk"), &strip, 16, 16);
/// assert_eq!(sheet.len(), 2);
/// assert_eq!(sheet.cell(1).0, sheet.cell(0).0 + 1);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct SpriteSheet {
    first: ImageId,
    cells: Vec<ImageData>,
}

impl SpriteSheet {
    /// Cells of `width` x `height` pixels, row by row from the top left.
    pub fn grid(id: ImageId, image: &ImageData, width: u32, height: u32) -> SpriteSheet {
        let (w, h) = (width.max(1), height.max(1));
        let mut rects = Vec::new();
        for row in 0..image.height / h {
            for col in 0..image.width / w {
                rects.push(Rect::new(
                    (col * w) as f32,
                    (row * h) as f32,
                    w as f32,
                    h as f32,
                ));
            }
        }
        SpriteSheet::regions(id, image, &rects)
    }

    /// The given pixel rects of `image`, in order. Parts outside it are clear.
    pub fn regions(id: ImageId, image: &ImageData, rects: &[Rect]) -> SpriteSheet {
        let cells = rects.iter().map(|r| crop(image, *r)).collect();
        SpriteSheet { first: id, cells }
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// The id of cell `i`.
    pub fn cell(&self, i: usize) -> ImageId {
        ImageId(self.first.0.wrapping_add(i as u64))
    }

    /// Every cell with its id, for `Runtime::register_image`.
    pub fn images(&self) -> Vec<(ImageId, Option<ImageData>)> {
        self.cells
            .iter()
            .enumerate()
            .map(|(i, data)| (self.cell(i), Some(data.clone())))
            .collect()
    }
}

fn crop(image: &ImageData, r: Rect) -> ImageData {
    let (x0, y0, w, h) = (
        r.x as i64,
        r.y as i64,
        r.width.max(0.0) as u32,
        r.height.max(0.0) as u32,
    );
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h as i64 {
        for x in 0..w as i64 {
            let (sx, sy) = (x0 + x, y0 + y);
            if sx < 0 || sy < 0 || sx >= image.width as i64 || sy >= image.height as i64 {
                continue;
            }
            let from = ((sy as usize) * image.width as usize + sx as usize) * 4;
            let to = ((y as usize) * w as usize + x as usize) * 4;
            rgba[to..to + 4].copy_from_slice(&image.rgba[from..from + 4]);
        }
    }
    ImageData {
        width: w,
        height: h,
        rgba: Arc::from(rgba),
    }
}

/// A grid of tiles from one [`SpriteSheet`], drawn as one baked image plus
/// a sprite for each animated or decorative tile, instead of one draw per
/// tile. [`TileMap::from_terrain`] picks the tiles from a [`TerrainGrid`]
/// so terrains meet with soft edges.
///
/// ```
/// use astryx_core::world::{SpriteSheet, TileMap};
/// use astryx_core::{ImageData, ImageId};
/// let strip = ImageData { width: 48, height: 16, rgba: vec![255; 48 * 16 * 4].into() };
/// let sheet = SpriteSheet::grid(ImageId::of("tiles"), &strip, 16, 16);
/// // Two water frames start at cell 1; cell 0 is grass.
/// let map = TileMap::new(3, 2, [Some(0), Some(1), Some(0), None, Some(0), Some(1)]).animate(1, 2, 2.0);
/// let (id, image) = map.bake(&sheet, ImageId::of("room/ground"));
/// assert_eq!(image.as_ref().map(|i| (i.width, i.height)), Some((48, 32)));
/// let layer = map.layer::<()>(id, &sheet, 32.0);
/// # let _ = layer;
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct TileMap {
    columns: u32,
    rows: u32,
    layers: Vec<TileLayer>,
    animations: Vec<(u32, u32, f32)>,
    decorations: Vec<Decoration>,
    terrain: Option<(TerrainGrid, Autotile)>,
    /// For a map from [`TileMap::autotile`]: its first tile.
    corner_first: Option<u32>,
}

/// One layer of tiles, drawn in order.
#[derive(Clone, Debug, PartialEq)]
struct TileLayer {
    tiles: Vec<Option<u32>>,
    /// Tiles sit on cell corners: one more column and row than the map,
    /// shifted up and left by half a tile (a corner set's dual grid).
    corners: bool,
}

impl TileLayer {
    fn size(&self, columns: u32, rows: u32) -> (u32, u32) {
        if self.corners {
            (columns + 1, rows + 1)
        } else {
            (columns, rows)
        }
    }
}

/// A tile drawn over the map that plays its frames from its own phase.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Decoration {
    x: u32,
    y: u32,
    first: u32,
    frames: u32,
    fps: f32,
    phase: f32,
}

impl TileMap {
    /// `columns` x `rows` cells, row by row: each a sheet cell index, or
    /// `None` for no tile. Missing cells are empty.
    pub fn new(columns: u32, rows: u32, tiles: impl IntoIterator<Item = Option<u32>>) -> TileMap {
        let mut tiles: Vec<Option<u32>> = tiles.into_iter().collect();
        tiles.resize((columns * rows) as usize, None);
        TileMap {
            columns,
            rows,
            layers: vec![TileLayer {
                tiles,
                corners: false,
            }],
            animations: Vec::new(),
            decorations: Vec::new(),
            terrain: None,
            corner_first: None,
        }
    }

    /// A two-terrain map from a corner set of 16 opaque tiles, one per mask
    /// in index order: tile (`x`, `y`) reads the corners (x, y), (x+1, y),
    /// (x, y+1), (x+1, y+1) of a `columns + 1` by `rows + 1` corner grid,
    /// where `corner` tells whether a corner holds the upper terrain, and
    /// shows `first + mask` with top left 1, top right 2, bottom left 4,
    /// bottom right 8. Mask 0 is all lower terrain, mask 15 all upper.
    pub fn autotile(
        columns: u32,
        rows: u32,
        corner: impl Fn(i32, i32) -> bool,
        first: u32,
    ) -> TileMap {
        let tiles = (0..rows as i32)
            .flat_map(|y| (0..columns as i32).map(move |x| (x, y)))
            .map(|(x, y)| {
                let mask = corner(x, y) as u32
                    | (corner(x + 1, y) as u32) << 1
                    | (corner(x, y + 1) as u32) << 2
                    | (corner(x + 1, y + 1) as u32) << 3;
                Some(first + mask)
            });
        let mut map = TileMap::new(columns, rows, tiles.collect::<Vec<_>>());
        map.corner_first = Some(first);
        map
    }

    /// For a map from [`TileMap::autotile`]: cells all lower terrain (mask 0)
    /// show their tile or one of `lower`, and cells all upper (mask 15) one
    /// of `upper`, picked by a hash of the cell, the same every time.
    pub fn full_variants(mut self, lower: &[u32], upper: &[u32]) -> TileMap {
        let Some(first) = self.corner_first else {
            return self;
        };
        let columns = self.columns;
        for (i, tile) in self.layers[0].tiles.iter_mut().enumerate() {
            let alternatives = match *tile {
                Some(t) if t == first => lower,
                Some(t) if t == first + 15 => upper,
                _ => continue,
            };
            let (x, y) = (i as u32 % columns, i as u32 / columns);
            let pick = cell_hash(x, y, 0xf011) % (alternatives.len() as u64 + 1);
            if pick > 0 {
                *tile = Some(alternatives[pick as usize - 1]);
            }
        }
        self
    }

    /// The tiles `autotile` picks for `grid`: its base terrain everywhere,
    /// then each terrain above it with edges that blend into the ones below.
    pub fn from_terrain(grid: &TerrainGrid, autotile: &Autotile) -> TileMap {
        let mut map = TileMap {
            columns: grid.columns,
            rows: grid.rows,
            layers: Vec::new(),
            animations: Vec::new(),
            decorations: Vec::new(),
            terrain: Some((grid.clone(), autotile.clone())),
            corner_first: None,
        };
        let (c, r) = (grid.columns, grid.rows);
        map.layers.push(TileLayer {
            tiles: (0..r)
                .flat_map(|y| (0..c).map(move |x| (x, y)))
                .map(|(x, y)| Some(autotile.base_tile(x, y)))
                .collect(),
            corners: false,
        });
        for (k, set) in autotile.sets.iter().enumerate() {
            let corners = set.layout == TileLayout::Corners16;
            let layer = TileLayer {
                tiles: Vec::new(),
                corners,
            };
            let (w, h) = layer.size(c, r);
            map.layers.push(layer);
            let tiles = (0..h)
                .flat_map(|y| (0..w).map(move |x| (x, y)))
                .map(|(x, y)| autotile.pick(grid, k, x, y))
                .collect();
            map.layers.last_mut().expect("pushed").tiles = tiles;
        }
        map
    }

    /// Changes one cell's terrain and picks tiles again around it only: its
    /// 3 x 3 neighborhood. Returns the (layer, x, y) of tiles that changed.
    /// Maps not built from terrain change nothing.
    pub fn set_terrain(&mut self, x: u32, y: u32, terrain: u8) -> Vec<(usize, u32, u32)> {
        let Some((grid, autotile)) = self.terrain.as_mut() else {
            return Vec::new();
        };
        if x >= grid.columns || y >= grid.rows || grid.get(x, y) == terrain {
            return Vec::new();
        }
        grid.set(x, y, terrain);
        let (grid, autotile) = (grid.clone(), autotile.clone());
        let mut changed = Vec::new();
        for (k, layer) in self.layers.iter_mut().enumerate().skip(1) {
            let (w, h) = layer.size(grid.columns, grid.rows);
            // A corner tile at (i, j) reads cells (i-1..=i, j-1..=j); a cell
            // tile reads its 8 neighbors.
            let (x0, y0, x1, y1) = if layer.corners {
                (x, y, x + 1, y + 1)
            } else {
                (x.saturating_sub(1), y.saturating_sub(1), x + 1, y + 1)
            };
            for ty in y0..=y1.min(h - 1) {
                for tx in x0..=x1.min(w - 1) {
                    let tile = autotile.pick(&grid, k - 1, tx, ty);
                    let slot = &mut layer.tiles[(ty * w + tx) as usize];
                    if *slot != tile {
                        *slot = tile;
                        changed.push((k, tx, ty));
                    }
                }
            }
        }
        changed
    }

    /// The terrain the map was built from, if any.
    pub fn terrain(&self) -> Option<&TerrainGrid> {
        self.terrain.as_ref().map(|t| &t.0)
    }

    /// Tiles of sheet cell `first` play cells `first` to `first + frames - 1`
    /// at `fps`, held on the first under reduced motion.
    pub fn animate(mut self, first: u32, frames: u32, fps: f32) -> TileMap {
        self.animations.retain(|a| a.0 != first);
        self.animations.push((first, frames.max(1), fps));
        self
    }

    /// Scatters decorations, such as swaying grass and flowers, over cells
    /// of `terrain` (every cell for a map not built from terrain): about
    /// `percent` of them, each one of `tiles` (each the first of `frames`
    /// cells), playing at `fps` from its own phase, so a field ripples
    /// instead of moving in step. The choice comes from a hash of the cell,
    /// so it is the same every time. Held still under reduced motion.
    pub fn scatter(
        mut self,
        terrain: u8,
        tiles: &[u32],
        percent: u32,
        frames: u32,
        fps: f32,
    ) -> TileMap {
        if tiles.is_empty() {
            return self;
        }
        let cycle = frames.max(1) as f32 / fps.max(0.001);
        for y in 0..self.rows {
            for x in 0..self.columns {
                let on = self
                    .terrain
                    .as_ref()
                    .is_none_or(|(g, _)| g.get(x, y) == terrain);
                let h = cell_hash(x, y, 0xdec0);
                if !on || h % 100 >= u64::from(percent) {
                    continue;
                }
                let pick = (cell_hash(x, y, 0x711e) % tiles.len() as u64) as usize;
                let phase = (cell_hash(x, y, 0xfa5e) % 1000) as f32 / 1000.0 * cycle;
                self.decorations.push(Decoration {
                    x,
                    y,
                    first: tiles[pick],
                    frames: frames.max(1),
                    fps,
                    phase,
                });
            }
        }
        self
    }

    /// Re-times the decorations as a wave: each column lags the one to its
    /// left by `radians` of the sway's cycle, so a ripple crosses the field
    /// left to right.
    pub fn wave(mut self, radians: f32) -> TileMap {
        for d in &mut self.decorations {
            let cycle = d.frames as f32 / d.fps.max(0.001);
            let lag = d.x as f32 * radians / std::f32::consts::TAU;
            d.phase = (-lag).rem_euclid(1.0) * cycle;
        }
        self
    }

    fn animation(&self, cell: u32) -> Option<(u32, f32)> {
        self.animations
            .iter()
            .find(|a| a.0 == cell)
            .map(|a| (a.1, a.2))
    }

    /// The tile at (`x`, `y`) of layer `layer`; layer 0 is the base.
    pub fn tile(&self, layer: usize, x: u32, y: u32) -> Option<u32> {
        let l = self.layers.get(layer)?;
        let (w, h) = l.size(self.columns, self.rows);
        (x < w && y < h)
            .then(|| l.tiles[(y * w + x) as usize])
            .flatten()
    }

    /// The still tiles of every layer drawn into one image, registered as
    /// `id`; animated tiles and decorations are left for [`TileMap::layer`].
    pub fn bake(&self, sheet: &SpriteSheet, id: ImageId) -> (ImageId, Option<ImageData>) {
        let (tw, th) = sheet.cells.first().map_or((0, 0), |c| (c.width, c.height));
        let (w, h) = (self.columns * tw, self.rows * th);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for layer in &self.layers {
            let (lw, _) = layer.size(self.columns, self.rows);
            // Corner tiles sit half a tile up and left of their cells.
            let (ox, oy) = if layer.corners {
                (tw as i64 / 2, th as i64 / 2)
            } else {
                (0, 0)
            };
            for (i, tile) in layer.tiles.iter().enumerate() {
                let Some(cell) = tile.filter(|&c| self.animation(c).is_none()) else {
                    continue;
                };
                let Some(src) = sheet.cells.get(cell as usize) else {
                    continue;
                };
                let (cx, cy) = (
                    (i as u32 % lw * tw) as i64 - ox,
                    (i as u32 / lw * th) as i64 - oy,
                );
                blend(&mut rgba, w, h, src, cx, cy);
            }
        }
        (
            id,
            Some(ImageData {
                width: w,
                height: h,
                rgba: Arc::from(rgba),
            }),
        )
    }

    /// The map at `tile` points per tile: the baked image `baked`, with a
    /// playing sprite over each animated tile and each decoration.
    pub fn layer<Msg: Clone + 'static>(
        &self,
        baked: ImageId,
        sheet: &SpriteSheet,
        tile: f32,
    ) -> Element<Msg> {
        let (w, h) = (self.columns as f32 * tile, self.rows as f32 * tile);
        let mut board = Board::new(w, h).child_at(0.0, 0.0, Sprite::new(baked).size(w, h));
        for layer in self.layers.iter().filter(|l| !l.corners) {
            for (i, t) in layer.tiles.iter().enumerate() {
                let Some((cell, (frames, fps))) = t.and_then(|c| Some((c, self.animation(c)?)))
                else {
                    continue;
                };
                let (x, y) = (
                    (i as u32 % self.columns) as f32 * tile,
                    (i as u32 / self.columns) as f32 * tile,
                );
                board = board.child_at(
                    x,
                    y,
                    Sprite::new(sheet.cell(cell as usize))
                        .size(tile, tile)
                        .frames(frames, fps),
                );
            }
        }
        for d in &self.decorations {
            board = board.child_at(
                d.x as f32 * tile,
                d.y as f32 * tile,
                Sprite::new(sheet.cell(d.first as usize))
                    .size(tile, tile)
                    .frames(d.frames, d.fps)
                    .phase(d.phase),
            );
        }
        board.into()
    }
}

/// Draws `src` over `dst` at (`x`, `y`), alpha over, clipped to `dst`.
fn blend(dst: &mut [u8], w: u32, h: u32, src: &ImageData, x: i64, y: i64) {
    for sy in 0..src.height as i64 {
        let dy = y + sy;
        if dy < 0 || dy >= h as i64 {
            continue;
        }
        for sx in 0..src.width as i64 {
            let dx = x + sx;
            if dx < 0 || dx >= w as i64 {
                continue;
            }
            let s = ((sy * src.width as i64 + sx) * 4) as usize;
            let d = ((dy * w as i64 + dx) * 4) as usize;
            let a = src.rgba[s + 3] as u32;
            if a == 0 {
                continue;
            }
            for c in 0..3 {
                let v = (src.rgba[s + c] as u32 * a + dst[d + c] as u32 * (255 - a)) / 255;
                dst[d + c] = v as u8;
            }
            dst[d + 3] = (a + dst[d + 3] as u32 * (255 - a) / 255) as u8;
        }
    }
}

/// A stable hash of a cell, for choices that must not change between runs.
fn cell_hash(x: u32, y: u32, salt: u64) -> u64 {
    let mut z = (u64::from(x) << 32 | u64::from(y)) ^ salt.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Terrain ids, one per cell, row by row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainGrid {
    columns: u32,
    rows: u32,
    cells: Vec<u8>,
}

impl TerrainGrid {
    pub fn new(columns: u32, rows: u32, fill: u8) -> TerrainGrid {
        TerrainGrid {
            columns,
            rows,
            cells: vec![fill; (columns * rows) as usize],
        }
    }

    /// One row per string, each character mapped to a terrain id by `id`.
    pub fn from_rows(rows: &[&str], id: impl Fn(char) -> u8) -> TerrainGrid {
        let columns = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0) as u32;
        let mut grid = TerrainGrid::new(columns, rows.len() as u32, 0);
        for (y, row) in rows.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                grid.set(x as u32, y as u32, id(c));
            }
        }
        grid
    }

    pub fn columns(&self) -> u32 {
        self.columns
    }

    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// The terrain at (`x`, `y`); outside the grid, the nearest edge cell's.
    pub fn get(&self, x: u32, y: u32) -> u8 {
        let (x, y) = (x.min(self.columns - 1), y.min(self.rows - 1));
        self.cells[(y * self.columns + x) as usize]
    }

    pub fn set(&mut self, x: u32, y: u32, terrain: u8) {
        if x < self.columns && y < self.rows {
            self.cells[(y * self.columns + x) as usize] = terrain;
        }
    }

    /// The terrain at a signed position, clamped to the grid.
    fn at(&self, x: i64, y: i64) -> u8 {
        self.get(x.max(0) as u32, y.max(0) as u32)
    }
}

/// How a terrain's edge tiles are laid out in its sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileLayout {
    /// 16 tiles on cell corners (Wang corner tiles, marching squares),
    /// indexed by which of the four cells around the corner hold the
    /// terrain: northwest 1, northeast 2, southwest 4, southeast 8. Tile 0
    /// is never drawn. Corner tiles sit half a tile up and left of cells,
    /// on one more column and row.
    Corners16,
    /// 47 tiles on cells (the blob set), for cells that hold the terrain,
    /// picked from the 8 neighbors: north 1, northeast 2, east 4, southeast
    /// 8, south 16, southwest 32, west 64, northwest 128, with a corner kept
    /// only when both sides next to it are set. The 47 masks that remain go
    /// in ascending order, 7 tiles to a row.
    Blob47,
}

impl TileLayout {
    /// Tiles in one copy of the layout.
    pub fn len(self) -> u32 {
        match self {
            TileLayout::Corners16 => 16,
            TileLayout::Blob47 => 47,
        }
    }

    pub fn is_empty(self) -> bool {
        false
    }
}

/// The 47 blob masks in ascending order.
fn blob_masks() -> &'static [u8; 47] {
    static MASKS: std::sync::OnceLock<[u8; 47]> = std::sync::OnceLock::new();
    MASKS.get_or_init(|| {
        let mut out = [0u8; 47];
        let mut n = 0;
        for m in 0..=255u8 {
            if reduce_blob(m) == m {
                out[n] = m;
                n += 1;
            }
        }
        out
    })
}

/// Drops each corner bit whose two neighboring sides are not both set.
fn reduce_blob(m: u8) -> u8 {
    let (n, e, s, w) = (m & 1 != 0, m & 4 != 0, m & 16 != 0, m & 64 != 0);
    let mut out = m & (1 | 4 | 16 | 64);
    for (bit, a, b) in [(2, n, e), (8, s, e), (32, s, w), (128, n, w)] {
        if m & bit != 0 && a && b {
            out |= bit;
        }
    }
    out
}

/// One terrain drawn over the terrains below it.
#[derive(Clone, Debug, PartialEq)]
struct TerrainSet {
    terrain: u8,
    layout: TileLayout,
    first: u32,
    variants: u32,
}

/// Rules that turn a [`TerrainGrid`] into tiles: a base terrain, then
/// terrains in order, each drawn with transparent soft edges over the ones
/// before it. Each terrain's tiles are the edge set between it and the
/// terrain before it, so its sheet holds that pair's look.
#[derive(Clone, Debug, PartialEq)]
pub struct Autotile {
    base: u8,
    base_tile: u32,
    base_variants: u32,
    sets: Vec<TerrainSet>,
}

impl Autotile {
    /// `base` covers every cell with sheet cell `tile`.
    pub fn new(base: u8, tile: u32) -> Autotile {
        Autotile {
            base,
            base_tile: tile,
            base_variants: 1,
            sets: Vec::new(),
        }
    }

    /// Adds `terrain` over the terrains so far, with its edge tiles from
    /// sheet cell `first` in `layout`.
    pub fn over(mut self, terrain: u8, layout: TileLayout, first: u32) -> Autotile {
        self.sets.push(TerrainSet {
            terrain,
            layout,
            first,
            variants: 1,
        });
        self
    }

    /// The last terrain added (or the base) has `n` interchangeable copies
    /// of its tiles, one after another in the sheet; each cell picks one by
    /// a hash of its place, the same every time.
    pub fn variants(mut self, n: u32) -> Autotile {
        match self.sets.last_mut() {
            Some(set) => set.variants = n.max(1),
            None => self.base_variants = n.max(1),
        }
        self
    }

    fn base_tile(&self, x: u32, y: u32) -> u32 {
        self.base_tile + (cell_hash(x, y, 0xba5e) % u64::from(self.base_variants)) as u32
    }

    /// Whether a cell's terrain is set `k` or one drawn after it.
    fn covers(&self, k: usize, terrain: u8) -> bool {
        self.sets[k..].iter().any(|s| s.terrain == terrain)
    }

    /// The tile of set `k` at tile position (`x`, `y`) of its layer.
    fn pick(&self, grid: &TerrainGrid, k: usize, x: u32, y: u32) -> Option<u32> {
        let set = &self.sets[k];
        let on = |dx: i64, dy: i64| self.covers(k, grid.at(x as i64 + dx, y as i64 + dy));
        let index = match set.layout {
            TileLayout::Corners16 => {
                // The four cells around the corner at the tile's center.
                let bits = on(-1, -1) as u32
                    | (on(0, -1) as u32) << 1
                    | (on(-1, 0) as u32) << 2
                    | (on(0, 0) as u32) << 3;
                if bits == 0 {
                    return None;
                }
                bits
            }
            TileLayout::Blob47 => {
                if !on(0, 0) {
                    return None;
                }
                let around = [
                    (0, -1),
                    (1, -1),
                    (1, 0),
                    (1, 1),
                    (0, 1),
                    (-1, 1),
                    (-1, 0),
                    (-1, -1),
                ];
                let mask =
                    around.iter().enumerate().fold(
                        0u8,
                        |m, (i, &(dx, dy))| if on(dx, dy) { m | 1 << i } else { m },
                    );
                let mask = reduce_blob(mask);
                blob_masks()
                    .iter()
                    .position(|&m| m == mask)
                    .expect("a reduced mask") as u32
            }
        };
        let variant = (cell_hash(x, y, 0x5ee0 + k as u64) % u64::from(set.variants)) as u32;
        Some(set.first + variant * set.layout.len() + index)
    }
}
