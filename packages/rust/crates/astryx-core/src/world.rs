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
}

impl SpriteDraw {
    /// The frame showing at `time` seconds.
    pub fn image_at(&self, time: f32) -> ImageId {
        if self.frames <= 1 {
            return self.first;
        }
        let frame = (time * self.fps).floor() as u64 % u64::from(self.frames);
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
/// a sprite for each animated tile, instead of one draw per tile.
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
    tiles: Vec<Option<u32>>,
    animations: Vec<(u32, u32, f32)>,
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
            tiles,
            animations: Vec::new(),
        }
    }

    /// Tiles of sheet cell `first` play cells `first` to `first + frames - 1`
    /// at `fps`, held on the first under reduced motion.
    pub fn animate(mut self, first: u32, frames: u32, fps: f32) -> TileMap {
        self.animations.retain(|a| a.0 != first);
        self.animations.push((first, frames.max(1), fps));
        self
    }

    fn animation(&self, cell: u32) -> Option<(u32, f32)> {
        self.animations
            .iter()
            .find(|a| a.0 == cell)
            .map(|a| (a.1, a.2))
    }

    /// The still tiles drawn into one image, registered as `id`; animated
    /// tiles are left out for [`TileMap::layer`] to draw.
    pub fn bake(&self, sheet: &SpriteSheet, id: ImageId) -> (ImageId, Option<ImageData>) {
        let (tw, th) = sheet.cells.first().map_or((0, 0), |c| (c.width, c.height));
        let (w, h) = (self.columns * tw, self.rows * th);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for (i, tile) in self.tiles.iter().enumerate() {
            let Some(cell) = tile.filter(|&c| self.animation(c).is_none()) else {
                continue;
            };
            let Some(src) = sheet.cells.get(cell as usize) else {
                continue;
            };
            let (cx, cy) = (i as u32 % self.columns * tw, i as u32 / self.columns * th);
            for y in 0..th.min(src.height) {
                let from = (y * src.width * 4) as usize;
                let to = (((cy + y) * w + cx) * 4) as usize;
                let n = (tw.min(src.width) * 4) as usize;
                rgba[to..to + n].copy_from_slice(&src.rgba[from..from + n]);
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
    /// playing sprite over each animated tile.
    pub fn layer<Msg: Clone + 'static>(
        &self,
        baked: ImageId,
        sheet: &SpriteSheet,
        tile: f32,
    ) -> Element<Msg> {
        let (w, h) = (self.columns as f32 * tile, self.rows as f32 * tile);
        let mut board = Board::new(w, h).child_at(0.0, 0.0, Sprite::new(baked).size(w, h));
        for (i, t) in self.tiles.iter().enumerate() {
            let Some((cell, (frames, fps))) = t.and_then(|c| Some((c, self.animation(c)?))) else {
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
        board.into()
    }
}
