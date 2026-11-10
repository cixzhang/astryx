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
        let named = tree
            .children
            .iter()
            .any(|c| c.role == Role::Group && c.label.as_deref() == Some("case"));
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
