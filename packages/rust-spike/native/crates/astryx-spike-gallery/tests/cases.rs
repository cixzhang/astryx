//! Every gallery case builds, names its box "case", and has a stable id.

use astryx_core::contract::Role;
use astryx_spike_gallery::{all_case_ids, Gallery};
use std::collections::HashSet;

#[test]
fn every_case_renders_in_a_named_box() {
    let ids = all_case_ids();
    assert!(ids.len() > 200, "{} cases", ids.len());
    let unique: HashSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len(), "case ids are unique");
    for id in ids {
        let g = Gallery::case(&id).unwrap_or_else(|| panic!("{id}"));
        let mut rt = astryx_core::Runtime::new(g);
        astryx_core::testing::draw(&mut rt, 420, 300);
        let tree = astryx_core::contract::Surface::accessibility(&rt).expect(&id);
        // A modal layer (BottomSheet) makes the page, case box included,
        // inert, so only the layer is in the tree.
        let named = astryx_spike_ui::testkit::flatten(&tree).iter().any(|c| {
            (c.role == Role::Group && c.label.as_deref() == Some("case"))
                || c.role == Role::Dialog
                || c.role == Role::AlertDialog
        });
        assert!(named, "{id}: {tree:#?}");
    }
}

#[test]
fn browse_mode_switches_family_and_scheme() {
    let mut rt = astryx_core::Runtime::new(Gallery::new());
    astryx_core::testing::draw(&mut rt, 1280, 800);
    rt.send(astryx_spike_gallery::Msg::Family(3));
    rt.send(astryx_spike_gallery::Msg::ToggleScheme);
    astryx_core::testing::draw(&mut rt, 1280, 800);
}

#[test]
fn playground_pages_keep_live_state() {
    use astryx_core::contract::{Input, Surface};
    use astryx_core::testing::tap;
    fn find<'a>(
        n: &'a astryx_core::A11yNode,
        role: Role,
        label: &str,
    ) -> Option<&'a astryx_core::A11yNode> {
        if n.role == role && n.label.as_deref() == Some(label) {
            return Some(n);
        }
        n.children.iter().find_map(|c| find(c, role, label))
    }
    // Page 1: Space on the switch toggles it.
    let mut rt = astryx_core::Runtime::new(Gallery::case("playground-0001").unwrap());
    astryx_core::testing::draw(&mut rt, 800, 600);
    let _ = tap(&mut rt, Input::A);
    astryx_core::testing::draw(&mut rt, 800, 600);
    let tree = rt.accessibility().unwrap();
    assert_eq!(
        find(&tree, Role::Switch, "Email notifications")
            .unwrap()
            .checked,
        Some(true)
    );
    assert!(find(&tree, Role::Text, "Notifications: on").is_some());
    // Page 2: typing changes the field.
    let mut rt = astryx_core::Runtime::new(Gallery::case("playground-0002").unwrap());
    astryx_core::testing::draw(&mut rt, 800, 600);
    rt.text_input("Ada");
    astryx_core::testing::draw(&mut rt, 800, 600);
    let tree = rt.accessibility().unwrap();
    assert_eq!(
        find(&tree, Role::TextInput, "Name")
            .unwrap()
            .value
            .as_deref(),
        Some("Ada")
    );
    // Page 4: the table renders five rows with sort buttons.
    let mut rt = astryx_core::Runtime::new(Gallery::case("playground-0004").unwrap());
    astryx_core::testing::draw(&mut rt, 1024, 768);
    let tree = rt.accessibility().unwrap();
    assert!(find(&tree, Role::Table, "People").is_some());
    assert!(find(&tree, Role::Button, "Age").is_some());
}
