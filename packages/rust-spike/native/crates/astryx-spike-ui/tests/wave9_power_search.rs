//! Stretch: PowerSearch.

use astryx_core::contract::Role;
use astryx_core::Key;
use astryx_spike_ui::testkit::{all, find, Rig};
use astryx_spike_ui::*;

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Filters(Vec<PowerSearchFilter>, FilterChange, usize),
}

fn config() -> PowerSearchConfig {
    PowerSearchConfig {
        fields: vec![
            PowerSearchField::new(
                "status",
                "Status",
                vec![PowerSearchOperator::new(
                    "is",
                    "is",
                    OperatorValue::Enum(vec![
                        ("open".into(), "Open".into()),
                        ("closed".into(), "Closed".into()),
                    ]),
                )],
            ),
            PowerSearchField::new(
                "priority",
                "Priority",
                vec![
                    PowerSearchOperator::new("gt", "greater than", OperatorValue::Integer),
                    PowerSearchOperator::new("lt", "less than", OperatorValue::Integer),
                ],
            ),
            PowerSearchField::new(
                "text",
                "Text",
                vec![PowerSearchOperator::new(
                    "contains",
                    "contains",
                    OperatorValue::String,
                )],
            ),
        ],
        content_search_field_key: Some("text".into()),
    }
}

#[test]
fn typing_an_enum_value_adds_a_filter() {
    let mut rig =
        Rig::<Msg>::new(|| PowerSearch::new("ps", config(), Vec::new(), Msg::Filters).into());
    rig.type_text("op");
    let opts: Vec<String> = all(&rig.tree(), Role::ListBoxOption)
        .iter()
        .filter_map(|o| o.label.clone())
        .collect();
    assert_eq!(opts, ["Status is Open", "Text contains \u{201c}op\u{201d}"]);
    rig.key(Key::Enter, Default::default());
    assert_eq!(
        rig.messages(),
        vec![Msg::Filters(
            vec![PowerSearchFilter {
                field: "status".into(),
                operator: "is".into(),
                value: FilterValue::Enum("open".into()),
            }],
            FilterChange::Add,
            0
        )]
    );
}

#[test]
fn field_then_operator_then_value() {
    let mut rig =
        Rig::<Msg>::new(|| PowerSearch::new("ps", config(), Vec::new(), Msg::Filters).into());
    rig.type_text("pri");
    rig.key(Key::Enter, Default::default()); // Filter by Priority
    let opts: Vec<String> = all(&rig.tree(), Role::ListBoxOption)
        .iter()
        .filter_map(|o| o.label.clone())
        .collect();
    assert_eq!(opts, ["Priority greater than", "Priority less than"]);
    rig.key(Key::Enter, Default::default()); // greater than
    rig.type_text("3");
    rig.key(Key::Enter, Default::default());
    assert_eq!(
        rig.messages(),
        vec![Msg::Filters(
            vec![PowerSearchFilter {
                field: "priority".into(),
                operator: "gt".into(),
                value: FilterValue::Integer(3),
            }],
            FilterChange::Add,
            0
        )]
    );
}

#[test]
fn backspace_on_empty_removes_the_last_filter() {
    let mut rig = Rig::<Msg>::new(|| {
        PowerSearch::new(
            "ps",
            config(),
            vec![PowerSearchFilter {
                field: "status".into(),
                operator: "is".into(),
                value: FilterValue::Enum("closed".into()),
            }],
            Msg::Filters,
        )
        .into()
    });
    assert!(find(&rig.tree(), Role::Button, "Remove Status is Closed").is_some());
    // Focus the field (after the token's remove button).
    rig.press(astryx_core::contract::Input::Next);
    rig.key(Key::Backspace, Default::default());
    assert_eq!(
        rig.messages(),
        vec![Msg::Filters(Vec::new(), FilterChange::Remove, 0)]
    );
}
