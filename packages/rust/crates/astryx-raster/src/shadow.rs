//! CSS `box-shadow` masks in physical pixels, shared by every renderer.
//!
//! A mask holds the final coverage of one shadow layer: the blurred shape
//! with the box itself cut out for outer shadows, or kept to the inside of
//! the box for inset shadows, as CSS draws them. Renderers tint it with the
//! layer color.

use std::collections::HashMap;
use std::rc::Rc;

use astryx_core::ShadowLayer;

use crate::geom::{spread_radius, PxRect};

/// Coverage of one shadow layer, positioned relative to the top left of the
/// pixel-snapped box that casts it.
#[derive(Debug, PartialEq)]
pub struct ShadowMask {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// Everything a shadow mask depends on. Color is not part of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShadowKey {
    width: u32,
    height: u32,
    radii: [u32; 4],
    x: u32,
    y: u32,
    blur: u32,
    spread: u32,
    inset: bool,
}

impl ShadowKey {
    /// `rect` and `radii` are physical and already fitted; the layer is
    /// logical and scaled here.
    pub fn new(rect: &PxRect, radii: [f32; 4], layer: &ShadowLayer, scale: f32) -> Self {
        Self {
            width: rect.width().max(0.0) as u32,
            height: rect.height().max(0.0) as u32,
            radii: radii.map(f32::to_bits),
            x: (layer.x * scale).to_bits(),
            y: (layer.y * scale).to_bits(),
            blur: (layer.blur.max(0.0) * scale).to_bits(),
            spread: (layer.spread * scale).to_bits(),
            inset: layer.inset,
        }
    }

    /// How a large box's shadow is built from a small one: the mask of
    /// `tile` with its center column repeated `extra.0` more times and its
    /// center row `extra.1` more times. Boxes small enough to compute
    /// directly have no extra. Every box of one size class and style shares
    /// a tile, so tiles are few and cheap to keep.
    pub fn tiling(&self) -> ShadowTiling {
        let radius = self.radii().iter().fold(0.0f32, |a, &r| a.max(r));
        let reach = radius
            + 1.5 * f32::from_bits(self.blur)
            + f32::from_bits(self.spread).abs()
            + f32::from_bits(self.x)
                .abs()
                .max(f32::from_bits(self.y).abs());
        // Half of the reduced size: everything a corner can influence, and margin.
        let k = reach.ceil() as u32 + 4;
        let side = 2 * k + 1;
        if self.width <= side + 2 && self.height <= side + 2 {
            return ShadowTiling {
                tile: *self,
                extra: (0, 0),
            };
        }
        let tile = ShadowKey {
            width: self.width.min(side),
            height: self.height.min(side),
            ..*self
        };
        ShadowTiling {
            tile,
            extra: (self.width - tile.width, self.height - tile.height),
        }
    }

    /// The column and row of `mask` (this key's mask) that a tiled shadow
    /// repeats: the center line of the box.
    pub fn center(&self, mask: &ShadowMask) -> (i32, i32) {
        (
            (self.width / 2) as i32 - mask.x,
            (self.height / 2) as i32 - mask.y,
        )
    }

    fn radii(&self) -> [f32; 4] {
        self.radii.map(f32::from_bits)
    }
}

/// See [`ShadowKey::tiling`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShadowTiling {
    pub tile: ShadowKey,
    pub extra: (u32, u32),
}

/// Computes masks on another thread, for example every theme's shadow
/// tiles at start. Results arrive in batches through the receiver.
#[cfg(not(target_arch = "wasm32"))]
pub fn prewarm(
    keys: Vec<ShadowKey>,
) -> std::sync::mpsc::Receiver<Vec<(ShadowKey, Option<ShadowMask>)>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for chunk in keys.chunks(4) {
            let batch = chunk.iter().map(|k| (*k, shadow_mask(k))).collect();
            if tx.send(batch).is_err() {
                return;
            }
        }
    });
    rx
}

/// Computes the mask for a key. Returns `None` when nothing is visible.
///
/// Far from the corners a shadow's cross-section does not change along the
/// box, so a large box is computed at a reduced size and its middle row or
/// column repeated. The result is the same, at a cost that does not grow
/// with the box.
pub fn shadow_mask(key: &ShadowKey) -> Option<ShadowMask> {
    let tiling = key.tiling();
    if tiling.extra == (0, 0) {
        return direct_mask(key);
    }
    let reduced = tiling.tile;
    let small = direct_mask(&reduced)?;
    let (extra_w, extra_h) = tiling.extra;
    let (w, h) = (small.width + extra_w, small.height + extra_h);
    let (cx, cy) = reduced.center(&small);
    let source = |full: i32, center: i32, extra: u32| -> usize {
        if full <= center {
            full as usize
        } else if full >= center + extra as i32 {
            (full - extra as i32) as usize
        } else {
            center as usize
        }
    };
    let cols: Vec<usize> = (0..w as i32).map(|c| source(c, cx, extra_w)).collect();
    let mut data = Vec::with_capacity((w * h) as usize);
    for row in 0..h as i32 {
        let src = source(row, cy, extra_h) * small.width as usize;
        data.extend(cols.iter().map(|&c| small.data[src + c]));
    }
    Some(ShadowMask {
        x: small.x,
        y: small.y,
        width: w,
        height: h,
        data,
    })
}

fn direct_mask(key: &ShadowKey) -> Option<ShadowMask> {
    let b = PxRect {
        x0: 0.0,
        y0: 0.0,
        x1: key.width as f32,
        y1: key.height as f32,
    };
    if b.is_empty() {
        return None;
    }
    let radii = key.radii();
    let (dx, dy) = (f32::from_bits(key.x), f32::from_bits(key.y));
    let spread = f32::from_bits(key.spread);
    // CSS defines the blur radius as twice the Gaussian standard deviation.
    let sigma = f32::from_bits(key.blur) / 2.0;
    let pad = if sigma > 0.0 {
        (3.0 * sigma).ceil() as i32 + 1
    } else {
        1
    };

    if !key.inset {
        let shape = b.offset(dx, dy).outset(spread);
        if shape.is_empty() {
            return None;
        }
        let shape_radii = radii.map(|r| spread_radius(r, spread));
        let x0 = shape.x0.floor() as i32 - pad;
        let y0 = shape.y0.floor() as i32 - pad;
        let w = (shape.x1.ceil() as i32 + pad - x0) as u32;
        let h = (shape.y1.ceil() as i32 + pad - y0) as u32;
        let mut coverage = coverage(&shape, shape_radii, x0, y0, w, h, false);
        blur(&mut coverage, w as usize, h as usize, sigma);
        let cut = coverage_u8(&b, radii, x0, y0, w, h);
        let data = coverage
            .iter()
            .zip(&cut)
            .map(|(&c, &m)| to_u8(c * (255 - m) as f32 / 255.0))
            .collect();
        Some(ShadowMask {
            x: x0,
            y: y0,
            width: w,
            height: h,
            data,
        })
    } else {
        let hole = b.offset(dx, dy).outset(-spread);
        let hole_radii = radii.map(|r| spread_radius(r, -spread));
        let (x0, y0) = (-pad, -pad);
        let (w, h) = (key.width + 2 * pad as u32, key.height + 2 * pad as u32);
        let mut coverage = if hole.is_empty() {
            vec![1.0; (w * h) as usize]
        } else {
            coverage(&hole, hole_radii, x0, y0, w, h, true)
        };
        blur(&mut coverage, w as usize, h as usize, sigma);
        let keep = coverage_u8(&b, radii, x0, y0, w, h);
        // Crop to the box: an inset shadow never paints outside it.
        let p = pad as usize;
        let mut data = Vec::with_capacity((key.width * key.height) as usize);
        for row in p..p + key.height as usize {
            for col in p..p + key.width as usize {
                let i = row * w as usize + col;
                data.push(to_u8(coverage[i] * keep[i] as f32 / 255.0));
            }
        }
        Some(ShadowMask {
            x: 0,
            y: 0,
            width: key.width,
            height: key.height,
            data,
        })
    }
}

fn to_u8(v: f32) -> u8 {
    (v * 255.0).round().clamp(0.0, 255.0) as u8
}

/// Antialiased coverage of a rounded rectangle in a `w`×`h` buffer whose top
/// left sits at (`x0`, `y0`). Straight edges get the exact pixel overlap;
/// corners use the distance from each pixel center to the arc.
fn coverage_u8(rect: &PxRect, radii: [f32; 4], x0: i32, y0: i32, w: u32, h: u32) -> Vec<u8> {
    let overlap = |lo: f32, hi: f32, p: f32| (hi.min(p + 1.0) - lo.max(p)).clamp(0.0, 1.0);
    let cols: Vec<f32> = (0..w)
        .map(|c| overlap(rect.x0, rect.x1, (x0 + c as i32) as f32))
        .collect();
    let mut out = vec![0u8; (w * h) as usize];
    let [tl, tr, br, bl] = radii;
    for row in 0..h {
        let py = (y0 + row as i32) as f32;
        let cy = overlap(rect.y0, rect.y1, py);
        if cy == 0.0 {
            continue;
        }
        let center_y = py + 0.5;
        for col in 0..w {
            let mut c = cols[col as usize] * cy;
            if c == 0.0 {
                continue;
            }
            let center_x = (x0 + col as i32) as f32 + 0.5;
            let corner = |r: f32, ax: f32, ay: f32, inside_x: bool, inside_y: bool| {
                if r > 0.0 && inside_x && inside_y {
                    (r + 0.5 - (center_x - ax).hypot(center_y - ay)).clamp(0.0, 1.0)
                } else {
                    1.0
                }
            };
            c = c
                .min(corner(
                    tl,
                    rect.x0 + tl,
                    rect.y0 + tl,
                    center_x < rect.x0 + tl,
                    center_y < rect.y0 + tl,
                ))
                .min(corner(
                    tr,
                    rect.x1 - tr,
                    rect.y0 + tr,
                    center_x > rect.x1 - tr,
                    center_y < rect.y0 + tr,
                ))
                .min(corner(
                    br,
                    rect.x1 - br,
                    rect.y1 - br,
                    center_x > rect.x1 - br,
                    center_y > rect.y1 - br,
                ))
                .min(corner(
                    bl,
                    rect.x0 + bl,
                    rect.y1 - bl,
                    center_x < rect.x0 + bl,
                    center_y > rect.y1 - bl,
                ));
            out[(row * w + col) as usize] = to_u8(c);
        }
    }
    out
}

fn coverage(
    rect: &PxRect,
    radii: [f32; 4],
    x0: i32,
    y0: i32,
    w: u32,
    h: u32,
    invert: bool,
) -> Vec<f32> {
    coverage_u8(rect, radii, x0, y0, w, h)
        .into_iter()
        .map(|c| {
            let v = c as f32 / 255.0;
            if invert {
                1.0 - v
            } else {
                v
            }
        })
        .collect()
}

/// Gaussian blur with standard deviation `sigma`, edges clamped. Large blurs
/// use three box blurs sized as the SVG and CSS filter specs prescribe.
fn blur(data: &mut [f32], w: usize, h: usize, sigma: f32) {
    if sigma <= 0.0 || w == 0 || h == 0 {
        return;
    }
    let mut scratch = vec![0.0f32; w.max(h)];
    let mut line = vec![0.0f32; w.max(h)];
    for pass in passes(sigma) {
        for row in 0..h {
            line[..w].copy_from_slice(&data[row * w..row * w + w]);
            pass.run(&line[..w], &mut scratch[..w]);
            data[row * w..row * w + w].copy_from_slice(&scratch[..w]);
        }
        for col in 0..w {
            for row in 0..h {
                line[row] = data[row * w + col];
            }
            pass.run(&line[..h], &mut scratch[..h]);
            for row in 0..h {
                data[row * w + col] = scratch[row];
            }
        }
    }
}

enum Pass {
    Gaussian(Vec<f32>),
    /// Box covering `lo` samples before and `hi` samples after each pixel.
    Box {
        lo: usize,
        hi: usize,
    },
}

fn passes(sigma: f32) -> Vec<Pass> {
    if sigma < 2.0 {
        let radius = (3.0 * sigma).ceil() as i32;
        let mut kernel: Vec<f32> = (-radius..=radius)
            .map(|i| (-(i * i) as f32 / (2.0 * sigma * sigma)).exp())
            .collect();
        let sum: f32 = kernel.iter().sum();
        kernel.iter_mut().for_each(|k| *k /= sum);
        return vec![Pass::Gaussian(kernel)];
    }
    let d = (sigma * 3.0 * (2.0 * std::f32::consts::PI).sqrt() / 4.0 + 0.5).floor() as usize;
    if d % 2 == 1 {
        let r = d / 2;
        (0..3).map(|_| Pass::Box { lo: r, hi: r }).collect()
    } else {
        let r = d / 2;
        vec![
            Pass::Box { lo: r, hi: r - 1 },
            Pass::Box { lo: r - 1, hi: r },
            Pass::Box { lo: r, hi: r },
        ]
    }
}

impl Pass {
    fn run(&self, src: &[f32], dst: &mut [f32]) {
        let n = src.len() as isize;
        let at = |i: isize| src[i.clamp(0, n - 1) as usize] as f64;
        match self {
            Pass::Gaussian(kernel) => {
                let r = (kernel.len() / 2) as isize;
                for (i, out) in dst.iter_mut().enumerate() {
                    let i = i as isize;
                    let mut acc = 0.0f64;
                    for (k, &wgt) in kernel.iter().enumerate() {
                        acc += at(i + k as isize - r) * wgt as f64;
                    }
                    *out = acc as f32;
                }
            }
            Pass::Box { lo, hi } => {
                let (lo, hi) = (*lo as isize, *hi as isize);
                let size = (lo + hi + 1) as f64;
                let mut acc: f64 = (-lo..=hi).map(at).sum();
                for (i, out) in dst.iter_mut().enumerate() {
                    let i = i as isize;
                    *out = (acc / size) as f32;
                    acc += at(i + hi + 1) - at(i - lo);
                }
            }
        }
    }
}

/// Keeps shadow masks across frames. Entries unused for a while are dropped.
#[derive(Default)]
pub struct ShadowCache {
    frame: u64,
    entries: HashMap<ShadowKey, (Option<Rc<ShadowMask>>, u64)>,
}

/// Frames an unused mask stays cached.
const SHADOW_CACHE_FRAMES: u64 = 120;

impl ShadowCache {
    pub fn begin_frame(&mut self) {
        self.frame += 1;
        let frame = self.frame;
        self.entries
            .retain(|_, (_, used)| frame - *used <= SHADOW_CACHE_FRAMES);
    }

    pub fn get(&mut self, key: &ShadowKey) -> Option<Rc<ShadowMask>> {
        let frame = self.frame;
        let entry = self
            .entries
            .entry(*key)
            .or_insert_with(|| (shadow_mask(key).map(Rc::new), frame));
        entry.1 = frame;
        entry.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astryx_core::Color;

    fn layer(x: f32, y: f32, blur: f32, spread: f32, inset: bool) -> ShadowLayer {
        ShadowLayer {
            x,
            y,
            blur,
            spread,
            inset,
            color: Color::rgba(0.0, 0.0, 0.0, 0.5),
        }
    }

    fn key(l: ShadowLayer) -> ShadowKey {
        ShadowKey::new(
            &PxRect {
                x0: 0.0,
                y0: 0.0,
                x1: 40.0,
                y1: 20.0,
            },
            [4.0; 4],
            &l,
            1.0,
        )
    }

    fn at(m: &ShadowMask, x: i32, y: i32) -> u8 {
        m.data[((y - m.y) * m.width as i32 + (x - m.x)) as usize]
    }

    #[test]
    fn outer_shadow_is_cut_out_under_the_box() {
        let m = shadow_mask(&key(layer(0.0, 4.0, 8.0, 0.0, false))).unwrap();
        assert_eq!(at(&m, 20, 10), 0);
        assert!(at(&m, 20, 22) > 60);
        assert_eq!(at(&m, m.x, m.y), 0);
    }

    #[test]
    fn hard_shadow_with_spread_has_a_sharp_edge() {
        let m = shadow_mask(&key(layer(0.0, 0.0, 0.0, 3.0, false))).unwrap();
        assert_eq!(at(&m, 20, -2), 255);
        assert_eq!(at(&m, 20, -4), 0);
    }

    #[test]
    fn blur_reaches_half_at_the_edge() {
        // A Gaussian blurred straight edge is half covered on the edge itself.
        let m = shadow_mask(&ShadowKey::new(
            &PxRect {
                x0: 0.0,
                y0: 0.0,
                x1: 200.0,
                y1: 200.0,
            },
            [0.0; 4],
            &layer(0.0, 30.0, 10.0, 0.0, false),
            1.0,
        ))
        .unwrap();
        let v = (at(&m, 100, 229) as i32 + at(&m, 100, 230) as i32) / 2;
        assert!((v - 127).abs() <= 2, "{v}");
    }

    #[test]
    fn inset_shadow_stays_inside() {
        let m = shadow_mask(&key(layer(0.0, 2.0, 4.0, 0.0, true))).unwrap();
        assert_eq!((m.x, m.y, m.width, m.height), (0, 0, 40, 20));
        assert!(at(&m, 20, 0) > 100);
        assert!(at(&m, 20, 19) < 40);
        assert!(at(&m, 20, 10) < 3);
    }

    #[test]
    fn coverage_is_exact_on_edges_and_round_in_corners() {
        let r = PxRect {
            x0: 0.0,
            y0: 0.0,
            x1: 20.0,
            y1: 10.5,
        };
        let c = coverage_u8(&r, [8.0, 0.0, 0.0, 0.0], 0, 0, 20, 11);
        assert_eq!(c[5 * 20 + 10], 255);
        assert_eq!(c[10 * 20 + 10], 128);
        assert_eq!(c[19], 255);
        assert_eq!(c[0], 0);
        // Halfway along the arc the edge crosses the pixel center.
        let (ax, ay) = (8.0 - 8.0 * 0.5f32.sqrt(), 8.0 - 8.0 * 0.5f32.sqrt());
        let v = c[ay as usize * 20 + ax as usize];
        assert!((40..=230).contains(&v), "{v}");
    }

    #[test]
    fn large_boxes_match_the_direct_computation() {
        for (l, radii) in [
            (layer(0.0, 4.0, 24.0, 0.0, false), [24.0; 4]),
            (layer(3.0, -2.0, 6.0, 4.0, false), [0.0, 12.0, 30.0, 4.0]),
            (layer(0.0, 3.0, 6.0, 1.0, true), [8.0; 4]),
            (layer(6.0, 6.0, 0.0, 0.0, false), [0.0; 4]),
        ] {
            let rect = PxRect {
                x0: 0.0,
                y0: 0.0,
                x1: 700.0,
                y1: 300.0,
            };
            let key = ShadowKey::new(&rect, radii, &l, 2.0);
            let fast = shadow_mask(&key).unwrap();
            let slow = direct_mask(&key).unwrap();
            assert_eq!(
                (fast.x, fast.y, fast.width, fast.height),
                (slow.x, slow.y, slow.width, slow.height)
            );
            let worst = fast
                .data
                .iter()
                .zip(&slow.data)
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            assert!(worst <= 1, "{l:?}: differs by {worst}");
        }
    }

    #[test]
    fn box_blur_preserves_mass() {
        let mut data = vec![0.0; 101];
        data[50] = 1.0;
        blur(&mut data, 101, 1, 6.0);
        let sum: f32 = data.iter().sum();
        assert!((sum - 1.0).abs() < 1e-4);
    }
}
