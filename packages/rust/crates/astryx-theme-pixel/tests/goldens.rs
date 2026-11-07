//! Golden images of every gallery section in two pixel palettes, light and
//! dark, at scale 3 (a 400 ppi handheld), plus a few scripted states. Set
//! `ASTRYX_UPDATE_GOLDENS=1` to rewrite them after an intended change.

use astryx_core::{Input, Mode};
use astryx_gallery::{Gallery, Section};
use astryx_headless::golden::{assert_golden, Tolerance};
use astryx_headless::render_png;
use astryx_theme_pixel::{Palette, DEFAULT_PIXEL};
use std::path::PathBuf;

// Nothing is antialiased on a pixel grid, so images match exactly.
const TOLERANCE: Tolerance = Tolerance {
    channel: 0,
    pixels: 0,
};
const SIZE: (u32, u32) = (400, 640);
const SCALE: f32 = 3.0;

fn check(gallery: Gallery, inputs: &[Input], golden: &str) {
    check_at(gallery, SCALE, inputs, golden);
}

fn check_at(gallery: Gallery, scale: f32, inputs: &[Input], golden: &str) {
    let mut rt = astryx_gallery::runtime(gallery);
    let png = render_png(&mut rt, SIZE.0, SIZE.1, scale, inputs);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens")
        .join(golden);
    assert_golden(&png, path, TOLERANCE);
}

#[test]
fn every_section_in_two_palettes_light_and_dark() {
    for palette in [Palette::Garden, Palette::GameBoy] {
        for section in Section::ALL {
            for (mode, name) in [(Mode::Light, "light"), (Mode::Dark, "dark")] {
                let gallery = Gallery::section(section).with_theme(palette.theme(), mode);
                let golden = format!("{}-{}-{name}@3x.png", palette.name(), section.slug());
                check(gallery, &[], &golden);
            }
        }
    }
}

#[test]
fn scripted_states() {
    use Input::{Down, Next, A};
    let garden =
        |section| Gallery::section(section).with_theme(Palette::Garden.theme(), Mode::Light);
    // The first switch toggled on, with the stepped focus ring on it.
    check(
        garden(Section::Switches),
        &[A],
        "garden-switches-toggled@3x.png",
    );
    // The first selector's menu open on its second option.
    check(
        garden(Section::Selectors),
        &[A, Down],
        "garden-selectors-open@3x.png",
    );
    // Focus on the second text input: the stepped field focus.
    check(
        garden(Section::TextInputs),
        &[Next],
        "garden-text-inputs-focus@3x.png",
    );
    // The whole gallery page in the NES and PICO-8 palettes.
    for (palette, mode, name) in [
        (Palette::Nes, Mode::Light, "light"),
        (Palette::Pico8, Mode::Dark, "dark"),
    ] {
        let gallery = Gallery::new().with_theme(palette.theme(), mode);
        check(
            gallery,
            &[],
            &format!("{}-page-{name}@3x.png", palette.name()),
        );
    }
}

#[test]
fn scale_two_tooltip_and_right_to_left() {
    use astryx_core::Direction;
    // The handheld's default scale: 4 device pixels per theme pixel.
    for section in [Section::Buttons, Section::Switches] {
        for (palette, mode, name) in [
            (Palette::Garden, Mode::Light, "light"),
            (Palette::GameBoy, Mode::Dark, "dark"),
        ] {
            let gallery = Gallery::section(section).with_theme(palette.theme(), mode);
            let golden = format!("{}-{}-{name}@2x.png", palette.name(), section.slug());
            check_at(gallery, 2.0, &[], &golden);
        }
    }
    // Twenty-one stops reach the Share button and its tooltip.
    let to_share: Vec<Input> = std::iter::repeat_n(Input::Next, 21).collect();
    let buttons =
        Gallery::section(Section::Buttons).with_theme(Palette::Garden.theme(), Mode::Light);
    check(buttons, &to_share, "garden-tooltip@3x.png");
    for section in [Section::Switches, Section::ListItems] {
        let gallery = Gallery::section(section)
            .with_theme(Palette::Garden.theme(), Mode::Light)
            .with_direction(Direction::Rtl);
        check(
            gallery,
            &[],
            &format!("garden-{}-rtl@3x.png", section.slug()),
        );
    }
}

/// Every edge between two colors falls on a whole theme pixel or, for small
/// icons, half of one, at scales 3 and 2, left to right and right to left.
#[test]
fn edges_fall_on_the_pixel_grid() {
    use astryx_core::Direction;
    let skip = [Section::Avatars, Section::Thumbnails];
    let runs = [
        (3.0, Direction::Ltr),
        (2.0, Direction::Ltr),
        (3.0, Direction::Rtl),
    ];
    for (section, (scale, direction)) in Section::ALL
        .into_iter()
        .filter(|s| !skip.contains(s))
        .flat_map(|s| runs.map(|r| (s, r)))
    {
        let step = (scale * DEFAULT_PIXEL / 2.0) as u32;
        let gallery = Gallery::section(section)
            .with_theme(Palette::Garden.theme(), Mode::Light)
            .with_direction(direction);
        let mut rt = astryx_gallery::runtime(gallery);
        let png = render_png(&mut rt, SIZE.0, SIZE.1, scale, &[]);
        let off_grid = off_grid(&png, step);
        assert!(
            off_grid.is_empty(),
            "{} at {scale}x {direction:?} has {} edges off the grid, first at {:?}",
            section.slug(),
            off_grid.len(),
            &off_grid[..off_grid.len().min(8)]
        );
    }
}

/// Pixels where the color changes across a column or row that is not a
/// multiple of `step`.
fn off_grid(png: &[u8], step: u32) -> Vec<(u32, u32)> {
    let (rgba, w, h) = astryx_headless::golden::decode_png(png).unwrap();
    let px = |x: u32, y: u32| &rgba[((y * w + x) * 4) as usize..][..4];
    let mut out = Vec::new();
    for y in 0..h {
        for x in 1..w {
            if px(x, y) != px(x - 1, y) && x % step != 0 {
                out.push((x, y));
            }
        }
    }
    for y in 1..h {
        for x in 0..w {
            if px(x, y) != px(x, y - 1) && y % step != 0 {
                out.push((x, y));
            }
        }
    }
    out
}

/// A section paged on a 1024 by 768, scale 2 screen: its dots, and a page
/// mid-slide after R2, stay on the grid.
#[test]
fn pages_stay_on_the_grid() {
    for (inputs, name) in [
        (vec![], "garden-buttons-page-1@2x.png"),
        (vec![Input::R2], "garden-buttons-page-sliding@2x.png"),
    ] {
        let gallery = Gallery {
            page: Some(Section::Buttons),
            ..Gallery::new()
        }
        .with_theme(Palette::Garden.theme(), Mode::Light);
        let mut rt = astryx_gallery::runtime(gallery);
        let png = render_png(&mut rt, 512, 384, 2.0, &inputs);
        let bad = off_grid(&png, 2);
        assert!(
            bad.is_empty(),
            "{name}: {} edges off the grid, first at {:?}",
            bad.len(),
            &bad[..bad.len().min(8)]
        );
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/goldens")
            .join(name);
        assert_golden(&png, path, TOLERANCE);
    }
}
