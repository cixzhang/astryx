//! Golden images of every fixture from the CPU renderer.

use std::path::PathBuf;

use astryx_headless::fixtures::{FixtureSurface, FIXTURES, FIXTURE_SIZE};
use astryx_headless::golden::{assert_golden, Tolerance};
use astryx_headless::render_png;

// Antialiasing may round differently on other CPUs; layout may not move.
const TOLERANCE: Tolerance = Tolerance {
    channel: 2,
    pixels: 64,
};

fn check(name: &str, scale: f32, inputs: &[astryx_core::Input], golden: &str) {
    let mut surface = FixtureSurface::new(name).unwrap();
    let png = render_png(&mut surface, FIXTURE_SIZE.0, FIXTURE_SIZE.1, scale, inputs);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens")
        .join(golden);
    assert_golden(&png, path, TOLERANCE);
}

#[test]
fn fixtures_at_1x() {
    for name in FIXTURES {
        check(name, 1.0, &[], &format!("{name}@1x.png"));
    }
}

#[test]
fn fixtures_at_2x() {
    for name in ["text", "shadows", "transforms", "bidi", "gallery", "images"] {
        check(name, 2.0, &[], &format!("{name}@2x.png"));
    }
}

#[test]
fn fractional_scale() {
    check("gallery", 1.25, &[], "gallery@1.25x.png");
}

#[test]
fn scripted_input() {
    use astryx_core::Input::{Next, A};
    check("gallery", 1.0, &[Next, Next, A], "gallery-switch-on@1x.png");
}
