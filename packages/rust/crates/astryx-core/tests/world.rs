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
