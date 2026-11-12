//! Wave 8: AppShell, TopNav, SideNav, MobileNav and TreeList.

use astryx_core::contract::{Input, Role};
use astryx_core::Key;
use astryx_spike_ui::testkit::{all, find, Rig};
use astryx_spike_ui::*;

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Go(&'static str),
}

fn shell() -> astryx_core::Element<Msg> {
    AppShell::new()
        .top_nav(
            TopNav::new()
                .heading(Heading::new(1, "Acme"))
                .end_content(Button::new("Help")),
        )
        .side_nav(|| {
            SideNav::new()
                .child(
                    SideNavSection::new("Workspace")
                        .child(
                            SideNavItem::new("Home")
                                .icon(Icon::new("home"))
                                .is_selected(true)
                                .on_click(Msg::Go("home")),
                        )
                        .child(SideNavItem::new("Projects").on_click(Msg::Go("projects"))),
                )
                .into()
        })
        .child(Text::new("Dashboard"))
        .into()
}

#[test]
fn app_shell_lays_out_navigation_and_main() {
    let rig = Rig::<Msg>::sized(shell, 1280, 800);
    let tree = rig.tree();
    assert!(find(&tree, Role::Navigation, "Top navigation").is_some());
    let side = find(&tree, Role::Navigation, "Side navigation").expect("side nav beside content");
    assert_eq!(side.bounds.width, 260.0);
    let main = all(&tree, Role::Main)[0];
    assert!(main.bounds.x >= 260.0 && main.bounds.width > 900.0);
    let home = find(&tree, Role::Button, "Home").unwrap();
    assert_eq!(home.selected, Some(true), "the current item");
}

#[test]
fn app_shell_moves_side_nav_into_a_drawer_on_phones() {
    let mut rig = Rig::<Msg>::sized(shell, 390, 800);
    assert!(find(&rig.tree(), Role::Navigation, "Side navigation").is_none());
    let menu = find(&rig.tree(), Role::Button, "Open navigation").is_some();
    assert!(menu);
    // Help is the first stop in the top nav; the menu button follows.
    rig.press(Input::Next);
    rig.press(Input::A);
    let tree = rig.tree();
    assert!(
        find(&tree, Role::Dialog, "Navigation").is_some(),
        "{tree:#?}"
    );
    assert!(find(&tree, Role::Navigation, "Side navigation").is_some());
    rig.press(Input::B);
    assert!(find(&rig.tree(), Role::Dialog, "Navigation").is_none());
}

#[test]
fn side_nav_collapses_to_a_rail() {
    let mut rig = Rig::<Msg>::sized(
        || {
            SideNav::new()
                .collapsible(true)
                .child(SideNavItem::new("Home").icon(Icon::new("home")))
                .into()
        },
        800,
        600,
    );
    rig.press(Input::A);
    let tree = rig.tree();
    let nav = find(&tree, Role::Navigation, "Side navigation").unwrap();
    assert_eq!(nav.bounds.width, 48.0);
    let home = find(&tree, Role::Button, "Home").unwrap();
    assert_eq!(home.bounds.width, home.bounds.height, "square icon item");
}

#[test]
fn tree_list_expands_and_navigates_by_arrows() {
    let mut rig = Rig::<Msg>::new(|| {
        TreeList::new(vec![
            TreeListItem::new("src", "src")
                .child(TreeListItem::new("lib", "lib.rs"))
                .child(TreeListItem::new("main", "main.rs")),
            TreeListItem::new("readme", "README.md"),
        ])
        .label("Files")
        .into()
    });
    let tree = rig.tree();
    assert!(find(&tree, Role::Tree, "Files").is_some());
    assert_eq!(all(&tree, Role::TreeItem).len(), 2);
    let src = find(&tree, Role::TreeItem, "src").unwrap();
    assert_eq!(src.state.expanded, Some(false));
    rig.key(Key::Right, Default::default());
    let tree = rig.tree();
    assert_eq!(all(&tree, Role::TreeItem).len(), 4);
    let lib = find(&tree, Role::TreeItem, "lib.rs").unwrap();
    assert_eq!(lib.state.level, Some(2));
    // Right again enters the branch; Left goes back to the parent; Left
    // again closes it.
    rig.key(Key::Right, Default::default());
    rig.key(Key::Left, Default::default());
    rig.key(Key::Left, Default::default());
    assert_eq!(all(&rig.tree(), Role::TreeItem).len(), 2);
}
