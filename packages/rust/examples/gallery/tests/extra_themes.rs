//! Themes outside the published set, such as pixel themes, sit in the
//! picker and on R1 and L1 when a shell adds them; the default is unchanged.

use astryx_core::{App, Input, Theme};
use astryx_gallery::{Gallery, Msg};
use astryx_theme_pixel::PixelTheme;

#[test]
fn the_default_gallery_starts_on_astryx_with_the_published_themes() {
    let g = Gallery::new();
    assert!(std::ptr::eq(g.theme, Theme::ASTRYX));
    assert!(g.extra_themes.is_empty());
    assert_eq!(g.mode, None);
}

#[test]
fn extra_themes_come_first_and_the_picker_returns_to_them() {
    let terrace = PixelTheme::Terrace.theme();
    let mut g = Gallery::new().with_extra_themes(PixelTheme::ALL.map(|p| p.theme()));
    g.theme = terrace;
    // R1 steps through every pixel theme, then the published ones, then wraps.
    let total = PixelTheme::ALL.len() + Theme::ALL.len();
    for _ in 0..total {
        let msg = g.on_input(Input::R1).unwrap();
        let _ = g.update(msg);
    }
    assert!(
        std::ptr::eq(g.theme, terrace),
        "back on Terrace after a full turn"
    );
    let _ = g.update(Msg::Theme("astryx".into()));
    assert!(std::ptr::eq(g.theme, Theme::ASTRYX));
    let _ = g.update(Msg::Theme("terrace".into()));
    assert!(std::ptr::eq(g.theme, terrace), "picked by name");
}
