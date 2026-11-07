//! The `check-palette` command, as CI runs it.
#![cfg(feature = "cli")]

use astryx_headless::astryx_render_soft::tiny_skia::{IntSize, Pixmap};
use std::process::Command;

fn png(path: &std::path::Path, pixels: &[[u8; 4]], width: u32) {
    let size = IntSize::from_wh(width, pixels.len() as u32 / width).unwrap();
    Pixmap::from_vec(pixels.concat(), size)
        .unwrap()
        .save_png(path)
        .unwrap();
}

#[test]
fn passes_palette_art_and_fails_off_palette_pixels() {
    let dir = std::env::temp_dir().join(format!("check-palette-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (good, bad) = (dir.join("good.png"), dir.join("bad.png"));
    let ink = [0x12, 0x29, 0x53, 255];
    png(&good, &[ink, [0, 0, 0, 0], ink, ink], 2);
    png(&bad, &[ink, [0, 0, 0, 255], ink, ink], 2);
    let run = |files: &[&std::path::Path]| {
        Command::new(env!("CARGO_BIN_EXE_check-palette"))
            .args(files)
            .output()
            .unwrap()
    };
    let ok = run(&[&good]);
    assert!(ok.status.success());
    let failed = run(&[&good, &bad]);
    assert_eq!(failed.status.code(), Some(1));
    let out = String::from_utf8(failed.stdout).unwrap();
    assert!(
        out.contains("bad.png: 1 off-palette pixels: (1, 0) #000000"),
        "{out}"
    );
    assert!(out.contains("1 of 2 files are off the palette"), "{out}");
    std::fs::remove_dir_all(&dir).unwrap();
}
