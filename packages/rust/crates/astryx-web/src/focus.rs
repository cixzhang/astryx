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

#[cfg(test)]
mod tests {
    use super::*;
    use astryx_core::{Rect, Role};

    fn node(id: u64, role: Role, focused: bool) -> A11yNode {
        A11yNode {
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
