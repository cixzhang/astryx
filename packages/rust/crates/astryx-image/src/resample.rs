//! Area-average downscaling, fed one source row at a time.
//!
//! Each output pixel is the average of the source area it covers, with
//! partial coverage at the edges, computed on premultiplied alpha so
//! transparent pixels do not darken their neighbours. Only one row of
//! accumulators is held besides the output.

use astryx_core::ImageData;

pub struct Resampler {
    src_w: usize,
    src_h: usize,
    out_w: usize,
    out_h: usize,
    /// For each output column: (first source column, weights per source column).
    columns: Vec<(usize, Vec<f32>)>,
    /// Source rows per output row.
    y_scale: f64,
    /// The current output row's premultiplied sums.
    acc: Vec<f32>,
    /// Scratch for one horizontally resampled source row.
    row: Vec<f32>,
    src_y: usize,
    out_y: usize,
    out: Vec<u8>,
}

impl Resampler {
    /// Downscales `src_w` x `src_h` to `out_w` x `out_h`; neither side grows.
    pub fn new(src_w: u32, src_h: u32, out_w: u32, out_h: u32) -> Self {
        let (src_w, src_h) = (src_w as usize, src_h as usize);
        let out_w = (out_w as usize).clamp(1, src_w);
        let out_h = (out_h as usize).clamp(1, src_h);
        let x_scale = src_w as f64 / out_w as f64;
        let columns = (0..out_w)
            .map(|ox| {
                let (a, b) = (ox as f64 * x_scale, (ox + 1) as f64 * x_scale);
                let first = a.floor() as usize;
                let last = (b.ceil() as usize).min(src_w);
                let weights = (first..last)
                    .map(|sx| {
                        let lo = (sx as f64).max(a);
                        let hi = ((sx + 1) as f64).min(b);
                        ((hi - lo) / x_scale) as f32
                    })
                    .collect();
                (first, weights)
            })
            .collect();
        Resampler {
            src_w,
            src_h,
            out_w,
            out_h,
            columns,
            y_scale: src_h as f64 / out_h as f64,
            acc: vec![0.0; out_w * 4],
            row: vec![0.0; out_w * 4],
            src_y: 0,
            out_y: 0,
            out: Vec::with_capacity(out_w * out_h * 4),
        }
    }

    /// Adds the next source row: `src_w` straight-alpha RGBA pixels.
    pub fn push_row(&mut self, rgba: &[u8]) {
        if self.src_y >= self.src_h || rgba.len() < self.src_w * 4 {
            return;
        }
        for (ox, (first, weights)) in self.columns.iter().enumerate() {
            let mut sum = [0.0f32; 4];
            for (i, &wgt) in weights.iter().enumerate() {
                let p = &rgba[(first + i) * 4..(first + i) * 4 + 4];
                let a = p[3] as f32 / 255.0;
                sum[0] += p[0] as f32 * a * wgt;
                sum[1] += p[1] as f32 * a * wgt;
                sum[2] += p[2] as f32 * a * wgt;
                sum[3] += a * wgt;
            }
            self.row[ox * 4..ox * 4 + 4].copy_from_slice(&sum);
        }
        // This source row covers [src_y, src_y + 1); split it between the
        // output rows it overlaps (at most two when downscaling).
        let (top, bottom) = (self.src_y as f64, (self.src_y + 1) as f64);
        let mut lo = top;
        while lo < bottom && self.out_y < self.out_h {
            let row_end = (self.out_y + 1) as f64 * self.y_scale;
            let hi = bottom.min(row_end);
            let wgt = ((hi - lo) / self.y_scale) as f32;
            for (a, r) in self.acc.iter_mut().zip(&self.row) {
                *a += r * wgt;
            }
            if hi >= row_end - 1e-9 {
                self.emit();
            }
            lo = hi;
        }
        self.src_y += 1;
    }

    fn emit(&mut self) {
        for px in self.acc.as_chunks::<4>().0 {
            let a = px[3];
            if a <= 1e-6 {
                self.out.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                let c = |v: f32| (v / a).round().clamp(0.0, 255.0) as u8;
                self.out.extend_from_slice(&[
                    c(px[0]),
                    c(px[1]),
                    c(px[2]),
                    (a * 255.0).round().clamp(0.0, 255.0) as u8,
                ]);
            }
        }
        self.acc.iter_mut().for_each(|v| *v = 0.0);
        self.out_y += 1;
    }

    /// The downscaled image. Rows never pushed come out transparent.
    pub fn finish(mut self) -> ImageData {
        while self.out_y < self.out_h {
            self.emit();
        }
        ImageData {
            width: self.out_w as u32,
            height: self.out_h as u32,
            rgba: self.out.into(),
        }
    }
}
