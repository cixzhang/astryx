//! Wave 6: composites — menus, dialogs, lists, disclosure — and the
//! uncontrolled overlays.

use astryx_core::contract::{Input, Role};
use astryx_core::{Element, Point};
use astryx_spike_ui::testkit::{all, find, Rig};
use astryx_spike_ui::*;

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Act(&'static str),
    Open(bool),
    Page(u32),
    Ctx(Option<Point>),
}

#[test]
fn uncontrolled_dropdown_menu_opens_acts_and_closes() {
    let mut rig = Rig::new(|| {
        DropdownMenu::new(
            "more",
            vec![
                MenuOption::item("Rename").on_click(Msg::Act("rename")),
                MenuOption::Divider,
                MenuOption::item("Delete")
                    .variant(MenuItemVariant::Destructive)
                    .on_click(Msg::Act("delete")),
            ],
        )
        .label("Actions")
        .into()
    });
    assert!(all(&rig.tree(), Role::Menu).is_empty());
    rig.press(Input::A);
    let tree = rig.tree();
    assert!(!all(&tree, Role::Menu).is_empty(), "{tree:#?}");
    assert_eq!(all(&tree, Role::MenuItem).len(), 2);
    // The menu traps focus on its first row; activate it.
    rig.press(Input::A);
    assert_eq!(rig.messages(), vec![Msg::Act("rename")]);
    assert!(
        all(&rig.tree(), Role::Menu).is_empty(),
        "closes after acting"
    );
}

#[test]
fn escape_closes_an_open_menu() {
    let mut rig = Rig::new(|| {
        DropdownMenu::new("m", vec![MenuOption::item("One").on_click(Msg::Act("one"))]).into()
    });
    rig.press(Input::A);
    assert!(!all(&rig.tree(), Role::Menu).is_empty());
    rig.press(Input::B);
    assert!(all(&rig.tree(), Role::Menu).is_empty());
}

#[test]
fn menu_checkbox_and_radio_rows_report_state() {
    let rig = Rig::new(|| {
        DropdownMenu::new(
            "view",
            vec![
                MenuOption::checkbox("Show grid", true).on_change(|_| Msg::Act("grid")),
                MenuOption::group(
                    Some("Sort".into()),
                    vec![
                        MenuOption::radio("Name", true).on_click(Msg::Act("name")),
                        MenuOption::radio("Date", false).on_click(Msg::Act("date")),
                    ],
                ),
            ],
        )
        .is_menu_open(true)
        .on_open_change(Msg::Open)
        .into()
    });
    let tree = rig.tree();
    let cb = all(&tree, Role::MenuItemCheckbox);
    assert_eq!(cb[0].checked, Some(true));
    let radios = all(&tree, Role::MenuItemRadio);
    assert_eq!(radios.len(), 2);
    assert_eq!(radios[0].checked, Some(true));
}

#[test]
fn context_menu_opens_at_a_point() {
    // Opening from Shift+F10, the Menu key or a secondary press waits on a
    // bubbling key handler and secondary press events in the runtime.
    let open = Rig::new(|| {
        ContextMenu::new(Button::new("Row"), vec![MenuOption::item("Copy")])
            .open_at(Some(Point { x: 120.0, y: 80.0 }))
            .on_open_change(Msg::Ctx)
            .into()
    });
    let tree = open.tree();
    let menu = find(&tree, Role::Menu, "Context menu").expect("menu at point");
    assert_eq!((menu.bounds.x, menu.bounds.y), (120.0, 80.0));
}

#[test]
fn dialog_with_header_traps_focus_and_closes_on_escape() {
    let mut rig = Rig::sized(
        || {
            VStack::new()
                .child(Button::new("Behind").on_click(Msg::Act("behind")))
                .child(
                    Dialog::new(true, Msg::Open).child(
                        Layout::new()
                            .height(LayoutHeight::Auto)
                            .header(
                                DialogHeader::new("Settings")
                                    .subtitle("Workspace")
                                    .on_open_change(Msg::Open),
                            )
                            .content(LayoutContent::new().child(Text::new("Body"))),
                    ),
                )
                .into()
        },
        1024,
        768,
    );
    let tree = rig.tree();
    assert!(find(&tree, Role::Heading(2), "Settings").is_some());
    assert!(find(&tree, Role::Button, "Close").is_some());
    assert!(
        find(&tree, Role::Button, "Behind").is_none(),
        "page is inert"
    );
    let dialog = all(&tree, Role::Dialog)[0];
    assert_eq!(dialog.bounds.width, 400.0);
    rig.press(Input::B);
    assert_eq!(rig.messages(), vec![Msg::Open(false)]);
}

#[test]
fn required_dialog_is_an_alert_dialog_and_ignores_escape() {
    let mut rig = Rig::new(|| {
        Dialog::new(true, Msg::Open)
            .purpose(DialogPurpose::Required)
            .label("Confirm")
            .child(Button::new("OK").on_click(Msg::Act("ok")))
            .into()
    });
    assert!(find(&rig.tree(), Role::AlertDialog, "Confirm").is_some());
    rig.press(Input::B);
    assert!(rig.messages().is_empty());
}

#[test]
fn collapsible_starts_open_and_hides_content_from_the_tree_when_closed() {
    let mut rig = Rig::<Msg>::new(|| {
        Collapsible::new("Details")
            .child(Button::new("Inside"))
            .into()
    });
    let t = rig.tree();
    let trigger = find(&t, Role::Button, "Details").unwrap();
    assert!(find(&t, Role::Button, "Inside").is_some());
    let _ = trigger;
    rig.press(Input::A);
    assert!(
        find(&rig.tree(), Role::Button, "Inside").is_none(),
        "closed content is inert"
    );
    rig.press(Input::A);
    assert!(find(&rig.tree(), Role::Button, "Inside").is_some());
}

#[test]
fn page_items_keep_first_last_and_siblings() {
    assert_eq!(page_items(1, 5, 1), (1..=5).map(Some).collect::<Vec<_>>());
    assert_eq!(
        page_items(10, 20, 1),
        vec![Some(1), None, Some(9), Some(10), Some(11), None, Some(20)]
    );
    assert_eq!(
        page_items(1, 20, 1),
        vec![Some(1), Some(2), Some(3), Some(4), Some(5), None, Some(20)]
    );
    assert_eq!(
        page_items(20, 20, 1),
        vec![
            Some(1),
            None,
            Some(16),
            Some(17),
            Some(18),
            Some(19),
            Some(20)
        ]
    );
}

#[test]
fn pagination_goes_to_next_page() {
    let mut rig = Rig::new(|| Pagination::new(2, Msg::Page).total_pages(5).into());
    let tree = rig.tree();
    assert!(find(&tree, Role::Navigation, "Pagination").is_some());
    let next = find(&tree, Role::Button, "Go to next page").unwrap();
    assert!(!next.disabled);
    // Focus starts on "previous"; walk to "next" (after 5 page buttons).
    for _ in 0..6 {
        rig.press(Input::Next);
    }
    rig.press(Input::A);
    assert_eq!(rig.messages(), vec![Msg::Page(3)]);
}

#[test]
fn toolbar_and_list_roles() {
    let rig = Rig::<Msg>::new(|| {
        VStack::new()
            .child(
                Toolbar::new("Formatting")
                    .start_content(Button::new("Bold"))
                    .end_content(Button::new("More")),
            )
            .child(
                List::new()
                    .has_dividers(true)
                    .child(ListItem::new("Inbox").description("12 new"))
                    .child(ListItem::new("Archive")),
            )
            .into()
    });
    let tree = rig.tree();
    assert!(find(&tree, Role::Toolbar, "Formatting").is_some());
    assert_eq!(all(&tree, Role::List).len(), 1);
}

#[test]
fn uncontrolled_popover_toggles_from_its_trigger() {
    let mut rig = Rig::new(|| -> Element<Msg> {
        Popover::new("filters", Text::new("Filters"), Text::new("Options"))
            .label("Filter options")
            .into()
    });
    assert!(find(&rig.tree(), Role::Dialog, "Filter options").is_none());
    rig.press(Input::A);
    assert!(find(&rig.tree(), Role::Dialog, "Filter options").is_some());
    rig.press(Input::B);
    assert!(find(&rig.tree(), Role::Dialog, "Filter options").is_none());
}

#[test]
fn button_tooltip_shows_on_keyboard_focus() {
    let mut rig = Rig::<Msg>::new(|| {
        VStack::new()
            .padding(astryx_core::Space::S10)
            .child(Button::new("A"))
            .child(Button::new("Save").tooltip("Save changes"))
            .into()
    });
    rig.press(Input::Next);
    // Several frames for the hover timer.
    for _ in 0..3 {
        rig.redraw();
    }
    let tree = rig.tree();
    let b = find(&tree, Role::Button, "Save").unwrap();
    assert_eq!(b.description.as_deref(), Some("Save changes"));
    assert!(
        find(&tree, Role::Tooltip, "Save changes").is_some(),
        "{tree:#?}"
    );
}
