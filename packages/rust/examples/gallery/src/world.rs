//! The World section: a small top-down board of pixel sprites under a
//! camera that follows focus, with a walking character that goes to the
//! focused thing. Uses the experimental `world` and `paged` modules.

use crate::Msg;
use astryx_core::paged::CameraFollow;
use astryx_core::world::{Autotile, Board, Sprite, SpriteSheet, TerrainGrid, TileLayout, TileMap};
use astryx_core::{Element, ImageData, ImageId};
use std::sync::OnceLock;

/// The ground's tile sheet, laid out as `ground_rules` reads it.
const TILES: ImageId = ImageId::of("gallery/world/tiles.png");
/// The ground's still tiles, baked into one image.
const GROUND: ImageId = ImageId::of("gallery/world/ground.png");
/// Terrains, lowest first: soil lies over grass, water over soil.
const GRASS: u8 = 0;
const SOIL: u8 = 1;
const WATER: u8 = 2;
/// Sheet cells: two grass tiles, soil's 16 corner tiles, two copies of
/// water's 47 blob tiles, then two swaying decorations of two frames.
const SOIL_TILES: u32 = 2;
const WATER_TILES: u32 = SOIL_TILES + 16;
const TUFT: u32 = WATER_TILES + 2 * 47;
const WILDFLOWER: u32 = TUFT + 2;
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

/// The ground's tile sheet, drawn and cut once.
fn tiles() -> &'static SpriteSheet {
    static SHEET: OnceLock<SpriteSheet> = OnceLock::new();
    SHEET.get_or_init(|| {
        let mut cells = vec![grass(0), grass(9)];
        cells
            .extend((0..16).map(|bits| edged(soil(), SOIL_EDGE, |x, y| corner_inside(bits, x, y))));
        for seed in [0, 7] {
            cells.extend(
                blob_masks()
                    .iter()
                    .map(|&m| edged(water(seed), WATER_EDGE, |x, y| blob_inside(m, x, y))),
            );
        }
        cells.extend([tuft(0), tuft(1), wildflower(0), wildflower(1)]);
        let mut rgba = Vec::with_capacity(cells.len() * 16 * 16 * 4);
        for y in 0..16 {
            for cell in &cells {
                rgba.extend_from_slice(&cell.rgba[y * 64..(y + 1) * 64]);
            }
        }
        let strip = ImageData {
            width: 16 * cells.len() as u32,
            height: 16,
            rgba: rgba.into(),
        };
        SpriteSheet::grid(TILES, &strip, 16, 16)
    })
}

/// Grass everywhere, a soil path, and a pond with a soil shore.
const TERRAIN: [&str; ROWS] = [
    "ggggggggggggggg",
    "ggggggggggggggg",
    "ggggggggggggggg",
    "gggggggggggsggg",
    "ggggggggsssssgg",
    "ssssssssssggggg",
    "sssggggggsggggg",
    "wwsggggggsggggg",
    "wwwsggggggggggg",
    "wwwssgggggggggg",
    "wwwwsgggggggggg",
];

/// Soil meets grass on corner tiles; water meets soil on blob tiles.
fn ground_rules() -> Autotile {
    Autotile::new(GRASS, 0)
        .variants(2)
        .over(SOIL, TileLayout::Corners16, SOIL_TILES)
        .over(WATER, TileLayout::Blob47, WATER_TILES)
        .variants(2)
}

fn ground_map() -> TileMap {
    let grid = TerrainGrid::from_rows(&TERRAIN, |c| match c {
        's' => SOIL,
        'w' => WATER,
        _ => GRASS,
    });
    // Grass and wildflowers sway in a ripple that crosses the field.
    TileMap::from_terrain(&grid, &ground_rules())
        .scatter(GRASS, &[TUFT, WILDFLOWER], 18, 2, 2.0)
        .wave(0.09 * 4.0)
}

const SOIL_EDGE: [u8; 4] = [0x8a, 0x5a, 0x34, 255];
const WATER_EDGE: [u8; 4] = [0x2a, 0x55, 0x9a, 255];

/// A tile of `fill` where `inside` holds, outlined in `edge`, clear elsewhere.
fn edged(fill: ImageData, edge: [u8; 4], inside: impl Fn(i32, i32) -> bool) -> ImageData {
    let mut rgba = vec![0u8; 16 * 16 * 4];
    for y in 0..16i32 {
        for x in 0..16i32 {
            if !inside(x, y) {
                continue;
            }
            // Pixels at a tile border never outline, so tiles meet cleanly.
            let border = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| {
                let (nx, ny) = (x + dx, y + dy);
                (0..16).contains(&nx) && (0..16).contains(&ny) && !inside(nx, ny)
            });
            let i = ((y * 16 + x) * 4) as usize;
            let color = if border {
                edge
            } else {
                [fill.rgba[i], fill.rgba[i + 1], fill.rgba[i + 2], 255]
            };
            rgba[i..i + 4].copy_from_slice(&color);
        }
    }
    ImageData {
        width: 16,
        height: 16,
        rgba: rgba.into(),
    }
}

/// Inside a corner tile: each quarter of the tile holds the terrain when its
/// corner bit is set (top left 1, top right 2, bottom left 4, bottom right
/// 8), rounded where a quarter stands alone and filled in where it is the
/// only one missing.
fn corner_inside(bits: u32, x: i32, y: i32) -> bool {
    let (right, bottom) = (x >= 8, y >= 8);
    let bit = |r: bool, b: bool| bits >> (r as u32 + 2 * b as u32) & 1 == 1;
    let (here, side, updown) = (bit(right, bottom), bit(!right, bottom), bit(right, !bottom));
    let (cx, cy) = (
        if right { 16.0 } else { 0.0 },
        if bottom { 16.0 } else { 0.0 },
    );
    let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
    match (here, side, updown) {
        (true, false, false) => d <= 8.5,
        (true, _, _) => true,
        (false, true, true) => d > 7.5,
        (false, _, _) => false,
    }
}

/// The 47 blob masks in ascending order, as `TileLayout::Blob47` lays out.
fn blob_masks() -> Vec<u8> {
    let reduce = |m: u8| {
        let (n, e, s, w) = (m & 1 != 0, m & 4 != 0, m & 16 != 0, m & 64 != 0);
        let mut out = m & (1 | 4 | 16 | 64);
        for (bit, a, b) in [(2, n, e), (8, s, e), (32, s, w), (128, n, w)] {
            if m & bit != 0 && a && b {
                out |= bit;
            }
        }
        out
    };
    (0..=255u8).filter(|&m| reduce(m) == m).collect()
}

/// Inside a blob tile: the terrain pulls in from each missing side, with
/// round outer corners and a small notch where only a diagonal is missing.
fn blob_inside(mask: u8, x: i32, y: i32) -> bool {
    let has = |bit: u8| mask & bit != 0;
    let (n, e, s, w) = (has(1), has(4), has(16), has(64));
    let inset = 4;
    let (l, r, t, b) = (
        if w { 0 } else { inset },
        if e { 16 } else { 16 - inset },
        if n { 0 } else { inset },
        if s { 16 } else { 16 - inset },
    );
    if x < l || x >= r || y < t || y >= b {
        return false;
    }
    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
    let round = 5.0;
    let dist = |cx: f32, cy: f32| ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
    let corners = [
        (
            !n && !w,
            l as f32 + round,
            t as f32 + round,
            px < l as f32 + round && py < t as f32 + round,
        ),
        (
            !n && !e,
            r as f32 - round,
            t as f32 + round,
            px > r as f32 - round && py < t as f32 + round,
        ),
        (
            !s && !w,
            l as f32 + round,
            b as f32 - round,
            px < l as f32 + round && py > b as f32 - round,
        ),
        (
            !s && !e,
            r as f32 - round,
            b as f32 - round,
            px > r as f32 - round && py > b as f32 - round,
        ),
    ];
    if corners
        .iter()
        .any(|&(on, cx, cy, near)| on && near && dist(cx, cy) > round)
    {
        return false;
    }
    let notches = [
        (n && w && !has(128), 0.0, 0.0),
        (n && e && !has(2), 16.0, 0.0),
        (s && w && !has(32), 0.0, 16.0),
        (s && e && !has(8), 16.0, 16.0),
    ];
    !notches
        .iter()
        .any(|&(on, cx, cy)| on && dist(cx, cy) < inset as f32)
}

/// A tuft of grass leaning one way, then the other.
fn tuft(frame: u32) -> ImageData {
    let lean = if frame == 0 { 0 } else { 1 };
    sprite(|x, y| {
        let blade =
            |base: i32, height: i32| y >= 15 - height && y < 15 && x == base + (15 - y) * lean / 3;
        (blade(6, 6) || blade(8, 8) || blade(10, 5)).then_some([0x7c, 0xc4, 0x5a, 255])
    })
}

/// A wildflower nodding on its stem.
fn wildflower(frame: u32) -> ImageData {
    let lean = frame as i32;
    sprite(|x, y| {
        let head = (x - 8 - lean).abs() <= 1 && (y - 8).abs() <= 1;
        let center = x == 8 + lean && y == 8;
        let stem = x == 8 && (10..15).contains(&y);
        if center {
            Some([0xf5, 0xc8, 0x3b, 255])
        } else if head {
            Some([0xf2, 0xf2, 0xf7, 255])
        } else {
            stem.then_some([0x3b, 0x8f, 0x3e, 255])
        }
    })
}

fn sprite(pixel: impl Fn(i32, i32) -> Option<[u8; 4]>) -> ImageData {
    let rgba: Vec<u8> = (0..16)
        .flat_map(|y| (0..16).map(move |x| (x, y)))
        .flat_map(|(x, y)| pixel(x, y).unwrap_or([0; 4]))
        .collect();
    ImageData {
        width: 16,
        height: 16,
        rgba: rgba.into(),
    }
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

fn grass(seed: u32) -> ImageData {
    speckled([0x4f, 0x9a, 0x45, 255], [0x6c, 0xb8, 0x55, 255], seed)
}

fn soil() -> ImageData {
    speckled([0xc4, 0x93, 0x5c, 255], [0xa8, 0x77, 0x46, 255], 5)
}

fn water(seed: u32) -> ImageData {
    speckled([0x3d, 0x7f, 0xd1, 255], [0x9c, 0xc8, 0xf2, 255], seed)
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
