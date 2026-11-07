//! Images: the runtime's registry, the updates shells drain, and Avatar and
//! Thumbnail painting a registered `src` with their fallbacks.

mod common;

use astryx_core::testing::draw;
use astryx_core::*;
use common::runtime;
use std::sync::Arc;

fn pixels(width: u32, height: u32) -> ImageData {
    ImageData {
        width,
        height,
        rgba: Arc::from(vec![255u8; (width * height * 4) as usize]),
    }
}

fn images(list: &DisplayList) -> Vec<(Rect, Radii, ImageId, ImageFit)> {
    list.commands
        .iter()
        .filter_map(|c| match c {
            DrawCmd::Image {
                rect,
                radii,
                image,
                fit,
            } => Some((*rect, *radii, *image, *fit)),
            _ => None,
        })
        .collect()
}

fn texts(list: &DisplayList) -> Vec<String> {
    list.commands
        .iter()
        .filter_map(|c| match c {
            DrawCmd::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn paths(list: &DisplayList) -> usize {
    list.commands
        .iter()
        .filter(|c| matches!(c, DrawCmd::Path { .. }))
        .count()
}

fn avatar(src: Option<&'static str>, fallback: Option<&'static str>) -> impl Fn() -> Element<()> {
    move || {
        let mut a = Avatar::<()>::new()
            .name("Ada Lovelace")
            .size(AvatarSize::Lg);
        if let Some(s) = src {
            a = a.src(s);
        }
        if let Some(f) = fallback {
            a = a.fallback_src(f);
        }
        a.into()
    }
}

#[test]
fn image_ids_are_stable_per_source() {
    assert_eq!(ImageId::of("a.png"), ImageId::of("a.png"));
    assert_ne!(ImageId::of("a.png"), ImageId::of("b.png"));
    const AT_COMPILE_TIME: ImageId = ImageId::of("");
    assert_eq!(AT_COMPILE_TIME, ImageId(0xcbf2_9ce4_8422_2325));
}

#[test]
fn fit_places_like_object_fit() {
    let rect = Rect::new(10.0, 10.0, 100.0, 50.0);
    // A square image: cover crops top and bottom, contain letterboxes.
    assert_eq!(
        ImageFit::Cover.place(rect, 20, 20),
        Rect::new(10.0, -15.0, 100.0, 100.0)
    );
    assert_eq!(
        ImageFit::Contain.place(rect, 20, 20),
        Rect::new(35.0, 10.0, 50.0, 50.0)
    );
    assert_eq!(ImageFit::Fill.place(rect, 20, 20), rect);
}

#[test]
fn updates_are_drained_once_and_keep_the_latest_per_image() {
    let mut rt = runtime::<()>(|| Text::new("hi").into());
    let a = ImageId(1);
    rt.register_image(a, Some(pixels(1, 1)));
    rt.register_image(a, Some(pixels(2, 2)));
    rt.register_image(ImageId(2), None);
    let updates = rt.take_image_updates();
    assert_eq!(updates.len(), 2);
    assert_eq!(updates[0].0, a);
    assert_eq!(updates[0].1.as_ref().map(|d| d.width), Some(2));
    assert_eq!(updates[1], (ImageId(2), None));
    assert!(rt.take_image_updates().is_empty());
    rt.forget_image(a);
    assert_eq!(rt.take_image_updates(), vec![(a, None)]);
    // Forgetting an unknown image sends nothing.
    rt.forget_image(ImageId(99));
    assert!(rt.take_image_updates().is_empty());
}

#[test]
fn avatar_shows_initials_while_its_image_loads() {
    let mut rt = runtime(avatar(Some("ada.png"), None));
    let list = draw(&mut rt, 400, 600);
    assert!(images(&list).is_empty());
    assert_eq!(texts(&list), vec!["AL".to_string()]);
}

#[test]
fn avatar_paints_its_ready_image_cover_fitted_in_its_shape() {
    let mut rt = runtime(avatar(Some("ada.png"), None));
    rt.register_image(ImageId::of("ada.png"), Some(pixels(4, 3)));
    let list = draw(&mut rt, 400, 600);
    let found = images(&list);
    assert_eq!(found.len(), 1);
    let (rect, radii, id, fit) = found[0];
    assert_eq!(id, ImageId::of("ada.png"));
    assert_eq!(fit, ImageFit::Cover);
    assert_eq!(
        (rect.width, rect.height),
        (AvatarSize::Lg.points(), AvatarSize::Lg.points())
    );
    // A circle: every corner is half the side.
    assert_eq!(radii, Radii::all(rect.width / 2.0));
    assert!(texts(&list).is_empty());
}

#[test]
fn avatar_tries_the_fallback_only_after_src_fails() {
    let mut rt = runtime(avatar(Some("ada.png"), Some("backup.png")));
    rt.register_image(ImageId::of("backup.png"), Some(pixels(2, 2)));
    let list = draw(&mut rt, 400, 600);
    assert!(images(&list).is_empty(), "src is still loading");
    rt.register_image(ImageId::of("ada.png"), None);
    let list = draw(&mut rt, 400, 600);
    assert_eq!(images(&list)[0].2, ImageId::of("backup.png"));
    // With no src at all, the fallback shows.
    let mut rt = runtime(avatar(None, Some("backup.png")));
    rt.register_image(ImageId::of("backup.png"), Some(pixels(2, 2)));
    assert_eq!(images(&draw(&mut rt, 400, 600)).len(), 1);
}

#[test]
fn avatar_falls_back_to_initials_when_every_image_fails() {
    let mut rt = runtime(avatar(Some("ada.png"), Some("backup.png")));
    rt.register_image(ImageId::of("ada.png"), None);
    rt.register_image(ImageId::of("backup.png"), None);
    let list = draw(&mut rt, 400, 600);
    assert!(images(&list).is_empty());
    assert_eq!(texts(&list), vec!["AL".to_string()]);
}

fn thumbnail(src: Option<&'static str>) -> impl Fn() -> Element<()> {
    move || {
        let mut t = Thumbnail::<()>::new().alt("Photo");
        if let Some(s) = src {
            t = t.src(s);
        }
        t.into()
    }
}

fn inset_borders(list: &DisplayList, color: Color) -> usize {
    list.commands
        .iter()
        .filter(|c| matches!(c, DrawCmd::Rect { stroke: Some((c, w)), fill, .. } if *c == color && *w == 1.0 && fill.a == 0.0))
        .count()
}

#[test]
fn thumbnail_paints_its_image_with_an_inset_border() {
    let border: Color = Theme::ASTRYX.light.color.border.into();
    let mut rt = runtime(thumbnail(Some("cat.png")));
    rt.register_image(ImageId::of("cat.png"), Some(pixels(8, 6)));
    let list = draw(&mut rt, 400, 600);
    let found = images(&list);
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].0.width, found[0].0.height), (64.0, 64.0));
    assert_eq!(found[0].3, ImageFit::Cover);
    assert_eq!(inset_borders(&list, border), 1);
    assert_eq!(paths(&list), 0, "no placeholder glyph over the image");
}

#[test]
fn thumbnail_is_blank_while_loading_and_shows_the_placeholder_after_failing() {
    let border: Color = Theme::ASTRYX.light.color.border.into();
    let mut rt = runtime(thumbnail(Some("cat.png")));
    let list = draw(&mut rt, 400, 600);
    assert!(images(&list).is_empty());
    assert_eq!(paths(&list), 0);
    assert_eq!(inset_borders(&list, border), 1);
    rt.register_image(ImageId::of("cat.png"), None);
    let list = draw(&mut rt, 400, 600);
    assert_eq!(paths(&list), 1);
    assert_eq!(inset_borders(&list, border), 0);
    let mut none = runtime(thumbnail(None));
    assert_eq!(paths(&draw(&mut none, 400, 600)), 1);
}
