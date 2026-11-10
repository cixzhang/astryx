//! Wave 4 families: light overlays, shown open.

use super::{v, Family};
use crate::props::Props;
use crate::Msg;
use astryx_core::{Element, Space};
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

/// Room around the trigger so the open layer stays inside the case box.
fn stage(child: Element<Msg>) -> Element<Msg> {
    Center::new()
        .width(SizeValue::Points(360.0))
        .height(SizeValue::Points(200.0))
        .child(child)
        .into()
}

fn render_tooltip(p: Props) -> Element<Msg> {
    stage(
        Tooltip::new(
            "tip",
            Button::new("Hover me"),
            p.str_or("content", "Saves your changes"),
        )
        .placement(side(p, OverlaySide::Above))
        .alignment(align(p, OverlayAlign::Center))
        .is_enabled(p.get("isEnabled").and_then(|v| v.as_bool()).unwrap_or(true))
        .is_open(true)
        .on_open_change(|_| Msg::Noop)
        .into(),
    )
}

fn render_info_tip(p: Props) -> Element<Msg> {
    let size = super::wave1::icon_size(p.str_or("size", "sm"));
    stage(
        HStack::new()
            .gap(Space::S1)
            .v_align(StackAxisAlign::Center)
            .child(Text::new("Visibility"))
            .child(
                InfoTip::new("info", p.str_or("content", "Who can see this project"))
                    .size(size)
                    .is_open(p.get("isOpen").and_then(|v| v.as_bool()).unwrap_or(true))
                    .on_open_change(|_| Msg::Noop),
            )
            .into(),
    )
}

fn render_hover_card(p: Props) -> Element<Msg> {
    stage(
        HoverCard::new(
            "card",
            Link::new("Ada Lovelace").href("#"),
            VStack::new()
                .gap(Space::S2)
                .child(
                    HStack::new()
                        .gap(Space::S2)
                        .v_align(StackAxisAlign::Center)
                        .child(Avatar::new().name("Ada Lovelace"))
                        .child(Text::new("Ada Lovelace").r#type(TextType::Label)),
                )
                .child(Text::new("Mathematician and writer.").color(TextColor::Secondary)),
        )
        .placement(side(p, OverlaySide::Above))
        .alignment(align(p, OverlayAlign::Center))
        .label("Ada Lovelace")
        .is_open(true)
        .on_open_change(|_| Msg::Noop)
        .into(),
    )
}

fn render_popover(p: Props) -> Element<Msg> {
    let mut pop = Popover::new(
        "pop",
        Button::new("Filters"),
        VStack::new()
            .gap(Space::S2)
            .child(Text::new("Filter results").r#type(TextType::Label))
            .child(Switch::new("Only mine", true).on_change(|_| Msg::Noop)),
    )
    .placement(side(p, OverlaySide::Below))
    .alignment(align(p, OverlayAlign::Start))
    .label("Filters")
    .is_open(p.get("isOpen").and_then(|v| v.as_bool()).unwrap_or(true))
    .on_open_change(|_| Msg::Noop);
    if let Some(s) = p.space("padding") {
        pop = pop.padding(s);
    }
    if let Some(w) = p.num("width") {
        pop = pop.width(PopoverWidth::Points(w as f32));
    }
    stage(pop.into())
}

fn render_bottom_sheet(p: Props) -> Element<Msg> {
    let mut s = BottomSheet::new("Share", true)
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
        .on_open_change(|_| Msg::Noop)
        .child(Heading::new(2, "Share"))
        .child(Text::new("Anyone with the link can view.").color(TextColor::Secondary));
    if let Some(pad) = p.space("padding") {
        s = s.padding(pad);
    }
    VStack::new()
        .child(Text::new("Page behind the sheet"))
        .child(s)
        .into()
}

pub fn families() -> Vec<Family> {
    vec![
        Family {
            name: "Tooltip",
            base: || json!({}),
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
            base: || json!({}),
            sweeps: || vec![("size", v(&["xsm", "sm", "md", "lg"]))],
            render: render_info_tip,
        },
        Family {
            name: "HoverCard",
            base: || json!({}),
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
            base: || json!({}),
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
            base: || json!({}),
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
