//! Sample images for the gallery, drawn in code so the repository carries no
//! photos: portraits for Avatar and small scenes for Thumbnail.

use astryx_core::{ImageData, ImageId};

/// A portrait: a head and shoulders on a gradient, in one person's colors.
pub const ADA: &str = "gallery/ada.png";
pub const GRACE: &str = "gallery/grace.png";
pub const ALAN: &str = "gallery/alan.png";
pub const KATHERINE: &str = "gallery/katherine.png";
/// Scenes for thumbnails.
pub const BEACH: &str = "gallery/beach.jpg";
pub const HILLS: &str = "gallery/hills.jpg";
pub const CITY: &str = "gallery/city.jpg";
pub const STICKER: &str = "gallery/sticker.png";
/// Registered as failed, so components show their fallback.
pub const BROKEN: &str = "gallery/broken.jpg";
/// Never registered, so components show what they show while loading.
pub const PENDING: &str = "gallery/pending.jpg";

/// Every sample image, by id: `None` marks a source that failed to load.
pub fn samples() -> Vec<(ImageId, Option<ImageData>)> {
    vec![
        (
            ImageId::of(ADA),
            Some(portrait(
                [0.55, 0.36, 0.86],
                [0.98, 0.80, 0.68],
                [0.30, 0.18, 0.12],
            )),
        ),
        (
            ImageId::of(GRACE),
            Some(portrait(
                [0.14, 0.60, 0.70],
                [0.93, 0.75, 0.62],
                [0.85, 0.85, 0.88],
            )),
        ),
        (
            ImageId::of(ALAN),
            Some(portrait(
                [0.95, 0.62, 0.20],
                [0.96, 0.82, 0.70],
                [0.42, 0.26, 0.14],
            )),
        ),
        (
            ImageId::of(KATHERINE),
            Some(portrait(
                [0.20, 0.70, 0.40],
                [0.55, 0.38, 0.27],
                [0.10, 0.08, 0.08],
            )),
        ),
        (ImageId::of(BEACH), Some(beach())),
        (ImageId::of(HILLS), Some(hills())),
        (ImageId::of(CITY), Some(city())),
        (ImageId::of(STICKER), Some(sticker())),
        (ImageId::of(BROKEN), None),
    ]
}

type Rgb = [f32; 3];

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// 0 outside, 1 inside, with a one-pixel soft edge (`d` is a signed distance).
fn cover(d: f32) -> f32 {
    (0.5 - d).clamp(0.0, 1.0)
}

fn image(w: u32, h: u32, f: impl Fn(f32, f32) -> [f32; 4]) -> ImageData {
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let c = f(x as f32 + 0.5, y as f32 + 0.5);
            rgba.extend(c.iter().map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8));
        }
    }
    ImageData {
        width: w,
        height: h,
        rgba: rgba.into(),
    }
}

fn portrait(background: Rgb, skin: Rgb, hair: Rgb) -> ImageData {
    let shirt = mix(background, [0.1, 0.1, 0.15], 0.55);
    image(96, 96, |x, y| {
        let mut c = mix(mix(background, [1.0; 3], 0.35), background, y / 96.0);
        // Shoulders: an ellipse rising from the bottom edge.
        let s = (((x - 48.0) / 38.0).powi(2) + ((y - 100.0) / 30.0).powi(2)).sqrt();
        c = mix(c, shirt, cover((s - 1.0) * 30.0));
        // Neck.
        let neck = (x - 48.0).abs().max((y - 70.0).abs() * 0.8) - 9.0;
        c = mix(c, mix(skin, [0.0; 3], 0.12), cover(neck));
        // Hair behind the head, then the face.
        let hair_d = ((x - 48.0).powi(2) + (y - 40.0).powi(2)).sqrt() - 23.0;
        c = mix(c, hair, cover(hair_d));
        let face_d = (((x - 48.0) / 18.0).powi(2) + ((y - 46.0) / 21.0).powi(2)).sqrt() - 1.0;
        c = mix(c, skin, cover(face_d * 19.0));
        [c[0], c[1], c[2], 1.0]
    })
}

fn beach() -> ImageData {
    image(120, 90, |x, y| {
        let sky = mix([0.55, 0.80, 0.98], [0.85, 0.94, 1.0], y / 40.0);
        let wave = 46.0 + 2.0 * (x / 9.0).sin();
        let sea = mix([0.10, 0.45, 0.70], [0.20, 0.65, 0.80], (y - 40.0) / 20.0);
        let shore = 62.0 + 5.0 * (x / 23.0).sin();
        let sand = mix([0.96, 0.86, 0.62], [0.90, 0.76, 0.50], (y - shore) / 30.0);
        let mut c = sky;
        c = mix(c, sea, cover(40.0 - y));
        c = mix(c, [0.95, 0.98, 1.0], cover((y - wave).abs() - 0.8) * 0.8);
        c = mix(c, sand, cover(shore - y));
        let sun = ((x - 92.0).powi(2) + (y - 16.0).powi(2)).sqrt() - 9.0;
        c = mix(c, [1.0, 0.92, 0.55], cover(sun));
        [c[0], c[1], c[2], 1.0]
    })
}

fn hills() -> ImageData {
    image(120, 90, |x, y| {
        let mut c = mix([0.98, 0.70, 0.45], [0.60, 0.50, 0.75], y / 60.0);
        let sun = ((x - 40.0).powi(2) + (y - 44.0).powi(2)).sqrt() - 14.0;
        c = mix(c, [1.0, 0.85, 0.50], cover(sun));
        let far = 52.0 + 6.0 * (x / 15.0).sin();
        c = mix(c, [0.40, 0.35, 0.55], cover(far - y));
        let near = 66.0 + 8.0 * (x / 21.0 + 1.0).sin();
        c = mix(c, [0.20, 0.30, 0.35], cover(near - y));
        [c[0], c[1], c[2], 1.0]
    })
}

fn city() -> ImageData {
    let heights = [34.0, 52.0, 26.0, 60.0, 40.0, 70.0, 30.0, 48.0];
    image(120, 90, |x, y| {
        let mut c = mix([0.08, 0.10, 0.25], [0.30, 0.20, 0.45], y / 90.0);
        let i = ((x / 15.0) as usize).min(heights.len() - 1);
        let top = 90.0 - heights[i];
        let gap = (x % 15.0 - 7.5).abs() - 6.5;
        let building = cover(top - y).min(cover(gap));
        let tone = 0.12 + 0.04 * (i % 3) as f32;
        c = mix(c, [tone, tone, tone + 0.06], building);
        // Lit windows on a grid.
        let lit = (x as u32 / 3 + y as u32 / 4 + i as u32).is_multiple_of(5) as u8 as f32;
        let window = (x % 3.0 < 1.5 && y % 4.0 < 2.0) as u8 as f32;
        c = mix(c, [1.0, 0.85, 0.45], building * lit * window);
        [c[0], c[1], c[2], 1.0]
    })
}

/// A star on transparency, to show the plate through the image.
fn sticker() -> ImageData {
    image(96, 96, |x, y| {
        let (dx, dy) = (x - 48.0, y - 50.0);
        let r = (dx * dx + dy * dy).sqrt();
        let angle = dy.atan2(dx) + std::f32::consts::FRAC_PI_2;
        let spike = (angle * 2.5).cos().abs().powf(3.0);
        let edge = 24.0 + 16.0 * spike - r;
        let a = cover(-edge);
        [1.0, 0.78, 0.15, a]
    })
}
