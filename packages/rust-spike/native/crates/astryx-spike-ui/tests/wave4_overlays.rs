//! Wave 4: light overlays on layers.

use astryx_core::contract::{Input, Role};
use astryx_core::Element;
use astryx_spike_ui::testkit::{all, find, Rig};
use astryx_spike_ui::*;

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Open(bool),
    Clicked,
}

#[test]
fn tooltip_shows_only_when_open_and_names_itself() {
    let view = |open: bool| {
        move || -> Element<Msg> {
            // Room above the trigger for the tooltip and the viewport gutter.
            VStack::new()
                .padding(astryx_core::Space::S10)
                .child(
                    VStack::new().padding(astryx_core::Space::S10).child(
                        Tooltip::new("save", Button::new("Save"), "Save your changes")
                            .is_open(open)
                            .on_open_change(Msg::Open),
                    ),
                )
                .into()
        }
    };
    let closed = Rig::new(view(false));
    assert!(all(&closed.tree(), Role::Tooltip).is_empty());
    let open = Rig::new(view(true));
    let tree = open.tree();
    let tip = find(&tree, Role::Tooltip, "Save your changes").expect("tooltip layer");
    let button = find(&tree, Role::Button, "Save").unwrap();
    // Above its anchor, with a 4 point gap.
    assert!(tip.bounds.y + tip.bounds.height <= button.bounds.y);
}

#[test]
fn popover_opens_below_its_trigger_and_escape_closes_it() {
    let mut rig = Rig::new(|| {
        VStack::new()
            .child(
                Popover::new(
                    "filters",
                    Button::new("Filters").on_click(Msg::Clicked),
                    Text::new("Filter options"),
                )
                .is_open(true)
                .label("Filters")
                .on_open_change(Msg::Open),
            )
            .into()
    });
    let tree = rig.tree();
    let dialog = find(&tree, Role::Dialog, "Filters").expect("popover dialog");
    let trigger = find(&tree, Role::Button, "Filters").unwrap();
    assert!(dialog.bounds.y >= trigger.bounds.y + trigger.bounds.height);
    rig.press(Input::B);
    assert_eq!(rig.messages(), vec![Msg::Open(false)]);
}

#[test]
fn hover_card_opens_as_a_dialog_layer() {
    let rig = Rig::new(|| {
        VStack::new()
            .padding(astryx_core::Space::S10)
            .child(
                HoverCard::new(
                    "ada",
                    Link::new("Ada").href("#"),
                    Text::new("Mathematician"),
                )
                .label("Ada Lovelace")
                .is_open(true)
                .on_open_change(Msg::Open),
            )
            .into()
    });
    let tree = rig.tree();
    assert!(find(&tree, Role::Dialog, "Ada Lovelace").is_some());
}

#[test]
fn bottom_sheet_is_a_modal_dialog_at_the_bottom() {
    let mut rig = Rig::sized(
        || {
            VStack::new()
                .child(Button::new("Behind").on_click(Msg::Clicked))
                .child(
                    BottomSheet::new("Share", true)
                        .on_open_change(Msg::Open)
                        .child(Text::new("Share with")),
                )
                .into()
        },
        800,
        600,
    );
    let tree = rig.tree();
    let sheet = find(&tree, Role::Dialog, "Share").expect("sheet");
    assert_eq!(sheet.bounds.y + sheet.bounds.height, 600.0);
    assert_eq!(sheet.bounds.width, 640.0);
    rig.press(Input::B);
    assert_eq!(rig.messages(), vec![Msg::Open(false)]);
}

#[test]
fn required_bottom_sheet_ignores_escape() {
    let mut rig = Rig::new(|| {
        BottomSheet::new("Confirm", true)
            .purpose(SheetPurpose::Required)
            .on_open_change(Msg::Open)
            .child(Button::new("OK").on_click(Msg::Clicked))
            .into()
    });
    rig.press(Input::B);
    assert!(rig.messages().is_empty());
}

#[test]
fn info_tip_trigger_toggles_and_describes() {
    let mut rig = Rig::new(|| {
        HStack::new()
            .child(InfoTip::new("why", "Shown to your whole team").on_open_change(Msg::Open))
            .into()
    });
    let tree = rig.tree();
    let b = find(&tree, Role::Button, "More information").unwrap();
    assert_eq!(b.description.as_deref(), Some("Shown to your whole team"));
    rig.press(Input::A);
    // Keyboard focus opens it as hover does; the press asks to open too.
    assert!(rig.messages().iter().all(|m| *m == Msg::Open(true)));
    assert!(!rig.messages().is_empty());
}
