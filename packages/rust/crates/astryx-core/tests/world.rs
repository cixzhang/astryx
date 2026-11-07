//! The experimental world module: boards, sprites, and sprite sheets.

mod common;

use astryx_core::paged::CameraFollow;
use astryx_core::testing::*;
use astryx_core::world::{Board, Sprite, SpriteSheet};
use astryx_core::*;
use common::*;

const HERO: ImageId = ImageId(1000);

fn images(list: &DisplayList) -> Vec<(Rect, ImageId)> {
    list.commands
        .iter()
        .filter_map(|c| match c {
            DrawCmd::Image { rect, image, .. } => Some((*rect, *image)),
            _ => None,
        })
        .collect()
}

fn at<M: Clone + 'static>(rt: &Runtime<View<M>>, label: &str) -> Rect {
    a11y_nodes(&rt.accessibility().unwrap())
        .into_iter()
        .find(|n| n.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("no {label}"))
        .bounds
}

fn focused<M: Clone + 'static>(rt: &Runtime<View<M>>) -> Option<String> {
    a11y_nodes(&rt.accessibility()?)
        .into_iter()
        .find(|n| n.focused)
        .and_then(|n| n.label)
}

fn tile(label: &str, n: usize) -> Sprite<usize> {
    Sprite::new(ImageId(n as u64))
        .size(32.0, 32.0)
        .label(label)
        .on_click(n)
}

#[test]
fn boards_place_children_at_points_and_arrows_follow_geometry() {
    let mut rt = runtime(|| {
        Board::new(400.0, 300.0)
            .child_at(20.0, 20.0, tile("tree", 1))
            .child_at(300.0, 30.0, tile("rock", 2))
            .child_at(40.0, 200.0, tile("well", 3))
            .into()
    });
    draw(&mut rt, 400, 300);
    assert_eq!(at(&rt, "tree"), Rect::new(20.0, 20.0, 32.0, 32.0));
    assert_eq!(at(&rt, "rock"), Rect::new(300.0, 30.0, 32.0, 32.0));
    assert_eq!(focused(&rt).as_deref(), Some("tree"));
    tap(&mut rt, Input::Right);
    draw(&mut rt, 400, 300);
    assert_eq!(focused(&rt).as_deref(), Some("rock"));
    tap(&mut rt, Input::Down);
    draw(&mut rt, 400, 300);
    assert_eq!(
        focused(&rt).as_deref(),
        Some("well"),
        "down, then the nearest by geometry"
    );
}

#[test]
fn sort_by_y_draws_lower_children_later_and_keeps_identity() {
    use std::cell::Cell;
    use std::rc::Rc;
    let hero_y = Rc::new(Cell::new(10.0f32));
    let y = hero_y.clone();
    let mut rt = runtime(move || {
        Board::new(200.0, 200.0)
            .sort_by_y(true)
            .child_at(
                0.0,
                y.get(),
                Sprite::new(HERO).size(32.0, 32.0).label("hero").on_click(0),
            )
            .child_at(
                0.0,
                50.0,
                Sprite::new(ImageId(7)).size(32.0, 32.0).label("bush"),
            )
            .into()
    });
    rt.set_preferences(&Preferences {
        reduced_motion: true,
        ..Preferences::default()
    });
    let order = |list: &DisplayList| {
        images(list)
            .into_iter()
            .map(|(_, id)| id)
            .collect::<Vec<_>>()
    };
    let list = draw(&mut rt, 200, 200);
    assert_eq!(
        order(&list),
        [HERO, ImageId(7)],
        "the hero is above the bush, so drawn first"
    );
    let id = a11y_nodes(&rt.accessibility().unwrap())
        .into_iter()
        .find(|n| n.focused)
        .unwrap()
        .id;
    hero_y.set(80.0);
    rt.app_mut();
    let list = draw(&mut rt, 200, 200);
    assert_eq!(
        order(&list),
        [ImageId(7), HERO],
        "now below it, so drawn over it"
    );
    let after = a11y_nodes(&rt.accessibility().unwrap())
        .into_iter()
        .find(|n| n.focused)
        .unwrap();
    assert_eq!(
        (after.id, after.label.as_deref()),
        (id, Some("hero")),
        "same element, focus kept"
    );
}

#[test]
fn board_children_glide_to_new_places() {
    use std::cell::Cell;
    use std::rc::Rc;
    let x = Rc::new(Cell::new(0.0f32));
    let xs = x.clone();
    let mut rt = runtime(move || {
        Board::<()>::new(300.0, 100.0)
            .child_at(xs.get(), 0.0, Sprite::new(HERO).size(32.0, 32.0))
            .into()
    });
    draw(&mut rt, 300, 100);
    x.set(100.0);
    rt.app_mut();
    let list = rt.frame(
        &FrameInfo {
            width: 300,
            height: 100,
            scale: 1.0,
            dt: 0.0,
        },
        &measure(),
    );
    let moving = list
        .commands
        .iter()
        .any(|c| matches!(c, DrawCmd::PushTransform { translate, .. } if translate.x == -100.0));
    assert!(moving, "drawn where it was, on its way");
    assert!(rt.wants_frames());
}

#[test]
fn sprites_draw_square_pixels_on_whole_device_pixels() {
    let mut rt = runtime(|| {
        VStack::<()>::new()
            .padding_block_start(Space::S0_5)
            .child(
                HStack::new()
                    .padding_inline_start(Space::S0_5)
                    .child(Sprite::new(HERO).size(15.3, 16.2)),
            )
            .into()
    });
    let list = rt.frame(
        &FrameInfo {
            width: 200,
            height: 200,
            scale: 3.0,
            dt: 0.0,
        },
        &measure(),
    );
    let ix = list
        .commands
        .iter()
        .position(|c| matches!(c, DrawCmd::Image { .. }))
        .unwrap();
    assert_eq!(
        list.commands[ix - 1],
        DrawCmd::PushPixelGrid { cell: 1.0 / 3.0 },
        "no smoothing"
    );
    assert_eq!(list.commands[ix + 1], DrawCmd::PopPixelGrid);
    let DrawCmd::Image { rect, fit, .. } = list.commands[ix] else {
        unreachable!()
    };
    assert_eq!(fit, ImageFit::Fill);
    for v in [rect.x, rect.y, rect.width, rect.height] {
        assert!(
            ((v * 3.0) - (v * 3.0).round()).abs() < 1e-3,
            "{rect:?} is on device pixels"
        );
    }
}

#[test]
fn sprite_frames_play_and_hold_under_reduced_motion() {
    let view = || {
        Sprite::<()>::new(HERO)
            .size(16.0, 16.0)
            .frames(4, 10.0)
            .into()
    };
    let mut rt = runtime(view);
    let frame = |rt: &mut Runtime<View<()>>, dt: f32| {
        images(&rt.frame(
            &FrameInfo {
                width: 100,
                height: 100,
                scale: 1.0,
                dt,
            },
            &measure(),
        ))[0]
            .1
    };
    assert_eq!(frame(&mut rt, 0.0), HERO);
    assert!(rt.wants_frames());
    assert_eq!(frame(&mut rt, 0.15), ImageId(HERO.0 + 1));
    assert_eq!(frame(&mut rt, 0.2), ImageId(HERO.0 + 3));
    assert_eq!(frame(&mut rt, 0.1), HERO, "the strip loops");
    let mut rt = runtime(view);
    rt.set_preferences(&Preferences {
        reduced_motion: true,
        ..Preferences::default()
    });
    assert_eq!(frame(&mut rt, 0.0), HERO);
    assert_eq!(frame(&mut rt, 0.25), HERO, "held on its first frame");
    assert!(!rt.wants_frames());
}

#[test]
fn sprite_sheets_cut_cells_and_regions() {
    // A 4 x 2 image: each pixel's red channel is its index.
    let rgba: Vec<u8> = (0..8u8).flat_map(|i| [i, 0, 0, 255]).collect();
    let image = ImageData {
        width: 4,
        height: 2,
        rgba: rgba.into(),
    };
    let sheet = SpriteSheet::grid(ImageId(50), &image, 2, 2);
    assert_eq!(sheet.len(), 2);
    let cells = sheet.images();
    assert_eq!(cells[0].0, ImageId(50));
    assert_eq!(cells[1].0, ImageId(51));
    let red = |d: &ImageData| d.rgba.chunks(4).map(|p| p[0]).collect::<Vec<_>>();
    assert_eq!(red(cells[0].1.as_ref().unwrap()), [0, 1, 4, 5]);
    assert_eq!(red(cells[1].1.as_ref().unwrap()), [2, 3, 6, 7]);
    let edge = SpriteSheet::regions(ImageId(60), &image, &[Rect::new(3.0, 1.0, 2.0, 2.0)]);
    let data = edge.images().remove(0).1.unwrap();
    assert_eq!(red(&data), [7, 0, 0, 0]);
    assert_eq!(data.rgba[7], 0, "outside the image is clear");
}

#[test]
fn the_camera_follows_focus_across_a_board() {
    let mut rt = runtime(|| {
        let mut board = Board::new(1200.0, 300.0);
        for i in 0..10 {
            board = board.child_at(i as f32 * 120.0, 100.0, tile(&format!("t{i}"), i));
        }
        CameraFollow::new(board).into()
    });
    draw(&mut rt, 400, 300);
    for _ in 0..6 {
        tap(&mut rt, Input::Right);
    }
    draw(&mut rt, 400, 300);
    rt.frame(
        &FrameInfo {
            width: 400,
            height: 300,
            scale: 1.0,
            dt: 1.0,
        },
        &measure(),
    );
    assert_eq!(focused(&rt).as_deref(), Some("t6"));
    let f = at(&rt, "t6");
    assert!(
        f.x >= 100.0 - 0.5 && f.x + f.width <= 300.0 + 0.5,
        "{f:?} outside the dead zone's bounds"
    );
}

#[test]
fn tile_maps_bake_still_tiles_into_one_image_and_play_animated_ones() {
    use astryx_core::world::TileMap;
    // Three 1 x 1 cells: red, green, blue.
    let rgba: Vec<u8> = [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]].concat();
    let sheet = SpriteSheet::grid(
        ImageId(70),
        &ImageData {
            width: 3,
            height: 1,
            rgba: rgba.into(),
        },
        1,
        1,
    );
    // Cells 1 and 2 are one animated tile; cell 0 is still.
    let map =
        TileMap::new(3, 2, [Some(0), Some(1), Some(0), None, Some(0), Some(0)]).animate(1, 2, 4.0);
    let (id, data) = map.bake(&sheet, ImageId(80));
    let data = data.unwrap();
    assert_eq!((id, data.width, data.height), (ImageId(80), 3, 2));
    let px: Vec<[u8; 4]> = data
        .rgba
        .chunks(4)
        .map(|p| [p[0], p[1], p[2], p[3]])
        .collect();
    assert_eq!(
        px,
        [
            [255, 0, 0, 255],
            [0, 0, 0, 0],
            [255, 0, 0, 255],
            [0, 0, 0, 0],
            [255, 0, 0, 255],
            [255, 0, 0, 255]
        ],
        "still tiles baked; animated and empty ones clear"
    );
    let mut rt = runtime(move || map.layer(ImageId(80), &sheet, 16.0));
    let frame = |rt: &mut Runtime<View<()>>, dt: f32| {
        images(&rt.frame(
            &FrameInfo {
                width: 100,
                height: 100,
                scale: 1.0,
                dt,
            },
            &measure(),
        ))
    };
    let first = frame(&mut rt, 0.0);
    assert_eq!(
        first.len(),
        2,
        "one baked image and one animated tile, not six draws"
    );
    assert_eq!(first[0], (Rect::new(0.0, 0.0, 48.0, 32.0), ImageId(80)));
    assert_eq!(
        first[1],
        (Rect::new(16.0, 0.0, 16.0, 16.0), sheet_cell(70, 1))
    );
    assert_eq!(
        frame(&mut rt, 0.3)[1].1,
        sheet_cell(70, 2),
        "the water moves"
    );
}

fn sheet_cell(first: u64, i: u64) -> ImageId {
    ImageId(first + i)
}

// ---- Autotiling ----

use astryx_core::world::{Autotile, TerrainGrid, TileLayout, TileMap};

const WATER: u8 = 0;
const SOIL: u8 = 1;
const GRASS: u8 = 2;

fn terrain(rows: &[&str]) -> TerrainGrid {
    TerrainGrid::from_rows(rows, |c| match c {
        's' => SOIL,
        'g' => GRASS,
        _ => WATER,
    })
}

/// Water at cell 0, soil corner tiles from 10, grass blob tiles from 100.
fn rules() -> Autotile {
    Autotile::new(WATER, 0)
        .over(SOIL, TileLayout::Corners16, 10)
        .over(GRASS, TileLayout::Blob47, 100)
}

#[test]
fn corner_tiles_follow_the_four_cells_around_each_corner() {
    let grid = terrain(&["...", ".s.", "..."]);
    let map = TileMap::from_terrain(&grid, &rules());
    // Layer 1 is soil, on the corner grid: the corners of cell (1, 1) are
    // tiles (1, 1), (2, 1), (1, 2), (2, 2), each with the soil cell in one
    // quarter.
    assert_eq!(map.tile(1, 1, 1), Some(10 + 8), "soil to the southeast");
    assert_eq!(map.tile(1, 2, 1), Some(10 + 4), "to the southwest");
    assert_eq!(map.tile(1, 1, 2), Some(10 + 2), "to the northeast");
    assert_eq!(map.tile(1, 2, 2), Some(10 + 1), "to the northwest");
    assert_eq!(map.tile(1, 0, 0), None, "no soil around it");
    assert_eq!(map.tile(0, 1, 1), Some(0), "water lies under everything");
}

#[test]
fn blob_tiles_use_all_47_shapes_from_the_8_neighbors() {
    let grid = terrain(&["g.g", "ggg", "..g"]);
    let map = TileMap::from_terrain(&grid, &rules());
    let at = |x, y| map.tile(2, x, y).map(|t| t - 100);
    assert_eq!(at(1, 0), None, "water has no grass tile");
    assert!(at(1, 1).is_some());
    // Every one of the 47 tiles is reachable: try every 3 x 3 pattern.
    let mut seen = std::collections::BTreeSet::new();
    for bits in 0u32..512 {
        if bits & 16 == 0 {
            continue; // the center must be grass
        }
        let rows: Vec<String> = (0..3)
            .map(|y| {
                (0..3)
                    .map(|x| {
                        if bits >> (y * 3 + x) & 1 == 1 {
                            'g'
                        } else {
                            '.'
                        }
                    })
                    .collect()
            })
            .collect();
        let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
        // Pad with water so edges do not continue the pattern.
        let padded: Vec<String> = std::iter::once(".....".to_string())
            .chain(refs.iter().map(|r| format!(".{r}.")))
            .chain(std::iter::once(".....".to_string()))
            .collect();
        let refs: Vec<&str> = padded.iter().map(String::as_str).collect();
        let map = TileMap::from_terrain(&terrain(&refs), &rules());
        seen.insert(map.tile(2, 2, 2).unwrap() - 100);
    }
    assert_eq!(seen.len(), 47);
    assert_eq!(seen.iter().max(), Some(&46));
}

#[test]
fn changing_a_cell_repicks_only_its_neighborhood() {
    let rows = ["gggggg", "gssssg", "gs..sg", "gs..sg", "gssssg", "gggggg"];
    let mut map = TileMap::from_terrain(&terrain(&rows), &rules());
    let changed = map.set_terrain(2, 2, GRASS);
    assert!(!changed.is_empty());
    for (layer, x, y) in &changed {
        assert!(
            (1..=3).contains(x) && (1..=3).contains(y),
            "layer {layer} tile ({x}, {y}) is outside the 3 x 3"
        );
    }
    // The result matches building the whole map again.
    let mut rows2 = rows.map(String::from);
    rows2[2].replace_range(2..3, "g");
    let refs: Vec<&str> = rows2.iter().map(String::as_str).collect();
    let fresh = TileMap::from_terrain(&terrain(&refs), &rules());
    for layer in 0..3 {
        for y in 0..7 {
            for x in 0..7 {
                assert_eq!(
                    map.tile(layer, x, y),
                    fresh.tile(layer, x, y),
                    "layer {layer} ({x}, {y})"
                );
            }
        }
    }
}

#[test]
fn variants_are_chosen_by_place_and_stay_the_same() {
    let rules = Autotile::new(WATER, 0).variants(3);
    let grid = TerrainGrid::new(8, 8, WATER);
    let a = TileMap::from_terrain(&grid, &rules);
    let b = TileMap::from_terrain(&grid, &rules);
    assert_eq!(a, b, "deterministic");
    let used: std::collections::BTreeSet<u32> = (0..8)
        .flat_map(|y| (0..8).map(move |x| (x, y)))
        .filter_map(|(x, y)| a.tile(0, x, y))
        .collect();
    assert_eq!(used, [0, 1, 2].into(), "every variant shows up");
}

#[test]
fn decorations_ripple_instead_of_moving_in_step() {
    let grid = TerrainGrid::new(10, 10, GRASS);
    let sheet = SpriteSheet::grid(
        ImageId(500),
        &ImageData {
            width: 64,
            height: 16,
            rgba: vec![255; 64 * 16 * 4].into(),
        },
        16,
        16,
    );
    let map =
        TileMap::from_terrain(&grid, &Autotile::new(GRASS, 0)).scatter(GRASS, &[1], 30, 2, 2.0);
    let (id, _) = map.bake(&sheet, ImageId(600));
    let view = move || map.layer::<()>(id, &sheet, 16.0);
    let mut rt = runtime(view.clone());
    let shown = images(&rt.frame(
        &FrameInfo {
            width: 160,
            height: 160,
            scale: 1.0,
            dt: 0.0,
        },
        &measure(),
    ));
    let decorations: Vec<ImageId> = shown[1..].iter().map(|s| s.1).collect();
    assert!(
        decorations.len() >= 15 && decorations.len() <= 45,
        "about 30 of 100 cells: {}",
        decorations.len()
    );
    assert!(
        decorations.contains(&ImageId(501)) && decorations.contains(&ImageId(502)),
        "different phases show different frames at once"
    );
    assert!(rt.wants_frames());
    let mut still = runtime(view);
    still.set_preferences(&Preferences {
        reduced_motion: true,
        ..Preferences::default()
    });
    let held = images(&still.frame(
        &FrameInfo {
            width: 160,
            height: 160,
            scale: 1.0,
            dt: 0.7,
        },
        &measure(),
    ));
    assert!(
        held[1..].iter().all(|s| s.1 == ImageId(501)),
        "held on the first frame"
    );
}

#[test]
fn corner_tiles_bake_half_a_tile_up_and_left() {
    // Cells 0 water (blue), 10.. soil: only tile 18 (soil to the southeast) is opaque red.
    let mut rgba = [0u8; 26 * 4];
    rgba[0..4].copy_from_slice(&[0, 0, 255, 255]);
    rgba[18 * 4..18 * 4 + 4].copy_from_slice(&[255, 0, 0, 255]);
    // 2 x 2 pixel tiles: widen each 1 x 1 cell.
    let mut wide = Vec::new();
    for _row in 0..2 {
        for c in 0..26 {
            for _ in 0..2 {
                wide.extend_from_slice(&rgba[c * 4..c * 4 + 4]);
            }
        }
    }
    let sheet = SpriteSheet::grid(
        ImageId(700),
        &ImageData {
            width: 52,
            height: 2,
            rgba: wide.into(),
        },
        2,
        2,
    );
    let grid = terrain(&["...", ".s.", "..."]);
    let map = TileMap::from_terrain(
        &grid,
        &Autotile::new(WATER, 0).over(SOIL, TileLayout::Corners16, 10),
    );
    let data = map.bake(&sheet, ImageId(800)).1.unwrap();
    let px = |x: u32, y: u32| {
        let i = ((y * data.width + x) * 4) as usize;
        [data.rgba[i], data.rgba[i + 1], data.rgba[i + 2]]
    };
    // Corner tile (1, 1) covers pixels (1..3, 1..3): its red reaches into
    // the soil cell's top left quarter at (2, 2) and the water beside it.
    assert_eq!(px(2, 2), [255, 0, 0]);
    assert_eq!(px(1, 1), [255, 0, 0]);
    assert_eq!(px(0, 0), [0, 0, 255]);
}

#[test]
fn autotile_matches_the_garden_corner_layout() {
    // Upper terrain on corners x >= 2 of a 4 x 2 map's 5 x 3 corner grid.
    let map = TileMap::autotile(4, 2, |x, _| x >= 2, 100);
    let row: Vec<u32> = (0..4).map(|x| map.tile(0, x, 0).unwrap() - 100).collect();
    // Column 1 has its right corners up (TR 2 + BR 8); columns 2 and 3 are all up.
    assert_eq!(row, [0, 2 + 8, 15, 15]);
    let varied = TileMap::autotile(8, 8, |_, _| false, 100).full_variants(&[200, 201], &[]);
    let tiles: std::collections::BTreeSet<u32> = (0..8)
        .flat_map(|y| (0..8).map(move |x| (x, y)))
        .filter_map(|(x, y)| varied.tile(0, x, y))
        .collect();
    assert_eq!(tiles, [100, 200, 201].into(), "lower cells vary by place");
    assert_eq!(
        varied,
        TileMap::autotile(8, 8, |_, _| false, 100).full_variants(&[200, 201], &[]),
        "the same every time"
    );
}

#[test]
fn wave_decorations_lag_column_by_column() {
    let sheet = SpriteSheet::grid(
        ImageId(900),
        &ImageData {
            width: 80,
            height: 16,
            rgba: vec![255; 80 * 16 * 4].into(),
        },
        16,
        16,
    );
    // A 40-column strip, fully decorated, four frames at 4 fps: a 1 s cycle.
    let map = TileMap::new(40, 1, vec![Some(0); 40])
        .scatter(0, &[1], 100, 4, 4.0)
        .wave(0.09 * 8.0);
    let (id, _) = map.bake(&sheet, ImageId(950));
    let mut rt = runtime(move || map.layer::<()>(id, &sheet, 16.0));
    let shown = images(&rt.frame(
        &FrameInfo {
            width: 640,
            height: 16,
            scale: 1.0,
            dt: 0.1,
        },
        &measure(),
    ));
    let frames: Vec<u64> = shown[1..].iter().map(|s| s.1 .0 - 901).collect();
    // Each column is 0.72 rad behind its left neighbor: frames step down
    // as x grows, wrapping around the strip.
    assert_ne!(frames[0], frames[1]);
    let steps: Vec<u64> = frames.windows(2).map(|w| (w[0] + 4 - w[1]) % 4).collect();
    assert!(steps.iter().all(|&s| s <= 1), "{frames:?}");
}
