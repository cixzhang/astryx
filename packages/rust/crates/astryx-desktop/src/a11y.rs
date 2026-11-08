//! Bridges the Astryx [`A11yNode`] tree to AccessKit.

use accesskit::{
    Action, ActionRequest, Node, NodeId, Rect, Role as AkRole, TextDirection, Toggled, TreeId,
    TreeInfo, TreeUpdate,
};
use astryx_core::{A11yAction, A11yNode, Live, Role};

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
        // AccessKit has no separator role; Splitter would announce a resize handle.
        Role::Separator => AkRole::GenericContainer,
        Role::ComboBox => AkRole::ComboBox,
        Role::ListBox => AkRole::ListBox,
        Role::ListBoxOption => AkRole::ListBoxOption,
        Role::RadioGroup => AkRole::RadioGroup,
        Role::Radio => AkRole::RadioButton,
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
    if let Some(description) = &node.description {
        ak.set_description(description.clone());
    }
    if let Some(selected) = node.selected {
        ak.set_selected(selected);
    }
    if let Some(live) = node.live {
        ak.set_live(match live {
            Live::Polite => accesskit::Live::Polite,
            Live::Assertive => accesskit::Live::Assertive,
        });
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
    if node.rtl {
        ak.set_text_direction(TextDirection::RightToLeft);
    }
    if node.focusable && !node.disabled {
        ak.add_action(Action::Focus);
    }
    if matches!(
        node.role,
        Role::Button
            | Role::Switch
            | Role::Checkbox
            | Role::TextInput
            | Role::ComboBox
            | Role::ListBoxOption
            | Role::Radio
    ) && !node.disabled
    {
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

/// The Astryx action for an AccessKit request, if the contract has one.
pub fn action(request: &ActionRequest) -> Option<(u64, A11yAction)> {
    if request.target_node == WINDOW_ID {
        return None;
    }
    let action = match request.action {
        Action::Click => A11yAction::Activate,
        Action::Focus => A11yAction::Focus,
        _ => return None,
    };
    Some((request.target_node.0, action))
}

#[cfg(test)]
mod tests {
    use super::*;
    use astryx_core::Rect as R;

    fn node(id: u64, role: Role, focused: bool, children: Vec<A11yNode>) -> A11yNode {
        A11yNode {
            rtl: false,
            text_selection: None,
            secure: false,
            description: None,
            selected: None,
            focusable: false,
            live: None,
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
    fn radios_map_to_radio_roles() {
        assert_eq!(role(Role::RadioGroup), AkRole::RadioGroup);
        assert_eq!(role(Role::Radio), AkRole::RadioButton);
    }

    #[test]
    fn click_and_focus_requests_map_to_actions() {
        let request = |action, node| ActionRequest {
            action,
            target_tree: TreeId::ROOT,
            target_node: node,
            data: None,
        };
        assert_eq!(
            action(&request(Action::Click, NodeId(7))),
            Some((7, A11yAction::Activate))
        );
        assert_eq!(
            action(&request(Action::Focus, NodeId(7))),
            Some((7, A11yAction::Focus))
        );
        assert_eq!(action(&request(Action::ScrollUp, NodeId(7))), None);
        assert_eq!(action(&request(Action::Click, WINDOW_ID)), None);
    }

    #[test]
    fn description_selection_live_and_focus_are_bridged() {
        let mut option = node(9, Role::ListBoxOption, false, vec![]);
        option.description = Some("Ships in 2 days".into());
        option.selected = Some(true);
        option.focusable = true;
        option.live = Some(Live::Polite);
        option.rtl = true;
        let update = tree_update(Some(&option), "App", 1.0);
        let ak = &update
            .nodes
            .iter()
            .find(|(id, _)| *id == NodeId(9))
            .unwrap()
            .1;
        assert_eq!(ak.description(), Some("Ships in 2 days"));
        assert_eq!(ak.is_selected(), Some(true));
        assert_eq!(ak.live(), Some(accesskit::Live::Polite));
        assert!(ak.supports_action(Action::Focus));
        assert_eq!(ak.text_direction(), Some(TextDirection::RightToLeft));
        assert!(ak.supports_action(Action::Click));
    }

    #[test]
    fn no_tree_focuses_the_window() {
        let update = tree_update(None, "App", 1.0);
        assert_eq!(update.nodes.len(), 1);
        assert_eq!(update.focus, WINDOW_ID);
    }
}
