//! The renderer redraws only what changed since its last frame. Every frame
//! it draws must equal, pixel for pixel, the same frame drawn whole by a
//! renderer that has drawn nothing before.

use std::sync::Arc;

use astryx_core::*;
use astryx_render_soft::SoftRenderer;

/// A small deterministic random source.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
    fn f(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * (self.next() % 10_000) as f32 / 10_000.0
    }
    fn color(&mut self) -> Color {
        Color::rgba(
            self.f(0.0, 1.0),
            self.f(0.0, 1.0),
            self.f(0.0, 1.0),
            self.f(0.3, 1.0),
        )
    }
}

const IMAGE: &str = "damage/checker.png";

fn checker() -> ImageData {
    let mut px = Vec::new();
    for y in 0..8u32 {
        for x in 0..8u32 {
            let on = (x + y) % 2 == 0;
            px.extend_from_slice(if on {
                &[220, 40, 40, 255]
            } else {
                &[40, 40, 220, 160]
            });
        }
    }
    ImageData {
        width: 8,
        height: 8,
        rgba: Arc::from(px),
    }
}

fn style(size: f32, weight: u16) -> TextStyle {
    TextStyle {
        direction: None,
        family: FontFamily::Body,
        size,
        weight,
        leading: 1.4,
    }
}

/// One drawing command anywhere in a `w` x `h` point frame.
fn leaf(r: &mut Rng, w: f32, h: f32) -> DrawCmd {
    let rect = Rect::new(
        r.f(-20.0, w),
        r.f(-20.0, h),
        r.f(4.0, w / 2.0),
        r.f(4.0, h / 3.0),
    );
    match r.below(8) {
        0 | 1 => DrawCmd::Rect {
            rect,
            radii: Radii::all(r.f(0.0, 12.0)),
            fill: r.color(),
            stroke: (r.below(2) == 0).then(|| (r.color(), r.f(1.0, 3.0))),
        },
        2 => DrawCmd::Shadow {
            rect,
            radii: Radii::all(r.f(0.0, 10.0)),
            layers: vec![ShadowLayer {
                x: r.f(-4.0, 4.0),
                y: r.f(0.0, 8.0),
                blur: r.f(0.0, 16.0),
                spread: r.f(-2.0, 4.0),
                inset: r.below(4) == 0,
                color: Color::rgba(0.0, 0.0, 0.0, r.f(0.1, 0.5)),
            }],
        },
        3 | 4 => DrawCmd::Text {
            rect: Rect::new(rect.x, rect.y, r.f(30.0, 200.0), 24.0),
            text: ["Wi-Fi", "Settings saved", "Ñandú gáfas", "Row 12", "Agj ÅÉ"]
                [r.below(5) as usize]
                .into(),
            style: style(r.f(10.0, 24.0), [400, 500, 700][r.below(3) as usize]),
            color: r.color(),
            align: [TextAlign::Start, TextAlign::Center, TextAlign::End][r.below(3) as usize],
        },
        5 => DrawCmd::Line {
            from: Point {
                x: rect.x,
                y: rect.y,
            },
            to: if r.below(2) == 0 {
                Point {
                    x: rect.x + rect.width,
                    y: rect.y,
                }
            } else {
                Point {
                    x: rect.x + rect.width,
                    y: rect.y + rect.height,
                }
            },
            color: r.color(),
            width: r.f(0.5, 4.0),
        },
        6 => DrawCmd::Path {
            commands: vec![
                PathCmd::MoveTo(Point {
                    x: rect.x,
                    y: rect.y,
                }),
                PathCmd::LineTo(Point {
                    x: rect.x + rect.width,
                    y: rect.y + rect.height / 2.0,
                }),
                PathCmd::QuadTo(
                    Point {
                        x: rect.x + rect.width / 2.0,
                        y: rect.y + rect.height * 1.5,
                    },
                    Point {
                        x: rect.x,
                        y: rect.y + rect.height,
                    },
                ),
                PathCmd::Close,
            ],
            fill: (r.below(2) == 0).then(|| r.color()),
            stroke: Some((r.color(), r.f(1.0, 3.0))),
        },
        _ => DrawCmd::Image {
            rect,
            radii: Radii::all(r.f(0.0, 8.0)),
            image: ImageId::of(IMAGE),
            fit: [ImageFit::Cover, ImageFit::Contain, ImageFit::Fill][r.below(3) as usize],
        },
    }
}

/// A list of leaves inside nested groups of every kind.
fn page(r: &mut Rng, w: f32, h: f32, depth: u32) -> Vec<DrawCmd> {
    let mut out = Vec::new();
    for _ in 0..(3 + r.below(6)) {
        if depth < 3 && r.below(4) == 0 {
            let inner = page(r, w, h, depth + 1);
            let (open, close) = match r.below(4) {
                0 => (
                    DrawCmd::PushClip {
                        rect: Rect::new(
                            r.f(0.0, w / 2.0),
                            r.f(0.0, h / 2.0),
                            r.f(20.0, w),
                            r.f(20.0, h),
                        ),
                        radii: Radii::all(r.f(0.0, 16.0)),
                    },
                    DrawCmd::PopClip,
                ),
                1 => (DrawCmd::PushOpacity(r.f(0.2, 1.0)), DrawCmd::PopOpacity),
                2 => (
                    DrawCmd::PushTransform {
                        origin: Point {
                            x: w / 2.0,
                            y: h / 2.0,
                        },
                        scale: [1.0, 0.9, 1.25][r.below(3) as usize],
                        translate: Point {
                            x: r.f(-10.0, 10.0).round(),
                            y: r.f(-10.0, 10.0).round(),
                        },
                    },
                    DrawCmd::PopTransform,
                ),
                _ => (DrawCmd::PushPixelGrid { cell: 2.0 }, DrawCmd::PopPixelGrid),
            };
            out.push(open);
            out.extend(inner);
            out.push(close);
        } else {
            out.push(leaf(r, w, h));
        }
    }
    out
}

/// A change to a page, as the frames of an app make them: one command
/// changes, moves, appears or goes, or a run of them shifts.
fn change(r: &mut Rng, cmds: &mut Vec<DrawCmd>, w: f32, h: f32) {
    let n = cmds.len();
    if n == 0 {
        cmds.push(leaf(r, w, h));
        return;
    }
    let i = r.below(n as u64) as usize;
    let structural = |c: &DrawCmd| {
        matches!(
            c,
            DrawCmd::PushClip { .. }
                | DrawCmd::PopClip
                | DrawCmd::PushOpacity(_)
                | DrawCmd::PopOpacity
                | DrawCmd::PushTransform { .. }
                | DrawCmd::PopTransform
                | DrawCmd::PushPixelGrid { .. }
                | DrawCmd::PopPixelGrid
        )
    };
    match r.below(6) {
        // Recolor or move one leaf.
        0 | 1 => {
            if !structural(&cmds[i]) {
                cmds[i] = leaf(r, w, h);
            }
        }
        // A leaf appears.
        2 => cmds.insert(i, leaf(r, w, h)),
        // A leaf goes.
        3 => {
            if !structural(&cmds[i]) {
                cmds.remove(i);
            }
        }
        // A run shifts down, as content below an inserted line does.
        4 => {
            let dy = r.f(4.0, 30.0);
            for c in cmds.iter_mut().skip(i) {
                match c {
                    DrawCmd::Rect { rect, .. }
                    | DrawCmd::Shadow { rect, .. }
                    | DrawCmd::Text { rect, .. }
                    | DrawCmd::Image { rect, .. } => rect.y += dy,
                    _ => {}
                }
            }
        }
        // A group changes: a clip moves, an opacity or a scale changes.
        _ => {
            for c in cmds.iter_mut().skip(i) {
                match c {
                    DrawCmd::PushClip { rect, .. } => {
                        rect.x += r.f(-8.0, 8.0);
                        break;
                    }
                    DrawCmd::PushOpacity(o) => {
                        *o = r.f(0.2, 1.0);
                        break;
                    }
                    DrawCmd::PushTransform { scale, .. } => {
                        *scale = [1.0, 0.95, 1.1][r.below(3) as usize];
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}

fn whole(list: &DisplayList, frame: &FrameInfo) -> Vec<u8> {
    let mut fresh = SoftRenderer::new();
    fresh.set_image(ImageId::of(IMAGE), Some(&checker()));
    fresh.render_list(list, frame);
    fresh.pixmap().data().to_vec()
}

fn first_difference(a: &[u8], b: &[u8], w: u32) -> String {
    match a.iter().zip(b).position(|(x, y)| x != y) {
        Some(i) => {
            let px = i as u32 / 4;
            format!(
                "first at ({}, {}): {:?} vs {:?}",
                px % w,
                px / w,
                &a[i / 4 * 4..i / 4 * 4 + 4],
                &b[i / 4 * 4..i / 4 * 4 + 4]
            )
        }
        None => "sizes differ".into(),
    }
}

#[test]
fn every_frame_equals_the_same_frame_drawn_whole() {
    let mut frames = 0;
    for seed in 1..=10u64 {
        let mut r = Rng(seed);
        let (w, h) = (
            [240.0, 317.0, 400.0][r.below(3) as usize],
            [180.0, 263.0][r.below(2) as usize],
        );
        let scale = [1.0, 2.0, 1.5][r.below(3) as usize];
        let frame = FrameInfo {
            width: (w * scale) as u32,
            height: (h * scale) as u32,
            scale,
            dt: 1.0 / 60.0,
        };
        let mut renderer = SoftRenderer::new();
        renderer.set_image(ImageId::of(IMAGE), Some(&checker()));
        let mut list = DisplayList {
            clear: Color::rgba(0.96, 0.96, 0.97, 1.0),
            commands: page(&mut r, w, h, 0),
        };
        for step in 0..12 {
            if step > 0 {
                for _ in 0..=r.below(2) {
                    change(&mut r, &mut list.commands, w, h);
                }
                if r.below(25) == 0 {
                    list.clear = r.color();
                    list.clear.a = 1.0;
                }
            }
            renderer.render_list(&list, &frame);
            let want = whole(&list, &frame);
            let got = renderer.pixmap().data();
            assert!(
                got == &want[..],
                "seed {seed}, frame {step}: redrawn frame differs from the whole frame, {}",
                first_difference(got, &want, frame.width)
            );
            frames += 1;
        }
    }
    assert_eq!(frames, 10 * 12);
}

#[test]
fn an_unchanged_frame_and_a_small_change_redraw_little() {
    // A switch thumb moving on a page of rows: the frame after it equals a
    // whole frame (above), and the renderer must not touch pixels far from
    // it. Pixels painted over outside the change would show up as a frame
    // that differs from the one before only there; check with a page whose
    // background the renderer cannot know changed.
    let mut list = DisplayList {
        clear: Color::rgba(1.0, 1.0, 1.0, 1.0),
        commands: (0..12)
            .map(|i| DrawCmd::Rect {
                rect: Rect::new(10.0, 10.0 + i as f32 * 24.0, 200.0, 20.0),
                radii: Radii::all(4.0),
                fill: Color::rgba(0.2, 0.4, 0.8, 1.0),
                stroke: None,
            })
            .collect(),
    };
    let frame = FrameInfo {
        width: 240,
        height: 320,
        scale: 1.0,
        dt: 0.0,
    };
    let mut renderer = SoftRenderer::new();
    renderer.render_list(&list, &frame);
    let before = renderer.pixmap().data().to_vec();
    renderer.render_list(&list, &frame);
    assert_eq!(
        renderer.pixmap().data(),
        &before[..],
        "an unchanged frame stays"
    );
    if let DrawCmd::Rect { fill, .. } = &mut list.commands[5] {
        *fill = Color::rgba(0.9, 0.2, 0.2, 1.0);
    }
    renderer.render_list(&list, &frame);
    let after = renderer.pixmap().data();
    let changed_rows: Vec<u32> = (0..320u32)
        .filter(|&y| {
            let row = (y * 240 * 4) as usize..((y + 1) * 240 * 4) as usize;
            after[row.clone()] != before[row]
        })
        .collect();
    let (first, last) = (changed_rows[0], *changed_rows.last().unwrap());
    assert!(
        first >= 10 + 5 * 24 && last < 10 + 5 * 24 + 20,
        "only row 5's pixels change: rows {first}..={last}"
    );
    assert_eq!(after, &whole(&list, &frame)[..]);
}
