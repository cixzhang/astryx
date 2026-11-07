//! Fixed display lists that cover every [`DrawCmd`], and a [`Surface`] that
//! serves them. Renderer tests, golden images, and the shells' smoke tests
//! use these instead of a real app.

use astryx_core::{
    A11yAction, A11yNode, Color, Command, DisplayList, DrawCmd, FontFamily, FrameInfo, ImageData,
    ImageFit, ImageId, Input, InputPhase, PathCmd, Point, Radii, Rect, Role, ShadowLayer, Surface,
    TextAlign, TextDirection, TextMeasure, TextStyle,
};

/// Logical size every fixture is designed for.
pub const FIXTURE_SIZE: (u32, u32) = (480, 320);

/// The fixture names, in a stable order.
pub const FIXTURES: [&str; 8] = [
    "shapes",
    "text",
    "shadows",
    "clips",
    "transforms",
    "bidi",
    "gallery",
    "images",
];

/// The images the `images` fixture draws: a landscape and a checkerboard
/// with transparent squares, made in code.
pub const LANDSCAPE: ImageId = ImageId(1);
pub const CHECKER: ImageId = ImageId(2);
/// An id nothing registers: drawing it draws nothing.
const MISSING: ImageId = ImageId(3);

/// A 96x64 landscape: a sky gradient, a sun with a soft edge, and hills.
pub fn landscape() -> ImageData {
    let (w, h) = (96u32, 64u32);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let t = y as f32 / h as f32;
            let mut c = [40.0 + 180.0 * t, 110.0 + 90.0 * t, 230.0 - 60.0 * t];
            let d = ((x as f32 - 66.0).powi(2) + (y as f32 - 22.0).powi(2)).sqrt();
            let sun = (12.0 - d).clamp(0.0, 1.0);
            c = [
                c[0] + (255.0 - c[0]) * sun,
                c[1] + (210.0 - c[1]) * sun,
                c[2] + (60.0 - c[2]) * sun,
            ];
            let hill = 46.0 + 8.0 * ((x as f32) / 12.0).sin();
            if y as f32 > hill {
                c = [40.0, 140.0 + (x % 7) as f32, 80.0];
            }
            rgba.extend_from_slice(&[c[0] as u8, c[1] as u8, c[2] as u8, 255]);
        }
    }
    ImageData {
        width: w,
        height: h,
        rgba: rgba.into(),
    }
}

/// A 16x16 checkerboard of 4-pixel squares: orange and transparent.
pub fn checker() -> ImageData {
    let mut rgba = Vec::with_capacity(16 * 16 * 4);
    for y in 0..16u32 {
        for x in 0..16u32 {
            let on = (x / 4 + y / 4) % 2 == 0;
            rgba.extend_from_slice(if on {
                &[242, 140, 40, 255]
            } else {
                &[0, 0, 0, 0]
            });
        }
    }
    ImageData {
        width: 16,
        height: 16,
        rgba: rgba.into(),
    }
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::rgba(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
}

const BG: Color = rgb(246, 247, 249);
const SURFACE: Color = rgb(255, 255, 255);
const INK: Color = rgb(28, 30, 33);
const MUTED: Color = rgb(101, 103, 107);
const ACCENT: Color = rgb(10, 124, 255);
const BORDER: Color = rgb(206, 208, 212);
const GREEN: Color = rgb(36, 164, 92);
const ORANGE: Color = rgb(242, 140, 40);

fn style(size: f32, weight: u16) -> TextStyle {
    TextStyle {
        direction: None,
        family: FontFamily::Body,
        size,
        weight,
        leading: 1.4,
    }
}

fn rect(x: f32, y: f32, w: f32, h: f32, r: f32, fill: Color) -> DrawCmd {
    DrawCmd::Rect {
        rect: Rect::new(x, y, w, h),
        radii: Radii::all(r),
        fill,
        stroke: None,
    }
}

fn text(r: Rect, s: &str, style: TextStyle, color: Color, align: TextAlign) -> DrawCmd {
    DrawCmd::Text {
        rect: r,
        text: s.into(),
        style,
        color,
        align,
    }
}

fn p(x: f32, y: f32) -> Point {
    Point { x, y }
}

fn shadow(x: f32, y: f32, blur: f32, spread: f32, alpha: f32) -> ShadowLayer {
    ShadowLayer {
        x,
        y,
        blur,
        spread,
        inset: false,
        color: Color::rgba(0.0, 0.0, 0.0, alpha),
    }
}

fn shapes() -> Vec<DrawCmd> {
    let mut c = vec![
        rect(16.0, 16.0, 96.0, 64.0, 0.0, ACCENT),
        rect(128.0, 16.0, 96.0, 64.0, 12.0, GREEN),
        DrawCmd::Rect {
            rect: Rect::new(240.0, 16.0, 96.0, 64.0),
            radii: Radii([0.0, 24.0, 8.0, 32.0]),
            fill: ORANGE,
            stroke: Some((INK, 2.0)),
        },
        DrawCmd::Rect {
            rect: Rect::new(352.0, 16.0, 112.0, 64.0),
            radii: Radii::all(999.0),
            fill: Color::TRANSPARENT,
            stroke: Some((ACCENT, 1.0)),
        },
        rect(
            16.0,
            96.0,
            96.0,
            64.0,
            8.0,
            Color::rgba(0.04, 0.49, 1.0, 0.5),
        ),
        rect(
            64.0,
            120.0,
            96.0,
            64.0,
            8.0,
            Color::rgba(0.95, 0.55, 0.16, 0.5),
        ),
        DrawCmd::Rect {
            rect: Rect::new(184.0, 96.0, 64.0, 64.0),
            radii: Radii::all(32.0),
            fill: SURFACE,
            stroke: Some((BORDER, 1.5)),
        },
        DrawCmd::Rect {
            rect: Rect::new(264.0, 96.0, 20.5, 20.5),
            radii: Radii::all(3.0),
            fill: INK,
            stroke: None,
        },
        DrawCmd::Rect {
            rect: Rect::new(290.3, 96.3, 7.4, 30.2),
            radii: Radii::all(0.0),
            fill: MUTED,
            stroke: None,
        },
    ];
    // Hairlines, a thick line, and diagonals.
    for (i, w) in [0.5, 1.0, 2.0, 3.0].iter().enumerate() {
        let y = 200.0 + i as f32 * 12.0;
        c.push(DrawCmd::Line {
            from: p(16.0, y),
            to: p(200.0, y),
            color: INK,
            width: *w,
        });
    }
    c.push(DrawCmd::Line {
        from: p(216.0, 196.0),
        to: p(216.0, 244.0),
        color: ACCENT,
        width: 1.0,
    });
    c.push(DrawCmd::Line {
        from: p(232.0, 196.0),
        to: p(296.0, 244.0),
        color: GREEN,
        width: 2.0,
    });
    c.push(DrawCmd::Line {
        from: p(232.0, 244.0),
        to: p(296.0, 196.0),
        color: ORANGE,
        width: 1.0,
    });
    // A stroked check mark, a filled star, and a curved blob.
    c.push(DrawCmd::Path {
        commands: vec![
            PathCmd::MoveTo(p(320.0, 120.0)),
            PathCmd::LineTo(p(334.0, 134.0)),
            PathCmd::LineTo(p(360.0, 104.0)),
        ],
        fill: None,
        stroke: Some((GREEN, 4.0)),
    });
    let star: Vec<PathCmd> = (0..10)
        .map(|i| {
            let a = std::f32::consts::PI * (i as f32 / 5.0) - std::f32::consts::FRAC_PI_2;
            let r = if i % 2 == 0 { 30.0 } else { 12.0 };
            let pt = p(420.0 + r * a.cos(), 130.0 + r * a.sin());
            if i == 0 {
                PathCmd::MoveTo(pt)
            } else {
                PathCmd::LineTo(pt)
            }
        })
        .chain([PathCmd::Close])
        .collect();
    c.push(DrawCmd::Path {
        commands: star,
        fill: Some(ORANGE),
        stroke: Some((INK, 1.0)),
    });
    c.push(DrawCmd::Path {
        commands: vec![
            PathCmd::MoveTo(p(330.0, 200.0)),
            PathCmd::CubicTo(p(370.0, 170.0), p(430.0, 230.0), p(460.0, 200.0)),
            PathCmd::QuadTo(p(470.0, 260.0), p(400.0, 290.0)),
            PathCmd::CubicTo(p(360.0, 300.0), p(310.0, 260.0), p(330.0, 200.0)),
            PathCmd::Close,
        ],
        fill: Some(Color::rgba(0.04, 0.49, 1.0, 0.25)),
        stroke: Some((ACCENT, 2.0)),
    });
    c
}

fn text_fixture() -> Vec<DrawCmd> {
    let para = "Astryx draws every pixel itself, so one app looks the same on desktop, on the web, and in headless tests.";
    let mut c = vec![
        text(
            Rect::new(16.0, 12.0, 448.0, 40.0),
            "Heading Bold 28",
            style(28.0, 700),
            INK,
            TextAlign::Start,
        ),
        text(
            Rect::new(16.0, 56.0, 448.0, 24.0),
            "Label Medium 16 — focus, hover, pressed",
            style(16.0, 500),
            INK,
            TextAlign::Start,
        ),
        text(
            Rect::new(16.0, 84.0, 448.0, 20.0),
            "Body Regular 14: The quick brown fox jumps over the lazy dog.",
            style(14.0, 400),
            MUTED,
            TextAlign::Start,
        ),
        text(
            Rect::new(16.0, 106.0, 448.0, 16.0),
            "Supporting 11 · 0123456789 · AVATAR Wave fi fl",
            style(11.0, 400),
            MUTED,
            TextAlign::Start,
        ),
    ];
    c.push(rect(16.0, 132.0, 200.0, 96.0, 4.0, SURFACE));
    c.push(text(
        Rect::new(16.0, 132.0, 200.0, 96.0),
        para,
        style(13.0, 400),
        INK,
        TextAlign::Start,
    ));
    for (i, align) in [TextAlign::Start, TextAlign::Center, TextAlign::End]
        .into_iter()
        .enumerate()
    {
        let y = 132.0 + i as f32 * 30.0;
        c.push(rect(232.0, y, 232.0, 24.0, 4.0, SURFACE));
        c.push(text(
            Rect::new(232.0, y, 232.0, 24.0),
            "Aligned text",
            style(14.0, 500),
            ACCENT,
            align,
        ));
    }
    c.push(text(
        Rect::new(16.0, 240.0, 448.0, 60.0),
        "Hard line break\nsecond line, same size",
        style(15.0, 400),
        INK,
        TextAlign::Center,
    ));
    c.push(rect(16.0, 292.0, 448.0, 24.0, 12.0, INK));
    c.push(text(
        Rect::new(16.0, 292.0, 448.0, 24.0),
        "Light text on dark",
        style(13.0, 600),
        SURFACE,
        TextAlign::Center,
    ));
    c
}

fn shadows() -> Vec<DrawCmd> {
    let mut c = Vec::new();
    let card =
        |c: &mut Vec<DrawCmd>, x: f32, y: f32, r: f32, layers: Vec<ShadowLayer>, label: &str| {
            let bounds = Rect::new(x, y, 128.0, 80.0);
            c.push(DrawCmd::Shadow {
                rect: bounds,
                radii: Radii::all(r),
                layers,
            });
            c.push(DrawCmd::Rect {
                rect: bounds,
                radii: Radii::all(r),
                fill: SURFACE,
                stroke: None,
            });
            c.push(text(
                Rect::new(x, y + 28.0, 128.0, 24.0),
                label,
                style(13.0, 500),
                INK,
                TextAlign::Center,
            ));
        };
    card(
        &mut c,
        24.0,
        24.0,
        8.0,
        vec![shadow(0.0, 1.0, 2.0, 0.0, 0.2)],
        "low",
    );
    card(
        &mut c,
        176.0,
        24.0,
        8.0,
        vec![
            shadow(0.0, 2.0, 4.0, 0.0, 0.1),
            shadow(0.0, 8.0, 16.0, 0.0, 0.15),
        ],
        "med",
    );
    card(
        &mut c,
        328.0,
        24.0,
        12.0,
        vec![
            shadow(0.0, 4.0, 8.0, 0.0, 0.1),
            shadow(0.0, 16.0, 40.0, -4.0, 0.25),
        ],
        "high",
    );
    card(
        &mut c,
        24.0,
        160.0,
        0.0,
        vec![shadow(6.0, 6.0, 0.0, 0.0, 0.8)],
        "hard offset",
    );
    card(
        &mut c,
        176.0,
        160.0,
        40.0,
        vec![ShadowLayer {
            x: 0.0,
            y: 0.0,
            blur: 12.0,
            spread: 4.0,
            inset: false,
            color: Color::rgba(0.04, 0.49, 1.0, 0.6),
        }],
        "glow + spread",
    );
    // An inset shadow on a translucent box shows the box itself is not filled by it.
    let well = Rect::new(328.0, 160.0, 128.0, 80.0);
    c.push(DrawCmd::Rect {
        rect: well,
        radii: Radii::all(8.0),
        fill: rgb(233, 235, 238),
        stroke: None,
    });
    c.push(DrawCmd::Shadow {
        rect: well,
        radii: Radii::all(8.0),
        layers: vec![ShadowLayer {
            x: 0.0,
            y: 3.0,
            blur: 6.0,
            spread: 1.0,
            inset: true,
            color: Color::rgba(0.0, 0.0, 0.0, 0.35),
        }],
    });
    c.push(text(
        Rect::new(328.0, 188.0, 128.0, 24.0),
        "inset",
        style(13.0, 500),
        INK,
        TextAlign::Center,
    ));
    // A translucent card over a shadow: CSS never paints the shadow under the box.
    let glass = Rect::new(24.0, 264.0, 432.0, 40.0);
    c.push(DrawCmd::Shadow {
        rect: glass,
        radii: Radii::all(20.0),
        layers: vec![shadow(0.0, 4.0, 12.0, 0.0, 0.5)],
    });
    c.push(DrawCmd::Rect {
        rect: glass,
        radii: Radii::all(20.0),
        fill: Color::rgba(1.0, 1.0, 1.0, 0.3),
        stroke: Some((BORDER, 1.0)),
    });
    c
}

fn bidi() -> Vec<DrawCmd> {
    let row = |c: &mut Vec<DrawCmd>, y: f32, h: f32, s: &str, st: TextStyle, align: TextAlign| {
        c.push(rect(16.0, y, 448.0, h, 6.0, SURFACE));
        c.push(text(
            Rect::new(24.0, y + 4.0, 432.0, h - 8.0),
            s,
            st,
            INK,
            align,
        ));
    };
    let mut c = Vec::new();
    // Latin paragraph with Hebrew and Arabic words inside.
    row(
        &mut c,
        12.0,
        30.0,
        "Save as שמור or حفظ, then press OK.",
        style(15.0, 400),
        TextAlign::Start,
    );
    // Hebrew paragraph with a Latin name and digits, aligned to its start (the right).
    row(
        &mut c,
        48.0,
        30.0,
        "ברוכים הבאים ל-Astryx 2.0, גרסה 98!",
        style(15.0, 400),
        TextAlign::End,
    );
    // Arabic with joined letters, a ligature, and marks.
    row(
        &mut c,
        84.0,
        34.0,
        "مرحبًا بكم في Astryx — لا شيء أسهل",
        style(16.0, 400),
        TextAlign::End,
    );
    // Bold Hebrew heading and Medium Arabic label.
    row(
        &mut c,
        124.0,
        36.0,
        "הגדרות מערכת",
        style(22.0, 700),
        TextAlign::End,
    );
    row(
        &mut c,
        166.0,
        30.0,
        "الإعدادات العامة",
        style(16.0, 500),
        TextAlign::End,
    );
    // A Latin-first string in a right-to-left region: with the direction set,
    // "Astryx" starts the line at the right instead of the left.
    row(
        &mut c,
        202.0,
        30.0,
        "Astryx הגדרות 2.0",
        TextStyle {
            direction: Some(TextDirection::Rtl),
            ..style(15.0, 500)
        },
        TextAlign::End,
    );
    // A right-to-left paragraph that wraps: the first words stay on the top line.
    c.push(rect(16.0, 238.0, 220.0, 76.0, 6.0, SURFACE));
    c.push(text(
        Rect::new(24.0, 242.0, 204.0, 68.0),
        "هذا نص عربي طويل يلتف على عدة أسطر مع كلمة English في الوسط.",
        style(14.0, 400),
        INK,
        TextAlign::End,
    ));
    c.push(rect(244.0, 238.0, 220.0, 76.0, 6.0, SURFACE));
    c.push(text(
        Rect::new(252.0, 242.0, 204.0, 68.0),
        "זהו טקסט עברי ארוך שנשבר לכמה שורות, עם המילה Astryx באמצע.",
        style(14.0, 400),
        MUTED,
        TextAlign::End,
    ));
    c
}

fn transforms() -> Vec<DrawCmd> {
    let push = |origin: Point, scale: f32, translate: Point| DrawCmd::PushTransform {
        origin,
        scale,
        translate,
    };
    let button = |c: &mut Vec<DrawCmd>, x: f32, y: f32, label: &str| {
        let r = Rect::new(x, y, 120.0, 40.0);
        c.push(DrawCmd::Shadow {
            rect: r,
            radii: Radii::all(10.0),
            layers: vec![shadow(0.0, 2.0, 6.0, 0.0, 0.3)],
        });
        c.push(DrawCmd::Rect {
            rect: r,
            radii: Radii::all(10.0),
            fill: ACCENT,
            stroke: None,
        });
        c.push(DrawCmd::Path {
            commands: vec![
                PathCmd::MoveTo(p(x + 14.0, y + 20.0)),
                PathCmd::LineTo(p(x + 19.0, y + 25.0)),
                PathCmd::LineTo(p(x + 28.0, y + 15.0)),
            ],
            fill: None,
            stroke: Some((SURFACE, 2.0)),
        });
        c.push(text(
            Rect::new(x + 30.0, y + 10.0, 84.0, 20.0),
            label,
            style(14.0, 600),
            SURFACE,
            TextAlign::Center,
        ));
    };
    let mut c = Vec::new();
    // At rest, and pressed: scaled to 96% about its center.
    button(&mut c, 24.0, 24.0, "At rest");
    c.push(push(p(224.0, 44.0), 0.96, p(0.0, 0.0)));
    button(&mut c, 164.0, 24.0, "Pressed");
    c.push(DrawCmd::PopTransform);
    // A switch thumb moved by a fractional amount still lands on whole pixels.
    for (i, travel) in [0.0f32, 10.5, 20.0].iter().enumerate() {
        let y = 92.0 + i as f32 * 34.0;
        c.push(rect(
            24.0,
            y,
            48.0,
            26.0,
            13.0,
            if *travel > 15.0 { GREEN } else { BORDER },
        ));
        c.push(push(p(0.0, 0.0), 1.0, p(*travel, 0.0)));
        c.push(DrawCmd::Shadow {
            rect: Rect::new(27.0, y + 3.0, 20.0, 20.0),
            radii: Radii::all(10.0),
            layers: vec![shadow(0.0, 1.0, 3.0, 0.0, 0.35)],
        });
        c.push(rect(27.0, y + 3.0, 20.0, 20.0, 10.0, SURFACE));
        c.push(DrawCmd::PopTransform);
    }
    // A card grown 125% about its corner, with a rounded clip and opacity inside.
    c.push(push(p(300.0, 90.0), 1.25, p(0.0, 0.0)));
    c.push(DrawCmd::Shadow {
        rect: Rect::new(300.0, 90.0, 120.0, 90.0),
        radii: Radii::all(12.0),
        layers: vec![shadow(0.0, 6.0, 14.0, 0.0, 0.25)],
    });
    c.push(DrawCmd::Rect {
        rect: Rect::new(300.0, 90.0, 120.0, 90.0),
        radii: Radii::all(12.0),
        fill: SURFACE,
        stroke: Some((BORDER, 1.0)),
    });
    c.push(DrawCmd::PushClip {
        rect: Rect::new(308.0, 98.0, 104.0, 30.0),
        radii: Radii::all(8.0),
    });
    for i in 0..6 {
        c.push(rect(300.0 + i as f32 * 20.0, 90.0, 10.0, 60.0, 0.0, ORANGE));
    }
    c.push(DrawCmd::PopClip);
    c.push(DrawCmd::PushOpacity(0.6));
    c.push(text(
        Rect::new(308.0, 134.0, 104.0, 40.0),
        "Scaled text wraps the same",
        style(12.0, 400),
        INK,
        TextAlign::Start,
    ));
    c.push(DrawCmd::PopOpacity);
    c.push(DrawCmd::Line {
        from: p(308.0, 131.0),
        to: p(412.0, 131.0),
        color: BORDER,
        width: 1.0,
    });
    c.push(DrawCmd::PopTransform);
    // Nested: a move inside a shrink; and text moved by half a point.
    c.push(push(p(100.0, 260.0), 0.8, p(0.0, 0.0)));
    c.push(push(p(0.0, 0.0), 1.0, p(30.0, 0.0)));
    c.push(DrawCmd::Rect {
        rect: Rect::new(24.0, 230.0, 160.0, 60.0),
        radii: Radii([4.0, 20.0, 4.0, 20.0]),
        fill: ORANGE,
        stroke: Some((INK, 2.0)),
    });
    c.push(text(
        Rect::new(24.0, 248.0, 160.0, 24.0),
        "nested",
        style(16.0, 700),
        INK,
        TextAlign::Center,
    ));
    c.push(DrawCmd::PopTransform);
    c.push(DrawCmd::PopTransform);
    c.push(push(p(0.0, 0.0), 1.0, p(0.5, 0.5)));
    c.push(text(
        Rect::new(240.0, 270.0, 220.0, 24.0),
        "Moved half a point",
        style(14.0, 400),
        MUTED,
        TextAlign::Start,
    ));
    c.push(DrawCmd::PopTransform);
    c
}

fn clips() -> Vec<DrawCmd> {
    let stripes = |c: &mut Vec<DrawCmd>, x: f32, y: f32| {
        for i in 0..8 {
            let col = if i % 2 == 0 { ACCENT } else { ORANGE };
            c.push(rect(x + i as f32 * 20.0, y, 20.0, 160.0, 0.0, col));
        }
    };
    let mut c = vec![DrawCmd::PushClip {
        rect: Rect::new(16.0, 16.0, 140.0, 120.0),
        radii: Radii::all(24.0),
    }];
    stripes(&mut c, 0.0, 0.0);
    c.push(text(
        Rect::new(16.0, 60.0, 140.0, 24.0),
        "rounded clip",
        style(14.0, 700),
        SURFACE,
        TextAlign::Center,
    ));
    c.push(DrawCmd::PopClip);
    // Nested clips: the inner rounded clip pokes past the outer square one.
    c.push(DrawCmd::PushClip {
        rect: Rect::new(176.0, 16.0, 120.0, 120.0),
        radii: Radii::all(0.0),
    });
    c.push(DrawCmd::PushClip {
        rect: Rect::new(196.0, 36.0, 140.0, 140.0),
        radii: Radii::all(70.0),
    });
    stripes(&mut c, 160.0, 0.0);
    c.push(DrawCmd::PopClip);
    c.push(DrawCmd::PopClip);
    // Group opacity: overlaps inside the group do not show through.
    c.push(DrawCmd::PushOpacity(0.5));
    c.push(rect(320.0, 16.0, 80.0, 80.0, 16.0, ACCENT));
    c.push(rect(360.0, 56.0, 80.0, 80.0, 16.0, ORANGE));
    c.push(DrawCmd::PopOpacity);
    // Opacity inside a clip, and a clip inside nested opacity.
    c.push(DrawCmd::PushClip {
        rect: Rect::new(16.0, 168.0, 200.0, 120.0),
        radii: Radii([32.0, 0.0, 32.0, 0.0]),
    });
    c.push(rect(0.0, 150.0, 240.0, 160.0, 0.0, INK));
    c.push(DrawCmd::PushOpacity(0.6));
    c.push(text(
        Rect::new(16.0, 200.0, 200.0, 60.0),
        "Opacity inside a clip spills nowhere",
        style(18.0, 700),
        SURFACE,
        TextAlign::Center,
    ));
    c.push(DrawCmd::PopOpacity);
    c.push(DrawCmd::PopClip);
    c.push(DrawCmd::PushOpacity(0.8));
    c.push(DrawCmd::PushOpacity(0.5));
    c.push(DrawCmd::PushClip {
        rect: Rect::new(240.0, 168.0, 224.0, 120.0),
        radii: Radii::all(12.0),
    });
    stripes(&mut c, 232.0, 150.0);
    c.push(rect(250.0, 260.0, 300.0, 40.0, 0.0, GREEN));
    c.push(DrawCmd::PopClip);
    c.push(DrawCmd::PopOpacity);
    c.push(DrawCmd::PopOpacity);
    c
}

/// Focusable controls of the gallery fixture, in reading order.
const GALLERY_CONTROLS: usize = 4;

fn gallery(text_measure: &dyn TextMeasure, focus: usize, toggled: bool) -> Vec<DrawCmd> {
    let mut c = Vec::new();
    let card = Rect::new(24.0, 24.0, 432.0, 272.0);
    c.push(DrawCmd::Shadow {
        rect: card,
        radii: Radii::all(12.0),
        layers: vec![
            shadow(0.0, 2.0, 4.0, 0.0, 0.08),
            shadow(0.0, 8.0, 24.0, 0.0, 0.12),
        ],
    });
    c.push(DrawCmd::Rect {
        rect: card,
        radii: Radii::all(12.0),
        fill: SURFACE,
        stroke: Some((BORDER, 1.0)),
    });
    c.push(text(
        Rect::new(48.0, 44.0, 384.0, 34.0),
        "Settings",
        style(24.0, 700),
        INK,
        TextAlign::Start,
    ));
    c.push(text(Rect::new(48.0, 80.0, 384.0, 40.0), "Every renderer measures this paragraph the same way, so it wraps at the same word everywhere.", style(13.0, 400), MUTED, TextAlign::Start));
    c.push(DrawCmd::Line {
        from: p(48.0, 128.0),
        to: p(432.0, 128.0),
        color: BORDER,
        width: 1.0,
    });

    let focus_ring = |c: &mut Vec<DrawCmd>, r: Rect, radius: f32| {
        c.push(DrawCmd::Rect {
            rect: Rect::new(r.x - 3.0, r.y - 3.0, r.width + 6.0, r.height + 6.0),
            radii: Radii::all(radius + 3.0),
            fill: Color::TRANSPARENT,
            stroke: Some((ACCENT, 2.0)),
        });
    };
    // Buttons sized from measured labels.
    let mut x = 48.0;
    for (i, (label, primary)) in [("Save", true), ("Cancel", false)].into_iter().enumerate() {
        let st = style(14.0, 500);
        let w = text_measure.measure(label, &st, None).width.ceil() + 32.0;
        let r = Rect::new(x, 144.0, w, 36.0);
        if primary {
            c.push(DrawCmd::Shadow {
                rect: r,
                radii: Radii::all(8.0),
                layers: vec![shadow(0.0, 1.0, 2.0, 0.0, 0.2)],
            });
        }
        let (fill, stroke, ink) = if primary {
            (ACCENT, None, SURFACE)
        } else {
            (SURFACE, Some((BORDER, 1.0)), INK)
        };
        c.push(DrawCmd::Rect {
            rect: r,
            radii: Radii::all(8.0),
            fill,
            stroke,
        });
        c.push(text(
            Rect::new(r.x, r.y + 8.0, r.width, 20.0),
            label,
            st,
            ink,
            TextAlign::Center,
        ));
        if focus == i {
            focus_ring(&mut c, r, 8.0);
        }
        x += w + 12.0;
    }
    // Switch.
    let track = Rect::new(48.0, 204.0, 44.0, 24.0);
    c.push(DrawCmd::Rect {
        rect: track,
        radii: Radii::all(12.0),
        fill: if toggled { GREEN } else { BORDER },
        stroke: None,
    });
    let knob_x = if toggled { 70.0 } else { 50.0 };
    c.push(DrawCmd::Shadow {
        rect: Rect::new(knob_x, 206.0, 20.0, 20.0),
        radii: Radii::all(10.0),
        layers: vec![shadow(0.0, 1.0, 3.0, 0.0, 0.3)],
    });
    c.push(rect(knob_x, 206.0, 20.0, 20.0, 10.0, SURFACE));
    c.push(text(
        Rect::new(104.0, 206.0, 200.0, 20.0),
        "Notifications",
        style(14.0, 400),
        INK,
        TextAlign::Start,
    ));
    if focus == 2 {
        focus_ring(&mut c, track, 12.0);
    }
    // Checkbox with an icon path.
    let bx = Rect::new(260.0, 206.0, 20.0, 20.0);
    c.push(DrawCmd::Rect {
        rect: bx,
        radii: Radii::all(4.0),
        fill: ACCENT,
        stroke: None,
    });
    c.push(DrawCmd::Path {
        commands: vec![
            PathCmd::MoveTo(p(265.0, 216.0)),
            PathCmd::LineTo(p(268.5, 219.5)),
            PathCmd::LineTo(p(275.0, 212.0)),
        ],
        fill: None,
        stroke: Some((SURFACE, 2.0)),
    });
    c.push(text(
        Rect::new(288.0, 206.0, 140.0, 20.0),
        "Remember me",
        style(14.0, 400),
        INK,
        TextAlign::Start,
    ));
    if focus == 3 {
        focus_ring(&mut c, bx, 4.0);
    }
    // Progress bar, clipped to its rounded track.
    let bar = Rect::new(48.0, 252.0, 384.0, 8.0);
    c.push(DrawCmd::Rect {
        rect: bar,
        radii: Radii::all(4.0),
        fill: rgb(233, 235, 238),
        stroke: None,
    });
    c.push(DrawCmd::PushClip {
        rect: bar,
        radii: Radii::all(4.0),
    });
    c.push(rect(48.0, 252.0, 250.0, 8.0, 0.0, ACCENT));
    c.push(DrawCmd::PopClip);
    c.push(DrawCmd::PushOpacity(0.6));
    c.push(text(
        Rect::new(48.0, 264.0, 384.0, 20.0),
        "65% uploaded",
        style(12.0, 400),
        MUTED,
        TextAlign::End,
    ));
    c.push(DrawCmd::PopOpacity);
    c
}

/// Images in each fit, with radii, alpha, a clip, and an opacity group.
fn images_fixture() -> Vec<DrawCmd> {
    let image = |x, y, w, h, r, image, fit| DrawCmd::Image {
        rect: Rect::new(x, y, w, h),
        radii: Radii::all(r),
        image,
        fit,
    };
    let frame = |x, y, w, h| DrawCmd::Rect {
        rect: Rect::new(x, y, w, h),
        radii: Radii::default(),
        fill: Color::TRANSPARENT,
        stroke: Some((BORDER, 1.0)),
    };
    vec![
        rect(16.0, 16.0, 448.0, 288.0, 12.0, SURFACE),
        // An avatar: cover in a circle.
        image(32.0, 32.0, 96.0, 96.0, 48.0, LANDSCAPE, ImageFit::Cover),
        // Contain letterboxes inside a tall box.
        frame(144.0, 32.0, 80.0, 120.0),
        image(144.0, 32.0, 80.0, 120.0, 0.0, LANDSCAPE, ImageFit::Contain),
        // Fill stretches, with rounded corners.
        image(240.0, 32.0, 96.0, 96.0, 12.0, LANDSCAPE, ImageFit::Fill),
        // Under a scaling transform, inside a clip the transform also moves.
        DrawCmd::PushTransform {
            origin: Point { x: 400.0, y: 80.0 },
            scale: 0.75,
            translate: Point { x: 0.0, y: 0.0 },
        },
        DrawCmd::PushClip {
            rect: Rect::new(352.0, 32.0, 96.0, 96.0),
            radii: Radii::all(24.0),
        },
        image(352.0, 32.0, 96.0, 96.0, 0.0, LANDSCAPE, ImageFit::Cover),
        DrawCmd::PopClip,
        DrawCmd::PopTransform,
        // Transparent pixels show what is under them.
        rect(32.0, 168.0, 120.0, 120.0, 0.0, ACCENT),
        image(32.0, 168.0, 120.0, 120.0, 16.0, CHECKER, ImageFit::Cover),
        // Inside a clip and an opacity group.
        DrawCmd::PushClip {
            rect: Rect::new(176.0, 168.0, 120.0, 120.0),
            radii: Radii::all(60.0),
        },
        DrawCmd::PushOpacity(0.5),
        image(176.0, 168.0, 160.0, 120.0, 0.0, LANDSCAPE, ImageFit::Cover),
        DrawCmd::PopOpacity,
        DrawCmd::PopClip,
        // Not registered: nothing is drawn inside the frame.
        frame(320.0, 168.0, 128.0, 120.0),
        image(320.0, 168.0, 128.0, 120.0, 8.0, MISSING, ImageFit::Cover),
    ]
}

/// Builds one fixture's display list. `focus` and `toggled` only affect `gallery`.
pub fn fixture(
    name: &str,
    text_measure: &dyn TextMeasure,
    focus: usize,
    toggled: bool,
) -> Option<DisplayList> {
    let commands = match name {
        "shapes" => shapes(),
        "text" => text_fixture(),
        "shadows" => shadows(),
        "clips" => clips(),
        "transforms" => transforms(),
        "bidi" => bidi(),
        "gallery" => gallery(text_measure, focus, toggled),
        "images" => images_fixture(),
        _ => return None,
    };
    Some(DisplayList {
        clear: BG,
        commands,
    })
}

/// A [`Surface`] that serves one fixture. Tab and arrows move focus in the
/// gallery, A toggles the switch, B quits.
pub struct FixtureSurface {
    name: &'static str,
    focus: usize,
    toggled: bool,
    last_size: (f32, f32),
    images_sent: bool,
}

impl FixtureSurface {
    pub fn new(name: &str) -> Option<Self> {
        let name = FIXTURES.iter().copied().find(|f| *f == name)?;
        Some(Self {
            name,
            focus: 0,
            toggled: false,
            last_size: (0.0, 0.0),
            images_sent: false,
        })
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
}

impl Surface for FixtureSurface {
    fn input(&mut self, input: Input, phase: InputPhase) -> Command {
        if phase != InputPhase::Pressed {
            return Command::None;
        }
        match input {
            Input::Next | Input::Right | Input::Down => {
                self.focus = (self.focus + 1) % GALLERY_CONTROLS
            }
            Input::Previous | Input::Left | Input::Up => {
                self.focus = (self.focus + GALLERY_CONTROLS - 1) % GALLERY_CONTROLS
            }
            Input::A if self.focus == 2 => self.toggled = !self.toggled,
            Input::B => return Command::Quit,
            _ => {}
        }
        Command::None
    }

    fn take_image_updates(&mut self) -> Vec<(ImageId, Option<ImageData>)> {
        if self.name != "images" || std::mem::replace(&mut self.images_sent, true) {
            return Vec::new();
        }
        vec![(LANDSCAPE, Some(landscape())), (CHECKER, Some(checker()))]
    }

    fn frame(&mut self, frame: &FrameInfo, text: &dyn TextMeasure) -> DisplayList {
        let size = frame.logical_size();
        self.last_size = (size.width, size.height);
        fixture(self.name, text, self.focus, self.toggled).expect("known fixture")
    }

    fn accessibility(&self) -> Option<A11yNode> {
        let node = |id, role, label: &str, bounds, focused, checked| A11yNode {
            rtl: false,
            text_selection: None,
            secure: false,
            description: None,
            selected: None,
            focusable: false,
            live: None,
            id,
            role,
            label: Some(label.to_owned()),
            value: None,
            bounds,
            focused,
            disabled: false,
            checked,
            children: Vec::new(),
        };
        let mut root = node(
            1,
            Role::Group,
            self.name,
            Rect::new(0.0, 0.0, self.last_size.0, self.last_size.1),
            false,
            None,
        );
        if self.name == "gallery" {
            root.children = vec![
                node(
                    2,
                    Role::Heading(1),
                    "Settings",
                    Rect::new(48.0, 44.0, 384.0, 34.0),
                    false,
                    None,
                ),
                node(
                    3,
                    Role::Button,
                    "Save",
                    Rect::new(48.0, 144.0, 80.0, 36.0),
                    self.focus == 0,
                    None,
                ),
                node(
                    4,
                    Role::Button,
                    "Cancel",
                    Rect::new(140.0, 144.0, 90.0, 36.0),
                    self.focus == 1,
                    None,
                ),
                node(
                    5,
                    Role::Switch,
                    "Notifications",
                    Rect::new(48.0, 204.0, 44.0, 24.0),
                    self.focus == 2,
                    Some(self.toggled),
                ),
                node(
                    6,
                    Role::Checkbox,
                    "Remember me",
                    Rect::new(260.0, 206.0, 20.0, 20.0),
                    self.focus == 3,
                    Some(true),
                ),
                A11yNode {
                    rtl: false,
                    text_selection: None,
                    secure: false,
                    description: None,
                    selected: None,
                    focusable: false,
                    live: None,
                    value: Some("65%".into()),
                    ..node(
                        7,
                        Role::ProgressBar,
                        "Upload",
                        Rect::new(48.0, 252.0, 384.0, 8.0),
                        false,
                        None,
                    )
                },
            ];
        }
        Some(root)
    }

    fn wants_frames(&self) -> bool {
        false
    }

    fn a11y_action(&mut self, id: u64, action: A11yAction) -> Command {
        // Accessibility ids 3..=6 are the gallery controls in focus order.
        if self.name != "gallery" || !(3..=6).contains(&id) {
            return Command::None;
        }
        self.focus = (id - 3) as usize;
        if action == A11yAction::Activate && self.focus == 2 {
            self.toggled = !self.toggled;
        }
        Command::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::mem::discriminant;

    struct Fixed;
    impl TextMeasure for Fixed {
        fn measure(&self, text: &str, _: &TextStyle, _: Option<f32>) -> astryx_core::Size {
            astryx_core::Size {
                width: text.len() as f32 * 7.0,
                height: 20.0,
            }
        }
    }

    #[test]
    fn fixtures_cover_every_draw_command() {
        let mut seen = HashSet::new();
        for name in FIXTURES {
            for cmd in fixture(name, &Fixed, 0, false).unwrap().commands {
                seen.insert(discriminant(&cmd));
            }
        }
        // Rect, Shadow, Text, Line, Path, Image, PushClip, PopClip,
        // PushOpacity, PopOpacity, PushTransform, PopTransform.
        assert_eq!(seen.len(), 12);
    }

    #[test]
    fn groups_are_balanced() {
        for name in FIXTURES {
            let mut depth = 0i32;
            for cmd in fixture(name, &Fixed, 0, false).unwrap().commands {
                match cmd {
                    DrawCmd::PushClip { .. }
                    | DrawCmd::PushOpacity(_)
                    | DrawCmd::PushTransform { .. } => depth += 1,
                    DrawCmd::PopClip | DrawCmd::PopOpacity | DrawCmd::PopTransform => depth -= 1,
                    _ => {}
                }
                assert!(depth >= 0, "{name}");
            }
            assert_eq!(depth, 0, "{name}");
        }
    }

    #[test]
    fn accessibility_actions_focus_and_activate() {
        let mut s = FixtureSurface::new("gallery").unwrap();
        s.a11y_action(5, A11yAction::Activate);
        let tree = s.accessibility().unwrap();
        let switch = tree
            .children
            .iter()
            .find(|n| n.role == Role::Switch)
            .unwrap();
        assert!(switch.focused);
        assert_eq!(switch.checked, Some(true));
        s.a11y_action(4, A11yAction::Focus);
        let tree = s.accessibility().unwrap();
        assert!(tree.children.iter().find(|n| n.id == 4).unwrap().focused);
    }

    #[test]
    fn inputs_move_focus_and_quit() {
        let mut s = FixtureSurface::new("gallery").unwrap();
        s.input(Input::Next, InputPhase::Pressed);
        s.input(Input::Next, InputPhase::Released);
        s.input(Input::Next, InputPhase::Pressed);
        s.input(Input::A, InputPhase::Pressed);
        let tree = s.accessibility().unwrap();
        let switch = tree
            .children
            .iter()
            .find(|n| n.role == Role::Switch)
            .unwrap();
        assert!(switch.focused);
        assert_eq!(switch.checked, Some(true));
        assert_eq!(s.input(Input::B, InputPhase::Pressed), Command::Quit);
    }
}
