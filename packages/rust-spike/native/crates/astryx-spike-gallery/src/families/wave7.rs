//! Wave 7 families: data.

use super::{kid_text, Family};
use crate::props::Props;
use crate::Msg;
use astryx_core::Element;
use astryx_spike_ui::*;
use serde_json::json;
use std::rc::Rc;

fn render_table(p: Props) -> Element<Msg> {
    let t: Table<(), Msg> = Table::new(Rc::new(Vec::new()), Vec::new(), |_| String::new())
        .density(match p.str_or("density", "balanced") {
            "compact" => ListDensity::Compact,
            "spacious" => ListDensity::Spacious,
            _ => ListDensity::Balanced,
        })
        .dividers(match p.str_or("dividers", "rows") {
            "columns" => TableDividers::Columns,
            "grid" => TableDividers::Grid,
            "none" => TableDividers::None,
            _ => TableDividers::Rows,
        })
        .is_striped(p.bool("isStriped"))
        .has_hover(p.bool("hasHover"));
    // With no columns, a text child shows before the table, as a browser
    // moves stray text out of a <table>.
    let mut s = VStack::new();
    if let Some(words) = kid_text(&p) {
        s = s.child(Text::new(words));
    }
    s.child(t).into()
}

pub fn families() -> Vec<Family> {
    vec![Family {
        name: "Table",
        base: || json!({}),
        sweeps: || vec![],
        render: render_table,
    }]
}
