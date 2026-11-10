//! Interactive pages for the behavior and text-input checks: the gallery
//! keeps their state, so Space toggles a switch, typing changes a field,
//! and sorting or selecting changes a table. `playground-0001` to `-0004`.

use crate::Msg;
use astryx_core::{Element, Space};
use astryx_spike_ui::*;
use std::collections::HashSet;
use std::rc::Rc;

/// The playground's state, kept by the gallery app.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayState {
    pub notifications: bool,
    pub plan: String,
    pub name: String,
    pub fruit: Option<Entry>,
    pub sort: Vec<SortEntry>,
    pub selected: HashSet<String>,
}

impl Default for PlayState {
    fn default() -> Self {
        PlayState {
            notifications: false,
            plan: "free".into(),
            name: String::new(),
            fruit: None,
            sort: Vec::new(),
            selected: HashSet::new(),
        }
    }
}

/// A change to the playground's state.
#[derive(Clone, Debug, PartialEq)]
pub enum PlayMsg {
    Notifications(bool),
    Plan(String),
    Name(String),
    Fruit(Option<Entry>),
    Sort(Vec<SortEntry>),
    Select(HashSet<String>),
}

impl PlayState {
    pub fn update(&mut self, m: PlayMsg) {
        match m {
            PlayMsg::Notifications(on) => self.notifications = on,
            PlayMsg::Plan(p) => self.plan = p,
            PlayMsg::Name(n) => self.name = n,
            PlayMsg::Fruit(f) => self.fruit = f,
            PlayMsg::Sort(s) => self.sort = s,
            PlayMsg::Select(s) => self.selected = s,
        }
    }
}

/// A line that reads the state back, for checks that look at text.
fn readout(label: &str, value: String) -> Element<Msg> {
    Text::new(format!("{label}: {value}"))
        .r#type(TextType::Supporting)
        .color(TextColor::Secondary)
        .into()
}

#[derive(Clone)]
struct Person {
    id: u32,
    name: &'static str,
    role: &'static str,
    age: u32,
}

fn people() -> Rc<Vec<Person>> {
    Rc::new(vec![
        Person {
            id: 1,
            name: "Ada Lovelace",
            role: "Mathematician",
            age: 36,
        },
        Person {
            id: 2,
            name: "Grace Hopper",
            role: "Rear admiral",
            age: 85,
        },
        Person {
            id: 3,
            name: "Alan Turing",
            role: "Computer scientist",
            age: 41,
        },
        Person {
            id: 4,
            name: "Katherine Johnson",
            role: "Mathematician",
            age: 101,
        },
        Person {
            id: 5,
            name: "Edsger Dijkstra",
            role: "Computer scientist",
            age: 72,
        },
    ])
}

/// The playground page `page` (1 to 4).
pub fn page(page: u32, s: &PlayState) -> Element<Msg> {
    let m = |p: PlayMsg| Msg::Play(Box::new(p));
    match page {
        // Switch and RadioList.
        1 => VStack::new()
            .gap(Space::S4)
            .child(
                Switch::new("Email notifications", s.notifications)
                    .on_change(move |on| m(PlayMsg::Notifications(on))),
            )
            .child(readout(
                "Notifications",
                if s.notifications {
                    "on".into()
                } else {
                    "off".into()
                },
            ))
            .child(
                RadioList::new("Plan", s.plan.clone())
                    .item(RadioListItem::new("Free", "free"))
                    .item(RadioListItem::new("Pro", "pro"))
                    .item(RadioListItem::new("Team", "team"))
                    .on_change(move |v: String| m(PlayMsg::Plan(v))),
            )
            .child(readout("Plan", s.plan.clone()))
            .into(),
        // TextInput.
        2 => VStack::new()
            .gap(Space::S4)
            .child(
                TextInput::new("Name", s.name.clone())
                    .placeholder("Your name")
                    .has_clear(true)
                    .on_change(move |v| m(PlayMsg::Name(v))),
            )
            .child(readout("Name", format!("\u{201c}{}\u{201d}", s.name)))
            .into(),
        // Typeahead.
        3 => {
            let source = static_source(
                [
                    "Apple",
                    "Apricot",
                    "Banana",
                    "Blueberry",
                    "Cherry",
                    "Grape",
                    "Mango",
                    "Orange",
                ]
                .iter()
                .map(|f| Entry::new(f.to_lowercase(), *f))
                .collect(),
            );
            VStack::new()
                .gap(Space::S4)
                .child(
                    Typeahead::new("fruit", "Fruit", source, s.fruit.clone())
                        .placeholder("Search fruit")
                        .on_change(move |f| m(PlayMsg::Fruit(f))),
                )
                .child(readout(
                    "Fruit",
                    s.fruit.as_ref().map_or("none".into(), |f| f.label.clone()),
                ))
                .into()
        }
        // Table with sorting and selection.
        _ => {
            let cols = vec![
                TableColumn::new("name", "Name")
                    .value(|p: &Person| p.name)
                    .sortable()
                    .is_row_header(true),
                TableColumn::new("role", "Role").value(|p: &Person| p.role),
                TableColumn::new("age", "Age")
                    .value(|p: &Person| p.age)
                    .sortable()
                    .align(ColumnAlign::End),
            ];
            let mut sel: Vec<String> = s.selected.iter().cloned().collect();
            sel.sort();
            VStack::new()
                .gap(Space::S4)
                .child(
                    Table::new(people(), cols, |p| p.id.to_string())
                        .label("People")
                        .sortable(Sortable::new(s.sort.clone(), move |v| m(PlayMsg::Sort(v))))
                        .plugin(Selection::new(s.selected.clone(), move |v| {
                            m(PlayMsg::Select(v))
                        })),
                )
                .child(readout(
                    "Sort",
                    s.sort.first().map_or("none".into(), |e| {
                        format!("{} {:?}", e.sort_key, e.direction).to_lowercase()
                    }),
                ))
                .child(readout(
                    "Selected",
                    if sel.is_empty() {
                        "none".into()
                    } else {
                        sel.join(", ")
                    },
                ))
                .into()
        }
    }
}
