//! Renders a populated Table with selection, sorting and pagination to a
//! PNG: `cargo run --example table_demo -- out.png [light|dark]`.

use astryx_core::{App, Command, Element, Mode, Runtime, Theme};
use astryx_spike_ui::*;
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Clone, Debug)]
enum Msg {
    Noop,
}

struct Demo(Mode);

impl App for Demo {
    type Msg = Msg;
    fn view(&self) -> Element<Msg> {
        #[derive(Clone)]
        struct Row(u32, &'static str, &'static str, u32);
        let data = Rc::new(vec![
            Row(1, "Ada Lovelace", "Analytical Engine", 36),
            Row(2, "Grace Hopper", "Compilers", 85),
            Row(3, "Alan Turing", "Computability", 41),
            Row(4, "Katherine Johnson", "Orbital mechanics", 101),
            Row(5, "Edsger Dijkstra", "Shortest paths", 72),
        ]);
        let cols = vec![
            TableColumn::new("name", "Name")
                .value(|r: &Row| r.1)
                .sortable()
                .is_row_header(true),
            TableColumn::new("field", "Known for").value(|r: &Row| r.2),
            TableColumn::new("age", "Age")
                .value(|r: &Row| r.3)
                .sortable()
                .align(ColumnAlign::End)
                .width(ColumnWidth::Pixel(96.0)),
        ];
        let sel: HashSet<String> = ["2".to_string()].into_iter().collect();
        HStack::new()
            .gap(astryx_core::Space::S4)
            .child(
                Card::new().child(
                    Calendar::new(
                        "cal",
                        CalendarValue::Range(Some(DateRange {
                            start: CivilDate::new(2026, 11, 9),
                            end: Some(CivilDate::new(2026, 11, 13)),
                        })),
                    )
                    .today(CivilDate::new(2026, 11, 11)),
                ),
            )
            .child(
                Card::new().padding(astryx_core::Space::S4).child(
                    Table::new(data, cols, |r| r.0.to_string())
                        .label("Pioneers")
                        .sortable(Sortable::new(
                            vec![SortEntry {
                                sort_key: "age".into(),
                                direction: SortDirection::Ascending,
                            }],
                            |_| Msg::Noop,
                        ))
                        .plugin(Selection::new(sel, |_| Msg::Noop))
                        .plugin(TablePagination::new(1, 4, |_| Msg::Noop)),
                ),
            )
            .into()
    }
    fn update(&mut self, _: Msg) -> Command {
        Command::None
    }
    fn theme(&self) -> astryx_core::ThemeChoice {
        astryx_core::ThemeChoice {
            theme: Theme::NEUTRAL,
            mode: Some(self.0),
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or_else(|| "table.png".into());
    let mode = if args.next().as_deref() == Some("dark") {
        Mode::Dark
    } else {
        Mode::Light
    };
    let mut rt = Runtime::new(Demo(mode)).hide_focus_until_keyboard();
    let png = astryx_headless::render_png(&mut rt, 1100, 420, 2.0, &[]);
    std::fs::write(out, png).unwrap();
}
