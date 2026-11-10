//! Wave 2 families: layout primitives and containers.

use super::{kids, v, Family};
use crate::props::{Props, SPACE_STEPS};
use crate::Msg;
use astryx_core::contract::Color;
use astryx_core::{Element, Space};
use astryx_spike_ui::*;
use serde_json::{json, Value};

fn steps() -> Vec<Value> {
    SPACE_STEPS.iter().map(|s| json!(s)).collect()
}

fn axis_align(s: &str) -> Option<StackAxisAlign> {
    Some(match s {
        "start" => StackAxisAlign::Start,
        "center" => StackAxisAlign::Center,
        "end" => StackAxisAlign::End,
        "between" => StackAxisAlign::Between,
        "around" => StackAxisAlign::Around,
        "evenly" => StackAxisAlign::Evenly,
        "stretch" => StackAxisAlign::Stretch,
        _ => return None,
    })
}

fn wrap(s: &str) -> StackWrap {
    match s {
        "wrap" => StackWrap::Wrap,
        "wrap-reverse" => StackWrap::WrapReverse,
        _ => StackWrap::Nowrap,
    }
}

fn stack(p: Props, direction: StackDirection) -> Element<Msg> {
    let mut s = Stack::new()
        .direction(direction)
        .children(kids(&p))
        .wrap(wrap(p.str_or("wrap", "nowrap")))
        .is_scrollable(p.bool("isScrollable"));
    if let Some(w) = p.size("width") {
        s = s.width(w);
    }
    if let Some(h) = p.size("height") {
        s = s.height(h);
    }
    if let Some(w) = p.size("maxWidth") {
        s = s.max_width(w);
    }
    if let Some(h) = p.size("minHeight") {
        s = s.min_height(h);
    }
    if let Some(g) = p.space("gap") {
        s = s.gap(g);
    }
    for (k, f) in [
        (
            "padding",
            Stack::padding as fn(Stack<Msg>, Space) -> Stack<Msg>,
        ),
        ("paddingInline", Stack::padding_inline),
        ("paddingInlineStart", Stack::padding_inline_start),
        ("paddingInlineEnd", Stack::padding_inline_end),
        ("paddingBlock", Stack::padding_block),
        ("paddingBlockStart", Stack::padding_block_start),
        ("paddingBlockEnd", Stack::padding_block_end),
    ] {
        if let Some(step) = p.space(k) {
            s = f(s, step);
        }
    }
    if let Some(a) = p.str("hAlign").and_then(axis_align) {
        s = s.h_align(a);
    }
    if let Some(a) = p.str("vAlign").and_then(axis_align) {
        s = s.v_align(a);
    }
    if let Some(a) = p.str("justify").and_then(axis_align) {
        s = match direction {
            StackDirection::Horizontal => s.h_align(a),
            StackDirection::Vertical => s.v_align(a),
        };
    }
    if let Some(a) = p.str("align").and_then(axis_align) {
        s = match direction {
            StackDirection::Horizontal => s.v_align(a),
            StackDirection::Vertical => s.h_align(a),
        };
    }
    s.into()
}

fn render_stack(p: Props) -> Element<Msg> {
    let dir = if p.str("direction") == Some("horizontal") {
        StackDirection::Horizontal
    } else {
        StackDirection::Vertical
    };
    stack(p, dir)
}

fn render_hstack(p: Props) -> Element<Msg> {
    stack(p, StackDirection::Horizontal)
}

fn render_vstack(p: Props) -> Element<Msg> {
    stack(p, StackDirection::Vertical)
}

fn grid_align(s: &str) -> GridAlign {
    match s {
        "start" => GridAlign::Start,
        "center" => GridAlign::Center,
        "end" => GridAlign::End,
        _ => GridAlign::Stretch,
    }
}

fn render_grid(p: Props) -> Element<Msg> {
    let columns = match p.get("columns") {
        Some(Value::Object(o)) => GridColumns::Auto {
            min_width: o.get("minWidth").and_then(Value::as_f64).unwrap_or(120.0) as f32,
            max: o.get("max").and_then(Value::as_u64).map(|m| m as u16),
            repeat: if o.get("repeat").and_then(Value::as_str) == Some("fit") {
                GridRepeat::Fit
            } else {
                GridRepeat::Fill
            },
        },
        Some(n) => GridColumns::Count(n.as_u64().unwrap_or(1) as u16),
        None => GridColumns::Count(1),
    };
    let mut g = Grid::new(columns).children(kids(&p));
    if let Some(s) = p.space("gap") {
        g = g.gap(s);
    }
    if let Some(s) = p.space("rowGap") {
        g = g.row_gap(s);
    }
    if let Some(s) = p.space("columnGap") {
        g = g.column_gap(s);
    }
    if let Some(a) = p.str("align") {
        g = g.align(grid_align(a));
    }
    if let Some(a) = p.str("justify") {
        g = g.justify(grid_align(a));
    }
    if let Some(h) = p.num("rowHeight") {
        g = g.row_height(h as f32);
    }
    if let Some(w) = p.size("width") {
        g = g.width(w);
    }
    if let Some(h) = p.size("height") {
        g = g.height(h);
    }
    if let Some(w) = p.size("maxWidth") {
        g = g.max_width(w);
    }
    if let Some(h) = p.size("minHeight") {
        g = g.min_height(h);
    }
    g.into()
}

fn render_center(p: Props) -> Element<Msg> {
    let mut c = Center::new()
        .axis(match p.str_or("axis", "both") {
            "horizontal" => CenterAxis::Horizontal,
            "vertical" => CenterAxis::Vertical,
            _ => CenterAxis::Both,
        })
        .is_inline(p.bool("isInline"));
    // Center lays its children out in a row, as display:flex does.
    c = c.child(HStack::new().children(kids(&p)));
    if let Some(w) = p.size("width") {
        c = c.width(w);
    }
    if let Some(h) = p.size("height") {
        c = c.height(h);
    }
    if let Some(w) = p.size("maxWidth") {
        c = c.max_width(w);
    }
    if let Some(h) = p.size("minHeight") {
        c = c.min_height(h);
    }
    for (k, f) in [
        (
            "padding",
            Center::padding as fn(Center<Msg>, Space) -> Center<Msg>,
        ),
        ("paddingInline", Center::padding_inline),
        ("paddingInlineStart", Center::padding_inline_start),
        ("paddingInlineEnd", Center::padding_inline_end),
        ("paddingBlock", Center::padding_block),
        ("paddingBlockStart", Center::padding_block_start),
        ("paddingBlockEnd", Center::padding_block_end),
    ] {
        if let Some(step) = p.space(k) {
            c = f(c, step);
        }
    }
    c.into()
}

fn render_aspect_ratio(p: Props) -> Element<Msg> {
    let ratio = p.num("ratio").unwrap_or(1.0) as f32;
    let mut a = AspectRatio::new(ratio, HStack::new().children(kids(&p))).shape(
        if p.str("shape") == Some("ellipse") {
            AspectRatioShape::Ellipse
        } else {
            AspectRatioShape::Rectangle
        },
    );
    if let Some(f) = p.str("fit") {
        a = a.fit(match f {
            "contain" => AspectRatioFit::Contain,
            "center" => AspectRatioFit::Center,
            _ => AspectRatioFit::Cover,
        });
    }
    a.into()
}

fn section_variant(s: &str) -> SectionVariant {
    match s {
        "transparent" => SectionVariant::Transparent,
        "muted" => SectionVariant::Muted,
        _ => SectionVariant::Section,
    }
}

fn render_section(p: Props) -> Element<Msg> {
    let mut s = Section::new()
        .variant(section_variant(p.str_or("variant", "section")))
        .children(kids(&p));
    if let Some(w) = p.size("width") {
        s = s.width(w);
    }
    if let Some(h) = p.size("height") {
        s = s.height(h);
    }
    if let Some(w) = p.size("maxWidth") {
        s = s.max_width(w);
    }
    if let Some(h) = p.size("minHeight") {
        s = s.min_height(h);
    }
    for (k, f) in [
        (
            "padding",
            Section::padding as fn(Section<Msg>, Space) -> Section<Msg>,
        ),
        ("paddingInline", Section::padding_inline),
        ("paddingInlineStart", Section::padding_inline_start),
        ("paddingInlineEnd", Section::padding_inline_end),
        ("paddingBlock", Section::padding_block),
        ("paddingBlockStart", Section::padding_block_start),
        ("paddingBlockEnd", Section::padding_block_end),
    ] {
        if let Some(step) = p.space(k) {
            s = f(s, step);
        }
    }
    if let Some(Value::Array(ds)) = p.get("dividers") {
        s = s.dividers(ds.iter().filter_map(Value::as_str).filter_map(|d| {
            Some(match d {
                "top" => SectionDivider::Top,
                "bottom" => SectionDivider::Bottom,
                "start" => SectionDivider::Start,
                "end" => SectionDivider::End,
                _ => return None,
            })
        }));
    }
    s.into()
}

pub(crate) fn card_variant(s: &str) -> CardVariant {
    use CardVariant as C;
    [
        C::Default,
        C::Transparent,
        C::Muted,
        C::Blue,
        C::Cyan,
        C::Gray,
        C::Green,
        C::Orange,
        C::Pink,
        C::Purple,
        C::Red,
        C::Teal,
        C::Yellow,
    ]
    .into_iter()
    .find(|c| format!("{c:?}").eq_ignore_ascii_case(s))
    .unwrap_or_default()
}

pub(crate) fn elevation(s: &str) -> Elevation {
    match s {
        "low" => Elevation::Low,
        "med" => Elevation::Med,
        "high" => Elevation::High,
        _ => Elevation::None,
    }
}

fn render_card(p: Props) -> Element<Msg> {
    let mut c = Card::new()
        .variant(card_variant(p.str_or("variant", "default")))
        .elevation(elevation(p.str_or("elevation", "none")))
        .children(kids(&p));
    if let Some(s) = p.space("padding") {
        c = c.padding(s);
    }
    if let Some(w) = p.size("width") {
        c = c.width(w);
    }
    if let Some(h) = p.size("height") {
        c = c.height(h);
    }
    if let Some(w) = p.size("maxWidth") {
        c = c.max_width(w);
    }
    if let Some(h) = p.size("minHeight") {
        c = c.min_height(h);
    }
    c.into()
}

fn render_empty_state(p: Props) -> Element<Msg> {
    let mut e = EmptyState::new(p.words("Title"))
        .heading_level(p.num("headingLevel").unwrap_or(3.0) as u8)
        .is_compact(p.bool("isCompact"));
    if let Some(d) = p.str("description") {
        e = e.description(d);
    }
    e.into()
}

fn render_layout(p: Props) -> Element<Msg> {
    let mut l = Layout::new()
        .height(if p.str("height") == Some("auto") {
            LayoutHeight::Auto
        } else {
            LayoutHeight::Fill
        })
        .default_has_dividers(p.bool("defaultHasDividers"));
    let mut content = LayoutContent::new();
    for k in kids(&p) {
        content = content.child(k);
    }
    l = l.content(content);
    if let Some(s) = p.space("padding") {
        l = l.padding(s);
    }
    if let Some(w) = p.size("contentWidth") {
        l = l.content_width(w);
    }
    l.into()
}

#[allow(dead_code)]
fn _unused(_: Color) {}

fn padding_sweeps() -> Vec<(&'static str, Vec<Value>)> {
    vec![
        ("padding", steps()),
        ("paddingInline", steps()),
        ("paddingInlineStart", steps()),
        ("paddingInlineEnd", steps()),
        ("paddingBlock", steps()),
        ("paddingBlockStart", steps()),
        ("paddingBlockEnd", steps()),
    ]
}

pub fn families() -> Vec<Family> {
    vec![
        Family {
            name: "Stack",
            base: || json!({"gap": 2}),
            sweeps: || {
                let mut s = vec![
                    ("direction", v(&["horizontal", "vertical"])),
                    ("gap", steps()),
                    (
                        "hAlign",
                        v(&[
                            "start", "center", "end", "between", "around", "evenly", "stretch",
                        ]),
                    ),
                    (
                        "vAlign",
                        v(&[
                            "start", "center", "end", "between", "around", "evenly", "stretch",
                        ]),
                    ),
                    ("wrap", v(&["nowrap", "wrap", "wrap-reverse"])),
                ];
                s.extend(padding_sweeps());
                s
            },
            render: render_stack,
        },
        Family {
            name: "HStack",
            base: || json!({"gap": 2}),
            sweeps: || {
                let mut s = vec![
                    ("gap", steps()),
                    (
                        "hAlign",
                        v(&["start", "center", "end", "between", "around", "evenly"]),
                    ),
                    ("vAlign", v(&["start", "center", "end", "stretch"])),
                ];
                s.extend(padding_sweeps());
                s
            },
            render: render_hstack,
        },
        Family {
            name: "VStack",
            base: || json!({"gap": 2}),
            sweeps: || {
                let mut s = vec![
                    ("gap", steps()),
                    ("hAlign", v(&["start", "center", "end", "stretch"])),
                    (
                        "vAlign",
                        v(&["start", "center", "end", "between", "around", "evenly"]),
                    ),
                ];
                s.extend(padding_sweeps());
                s
            },
            render: render_vstack,
        },
        Family {
            name: "Grid",
            base: || json!({"columns": 3, "gap": 2}),
            sweeps: || {
                vec![
                    ("gap", steps()),
                    ("rowGap", steps()),
                    ("columnGap", steps()),
                    ("align", v(&["start", "center", "end", "stretch"])),
                    ("justify", v(&["start", "center", "end", "stretch"])),
                    (
                        "columns",
                        vec![
                            json!(2),
                            json!(4),
                            json!({"minWidth": 100}),
                            json!({"minWidth": 100, "max": 2}),
                        ],
                    ),
                ]
            },
            render: render_grid,
        },
        Family {
            name: "Center",
            base: || json!({}),
            sweeps: || {
                let mut s = vec![("axis", v(&["both", "horizontal", "vertical"]))];
                s.extend(padding_sweeps());
                s
            },
            render: render_center,
        },
        Family {
            name: "AspectRatio",
            base: || json!({"ratio": 1.7778}),
            sweeps: || {
                vec![
                    ("shape", v(&["rectangle", "ellipse"])),
                    ("fit", v(&["cover", "contain", "center"])),
                    ("ratio", vec![json!(1), json!(1.3333), json!(0.75)]),
                ]
            },
            render: render_aspect_ratio,
        },
        Family {
            name: "Section",
            base: || json!({}),
            sweeps: || {
                let mut s = vec![
                    ("variant", v(&["section", "transparent", "muted"])),
                    (
                        "dividers",
                        vec![json!(["top"]), json!(["bottom"]), json!(["start", "end"])],
                    ),
                ];
                s.extend(padding_sweeps());
                s
            },
            render: render_section,
        },
        Family {
            name: "Card",
            base: || json!({}),
            sweeps: || {
                vec![
                    ("padding", steps()),
                    (
                        "variant",
                        v(&[
                            "default",
                            "transparent",
                            "muted",
                            "blue",
                            "cyan",
                            "gray",
                            "green",
                            "orange",
                            "pink",
                            "purple",
                            "red",
                            "teal",
                            "yellow",
                        ]),
                    ),
                    ("elevation", v(&["none", "low", "med", "high"])),
                ]
            },
            render: render_card,
        },
        Family {
            name: "EmptyState",
            base: || json!({}),
            sweeps: || {
                vec![
                    ("headingLevel", (1..=6).map(|l| json!(l)).collect()),
                    ("isCompact", vec![json!(true)]),
                ]
            },
            render: render_empty_state,
        },
        Family {
            name: "Layout",
            base: || json!({}),
            sweeps: || {
                vec![
                    ("padding", steps()),
                    ("defaultHasDividers", vec![json!(true)]),
                    ("hasDivider", vec![json!(true), json!(false)]),
                    ("withPanel", vec![json!(true)]),
                ]
            },
            render: render_layout,
        },
    ]
}
