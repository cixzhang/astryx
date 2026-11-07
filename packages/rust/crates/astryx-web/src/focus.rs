//! Focus helpers over the accessibility tree.

use astryx_core::A11yNode;

/// The focused node's id.
pub(crate) fn focused_id(node: &A11yNode) -> Option<u64> {
    if node.focused {
        return Some(node.id);
    }
    node.children.iter().find_map(focused_id)
}

/// Ids of focusable nodes in tree (reading) order.
pub(crate) fn focus_order(node: &A11yNode) -> Vec<u64> {
    fn visit(node: &A11yNode, out: &mut Vec<u64>) {
        if node.focusable {
            out.push(node.id);
        }
        for child in &node.children {
            visit(child, out);
        }
    }
    let mut out = Vec::new();
    visit(node, &mut out);
    out
}

/// Whether a `Next` or `Previous` step moved focus forward or backward in
/// reading order without wrapping. The shell lets the browser move focus out
/// of the app when this is false.
pub(crate) fn stepped_within(
    order: &[u64],
    before: Option<u64>,
    after: Option<u64>,
    forward: bool,
) -> bool {
    let index = |id: Option<u64>| id.and_then(|id| order.iter().position(|o| *o == id));
    match (index(before), index(after)) {
        (Some(b), Some(a)) => {
            if forward {
                a > b
            } else {
                a < b
            }
        }
        (None, Some(_)) => true,
        _ => false,
    }
}

/// The deepest focusable node whose bounds contain `(x, y)`, in logical
/// points. Later siblings win, as they paint on top.
pub(crate) fn hit_test(node: &A11yNode, x: f32, y: f32) -> Option<&A11yNode> {
    let b = &node.bounds;
    let inside = x >= b.x && x < b.x + b.width && y >= b.y && y < b.y + b.height;
    let child = node.children.iter().rev().find_map(|c| hit_test(c, x, y));
    child.or((inside && node.focusable).then_some(node))
}

#[cfg(test)]
mod tests {
    use super::*;
    use astryx_core::{Rect, Role};

    fn node(id: u64, role: Role, focused: bool) -> A11yNode {
        A11yNode {
            rtl: false,
            text_selection: None,
            secure: false,
            description: None,
            selected: None,
            focusable: matches!(role, Role::Button | Role::Switch | Role::ListItem),
            live: None,
            id,
            role,
            label: None,
            value: None,
            bounds: Rect::default(),
            focused,
            disabled: false,
            checked: None,
            children: vec![],
        }
    }

    #[test]
    fn order_follows_the_tree_and_skips_static_nodes() {
        let mut list = node(5, Role::List, false);
        list.children = vec![
            node(6, Role::ListItem, true),
            node(7, Role::ListItem, false),
        ];
        let mut root = node(1, Role::Group, false);
        root.children = vec![
            node(2, Role::Text, false),
            node(3, Role::Button, false),
            list,
            node(4, Role::Switch, false),
        ];
        assert_eq!(focus_order(&root), vec![3, 6, 7, 4]);
        assert_eq!(focused_id(&root), Some(6));
    }

    #[test]
    fn hit_test_finds_the_deepest_focusable_node() {
        let mut card = node(5, Role::List, false);
        card.bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut item = node(6, Role::ListItem, false);
        item.bounds = Rect::new(10.0, 10.0, 50.0, 20.0);
        let mut button = node(7, Role::Button, false);
        button.bounds = Rect::new(20.0, 12.0, 10.0, 10.0);
        item.children = vec![button];
        card.children = vec![item];
        let mut root = node(1, Role::Group, false);
        root.bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        root.children = vec![card];
        assert_eq!(hit_test(&root, 25.0, 15.0).map(|n| n.id), Some(7));
        assert_eq!(hit_test(&root, 12.0, 15.0).map(|n| n.id), Some(6));
        assert_eq!(hit_test(&root, 90.0, 90.0).map(|n| n.id), None);
        assert_eq!(hit_test(&root, 150.0, 150.0).map(|n| n.id), None);
    }

    #[test]
    fn detects_wrapping_steps() {
        let order = [10, 20, 30];
        assert!(stepped_within(&order, Some(10), Some(20), true));
        assert!(!stepped_within(&order, Some(30), Some(10), true));
        assert!(!stepped_within(&order, Some(30), Some(30), true));
        assert!(stepped_within(&order, Some(20), Some(10), false));
        assert!(!stepped_within(&order, Some(10), Some(30), false));
        assert!(stepped_within(&order, None, Some(10), true));
    }
}
