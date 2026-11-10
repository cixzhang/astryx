//! Builds the accessibility tree a frame exposes.

use crate::contract::{A11yNode, A11yState, Rect, Role};
use crate::element::{A11yRefs, Node};
use std::collections::HashMap;

/// The tree for `nodes`: a root group holding every node with semantics, in
/// reading order. Nodes without semantics pass their children up. A node
/// with a label speaks for its subtree, as an ARIA button does for its
/// content, so its descendants are left out unless they are focus stops.
pub(crate) fn tree<Msg>(
    nodes: &[Node<Msg>],
    rects: &[Rect],
    focused: Option<u64>,
    modal: Option<usize>,
) -> Option<A11yNode> {
    let root = nodes.first()?;
    let names: HashMap<&str, u64> = nodes
        .iter()
        .filter_map(|n| Some((n.name.as_deref()?, n.id)))
        .collect();
    let cx = Cx {
        nodes,
        rects,
        focused,
        names: &names,
    };
    let mut out = A11yNode {
        rtl: root.rtl,
        text_selection: None,
        secure: false,
        id: root.id,
        role: Role::Group,
        label: None,
        value: None,
        bounds: rects[0],
        focused: false,
        disabled: false,
        checked: None,
        description: None,
        selected: None,
        focusable: false,
        live: None,
        state: A11yState::default(),
        children: Vec::new(),
    };
    for &c in &root.children {
        // Behind a modal layer the page is inert: left out of the tree, as
        // the web's `inert` does. Layers above the modal one stay.
        if modal.is_some_and(|m| (c as usize) < m) {
            continue;
        }
        collect(&cx, c as usize, false, &mut out.children);
    }
    Some(out)
}

/// Whether a labeled node of `role` speaks for its subtree, as ARIA roles
/// named from content do. Containers and landmarks never do: their
/// children stay in the tree under the container's own name.
fn names_from_content(role: Role) -> bool {
    !matches!(
        role,
        Role::Group
            | Role::List
            | Role::RadioGroup
            | Role::ListBox
            | Role::Dialog
            | Role::AlertDialog
            | Role::Menu
            | Role::TabList
            | Role::TabPanel
            | Role::Toolbar
            | Role::Grid
            | Role::Table
            | Role::RowGroup
            | Role::Row
            | Role::Tree
            | Role::Navigation
            | Role::Banner
            | Role::Main
            | Role::Complementary
            | Role::ContentInfo
            | Role::Region
            | Role::Search
            | Role::Form
    )
}

struct Cx<'a, Msg> {
    nodes: &'a [Node<Msg>],
    rects: &'a [Rect],
    focused: Option<u64>,
    names: &'a HashMap<&'a str, u64>,
}

/// `state` with its relations filled from `refs`.
fn resolve(state: &A11yState, refs: &A11yRefs, names: &HashMap<&str, u64>) -> A11yState {
    let id = |name: &String| names.get(name.as_str()).copied();
    let mut out = state.clone();
    if out.active_descendant.is_none() {
        out.active_descendant = refs.active_descendant.as_ref().and_then(id);
    }
    if out.error_message.is_none() {
        out.error_message = refs.error_message.as_ref().and_then(id);
    }
    out.controls.extend(refs.controls.iter().filter_map(id));
    out.labelled_by
        .extend(refs.labelled_by.iter().filter_map(id));
    out.described_by
        .extend(refs.described_by.iter().filter_map(id));
    out
}

fn collect<Msg>(cx: &Cx<Msg>, ix: usize, covered: bool, out: &mut Vec<A11yNode>) {
    let (nodes, rects, focused) = (cx.nodes, cx.rects, cx.focused);
    let node = &nodes[ix];
    let sem = match &node.sem {
        Some(sem) if !covered || node.interact.is_some() || sem.live.is_some() => sem,
        _ => {
            for &c in &node.children {
                collect(cx, c as usize, covered, out);
            }
            return;
        }
    };
    let mut a = A11yNode {
        rtl: node.rtl,
        text_selection: None,
        secure: sem.secure,
        id: node.id,
        role: sem.role,
        label: sem.label.clone(),
        value: sem.value.clone(),
        bounds: rects[ix],
        focused: node.interact.is_some() && focused == Some(node.id),
        disabled: sem.disabled,
        checked: sem.checked,
        description: sem.description.clone(),
        selected: sem.selected,
        focusable: node.interact.is_some(),
        live: sem.live,
        state: resolve(&sem.state, &sem.refs, cx.names),
        children: Vec::new(),
    };
    let covers = sem.label.is_some() && names_from_content(sem.role);
    for &c in &node.children {
        collect(cx, c as usize, covers, &mut a.children);
    }
    out.push(a);
}
