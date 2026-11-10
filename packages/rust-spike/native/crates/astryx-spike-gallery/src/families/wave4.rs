//! Wave 4 families: light overlays, shown open.

use super::{inline, kid_text, kids, v, Family};
use crate::props::Props;
use crate::Msg;
use astryx_core::Element;
use astryx_spike_ui::*;
use serde_json::json;

fn side(p: Props, default: OverlaySide) -> OverlaySide {
    match p.str("placement") {
        Some("above") => OverlaySide::Above,
        Some("below") => OverlaySide::Below,
        Some("start") => OverlaySide::Start,
        Some("end") => OverlaySide::End,
        _ => default,
    }
}

fn align(p: Props, default: OverlayAlign) -> OverlayAlign {
    match p.str("alignment") {
        Some("start") => OverlayAlign::Start,
        Some("center") => OverlayAlign::Center,
        Some("end") => OverlayAlign::End,
        _ => default,
    }
}

fn trigger(p: &Props) -> Element<Msg> {
    match kid_text(p) {
        Some(t) => Text::new(t).into(),
        None => HStack::new().children(kids(p)).into(),
    }
}

fn open(p: &Props) -> bool {
    p.get("isOpen").and_then(|v| v.as_bool()).unwrap_or(false)
        || p.get("isDefaultOpen")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
}

fn render_tooltip(p: Props) -> Element<Msg> {
    inline(
        Tooltip::new("tip", trigger(&p), p.str_or("content", "Content"))
            .placement(side(p, OverlaySide::Above))
            .alignment(align(p, OverlayAlign::Center))
            .is_enabled(p.get("isEnabled").and_then(|v| v.as_bool()).unwrap_or(true))
            .is_open(open(&p))
            .on_open_change(|_| Msg::Noop),
    )
}

fn render_info_tip(p: Props) -> Element<Msg> {
    let size = super::wave1::icon_size(p.str_or("size", "sm"));
    let mut t = InfoTip::new("info", p.str_or("content", "Content"))
        .size(size)
        .is_open(open(&p))
        .on_open_change(|_| Msg::Noop);
    if let Some(l) = p.str("label") {
        t = t.label(l);
    }
    inline(t)
}

fn render_hover_card(p: Props) -> Element<Msg> {
    inline(
        HoverCard::new(
            "card",
            trigger(&p),
            Text::new(p.str_or("content", "Content")),
        )
        .placement(side(p, OverlaySide::Above))
        .alignment(align(p, OverlayAlign::Center))
        .is_open(open(&p))
        .on_open_change(|_| Msg::Noop),
    )
}

fn render_popover(p: Props) -> Element<Msg> {
    let mut pop = Popover::new(
        "pop",
        trigger(&p),
        Text::new(p.str_or("content", "Content")),
    )
    .placement(side(p, OverlaySide::Below))
    .alignment(align(p, OverlayAlign::Start))
    .is_open(open(&p))
    .on_open_change(|_| Msg::Noop);
    if let Some(l) = p.str("label") {
        pop = pop.label(l);
    }
    if let Some(s) = p.space("padding") {
        pop = pop.padding(s);
    }
    if let Some(w) = p.num("width") {
        pop = pop.width(PopoverWidth::Points(w as f32));
    }
    inline(pop)
}

fn render_bottom_sheet(p: Props) -> Element<Msg> {
    let mut s = BottomSheet::new(p.str_or("label", "Label"), open(&p))
        .purpose(match p.str_or("purpose", "info") {
            "required" => SheetPurpose::Required,
            "form" => SheetPurpose::Form,
            _ => SheetPurpose::Info,
        })
        .height(match p.str_or("height", "capped") {
            "hug" => SheetHeight::Hug,
            "tall" => SheetHeight::Tall,
            _ => SheetHeight::Capped,
        })
        .has_scrim(p.get("hasScrim").and_then(|v| v.as_bool()).unwrap_or(true))
        .on_open_change(|_| Msg::Noop);
    for k in kids(&p) {
        s = s.child(k);
    }
    if let Some(pad) = p.space("padding") {
        s = s.padding(pad);
    }
    s.into()
}

pub fn families() -> Vec<Family> {
    vec![
        Family {
            name: "Tooltip",
            base: || json!({"isOpen": true, "$fixture": "text"}),
            sweeps: || {
                vec![
                    ("placement", v(&["above", "below", "start", "end"])),
                    ("alignment", v(&["start", "center", "end"])),
                ]
            },
            render: render_tooltip,
        },
        Family {
            name: "InfoTip",
            base: || json!({"isOpen": true, "$fixture": "text"}),
            sweeps: || vec![("size", v(&["xsm", "sm", "md", "lg"]))],
            render: render_info_tip,
        },
        Family {
            name: "HoverCard",
            base: || json!({"isOpen": true, "$fixture": "text"}),
            sweeps: || {
                vec![
                    ("placement", v(&["above", "below", "start", "end"])),
                    ("alignment", v(&["start", "center", "end"])),
                ]
            },
            render: render_hover_card,
        },
        Family {
            name: "Popover",
            base: || json!({"isOpen": true, "$fixture": "text"}),
            sweeps: || {
                vec![
                    ("placement", v(&["above", "below", "start", "end"])),
                    ("alignment", v(&["start", "center", "end"])),
                    (
                        "padding",
                        crate::props::SPACE_STEPS.iter().map(|s| json!(s)).collect(),
                    ),
                ]
            },
            render: render_popover,
        },
        Family {
            name: "BottomSheet",
            base: || json!({"isOpen": true}),
            sweeps: || {
                vec![
                    ("purpose", v(&["required", "form", "info"])),
                    ("height", v(&["hug", "capped", "tall"])),
                    (
                        "padding",
                        crate::props::SPACE_STEPS.iter().map(|s| json!(s)).collect(),
                    ),
                ]
            },
            render: render_bottom_sheet,
        },
    ]
}
