//! Decodes PNG, JPEG, and WebP bytes into [`ImageData`] (straight-alpha
//! RGBA8), downscaled so a small-memory device never holds a full-size photo.
//!
//! ```no_run
//! # let bytes: &[u8] = &[];
//! // At most the screen's size, keeping the aspect ratio.
//! let data = astryx_image::decode_to_fit(bytes, 1024, 768).unwrap();
//! // Just big enough to cover a 64 x 64 thumbnail at scale 3.
//! let thumb = astryx_image::decode_to_cover(bytes, 192, 192).unwrap();
//! ```
//!
//! Memory: a JPEG decodes at 1/2, 1/4, or 1/8 size when that still fits
//! (the decoder's DCT scaling), and a non-interlaced PNG is downscaled row by
//! row as it decodes, so neither holds the full-size pixels. WebP decodes
//! whole, then downscales.
//!
//! Colors are taken as sRGB: embedded color profiles and PNG gamma are not
//! applied. A JPEG's EXIF orientation is applied. An animated image gives its
//! first frame.

mod resample;

use astryx_core::ImageData;
use resample::Resampler;
use std::io::Cursor;

/// An image format this crate decodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Png,
    Jpeg,
    WebP,
}

/// Why decoding failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// Not PNG, JPEG, or WebP.
    Unsupported,
    /// The bytes are damaged or use a feature the decoder lacks.
    Decode(String),
    /// Wider or taller than [`MAX_SIDE`].
    TooLarge { width: u32, height: u32 },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Unsupported => write!(f, "not a PNG, JPEG, or WebP image"),
            Error::Decode(e) => write!(f, "image decode failed: {e}"),
            Error::TooLarge { width, height } => write!(f, "image too large: {width}x{height}"),
        }
    }
}

impl std::error::Error for Error {}

/// The largest width or height accepted, before downscaling.
pub const MAX_SIDE: u32 = 16_384;

/// The format of `bytes`, from its signature.
pub fn format(bytes: &[u8]) -> Option<Format> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(Format::Png)
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(Format::Jpeg)
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(Format::WebP)
    } else {
        None
    }
}

/// How the output size follows from the image's size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Full,
    /// As large as fits inside the box.
    Fit(u32, u32),
    /// As small as still covers the box.
    Cover(u32, u32),
}

impl Target {
    /// The output size for an image of `w` x `h`. Never upscales.
    fn size(self, w: u32, h: u32) -> (u32, u32) {
        let (fw, fh) = (w as f64, h as f64);
        let scale = match self {
            Target::Full => 1.0,
            Target::Fit(bw, bh) => (bw as f64 / fw).min(bh as f64 / fh),
            Target::Cover(bw, bh) => (bw as f64 / fw).max(bh as f64 / fh),
        }
        .min(1.0);
        if scale >= 1.0 {
            return (w, h);
        }
        (
            ((fw * scale).round() as u32).max(1),
            ((fh * scale).round() as u32).max(1),
        )
    }
}

/// Decodes at full size.
pub fn decode(bytes: &[u8]) -> Result<ImageData, Error> {
    decode_with(bytes, Target::Full)
}

/// Decodes and downscales to fit inside `max_width` x `max_height`, keeping
/// the aspect ratio. Smaller images keep their size.
pub fn decode_to_fit(bytes: &[u8], max_width: u32, max_height: u32) -> Result<ImageData, Error> {
    decode_with(bytes, Target::Fit(max_width.max(1), max_height.max(1)))
}

/// Decodes and downscales to the smallest size that still covers
/// `width` x `height`, for an image drawn with `ImageFit::Cover`.
pub fn decode_to_cover(bytes: &[u8], width: u32, height: u32) -> Result<ImageData, Error> {
    decode_with(bytes, Target::Cover(width.max(1), height.max(1)))
}

/// Downscales decoded pixels to fit inside `max_width` x `max_height`.
pub fn downscale_to_fit(image: &ImageData, max_width: u32, max_height: u32) -> ImageData {
    let (w, h) = Target::Fit(max_width.max(1), max_height.max(1)).size(image.width, image.height);
    resample_rgba(image.width, image.height, &image.rgba, w, h)
}

fn decode_with(bytes: &[u8], target: Target) -> Result<ImageData, Error> {
    match format(bytes).ok_or(Error::Unsupported)? {
        Format::Png => decode_png(bytes, target),
        Format::Jpeg => decode_jpeg(bytes, target),
        Format::WebP => decode_webp(bytes, target),
    }
}

fn check_size(width: u32, height: u32) -> Result<(), Error> {
    if width == 0 || height == 0 {
        return Err(Error::Decode("empty image".into()));
    }
    if width > MAX_SIDE || height > MAX_SIDE {
        return Err(Error::TooLarge { width, height });
    }
    Ok(())
}

fn resample_rgba(w: u32, h: u32, rgba: &[u8], out_w: u32, out_h: u32) -> ImageData {
    if (w, h) == (out_w, out_h) {
        return ImageData {
            width: w,
            height: h,
            rgba: rgba.into(),
        };
    }
    let mut r = Resampler::new(w, h, out_w, out_h);
    for row in rgba.chunks_exact(w as usize * 4) {
        r.push_row(row);
    }
    r.finish()
}

fn decode_png(bytes: &[u8], target: Target) -> Result<ImageData, Error> {
    let err = |e: png::DecodingError| Error::Decode(e.to_string());
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(err)?;
    let (w, h) = (reader.info().width, reader.info().height);
    check_size(w, h)?;
    let interlaced = reader.info().interlaced;
    let (color, depth) = reader.output_color_type();
    if depth != png::BitDepth::Eight {
        return Err(Error::Decode(format!("unexpected bit depth {depth:?}")));
    }
    let to_rgba = move |src: &[u8], dst: &mut Vec<u8>| {
        dst.clear();
        match color {
            png::ColorType::Rgba => dst.extend_from_slice(src),
            png::ColorType::Rgb => {
                for p in src.as_chunks::<3>().0 {
                    dst.extend_from_slice(&[p[0], p[1], p[2], 255]);
                }
            }
            png::ColorType::GrayscaleAlpha => {
                for p in src.as_chunks::<2>().0 {
                    dst.extend_from_slice(&[p[0], p[0], p[0], p[1]]);
                }
            }
            png::ColorType::Grayscale => {
                for &g in src {
                    dst.extend_from_slice(&[g, g, g, 255]);
                }
            }
            // EXPAND turns palettes into RGB or RGBA.
            png::ColorType::Indexed => {}
        }
    };
    let (out_w, out_h) = target.size(w, h);
    let mut line = Vec::with_capacity(w as usize * 4);
    if interlaced {
        // Interlaced rows arrive in passes, so decode the whole frame first.
        let mut buf = vec![
            0;
            reader.output_buffer_size().ok_or(Error::TooLarge {
                width: w,
                height: h
            })?
        ];
        let info = reader.next_frame(&mut buf).map_err(err)?;
        let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
        for row in buf[..info.buffer_size()].chunks_exact(info.line_size) {
            to_rgba(row, &mut line);
            rgba.extend_from_slice(&line);
        }
        return Ok(resample_rgba(w, h, &rgba, out_w, out_h));
    }
    let mut r = Resampler::new(w, h, out_w, out_h);
    while let Some(row) = reader.next_row().map_err(err)? {
        to_rgba(row.data(), &mut line);
        r.push_row(&line);
    }
    Ok(r.finish())
}

fn decode_jpeg(bytes: &[u8], target: Target) -> Result<ImageData, Error> {
    let err = |e: jpeg_decoder::Error| Error::Decode(e.to_string());
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(bytes));
    decoder.read_info().map_err(err)?;
    let info = decoder
        .info()
        .ok_or_else(|| Error::Decode("no frame".into()))?;
    let (w, h) = (info.width as u32, info.height as u32);
    check_size(w, h)?;
    let orientation = decoder.exif_data().map(exif_orientation).unwrap_or(1);
    let swaps = orientation >= 5;
    // The box applies to the image as shown, which is rotated for 5 to 8.
    let shown = if swaps { (h, w) } else { (w, h) };
    let (out_shown_w, out_shown_h) = target.size(shown.0, shown.1);
    let (out_w, out_h) = if swaps {
        (out_shown_h, out_shown_w)
    } else {
        (out_shown_w, out_shown_h)
    };
    if (out_w, out_h) != (w, h) {
        // Decode at the smallest DCT scale that is still at least the output size.
        decoder
            .scale(
                out_w.min(u16::MAX as u32) as u16,
                out_h.min(u16::MAX as u32) as u16,
            )
            .map_err(err)?;
    }
    let pixels = decoder.decode().map_err(err)?;
    let info = decoder
        .info()
        .ok_or_else(|| Error::Decode("no frame".into()))?;
    let (dw, dh) = (info.width as u32, info.height as u32);
    let mut rgba = Vec::with_capacity(dw as usize * dh as usize * 4);
    match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => {
            for p in pixels.as_chunks::<3>().0 {
                rgba.extend_from_slice(&[p[0], p[1], p[2], 255]);
            }
        }
        jpeg_decoder::PixelFormat::L8 => {
            for &g in &pixels {
                rgba.extend_from_slice(&[g, g, g, 255]);
            }
        }
        jpeg_decoder::PixelFormat::L16 => {
            for p in pixels.as_chunks::<2>().0 {
                // Big-endian samples; keep the high byte.
                rgba.extend_from_slice(&[p[0], p[0], p[0], 255]);
            }
        }
        jpeg_decoder::PixelFormat::CMYK32 => {
            for p in pixels.as_chunks::<4>().0 {
                let k = 1.0 - p[3] as f32 / 255.0;
                let ch = |c: u8| ((255.0 - c as f32) * k) as u8;
                rgba.extend_from_slice(&[ch(p[0]), ch(p[1]), ch(p[2]), 255]);
            }
        }
    }
    let image = resample_rgba(dw, dh, &rgba, out_w.min(dw), out_h.min(dh));
    Ok(orient(image, orientation))
}

fn decode_webp(bytes: &[u8], target: Target) -> Result<ImageData, Error> {
    let err = |e: image_webp::DecodingError| Error::Decode(e.to_string());
    let mut decoder = image_webp::WebPDecoder::new(Cursor::new(bytes)).map_err(err)?;
    let (w, h) = decoder.dimensions();
    check_size(w, h)?;
    let alpha = decoder.has_alpha();
    let size = decoder.output_buffer_size().ok_or(Error::TooLarge {
        width: w,
        height: h,
    })?;
    let mut buf = vec![0; size];
    decoder.read_image(&mut buf).map_err(err)?;
    let rgba = if alpha {
        buf
    } else {
        let mut out = Vec::with_capacity(w as usize * h as usize * 4);
        for p in buf.as_chunks::<3>().0 {
            out.extend_from_slice(&[p[0], p[1], p[2], 255]);
        }
        out
    };
    let (out_w, out_h) = target.size(w, h);
    Ok(resample_rgba(w, h, &rgba, out_w, out_h))
}

/// The EXIF orientation (1 to 8) in TIFF-formatted EXIF data, or 1.
fn exif_orientation(tiff: &[u8]) -> u8 {
    let le = match tiff.get(0..2) {
        Some(b"II") => true,
        Some(b"MM") => false,
        _ => return 1,
    };
    let u16_at = |i: usize| -> Option<u16> {
        let b = tiff.get(i..i + 2)?;
        Some(if le {
            u16::from_le_bytes([b[0], b[1]])
        } else {
            u16::from_be_bytes([b[0], b[1]])
        })
    };
    let u32_at = |i: usize| -> Option<u32> {
        let b = tiff.get(i..i + 4)?;
        let a = [b[0], b[1], b[2], b[3]];
        Some(if le {
            u32::from_le_bytes(a)
        } else {
            u32::from_be_bytes(a)
        })
    };
    let find = || -> Option<u8> {
        let ifd = u32_at(4)? as usize;
        let count = u16_at(ifd)? as usize;
        for i in 0..count {
            let entry = ifd + 2 + i * 12;
            if u16_at(entry)? == 0x0112 {
                let v = u16_at(entry + 8)?;
                return (1..=8).contains(&v).then_some(v as u8);
            }
        }
        None
    };
    find().unwrap_or(1)
}

/// Applies an EXIF orientation so the pixels are upright.
fn orient(image: ImageData, orientation: u8) -> ImageData {
    if orientation <= 1 || orientation > 8 {
        return image;
    }
    let (w, h) = (image.width as usize, image.height as usize);
    let swaps = orientation >= 5;
    let (ow, oh) = if swaps { (h, w) } else { (w, h) };
    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            // Where source pixel (x, y) lands, per the EXIF definition.
            let (nx, ny) = match orientation {
                2 => (w - 1 - x, y),
                3 => (w - 1 - x, h - 1 - y),
                4 => (x, h - 1 - y),
                5 => (y, x),
                6 => (h - 1 - y, x),
                7 => (h - 1 - y, w - 1 - x),
                _ => (y, w - 1 - x),
            };
            let s = (y * w + x) * 4;
            let d = (ny * ow + nx) * 4;
            out[d..d + 4].copy_from_slice(&image.rgba[s..s + 4]);
        }
    }
    ImageData {
        width: ow as u32,
        height: oh as u32,
        rgba: out.into(),
    }
}

#[cfg(test)]
mod tests;
