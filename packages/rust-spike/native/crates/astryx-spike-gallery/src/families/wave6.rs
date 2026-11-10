//! Wave 6 families: composites.

use super::{inline, kid_text, kids, v, Family};
use crate::props::Props;
use crate::Msg;
use astryx_core::{Element, Point, Space};
use astryx_spike_ui::*;
use serde_json::json;

fn density(p: Props) -> ListDensity {
    match p.str_or("density", "balanced") {
        "compact" => ListDensity::Compact,
        "spacious" => ListDensity::Spacious,
        _ => ListDensity::Balanced,
    }
}

fn render_dialog(p: Props) -> Element<Msg> {
    let open = p.get("isOpen").and_then(|v| v.as_bool()).unwrap_or(false);
    let mut d = Dialog::new(open, |_| Msg::Noop)
        .variant(if p.str("variant") == Some("fullscreen") {
            DialogVariant::Fullscreen
        } else {
            DialogVariant::Standard
        })
        .purpose(match p.str_or("purpose", "info") {
            "required" => DialogPurpose::Required,
            "form" => DialogPurpose::Form,
            _ => DialogPurpose::Info,
        })
        .is_inline(p.bool("isInline"));
    if let Some(w) = p.num("width") {
        d = d.width(w as f32);
    }
    if let Some(s) = p.space("padding") {
        d = d.padding(s);
    }
    for k in kids(&p) {
        d = d.child(k);
    }
    d.into()
}

fn render_dialog_header(p: Props) -> Element<Msg> {
    let mut h = DialogHeader::new(p.str_or("title", "Title")).has_divider(
        p.get("hasDivider")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
    );
    if let Some(s) = p.str("subtitle") {
        h = h.subtitle(s);
    }
    if p.get("onOpenChange").is_some() {
        h = h.on_open_change(|_| Msg::Noop);
    }
    if p.get("endContent").is_some() {
        h = h.end_content(Button::new("Action"));
    }
    if p.get("startContent").is_some() {
        h = h.start_content(Icon::new("info"));
    }
    Layout::new()
        .height(LayoutHeight::Auto)
        .header(h)
        .content(LayoutContent::new())
        .into()
}

fn render_list(p: Props) -> Element<Msg> {
    let mut l = List::new()
        .density(density(p))
        .has_dividers(p.bool("hasDividers"))
        .edge(match p.str_or("edge", "padded") {
            "flush" => ListEdge::Flush,
            "outset" => ListEdge::Outset,
            "bleed" => ListEdge::Bleed,
            _ => ListEdge::Padded,
        })
        .list_style(match p.str_or("listStyle", "none") {
            "disc" => ListStyle::Disc,
            "decimal" => ListStyle::Decimal,
            "circle" => ListStyle::Circle,
            _ => ListStyle::None,
        });
    if p.str("$component") == Some("ListItem") {
        let mut item = ListItem::new(p.str_or("label", "Label"))
            .is_disabled(p.bool("isDisabled"))
            .is_selected(p.bool("isSelected"));
        if let Some(d) = p.str("description") {
            item = item.description(d);
        }
        item = item.align(match p.str_or("align", "center") {
            "start" => ItemAlign::Start,
            "first-line" => ItemAlign::FirstLine,
            _ => ItemAlign::Center,
        });
        l = l.child(item);
    } else {
        for k in kids(&p) {
            l = l.child(k);
        }
    }
    l.into()
}

fn render_collapsible(p: Props) -> Element<Msg> {
    let mut c = Collapsible::new(p.str_or("trigger", "Trigger"))
        .default_is_open(
            p.get("defaultIsOpen")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
        )
        .is_disabled(p.bool("isDisabled"))
        .chevron_position(if p.str("chevronPosition") == Some("start") {
            ChevronPosition::Start
        } else {
            ChevronPosition::End
        });
    if let Some(o) = p.get("isOpen").and_then(|v| v.as_bool()) {
        c = c.is_open(o);
    }
    if let Some(t) = kid_text(&p) {
        c = c.child(Text::new(t));
    }
    c.into()
}

fn render_dropdown(p: Props) -> Element<Msg> {
    let mut m = DropdownMenu::new(
        "menu",
        vec![
            MenuOption::item("Rename").icon(Icon::new("copy")),
            MenuOption::item("Duplicate"),
            MenuOption::Divider,
            MenuOption::item("Delete").variant(MenuItemVariant::Destructive),
        ],
    )
    .label(p.str_or("label", "Menu"))
    .has_chevron(
        p.get("hasChevron")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
    );
    if let Some(o) = p.get("isMenuOpen").and_then(|v| v.as_bool()) {
        m = m.is_menu_open(o).on_open_change(|_| Msg::Noop);
    }
    inline(m)
}

fn render_context_menu(p: Props) -> Element<Msg> {
    let open = p.bool("isOpen");
    ContextMenu::new(
        Card::new().child(Text::new("Right-click here")),
        vec![MenuOption::item("Copy"), MenuOption::item("Paste")],
    )
    .open_at(open.then_some(Point { x: 24.0, y: 64.0 }))
    .on_open_change(|_| Msg::Noop)
    .into()
}

fn render_toolbar(p: Props) -> Element<Msg> {
    let mut t = Toolbar::new(p.str_or("label", "Label"))
        .start_content(Button::new("Bold"))
        .start_content(Button::new("Italic"))
        .end_content(Button::new("Share").variant(ButtonVariant::Primary))
        .orientation(if p.str("orientation") == Some("vertical") {
            ToolbarOrientation::Vertical
        } else {
            ToolbarOrientation::Horizontal
        })
        .variant(match p.str_or("variant", "transparent") {
            "section" => SectionVariant::Section,
            "muted" => SectionVariant::Muted,
            _ => SectionVariant::Transparent,
        });
    if let Some(g) = p.space("gap") {
        t = t.gap(g);
    }
    if let Some(s) = p.str("size") {
        t = t.size(match s {
            "sm" => astryx_spike_foundations::ComponentSize::Sm,
            "lg" => astryx_spike_foundations::ComponentSize::Lg,
            _ => astryx_spike_foundations::ComponentSize::Md,
        });
    }
    t.into()
}

fn render_pagination(p: Props) -> Element<Msg> {
    let mut pg = Pagination::new(p.num("page").unwrap_or(3.0) as u32, |_| Msg::Noop)
        .total_pages(p.num("totalPages").unwrap_or(10.0) as u32)
        .variant(match p.str_or("variant", "pages") {
            "count" => PaginationVariant::Count,
            "compact" => PaginationVariant::Compact,
            "dots" => PaginationVariant::Dots,
            "input" => PaginationVariant::Input,
            "none" => PaginationVariant::None,
            _ => PaginationVariant::Pages,
        })
        .is_disabled(p.bool("isDisabled"));
    if p.str("size") == Some("sm") {
        pg = pg.size(astryx_spike_foundations::ComponentSize::Sm);
    }
    pg.into()
}

fn render_overflow(p: Props) -> Element<Msg> {
    let mut o =
        OverflowList::new().max_visible_items(p.num("maxVisibleItems").unwrap_or(3.0) as usize);
    if let Some(g) = p.space("gap") {
        o = o.gap(g);
    }
    for name in ["Design", "Engineering", "Research", "Marketing", "Sales"] {
        o = o.child(Token::new(name));
    }
    o.into()
}

fn steps() -> Vec<serde_json::Value> {
    crate::props::SPACE_STEPS.iter().map(|s| json!(s)).collect()
}

pub fn families() -> Vec<Family> {
    let _ = Space::S0;
    vec![
        Family {
            name: "Dialog",
            base: || json!({"isOpen": true, "$fixture": "text"}),
            sweeps: || {
                vec![
                    ("variant", v(&["standard", "fullscreen"])),
                    ("purpose", v(&["required", "form", "info"])),
                ]
            },
            render: render_dialog,
        },
        Family {
            name: "DialogHeader",
            base: || json!({"title": "Title", "onOpenChange": "x"}),
            sweeps: || {
                vec![
                    ("subtitle", v(&["Subtitle"])),
                    ("hasDivider", vec![json!(false)]),
                    ("endContent", vec![json!(true)]),
                ]
            },
            render: render_dialog_header,
        },
        Family {
            name: "List",
            base: || json!({"$fixture": "text"}),
            sweeps: || {
                vec![
                    ("density", v(&["compact", "balanced", "spacious"])),
                    ("hasDividers", vec![json!(true)]),
                    ("edge", v(&["padded", "flush", "outset", "bleed"])),
                    ("listStyle", v(&["none", "disc", "decimal", "circle"])),
                ]
            },
            render: render_list,
        },
        Family {
            name: "Collapsible",
            base: || json!({"$fixture": "text"}),
            sweeps: || {
                vec![
                    ("defaultIsOpen", vec![json!(false)]),
                    ("isDisabled", vec![json!(true)]),
                    ("chevronPosition", v(&["start", "end"])),
                ]
            },
            render: render_collapsible,
        },
        Family {
            name: "DropdownMenu",
            base: || json!({}),
            sweeps: || {
                vec![
                    ("isMenuOpen", vec![json!(true)]),
                    ("hasChevron", vec![json!(false)]),
                ]
            },
            render: render_dropdown,
        },
        Family {
            name: "ContextMenu",
            base: || json!({}),
            sweeps: || vec![("isOpen", vec![json!(true)])],
            render: render_context_menu,
        },
        Family {
            name: "Toolbar",
            base: || json!({"label": "Formatting"}),
            sweeps: || {
                vec![
                    ("size", v(&["sm", "md", "lg"])),
                    ("gap", steps()),
                    ("orientation", v(&["horizontal", "vertical"])),
                    ("variant", v(&["section", "transparent", "muted"])),
                ]
            },
            render: render_toolbar,
        },
        Family {
            name: "Pagination",
            base: || json!({"page": 3, "totalPages": 10}),
            sweeps: || {
                vec![
                    (
                        "variant",
                        v(&["pages", "count", "compact", "dots", "input", "none"]),
                    ),
                    ("size", v(&["sm", "md"])),
                    ("isDisabled", vec![json!(true)]),
                ]
            },
            render: render_pagination,
        },
        Family {
            name: "OverflowList",
            base: || json!({}),
            sweeps: || vec![("gap", steps())],
            render: render_overflow,
        },
    ]
}
