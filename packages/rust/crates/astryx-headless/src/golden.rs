//! Golden image comparison for tests.
//!
//! A golden is a PNG checked into the repository. [`assert_golden`] compares
//! a rendered PNG against it with a tolerance and, on failure, writes the
//! actual image and a diff image next to the golden. A missing golden is
//! written and the check passes; set `ASTRYX_UPDATE_GOLDENS=1` to rewrite
//! all goldens.

use std::path::{Path, PathBuf};

use astryx_render_soft::tiny_skia::{IntSize, Pixmap};

/// How far a rendering may drift from its golden.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerance {
    /// Largest per-channel difference (0..=255) that still counts as equal.
    pub channel: u8,
    /// How many pixels may differ by more than `channel`.
    pub pixels: usize,
}

impl Tolerance {
    pub const EXACT: Tolerance = Tolerance {
        channel: 0,
        pixels: 0,
    };
}

impl Default for Tolerance {
    fn default() -> Self {
        Tolerance {
            channel: 2,
            pixels: 0,
        }
    }
}

/// The result of comparing two images of the same size.
#[derive(Clone, Debug)]
pub struct Comparison {
    pub width: u32,
    pub height: u32,
    /// Largest per-channel difference over all pixels.
    pub max_diff: u8,
    /// Mean per-channel difference.
    pub mean_diff: f64,
    /// Pixels that differ by more than the tolerance's channel threshold.
    pub differing: usize,
    /// Red where pixels differ, a faded copy of `expected` elsewhere.
    pub diff_image: Pixmap,
}

impl Comparison {
    pub fn passes(&self, tolerance: Tolerance) -> bool {
        self.differing <= tolerance.pixels
    }
}

/// Compares two straight-alpha RGBA images of `width`×`height`.
pub fn compare_rgba(
    actual: &[u8],
    expected: &[u8],
    width: u32,
    height: u32,
    channel: u8,
) -> Comparison {
    assert_eq!(
        actual.len(),
        expected.len(),
        "images must have the same size"
    );
    let mut diff = vec![0u8; actual.len()];
    let (mut max_diff, mut sum, mut differing) = (0u8, 0u64, 0usize);
    for ((a, e), out) in actual
        .as_chunks::<4>()
        .0
        .iter()
        .zip(expected.as_chunks::<4>().0)
        .zip(diff.as_chunks_mut::<4>().0)
    {
        let d = (0..4).map(|k| a[k].abs_diff(e[k])).max().unwrap_or(0);
        sum += (0..4).map(|k| a[k].abs_diff(e[k]) as u64).sum::<u64>();
        max_diff = max_diff.max(d);
        if d > channel {
            differing += 1;
            out.copy_from_slice(&[255, 0, 0, 255]);
        } else {
            let luma = (e[0] as u32 * 3 + e[1] as u32 * 6 + e[2] as u32) / 10;
            let faded = (255 - (255 - luma) / 4) as u8;
            out.copy_from_slice(&[faded, faded, faded, 255]);
        }
    }
    let diff_image = Pixmap::from_vec(
        diff,
        IntSize::from_wh(width.max(1), height.max(1)).expect("size"),
    )
    .expect("diff pixmap");
    Comparison {
        width,
        height,
        max_diff,
        mean_diff: sum as f64 / actual.len().max(1) as f64,
        differing,
        diff_image,
    }
}

/// Decodes a PNG to straight-alpha RGBA bytes and its size.
pub fn decode_png(png: &[u8]) -> Result<(Vec<u8>, u32, u32), String> {
    let pixmap = Pixmap::decode_png(png).map_err(|e| e.to_string())?;
    let (w, h) = (pixmap.width(), pixmap.height());
    Ok((pixmap.take_demultiplied(), w, h))
}

/// Compares two PNG files. Fails when they cannot be decoded or differ in size.
pub fn compare_png(actual: &[u8], expected: &[u8], channel: u8) -> Result<Comparison, String> {
    let (a, aw, ah) = decode_png(actual)?;
    let (e, ew, eh) = decode_png(expected)?;
    if (aw, ah) != (ew, eh) {
        return Err(format!("size {aw}x{ah} differs from expected {ew}x{eh}"));
    }
    Ok(compare_rgba(&a, &e, aw, ah, channel))
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("golden");
    path.with_file_name(format!("{stem}.{suffix}.png"))
}

/// Checks `actual` (PNG bytes) against the golden at `path`. Panics with a
/// summary on failure, after writing `<name>.actual.png` and
/// `<name>.diff.png` beside the golden.
pub fn assert_golden(actual: &[u8], path: impl AsRef<Path>, tolerance: Tolerance) {
    let path = path.as_ref();
    let update = std::env::var_os("ASTRYX_UPDATE_GOLDENS").is_some_and(|v| v != "0");
    if update || !path.exists() {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).expect("create golden directory");
        }
        std::fs::write(path, actual).expect("write golden");
        eprintln!("wrote golden {}", path.display());
        return;
    }
    let expected = std::fs::read(path).expect("read golden");
    let (actual_path, diff_path) = (sibling(path, "actual"), sibling(path, "diff"));
    let failure = match compare_png(actual, &expected, tolerance.channel) {
        Ok(cmp) if cmp.passes(tolerance) => {
            let _ = std::fs::remove_file(&actual_path);
            let _ = std::fs::remove_file(&diff_path);
            return;
        }
        Ok(cmp) => {
            cmp.diff_image
                .save_png(&diff_path)
                .expect("write diff image");
            format!(
                "{} pixels differ by more than {} (allowed {}), max difference {}; diff image {}",
                cmp.differing,
                tolerance.channel,
                tolerance.pixels,
                cmp.max_diff,
                diff_path.display()
            )
        }
        Err(e) => e,
    };
    std::fs::write(&actual_path, actual).expect("write actual image");
    panic!(
        "golden mismatch for {}: {failure}; actual image {}",
        path.display(),
        actual_path.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(color: [u8; 4], w: u32, h: u32) -> Vec<u8> {
        let mut p = Pixmap::new(w, h).unwrap();
        p.fill(astryx_render_soft::tiny_skia::Color::from_rgba8(
            color[0], color[1], color[2], color[3],
        ));
        p.encode_png().unwrap()
    }

    #[test]
    fn identical_images_match() {
        let c = compare_png(
            &png([10, 20, 30, 255], 4, 4),
            &png([10, 20, 30, 255], 4, 4),
            0,
        )
        .unwrap();
        assert_eq!((c.max_diff, c.differing), (0, 0));
    }

    #[test]
    fn differences_are_counted_against_the_threshold() {
        let c = compare_png(
            &png([10, 20, 30, 255], 4, 4),
            &png([13, 20, 30, 255], 4, 4),
            2,
        )
        .unwrap();
        assert_eq!((c.max_diff, c.differing), (3, 16));
        assert!(!c.passes(Tolerance::default()));
        assert!(c.passes(Tolerance {
            channel: 2,
            pixels: 16
        }));
    }

    #[test]
    fn size_mismatch_is_an_error() {
        assert!(compare_png(&png([0; 4], 4, 4), &png([0; 4], 5, 4), 0).is_err());
    }

    #[test]
    fn failure_writes_actual_and_diff() {
        let dir = std::env::temp_dir().join(format!("astryx-golden-{}", std::process::id()));
        let golden = dir.join("sample.png");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&golden, png([0, 0, 0, 255], 4, 4)).unwrap();
        let result = std::panic::catch_unwind(|| {
            assert_golden(&png([255, 0, 0, 255], 4, 4), &golden, Tolerance::default())
        });
        assert!(result.is_err());
        assert!(dir.join("sample.actual.png").exists());
        assert!(dir.join("sample.diff.png").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
