//! Golden images of every gallery section, light and dark, from the CPU
//! renderer, plus a few scripted states. Set `ASTRYX_UPDATE_GOLDENS=1` to
//! rewrite them after an intended visual change.

use astryx_core::{Input, Mode, Runtime, Theme};
use astryx_gallery::{Gallery, Section};
use astryx_headless::golden::{assert_golden, Tolerance};
mod settle;

use settle::render_png;
use std::path::PathBuf;

// Antialiasing may round differently on other CPUs; layout may not move.
const TOLERANCE: Tolerance = Tolerance {
    channel: 2,
    pixels: 64,
};
const SIZE: (u32, u32) = (720, 900);

fn check(gallery: Gallery, scale: f32, inputs: &[Input], golden: &str) {
    let mut rt = astryx_gallery::runtime(gallery);
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

/// Focus at a scroll container's edge and inside a card with no padding.
struct Edges;

impl astryx_core::App for Edges {
    type Msg = u8;
    fn view(&self) -> astryx_core::Element<u8> {
        use astryx_core::*;
        VStack::new()
            .is_scrollable(true)
            .child(
                Button::new("Flush with the edge")
                    .variant(ButtonVariant::Primary)
                    .on_click(0),
            )
            .child(
                Card::new()
                    .padding(Space::S0)
                    .child(Button::new("Flush in a card").on_click(1)),
            )
            .children((0..12).map(|i| Text::new(format!("Line {i}"))))
            .into()
    }
    fn update(&mut self, _: u8) -> astryx_core::Command {
        astryx_core::Command::None
    }
}

#[test]
fn focus_rings_at_edges_are_whole() {
    for (inputs, name) in [
        (&[][..], "focus-ring-scroll-edge.png"),
        (&[Input::Next][..], "focus-ring-card-edge.png"),
    ] {
        let mut rt = Runtime::new(Edges);
        let png = render_png(&mut rt, 240, 160, 2.0, inputs);
        assert_golden(
            &png,
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/goldens")
                .join(name),
            TOLERANCE,
        );
    }
}

#[test]
fn header_at_scale_3() {
    let mut rt = astryx_gallery::runtime(Gallery::new());
    let png = render_png(&mut rt, 960, 250, 3.0, &[]);
    assert_golden(
        &png,
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/goldens/header@3x.png"),
        TOLERANCE,
    );
}

#[test]
fn tooltip_states() {
    use Input::Next;
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/goldens");
    // Twenty-one stops in reading order reach the Share button.
    let to_share: Vec<Input> = std::iter::repeat_n(Next, 21).collect();
    for (inputs, name) in [
        (to_share.clone(), "tooltip-button.png"),
        (
            [to_share.clone(), vec![Next]].concat(),
            "tooltip-disabled-reason.png",
        ),
        ([to_share, vec![Input::B]].concat(), "tooltip-dismissed.png"),
    ] {
        let mut rt = Runtime::new(Gallery::section(Section::Buttons));
        let png = render_png(&mut rt, SIZE.0, SIZE.1, 1.0, &inputs);
        assert_golden(&png, dir.join(name), TOLERANCE);
    }
}

#[test]
fn right_to_left_sections() {
    use astryx_core::Direction;
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/goldens");
    for section in [Section::Switches, Section::ListItems, Section::ProgressBars] {
        for (mode, m) in [(Mode::Light, "light"), (Mode::Dark, "dark")] {
            let gallery = Gallery::section(section)
                .with_theme(Theme::ASTRYX, mode)
                .with_direction(Direction::Rtl);
            let mut rt = Runtime::new(gallery);
            let png = render_png(&mut rt, SIZE.0, SIZE.1, 1.0, &[]);
            assert_golden(
                &png,
                dir.join(format!("{}-rtl-{m}.png", section.slug())),
                TOLERANCE,
            );
        }
    }
    // The home screen, with its forward chevrons swapped.
    let mut rt = Runtime::new(Gallery::new().with_direction(Direction::Rtl));
    assert_golden(
        &render_png(&mut rt, 960, 640, 1.0, &[]),
        dir.join("home-rtl.png"),
        TOLERANCE,
    );
}
