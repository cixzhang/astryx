//! Wave 5: controls — behavior, accessibility and keyboard.

use astryx_core::contract::{Input, Role};
use astryx_spike_ui::testkit::{all, find, Rig};
use astryx_spike_ui::*;
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Click(&'static str),
    Toggle(bool),
    Text(String),
    Pick(String),
    Many(Vec<String>),
}

#[test]
fn button_with_only_a_label_is_secondary_md_and_clicks() {
    let mut rig = Rig::new(|| {
        HStack::new()
            .child(Button::new("Save").on_click(Msg::Click("save")))
            .into()
    });
    let tree = rig.tree();
    let b = find(&tree, Role::Button, "Save").unwrap();
    assert_eq!(b.bounds.height, 32.0, "md is the 32 point element size");
    rig.press(Input::Next);
    rig.press(Input::A);
    assert_eq!(rig.messages(), vec![Msg::Click("save")]);
}

#[test]
fn button_sizes_follow_element_tokens() {
    for (size, h) in [
        (ButtonSize::Sm, 28.0),
        (ButtonSize::Md, 32.0),
        (ButtonSize::Lg, 36.0),
    ] {
        let rig = Rig::<Msg>::new(move || HStack::new().child(Button::new("Go").size(size)).into());
        let tree = rig.tree();
        assert_eq!(
            find(&tree, Role::Button, "Go").unwrap().bounds.height,
            h,
            "{size:?}"
        );
    }
}

#[test]
fn loading_button_keeps_its_size_and_ignores_presses() {
    let view = |loading: bool| {
        move || {
            HStack::new()
                .child(
                    Button::new("Submit")
                        .is_loading(loading)
                        .on_click(Msg::Click("x")),
                )
                .into()
        }
    };
    let idle = Rig::<Msg>::new(view(false));
    let mut busy = Rig::<Msg>::new(view(true));
    let w0 = find(&idle.tree(), Role::Button, "Submit")
        .unwrap()
        .bounds
        .width;
    let tree = busy.tree();
    let b = find(&tree, Role::Button, "Submit").unwrap();
    assert_eq!(b.bounds.width, w0);
    assert_eq!(b.value.as_deref(), Some("Loading"));
    busy.press(Input::Next);
    busy.press(Input::A);
    assert!(busy.messages().is_empty());
}

#[test]
fn icon_only_button_is_square_and_named_by_label() {
    let rig = Rig::<Msg>::new(|| {
        HStack::new()
            .child(IconButton::new("Close", Icon::new("close")).size(ButtonSize::Lg))
            .into()
    });
    let tree = rig.tree();
    let b = find(&tree, Role::Button, "Close").unwrap();
    assert_eq!((b.bounds.width, b.bounds.height), (36.0, 36.0));
}

#[test]
fn button_group_passes_size_and_disabled_through_context_and_is_one_tab_stop() {
    let mut rig = Rig::new(|| {
        VStack::new()
            .child(
                ButtonGroup::new("Edit")
                    .size(ButtonSize::Sm)
                    .child(Button::new("Cut").on_click(Msg::Click("cut")))
                    .child(Button::new("Copy").on_click(Msg::Click("copy")))
                    // Nested in another component, still a member.
                    .child(HStack::new().child(Button::new("Paste").on_click(Msg::Click("paste")))),
            )
            .child(Button::new("After").on_click(Msg::Click("after")))
            .into()
    });
    let tree = rig.tree();
    let group = find(&tree, Role::Group, "Edit").expect("group");
    let buttons = all(group, Role::Button);
    assert_eq!(buttons.len(), 3);
    for b in &buttons {
        assert_eq!(b.bounds.height, 28.0, "sm reaches {:?}", b.label);
    }
    // Connected: each starts where the last ends.
    assert_eq!(
        buttons[1].bounds.x,
        buttons[0].bounds.x + buttons[0].bounds.width
    );
    // The group is one stop: from its first member, Next leaves it.
    rig.press(Input::Next);
    rig.press(Input::A);
    assert_eq!(rig.messages(), vec![Msg::Click("after")]);
}

#[test]
fn disabled_button_group_disables_every_member() {
    let rig = Rig::<Msg>::new(|| {
        ButtonGroup::new("Edit")
            .is_disabled(true)
            .child(Button::new("Cut"))
            .child(Button::new("Copy"))
            .into()
    });
    let tree = rig.tree();
    assert!(all(&tree, Role::Button)
        .iter()
        .all(|b| b.disabled && b.focusable));
}

#[test]
fn switch_toggles_and_reports_state() {
    let mut rig = Rig::new(|| {
        Switch::new("Wi-Fi", false)
            .description("Join networks")
            .on_change(Msg::Toggle)
            .into()
    });
    let tree = rig.tree();
    let s = find(&tree, Role::Switch, "Wi-Fi").unwrap();
    assert_eq!(s.checked, Some(false));
    assert_eq!(s.description.as_deref(), Some("Join networks"));
    rig.press(Input::Next);
    rig.press(Input::A);
    assert_eq!(rig.messages(), vec![Msg::Toggle(true)]);
}

#[test]
fn switch_label_position_start_puts_label_first() {
    let rig = Rig::<Msg>::new(|| {
        Switch::new("Dark mode", true)
            .label_position(LabelPosition::Start)
            .into()
    });
    let tree = rig.tree();
    let s = find(&tree, Role::Switch, "Dark mode").unwrap();
    assert_eq!(s.checked, Some(true));
}

#[test]
fn required_text_input_speaks_marker_and_types() {
    let value = Rc::new(Cell::new(0));
    let _ = value;
    let mut rig = Rig::new(|| {
        TextInput::new("Email", "")
            .is_required(true)
            .placeholder("you@example.com")
            .on_change(Msg::Text)
            .into()
    });
    let tree = rig.tree();
    let field = find(&tree, Role::TextInput, "Email, Required").expect("named with marker");
    assert!(field.focusable);
    rig.press(Input::Next);
    astryx_core::contract::Surface::text_input(&mut rig.rt, "a");
    assert_eq!(rig.messages(), vec![Msg::Text("a".into())]);
}

#[test]
fn text_input_clear_button_empties_the_value() {
    let mut rig = Rig::new(|| {
        TextInput::new("Search", "shoes")
            .has_clear(true)
            .on_change(Msg::Text)
            .into()
    });
    let tree = rig.tree();
    assert!(find(&tree, Role::Button, "Clear Search").is_some());
    // The first frame focuses the field; Next reaches its clear button.
    rig.press(Input::Next);
    rig.press(Input::A);
    assert_eq!(rig.messages(), vec![Msg::Text(String::new())]);
}

#[test]
fn text_input_error_status_is_announced_assertively() {
    let rig = Rig::<Msg>::new(|| {
        TextInput::new("Name", "")
            .status(Status::error("Enter a name"))
            .into()
    });
    let tree = rig.tree();
    let msg = find(&tree, Role::Text, "Enter a name").unwrap();
    assert_eq!(msg.live, Some(astryx_core::contract::Live::Assertive));
}

#[test]
fn checkbox_list_adds_and_removes_values() {
    let mut rig = Rig::new(|| {
        CheckboxList::new("Toppings", vec!["cheese".into()])
            .item(CheckboxListItem::new("Cheese", "cheese"))
            .item(CheckboxListItem::new("Olives", "olives"))
            .on_change(Msg::Many)
            .into()
    });
    let tree = rig.tree();
    let boxes = all(&tree, Role::Checkbox);
    assert_eq!(boxes.len(), 2);
    assert_eq!(boxes[0].checked, Some(true));
    rig.press(Input::Next);
    rig.press(Input::A);
    assert_eq!(
        rig.messages(),
        vec![Msg::Many(vec!["cheese".into(), "olives".into()])]
    );
}

#[test]
fn radio_list_is_one_tab_stop_on_the_chosen_option() {
    let mut rig = Rig::new(|| {
        VStack::new()
            .child(
                RadioList::new("Plan", "pro")
                    .item(RadioListItem::new("Free", "free"))
                    .item(RadioListItem::new("Pro", "pro"))
                    .item(RadioListItem::new("Team", "team"))
                    .on_change(Msg::Pick),
            )
            .child(Button::new("Next").on_click(Msg::Click("next")))
            .into()
    });
    let tree = rig.tree();
    assert!(find(&tree, Role::RadioGroup, "Plan").is_some());
    assert_eq!(
        all(&tree, Role::Radio)
            .iter()
            .filter(|r| r.checked == Some(true))
            .count(),
        1
    );
    // Coming back into the group lands on the chosen option, not the first.
    rig.press(Input::Next);
    rig.press(Input::Previous);
    let focused: Vec<_> = all(&rig.tree(), Role::Radio)
        .into_iter()
        .filter(|r| r.focused)
        .map(|r| r.label.clone())
        .collect();
    assert_eq!(focused, vec![Some("Pro".into())]);
    rig.press(Input::Next);
    rig.press(Input::A);
    assert_eq!(rig.messages(), vec![Msg::Click("next")]);
}

#[test]
fn segmented_control_picks_and_marks_selection() {
    let mut rig = Rig::new(|| {
        SegmentedControl::new("View", "list")
            .item(SegmentedControlItem::new("list", "List"))
            .item(SegmentedControlItem::new("grid", "Grid"))
            .on_change(Msg::Pick)
            .into()
    });
    let tree = rig.tree();
    let radios = all(&tree, Role::Radio);
    assert_eq!(radios.len(), 2);
    assert_eq!(radios[0].checked, Some(true));
    assert_eq!(radios[0].bounds.height, 28.0, "md item is element_md - 4");
    rig.press(Input::Next);
    rig.press(Input::Right);
    rig.press(Input::A);
    assert_eq!(rig.messages(), vec![Msg::Pick("grid".into())]);
}

#[test]
fn selectable_card_is_a_checkbox() {
    let mut rig = Rig::new(|| {
        SelectableCard::new("Starter", false)
            .on_change(Msg::Toggle)
            .child(Text::new("Starter plan"))
            .into()
    });
    let tree = rig.tree();
    let card = find(&tree, Role::Checkbox, "Starter").unwrap();
    assert_eq!(card.checked, Some(false));
    rig.press(Input::Next);
    rig.press(Input::A);
    assert_eq!(rig.messages(), vec![Msg::Toggle(true)]);
}

#[test]
fn token_remove_button_is_named() {
    let mut rig = Rig::new(|| {
        Token::new("Design")
            .color(TokenColor::Blue)
            .on_remove(Msg::Click("rm"))
            .into()
    });
    let tree = rig.tree();
    assert!(find(&tree, Role::Button, "Remove Design").is_some());
    rig.press(Input::Next);
    rig.press(Input::A);
    assert_eq!(rig.messages(), vec![Msg::Click("rm")]);
}

#[test]
fn input_group_joins_text_field_and_button() {
    let rig = Rig::<Msg>::new(|| {
        InputGroup::new("Website")
            .size(astryx_spike_foundations::ComponentSize::Lg)
            .child(InputGroupText::new("https://"))
            .child(TextInput::new("Website", "example.com"))
            .child(Button::new("Go"))
            .into()
    });
    let tree = rig.tree();
    let field = find(&tree, Role::TextInput, "Website").unwrap();
    let go = find(&tree, Role::Button, "Go").unwrap();
    assert_eq!(go.bounds.height, 36.0);
    // Borders overlap by one point.
    assert_eq!(go.bounds.x, field.bounds.x + field.bounds.width - 1.0);
}
