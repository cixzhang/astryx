//! Wave 9: the chat composer.

use astryx_core::contract::Role;
use astryx_core::{Key, Modifiers};
use astryx_spike_ui::testkit::{find, Rig};
use astryx_spike_ui::*;

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Send(String),
    Stop,
}

#[test]
fn composer_sends_on_enter_and_clears() {
    let mut rig = Rig::<Msg>::sized(|| ChatComposer::new("chat", Msg::Send).into(), 640, 400);
    let input = find(&rig.tree(), Role::TextInput, "Message input")
        .cloned()
        .unwrap();
    assert_eq!(input.value.as_deref(), Some(""));
    let send = find(&rig.tree(), Role::Button, "Send").cloned().unwrap();
    assert!(send.disabled, "nothing to send yet");
    rig.type_text("Hello");
    // Shift+Enter adds a line at the caret.
    rig.key(
        Key::Enter,
        Modifiers {
            shift: true,
            ..Default::default()
        },
    );
    assert_eq!(
        find(&rig.tree(), Role::TextInput, "Message input")
            .unwrap()
            .value
            .as_deref(),
        Some("Hello\n")
    );
    rig.type_text("there");
    assert_eq!(
        find(&rig.tree(), Role::TextInput, "Message input")
            .unwrap()
            .value
            .as_deref(),
        Some("Hello\nthere"),
        "the next text lands after the line break"
    );
    rig.key(Key::Enter, Default::default());
    assert_eq!(rig.messages(), vec![Msg::Send("Hello\nthere".into())]);
    assert_eq!(
        find(&rig.tree(), Role::TextInput, "Message input")
            .unwrap()
            .value
            .as_deref(),
        Some("")
    );
}

#[test]
fn stop_replaces_send_while_streaming() {
    let mut rig = Rig::<Msg>::sized(
        || {
            ChatComposer::new("chat", Msg::Send)
                .is_stop_shown(true)
                .on_stop(Msg::Stop)
                .into()
        },
        640,
        400,
    );
    assert!(find(&rig.tree(), Role::Button, "Send").is_none());
    rig.press(astryx_core::contract::Input::Next);
    rig.press(astryx_core::contract::Input::A);
    assert_eq!(rig.messages(), vec![Msg::Stop]);
}
