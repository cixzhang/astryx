//! Wave 7: Table and its plugins.

use astryx_core::contract::{Input, Role, Sort};
use astryx_spike_ui::testkit::{all, find, Rig};
use astryx_spike_ui::*;
use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Sort(Vec<SortEntry>),
    Select(HashSet<String>),
    Filter(String, Option<String>),
    Page(u32),
}

#[derive(Clone)]
struct Person {
    id: u32,
    name: &'static str,
    age: u32,
    team: &'static str,
}

fn people() -> Rc<Vec<Person>> {
    Rc::new(vec![
        Person {
            id: 1,
            name: "Grace",
            age: 85,
            team: "Navy",
        },
        Person {
            id: 2,
            name: "Ada",
            age: 36,
            team: "Math",
        },
        Person {
            id: 3,
            name: "Alan",
            age: 41,
            team: "Math",
        },
        Person {
            id: 4,
            name: "Katherine",
            age: 101,
            team: "NASA",
        },
    ])
}

fn columns() -> Vec<TableColumn<Person, Msg>> {
    vec![
        TableColumn::new("name", "Name")
            .value(|p: &Person| p.name)
            .sortable()
            .filter("name")
            .is_row_header(true),
        TableColumn::new("age", "Age")
            .value(|p: &Person| p.age)
            .sortable()
            .align(ColumnAlign::End),
        TableColumn::new("team", "Team").value(|p: &Person| p.team),
    ]
}

fn row_names(rig: &Rig<Msg>) -> Vec<String> {
    let tree = rig.tree();
    all(&tree, Role::RowHeader)
        .iter()
        .filter_map(|n| n.label.clone())
        .collect()
}

#[test]
fn table_has_header_rows_and_cells() {
    let rig = Rig::<Msg>::sized(
        || {
            Table::new(people(), columns(), |p| p.id.to_string())
                .label("People")
                .into()
        },
        800,
        600,
    );
    let tree = rig.tree();
    assert!(find(&tree, Role::Table, "People").is_some());
    assert_eq!(all(&tree, Role::ColumnHeader).len(), 3);
    assert_eq!(all(&tree, Role::Row).len(), 5);
    assert_eq!(row_names(&rig), ["Grace", "Ada", "Alan", "Katherine"]);
    // Proportional columns share the width equally.
    let headers = all(&tree, Role::ColumnHeader);
    let w: Vec<f32> = headers.iter().map(|h| h.bounds.width).collect();
    assert!(
        (w[0] - w[1]).abs() <= 1.0,
        "{w:?} {:?}",
        all(&tree, Role::Table)[0].bounds
    );
}

#[test]
fn sorting_orders_rows_and_cycles_direction() {
    let sorted = |sort: Vec<SortEntry>| {
        Rig::<Msg>::sized(
            move || {
                Table::new(people(), columns(), |p| p.id.to_string())
                    .sortable(Sortable::new(sort.clone(), Msg::Sort))
                    .into()
            },
            800,
            600,
        )
    };
    let rig = sorted(vec![SortEntry {
        sort_key: "age".into(),
        direction: SortDirection::Descending,
    }]);
    assert_eq!(row_names(&rig), ["Katherine", "Grace", "Alan", "Ada"]);
    let tree = rig.tree();
    let age = find(&tree, Role::ColumnHeader, "Age").unwrap();
    assert_eq!(age.state.sort, Some(Sort::Descending));
    let mut rig = sorted(Vec::new());
    assert_eq!(row_names(&rig), ["Grace", "Ada", "Alan", "Katherine"]);
    // Focus starts on the first sort button (Name); press it.
    rig.press(Input::A);
    assert_eq!(
        rig.messages(),
        vec![Msg::Sort(vec![SortEntry {
            sort_key: "name".into(),
            direction: SortDirection::Ascending
        }])]
    );
    let rig = sorted(vec![SortEntry {
        sort_key: "name".into(),
        direction: SortDirection::Ascending,
    }]);
    assert_eq!(row_names(&rig), ["Ada", "Alan", "Grace", "Katherine"]);
}

#[test]
fn selection_adds_a_checkbox_column_and_marks_rows() {
    let mut rig = Rig::<Msg>::sized(
        || {
            Table::new(people(), columns(), |p| p.id.to_string())
                .plugin(Selection::new(
                    ["2".to_string()].into_iter().collect(),
                    Msg::Select,
                ))
                .into()
        },
        800,
        600,
    );
    let tree = rig.tree();
    let boxes = all(&tree, Role::Checkbox);
    assert_eq!(boxes.len(), 5, "select-all plus one per row");
    let rows = all(&tree, Role::Row);
    assert_eq!(rows.iter().filter(|r| r.selected == Some(true)).count(), 1);
    // Select all from the header checkbox (first focus stop).
    rig.press(Input::A);
    let msgs = rig.messages();
    let Msg::Select(all_ids) = &msgs[0] else {
        panic!()
    };
    assert_eq!(all_ids.len(), 4);
}

#[test]
fn filtering_keeps_matching_rows() {
    let filters: BTreeMap<String, String> = [("name".to_string(), "al".to_string())]
        .into_iter()
        .collect();
    let rig = Rig::<Msg>::sized(
        move || {
            Table::new(people(), columns(), |p| p.id.to_string())
                .filtering(
                    Filtering::new(filters.clone(), Msg::Filter).variant(FilterVariant::Inline),
                )
                .into()
        },
        800,
        600,
    );
    assert_eq!(row_names(&rig), ["Alan"]);
    assert!(find(&rig.tree(), Role::TextInput, "Filter Name").is_some());
}

#[test]
fn pagination_shows_one_page_and_a_pager() {
    let rig = Rig::<Msg>::sized(
        || {
            Table::new(people(), columns(), |p| p.id.to_string())
                .plugin(TablePagination::new(2, 3, Msg::Page))
                .into()
        },
        800,
        600,
    );
    assert_eq!(row_names(&rig), ["Katherine"]);
    assert!(find(&rig.tree(), Role::Navigation, "Pagination").is_some());
}

#[test]
fn grouped_rows_and_virtual_window() {
    let rig = Rig::<Msg>::sized(
        || {
            Table::new(people(), columns(), |p| p.id.to_string())
                .plugin(GroupedRows {
                    group_by: Rc::new(|p: &Person| p.team.to_string()),
                    collapsed: ["Navy".to_string()].into_iter().collect(),
                    on_toggle: Rc::new(|_| Msg::Page(0)),
                    group_order: None,
                })
                .into()
        },
        800,
        600,
    );
    assert_eq!(
        row_names(&rig),
        ["Ada", "Alan", "Katherine"],
        "Navy is collapsed"
    );
    let w = row_window(1000, 40.0, 400.0, 4000.0, 2);
    assert_eq!((w.start, w.end), (98, 113));
    assert_eq!(w.before, 98.0 * 40.0);
    assert_eq!(w.after, (1000 - 113) as f32 * 40.0);
}

#[test]
fn virtualized_table_draws_only_rows_in_view_and_follows_scroll() {
    #[derive(Clone)]
    struct R(u32);
    let mut rig = Rig::<Msg>::sized(
        || {
            let data: Rc<Vec<R>> = Rc::new((0..10_000).map(R).collect());
            Table::new(
                data,
                vec![TableColumn::new("n", "Number")
                    .value(|r: &R| r.0)
                    .is_row_header(true)],
                |r| r.0.to_string(),
            )
            .plugin(RowsVirtualization {
                row_height: 37.0,
                overscan: 2,
            })
            .max_height(370.0)
            .into()
        },
        800,
        600,
    );
    let shown = |rig: &Rig<Msg>| row_names(rig);
    let first = shown(&rig);
    assert!(first.len() < 20, "{} rows drawn", first.len());
    assert_eq!(first[0], "0");
    use astryx_core::contract::Surface;
    rig.rt.scroll(
        astryx_core::Point { x: 100.0, y: 200.0 },
        astryx_core::Point { x: 0.0, y: 3700.0 },
    );
    rig.redraw();
    rig.redraw();
    let later = shown(&rig);
    assert!(later.len() < 20);
    let top: u32 = later[0].parse().unwrap();
    assert!((95..=100).contains(&top), "top row {top}");
}
