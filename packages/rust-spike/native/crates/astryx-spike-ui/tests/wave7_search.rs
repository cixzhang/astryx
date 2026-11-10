//! Wave 7: the combobox family.

use astryx_core::contract::Role;
use astryx_core::Key;
use astryx_spike_ui::testkit::{all, find, Rig};
use astryx_spike_ui::*;

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Pick(Option<Entry>),
    Values(Vec<String>),
    Tokens(Vec<Entry>),
    Run(Entry),
    Open(bool),
}

fn fruits() -> SearchSource<Entry> {
    static_source(
        ["Apple", "Apricot", "Banana", "Blueberry", "Cherry"]
            .iter()
            .map(|f| Entry::new(f.to_lowercase(), *f))
            .collect(),
    )
}

#[test]
fn static_source_ranks_word_starts_first() {
    let s = static_source(vec![
        Entry::new("a", "Pineapple"),
        Entry::new("b", "Apple pie"),
    ]);
    let got: Vec<String> = s("app").into_iter().map(|e| e.label).collect();
    assert_eq!(got, ["Apple pie", "Pineapple"]);
}

#[test]
fn typeahead_suggests_and_picks_with_the_keyboard() {
    let mut rig = Rig::<Msg>::new(|| {
        Typeahead::new("fruit", "Fruit", fruits(), None)
            .on_change(Msg::Pick)
            .into()
    });
    let field = find(&rig.tree(), Role::ComboBox, "Fruit")
        .cloned()
        .expect("combobox");
    assert_eq!(field.state.expanded, Some(false));
    rig.type_text("ap");
    let tree = rig.tree();
    let options: Vec<String> = all(&tree, Role::ListBoxOption)
        .iter()
        .filter_map(|o| o.label.clone())
        .collect();
    assert_eq!(options, ["Apple", "Apricot"]);
    assert_eq!(
        find(&tree, Role::ComboBox, "Fruit").unwrap().state.expanded,
        Some(true)
    );
    rig.key(Key::Down, Default::default());
    rig.key(Key::Down, Default::default());
    rig.key(Key::Enter, Default::default());
    assert_eq!(
        rig.messages(),
        vec![Msg::Pick(Some(Entry::new("apricot", "Apricot")))]
    );
    assert!(
        all(&rig.tree(), Role::ListBox).is_empty(),
        "closes after picking"
    );
}

#[test]
fn escape_closes_the_suggestions() {
    let mut rig = Rig::<Msg>::new(|| {
        Typeahead::new("f", "Fruit", fruits(), None)
            .on_change(Msg::Pick)
            .into()
    });
    rig.type_text("b");
    assert_eq!(all(&rig.tree(), Role::ListBoxOption).len(), 2);
    rig.key(Key::Escape, Default::default());
    assert!(all(&rig.tree(), Role::ListBoxOption).is_empty());
}

fn options() -> Vec<SelectorOption> {
    vec![
        SelectorOption::new("s", "Small"),
        SelectorOption::new("m", "Medium"),
        SelectorOption::new("l", "Large").is_disabled(true),
    ]
}

#[test]
fn selector_opens_from_the_keyboard_and_picks() {
    let mut rig = Rig::<Msg>::new(|| {
        Selector::single("size", "Size", options(), Some("s".into()))
            .on_change(Msg::Values)
            .into()
    });
    let trigger = find(&rig.tree(), Role::ComboBox, "Size").cloned().unwrap();
    assert_eq!(trigger.value.as_deref(), Some("Small"));
    rig.key(Key::Down, Default::default());
    assert_eq!(all(&rig.tree(), Role::ListBoxOption).len(), 3);
    rig.key(Key::Down, Default::default());
    rig.key(Key::Enter, Default::default());
    assert_eq!(rig.messages(), vec![Msg::Values(vec!["m".into()])]);
    assert!(all(&rig.tree(), Role::ListBoxOption).is_empty());
}

#[test]
fn multi_selector_toggles_and_stays_open() {
    let mut rig = Rig::<Msg>::new(|| {
        Selector::multiple("sizes", "Sizes", options(), vec!["s".into()])
            .on_change(Msg::Values)
            .into()
    });
    assert_eq!(
        find(&rig.tree(), Role::ComboBox, "Sizes")
            .unwrap()
            .value
            .as_deref(),
        Some("1 selected")
    );
    rig.key(Key::Down, Default::default());
    let tree = rig.tree();
    assert!(all(&tree, Role::ListBox)[0].state.multiselectable);
    rig.key(Key::Down, Default::default());
    rig.key(Key::Space, Default::default());
    assert_eq!(
        rig.messages(),
        vec![Msg::Values(vec!["s".into(), "m".into()])]
    );
    assert!(
        !all(&rig.tree(), Role::ListBoxOption).is_empty(),
        "stays open"
    );
}

#[test]
fn tokenizer_adds_and_removes_tokens() {
    let mut rig = Rig::<Msg>::new(|| {
        Tokenizer::new(
            "tags",
            "Tags",
            fruits(),
            vec![Entry::new("apple", "Apple")],
            Msg::Tokens,
        )
        .into()
    });
    let tree = rig.tree();
    assert!(find(&tree, Role::Button, "Remove Apple").is_some());
    // Focus the field (after the remove button) and add Banana.
    rig.press(astryx_core::contract::Input::Next);
    rig.type_text("ban");
    rig.key(Key::Down, Default::default());
    rig.key(Key::Enter, Default::default());
    assert_eq!(
        rig.messages(),
        vec![Msg::Tokens(vec![
            Entry::new("apple", "Apple"),
            Entry::new("banana", "Banana")
        ])]
    );
}

#[test]
fn command_palette_runs_the_active_result() {
    let mut rig = Rig::<Msg>::sized(
        || {
            CommandPalette::new("cmd", true, Msg::Open, fruits())
                .label("Search commands")
                .on_value_change(Msg::Run)
                .into()
        },
        1024,
        768,
    );
    assert!(find(&rig.tree(), Role::Dialog, "Search commands").is_some());
    rig.type_text("ch");
    rig.key(Key::Enter, Default::default());
    assert_eq!(
        rig.messages(),
        vec![Msg::Run(Entry::new("cherry", "Cherry"))]
    );
}
