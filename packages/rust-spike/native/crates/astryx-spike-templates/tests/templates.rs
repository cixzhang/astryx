//! The templates build, respond, and keep their structure.

use astryx_core::contract::{Role, Surface};
use astryx_core::testing::{draw, tap};
use astryx_core::{Input, InputPhase, Key, KeyEvent, Modifiers, Runtime};
use astryx_spike_templates::{settings_dialog, SettingsDialog, ShellNav};

fn find<'a>(
    n: &'a astryx_core::A11yNode,
    role: Role,
    label: &str,
) -> Option<&'a astryx_core::A11yNode> {
    if n.role == role && n.label.as_deref() == Some(label) {
        return Some(n);
    }
    n.children.iter().find_map(|c| find(c, role, label))
}

#[test]
fn shell_nav_opens_the_palette_with_mod_k() {
    let mut rt = Runtime::new(ShellNav::default());
    draw(&mut rt, 1280, 800);
    let tree = rt.accessibility().unwrap();
    assert!(find(&tree, Role::Navigation, "Astryx Studio menu bar").is_some());
    assert!(find(&tree, Role::Tree, "Files").is_some());
    assert!(find(&tree, Role::Dialog, "Search files and commands").is_none());
    rt.key(KeyEvent {
        key: Key::Character("k".into()),
        phase: InputPhase::Pressed,
        modifiers: Modifiers {
            ctrl: true,
            ..Default::default()
        },
        repeat: false,
    });
    draw(&mut rt, 1280, 800);
    let tree = rt.accessibility().unwrap();
    assert!(find(&tree, Role::Dialog, "Search files and commands").is_some());
}

#[test]
fn settings_dialog_switches_panels_and_searches() {
    let mut app = SettingsDialog::default();
    use astryx_core::App;
    let _ = app.update(settings_dialog::Msg::Panel("notifications"));
    let mut rt = Runtime::new(app);
    draw(&mut rt, 1280, 800);
    let tree = rt.accessibility().unwrap();
    assert!(find(&tree, Role::Heading(2), "Notifications").is_some());
    assert!(find(&tree, Role::Switch, "Mentions").is_some());
    // Phone width: one column with section tabs.
    draw(&mut rt, 390, 844);
    let tree = rt.accessibility().unwrap();
    assert!(find(&tree, Role::Navigation, "Settings sections").is_none());
    assert!(find(&tree, Role::Button, "Appearance").is_some());
    let _ = tap(&mut rt, Input::Next);
}
