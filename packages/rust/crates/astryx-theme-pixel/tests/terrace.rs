//! Terrace, the pixel theme drawn only from the fixed palette: every gallery
//! section light and dark at the handheld's scale 2, and world art that
//! the palette checks before it is registered. Set `ASTRYX_UPDATE_GOLDENS=1`
//! to rewrite the images after an intended change.

use astryx_core::world::{Board, Sprite};
use astryx_core::{App, Command, Element, ImageData, ImageId, Mode, Runtime, ThemeChoice};
use astryx_gallery::{Gallery, Section};
use astryx_headless::golden::{assert_golden, Tolerance};
use astryx_headless::render_png;
use astryx_theme_pixel::{Import, PixelTheme, PALETTE};
use std::path::PathBuf;

const TOLERANCE: Tolerance = Tolerance {
    channel: 0,
    pixels: 0,
};

fn golden(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens")
        .join(name)
}

#[test]
fn every_section_light_and_dark_at_scale_two() {
    for section in Section::ALL {
        for (mode, name) in [(Mode::Light, "light"), (Mode::Dark, "dark")] {
            let gallery = Gallery::section(section).with_theme(PixelTheme::Terrace.theme(), mode);
            let mut rt = astryx_gallery::runtime(gallery);
            let png = render_png(&mut rt, 400, 640, 2.0, &[]);
            assert_golden(
                &png,
                golden(&format!("terrace-{}-{name}@2x.png", section.slug())),
                TOLERANCE,
            );
        }
    }
}

/// One letter per swatch, for art drawn in code.
const KEY: [(char, &str); 9] = [
    ('k', "ink"),
    ('p', "ink-plum"),
    ('1', "leaf-1"),
    ('2', "leaf-2"),
    ('3', "leaf-3"),
    ('g', "green-2"),
    ('w', "wood-1"),
    ('d', "wood-2"),
    ('s', "soil-1"),
];

/// A 12 by 12 tree with an ink outline, as world art is drawn.
const TREE: [&str; 12] = [
    "....kkkk....",
    "..kk1122kk..",
    ".k11222233k.",
    ".k12223333k.",
    "k1222333g33k",
    "k223333ggg3k",
    ".k233gggg3k.",
    "..kk3ggg3kk.",
    "....kddk....",
    "....kwdk....",
    "...kswdsk...",
    "...kkkkkk...",
];

fn art(rows: &[&str]) -> ImageData {
    let width = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut rgba = Vec::with_capacity(width * rows.len() * 4);
    for row in rows {
        for x in 0..width {
            let ch = row.as_bytes().get(x).copied().unwrap_or(b'.') as char;
            match KEY.iter().find(|(k, _)| *k == ch) {
                Some((_, name)) => {
                    let s = PALETTE.swatch(name).expect("swatch in the key");
                    rgba.extend_from_slice(&[s.rgb[0], s.rgb[1], s.rgb[2], 255]);
                }
                None => rgba.extend_from_slice(&[0, 0, 0, 0]),
            }
        }
    }
    ImageData {
        width: width as u32,
        height: rows.len() as u32,
        rgba: rgba.into(),
    }
}

/// A photo-like gradient made outside the palette, to import by quantizing.
fn gradient() -> ImageData {
    let (w, h) = (12u32, 12u32);
    let mut rgba = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32 / (w - 1) as f32, y as f32 / (h - 1) as f32);
            rgba.extend_from_slice(&[
                (40.0 + 200.0 * fx) as u8,
                (90.0 + 120.0 * fy) as u8,
                (160.0 - 100.0 * fx) as u8,
                255,
            ]);
        }
    }
    ImageData {
        width: w,
        height: h,
        rgba: rgba.into(),
    }
}

struct Grove(Mode);

impl App for Grove {
    type Msg = ();
    fn view(&self) -> Element<()> {
        let mut board = Board::new(192.0, 72.0);
        for (i, x) in [8.0, 56.0, 104.0].into_iter().enumerate() {
            board = board.child_at(
                x,
                12.0,
                Sprite::new(ImageId(1))
                    .size(48.0, 48.0)
                    .label(format!("Tree {}", i + 1)),
            );
        }
        board
            .child_at(
                160.0,
                24.0,
                Sprite::new(ImageId(2)).size(24.0, 24.0).label("Imported"),
            )
            .into()
    }
    fn update(&mut self, _: ()) -> Command {
        Command::None
    }
    fn theme(&self) -> ThemeChoice {
        ThemeChoice::new(PixelTheme::Terrace.theme(), self.0)
    }
}

/// World art on the palette registers as drawn; art made elsewhere is
/// quantized onto it. Every pixel of the frame is then a swatch.
#[test]
fn world_art_is_drawn_from_the_palette() {
    let tree = art(&TREE);
    assert_eq!(PALETTE.check(&tree), Ok(()));
    let photo = gradient();
    assert!(
        PALETTE.check(&photo).is_err(),
        "a gradient is off the palette"
    );
    for (mode, name) in [(Mode::Light, "light"), (Mode::Dark, "dark")] {
        let mut rt = Runtime::new(Grove(mode));
        PALETTE
            .register(&mut rt, ImageId(1), &tree, Import::Exact)
            .unwrap();
        // Exact imports are checked in debug builds only.
        let exact = PALETTE.register(&mut rt, ImageId(2), &photo, Import::Exact);
        assert_eq!(exact.is_err(), cfg!(debug_assertions));
        PALETTE
            .register(&mut rt, ImageId(2), &photo, Import::Quantize)
            .unwrap();
        let png = render_png(&mut rt, 192, 72, 2.0, &[]);
        let (rgba, w, h) = astryx_headless::golden::decode_png(&png).unwrap();
        let frame = ImageData {
            width: w,
            height: h,
            rgba: rgba.into(),
        };
        assert_eq!(PALETTE.check(&frame), Ok(()), "Terrace {name}");
        assert_golden(
            &png,
            golden(&format!("terrace-grove-{name}@2x.png")),
            TOLERANCE,
        );
    }
}
