//! Builds the accessibility tree a frame exposes.

use crate::contract::{A11yNode, Rect, Role};
use crate::element::Node;

/// The tree for `nodes`: a root group holding every node with semantics, in
/// reading order. Nodes without semantics pass their children up. A node
/// with a label speaks for its subtree, as an ARIA button does for its
/// content, so its descendants are left out unless they are focus stops.
pub(crate) fn tree<Msg>(
    nodes: &[Node<Msg>],
    rects: &[Rect],
    focused: Option<u64>,
) -> Option<A11yNode> {
    let root = nodes.first()?;
    let mut out = A11yNode {
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
        children: Vec::new(),
    };
    for &c in &root.children {
        collect(nodes, rects, focused, c as usize, false, &mut out.children);
    }
    Some(out)
}

fn collect<Msg>(
    nodes: &[Node<Msg>],
    rects: &[Rect],
    focused: Option<u64>,
    ix: usize,
    covered: bool,
    out: &mut Vec<A11yNode>,
) {
    let node = &nodes[ix];
    let sem = match &node.sem {
        Some(sem) if !covered || node.interact.is_some() || sem.live.is_some() => sem,
        _ => {
            for &c in &node.children {
                collect(nodes, rects, focused, c as usize, covered, out);
            }
            return;
        }
    };
    let mut a = A11yNode {
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
        children: Vec::new(),
    };
    for &c in &node.children {
        collect(
            nodes,
            rects,
            focused,
            c as usize,
            sem.label.is_some(),
            &mut a.children,
        );
    }
    out.push(a);
}
