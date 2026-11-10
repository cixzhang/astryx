//! Wave 1: primitives build, name themselves, and act.

use astryx_core::contract::{Input, Role};
use astryx_spike_ui::testkit::{all, find, Rig};
use astryx_spike_ui::*;

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Clicked,
    More,
}

#[test]
fn spinner_is_named_loading_and_keeps_frames_coming() {
    let rig = Rig::<Msg>::new(|| Spinner::new().size(SpinnerSize::Xl).into());
    let tree = rig.tree();
    assert!(find(&tree, Role::Status, "Loading").is_some());
    assert!(astryx_core::contract::Surface::wants_frames(&rig.rt));
}

#[test]
fn spinner_label_names_it_and_shows_below() {
    let rig = Rig::<Msg>::new(|| Spinner::new().label("Fetching").into());
    let tree = rig.tree();
    let ring = find(&tree, Role::Status, "Fetching").expect("named by its label");
    assert!(ring.bounds.height > 18.0, "label sits below the ring");
}

#[test]
fn spinner_box_is_diameter_plus_two_strokes() {
    for (size, side) in [
        (SpinnerSize::Sm, 14.0),
        (SpinnerSize::Md, 20.0),
        (SpinnerSize::Lg, 24.0),
        (SpinnerSize::Xl, 36.0),
    ] {
        let rig = Rig::<Msg>::new(move || HStack::new().child(Spinner::new().size(size)).into());
        let tree = rig.tree();
        let ring = find(&tree, Role::Status, "Loading").unwrap();
        assert_eq!(ring.bounds.width, side, "{size:?}");
    }
}

#[test]
fn link_activates_and_announces_new_tab() {
    let mut rig = Rig::new(|| {
        Link::new("Docs")
            .href("https://example.com")
            .is_external_link(true)
            .on_click(Msg::Clicked)
            .into()
    });
    let tree = rig.tree();
    let link = find(&tree, Role::Link, "Docs (opens in new tab)").expect("link node");
    assert!(link.focusable);
    rig.press(Input::Next);
    rig.press(Input::A);
    assert_eq!(rig.messages(), vec![Msg::Clicked]);
}

#[test]
fn disabled_link_stays_focusable_and_ignores_presses() {
    let mut rig = Rig::new(|| {
        Link::new("Docs")
            .is_disabled(true)
            .on_click(Msg::Clicked)
            .into()
    });
    let tree = rig.tree();
    let link = find(&tree, Role::Button, "Docs").unwrap();
    assert!(link.disabled && link.focusable);
    rig.press(Input::Next);
    rig.press(Input::A);
    assert!(rig.messages().is_empty());
}

#[test]
fn visually_hidden_is_read_but_takes_no_space() {
    let rig = Rig::<Msg>::new(|| {
        VStack::new()
            .child(VisuallyHidden::new("Only for screen readers"))
            .into()
    });
    let tree = rig.tree();
    let n = find(&tree, Role::Text, "Only for screen readers").unwrap();
    assert!(n.bounds.width <= 1.0 && n.bounds.height <= 1.0);
}

#[test]
fn avatar_group_overlaps_by_a_quarter_and_counts_overflow() {
    let mut rig = Rig::new(|| {
        HStack::new()
            .child(
                AvatarGroup::new([
                    Avatar::new().name("Ada Lovelace"),
                    Avatar::new().name("Grace Hopper"),
                    Avatar::new().name("Alan Turing"),
                ])
                .size(AvatarSize::Lg)
                .overflow(AvatarGroupOverflow::new(4).on_click(Msg::More)),
            )
            .into()
    });
    let tree = rig.tree();
    let group = find(&tree, Role::Group, "Avatars").unwrap_or_else(|| panic!("{tree:#?}"));
    assert_eq!(all(group, Role::Image).len(), 3);
    // Three 52 point avatars (48 + a 2 point ring each side) step by 40,
    // so the overflow starts at 3 x 40 = 120.
    let more = find(&tree, Role::Button, "4 more").unwrap();
    assert_eq!(more.bounds.x, 120.0);
    assert!(find(&tree, Role::Button, "4 more").is_some());
    rig.press(Input::Next);
    rig.press(Input::A);
    assert_eq!(rig.messages(), vec![Msg::More]);
}

#[test]
fn icon_draws_all_twenty_colors_and_unknown_names_draw_nothing() {
    assert_eq!(IconColor::ALL.len(), 20);
    let rig = Rig::<Msg>::new(|| {
        HStack::new()
            .children(IconColor::ALL.map(|c| Icon::new("check").color(c).label("ok")))
            .child(Icon::new("not-an-icon").label("missing"))
            .into()
    });
    let tree = rig.tree();
    assert_eq!(all(&tree, Role::Image).len(), 20);
}

#[test]
fn indicators_size_like_astryx() {
    let rig = Rig::<Msg>::new(|| {
        HStack::new()
            .v_align(StackAxisAlign::Start)
            .child(CheckboxIndicator::new(CheckboxState::Checked).size(IndicatorSize::Sm))
            .child(RadioIndicator::new(true))
            .child(CheckIndicator::new(true))
            .child(Text::new("end"))
            .into()
    });
    let tree = rig.tree();
    let end = find(&tree, Role::Text, "end").unwrap();
    // 20 + 24 + 16 points of marks before the text.
    assert_eq!(end.bounds.x, 60.0);
}
