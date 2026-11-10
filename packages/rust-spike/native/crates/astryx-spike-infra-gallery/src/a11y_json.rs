//! The accessibility tree as JSON, for the QA harness.

use astryx_core::contract::{A11yNode, Role};
use serde_json::{json, Value};

pub fn role_name(role: Role) -> String {
    match role {
        Role::Heading(level) => format!("heading{level}"),
        other => {
            let s = format!("{other:?}");
            let mut out = String::new();
            for (i, ch) in s.chars().enumerate() {
                if ch.is_uppercase() && i > 0 {
                    out.push('_');
                }
                out.push(ch.to_ascii_lowercase());
            }
            out
        }
    }
}

pub fn to_json(node: &A11yNode) -> Value {
    let b = node.bounds;
    json!({
        "role": role_name(node.role),
        "label": node.label,
        "value": node.value,
        "description": node.description,
        "bounds": [b.x, b.y, b.width, b.height],
        "focused": node.focused,
        "focusable": node.focusable,
        "disabled": node.disabled,
        "checked": node.checked,
        "selected": node.selected,
        "rtl": node.rtl,
        "children": node.children.iter().map(to_json).collect::<Vec<_>>(),
    })
}
