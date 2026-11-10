//! Wave 2: layout primitives place children on the spacing scale.

use astryx_core::contract::Role;
use astryx_core::{Direction, Space};
use astryx_spike_ui::testkit::{find, Rig};
use astryx_spike_ui::*;

type Msg = ();

fn x_of(rig: &Rig<Msg>, label: &str) -> (f32, f32) {
    let tree = rig.tree();
    let n = find(&tree, Role::Text, label).unwrap_or_else(|| panic!("{label}"));
    (n.bounds.x, n.bounds.y)
}

#[test]
fn hstack_gap_and_padding_use_space_tokens() {
    let rig = Rig::<Msg>::new(|| {
        HStack::new()
            .gap(Space::S4)
            .padding(Space::S2)
            .child(Text::new("aa"))
            .child(Text::new("bb"))
            .into()
    });
    let (ax, ay) = x_of(&rig, "aa");
    let (bx, _) = x_of(&rig, "bb");
    assert_eq!((ax, ay), (8.0, 8.0));
    assert!(bx > ax + 16.0);
}

#[test]
fn padding_inline_start_flips_in_rtl() {
    let view = || {
        Element::from(
            HStack::new()
                .padding_inline_start(Space::S10)
                .child(Text::new("x")),
        )
    };
    let ltr = Rig::<Msg>::new(view);
    let rtl = Rig::<Msg>::new(move || view().direction(Direction::Rtl));
    assert_eq!(x_of(&ltr, "x").0, 40.0);
    assert!(
        x_of(&rtl, "x").0 > 400.0,
        "start padding is on the right in RTL"
    );
}

#[test]
fn vstack_h_align_center_centers_children() {
    let rig = Rig::<Msg>::new(|| {
        VStack::new()
            .width(SizeValue::Points(200.0))
            .h_align(StackAxisAlign::Center)
            .child(Text::new("mid"))
            .into()
    });
    let tree = rig.tree();
    let n = find(&tree, Role::Text, "mid").unwrap();
    let center = n.bounds.x + n.bounds.width / 2.0;
    assert!((center - 100.0).abs() <= 1.0, "{:?}", n.bounds);
}

#[test]
fn grid_count_makes_equal_columns() {
    let rig = Rig::<Msg>::new(|| {
        Grid::new(GridColumns::Count(3))
            .width(SizeValue::Points(300.0))
            .gap(Space::S0)
            .children(["a", "b", "c", "d"].map(Text::new))
            .into()
    });
    assert_eq!(x_of(&rig, "a").0, 0.0);
    assert_eq!(x_of(&rig, "b").0, 100.0);
    assert_eq!(x_of(&rig, "c").0, 200.0);
    let (dx, dy) = x_of(&rig, "d");
    assert_eq!(dx, 0.0);
    assert!(dy > 0.0, "fourth item wraps to row two");
}

#[test]
fn grid_auto_fits_as_many_min_width_columns_as_fit() {
    let rig = Rig::<Msg>::new(|| {
        Grid::new(GridColumns::from(100.0))
            .width(SizeValue::Points(350.0))
            .children(["a", "b", "c", "d"].map(Text::new))
            .into()
    });
    let (_, cy) = x_of(&rig, "c");
    let (dx, dy) = x_of(&rig, "d");
    assert_eq!(cy, 0.0);
    assert_eq!(dx, 0.0);
    assert!(dy > 0.0);
}

#[test]
fn center_centers_on_both_axes() {
    let rig = Rig::<Msg>::sized(
        || {
            Center::new()
                .width(SizeValue::Points(200.0))
                .height(SizeValue::Points(100.0))
                .child(Text::new("c"))
                .into()
        },
        400,
        300,
    );
    let tree = rig.tree();
    let n = find(&tree, Role::Text, "c").unwrap();
    assert!((n.bounds.x + n.bounds.width / 2.0 - 100.0).abs() <= 1.0);
    assert!((n.bounds.y + n.bounds.height / 2.0 - 50.0).abs() <= 1.0);
}

#[test]
fn aspect_ratio_holds_its_ratio() {
    let rig = Rig::<Msg>::new(|| {
        VStack::new()
            .width(SizeValue::Points(320.0))
            .child(AspectRatio::new(16.0 / 9.0, Text::new("media")))
            .child(Text::new("after"))
            .into()
    });
    assert_eq!(x_of(&rig, "after").1, 180.0);
}

#[test]
fn section_defaults_to_step_four_padding_and_bleeds_in_cards() {
    let rig = Rig::<Msg>::new(|| {
        Card::new()
            .width(SizeValue::Points(300.0))
            .child(Section::new().child(Text::new("in section")))
            .into()
    });
    // The section reaches through the card's 16 point inset, then pads 16.
    assert_eq!(x_of(&rig, "in section"), (16.0, 32.0));
}

#[test]
fn card_padding_and_variants() {
    let rig = Rig::<Msg>::new(|| {
        VStack::new()
            .child(Card::new().padding(Space::S6).child(Text::new("p6")))
            .child(
                Card::new()
                    .variant(CardVariant::Muted)
                    .padding(Space::S2)
                    .child(Text::new("p2")),
            )
            .into()
    });
    assert_eq!(x_of(&rig, "p6").0, 24.0);
    assert_eq!(x_of(&rig, "p2").0, 8.0);
}

#[test]
fn empty_state_has_a_heading_at_the_level_asked() {
    let rig = Rig::<Msg>::new(|| {
        EmptyState::new("No results")
            .description("Try another search.")
            .heading_level(2)
            .into()
    });
    let tree = rig.tree();
    assert!(find(&tree, Role::Heading(2), "No results").is_some());
}

#[test]
fn layout_places_header_panels_content_and_footer() {
    let rig = Rig::<Msg>::sized(
        || {
            Layout::new()
                .header(
                    LayoutHeader::new()
                        .has_divider(true)
                        .child(Text::new("head")),
                )
                .start(
                    LayoutPanel::new()
                        .width(200.0)
                        .has_divider(true)
                        .child(Text::new("nav")),
                )
                .content(LayoutContent::new().child(Text::new("body")))
                .footer(LayoutFooter::new().child(Text::new("foot")))
                .into()
        },
        800,
        600,
    );
    let (hx, hy) = x_of(&rig, "head");
    let (nx, ny) = x_of(&rig, "nav");
    let (bx, by) = x_of(&rig, "body");
    let (_, fy) = x_of(&rig, "foot");
    assert_eq!((hx, hy), (16.0, 16.0));
    assert_eq!(nx, 16.0);
    assert_eq!(bx, 216.0);
    assert!(ny > hy && by == ny);
    assert!(fy > 500.0, "footer sits at the bottom of a fill layout");
}
