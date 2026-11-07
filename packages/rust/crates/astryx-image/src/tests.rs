use super::*;

/// A `w` x `h` image whose left half is red and right half blue, with alpha.
fn halves(w: u32, h: u32, alpha: u8) -> Vec<u8> {
    let mut v = Vec::new();
    for _ in 0..h {
        for x in 0..w {
            let p = if x < w / 2 {
                [255, 0, 0, alpha]
            } else {
                [0, 0, 255, alpha]
            };
            v.extend_from_slice(&p);
        }
    }
    v
}

fn png(w: u32, h: u32, rgba: &[u8], color: png::ColorType) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut e = png::Encoder::new(&mut out, w, h);
        e.set_color(color);
        e.set_depth(png::BitDepth::Eight);
        let mut writer = e.write_header().unwrap();
        writer.write_image_data(rgba).unwrap();
    }
    out
}

fn jpeg(w: u16, h: u16, rgb: &[u8], exif_orientation: Option<u16>) -> Vec<u8> {
    let mut out = Vec::new();
    let mut e = jpeg_encoder::Encoder::new(&mut out, 95);
    if let Some(o) = exif_orientation {
        // "Exif\0\0", a big-endian TIFF header, one IFD with one entry.
        let mut app1 = b"Exif\0\0MM\0\x2a\0\0\0\x08\0\x01".to_vec();
        app1.extend_from_slice(&[0x01, 0x12, 0, 3, 0, 0, 0, 1]);
        app1.extend_from_slice(&o.to_be_bytes());
        app1.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
        e.add_app_segment(1, &app1).unwrap();
    }
    e.encode(rgb, w, h, jpeg_encoder::ColorType::Rgb).unwrap();
    out
}

fn webp(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    image_webp::WebPEncoder::new(&mut out)
        .encode(rgba, w, h, image_webp::ColorType::Rgba8)
        .unwrap();
    out
}

fn px(img: &ImageData, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * img.width + x) * 4) as usize;
    img.rgba[i..i + 4].try_into().unwrap()
}

fn close(a: [u8; 4], b: [u8; 4], tol: u8) -> bool {
    a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= tol)
}

#[test]
fn detects_formats() {
    assert_eq!(
        format(&png(1, 1, &[0, 0, 0, 0], png::ColorType::Rgba)),
        Some(Format::Png)
    );
    assert_eq!(format(&jpeg(8, 8, &[0; 192], None)), Some(Format::Jpeg));
    assert_eq!(format(&webp(1, 1, &[0, 0, 0, 0])), Some(Format::WebP));
    assert_eq!(format(b"GIF89a"), None);
    assert_eq!(decode(b"GIF89a....").unwrap_err(), Error::Unsupported);
}

#[test]
fn png_decodes_every_color_type_to_rgba() {
    let rgba = halves(4, 2, 200);
    let img = decode(&png(4, 2, &rgba, png::ColorType::Rgba)).unwrap();
    assert_eq!((img.width, img.height), (4, 2));
    assert_eq!(&img.rgba[..], &rgba[..]);
    let rgb: Vec<u8> = rgba.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
    let img = decode(&png(4, 2, &rgb, png::ColorType::Rgb)).unwrap();
    assert_eq!(px(&img, 0, 0), [255, 0, 0, 255]);
    let gray = decode(&png(2, 1, &[10, 200], png::ColorType::Grayscale)).unwrap();
    assert_eq!(px(&gray, 1, 0), [200, 200, 200, 255]);
    let ga = decode(&png(1, 1, &[50, 128], png::ColorType::GrayscaleAlpha)).unwrap();
    assert_eq!(px(&ga, 0, 0), [50, 50, 50, 128]);
}

#[test]
fn png_downscales_while_streaming() {
    let rgba = halves(64, 32, 255);
    let a = decode_to_fit(&png(64, 32, &rgba, png::ColorType::Rgba), 16, 16).unwrap();
    assert_eq!((a.width, a.height), (16, 8));
    assert_eq!(px(&a, 0, 0), [255, 0, 0, 255]);
    assert_eq!(px(&a, 15, 7), [0, 0, 255, 255]);
    // Downscaling decoded pixels gives the same result.
    let full = decode(&png(64, 32, &rgba, png::ColorType::Rgba)).unwrap();
    assert_eq!(downscale_to_fit(&full, 16, 16), a);
}

#[test]
fn fit_and_cover_choose_sizes_and_never_upscale() {
    let bytes = png(200, 100, &halves(200, 100, 255), png::ColorType::Rgba);
    let fit = decode_to_fit(&bytes, 50, 50).unwrap();
    assert_eq!((fit.width, fit.height), (50, 25));
    let cover = decode_to_cover(&bytes, 50, 50).unwrap();
    assert_eq!((cover.width, cover.height), (100, 50));
    let small = decode_to_fit(&bytes, 1000, 1000).unwrap();
    assert_eq!((small.width, small.height), (200, 100));
}

#[test]
fn averaging_uses_premultiplied_alpha() {
    // An opaque red pixel beside a transparent black one averages to
    // half-transparent red, not dark red.
    let rgba = [255, 0, 0, 255, 0, 0, 0, 0];
    let img = downscale_to_fit(
        &ImageData {
            width: 2,
            height: 1,
            rgba: rgba.as_slice().into(),
        },
        1,
        1,
    );
    assert_eq!(px(&img, 0, 0), [255, 0, 0, 128]);
}

#[test]
fn fractional_downscale_keeps_the_average() {
    let gray: Vec<u8> = (0..30)
        .flat_map(|i| {
            let g = if i % 3 == 0 { 0 } else { 255 };
            [g, g, g, 255]
        })
        .collect();
    let img = downscale_to_fit(
        &ImageData {
            width: 30,
            height: 1,
            rgba: gray.into(),
        },
        7,
        7,
    );
    assert_eq!(img.width, 7);
    for x in 0..7 {
        let v = px(&img, x, 0)[0] as i32;
        assert!((v - 170).abs() <= 40, "column {x}: {v}");
    }
}

#[test]
fn jpeg_decodes_and_scales_with_the_dct() {
    let rgb: Vec<u8> = halves(64, 64, 255)
        .chunks(4)
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    let bytes = jpeg(64, 64, &rgb, None);
    let full = decode(&bytes).unwrap();
    assert_eq!((full.width, full.height), (64, 64));
    assert!(close(px(&full, 8, 32), [255, 0, 0, 255], 12));
    let small = decode_to_fit(&bytes, 10, 10).unwrap();
    assert_eq!((small.width, small.height), (10, 10));
    assert!(close(px(&small, 1, 5), [255, 0, 0, 255], 16));
    assert!(close(px(&small, 8, 5), [0, 0, 255, 255], 16));
}

#[test]
fn jpeg_exif_orientation_is_applied() {
    // 16 wide, 8 tall; red on the left. Orientation 6 rotates it 90° clockwise:
    // 8 wide, 16 tall, red at the top.
    let rgb: Vec<u8> = halves(16, 8, 255)
        .chunks(4)
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    let img = decode(&jpeg(16, 8, &rgb, Some(6))).unwrap();
    assert_eq!((img.width, img.height), (8, 16));
    assert!(close(px(&img, 4, 2), [255, 0, 0, 255], 16));
    assert!(close(px(&img, 4, 13), [0, 0, 255, 255], 16));
    // The fit box applies to the image as shown.
    let fit = decode_to_fit(&jpeg(16, 8, &rgb, Some(6)), 4, 8).unwrap();
    assert_eq!((fit.width, fit.height), (4, 8));
    // Orientation 3 turns it upside down: red on the right.
    let flipped = decode(&jpeg(16, 8, &rgb, Some(3))).unwrap();
    assert!(close(px(&flipped, 13, 4), [255, 0, 0, 255], 16));
}

#[test]
fn webp_decodes_with_alpha_and_downscales() {
    let rgba = halves(32, 16, 255);
    let img = decode(&webp(32, 16, &rgba)).unwrap();
    assert_eq!(&img.rgba[..], &rgba[..]);
    let small = decode_to_fit(&webp(32, 16, &rgba), 8, 8).unwrap();
    assert_eq!((small.width, small.height), (8, 4));
    assert_eq!(px(&small, 0, 0), [255, 0, 0, 255]);
}

#[test]
fn damaged_bytes_fail_cleanly() {
    let mut bytes = png(8, 8, &halves(8, 8, 255), png::ColorType::Rgba);
    bytes.truncate(bytes.len() / 2);
    assert!(matches!(decode(&bytes), Err(Error::Decode(_))));
    let mut j = jpeg(8, 8, &[0; 192], None);
    j.truncate(20);
    assert!(decode(&j).is_err());
}
