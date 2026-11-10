//! Families for the QA case files of waves 7 and 8 that have no family of
//! their own yet: dates and times, selectors, NumberInput, Tokenizer,
//! Resizable, and the navigation frame.

use super::{kid_text, kids, v, Family};
use crate::props::Props;
use crate::Msg;
use astryx_core::Element;
use astryx_spike_foundations::ComponentSize;
use astryx_spike_ui::*;
use serde_json::json;

fn size(p: Props) -> Option<ComponentSize> {
    p.str("size").map(|s| match s {
        "sm" => ComponentSize::Sm,
        "lg" => ComponentSize::Lg,
        _ => ComponentSize::Md,
    })
}

fn render_calendar(p: Props) -> Element<Msg> {
    let value = if p.str("mode") == Some("range") {
        CalendarValue::Range(None)
    } else {
        CalendarValue::Single(None)
    };
    Calendar::new("calendar", value)
        .number_of_months(p.num("numberOfMonths").unwrap_or(1.0) as u32)
        .has_outside_days(
            p.get("hasOutsideDays")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
        )
        .has_week_numbers(p.bool("hasWeekNumbers"))
        .has_variable_row_count(p.bool("hasVariableRowCount"))
        .focus_date(CivilDate::new(2026, 11, 1))
        .on_change(|_| Msg::Noop)
        .into()
}

fn render_date_input(p: Props) -> Element<Msg> {
    let mut d = if p.str("$component") == Some("DateRangeInput") {
        DateInput::range("date", p.str_or("label", "Label"), None)
    } else {
        DateInput::single("date", p.str_or("label", "Label"), None)
    };
    d = d.is_disabled(p.bool("isDisabled")).on_change(|_| Msg::Noop);
    if let Some(b) = p.get("hasClear").and_then(|v| v.as_bool()) {
        d = d.has_clear(b);
    }
    if let Some(n) = p.num("numberOfMonths") {
        d = d.number_of_months(n as u32);
    }
    d.into()
}

fn render_time_input(p: Props) -> Element<Msg> {
    TimeInput::new("time", p.str_or("label", "Label"), None)
        .twelve_hour(p.str("hourFormat") != Some("24h"))
        .is_disabled(p.bool("isDisabled"))
        .on_change(|_| Msg::Noop)
        .into()
}

fn render_date_time_input(p: Props) -> Element<Msg> {
    DateTimeInput::new("datetime", p.str_or("label", "Label"), None, None)
        .on_change(|_, _| Msg::Noop)
        .into()
}

fn selector_variant(p: Props) -> SelectorVariant {
    match p.str_or("variant", "input") {
        "secondary" => SelectorVariant::Secondary,
        "ghost" => SelectorVariant::Ghost,
        _ => SelectorVariant::Input,
    }
}

fn render_selector(p: Props) -> Element<Msg> {
    let mut s = Selector::single("selector", p.str_or("label", "Label"), Vec::new(), None)
        .variant(selector_variant(p))
        .is_disabled(p.bool("isDisabled"))
        .is_label_hidden(p.bool("isLabelHidden"))
        .on_change(|_| Msg::Noop);
    if let Some(sz) = size(p) {
        s = s.size(sz);
    }
    s.into()
}

fn render_multi_selector(p: Props) -> Element<Msg> {
    let mut s = Selector::multiple("multi", p.str_or("label", "Label"), Vec::new(), Vec::new())
        .variant(selector_variant(p))
        .is_disabled(p.bool("isDisabled"))
        .is_label_hidden(p.bool("isLabelHidden"))
        .has_select_all(p.bool("hasSelectAll"))
        .trigger_display(match p.str_or("triggerDisplay", "count") {
            "labels" => TriggerDisplay::Labels,
            "badges" => TriggerDisplay::Badges,
            _ => TriggerDisplay::Count,
        })
        .on_change(|_| Msg::Noop);
    if let Some(sz) = size(p) {
        s = s.size(sz);
    }
    s.into()
}

fn render_number_input(p: Props) -> Element<Msg> {
    let mut n = NumberInput::new("number", p.str_or("label", "Label"), None)
        .has_number_steppers(p.bool("hasNumberSteppers"))
        .is_disabled(p.bool("isDisabled"))
        .is_read_only(p.bool("isReadOnly"))
        .is_label_hidden(p.bool("isLabelHidden"))
        .is_integer_only(p.bool("isIntegerOnly"))
        .on_change(|_| Msg::Noop);
    if let Some(sz) = size(p) {
        n = n.size(sz);
    }
    n.into()
}

fn render_tokenizer(p: Props) -> Element<Msg> {
    Tokenizer::new(
        "tokens",
        p.str_or("label", "Label"),
        static_source(Vec::<Entry>::new()),
        Vec::new(),
        |_| Msg::Noop,
    )
    .is_disabled(p.bool("isDisabled"))
    .into()
}

fn render_resizable(p: Props) -> Element<Msg> {
    match p.str("$component") {
        Some("ResizeHandle") => {
            let mut h = ResizeHandle::new(p.str_or("label", "Label"))
                .direction(if p.str("direction") == Some("vertical") {
                    ResizeDirection::Vertical
                } else {
                    ResizeDirection::Horizontal
                })
                .has_divider(p.bool("hasDivider"))
                .is_disabled(p.bool("isDisabled"));
            for k in kids(&p) {
                h = h.child(k);
            }
            h.into()
        }
        // `Resizable` and `useResizable` are hooks and factories: they draw
        // nothing by themselves.
        _ => VStack::new().into(),
    }
}

fn render_mobile_nav(p: Props) -> Element<Msg> {
    if p.str("$component") == Some("MobileNavToggle") {
        return super::inline(
            Button::new(p.str_or("label", "Label"))
                .variant(ButtonVariant::Ghost)
                .icon(Icon::new("menu"))
                .is_icon_only(true),
        );
    }
    let mut m =
        MobileNav::new(p.bool("isOpen"), |_| None::<Msg>).side(if p.str("side") == Some("end") {
            MobileNavSide::End
        } else {
            MobileNavSide::Start
        });
    if let Some(t) = kid_text(&p) {
        m = m.child(Text::new(t));
    }
    m.into()
}

fn render_side_nav(p: Props) -> Element<Msg> {
    let mut s = SideNav::new();
    if let Some(t) = kid_text(&p) {
        s = s.child(Text::new(t));
    }
    s.into()
}

fn render_top_nav(p: Props) -> Element<Msg> {
    let mut t = TopNav::new().label(p.str_or("label", "Top navigation"));
    if let Some(words) = kid_text(&p) {
        t = t.start_content(Text::new(words));
    }
    t.into()
}

fn render_chat_composer(p: Props) -> Element<Msg> {
    let mut c = ChatComposer::new("composer", |_| Msg::Noop)
        .is_stop_shown(p.bool("isStopShown"))
        .is_disabled(p.bool("isDisabled"))
        .is_elevated(p.str("elevation") != Some("none"))
        .density(match p.str_or("density", "balanced") {
            "compact" => ListDensity::Compact,
            "spacious" => ListDensity::Spacious,
            _ => ListDensity::Balanced,
        });
    if let Some(v) = p.str("value") {
        c = c.value(v).on_change(|_| Msg::Noop);
    }
    if let Some(ph) = p.str("placeholder") {
        c = c.placeholder(ph);
    }
    c.into()
}

pub fn families() -> Vec<Family> {
    vec![
        Family {
            name: "Calendar",
            base: || json!({}),
            sweeps: || vec![("mode", v(&["single", "range"]))],
            render: render_calendar,
        },
        Family {
            name: "DateInput",
            base: || json!({"label": "Label"}),
            sweeps: || vec![],
            render: render_date_input,
        },
        Family {
            name: "DateRangeInput",
            base: || json!({"label": "Label", "$component": "DateRangeInput"}),
            sweeps: || vec![],
            render: render_date_input,
        },
        Family {
            name: "TimeInput",
            base: || json!({"label": "Label"}),
            sweeps: || vec![],
            render: render_time_input,
        },
        Family {
            name: "DateTimeInput",
            base: || json!({"label": "Label"}),
            sweeps: || vec![],
            render: render_date_time_input,
        },
        Family {
            name: "Selector",
            base: || json!({"label": "Label"}),
            sweeps: || vec![("variant", v(&["input", "secondary", "ghost"]))],
            render: render_selector,
        },
        Family {
            name: "MultiSelector",
            base: || json!({"label": "Label"}),
            sweeps: || vec![("triggerDisplay", v(&["count", "labels", "badges"]))],
            render: render_multi_selector,
        },
        Family {
            name: "NumberInput",
            base: || json!({"label": "Label"}),
            sweeps: || vec![("hasNumberSteppers", vec![json!(true)])],
            render: render_number_input,
        },
        Family {
            name: "Tokenizer",
            base: || json!({"label": "Label"}),
            sweeps: || vec![],
            render: render_tokenizer,
        },
        Family {
            name: "Resizable",
            base: || json!({"$component": "ResizeHandle", "label": "Label", "$fixture": "boxes"}),
            sweeps: || vec![("direction", v(&["horizontal", "vertical"]))],
            render: render_resizable,
        },
        Family {
            name: "MobileNav",
            base: || json!({"isOpen": true, "$fixture": "text"}),
            sweeps: || vec![("side", v(&["start", "end"]))],
            render: render_mobile_nav,
        },
        Family {
            name: "SideNav",
            base: || json!({"$fixture": "text"}),
            sweeps: || vec![],
            render: render_side_nav,
        },
        Family {
            name: "ChatComposer",
            base: || json!({}),
            sweeps: || {
                vec![
                    ("density", v(&["compact", "balanced", "spacious"])),
                    ("isStopShown", vec![json!(true)]),
                    ("value", v(&["Draft a reply about the launch plan"])),
                    ("elevation", v(&["none", "low"])),
                ]
            },
            render: render_chat_composer,
        },
        Family {
            name: "TopNav",
            base: || json!({"label": "Label", "$fixture": "text"}),
            sweeps: || vec![],
            render: render_top_nav,
        },
    ]
}
