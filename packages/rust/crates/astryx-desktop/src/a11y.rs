//! Bridges the Astryx [`A11yNode`] tree to AccessKit.

use accesskit::{
    Action, Node, NodeId, Rect, Role as AkRole, Toggled, TreeId, TreeInfo, TreeUpdate,
};
use astryx_core::{A11yNode, Role};

/// The id of the window node that holds the app's tree. App ids never use it.
pub const WINDOW_ID: NodeId = NodeId(u64::MAX);

fn role(role: Role) -> AkRole {
    match role {
        Role::Group => AkRole::Group,
        Role::Text => AkRole::Label,
        Role::Heading(_) => AkRole::Heading,
        Role::Button => AkRole::Button,
        Role::Switch => AkRole::Switch,
        Role::Checkbox => AkRole::CheckBox,
        Role::ProgressBar => AkRole::ProgressIndicator,
        Role::TextInput => AkRole::TextInput,
        Role::List => AkRole::List,
        Role::ListItem => AkRole::ListItem,
        Role::Image => AkRole::Image,
    }
}

fn add(node: &A11yNode, scale: f64, out: &mut Vec<(NodeId, Node)>, focus: &mut Option<NodeId>) {
    let mut ak = Node::new(role(node.role));
    if let Some(label) = &node.label {
        ak.set_label(label.clone());
    }
    if let Some(value) = &node.value {
        ak.set_value(value.clone());
    }
    let b = node.bounds;
    ak.set_bounds(Rect {
        x0: b.x as f64 * scale,
        y0: b.y as f64 * scale,
        x1: (b.x + b.width) as f64 * scale,
        y1: (b.y + b.height) as f64 * scale,
    });
    if let Role::Heading(level) = node.role {
        ak.set_level(level as usize);
    }
    if let Some(checked) = node.checked {
        ak.set_toggled(Toggled::from(checked));
    }
    if node.disabled {
        ak.set_disabled();
    }
    if matches!(node.role, Role::Button | Role::Switch | Role::Checkbox) && !node.disabled {
        ak.add_action(Action::Click);
    }
    if node.focused {
        *focus = Some(NodeId(node.id));
    }
    ak.set_children(
        node.children
            .iter()
            .map(|c| NodeId(c.id))
            .collect::<Vec<_>>(),
    );
    out.push((NodeId(node.id), ak));
    for child in &node.children {
        add(child, scale, out, focus);
    }
}

/// A full tree update: a window node named `title` holding `root`. Bounds
/// go from logical points to physical pixels with `scale`.
pub fn tree_update(root: Option<&A11yNode>, title: &str, scale: f64) -> TreeUpdate {
    let mut nodes = Vec::new();
    let mut focus = None;
    let mut window = Node::new(AkRole::Window);
    window.set_label(title.to_owned());
    if let Some(root) = root {
        window.set_children(vec![NodeId(root.id)]);
        add(root, scale, &mut nodes, &mut focus);
    }
    nodes.insert(0, (WINDOW_ID, window));
    TreeUpdate {
        nodes,
        tree: Some(TreeInfo::new(WINDOW_ID)),
        tree_id: TreeId::ROOT,
        focus: focus.unwrap_or(WINDOW_ID),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astryx_core::Rect as R;

    fn node(id: u64, role: Role, focused: bool, children: Vec<A11yNode>) -> A11yNode {
        A11yNode {
            id,
            role,
            label: Some(format!("n{id}")),
            value: None,
            bounds: R::new(1.0, 2.0, 10.0, 20.0),
            focused,
            disabled: false,
            checked: if role == Role::Switch {
                Some(true)
            } else {
                None
            },
            children,
        }
    }

    #[test]
    fn tree_is_wrapped_in_a_window_and_tracks_focus() {
        let root = node(
            1,
            Role::Group,
            false,
            vec![
                node(2, Role::Button, false, vec![]),
                node(3, Role::Switch, true, vec![]),
            ],
        );
        let update = tree_update(Some(&root), "App", 2.0);
        assert_eq!(update.nodes.len(), 4);
        assert_eq!(update.nodes[0].0, WINDOW_ID);
        assert_eq!(update.focus, NodeId(3));
        let switch = &update
            .nodes
            .iter()
            .find(|(id, _)| *id == NodeId(3))
            .unwrap()
            .1;
        assert_eq!(switch.role(), AkRole::Switch);
        assert_eq!(switch.toggled(), Some(Toggled::True));
        assert_eq!(
            switch.bounds(),
            Some(Rect {
                x0: 2.0,
                y0: 4.0,
                x1: 22.0,
                y1: 44.0
            })
        );
        let group = &update
            .nodes
            .iter()
            .find(|(id, _)| *id == NodeId(1))
            .unwrap()
            .1;
        assert_eq!(group.children(), &[NodeId(2), NodeId(3)]);
    }

    #[test]
    fn no_tree_focuses_the_window() {
        let update = tree_update(None, "App", 1.0);
        assert_eq!(update.nodes.len(), 1);
        assert_eq!(update.focus, WINDOW_ID);
    }
}
