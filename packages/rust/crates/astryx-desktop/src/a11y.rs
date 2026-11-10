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
        Role::Link => AkRole::Link,
        Role::Dialog => AkRole::Dialog,
        Role::AlertDialog => AkRole::AlertDialog,
        Role::Tooltip => AkRole::Tooltip,
        Role::Menu => AkRole::Menu,
        Role::MenuItem => AkRole::MenuItem,
        Role::MenuItemCheckbox => AkRole::MenuItemCheckBox,
        Role::MenuItemRadio => AkRole::MenuItemRadio,
        Role::TabList => AkRole::TabList,
        Role::Tab => AkRole::Tab,
        Role::TabPanel => AkRole::TabPanel,
        Role::Toolbar => AkRole::Toolbar,
        Role::Grid => AkRole::Grid,
        Role::Table => AkRole::Table,
        Role::RowGroup => AkRole::RowGroup,
        Role::Row => AkRole::Row,
        Role::Cell => AkRole::Cell,
        Role::GridCell => AkRole::GridCell,
        Role::ColumnHeader => AkRole::ColumnHeader,
        Role::RowHeader => AkRole::RowHeader,
        Role::Tree => AkRole::Tree,
        Role::TreeItem => AkRole::TreeItem,
        Role::Navigation => AkRole::Navigation,
        Role::Banner => AkRole::Banner,
        Role::Main => AkRole::Main,
        Role::Complementary => AkRole::Complementary,
        Role::ContentInfo => AkRole::ContentInfo,
        Role::Region => AkRole::Region,
        Role::Search => AkRole::Search,
        Role::Form => AkRole::Form,
        Role::Status => AkRole::Status,
        Role::Alert => AkRole::Alert,
        Role::SearchBox => AkRole::SearchInput,
        Role::SpinButton => AkRole::SpinButton,
        Role::Slider => AkRole::Slider,
        Role::ScrollBar => AkRole::ScrollBar,
    }
}

/// Bridges the ARIA states and relations of `state` onto `ak`.
fn add_state(ak: &mut Node, state: &astryx_core::A11yState) {
    use astryx_core::{A11yOrientation, Current, HasPopup, Sort};
    if let Some(expanded) = state.expanded {
        ak.set_expanded(expanded);
    }
    if let Some(popup) = state.has_popup {
        ak.set_has_popup(match popup {
            HasPopup::Menu => accesskit::HasPopup::Menu,
            HasPopup::ListBox => accesskit::HasPopup::Listbox,
            HasPopup::Tree => accesskit::HasPopup::Tree,
            HasPopup::Grid => accesskit::HasPopup::Grid,
            HasPopup::Dialog => accesskit::HasPopup::Dialog,
        });
    }
    if let Some(pressed) = state.pressed {
        ak.set_toggled(Toggled::from(pressed));
    }
    if state.required {
        ak.set_required();
    }
    if state.invalid {
        ak.set_invalid(accesskit::Invalid::True);
    }
    if state.read_only {
        ak.set_read_only();
    }
    if state.multiselectable {
        ak.set_multiselectable();
    }
    if state.busy {
        ak.set_busy();
    }
    if state.modal {
        ak.set_modal();
    }
    if let Some(current) = state.current {
        ak.set_aria_current(match current {
            Current::True => accesskit::AriaCurrent::True,
            Current::Page => accesskit::AriaCurrent::Page,
            Current::Step => accesskit::AriaCurrent::Step,
            Current::Location => accesskit::AriaCurrent::Location,
            Current::Date => accesskit::AriaCurrent::Date,
            Current::Time => accesskit::AriaCurrent::Time,
        });
    }
    if let Some(o) = state.orientation {
        ak.set_orientation(match o {
            A11yOrientation::Horizontal => accesskit::Orientation::Horizontal,
            A11yOrientation::Vertical => accesskit::Orientation::Vertical,
        });
    }
    if let Some(keys) = &state.key_shortcuts {
        ak.set_keyboard_shortcut(keys.clone());
    }
    if let Some(id) = state.active_descendant {
        ak.set_active_descendant(NodeId(id));
    }
    if !state.controls.is_empty() {
        ak.set_controls(
            state
                .controls
                .iter()
                .map(|&i| NodeId(i))
                .collect::<Vec<_>>(),
        );
    }
    if !state.labelled_by.is_empty() {
        ak.set_labelled_by(
            state
                .labelled_by
                .iter()
                .map(|&i| NodeId(i))
                .collect::<Vec<_>>(),
        );
    }
    if !state.described_by.is_empty() {
        ak.set_described_by(
            state
                .described_by
                .iter()
                .map(|&i| NodeId(i))
                .collect::<Vec<_>>(),
        );
    }
    if let Some(id) = state.error_message {
        ak.set_error_message(NodeId(id));
    }
    if let Some(level) = state.level {
        ak.set_level(level as usize);
    }
    if let Some(pos) = state.pos_in_set {
        ak.set_position_in_set(pos as usize);
    }
    if let Some(size) = state.set_size {
        ak.set_size_of_set(size as usize);
    }
    if let Some(n) = state.row_count {
        ak.set_row_count(n as usize);
    }
    if let Some(n) = state.col_count {
        ak.set_column_count(n as usize);
    }
    // AccessKit indexes rows and columns from 0; ARIA from 1.
    if let Some(n) = state.row_index {
        ak.set_row_index(n.saturating_sub(1) as usize);
    }
    if let Some(n) = state.col_index {
        ak.set_column_index(n.saturating_sub(1) as usize);
    }
    if let Some(sort) = state.sort {
        ak.set_sort_direction(match sort {
            Sort::Ascending => accesskit::SortDirection::Ascending,
            Sort::Descending => accesskit::SortDirection::Descending,
            Sort::Other => accesskit::SortDirection::Other,
        });
    }
    if let Some((min, max, now)) = state.range {
        ak.set_min_numeric_value(min);
        ak.set_max_numeric_value(max);
        ak.set_numeric_value(now);
    }
}

fn add(node: &A11yNode, scale: f64, out: &mut Vec<(NodeId, Node)>, focus: &mut Option<NodeId>) {
    let ak_role = match role(node.role) {
        AkRole::TextInput if node.state.multiline => AkRole::MultilineTextInput,
        r => r,
    };
    let mut ak = Node::new(ak_role);
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
            | Role::Link
            | Role::MenuItem
            | Role::MenuItemCheckbox
            | Role::MenuItemRadio
            | Role::Tab
            | Role::TreeItem
            | Role::GridCell
            | Role::SearchBox
    ) && !node.disabled
    {
        ak.add_action(Action::Click);
    }
    add_state(&mut ak, &node.state);
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
            state: Default::default(),
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
    fn grid_rows_and_combobox_relations_are_bridged() {
        let mut row = node(20, Role::Row, false, vec![]);
        row.state.row_index = Some(5001);
        let mut grid = node(10, Role::Grid, false, vec![row]);
        grid.state.row_count = Some(10_000);
        grid.state.col_count = Some(3);
        let mut combo = node(30, Role::ComboBox, true, vec![]);
        combo.state.expanded = Some(true);
        combo.state.has_popup = Some(astryx_core::HasPopup::ListBox);
        combo.state.active_descendant = Some(20);
        combo.state.controls = vec![10];
        let root = node(1, Role::Group, false, vec![grid, combo]);
        let update = tree_update(Some(&root), "App", 1.0);
        let get = |id| {
            &update
                .nodes
                .iter()
                .find(|(n, _)| *n == NodeId(id))
                .unwrap()
                .1
        };
        assert_eq!(get(10).role(), AkRole::Grid);
        assert_eq!(get(10).row_count(), Some(10_000));
        assert_eq!(get(20).row_index(), Some(5000), "AccessKit is 0-based");
        assert_eq!(get(30).is_expanded(), Some(true));
        assert_eq!(get(30).has_popup(), Some(accesskit::HasPopup::Listbox));
        assert_eq!(get(30).active_descendant(), Some(NodeId(20)));
        assert_eq!(get(30).controls(), &[NodeId(10)]);
    }

    #[test]
    fn no_tree_focuses_the_window() {
        let update = tree_update(None, "App", 1.0);
        assert_eq!(update.nodes.len(), 1);
        assert_eq!(update.focus, WINDOW_ID);
    }
}
