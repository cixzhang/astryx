//! After a shell prewarms the Terrace pixel theme's text styles, its
//! screens draw their text without rasterizing or uploading glyphs.

mod common;

use astryx_core::{
    FontFamily, FrameInfo, Input, InputPhase, Runtime, Surface, TextStyle, Theme, Tokens,
};
use astryx_gallery::Gallery;
use astryx_theme_pixel::PixelTheme;

/// The theme's text styles as its elements draw them: the theme's font,
/// sizes in whole design pixels on the grid. Mirrors the core's themed text.
fn styles(theme: &'static Theme, t: &Tokens) -> Vec<(TextStyle, Option<f32>)> {
    let f = &t.font;
    let named = [
        (f.body, FontFamily::Body),
        (f.label, FontFamily::Body),
        (f.supporting, FontFamily::Body),
        (f.large, FontFamily::Body),
        (f.code, FontFamily::Code),
        (f.heading1, FontFamily::Heading),
        (f.heading2, FontFamily::Heading),
        (f.heading3, FontFamily::Heading),
        (f.heading4, FontFamily::Heading),
        (f.heading5, FontFamily::Heading),
        (f.heading6, FontFamily::Heading),
    ];
    let mut out = Vec::new();
    for (s, family) in named {
        for weight in [s.weight, 400, 500, 700] {
            let stack = match family {
                FontFamily::Body => f.family.body,
                FontFamily::Heading => f.family.heading,
                _ => f.family.code,
            };
            let mut style = TextStyle {
                family,
                size: s.size,
                weight,
                leading: s.leading,
                direction: None,
            };
            if let Some(font) = stack
                .iter()
                .find_map(|n| theme.fonts.iter().find(|tf| tf.family == *n))
            {
                style.family = FontFamily::Theme(font);
                if let (Some(cell), true) = (theme.grid, font.em_pixels > 0) {
                    let step = cell * f32::from(font.em_pixels);
                    style.size = (s.size / step).round().max(1.0) * step;
                }
            }
            out.push((style, theme.grid));
        }
    }
    out
}

#[test]
fn terrace_text_draws_from_the_prewarm() {
    let mut gl = match common::OffscreenGl::new() {
        Ok(gl) => gl,
        Err(e) => {
            eprintln!("skipping: {e}");
            return;
        }
    };
    let theme = PixelTheme::Terrace.theme();
    let frame = FrameInfo {
        width: 1024,
        height: 768,
        scale: 2.0,
        dt: 1.0 / 60.0,
    };
    let styles = styles(theme, &theme.light);
    let chars = concat!(
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
        ".,:;!?'\"()[]{}<>/\\|-_+=*&^%$#@~`\u{2014}\u{2013}\u{2026}\u{00B7}\u{2022}\u{2019}\u{201C}\u{201D}"
    );
    let mut warm = gl.renderer.text().prewarm(&styles, 2.0, chars, 64.0);
    while !gl.renderer.text().install_prewarmed(&mut warm) {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    gl.renderer.preload_glyphs(&styles, 2.0);
    gl.renderer.finish_preload();
    let mut rt = Runtime::new(Gallery {
        theme,
        ..Gallery::new()
    });
    let (mut glyphs, mut uploads) = (0, 0);
    for i in 0..40 {
        if i % 8 == 4 {
            rt.input(Input::Down, InputPhase::Pressed);
            rt.input(Input::Down, InputPhase::Released);
        }
        for (id, data) in rt.take_image_updates() {
            gl.renderer.set_image(id, data.as_ref());
        }
        let list = rt.frame(&frame, gl.renderer.text());
        gl.render(&list, &frame);
        let s = gl.renderer.last_stats();
        glyphs += s.glyphs;
        uploads += s.upload_kinds[0];
    }
    eprintln!("terrace: {glyphs} glyphs rasterized, {uploads} glyph uploads");
    assert_eq!((glyphs, uploads), (0, 0));
}
