//! Golden images of every gallery section, light and dark, from the CPU
//! renderer, plus a few scripted states. Set `ASTRYX_UPDATE_GOLDENS=1` to
//! rewrite them after an intended visual change.

use astryx_core::{Input, Mode, Runtime, Theme};
use astryx_gallery::{Gallery, Section};
use astryx_headless::golden::{assert_golden, Tolerance};
use astryx_headless::render_png;
use std::path::PathBuf;

// Antialiasing may round differently on other CPUs; layout may not move.
const TOLERANCE: Tolerance = Tolerance {
    channel: 2,
    pixels: 64,
};
const SIZE: (u32, u32) = (720, 900);

fn check(gallery: Gallery, scale: f32, inputs: &[Input], golden: &str) {
    let mut rt = Runtime::new(gallery);
    let png = render_png(&mut rt, SIZE.0, SIZE.1, scale, inputs);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens")
        .join(golden);
    assert_golden(&png, path, TOLERANCE);
}

#[test]
fn every_section_light_and_dark() {
    for section in Section::ALL {
        for (mode, name) in [(Mode::Light, "light"), (Mode::Dark, "dark")] {
            let gallery = Gallery::section(section).with_theme(Theme::ASTRYX, mode);
            check(gallery, 1.0, &[], &format!("{}-{name}.png", section.slug()));
        }
    }
}

#[test]
fn scripted_states() {
    use Input::{Down, Next, A};
    // The first switch toggled on, with focus on it.
    check(
        Gallery::section(Section::Switches),
        1.0,
        &[A],
        "switches-toggled.png",
    );
    // The second row selected after moving focus down.
    check(
        Gallery::section(Section::ListItems),
        1.0,
        &[Down, A],
        "list-items-selected.png",
    );
    // The first selector's menu open on its second option.
    check(
        Gallery::section(Section::Selectors),
        1.0,
        &[A, Down],
        "selectors-open.png",
    );
    // Focus on a disabled button.
    check(
        Gallery::section(Section::Buttons),
        1.0,
        &[Next, Next, Next, Next],
        "buttons-focus-disabled.png",
    );
}

#[test]
fn another_theme_at_2x() {
    let gallery = Gallery::section(Section::Buttons).with_theme(Theme::GOTHIC, Mode::Dark);
    check(gallery, 2.0, &[], "buttons-gothic-dark@2x.png");
}
