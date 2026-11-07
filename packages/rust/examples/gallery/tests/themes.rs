//! Theme switching in the gallery: the picker, the D-pad shortcuts, focus
//! kept across switches, and goldens of non-default themes in both modes.

use astryx_core::testing::{draw, tap};
use astryx_core::*;
use astryx_gallery::Gallery;
use astryx_headless::golden::{assert_golden, Tolerance};
use astryx_headless::render_png;
use std::path::PathBuf;

const TOLERANCE: Tolerance = Tolerance {
    channel: 2,
    pixels: 64,
};

fn badge(rt: &Runtime<Gallery>) -> Option<String> {
    fn walk(n: &A11yNode, out: &mut Vec<A11yNode>) {
        out.push(n.clone());
        n.children.iter().for_each(|c| walk(c, out));
    }
    let mut all = Vec::new();
    walk(&rt.accessibility()?, &mut all);
    let theme_names: Vec<String> = Theme::ALL.iter().map(|t| t.name.to_string()).collect();
    all.into_iter()
        .filter_map(|n| n.label)
        .find(|l| l.contains(", ") && theme_names.iter().any(|t| l.to_lowercase().starts_with(t)))
}

#[test]
fn shoulder_buttons_step_through_every_theme_and_wrap() {
    let mut rt = Runtime::new(Gallery::new());
    draw(&mut rt, 960, 640);
    let mut seen = vec![rt.app().theme.name];
    for _ in 0..Theme::ALL.len() {
        tap(&mut rt, Input::R1);
        seen.push(rt.app().theme.name);
    }
    let names: Vec<&str> = Theme::ALL.iter().map(|t| t.name).collect();
    assert_eq!(&seen[..8], &names[..]);
    assert_eq!(seen[8], "astryx", "R1 wraps to the first theme");
    tap(&mut rt, Input::L1);
    assert_eq!(rt.app().theme.name, "y2k", "L1 wraps backward");
}

#[test]
fn select_cycles_light_dark_system() {
    let mut rt = Runtime::new(Gallery::new());
    assert_eq!(
        rt.app().mode,
        None,
        "the gallery follows the system at first"
    );
    let mut modes = Vec::new();
    for _ in 0..3 {
        tap(&mut rt, Input::Select);
        modes.push(rt.app().mode);
    }
    assert_eq!(modes, vec![Some(Mode::Light), Some(Mode::Dark), None]);
}

#[test]
fn system_mode_follows_preferences() {
    let mut rt = Runtime::new(Gallery::new());
    let light = draw(&mut rt, 960, 640).clear;
    rt.set_preferences(&Preferences {
        color_scheme: ColorScheme::Dark,
        ..Preferences::default()
    });
    let dark = draw(&mut rt, 960, 640).clear;
    assert_eq!(light, Theme::ASTRYX.light.color.background_body.into());
    assert_eq!(dark, Theme::ASTRYX.dark.color.background_body.into());
}

#[test]
fn switching_keeps_focus_and_scroll_and_names_the_theme_briefly() {
    let mut rt = Runtime::new(Gallery::new());
    draw(&mut rt, 960, 640);
    for _ in 0..12 {
        tap(&mut rt, Input::Next);
    }
    draw(&mut rt, 960, 640);
    let focused = rt.focused();
    let bounds = |rt: &Runtime<Gallery>| {
        fn find(n: &A11yNode) -> Option<Rect> {
            if n.focused {
                return Some(n.bounds);
            }
            n.children.iter().find_map(find)
        }
        find(&rt.accessibility().unwrap())
    };
    let visible = |r: Rect| r.y >= 0.0 && r.y + r.height <= 640.0;
    assert!(
        visible(bounds(&rt).unwrap()),
        "focus scrolled into view: {:?}",
        bounds(&rt)
    );
    tap(&mut rt, Input::R1);
    let list = draw(&mut rt, 960, 640);
    assert_eq!(rt.focused(), focused, "focus stays on the same control");
    // Themes size controls differently, so the control may move, but it stays in view.
    assert!(visible(bounds(&rt).unwrap()), "{:?}", bounds(&rt));
    assert_eq!(list.clear, Theme::BUTTER.light.color.background_body.into());
    assert_eq!(badge(&rt).as_deref(), Some("Butter, system mode"));
    // Any other message clears the label.
    tap(&mut rt, Input::B);
    draw(&mut rt, 960, 640);
    assert_eq!(badge(&rt), None);
}

#[test]
fn picker_selectors_change_theme_and_mode() {
    let mut rt = Runtime::new(Gallery::new());
    draw(&mut rt, 960, 640);
    // The theme picker is the first stop; open it and pick the third theme.
    tap(&mut rt, Input::A);
    draw(&mut rt, 960, 640);
    tap(&mut rt, Input::Down);
    tap(&mut rt, Input::Down);
    tap(&mut rt, Input::A);
    assert_eq!(rt.app().theme.name, Theme::ALL[2].name);
    draw(&mut rt, 960, 640);
    // The mode picker is next; its menu opens on System, so Up is Dark.
    tap(&mut rt, Input::Right);
    tap(&mut rt, Input::A);
    draw(&mut rt, 960, 640);
    tap(&mut rt, Input::Up);
    tap(&mut rt, Input::A);
    assert_eq!(rt.app().mode, Some(Mode::Dark));
    assert_eq!(badge(&rt), None, "the picker needs no label");
}

fn golden(gallery: Gallery, inputs: &[Input], name: &str) {
    let mut rt = Runtime::new(gallery);
    let png = render_png(&mut rt, 960, 640, 1.0, inputs);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens")
        .join(name);
    assert_golden(&png, path, TOLERANCE);
}

#[test]
fn non_default_themes_in_both_modes() {
    for theme in [Theme::GOTHIC, Theme::MATCHA, Theme::Y2K] {
        for (mode, m) in [(Mode::Light, "light"), (Mode::Dark, "dark")] {
            golden(
                Gallery::new().with_theme(theme, mode),
                &[],
                &format!("theme-{}-{m}.png", theme.name),
            );
        }
    }
}

#[test]
fn shortcut_label_golden() {
    golden(
        Gallery::new(),
        &[Input::R1, Input::R1, Input::Select],
        "theme-shortcut-label.png",
    );
}
