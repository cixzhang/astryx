//! The World section: a small top-down board of pixel sprites under a
//! camera that follows focus, with a walking character that goes to the
//! focused thing. Uses the experimental `world` and `paged` modules.

use crate::Msg;
use astryx_core::paged::CameraFollow;
use astryx_core::world::{Board, Sprite, SpriteSheet, TileMap};
use astryx_core::{Element, ImageData, ImageId};
use std::sync::OnceLock;

/// The ground's tiles: grass, path, and two frames of water.
const TILES: ImageId = ImageId::of("gallery/world/tiles.png");
/// The ground's still tiles, baked into one image.
const GROUND: ImageId = ImageId::of("gallery/world/ground.png");
const GRASS_TILE: u32 = 0;
const PATH_TILE: u32 = 1;
const WATER_TILE: u32 = 2;
const TREE: ImageId = ImageId::of("gallery/world/tree.png");
const ROCK: ImageId = ImageId::of("gallery/world/rock.png");
const FLOWER: ImageId = ImageId::of("gallery/world/flower.png");
/// The first of the hero's four walking frames.
const HERO: ImageId = ImageId::of("gallery/world/hero.png");

/// One art pixel, in points.
const PX: f32 = 4.0;
/// One tile: 16 art pixels.
const TILE: f32 = 16.0 * PX;
const COLS: usize = 15;
const ROWS: usize = 11;

/// The things on the board, by id, kind, and tile.
const THINGS: [(&str, &str, usize, usize); 12] = [
    ("world-oak", "Oak tree", 2, 1),
    ("world-pine", "Pine tree", 6, 2),
    ("world-birch", "Birch tree", 11, 1),
    ("world-willow", "Willow tree", 13, 6),
    ("world-elm", "Elm tree", 4, 8),
    ("world-boulder", "Boulder", 9, 4),
    ("world-pebble", "Pebble", 1, 5),
    ("world-stone", "Standing stone", 12, 9),
    ("world-daisy", "Daisy", 4, 4),
    ("world-poppy", "Poppy", 7, 7),
    ("world-tulip", "Tulip", 10, 9),
    ("world-violet", "Violet", 13, 3),
];

/// The board, with the hero standing beside the thing `at` names.
pub fn world(at: Option<&str>) -> Element<Msg> {
    let (width, height) = (COLS as f32 * TILE, ROWS as f32 * TILE);
    // The ground is its own layer, one baked image and the moving water, so
    // the things and the hero, sorted top down among themselves, stand on it.
    let ground = ground_map().layer(GROUND, tiles(), TILE);
    let mut board = Board::new(width, height).sort_by_y(true);
    for (id, label, col, row) in THINGS {
        let image = match id {
            "world-boulder" | "world-pebble" | "world-stone" => ROCK,
            "world-daisy" | "world-poppy" | "world-tulip" | "world-violet" => FLOWER,
            _ => TREE,
        };
        board = board.child_at(
            col as f32 * TILE,
            row as f32 * TILE,
            Sprite::new(image)
                .size(TILE, TILE)
                .label(label)
                .on_click(Msg::Pressed(id))
                .id(id),
        );
    }
    // The hero stands just below and left of the focused thing.
    let (col, row) = THINGS
        .iter()
        .find(|t| Some(t.0) == at)
        .map_or((7, 5), |t| (t.2, t.3));
    board = board.child_at(
        col as f32 * TILE - TILE / 2.0,
        row as f32 * TILE + TILE / 2.0,
        Sprite::new(HERO)
            .size(TILE, TILE)
            .frames(4, 6.0)
            .label("Hero"),
    );
    CameraFollow::new(
        Board::new(width, height)
            .child_at(0.0, 0.0, ground)
            .child_at(0.0, 0.0, board),
    )
    .into()
}

/// The world's images: its tiles and the hero's frames.
pub fn images() -> Vec<(ImageId, Option<ImageData>)> {
    let mut out = vec![
        ground_map().bake(tiles(), GROUND),
        (TREE, Some(art(&TREE_ART))),
        (ROCK, Some(art(&ROCK_ART))),
        (FLOWER, Some(art(&FLOWER_ART))),
    ];
    // Four walking frames side by side: stand, step, stand, other step.
    let strip: Vec<String> = (0..16)
        .map(|y| [HERO_A[y], HERO_B[y], HERO_A[y], HERO_C[y]].concat())
        .collect();
    let sheet = SpriteSheet::grid(HERO, &art(&strip), 16, 16);
    out.extend(sheet.images());
    out.extend(tiles().images());
    out
}

/// The ground's tile sheet, cut once.
fn tiles() -> &'static SpriteSheet {
    static SHEET: OnceLock<SpriteSheet> = OnceLock::new();
    SHEET.get_or_init(|| {
        let (grass, sand, still, ripple) = (grass(), path(), water(0), water(1));
        let mut rgba = Vec::with_capacity(64 * 16 * 4);
        for y in 0..16 {
            for tile in [&grass, &sand, &still, &ripple] {
                rgba.extend_from_slice(&tile.rgba[y * 64..(y + 1) * 64]);
            }
        }
        let strip = ImageData {
            width: 64,
            height: 16,
            rgba: rgba.into(),
        };
        SpriteSheet::grid(TILES, &strip, 16, 16)
    })
}

/// Grass everywhere, a path across the middle, and a pond.
fn ground_map() -> TileMap {
    let cells = (0..ROWS).flat_map(|row| {
        (0..COLS).map(move |col| {
            Some(if (1..=3).contains(&col) && (7..=9).contains(&row) {
                WATER_TILE
            } else if row == 5 || (col == 8 && row > 5) {
                PATH_TILE
            } else {
                GRASS_TILE
            })
        })
    });
    TileMap::new(COLS as u32, ROWS as u32, cells).animate(WATER_TILE, 2, 2.0)
}

fn color(c: char) -> [u8; 4] {
    match c {
        'k' => [0x2b, 0x21, 0x1d, 255],
        't' => [0x7a, 0x4a, 0x2a, 255],
        'l' => [0x2f, 0x7d, 0x3a, 255],
        'L' => [0x5b, 0xb0, 0x4e, 255],
        'r' => [0x76, 0x7a, 0x85, 255],
        'R' => [0xa9, 0xae, 0xb8, 255],
        'f' => [0xe8, 0x4d, 0x6a, 255],
        'y' => [0xf5, 0xc8, 0x3b, 255],
        's' => [0xf1, 0xc2, 0x9a, 255],
        'h' => [0x5a, 0x34, 0x1e, 255],
        'b' => [0x3a, 0x6e, 0xd8, 255],
        'p' => [0x34, 0x3a, 0x5e, 255],
        _ => [0, 0, 0, 0],
    }
}

fn art<S: AsRef<str>>(rows: &[S]) -> ImageData {
    let (h, w) = (rows.len(), rows[0].as_ref().len());
    let rgba: Vec<u8> = rows
        .iter()
        .flat_map(|r| r.as_ref().chars().flat_map(color))
        .collect();
    debug_assert_eq!(rgba.len(), w * h * 4);
    ImageData {
        width: w as u32,
        height: h as u32,
        rgba: rgba.into(),
    }
}

/// A 16 x 16 tile of `base` with a few `fleck` pixels, the same every time.
fn speckled(base: [u8; 4], fleck: [u8; 4], seed: u32) -> ImageData {
    let mut rgba = Vec::with_capacity(16 * 16 * 4);
    for y in 0u32..16 {
        for x in 0u32..16 {
            let n = (x * 7 + y * 13 + x * y * 3 + seed) % 17;
            rgba.extend(if n < 2 { fleck } else { base });
        }
    }
    ImageData {
        width: 16,
        height: 16,
        rgba: rgba.into(),
    }
}

fn grass() -> ImageData {
    speckled([0x4f, 0x9a, 0x45, 255], [0x6c, 0xb8, 0x55, 255], 0)
}

fn path() -> ImageData {
    speckled([0xd9, 0xb7, 0x7e, 255], [0xbf, 0x98, 0x5f, 255], 5)
}

/// Water whose highlights shift between frames.
fn water(frame: u32) -> ImageData {
    speckled([0x3d, 0x7f, 0xd1, 255], [0x9c, 0xc8, 0xf2, 255], frame * 8)
}

const TREE_ART: [&str; 16] = [
    "......kkkk......",
    "....kkLLLLkk....",
    "...kLLLLlLLLk...",
    "..kLLlLLLLlLLk..",
    "..kLLLLlLLLLLk..",
    ".kLlLLLLLLlLLLk.",
    ".kLLLlLLlLLLlLk.",
    ".klLLLLLLLLLLlk.",
    "..klLlLLLlLLlk..",
    "..kkllllllllkk..",
    "....kkkttkkk....",
    "......kttk......",
    "......kttk......",
    "......kttk......",
    ".....kkttkk.....",
    "................",
];

const ROCK_ART: [&str; 16] = [
    "................",
    "................",
    "................",
    "................",
    "................",
    "......kkkk......",
    "....kkRRRRkk....",
    "...kRRRRRRrrk...",
    "..kRRRRRrrrrrk..",
    "..kRRRRrrrrrrk..",
    ".kRRRrrrrrrrrrk.",
    ".krrrrrrrrrrrrk.",
    ".krrrrrrrrrrrrk.",
    "..kkrrrrrrrrkk..",
    "....kkkkkkkk....",
    "................",
];

const FLOWER_ART: [&str; 16] = [
    "................",
    "................",
    "................",
    "......kkkk......",
    ".....kffffk.....",
    "....kffyyffk....",
    "....kfyyyyfk....",
    "....kffyyffk....",
    ".....kffffk.....",
    "......kllk......",
    "...kk..kl..kk...",
    "..kLLk.kl.kLLk..",
    "...kLLkklkLLk...",
    "....kkLlLLkk....",
    "......kllk......",
    "................",
];

const HERO_A: [&str; 16] = [
    "......kkkk......",
    ".....khhhhk.....",
    "....khhhhhhk....",
    "....khsssshk....",
    "....kskssksk....",
    "....kssssssk....",
    ".....kssssk.....",
    "....kbbbbbbk....",
    "...kbbbbbbbbk...",
    "...ksbbbbbbsk...",
    "...kskbbbbksk...",
    "....kppppppk....",
    "....kppkkppk....",
    "....kppk.kppk...",
    "....kkkk.kkkk...",
    "................",
];

const HERO_B: [&str; 16] = [
    "................",
    "......kkkk......",
    ".....khhhhk.....",
    "....khhhhhhk....",
    "....khsssshk....",
    "....kskssksk....",
    "....kssssssk....",
    ".....kssssk.....",
    "....kbbbbbbk....",
    "...ksbbbbbbsk...",
    "...kskbbbbksk...",
    "....kppppppk....",
    "...kppk.kppk....",
    "...kkkk..kppk...",
    "..........kkk...",
    "................",
];

const HERO_C: [&str; 16] = [
    "................",
    "......kkkk......",
    ".....khhhhk.....",
    "....khhhhhhk....",
    "....khsssshk....",
    "....kskssksk....",
    "....kssssssk....",
    ".....kssssk.....",
    "....kbbbbbbk....",
    "...ksbbbbbbsk...",
    "...kskbbbbksk...",
    "....kppppppk....",
    "....kppk.kppk...",
    "...kppk...kkkk..",
    "...kkk..........",
    "................",
];
