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
    let mut rig = Rig::new(|| {
        ContextMenu::new(
            Button::new("Row"),
            vec![MenuOption::item("Copy").on_click(Msg::Act("copy"))],
        )
        .on_open_change(Msg::Ctx)
        .into()
    });
    rig.right_click(Point { x: 10.0, y: 10.0 });
    assert_eq!(
        rig.messages(),
        vec![Msg::Ctx(Some(Point { x: 10.0, y: 10.0 }))]
    );
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

#[test]
fn resizable_pane_follows_a_drag_within_its_limits() {
    let mut rig = Rig::<Msg>::sized(
        || {
            Resizable::new(
                "side",
                "Resize sidebar",
                Text::new("Pane"),
                Text::new("Rest"),
            )
            .min_size(120.0)
            .max_size(400.0)
            .into()
        },
        800,
        400,
    );
    let sep = |rig: &Rig<Msg>| {
        find(&rig.tree(), Role::Separator, "Resize sidebar")
            .unwrap()
            .value
            .clone()
    };
    assert_eq!(sep(&rig).as_deref(), Some("250"));
    rig.drag(
        Point { x: 250.0, y: 100.0 },
        Point { x: 330.0, y: 100.0 },
        8,
    );
    assert_eq!(sep(&rig).as_deref(), Some("330"));
    rig.drag(
        Point { x: 330.0, y: 100.0 },
        Point { x: 700.0, y: 100.0 },
        8,
    );
    assert_eq!(sep(&rig).as_deref(), Some("400"), "clamped to max");
}

#[test]
fn resizable_collapses_and_snaps() {
    assert_eq!(
        resolve_size(60.0, 120.0, 400.0, true, 40.0, &[], false),
        40.0
    );
    assert_eq!(
        resolve_size(205.0, 120.0, 400.0, false, 40.0, &[200.0], true),
        200.0
    );
    assert_eq!(
        resolve_size(205.0, 120.0, 400.0, false, 40.0, &[200.0], false),
        205.0
    );
}

#[test]
fn uncontrolled_context_menu_opens_from_shift_f10_and_closes_on_act() {
    let mut rig = Rig::new(|| {
        ContextMenu::new(
            Button::new("Row"),
            vec![MenuOption::item("Copy").on_click(Msg::Act("copy"))],
        )
        .into()
    });
    assert!(all(&rig.tree(), Role::Menu).is_empty());
    rig.key(
        astryx_core::Key::F(10),
        astryx_core::Modifiers {
            shift: true,
            ..Default::default()
        },
    );
    assert!(
        !all(&rig.tree(), Role::Menu).is_empty(),
        "opens from the keyboard"
    );
    rig.press(Input::A);
    assert_eq!(rig.messages(), vec![Msg::Act("copy")]);
    assert!(all(&rig.tree(), Role::Menu).is_empty());
    rig.right_click(Point { x: 10.0, y: 10.0 });
    assert!(
        !all(&rig.tree(), Role::Menu).is_empty(),
        "opens from a right click"
    );
}

#[derive(Clone, Debug, PartialEq)]
enum RMsg {
    Moved(usize, usize),
}

#[test]
fn list_reorders_by_keyboard_with_announcements() {
    let mut rig = Rig::<RMsg>::new(|| {
        List::new()
            .reorderable(Reorderable::new(RMsg::Moved).labels(["Inbox", "Drafts", "Sent"]))
            .child(ListItem::new("Inbox"))
            .child(ListItem::new("Drafts"))
            .child(ListItem::new("Sent"))
            .into()
    });
    let tree = rig.tree();
    assert!(find(&tree, Role::Button, "Reorder Inbox").is_some());
    // Focus starts on the first grip: pick up, move down twice, drop.
    rig.key(astryx_core::Key::Space, Default::default());
    let status = |rig: &Rig<RMsg>| {
        all(&rig.tree(), Role::Status)
            .first()
            .and_then(|s| s.label.clone())
    };
    assert_eq!(status(&rig).as_deref(), Some("Picked up, position 1 of 3."));
    rig.key(astryx_core::Key::Down, Default::default());
    rig.key(astryx_core::Key::Down, Default::default());
    // The row shows at its new place while it is held.
    let names: Vec<String> = all(&rig.tree(), Role::Button)
        .iter()
        .filter_map(|b| b.label.clone())
        .collect();
    assert_eq!(names, ["Reorder Drafts", "Reorder Sent", "Reorder Inbox"]);
    rig.key(astryx_core::Key::Space, Default::default());
    assert_eq!(rig.messages(), vec![RMsg::Moved(0, 2)]);
    assert_eq!(status(&rig).as_deref(), Some("Dropped, position 3 of 3"));
}

#[test]
fn escape_puts_a_picked_up_row_back() {
    let mut rig = Rig::<RMsg>::new(|| {
        List::new()
            .reorderable(Reorderable::new(RMsg::Moved))
            .child(ListItem::new("A"))
            .child(ListItem::new("B"))
            .into()
    });
    rig.key(astryx_core::Key::Enter, Default::default());
    rig.key(astryx_core::Key::Down, Default::default());
    rig.key(astryx_core::Key::Escape, Default::default());
    assert!(rig.messages().is_empty());
    let names: Vec<String> = all(&rig.tree(), Role::Button)
        .iter()
        .filter_map(|b| b.label.clone())
        .collect();
    assert_eq!(names, ["Reorder Item 1", "Reorder Item 2"]);
}
