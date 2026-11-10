//! Wave 7 families: data.

use super::{kid_text, kids, v, Family};
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

fn render_typeahead(p: Props) -> Element<Msg> {
    let source = static_source(vec![Entry::new("a", "Apple"), Entry::new("b", "Banana")]);
    let mut t = Typeahead::new("typeahead", p.str_or("label", "Label"), source, None)
        .has_entries_on_focus(p.bool("hasEntriesOnFocus"))
        .has_clear(p.get("hasClear").and_then(|v| v.as_bool()).unwrap_or(true))
        .is_disabled(p.bool("isDisabled"))
        .is_label_hidden(p.bool("isLabelHidden"))
        .on_change(|_| Msg::Noop);
    if let Some(s) = p.str("size") {
        t = t.size(match s {
            "sm" => astryx_spike_foundations::ComponentSize::Sm,
            "lg" => astryx_spike_foundations::ComponentSize::Lg,
            _ => astryx_spike_foundations::ComponentSize::Md,
        });
    }
    t.into()
}

fn render_tree_list(p: Props) -> Element<Msg> {
    TreeList::new(vec![
        TreeListItem::new("src", "src")
            .is_expanded(true)
            .child(TreeListItem::new("lib", "lib.rs").is_selected(true))
            .child(TreeListItem::new("main", "main.rs")),
        TreeListItem::new("readme", "README.md"),
    ])
    .density(match p.str_or("density", "balanced") {
        "compact" => ListDensity::Compact,
        "spacious" => ListDensity::Spacious,
        _ => ListDensity::Balanced,
    })
    .variant(if p.str("variant") == Some("noGuides") {
        TreeListVariant::NoGuides
    } else {
        TreeListVariant::LineGuides
    })
    .into()
}

fn render_command_palette(p: Props) -> Element<Msg> {
    let source = static_source(vec![
        Entry::new("new", "New File"),
        Entry::new("open", "Open File…"),
    ]);
    CommandPalette::new("palette", p.bool("isOpen"), |_| Msg::Noop, source)
        .label(p.str_or("label", "Label"))
        .is_inline(p.bool("isInline"))
        .into()
}

fn render_app_shell(p: Props) -> Element<Msg> {
    let mut a = AppShell::new()
        .height(if p.str("height") == Some("auto") {
            AppShellHeight::Auto
        } else {
            AppShellHeight::Fill
        })
        .variant(match p.str_or("variant", "elevated") {
            "wash" => AppShellVariant::Wash,
            "surface" => AppShellVariant::Surface,
            "section" => AppShellVariant::Section,
            _ => AppShellVariant::Elevated,
        })
        .children(kids(&p));
    if let Some(s) = p.space("contentPadding") {
        a = a.content_padding(s);
    }
    a.into()
}

pub fn families() -> Vec<Family> {
    vec![
        Family {
            name: "Table",
            base: || json!({}),
            sweeps: || vec![],
            render: render_table,
        },
        Family {
            name: "Typeahead",
            base: || json!({"label": "Label"}),
            sweeps: || vec![("size", v(&["sm", "md", "lg"]))],
            render: render_typeahead,
        },
        Family {
            name: "TreeList",
            base: || json!({}),
            sweeps: || vec![("density", v(&["compact", "balanced", "spacious"]))],
            render: render_tree_list,
        },
        Family {
            name: "CommandPalette",
            base: || json!({"label": "Label"}),
            sweeps: || vec![("isInline", vec![json!(true)])],
            render: render_command_palette,
        },
        Family {
            name: "AppShell",
            base: || json!({"$fixture": "boxes"}),
            sweeps: || vec![("variant", v(&["wash", "surface", "section", "elevated"]))],
            render: render_app_shell,
        },
    ]
}
