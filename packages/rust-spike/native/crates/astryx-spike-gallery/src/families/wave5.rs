//! Wave 5 families: controls.

use super::{v, Family};
use crate::props::Props;
use crate::Msg;
use astryx_core::{Element, Space};
use astryx_spike_foundations::ComponentSize;
use astryx_spike_ui::*;
use serde_json::{json, Value};

fn size(p: Props) -> Option<ComponentSize> {
    match p.str("size")? {
        "sm" => Some(ComponentSize::Sm),
        "lg" => Some(ComponentSize::Lg),
        "md" => Some(ComponentSize::Md),
        _ => None,
    }
}

fn variant(p: Props) -> ButtonVariant {
    match p.str_or("variant", "secondary") {
        "primary" => ButtonVariant::Primary,
        "ghost" => ButtonVariant::Ghost,
        "destructive" => ButtonVariant::Destructive,
        _ => ButtonVariant::Secondary,
    }
}

fn status(p: Props) -> Option<Status> {
    let (kind, message) = match p.get("status") {
        Some(Value::Object(o)) => (
            o.get("type").and_then(Value::as_str)?,
            o.get("message").and_then(Value::as_str).map(String::from),
        ),
        Some(Value::String(s)) => (s.as_str(), Some(format!("This is a {s} message"))),
        _ => match p.str("statusVariant") {
            Some(s @ ("error" | "warning" | "success")) => {
                (s, Some(format!("This is a {s} message")))
            }
            _ => return None,
        },
    };
    let t = match kind {
        "error" => StatusType::Error,
        "warning" => StatusType::Warning,
        "success" => StatusType::Success,
        _ => return None,
    };
    Some(Status::new(t, message))
}

fn render_button(p: Props) -> Element<Msg> {
    let mut b = Button::new(p.words("Label"))
        .variant(variant(p))
        .elevation(super::wave2::elevation(p.str_or("elevation", "none")))
        .is_loading(p.bool("isLoading"))
        .is_disabled(p.bool("isDisabled"))
        .is_icon_only(p.bool("isIconOnly"))
        .on_click(Msg::Noop);
    if let Some(s) = size(p) {
        b = b.size(s);
    }
    if p.bool("isIconOnly") || p.get("icon").is_some() {
        b = b.icon(Icon::new(p.str("icon").unwrap_or("check")));
    }
    if let Some(w) = p.size("width") {
        b = b.width(w);
    }
    if p.get("endContent").is_some() {
        b = b.end_content(Badge::new("3"));
    }
    b.into()
}

fn render_icon_button(p: Props) -> Element<Msg> {
    let mut b = IconButton::new(
        p.words("Settings"),
        Icon::new(p.str("icon").unwrap_or("wrench")),
    )
    .variant(variant(p))
    .elevation(super::wave2::elevation(p.str_or("elevation", "none")))
    .is_loading(p.bool("isLoading"))
    .is_disabled(p.bool("isDisabled"))
    .on_click(Msg::Noop);
    if let Some(s) = size(p) {
        b = b.size(s);
    }
    b.into()
}

fn render_button_group(p: Props) -> Element<Msg> {
    let mut g = ButtonGroup::new(p.str_or("label", "Text formatting"))
        .orientation(if p.str("orientation") == Some("vertical") {
            Orientation::Vertical
        } else {
            Orientation::Horizontal
        })
        .is_disabled(p.bool("isDisabled"))
        .elevation(super::wave2::elevation(p.str_or("elevation", "none")));
    if let Some(s) = size(p) {
        g = g.size(s);
    }
    let v = variant(p);
    g.child(Button::new("Bold").variant(v))
        .child(Button::new("Italic").variant(v))
        .child(Button::new("Underline").variant(v))
        .into()
}

fn render_switch(p: Props) -> Element<Msg> {
    let mut s = Switch::new(
        p.words("Notifications"),
        p.bool("value") || !p.get("value").is_some_and(|v| v == &json!(false)),
    )
    .is_loading(p.bool("isLoading"))
    .is_label_hidden(p.bool("isLabelHidden"))
    .is_disabled(p.bool("isDisabled"))
    .size(if p.str("size") == Some("sm") {
        SwitchSize::Sm
    } else {
        SwitchSize::Md
    })
    .label_position(if p.str("labelPosition") == Some("start") {
        LabelPosition::Start
    } else {
        LabelPosition::End
    })
    .label_spacing(if p.str("labelSpacing") == Some("spread") {
        LabelSpacing::Spread
    } else {
        LabelSpacing::Hug
    })
    .on_change(|_| Msg::Noop);
    if p.str("labelSpacing") == Some("spread") {
        s = s.width(SizeValue::Points(280.0));
    }
    if let Some(d) = p.str("description") {
        s = s.description(d);
    }
    if let Some(st) = status(p) {
        s = s.status(st);
    }
    s.is_optional(p.bool("isOptional"))
        .is_required(p.bool("isRequired"))
        .into()
}

fn render_text_input(p: Props) -> Element<Msg> {
    let value = p
        .str("value")
        .unwrap_or(if p.bool("hasClear") { "Hello" } else { "" });
    let mut t = TextInput::new(p.words("Name"), value)
        .placeholder(p.str_or("placeholder", "Placeholder"))
        .is_label_hidden(p.bool("isLabelHidden"))
        .is_disabled(p.bool("isDisabled"))
        .is_read_only(p.bool("isReadOnly"))
        .is_loading(p.bool("isLoading"))
        .has_clear(p.bool("hasClear"))
        .is_optional(p.bool("isOptional"))
        .is_required(p.bool("isRequired"))
        .width(SizeValue::Points(280.0))
        .on_change(|_| Msg::Noop);
    if let Some(s) = size(p) {
        t = t.size(s);
    }
    if let Some(st) = status(p) {
        t = t.status(st);
    }
    if let Some(d) = p.str("description") {
        t = t.description(d);
    }
    if p.get("startIcon").is_some() {
        t = t.start_icon(Icon::new("search"));
    }
    t.into()
}

fn checkbox_state(p: Props) -> CheckboxState {
    match p.get("value") {
        Some(Value::String(s)) if s == "indeterminate" => CheckboxState::Indeterminate,
        Some(Value::Bool(false)) => CheckboxState::Unchecked,
        _ => CheckboxState::Checked,
    }
}

fn render_checkbox(p: Props) -> Element<Msg> {
    let mut c = CheckboxInput::new(p.words("Accept the terms"), checkbox_state(p))
        .is_label_hidden(p.bool("isLabelHidden"))
        .is_loading(p.bool("isLoading"))
        .is_disabled(p.bool("isDisabled"))
        .is_read_only(p.bool("isReadOnly"))
        .size(if p.str("size") == Some("sm") {
            ChoiceSize::Sm
        } else {
            ChoiceSize::Md
        })
        .on_change(|_| Msg::Noop);
    if let Some(d) = p.str("description") {
        c = c.description(d);
    }
    if let Some(st) = status(p) {
        c = c.status(st);
    }
    c.into()
}

fn density(p: Props) -> ListDensity {
    match p.str_or("density", "balanced") {
        "compact" => ListDensity::Compact,
        "spacious" => ListDensity::Spacious,
        _ => ListDensity::Balanced,
    }
}

fn align(p: Props) -> ChoiceAlign {
    match p.str_or("align", "center") {
        "start" => ChoiceAlign::Start,
        "first-line" => ChoiceAlign::FirstLine,
        _ => ChoiceAlign::Center,
    }
}

fn render_checkbox_list(p: Props) -> Element<Msg> {
    let item_disabled = p.bool("isDisabled") && p.str("$component") == Some("CheckboxListItem");
    CheckboxList::new(p.str_or("label", "Notifications"), vec!["email".into()])
        .item(
            CheckboxListItem::new("Email", "email")
                .description("Daily digest")
                .align(align(p))
                .is_disabled(item_disabled),
        )
        .item(CheckboxListItem::new("Push", "push").align(align(p)))
        .item(CheckboxListItem::new("SMS", "sms").align(align(p)))
        .density(density(p))
        .has_dividers(p.bool("hasDividers"))
        .is_disabled(p.bool("isDisabled") && !item_disabled)
        .is_read_only(p.bool("isReadOnly"))
        .is_label_hidden(p.bool("isLabelHidden"))
        .width(SizeValue::Points(280.0))
        .on_change(|_| Msg::Noop)
        .into()
}

fn render_radio_list(p: Props) -> Element<Msg> {
    let item_disabled = p.bool("isDisabled") && p.str("$component") == Some("RadioListItem");
    RadioList::new(p.str_or("label", "Plan"), "pro")
        .item(
            RadioListItem::new("Free", "free")
                .align(align(p))
                .is_disabled(item_disabled),
        )
        .item(
            RadioListItem::new("Pro", "pro")
                .description("For teams")
                .align(align(p)),
        )
        .item(RadioListItem::new("Enterprise", "ent").align(align(p)))
        .orientation(if p.str("orientation") == Some("horizontal") {
            RadioOrientation::Horizontal
        } else {
            RadioOrientation::Vertical
        })
        .size(if p.str("size") == Some("sm") {
            ChoiceSize::Sm
        } else {
            ChoiceSize::Md
        })
        .is_disabled(p.bool("isDisabled") && !item_disabled)
        .is_label_hidden(p.bool("isLabelHidden"))
        .on_change(|_| Msg::Noop)
        .into()
}

pub(crate) fn token_color(s: &str) -> TokenColor {
    TokenColor::ALL
        .into_iter()
        .find(|c| format!("{c:?}").eq_ignore_ascii_case(s))
        .unwrap_or_default()
}

fn render_token(p: Props) -> Element<Msg> {
    let mut t = Token::new(p.words("Design"))
        .color(token_color(p.str_or("color", "default")))
        .is_disabled(p.bool("isDisabled"))
        .is_label_hidden(p.bool("isLabelHidden"))
        .on_remove(Msg::Noop);
    if let Some(s) = size(p) {
        t = t.size(s);
    }
    if p.bool("isLabelHidden") || p.get("icon").is_some() {
        t = t.icon(Icon::new("check"));
    }
    t.into()
}

fn render_selectable_card(p: Props) -> Element<Msg> {
    let mut c = SelectableCard::new(
        p.words("Starter"),
        p.get("isSelected").and_then(Value::as_bool).unwrap_or(true),
    )
    .is_disabled(p.bool("isDisabled"))
    .variant(super::wave2::card_variant(p.str_or("variant", "default")))
    .elevation(super::wave2::elevation(p.str_or("elevation", "none")))
    .width(p.size("width").unwrap_or(SizeValue::Points(260.0)))
    .on_change(|_| Msg::Noop)
    .child(Text::new("Starter").r#type(TextType::Label))
    .child(Text::new("For small projects.").color(TextColor::Secondary));
    if let Some(s) = p.space("padding") {
        c = c.padding(s);
    }
    c.into()
}

fn render_segmented(p: Props) -> Element<Msg> {
    let hidden = p.bool("isLabelHidden");
    let item = |v: &str, l: &str, i: &str| {
        let mut it = SegmentedControlItem::new(v, l).is_label_hidden(hidden);
        if hidden || p.get("icon").is_some() {
            it = it.icon(Icon::new(i));
        }
        it
    };
    let mut s = SegmentedControl::new(p.str_or("label", "View"), "list")
        .item(item("list", "List", "menu"))
        .item(item("grid", "Grid", "viewColumns").is_disabled(
            p.bool("isDisabled") && p.str("$component") == Some("SegmentedControlItem"),
        ))
        .item(item("table", "Table", "copy"))
        .layout(if p.str("layout") == Some("fill") {
            SegmentedLayout::Fill
        } else {
            SegmentedLayout::Hug
        })
        .is_disabled(p.bool("isDisabled") && p.str("$component") != Some("SegmentedControlItem"))
        .on_change(|_| Msg::Noop);
    if let Some(sz) = size(p) {
        s = s.size(sz);
    }
    let el: Element<Msg> = s.into();
    if p.str("layout") == Some("fill") {
        VStack::new()
            .width(SizeValue::Points(360.0))
            .child(el)
            .into()
    } else {
        el
    }
}

fn render_input_group(p: Props) -> Element<Msg> {
    let mut g = InputGroup::new(p.str_or("label", "Website"))
        .is_label_hidden(p.bool("isLabelHidden"))
        .is_disabled(p.bool("isDisabled"))
        .is_optional(p.bool("isOptional"))
        .is_required(p.bool("isRequired"))
        .child(InputGroupText::new(p.str_or("children", "https://")))
        .child(TextInput::new("Website", "example.com").on_change(|_| Msg::Noop))
        .child(Button::new("Copy"));
    if let Some(s) = size(p) {
        g = g.size(s);
    }
    if let Some(d) = p.str("description") {
        g = g.description(d);
    }
    if let Some(st) = status(p) {
        g = g.status(st);
    }
    VStack::new()
        .width(SizeValue::Points(360.0))
        .child(g)
        .into()
}

fn render_field(p: Props) -> Element<Msg> {
    if p.str("$component") == Some("FieldStatus") {
        let t = match p.str_or("type", "error") {
            "warning" => StatusType::Warning,
            "success" => StatusType::Success,
            _ => StatusType::Error,
        };
        return FieldStatus::new(t, p.str_or("message", "Something needs attention"))
            .variant(if p.str("variant") == Some("detached") {
                StatusVariant::Detached
            } else {
                StatusVariant::Attached
            })
            .into();
    }
    let mut f = Field::new(
        p.str_or("label", "Project name"),
        Card::new()
            .padding(Space::S2)
            .child(Text::new("Custom control")),
    )
    .is_label_hidden(p.bool("isLabelHidden"))
    .is_disabled(p.bool("isDisabled"))
    .is_optional(p.bool("isOptional"))
    .is_required(p.bool("isRequired"))
    .width(SizeValue::Points(280.0));
    if let Some(d) = p.str("description") {
        f = f.description(d);
    }
    if let Some(st) = status(p) {
        f = f.status(st);
    }
    if let Some(t) = p.str("labelTooltip") {
        f = f.label_tooltip(t);
    }
    f.into()
}

fn render_form_layout(p: Props) -> Element<Msg> {
    let d = match p.str_or("direction", "vertical") {
        "horizontal" => FormDirection::Horizontal,
        "horizontal-labels" => FormDirection::HorizontalLabels,
        _ => FormDirection::Vertical,
    };
    VStack::new()
        .width(SizeValue::Points(420.0))
        .child(
            FormLayout::new()
                .direction(d)
                .child(TextInput::new("First name", "Ada").on_change(|_| Msg::Noop))
                .child(TextInput::new("Last name", "Lovelace").on_change(|_| Msg::Noop)),
        )
        .into()
}

pub fn families() -> Vec<Family> {
    vec![
        Family {
            name: "Button",
            base: || json!({"label": "Label"}),
            sweeps: || {
                vec![
                    (
                        "variant",
                        v(&["primary", "secondary", "ghost", "destructive"]),
                    ),
                    ("size", v(&["sm", "md", "lg"])),
                    ("elevation", v(&["none", "low", "med", "high"])),
                    ("isLoading", vec![json!(true)]),
                    ("isDisabled", vec![json!(true)]),
                    ("isIconOnly", vec![json!(true)]),
                ]
            },
            render: render_button,
        },
        Family {
            name: "IconButton",
            base: || json!({"label": "Settings"}),
            sweeps: || {
                vec![
                    (
                        "variant",
                        v(&["primary", "secondary", "ghost", "destructive"]),
                    ),
                    ("size", v(&["sm", "md", "lg"])),
                    ("elevation", v(&["none", "low", "med", "high"])),
                    ("isLoading", vec![json!(true)]),
                    ("isDisabled", vec![json!(true)]),
                ]
            },
            render: render_icon_button,
        },
        Family {
            name: "ButtonGroup",
            base: || json!({}),
            sweeps: || {
                vec![
                    ("orientation", v(&["horizontal", "vertical"])),
                    ("size", v(&["sm", "md", "lg"])),
                    ("isDisabled", vec![json!(true)]),
                    ("elevation", v(&["none", "low", "med", "high"])),
                    ("variant", v(&["primary", "ghost"])),
                ]
            },
            render: render_button_group,
        },
        Family {
            name: "Switch",
            base: || json!({"label": "Notifications", "value": true}),
            sweeps: || {
                vec![
                    ("value", vec![json!(false)]),
                    ("isLoading", vec![json!(true)]),
                    ("isLabelHidden", vec![json!(true)]),
                    ("isDisabled", vec![json!(true)]),
                    ("size", v(&["sm", "md"])),
                    ("labelPosition", v(&["start", "end"])),
                    ("labelSpacing", v(&["hug", "spread"])),
                    ("description", v(&["Get alerts on this device"])),
                    ("status", v(&["error", "warning", "success"])),
                ]
            },
            render: render_switch,
        },
        Family {
            name: "TextInput",
            base: || json!({"label": "Name"}),
            sweeps: || {
                vec![
                    ("size", v(&["sm", "md", "lg"])),
                    ("isLabelHidden", vec![json!(true)]),
                    ("isDisabled", vec![json!(true)]),
                    ("isReadOnly", vec![json!(true)]),
                    ("isLoading", vec![json!(true)]),
                    ("statusVariant", v(&["error", "warning", "success"])),
                    ("hasClear", vec![json!(true)]),
                    ("isRequired", vec![json!(true)]),
                    ("description", v(&["As shown on your ID"])),
                ]
            },
            render: render_text_input,
        },
        Family {
            name: "CheckboxInput",
            base: || json!({}),
            sweeps: || {
                vec![
                    ("value", vec![json!(false), json!("indeterminate")]),
                    ("isLabelHidden", vec![json!(true)]),
                    ("isLoading", vec![json!(true)]),
                    ("isDisabled", vec![json!(true)]),
                    ("isReadOnly", vec![json!(true)]),
                    ("size", v(&["sm", "md"])),
                ]
            },
            render: render_checkbox,
        },
        Family {
            name: "CheckboxList",
            base: || json!({}),
            sweeps: || {
                vec![
                    ("density", v(&["compact", "balanced", "spacious"])),
                    ("hasDividers", vec![json!(true)]),
                    ("isDisabled", vec![json!(true)]),
                    ("isReadOnly", vec![json!(true)]),
                    ("isLabelHidden", vec![json!(true)]),
                    ("align", v(&["center", "start", "first-line"])),
                ]
            },
            render: render_checkbox_list,
        },
        Family {
            name: "RadioList",
            base: || json!({}),
            sweeps: || {
                vec![
                    ("orientation", v(&["vertical", "horizontal"])),
                    ("isDisabled", vec![json!(true)]),
                    ("isLabelHidden", vec![json!(true)]),
                    ("size", v(&["sm", "md"])),
                    ("align", v(&["center", "start", "first-line"])),
                ]
            },
            render: render_radio_list,
        },
        Family {
            name: "Token",
            base: || json!({"label": "Design"}),
            sweeps: || {
                vec![
                    ("size", v(&["sm", "md", "lg"])),
                    (
                        "color",
                        v(&[
                            "default", "red", "orange", "yellow", "green", "teal", "cyan", "blue",
                            "purple", "pink", "gray",
                        ]),
                    ),
                    ("isDisabled", vec![json!(true)]),
                    ("isLabelHidden", vec![json!(true)]),
                ]
            },
            render: render_token,
        },
        Family {
            name: "SelectableCard",
            base: || json!({"isSelected": true}),
            sweeps: || {
                vec![
                    ("isSelected", vec![json!(false)]),
                    ("isDisabled", vec![json!(true)]),
                    (
                        "padding",
                        crate::props::SPACE_STEPS.iter().map(|s| json!(s)).collect(),
                    ),
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
            render: render_selectable_card,
        },
        Family {
            name: "SegmentedControl",
            base: || json!({}),
            sweeps: || {
                vec![
                    ("size", v(&["sm", "md", "lg"])),
                    ("layout", v(&["hug", "fill"])),
                    ("isDisabled", vec![json!(true)]),
                    ("isLabelHidden", vec![json!(true)]),
                ]
            },
            render: render_segmented,
        },
        Family {
            name: "InputGroup",
            base: || json!({}),
            sweeps: || {
                vec![
                    ("size", v(&["sm", "md", "lg"])),
                    ("isLabelHidden", vec![json!(true)]),
                    ("isDisabled", vec![json!(true)]),
                    ("status", v(&["error", "warning", "success"])),
                ]
            },
            render: render_input_group,
        },
        Family {
            name: "Field",
            base: || json!({}),
            sweeps: || {
                vec![
                    ("isLabelHidden", vec![json!(true)]),
                    ("isDisabled", vec![json!(true)]),
                    ("statusVariant", v(&["error", "warning", "success"])),
                    ("isOptional", vec![json!(true)]),
                    ("isRequired", vec![json!(true)]),
                    ("labelTooltip", v(&["Shown to your team"])),
                ]
            },
            render: render_field,
        },
        Family {
            name: "FieldStatus",
            base: || json!({"$component": "FieldStatus", "type": "error", "message": "Enter a valid email"}),
            sweeps: || {
                vec![
                    ("type", v(&["error", "warning", "success"])),
                    ("variant", v(&["attached", "detached"])),
                ]
            },
            render: render_field,
        },
        Family {
            name: "FormLayout",
            base: || json!({}),
            sweeps: || {
                vec![(
                    "direction",
                    v(&["vertical", "horizontal", "horizontal-labels"]),
                )]
            },
            render: render_form_layout,
        },
    ]
}
