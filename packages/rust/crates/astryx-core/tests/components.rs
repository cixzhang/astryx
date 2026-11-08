//! Display-list tests: each component state paints the geometry and token
//! colors Astryx specifies.

mod common;
use astryx_core::testing::{draw, tap};
use astryx_core::*;
use common::*;

// ---- Button ----

#[test]
fn button_md_primary_paints_accent_at_element_height() {
    let t = light();
    let list = paint(|| {
        Button::<()>::new("Save")
            .variant(ButtonVariant::Primary)
            .into()
    });
    let (rect, radii, _) = rect_filled(&list, c(t.color.accent)).expect("accent fill");
    assert_eq!(rect.height, t.size.element_md);
    assert_eq!(radii, Radii::all(t.radius.element));
    let (label, style, color) = text(&list, "Save");
    assert_eq!(color, c(t.color.on_accent));
    assert_eq!(
        (style.size, style.weight),
        (t.font.label.size, t.font.label.weight)
    );
    // Inline padding is spacing step 3 on each side; the label is centered.
    assert_eq!(label.x - rect.x, t.space.s3);
    assert_eq!(rect.width, label.width + 2.0 * t.space.s3);
    assert_eq!(label.y + label.height / 2.0, rect.y + rect.height / 2.0);
}

#[test]
fn button_sizes_use_element_heights() {
    let t = light();
    for (size, h) in [
        (ButtonSize::Sm, t.size.element_sm),
        (ButtonSize::Md, t.size.element_md),
        (ButtonSize::Lg, t.size.element_lg),
    ] {
        let list = paint(move || Button::<()>::new("Go").size(size).into());
        let (rect, _, _) = rect_filled(&list, c(t.color.neutral)).unwrap();
        assert_eq!(rect.height, h, "{size:?}");
    }
}

#[test]
fn button_variant_colors_follow_tokens() {
    let t = light();
    let cases = [
        (
            ButtonVariant::Secondary,
            Some(t.color.neutral),
            t.color.text_primary,
        ),
        (ButtonVariant::Ghost, None, t.color.text_primary),
        (
            ButtonVariant::Destructive,
            Some(t.color.error),
            t.color.on_error,
        ),
    ];
    for (variant, fill, fg) in cases {
        let list = paint(move || Button::<()>::new("Delete").variant(variant).into());
        match fill {
            Some(f) => assert!(rect_filled(&list, c(f)).is_some(), "{variant:?} fill"),
            // Ghost paints no ground until pressed.
            None => assert!(
                rects(&list).iter().all(|r| r.2.a == 0.0),
                "ghost paints a fill"
            ),
        }
        assert_eq!(text(&list, "Delete").2, c(fg), "{variant:?} label");
    }
}

#[test]
fn button_content_width_hugs_and_fill_spans() {
    let t = light();
    let hug = paint(|| {
        VStack::<()>::new()
            .padding(Space::S4)
            .child(Button::new("Hi"))
            .into()
    });
    let (r, _, _) = rect_filled(&hug, c(t.color.neutral)).unwrap();
    assert_eq!(r.width, 14.0 + 2.0 * t.space.s3);
    let fill = paint(|| {
        VStack::<()>::new()
            .padding(Space::S4)
            .child(Button::new("Hi").width(ButtonWidth::Fill))
            .into()
    });
    let (r, _, _) = rect_filled(&fill, c(t.color.neutral)).unwrap();
    assert_eq!(r.width, 400.0 - 2.0 * t.space.s4);
}

#[test]
fn button_elevation_paints_the_shadow_token_first() {
    let t = light();
    let list = paint(|| Button::<()>::new("Up").elevation(Elevation::Med).into());
    let shadow = list
        .commands
        .iter()
        .position(|c| matches!(c, DrawCmd::Shadow { .. }))
        .expect("shadow");
    let fill = list
        .commands
        .iter()
        .position(|c| matches!(c, DrawCmd::Rect { .. }))
        .unwrap();
    assert!(shadow < fill);
    let DrawCmd::Shadow { layers, .. } = &list.commands[shadow] else {
        unreachable!()
    };
    assert_eq!(layers.len(), t.shadow.med.len());
    assert_eq!(
        (layers[1].y, layers[1].blur),
        (t.shadow.med[1].y, t.shadow.med[1].blur)
    );
}

#[test]
fn focused_button_shows_the_astryx_ring() {
    let t = light();
    let list = paint(|| {
        Button::<()>::new("Ok")
            .variant(ButtonVariant::Primary)
            .into()
    });
    let (rect, _, _) = rect_filled(&list, c(t.color.accent)).unwrap();
    let ring = rings(&list, c(t.color.accent));
    assert_eq!(ring.len(), 1);
    assert_eq!(ring[0].0, ring_rect(rect));
    assert_eq!(ring[0].2, 2.0);
    assert_eq!(ring[0].1, Radii::all(t.radius.element + 5.0));
}

#[test]
fn destructive_ring_takes_the_error_color() {
    let t = light();
    let list = paint(|| {
        Button::<()>::new("Delete")
            .variant(ButtonVariant::Destructive)
            .into()
    });
    assert_eq!(rings(&list, c(t.color.border_error)).len(), 1);
    assert!(rings(&list, c(t.color.accent)).is_empty());
}

#[test]
fn pressing_a_paints_the_pressed_overlay_and_release_activates() {
    let t = light();
    let mut rt = runtime(|| Button::new("Ok").on_click("ok").into());
    draw(&mut rt, 400, 600);
    rt.input(Input::A, InputPhase::Pressed);
    let pressed = draw(&mut rt, 400, 600);
    assert!(rect_filled(&pressed, c(t.color.overlay_pressed)).is_some());
    assert!(rt.app().sent.is_empty(), "activates on release, not press");
    rt.input(Input::A, InputPhase::Released);
    assert_eq!(rt.app().sent, vec!["ok"]);
    // The press fades out on the theme's exit motion after release.
    let overlay = c(t.color.overlay_pressed);
    // The overlay paints right after the button's own fill, which shares
    // its hue in this theme, so it is the second matching rect.
    let alpha = |l: &DisplayList| {
        rects(l)
            .iter()
            .filter(|r| r.2.r == overlay.r && r.2.g == overlay.g && r.2.b == overlay.b)
            .nth(1)
            .map(|r| r.2.a)
    };
    let at = |rt: &mut Runtime<View<&str>>, dt: f32| {
        rt.frame(
            &FrameInfo {
                width: 400,
                height: 600,
                scale: 1.0,
                dt,
            },
            &measure(),
        )
    };
    let start = at(&mut rt, 0.0);
    assert_eq!(alpha(&start), Some(overlay.a));
    assert!(rt.wants_frames());
    let mid = at(&mut rt, 0.08);
    assert!(
        alpha(&mid).is_some_and(|a| a > 0.0 && a < overlay.a),
        "{:?}",
        alpha(&mid)
    );
    let end = at(&mut rt, 0.3);
    assert_eq!(alpha(&end), None);
    assert!(!rt.wants_frames());
}

#[test]
fn reduced_motion_ends_a_press_at_once() {
    let t = light();
    let mut rt = runtime(|| Button::new("Ok").on_click("ok").into());
    rt.set_preferences(&Preferences {
        reduced_motion: true,
        ..Preferences::default()
    });
    draw(&mut rt, 400, 600);
    rt.input(Input::A, InputPhase::Pressed);
    rt.input(Input::A, InputPhase::Released);
    let list = draw(&mut rt, 400, 600);
    assert!(rect_filled(&list, c(t.color.overlay_pressed)).is_none());
}

#[test]
fn a_held_past_the_long_press_does_not_activate() {
    let mut rt = runtime(|| Button::new("Ok").on_click("ok").into());
    draw(&mut rt, 400, 600);
    rt.input(Input::A, InputPhase::Pressed);
    assert!(rt.wants_frames(), "a held press may become a long press");
    rt.frame(
        &FrameInfo {
            width: 400,
            height: 600,
            scale: 1.0,
            dt: 0.6,
        },
        &measure(),
    );
    rt.input(Input::A, InputPhase::Released);
    assert!(rt.app().sent.is_empty());
}

#[test]
fn disabled_button_is_dimmed_focusable_and_inert() {
    let t = light();
    let mut rt = runtime(|| Button::new("Off").is_disabled(true).on_click(1).into());
    let list = draw(&mut rt, 400, 600);
    assert!(list.commands.contains(&DrawCmd::PushOpacity(0.5)));
    let node = &a11y(&rt, Role::Button)[0];
    assert!(node.disabled && node.focused);
    rt.input(Input::A, InputPhase::Pressed);
    let pressed = draw(&mut rt, 400, 600);
    assert!(
        rect_filled(&pressed, c(t.color.overlay_pressed)).is_none(),
        "no press paint"
    );
    rt.input(Input::A, InputPhase::Released);
    assert!(rt.app().sent.is_empty());
}

#[test]
fn loading_button_keeps_its_width_hides_the_label_and_spins() {
    let t = light();
    let plain = paint(|| Button::<()>::new("Upload").into());
    let mut rt = runtime(|| Button::new("Upload").is_loading(true).on_click(1).into());
    let loading = draw(&mut rt, 400, 600);
    let width = |l: &DisplayList| rect_filled(l, c(t.color.neutral)).unwrap().0.width;
    assert_eq!(width(&plain), width(&loading));
    assert!(
        texts(&loading).is_empty(),
        "label is transparent while loading"
    );
    assert!(has(
        &loading,
        |c| matches!(c, DrawCmd::Path { stroke: Some((col, _)), .. } if *col == Color::from(t.color.text_primary))
    ));
    assert!(rt.wants_frames());
    assert_eq!(a11y(&rt, Role::Button)[0].value.as_deref(), Some("Loading"));
    tap(&mut rt, Input::A);
    assert!(rt.app().sent.is_empty(), "busy ignores presses");
}

#[test]
fn interruptible_loading_button_still_activates() {
    let mut rt = runtime(|| {
        Button::new("Stop")
            .is_loading(true)
            .is_interruptible(true)
            .on_click(7)
            .into()
    });
    draw(&mut rt, 400, 600);
    tap(&mut rt, Input::A);
    assert_eq!(rt.app().sent, vec![7]);
}

#[test]
fn long_button_label_truncates_in_a_narrow_row() {
    let mut rt = runtime(|| {
        HStack::<()>::new()
            .child(Button::new(
                "A label that is far too long for one hundred points",
            ))
            .into()
    });
    let list = draw(&mut rt, 120, 100);
    let shown = &texts(&list)[0].1;
    assert!(shown.ends_with('\u{2026}'), "{shown}");
    assert!(texts(&list)[0].0.width <= 120.0 - 24.0);
}

// ---- Card ----

#[test]
fn default_card_paints_card_ground_border_and_container_radius() {
    let t = light();
    let list = paint(|| Card::<()>::new().child(Text::new("Inside")).into());
    let (rect, radii, stroke) = rect_filled(&list, c(t.color.background_card)).unwrap();
    assert_eq!(radii, Radii::all(t.radius.container));
    assert_eq!(stroke, Some((c(t.color.border), t.size.border_width)));
    let (inside, _, _) = text(&list, "Inside");
    // The border sits inside the padding: content inset is the padding step.
    assert_eq!(
        (inside.x - rect.x, inside.y - rect.y),
        (t.space.s4, t.space.s4)
    );
    assert!(has(
        &list,
        |c| matches!(c, DrawCmd::PushClip { radii, .. } if *radii == Radii::all(t.radius.container))
    ));
}

#[test]
fn card_variants_paint_their_grounds_and_only_default_has_a_border() {
    let t = light();
    for (variant, ground) in [
        (CardVariant::Muted, t.color.background_muted),
        (CardVariant::Blue, t.color.background_blue),
        (CardVariant::Yellow, t.color.background_yellow),
    ] {
        let list = paint(move || Card::<()>::new().variant(variant).into());
        let (_, _, stroke) = rect_filled(&list, c(ground)).unwrap_or_else(|| panic!("{variant:?}"));
        assert_eq!(stroke, None);
    }
    let transparent = paint(|| {
        Card::<()>::new()
            .variant(CardVariant::Transparent)
            .child(Text::new("x"))
            .into()
    });
    assert!(rects(&transparent).is_empty());
}

#[test]
fn card_padding_and_elevation() {
    let t = light();
    let list = paint(|| {
        Card::<()>::new()
            .padding(Space::S2)
            .elevation(Elevation::High)
            .child(Text::new("P"))
            .into()
    });
    let (rect, _, _) = rect_filled(&list, c(t.color.background_card)).unwrap();
    assert_eq!(text(&list, "P").0.x - rect.x, t.space.s2);
    assert!(has(
        &list,
        |c| matches!(c, DrawCmd::Shadow { layers, .. } if layers.len() == t.shadow.high.len())
    ));
}

// ---- Text and Heading ----

#[test]
fn text_types_use_their_type_styles_and_default_colors() {
    let t = light();
    let list = paint(|| {
        VStack::<()>::new()
            .child(Text::new("body"))
            .child(Text::new("supporting").r#type(TextType::Supporting))
            .child(Text::new("code").r#type(TextType::Code))
            .child(Text::new("display").r#type(TextType::Display1))
            .into()
    });
    let (r, s, col) = text(&list, "body");
    assert_eq!(
        (s.size, s.weight, s.family),
        (t.font.body.size, t.font.body.weight, FontFamily::Body)
    );
    assert_eq!(col, c(t.color.text_primary));
    assert_eq!(r.height, 20.0, "one line of 14 × 1.4286");
    let (_, s, col) = text(&list, "supporting");
    assert_eq!(
        (s.size, col),
        (t.font.supporting.size, c(t.color.text_secondary))
    );
    assert_eq!(text(&list, "code").1.family, FontFamily::Code);
    let (_, s, _) = text(&list, "display");
    assert_eq!(
        (s.size, s.family),
        (t.font.display1.size, FontFamily::Heading)
    );
}

#[test]
fn text_overrides_size_weight_color_and_justify() {
    let t = light();
    let list = paint(|| {
        Text::new("x")
            .size(TextSize::S2xl)
            .weight(TextWeight::Bold)
            .color(TextColor::Accent)
            .justify(TextJustify::Center)
            .into_el()
    });
    let DrawCmd::Text {
        style,
        color,
        align,
        ..
    } = list
        .commands
        .iter()
        .find(|c| matches!(c, DrawCmd::Text { .. }))
        .unwrap()
    else {
        unreachable!()
    };
    assert_eq!(
        (style.size, style.weight),
        (t.font.size.s2xl, t.font.weight.bold)
    );
    assert_eq!(*color, c(t.color.text_accent));
    assert_eq!(*align, TextAlign::Center);
}

#[test]
fn placeholder_and_disabled_text_colors() {
    let t = light();
    let list = paint(|| {
        VStack::<()>::new()
            .child(Text::new("p").color(TextColor::Placeholder))
            .child(Text::new("d").color(TextColor::Disabled))
            .into()
    });
    assert_eq!(text(&list, "p").2, c(t.color.text_secondary));
    assert_eq!(text(&list, "d").2, c(t.color.text_disabled));
}

#[test]
fn text_wraps_to_whole_lines_and_max_lines_truncates() {
    let words = "one two three four five six seven eight nine ten eleven twelve";
    let mut rt = runtime(move || {
        VStack::<()>::new()
            .child(Text::new(words))
            .child(Text::new(words).max_lines(2))
            .into()
    });
    let list = draw(&mut rt, 100, 600);
    let all = texts(&list);
    assert_eq!(all[0].0.height % 20.0, 0.0);
    assert!(all[0].0.height > 40.0);
    assert_eq!(all[1].0.height, 40.0);
    assert!(all[1].1.ends_with('\u{2026}'));
}

#[test]
fn strikethrough_draws_a_line_per_text_line() {
    let list = paint(|| Text::new("gone").has_strikethrough(true).into_el());
    assert_eq!(
        list.commands
            .iter()
            .filter(|c| matches!(c, DrawCmd::Line { .. }))
            .count(),
        1
    );
}

#[test]
fn heading_levels_use_heading_tokens_and_expose_levels() {
    let t = light();
    let mut rt = runtime(|| {
        VStack::<()>::new()
            .child(Heading::new(1, "One"))
            .child(Heading::new(4, "Four"))
            .child(Heading::new(2, "Shown as two, read as three").accessibility_level(3))
            .child(Heading::new(2, "Display").r#type(HeadingType::Display2))
            .into()
    });
    let list = draw(&mut rt, 400, 600);
    let (_, s, col) = text(&list, "One");
    assert_eq!(
        (s.size, s.weight, s.family),
        (
            t.font.heading1.size,
            t.font.heading1.weight,
            FontFamily::Heading
        )
    );
    assert_eq!(col, c(t.color.text_primary));
    assert_eq!(text(&list, "Four").1.size, t.font.heading4.size);
    assert_eq!(text(&list, "Display").1.size, t.font.display2.size);
    let levels: Vec<Role> = a11y_nodes(&rt.accessibility().unwrap())
        .iter()
        .map(|n| n.role)
        .filter(|r| matches!(r, Role::Heading(_)))
        .collect();
    assert_eq!(
        levels,
        vec![
            Role::Heading(1),
            Role::Heading(4),
            Role::Heading(3),
            Role::Heading(2)
        ]
    );
}

// ---- Stacks ----

#[test]
fn stacks_place_children_by_gap_and_padding() {
    let t = light();
    let list = paint(|| {
        VStack::<()>::new()
            .gap(Space::S3)
            .padding(Space::S4)
            .padding_block_start(Space::S8)
            .child(Text::new("a"))
            .child(
                HStack::new()
                    .gap(Space::S2)
                    .child(Text::new("b"))
                    .child(Text::new("c")),
            )
            .into()
    });
    let (a, b, cc) = (text(&list, "a").0, text(&list, "b").0, text(&list, "c").0);
    assert_eq!((a.x, a.y), (t.space.s4, t.space.s8));
    assert_eq!(b.y, a.y + a.height + t.space.s3);
    assert_eq!(cc.x, b.x + b.width + t.space.s2);
}

#[test]
fn stack_alignment_props_map_to_axes() {
    let list = paint(|| {
        VStack::<()>::new()
            .child(
                HStack::new()
                    .h_align(StackJustify::End)
                    .child(Text::new("end")),
            )
            .child(
                VStack::new()
                    .h_align(StackAlign::Center)
                    .child(Text::new("mid")),
            )
            .child(
                HStack::new()
                    .h_align(StackJustify::Between)
                    .child(Text::new("L"))
                    .child(Text::new("R")),
            )
            .into()
    });
    let end = text(&list, "end").0;
    assert_eq!(end.x + end.width, 400.0);
    let mid = text(&list, "mid").0;
    assert!((mid.x + mid.width / 2.0 - 200.0).abs() <= 0.5);
    assert_eq!(text(&list, "L").0.x, 0.0);
    let r = text(&list, "R").0;
    assert_eq!(r.x + r.width, 400.0);
}

#[test]
fn stack_wraps_children_onto_new_lines() {
    let mut rt = runtime(|| {
        HStack::<()>::new()
            .gap(Space::S2)
            .wrap(StackWrap::Wrap)
            .children((0..6).map(|i| Text::new(format!("item{i}"))))
            .into()
    });
    let list = draw(&mut rt, 120, 400);
    let ys: Vec<f32> = texts(&list).iter().map(|t| t.0.y).collect();
    assert!(ys.iter().any(|&y| y > 0.0), "{ys:?}");
}

#[test]
fn hidden_elements_are_not_built() {
    let list = paint(|| {
        VStack::<()>::new()
            .child(Text::new("shown"))
            .child(Text::new("gone").is_hidden(true))
            .into()
    });
    assert_eq!(texts(&list).len(), 1);
}

// ---- Switch ----

#[test]
fn switch_off_and_on_geometry_and_colors() {
    let t = light();
    for (on, track_color, thumb) in [
        (false, t.color.background_gray, (16.0, 4.0, 4.0)),
        (true, t.color.accent, (20.0, 18.0, 2.0)),
    ] {
        let list = paint(move || Switch::<()>::new("Wi-Fi", on).into());
        let (track, radii, _) = rect_filled(&list, c(track_color)).unwrap();
        assert_eq!((track.width, track.height), (40.0, 24.0));
        assert_eq!(radii, Radii::all(12.0));
        let (knob, _, _) = rect_filled(&list, c(t.color.background_surface)).unwrap();
        assert_eq!(
            (knob.width, knob.x - track.x, knob.y - track.y),
            thumb,
            "on = {on}"
        );
        let (label, style, color) = text(&list, "Wi-Fi");
        assert_eq!(label.x, track.x + track.width + t.space.s2);
        assert_eq!(
            (style.size, style.weight),
            (t.font.label.size, t.font.weight.medium)
        );
        assert_eq!(color, c(t.color.text_secondary));
    }
}

#[test]
fn small_switch_geometry() {
    let t = light();
    let list = paint(|| Switch::<()>::new("Sm", true).size(SwitchSize::Sm).into());
    let (track, _, _) = rect_filled(&list, c(t.color.accent)).unwrap();
    assert_eq!((track.width, track.height), (32.0, 20.0));
    let (knob, _, _) = rect_filled(&list, c(t.color.background_surface)).unwrap();
    assert_eq!(
        (knob.width, knob.x - track.x, knob.y - track.y),
        (16.0, 14.0, 2.0)
    );
}

#[test]
fn switch_ring_draws_on_the_track_and_a_toggles() {
    let t = light();
    let mut rt = runtime(|| Switch::new("Wi-Fi", false).on_change(|v| v).into());
    let list = draw(&mut rt, 400, 600);
    let (track, _, _) = rect_filled(&list, c(t.color.background_gray)).unwrap();
    let ring = rings(&list, c(t.color.accent));
    assert_eq!(ring.len(), 1);
    assert_eq!(ring[0].0, ring_rect(track));
    rt.input(Input::A, InputPhase::Pressed);
    let pressed = draw(&mut rt, 400, 600);
    let overlays = rects(&pressed)
        .iter()
        .filter(|r| r.2 == c(t.color.overlay_pressed))
        .count();
    assert_eq!(overlays, 2, "press paints the track and the thumb");
    rt.input(Input::A, InputPhase::Released);
    assert_eq!(rt.app().sent, vec![true]);
    let node = &a11y(&rt, Role::Switch)[0];
    assert_eq!(
        (node.label.as_deref(), node.checked),
        (Some("Wi-Fi"), Some(false))
    );
}

#[test]
fn disabled_and_loading_switches_refuse_changes() {
    for disabled in [true, false] {
        let mut rt = runtime(move || {
            Switch::new("S", true)
                .is_disabled(disabled)
                .is_loading(!disabled)
                .on_change(|v| v)
                .into()
        });
        let list = draw(&mut rt, 400, 600);
        tap(&mut rt, Input::A);
        assert!(rt.app().sent.is_empty());
        if disabled {
            assert!(list.commands.contains(&DrawCmd::PushOpacity(0.5)));
            assert!(a11y(&rt, Role::Switch)[0].disabled);
        } else {
            assert!(rt.wants_frames(), "loading spins");
        }
    }
}

#[test]
fn switch_markers_description_and_status() {
    let t = light();
    let list = paint(|| {
        VStack::<()>::new()
            .child(Switch::new("A", true).is_required(true).description("Why"))
            .child(
                Switch::new("B", false)
                    .is_optional(true)
                    .status(Status::error("Broken")),
            )
            .into()
    });
    assert_eq!(text(&list, "\u{b7} Required").2, c(t.color.text_secondary));
    assert_eq!(text(&list, "Why").1.size, t.font.supporting.size);
    assert!(texts(&list).iter().any(|t| t.1 == "\u{b7} Optional"));
    assert_eq!(text(&list, "Broken").2, c(t.color.on_error_muted));
    assert!(rect_filled(&list, c(t.color.error_muted)).is_some());
}

#[test]
fn hidden_switch_label_keeps_the_name() {
    let mut rt = runtime(|| {
        Switch::<()>::new("Secret", true)
            .is_label_hidden(true)
            .into()
    });
    let list = draw(&mut rt, 400, 600);
    assert!(texts(&list).is_empty());
    assert_eq!(a11y(&rt, Role::Switch)[0].label.as_deref(), Some("Secret"));
}

// ---- ListItem and Item ----

#[test]
fn list_item_padding_type_and_roles() {
    let t = light();
    let mut rt = runtime(|| {
        VStack::<()>::new()
            .child(ListItem::new("Static").description("Words"))
            .into()
    });
    let list = draw(&mut rt, 400, 600);
    let (label, style, color) = text(&list, "Static");
    assert_eq!((label.x, label.y), (t.space.s2, t.space.s2));
    assert_eq!(
        (style.size, color),
        (t.font.body.size, c(t.color.text_primary))
    );
    let (desc, style, color) = text(&list, "Words");
    assert_eq!(desc.y, label.y + label.height);
    assert_eq!(
        (style.size, color),
        (t.font.supporting.size, c(t.color.text_secondary))
    );
    let node = &a11y(&rt, Role::ListItem)[0];
    assert_eq!(
        (node.label.as_deref(), node.description.as_deref()),
        (Some("Static"), Some("Words"))
    );
    assert!(rt.focused().is_none(), "a static row is not a focus stop");
}

#[test]
fn pressable_selected_list_item() {
    let t = light();
    let mut rt = runtime(|| {
        ListItem::new("Inbox")
            .is_selected(true)
            .on_click("inbox")
            .into()
    });
    let list = draw(&mut rt, 400, 600);
    let (row, radii, _) = rect_filled(&list, c(t.color.accent_muted)).unwrap();
    assert_eq!(radii, Radii::all(t.radius.element));
    assert_eq!(rings(&list, c(t.color.accent))[0].0, ring_rect(row));
    assert_eq!(a11y(&rt, Role::Button)[0].label.as_deref(), Some("Inbox"));
    tap(&mut rt, Input::A);
    assert_eq!(rt.app().sent, vec!["inbox"]);
}

#[test]
fn disabled_list_item_dims_its_words_and_stays_inert() {
    let mut rt = runtime(|| ListItem::new("Off").is_disabled(true).on_click(1).into());
    let list = draw(&mut rt, 400, 600);
    assert!(list.commands.contains(&DrawCmd::PushOpacity(0.5)));
    tap(&mut rt, Input::A);
    assert!(rt.app().sent.is_empty());
}

#[test]
fn item_density_highlight_and_inline_layout() {
    let t = light();
    for (density, block, inline) in [
        (ItemDensity::Compact, t.space.s1, t.space.s2),
        (ItemDensity::Spacious, t.space.s3, t.space.s3),
    ] {
        let list = paint(move || Item::<()>::new("Row").density(density).into());
        let label = text(&list, "Row").0;
        assert_eq!((label.x, label.y), (inline, block), "{density:?}");
    }
    let list = paint(|| Item::<()>::new("Hi").is_highlighted(true).into());
    assert!(rect_filled(&list, c(t.color.overlay_hover)).is_some());
    let list = paint(|| {
        Item::<()>::new("Label")
            .description("beside")
            .layout(ItemLayout::Inline)
            .into()
    });
    let (label, desc) = (text(&list, "Label").0, text(&list, "beside").0);
    assert_eq!(desc.x, label.x + label.width + t.space.s1);
}

#[test]
fn item_slots_sit_beside_the_words() {
    let t = light();
    let list = paint(|| {
        Item::<()>::new("Ada")
            .start_content(Icon::new("check"))
            .end_content(Badge::new("New"))
            .into()
    });
    let icon = list.commands.iter().find_map(|c| match c {
        DrawCmd::Path { commands, .. } => match commands[0] {
            PathCmd::MoveTo(p) => Some(p),
            _ => None,
        },
        _ => None,
    });
    assert!(icon.is_some());
    let label = text(&list, "Ada").0;
    assert_eq!(label.x, t.space.s2 + IconSize::Md.points() + t.space.s2);
    let badge = text(&list, "New").0;
    assert!(badge.x > label.x + label.width);
}

// ---- Badge, Divider, Icon ----

#[test]
fn badges_are_pills_in_variant_colors() {
    let t = light();
    for (variant, bg, fg) in [
        (BadgeVariant::Neutral, t.color.neutral, t.color.text_primary),
        (BadgeVariant::Success, t.color.success, t.color.on_success),
        (
            BadgeVariant::Purple,
            t.color.background_purple,
            t.color.text_purple,
        ),
    ] {
        let list = paint(move || Badge::new("3").variant(variant).into_el());
        let (rect, radii, _) = rect_filled(&list, c(bg)).unwrap();
        assert_eq!(rect.height, t.space.s5);
        assert_eq!(radii, Radii::all(rect.height / 2.0));
        let (label, style, color) = text(&list, "3");
        assert_eq!(
            (label.x - rect.x, color, style.weight),
            (t.space.s2, c(fg), t.font.weight.medium)
        );
    }
}

trait El {
    fn into_el(self) -> Element<()>;
}
impl<T: Into<Element<()>>> El for T {
    fn into_el(self) -> Element<()> {
        self.into()
    }
}

#[test]
fn dividers_draw_border_lines() {
    let t = light();
    let list = paint(|| Divider::new().into_el());
    let (line, _, _) = rect_filled(&list, c(t.color.border)).unwrap();
    assert_eq!((line.width, line.height), (400.0, t.size.border_width));
    let strong = paint(|| Divider::new().variant(DividerVariant::Strong).into_el());
    assert!(rect_filled(&strong, c(t.color.border_emphasized)).is_some());
    let labeled = paint(|| Divider::new().label("or").into_el());
    let lines: Vec<_> = rects(&labeled)
        .into_iter()
        .filter(|r| r.2 == c(t.color.border))
        .collect();
    assert_eq!(lines.len(), 2);
    let or = text(&labeled, "or").0;
    assert_eq!(or.x, lines[0].0.x + lines[0].0.width + t.space.s3);
    assert_eq!(lines[0].0.y, (or.y + or.height / 2.0).floor());
}

#[test]
fn full_bleed_divider_reaches_through_card_padding() {
    let t = light();
    let list = paint(|| {
        Card::<()>::new()
            .child(Divider::new().is_full_bleed(true))
            .into()
    });
    let (card, _, _) = rect_filled(&list, c(t.color.background_card)).unwrap();
    let (line, _, _) = rect_filled(&list, c(t.color.border)).unwrap();
    assert_eq!(line.x, card.x + t.size.border_width);
    assert_eq!(line.width, card.width - 2.0 * t.size.border_width);
}

#[test]
fn icons_draw_every_built_in_name_at_their_sizes() {
    let t = light();
    for name in BUILT_IN_ICONS {
        let list = paint(move || Icon::new(name).color(IconColor::Primary).into_el());
        let path = list
            .commands
            .iter()
            .find(|c| matches!(c, DrawCmd::Path { .. }))
            .unwrap_or_else(|| panic!("{name}"));
        let DrawCmd::Path { fill, stroke, .. } = path else {
            unreachable!()
        };
        assert!(
            fill.map(|f| f == c(t.color.icon_primary)).unwrap_or(false)
                || stroke
                    .map(|s| s.0 == c(t.color.icon_primary))
                    .unwrap_or(false)
        );
    }
    for (size, pts) in [(IconSize::Xsm, 12.0), (IconSize::Lg, 24.0)] {
        let list = paint(move || Icon::new("close").size(size).into_el());
        let DrawCmd::Path {
            commands, stroke, ..
        } = list
            .commands
            .iter()
            .find(|c| matches!(c, DrawCmd::Path { .. }))
            .unwrap()
        else {
            unreachable!()
        };
        // The close icon runs from 6 to 18 in its 24 point view box.
        let PathCmd::MoveTo(p) = commands[0] else {
            panic!()
        };
        assert_eq!(p.x, 6.0 * pts / 24.0);
        assert_eq!(stroke.unwrap().1, 1.5 * pts / 24.0);
    }
}

#[test]
fn labeled_icons_are_images_and_others_are_decorative() {
    let mut rt = runtime(|| {
        HStack::<()>::new()
            .child(Icon::new("info").label("Info"))
            .child(Icon::new("check"))
            .into()
    });
    draw(&mut rt, 400, 600);
    let images = a11y(&rt, Role::Image);
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].label.as_deref(), Some("Info"));
}

// ---- ProgressBar ----

#[test]
fn progress_bar_fill_width_variant_and_value() {
    let t = light();
    let mut rt = runtime(|| {
        ProgressBar::new("Upload")
            .value(25.0)
            .has_value_label(true)
            .variant(ProgressBarVariant::Success)
            .into_el()
    });
    let list = draw(&mut rt, 400, 600);
    let (track, _, _) = rect_filled(&list, c(t.color.background_muted)).unwrap();
    assert_eq!((track.width, track.height), (400.0, 8.0));
    let (fill, _, _) = rect_filled(&list, c(t.color.success)).unwrap();
    assert_eq!(fill.width, 100.0);
    assert!(texts(&list).iter().any(|t| t.1 == "25%"));
    let node = &a11y(&rt, Role::ProgressBar)[0];
    assert_eq!(
        (node.label.as_deref(), node.value.as_deref()),
        (Some("Upload"), Some("25%"))
    );
}

#[test]
fn indeterminate_progress_animates_and_disabled_dims() {
    let t = light();
    let mut rt = runtime(|| ProgressBar::new("Wait").is_indeterminate(true).into_el());
    draw(&mut rt, 400, 600);
    assert!(rt.wants_frames());
    let list = paint(|| {
        ProgressBar::new("Off")
            .value(50.0)
            .is_disabled(true)
            .into_el()
    });
    assert!(rect_filled(&list, c(t.color.text_disabled)).is_some());
    assert_eq!(text(&list, "Off").2, c(t.color.text_disabled));
}

// ---- Avatar and Thumbnail ----

#[test]
fn avatar_sizes_shapes_and_initials() {
    let t = light();
    for (size, side) in [
        (AvatarSize::Xsm, 20.0),
        (AvatarSize::Md, 36.0),
        (AvatarSize::Xl, 128.0),
    ] {
        let list = paint(move || {
            Avatar::<()>::new()
                .name("Grace Brewster Hopper")
                .size(size)
                .into()
        });
        let (plate, radii, _) = rect_filled(&list, c(t.color.neutral)).unwrap();
        assert_eq!((plate.width, radii), (side, Radii::all(side / 2.0)));
        let (_, style, color) = text(&list, "GH");
        assert_eq!((style.size, color), (side * 0.4, c(t.color.text_secondary)));
    }
    let rounded = paint(|| {
        Avatar::<()>::new()
            .name("A")
            .shape(AvatarShape::Rounded)
            .into()
    });
    assert_eq!(
        rect_filled(&rounded, c(t.color.neutral)).unwrap().1,
        Radii::all(t.radius.element)
    );
    let anon = paint(|| Avatar::<()>::new().into());
    assert!(texts(&anon).is_empty());
    assert!(
        has(&anon, |c| matches!(c, DrawCmd::Path { .. })),
        "person symbol"
    );
}

#[test]
fn avatar_status_dot_sits_on_the_edge_and_names_the_status() {
    let t = light();
    let mut rt = runtime(|| {
        Avatar::<()>::new()
            .name("Ada")
            .size(AvatarSize::Lg)
            .status(AvatarStatusDot::new(AvatarStatusDotVariant::Success).label("Online"))
            .into()
    });
    let list = draw(&mut rt, 400, 600);
    let (plate, _, _) = rect_filled(&list, c(t.color.neutral)).unwrap();
    let (dot, _, stroke) = rect_filled(&list, c(t.color.success)).unwrap();
    assert_eq!(dot.width, 20.0, "the medium tier for a 48 point avatar");
    assert_eq!(stroke, Some((c(t.color.background_surface), 2.0)));
    // Its center is on the circle at the bottom trailing 45 degree point.
    let center = plate.width / 2.0 * (1.0 + std::f32::consts::FRAC_1_SQRT_2);
    assert!((dot.x + dot.width / 2.0 - (plate.x + center)).abs() < 1.0);
    assert_eq!(
        a11y(&rt, Role::Image)[0].label.as_deref(),
        Some("Ada, Online")
    );
}

#[test]
fn thumbnail_tile_remove_and_open_are_separate_stops() {
    let t = light();
    let mut rt = runtime(|| {
        Thumbnail::new()
            .label("a.jpg")
            .on_click("open")
            .on_remove("remove")
            .into()
    });
    let list = draw(&mut rt, 400, 600);
    let (tile, _, _) = rect_filled(&list, c(t.color.neutral)).unwrap();
    assert_eq!((tile.width, tile.height), (64.0, 64.0));
    let (remove, _, _) = rect_filled(&list, c(t.color.overlay)).unwrap();
    assert_eq!(
        (remove.width, remove.x + remove.width),
        (20.0, tile.x + tile.width - t.space.s1)
    );
    let buttons: Vec<String> = a11y(&rt, Role::Button)
        .iter()
        .filter_map(|n| n.label.clone())
        .collect();
    assert_eq!(buttons, vec!["Open a.jpg", "Remove a.jpg"]);
    tap(&mut rt, Input::A);
    tap(&mut rt, Input::Next);
    tap(&mut rt, Input::A);
    assert_eq!(rt.app().sent, vec!["open", "remove"]);
}

// ---- TextInput and Selector ----

#[test]
fn text_input_field_border_and_focus_style() {
    let t = light();
    let mut rt = runtime(|| TextInput::<()>::new("Name", "Ada").into());
    let list = draw(&mut rt, 400, 600);
    let (field, radii, stroke) = rect_filled(&list, c(t.color.background_surface)).unwrap();
    assert_eq!(
        (field.height, radii),
        (t.size.element_md, Radii::all(t.radius.element))
    );
    assert_eq!(
        stroke,
        Some((c(t.color.border_emphasized), t.size.border_width))
    );
    // Focus repaints the border in accent with an accent-muted ring inside.
    assert_eq!(rings(&list, c(t.color.accent))[0].0, field);
    assert_eq!(
        rings(&list, c(t.color.accent_muted))[0],
        (field.inset(1.0), Radii::all(t.radius.element - 1.0), 2.0)
    );
    assert_eq!(text(&list, "Ada").0.x, field.x + t.space.s3);
    let node = &a11y(&rt, Role::TextInput)[0];
    assert_eq!(
        (node.label.as_deref(), node.value.as_deref(), node.focused),
        (Some("Name"), Some("Ada"), true)
    );
}

#[test]
fn text_input_placeholder_password_status_and_clear() {
    let t = light();
    let list = paint(|| {
        VStack::<()>::new()
            .child(TextInput::new("E", "").placeholder("you@example.com"))
            .child(TextInput::new("P", "abc").r#type(TextInputType::Password))
            .into()
    });
    assert_eq!(text(&list, "you@example.com").2, c(t.color.text_secondary));
    assert!(texts(&list)
        .iter()
        .any(|t| t.1 == "\u{2022}\u{2022}\u{2022}"));
    let list = paint(|| {
        TextInput::<()>::new("U", "x")
            .status(Status::error("Taken"))
            .into()
    });
    assert!(rects(&list)
        .iter()
        .any(|r| r.3 == Some((c(t.color.border_error), 1.0))));
    let mut rt = runtime(|| {
        TextInput::new("Q", "query")
            .has_clear(true)
            .on_change(|s: String| s)
            .into()
    });
    draw(&mut rt, 400, 600);
    tap(&mut rt, Input::Next);
    tap(&mut rt, Input::A);
    assert_eq!(rt.app().sent, vec![String::new()]);
}

#[test]
fn selector_opens_a_menu_traps_focus_and_picks() {
    let t = light();
    let mut rt = runtime(|| {
        Selector::new("Fruit", ["apple", "banana", "cherry"])
            .value("apple")
            .on_change(|v: String| v)
            .into()
    });
    let list = draw(&mut rt, 400, 600);
    assert_eq!(text(&list, "apple").2, c(t.color.text_primary));
    let trigger = rt.focused();
    tap(&mut rt, Input::A);
    let open = draw(&mut rt, 400, 600);
    assert!(rect_filled(&open, c(t.color.background_popover)).is_some());
    let items = a11y(&rt, Role::ListBoxOption);
    assert_eq!(items.len(), 3);
    assert!(
        items[0].focused && items[0].selected == Some(true),
        "the chosen option takes focus"
    );
    assert_eq!(items[1].selected, Some(false));
    assert_eq!(a11y(&rt, Role::ListBox).len(), 1);
    assert_eq!(a11y(&rt, Role::ComboBox)[0].value.as_deref(), Some("apple"));
    // Focus stays in the menu.
    tap(&mut rt, Input::Up);
    assert!(a11y(&rt, Role::ListBoxOption)[0].focused);
    tap(&mut rt, Input::Down);
    tap(&mut rt, Input::A);
    assert_eq!(rt.app().sent, vec!["banana".to_string()]);
    draw(&mut rt, 400, 600);
    assert!(
        a11y(&rt, Role::ListBoxOption).is_empty(),
        "picking closes the menu"
    );
    assert_eq!(rt.focused(), trigger, "focus returns to the trigger");
}

#[test]
fn b_closes_an_open_selector_without_choosing() {
    let mut rt = runtime(|| {
        Selector::new("Fruit", ["apple", "banana"])
            .on_change(|v: String| v)
            .into()
    });
    draw(&mut rt, 400, 600);
    tap(&mut rt, Input::A);
    draw(&mut rt, 400, 600);
    assert_eq!(a11y(&rt, Role::ListBoxOption).len(), 2);
    tap(&mut rt, Input::B);
    draw(&mut rt, 400, 600);
    assert!(a11y(&rt, Role::ListBoxOption).is_empty());
    assert!(rt.app().sent.is_empty());
}

// ---- Links, separators, and editing ----

#[test]
fn hrefs_open_after_the_press_handler() {
    let mut rt = runtime(|| {
        VStack::new()
            .child(
                Button::new("Docs")
                    .href("https://example.com/docs")
                    .on_click("docs"),
            )
            .child(ListItem::new("Row link").href(" https://example.com/row "))
            .child(
                Button::new("Bad")
                    .href("javascript:alert(1)")
                    .on_click("bad"),
            )
            .child(
                Button::new("Off")
                    .href("https://example.com")
                    .is_disabled(true),
            )
            .into()
    });
    draw(&mut rt, 400, 600);
    assert_eq!(
        tap(&mut rt, Input::A),
        Command::OpenUrl("https://example.com/docs".into())
    );
    assert_eq!(rt.app().sent, vec!["docs"]);
    tap(&mut rt, Input::Down);
    assert_eq!(
        tap(&mut rt, Input::A),
        Command::OpenUrl("https://example.com/row".into()),
        "trimmed"
    );
    tap(&mut rt, Input::Down);
    assert_eq!(
        tap(&mut rt, Input::A),
        Command::None,
        "script links are refused"
    );
    tap(&mut rt, Input::Down);
    assert_eq!(
        tap(&mut rt, Input::A),
        Command::None,
        "disabled links never open"
    );
}

#[test]
fn dividers_are_separators() {
    let mut rt = runtime(|| {
        VStack::<()>::new()
            .child(Divider::new())
            .child(Divider::new().label("or"))
            .into()
    });
    draw(&mut rt, 400, 600);
    let seps = a11y(&rt, Role::Separator);
    assert_eq!(seps.len(), 2);
    assert_eq!(
        (seps[0].label.as_deref(), seps[1].label.as_deref()),
        (None, Some("or"))
    );
}

/// A text field app that stores what it is told.
struct Field {
    value: String,
    masked: bool,
    read_only: bool,
}

impl App for Field {
    type Msg = String;
    fn view(&self) -> Element<String> {
        TextInput::new("Name", self.value.clone())
            .r#type(if self.masked {
                TextInputType::Password
            } else {
                TextInputType::Text
            })
            .is_read_only(self.read_only)
            .on_change(|v| v)
            .into()
    }
    fn update(&mut self, v: String) -> Command {
        self.value = v;
        Command::None
    }
}

fn field(value: &str) -> Runtime<Field> {
    let mut rt = Runtime::new(Field {
        value: value.into(),
        masked: false,
        read_only: false,
    });
    draw(&mut rt, 300, 200);
    rt
}

#[test]
fn typing_inserts_at_the_caret_and_control_characters_edit() {
    let mut rt = field("ac");
    rt.text_input("d");
    assert_eq!(rt.app().value, "acd", "the caret starts at the end");
    draw(&mut rt, 300, 200);
    tap(&mut rt, Input::Left);
    tap(&mut rt, Input::Left);
    rt.text_input("b");
    assert_eq!(rt.app().value, "abcd");
    rt.text_input("\u{8}");
    assert_eq!(rt.app().value, "acd", "backspace removes before the caret");
    rt.text_input("\u{7f}");
    assert_eq!(rt.app().value, "ad", "delete removes after the caret");
    rt.text_input("\n\t");
    assert_eq!(rt.app().value, "ad", "other control characters are ignored");
    rt.text_input("héllo ");
    assert_eq!(rt.app().value, "ahéllo d");
}

#[test]
fn arrows_move_the_caret_then_focus_at_the_ends() {
    let mut rt = Runtime::new(View {
        view: Box::new(|| {
            HStack::new()
                .gap(Space::S2)
                .child(TextInput::new("A", "xy").on_change(|v| v))
                .child(Button::new("Next").on_click(String::new()))
                .into()
        }),
        sent: Vec::new(),
        choice: ThemeChoice::default(),
    });
    draw(&mut rt, 400, 200);
    let field = rt.focused();
    tap(&mut rt, Input::Right);
    assert_ne!(rt.focused(), field, "at the end, Right moves focus");
    tap(&mut rt, Input::Left);
    assert_eq!(rt.focused(), field);
    tap(&mut rt, Input::Left);
    tap(&mut rt, Input::Left);
    assert_eq!(rt.focused(), field, "Left moves the caret while it can");
}

#[test]
fn caret_draws_in_the_text_color_and_follows_typing() {
    let t = light();
    let mut rt = field("ab");
    let list = draw(&mut rt, 300, 200);
    let value = text(&list, "ab").0;
    let caret = rects(&list)
        .into_iter()
        .find(|r| r.2 == c(t.color.text_primary))
        .expect("caret");
    assert_eq!(
        (caret.0.x, caret.0.width, caret.0.height),
        (value.x + value.width, 1.0, 20.0)
    );
    rt.text_input("c");
    let list = draw(&mut rt, 300, 200);
    let moved = rects(&list)
        .into_iter()
        .find(|r| r.2 == c(t.color.text_primary))
        .unwrap();
    assert_eq!(moved.0.x, caret.0.x + 7.0);
}

#[test]
fn long_values_scroll_to_the_caret_inside_the_field() {
    let t = light();
    let mut rt = field(&"w".repeat(80));
    let list = draw(&mut rt, 300, 200);
    let (field_rect, _, _) = rect_filled(&list, c(t.color.background_surface)).unwrap();
    let caret = rects(&list)
        .into_iter()
        .find(|r| r.2 == c(t.color.text_primary))
        .unwrap()
        .0;
    assert!(
        caret.x <= field_rect.x + field_rect.width - t.space.s3,
        "caret stays inside"
    );
    let value = &texts(&list)[1];
    assert!(value.0.x < field_rect.x, "the value scrolls left");
    assert!(
        !value.1.ends_with('\u{2026}'),
        "editable values clip, not truncate"
    );
}

#[test]
fn password_and_read_only_fields() {
    let mut rt = Runtime::new(Field {
        value: "pw".into(),
        masked: true,
        read_only: false,
    });
    draw(&mut rt, 300, 200);
    rt.text_input("x");
    assert_eq!(rt.app().value, "pwx");
    let list = draw(&mut rt, 300, 200);
    assert!(texts(&list)
        .iter()
        .any(|t| t.1 == "\u{2022}\u{2022}\u{2022}"));
    let mut ro = Runtime::new(Field {
        value: "fixed".into(),
        masked: false,
        read_only: true,
    });
    draw(&mut ro, 300, 200);
    assert_eq!(ro.text_input("x"), Command::None);
    assert_eq!(ro.app().value, "fixed");
}

#[test]
fn text_reaches_only_a_focused_field() {
    let mut rt = runtime(|| Button::new("Not a field").on_click(String::new()).into());
    draw(&mut rt, 300, 200);
    assert_eq!(rt.text_input("x"), Command::None);
    assert!(rt.app().sent.is_empty());
}

#[test]
fn editing_keys_from_a_keyboard() {
    let mut rt = field("bc");
    tap(&mut rt, Input::Home);
    rt.text_input("a");
    assert_eq!(rt.app().value, "abc");
    tap(&mut rt, Input::Delete);
    assert_eq!(rt.app().value, "ac");
    tap(&mut rt, Input::End);
    tap(&mut rt, Input::Backspace);
    assert_eq!(rt.app().value, "a");
    tap(&mut rt, Input::Backspace);
    tap(&mut rt, Input::Backspace);
    assert_eq!(rt.app().value, "", "backspace at the start does nothing");
}

#[test]
fn composing_text_shows_inline_underlined_and_commits_through_text_input() {
    let t = light();
    let mut rt = field("ab");
    tap(&mut rt, Input::Left);
    rt.text_compose("xy", 1);
    let list = draw(&mut rt, 300, 200);
    let shown = text(&list, "axyb").0;
    let primary = c(t.color.text_primary);
    let underline = rects(&list)
        .into_iter()
        .find(|r| r.2 == primary && r.0.height == 1.0)
        .expect("underline");
    assert_eq!((underline.0.x, underline.0.width), (shown.x + 7.0, 14.0));
    let caret = rects(&list)
        .into_iter()
        .find(|r| r.2 == primary && r.0.width == 1.0 && r.0.height > 1.0)
        .unwrap();
    assert_eq!(caret.0.x, shown.x + 14.0, "caret at the composing cursor");
    assert_eq!(rt.app().value, "ab", "composing does not change the value");
    rt.text_compose("", 0);
    rt.text_input("xy");
    assert_eq!(rt.app().value, "axyb");
}

#[test]
fn fields_expose_their_caret_and_secrecy() {
    let mut rt = field("héllo");
    tap(&mut rt, Input::Left);
    draw(&mut rt, 300, 200);
    let node = &a11y_nodes(&rt.accessibility().unwrap())
        .into_iter()
        .find(|n| n.role == Role::TextInput)
        .unwrap();
    // Four characters before the caret, "é" taking two bytes.
    assert_eq!(node.text_selection, Some((5, 5)));
    assert!(!node.secure);
    let mut pw = Runtime::new(Field {
        value: "pw".into(),
        masked: true,
        read_only: false,
    });
    draw(&mut pw, 300, 200);
    let node = a11y_nodes(&pw.accessibility().unwrap())
        .into_iter()
        .find(|n| n.role == Role::TextInput)
        .unwrap();
    assert!(node.secure);
    assert_eq!(
        node.text_selection,
        Some((6, 6)),
        "offsets into the masked value"
    );
}

// ---- Tooltips ----

#[test]
fn a_focused_button_shows_its_tooltip_above_it() {
    let t = light();
    let mut rt = runtime(|| {
        VStack::new()
            .padding(Space::S10)
            .padding_block_start(Space::S10)
            .gap(Space::S10)
            .h_align(StackAlign::Center)
            .child(Text::new("Room above"))
            .child(Button::new("Save").tooltip("Saves a draft").on_click(()))
            .into()
    });
    let list = draw(&mut rt, 400, 300);
    let button = a11y(&rt, Role::Button)[0].bounds;
    let (bubble, radii, _) = rect_filled(&list, c(t.color.text_primary)).expect("tooltip surface");
    assert_eq!(radii, Radii::all(t.radius.container));
    assert_eq!(
        bubble.y + bubble.height,
        button.y - t.space.s1,
        "space.s1 above the trigger"
    );
    assert!(
        (bubble.x + bubble.width / 2.0 - (button.x + button.width / 2.0)).abs() <= 0.5,
        "centered on the trigger"
    );
    let (words, style, color) = text(&list, "Saves a draft");
    assert_eq!(
        (color, style.size),
        (c(t.color.background_surface), t.font.body.size)
    );
    assert_eq!(
        (words.x - bubble.x, words.y - bubble.y),
        (t.space.s2, t.space.s1)
    );
    // It is the last thing painted, above the focus ring.
    assert!(matches!(list.commands.last(), Some(DrawCmd::Text { .. })));
    assert_eq!(
        a11y(&rt, Role::Button)[0].description.as_deref(),
        Some("Saves a draft")
    );
}

#[test]
fn tooltips_flip_below_and_slide_inside_the_window() {
    let t = light();
    let mut rt = runtime(|| {
        HStack::new()
            .h_align(StackJustify::End)
            .child(
                Button::new("Edge")
                    .tooltip("A tooltip whose words are long enough to wrap onto a second line")
                    .on_click(()),
            )
            .into()
    });
    let list = draw(&mut rt, 400, 300);
    let button = a11y(&rt, Role::Button)[0].bounds;
    let (bubble, _, _) = rect_filled(&list, c(t.color.text_primary)).unwrap();
    assert_eq!(
        bubble.y,
        button.y + button.height + t.space.s1,
        "no room above, so below"
    );
    assert_eq!(
        bubble.x + bubble.width,
        400.0 - t.space.s4,
        "slid inside the right edge"
    );
    assert!(bubble.width <= 300.0);
    assert!(bubble.height > 2.0 * 20.0, "wrapped");
}

#[test]
fn b_dismisses_a_tooltip_until_focus_moves() {
    let t = light();
    let mut rt = runtime(|| {
        VStack::new()
            .padding(Space::S10)
            .gap(Space::S2)
            .child(Button::new("One").tooltip("First").on_click(1))
            .child(Button::new("Two").tooltip("Second").on_click(2))
            .into()
    });
    draw(&mut rt, 400, 300);
    tap(&mut rt, Input::B);
    let list = draw(&mut rt, 400, 300);
    assert!(
        rect_filled(&list, c(t.color.text_primary)).is_none(),
        "dismissed"
    );
    tap(&mut rt, Input::Down);
    let list = draw(&mut rt, 400, 300);
    assert_eq!(
        text(&list, "Second").2,
        c(t.color.background_surface),
        "a new focus shows its own"
    );
    tap(&mut rt, Input::Up);
    let list = draw(&mut rt, 400, 300);
    assert!(
        texts(&list).iter().any(|t| t.1 == "First"),
        "and coming back shows it again"
    );
}

#[test]
fn disabled_reasons_and_label_tips_show_for_fields() {
    let mut rt = runtime(|| {
        VStack::new()
            .padding(Space::S10)
            .gap(Space::S4)
            .child(
                Switch::new("Sync", false)
                    .is_disabled(true)
                    .disabled_message("Sign in to sync")
                    .label_tooltip("Keeps devices alike")
                    .on_change(|_| ()),
            )
            .child(
                Switch::new("Wi-Fi", true)
                    .label_tooltip("Uses your saved networks")
                    .on_change(|_| ()),
            )
            .child(
                Selector::new("Fruit", ["apple"])
                    .is_disabled(true)
                    .disabled_message("Out of season"),
            )
            .into()
    });
    let list = draw(&mut rt, 400, 400);
    assert!(
        texts(&list).iter().any(|t| t.1 == "Sign in to sync"),
        "a disabled control shows its reason"
    );
    tap(&mut rt, Input::Down);
    let list = draw(&mut rt, 400, 400);
    assert!(texts(&list)
        .iter()
        .any(|t| t.1 == "Uses your saved networks"));
    let switches = a11y(&rt, Role::Switch);
    assert_eq!(switches[0].description.as_deref(), Some("Sign in to sync"));
    assert_eq!(
        a11y(&rt, Role::ComboBox)[0].description.as_deref(),
        Some("Out of season")
    );
}

#[test]
fn a_tooltip_that_repeats_the_name_is_not_read_twice() {
    let mut rt = runtime(|| Button::new("Save").tooltip("Save").on_click(()).into());
    draw(&mut rt, 300, 200);
    assert_eq!(a11y(&rt, Role::Button)[0].description, None);
}

// ---- Press scale and thumb travel ----

fn transforms(list: &DisplayList) -> Vec<(Point, f32, Point)> {
    list.commands
        .iter()
        .filter_map(|c| match c {
            DrawCmd::PushTransform {
                origin,
                scale,
                translate,
            } => Some((*origin, *scale, *translate)),
            _ => None,
        })
        .collect()
}

fn step<A: App>(rt: &mut Runtime<A>, dt: f32) -> DisplayList {
    rt.frame(
        &FrameInfo {
            width: 400,
            height: 300,
            scale: 1.0,
            dt,
        },
        &measure(),
    )
}

#[test]
fn a_pressed_button_scales_to_0_98_and_back() {
    let mut rt = runtime(|| Button::new("Ok").on_click(()).into());
    step(&mut rt, 0.0);
    rt.input(Input::A, InputPhase::Pressed);
    let start = step(&mut rt, 0.0);
    let button = a11y(&rt, Role::Button)[0].bounds;
    let mid = step(&mut rt, 0.06);
    let s = transforms(&mid);
    assert_eq!(s.len(), 2, "the button and its ring");
    assert!(s[0].1 < 1.0 && s[0].1 > 0.98, "{s:?}");
    assert_eq!(
        s[0].0,
        Point {
            x: button.x + button.width / 2.0,
            y: button.y + button.height / 2.0
        }
    );
    assert!(transforms(&start).iter().all(|t| t.1 == 1.0));
    let held = step(&mut rt, 0.3);
    assert!((transforms(&held)[0].1 - 0.98).abs() < 1e-4);
    assert!(!rt.wants_frames() || rt.app().sent.is_empty());
    rt.input(Input::A, InputPhase::Released);
    step(&mut rt, 0.06);
    let back = step(&mut rt, 0.3);
    assert!(transforms(&back).is_empty(), "back at rest, no transform");
    assert!(!rt.wants_frames());
}

#[test]
fn no_press_scale_under_reduced_motion_or_when_disabled() {
    for (reduced, disabled) in [(true, false), (false, true)] {
        let mut rt = runtime(move || Button::new("Ok").is_disabled(disabled).on_click(()).into());
        rt.set_preferences(&Preferences {
            reduced_motion: reduced,
            ..Preferences::default()
        });
        step(&mut rt, 0.0);
        rt.input(Input::A, InputPhase::Pressed);
        step(&mut rt, 0.0);
        assert!(transforms(&step(&mut rt, 0.1)).is_empty());
    }
}

struct Toggle {
    on: bool,
}

impl App for Toggle {
    type Msg = bool;
    fn view(&self) -> Element<bool> {
        Switch::new("Wi-Fi", self.on).on_change(|v| v).into()
    }
    fn update(&mut self, msg: bool) -> Command {
        self.on = msg;
        Command::None
    }
}

#[test]
fn the_switch_thumb_slides_to_its_new_side() {
    for reduced in [false, true] {
        let mut rt = Runtime::new(Toggle { on: false });
        rt.set_preferences(&Preferences {
            reduced_motion: reduced,
            ..Preferences::default()
        });
        step(&mut rt, 0.0);
        tap(&mut rt, Input::A);
        let first = step(&mut rt, 0.0);
        let mid = step(&mut rt, 0.08);
        if reduced {
            assert!(
                transforms(&first).is_empty() && transforms(&mid).is_empty(),
                "lands at once"
            );
            continue;
        }
        let at_start = transforms(&first);
        assert_eq!(at_start.len(), 1);
        // md: the thumb moves from x 4 to x 18 and grows from 16 to 20.
        assert_eq!(at_start[0].2.x, -14.0, "drawn where it was");
        assert!((at_start[0].1 - 0.8).abs() < 1e-4);
        let m = transforms(&mid)[0];
        assert!(m.2.x > -14.0 && m.2.x < 0.0, "{m:?}");
        assert!(rt.wants_frames());
        assert!(transforms(&step(&mut rt, 0.3)).is_empty());
        assert!(!rt.wants_frames());
    }
}

#[test]
fn selector_clear_is_a_focus_stop_that_sends_an_empty_value() {
    let mut rt = runtime(|| {
        Selector::new("Fruit", ["apple", "pear"])
            .value("apple")
            .has_clear(true)
            .on_change(|v: String| v)
            .into()
    });
    draw(&mut rt, 400, 200);
    let clear = a11y(&rt, Role::Button);
    assert_eq!(clear[0].label.as_deref(), Some("Clear Fruit"));
    tap(&mut rt, Input::Right);
    draw(&mut rt, 400, 200);
    assert!(
        a11y(&rt, Role::Button)[0].focused,
        "Right from the trigger reaches it"
    );
    tap(&mut rt, Input::A);
    assert_eq!(rt.app().sent, vec![String::new()]);
    for (read_only, value) in [(true, Some("apple")), (false, None)] {
        let mut rt = runtime(move || {
            let s = Selector::new("Fruit", ["apple"])
                .has_clear(true)
                .is_read_only(read_only)
                .on_change(|v: String| v);
            match value {
                Some(v) => s.value(v),
                None => s,
            }
            .into()
        });
        draw(&mut rt, 400, 200);
        assert!(
            a11y(&rt, Role::Button).is_empty(),
            "no clear when read only or empty"
        );
    }
}
