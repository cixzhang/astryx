//! Mouse and trackpad: hit testing on the accessibility tree, wheel deltas,
//! and cursor icons.

use astryx_core::{A11yNode, Point, Role};
use winit::event::MouseScrollDelta;
use winit::window::CursorIcon;

/// Points one wheel notch scrolls. winit reports a notch as one line;
/// browsers scroll about three 16-point lines per notch.
pub const POINTS_PER_LINE: f32 = 48.0;

/// The deepest focusable, enabled node whose bounds contain `(x, y)`, in
/// logical points. Later siblings win, as they paint on top.
pub fn hit_test(node: &A11yNode, x: f32, y: f32) -> Option<&A11yNode> {
    let b = &node.bounds;
    let inside = x >= b.x && x < b.x + b.width && y >= b.y && y < b.y + b.height;
    let child = node.children.iter().rev().find_map(|c| hit_test(c, x, y));
    child.or((inside && node.focusable && !node.disabled).then_some(node))
}

/// A wheel or trackpad delta as `Surface::scroll` takes it: logical points,
/// positive y scrolls content up. winit's positive values mean the wheel
/// turned away from the person, which reveals content above.
pub fn scroll_delta(delta: MouseScrollDelta, scale: f64) -> Point {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => Point {
            x: -x * POINTS_PER_LINE,
            y: -y * POINTS_PER_LINE,
        },
        MouseScrollDelta::PixelDelta(p) => Point {
            x: -(p.x / scale) as f32,
            y: -(p.y / scale) as f32,
        },
    }
}

/// The cursor over a node: a text cursor over fields, a pointer over other
/// controls, the default elsewhere.
pub fn cursor_for(node: Option<&A11yNode>) -> CursorIcon {
    match node.map(|n| n.role) {
        Some(Role::TextInput) => CursorIcon::Text,
        Some(_) => CursorIcon::Pointer,
        None => CursorIcon::Default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astryx_core::Rect;
    use winit::dpi::PhysicalPosition;

    fn node(id: u64, role: Role, bounds: Rect, focusable: bool) -> A11yNode {
        A11yNode {
            rtl: false,
            id,
            role,
            label: None,
            description: None,
            value: None,
            bounds,
            focused: false,
            disabled: false,
            checked: None,
            selected: None,
            focusable,
            text_selection: None,
            secure: false,
            live: None,
            children: Vec::new(),
        }
    }

    fn tree() -> A11yNode {
        let mut item = node(2, Role::ListItem, Rect::new(10.0, 10.0, 100.0, 40.0), true);
        item.children = vec![node(
            3,
            Role::Button,
            Rect::new(70.0, 15.0, 30.0, 30.0),
            true,
        )];
        let mut off = node(4, Role::Button, Rect::new(10.0, 60.0, 100.0, 40.0), true);
        off.disabled = true;
        let mut root = node(1, Role::Group, Rect::new(0.0, 0.0, 300.0, 300.0), false);
        root.children = vec![
            item,
            off,
            node(
                5,
                Role::TextInput,
                Rect::new(10.0, 110.0, 100.0, 30.0),
                true,
            ),
        ];
        root
    }

    #[test]
    fn hits_the_deepest_enabled_focusable_node() {
        let t = tree();
        assert_eq!(hit_test(&t, 80.0, 20.0).map(|n| n.id), Some(3));
        assert_eq!(hit_test(&t, 20.0, 20.0).map(|n| n.id), Some(2));
        assert_eq!(hit_test(&t, 20.0, 70.0).map(|n| n.id), None);
        assert_eq!(hit_test(&t, 250.0, 250.0).map(|n| n.id), None);
    }

    #[test]
    fn cursors_follow_the_hit_role() {
        let t = tree();
        assert_eq!(cursor_for(hit_test(&t, 20.0, 120.0)), CursorIcon::Text);
        assert_eq!(cursor_for(hit_test(&t, 20.0, 20.0)), CursorIcon::Pointer);
        assert_eq!(cursor_for(hit_test(&t, 250.0, 250.0)), CursorIcon::Default);
    }

    #[test]
    fn wheel_deltas_become_points() {
        // One notch toward the person scrolls content up by one line.
        let down = scroll_delta(MouseScrollDelta::LineDelta(0.0, -1.0), 2.0);
        assert_eq!((down.x, down.y), (0.0, POINTS_PER_LINE));
        let pixels = scroll_delta(
            MouseScrollDelta::PixelDelta(PhysicalPosition::new(10.0, 30.0)),
            2.0,
        );
        assert_eq!((pixels.x, pixels.y), (-5.0, -15.0));
    }
}
