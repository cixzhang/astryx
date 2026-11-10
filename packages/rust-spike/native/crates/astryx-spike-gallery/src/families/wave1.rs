//! Wave 1 families: primitives.

use super::{v, Family};
use crate::props::Props;
use crate::Msg;
use astryx_core::Element;
use astryx_spike_ui::*;
use serde_json::json;

fn text_type(s: &str) -> TextType {
    match s {
        "large" => TextType::Large,
        "label" => TextType::Label,
        "supporting" => TextType::Supporting,
        "code" => TextType::Code,
        "display-1" => TextType::Display1,
        "display-2" => TextType::Display2,
        "display-3" => TextType::Display3,
        "inherit" => TextType::Inherit,
        _ => TextType::Body,
    }
}

pub(crate) fn text_size(s: &str) -> Option<TextSize> {
    Some(match s {
        "4xs" => TextSize::S4xs,
        "3xs" => TextSize::S3xs,
        "2xs" => TextSize::S2xs,
        "xsm" => TextSize::Xsm,
        "sm" => TextSize::Sm,
        "base" => TextSize::Base,
        "lg" => TextSize::Lg,
        "xl" => TextSize::Xl,
        "2xl" => TextSize::S2xl,
        "3xl" => TextSize::S3xl,
        "4xl" => TextSize::S4xl,
        _ => return None,
    })
}

pub(crate) fn text_color(s: &str) -> Option<TextColor> {
    Some(match s {
        "primary" => TextColor::Primary,
        "secondary" => TextColor::Secondary,
        "disabled" => TextColor::Disabled,
        "placeholder" => TextColor::Placeholder,
        "accent" => TextColor::Accent,
        "inherit" => TextColor::Inherit,
        _ => return None,
    })
}

pub(crate) fn text_weight(s: &str) -> Option<TextWeight> {
    Some(match s {
        "normal" => TextWeight::Normal,
        "medium" => TextWeight::Medium,
        "semibold" => TextWeight::Semibold,
        "bold" => TextWeight::Bold,
        _ => return None,
    })
}

fn justify(s: &str) -> TextJustify {
    match s {
        "center" => TextJustify::Center,
        "end" => TextJustify::End,
        _ => TextJustify::Start,
    }
}

fn render_text(p: Props) -> Element<Msg> {
    let mut t = Text::new(p.words("The quick brown fox jumps over the lazy dog."))
        .r#type(text_type(p.str_or("type", "body")));
    if let Some(s) = p.str("size").and_then(text_size) {
        t = t.size(s);
    }
    if let Some(c) = p.str("color").and_then(text_color) {
        t = t.color(c);
    }
    if let Some(w) = p.str("weight").and_then(text_weight) {
        t = t.weight(w);
    }
    if let Some(j) = p.str("justify") {
        t = t.justify(justify(j));
    }
    if let Some(n) = p.num("maxLines") {
        t = t.max_lines(n as u32);
    }
    t.has_strikethrough(p.bool("hasStrikethrough")).into()
}

fn render_heading(p: Props) -> Element<Msg> {
    let level = p.num("level").unwrap_or(2.0) as u8;
    let mut h = Heading::new(level, p.words("Heading"));
    if let Some(c) = p.str("color").and_then(text_color) {
        h = h.color(c);
    }
    if let Some(w) = p.str("weight").and_then(text_weight) {
        h = h.weight(w);
    }
    h = match p.str("type") {
        Some("display-1") => h.r#type(HeadingType::Display1),
        Some("display-2") => h.r#type(HeadingType::Display2),
        Some("display-3") => h.r#type(HeadingType::Display3),
        _ => h,
    };
    h.into()
}

pub(crate) fn icon_color(s: &str) -> IconColor {
    IconColor::ALL
        .into_iter()
        .find(|c| format!("{c:?}").eq_ignore_ascii_case(s))
        .unwrap_or_default()
}

pub(crate) fn icon_size(s: &str) -> IconSize {
    match s {
        "xsm" => IconSize::Xsm,
        "sm" => IconSize::Sm,
        "lg" => IconSize::Lg,
        _ => IconSize::Md,
    }
}

fn render_icon(p: Props) -> Element<Msg> {
    let mut i =
        Icon::new(p.str_or("icon", "check")).color(icon_color(p.str_or("color", "inherit")));
    if let Some(s) = p.str("size") {
        i = i.size(icon_size(s));
    }
    if let Some(l) = p.str("label") {
        i = i.label(l);
    }
    i.into()
}

fn render_spinner(p: Props) -> Element<Msg> {
    let size = match p.str_or("size", "md") {
        "sm" => SpinnerSize::Sm,
        "lg" => SpinnerSize::Lg,
        "xl" => SpinnerSize::Xl,
        _ => SpinnerSize::Md,
    };
    let shade = match p.str_or("shade", "default") {
        "subtle" => SpinnerShade::Subtle,
        "onMedia" => SpinnerShade::OnMedia,
        "inherit" => SpinnerShade::Inherit,
        _ => SpinnerShade::Default,
    };
    let mut s = Spinner::new().size(size).shade(shade);
    if let Some(l) = p.str("label") {
        s = s.label(l);
    }
    s.into()
}

fn render_link(p: Props) -> Element<Msg> {
    let mut l = Link::new(p.words("Read the docs"))
        .href(p.str_or("href", "https://example.com"))
        .has_underline(p.bool("hasUnderline"))
        .is_disabled(p.bool("isDisabled"))
        .is_external_link(p.bool("isExternalLink"))
        .is_standalone(p.bool("isStandalone"))
        .on_click(Msg::Noop);
    if let Some(c) = p.str("color").and_then(text_color) {
        l = l.color(c);
    }
    if let Some(s) = p.str("size").and_then(text_size) {
        l = l.size(s);
    }
    if let Some(w) = p.str("weight").and_then(text_weight) {
        l = l.weight(w);
    }
    if p.str("display") == Some("block") {
        l = l.display(LinkDisplay::Block);
    }
    l.into()
}

fn badge_variant(s: &str) -> BadgeVariant {
    use BadgeVariant as B;
    [
        B::Neutral,
        B::Info,
        B::Success,
        B::Warning,
        B::Error,
        B::Blue,
        B::Cyan,
        B::Green,
        B::Orange,
        B::Pink,
        B::Purple,
        B::Red,
        B::Teal,
        B::Yellow,
    ]
    .into_iter()
    .find(|b| format!("{b:?}").eq_ignore_ascii_case(s))
    .unwrap_or_default()
}

fn render_badge(p: Props) -> Element<Msg> {
    Badge::new(p.words("Badge"))
        .variant(badge_variant(p.str_or("variant", "neutral")))
        .into()
}

pub(crate) fn avatar_size(s: &str) -> AvatarSize {
    match s {
        "xsm" => AvatarSize::Xsm,
        "sm" => AvatarSize::Sm,
        "lg" => AvatarSize::Lg,
        "xl" => AvatarSize::Xl,
        _ => AvatarSize::Md,
    }
}

pub(crate) fn avatar_shape(s: &str) -> AvatarShape {
    match s {
        "rounded" => AvatarShape::Rounded,
        "square" => AvatarShape::Square,
        _ => AvatarShape::Circle,
    }
}

fn render_avatar(p: Props) -> Element<Msg> {
    let mut a = Avatar::new()
        .name(p.str_or("name", "Ada Lovelace"))
        .size(avatar_size(p.str_or("size", "md")))
        .shape(avatar_shape(p.str_or("shape", "circle")));
    if let Some(s) = p.str("status") {
        let variant = match s {
            "neutral" => AvatarStatusDotVariant::Neutral,
            "error" => AvatarStatusDotVariant::Error,
            _ => AvatarStatusDotVariant::Success,
        };
        a = a.status(AvatarStatusDot::new(variant));
    }
    a.into()
}

fn render_avatar_group(p: Props) -> Element<Msg> {
    let names = [
        "Ada Lovelace",
        "Grace Hopper",
        "Alan Turing",
        "Edsger Dijkstra",
    ];
    let n = p.num("count").unwrap_or(3.0) as usize;
    let mut g = AvatarGroup::new(names.iter().take(n).map(|name| Avatar::new().name(*name)))
        .size(avatar_size(p.str_or("size", "md")))
        .shape(avatar_shape(p.str_or("shape", "circle")));
    if let Some(o) = p.num("overflow") {
        g = g.overflow(AvatarGroupOverflow::new(o as u32));
    }
    g.into()
}

fn render_kbd(p: Props) -> Element<Msg> {
    Kbd::new(p.str_or("keys", "mod+k")).into()
}

fn render_divider(p: Props) -> Element<Msg> {
    let mut d = Divider::new()
        .orientation(if p.str("orientation") == Some("vertical") {
            DividerOrientation::Vertical
        } else {
            DividerOrientation::Horizontal
        })
        .variant(if p.str("variant") == Some("strong") {
            DividerVariant::Strong
        } else {
            DividerVariant::Subtle
        });
    if let Some(l) = p.str("label") {
        d = d.label(l);
    }
    if p.str("orientation") == Some("vertical") {
        HStack::new()
            .height(SizeValue::Points(48.0))
            .gap(astryx_core::Space::S2)
            .child(Text::new("Start"))
            .child(d)
            .child(Text::new("End"))
            .into()
    } else {
        VStack::new()
            .width(SizeValue::Points(240.0))
            .gap(astryx_core::Space::S2)
            .child(Text::new("Above"))
            .child(d)
            .child(Text::new("Below"))
            .into()
    }
}

fn render_indicator(p: Props) -> Element<Msg> {
    let size = if p.str("size") == Some("sm") {
        IndicatorSize::Sm
    } else {
        IndicatorSize::Md
    };
    let disabled = p.bool("isDisabled");
    let state = p.str_or("state", "checked");
    match p.str_or("kind", "checkbox") {
        "radio" => RadioIndicator::new(state == "checked")
            .size(size)
            .is_disabled(disabled)
            .into(),
        "check" => CheckIndicator::new(state == "checked")
            .is_disabled(disabled)
            .into(),
        _ => CheckboxIndicator::new(match state {
            "unchecked" => CheckboxState::Unchecked,
            "indeterminate" => CheckboxState::Indeterminate,
            _ => CheckboxState::Checked,
        })
        .size(size)
        .is_disabled(disabled)
        .into(),
    }
}

fn render_visually_hidden(p: Props) -> Element<Msg> {
    HStack::new()
        .child(Text::new("Visible"))
        .child(VisuallyHidden::new(p.words("Hidden from sight")))
        .into()
}

pub fn families() -> Vec<Family> {
    vec![
        Family {
            name: "Text",
            base: || json!({}),
            sweeps: || {
                vec![
                    (
                        "type",
                        v(&[
                            "body",
                            "large",
                            "label",
                            "supporting",
                            "code",
                            "display-1",
                            "display-2",
                            "display-3",
                        ]),
                    ),
                    (
                        "size",
                        v(&[
                            "4xs", "3xs", "2xs", "xsm", "sm", "base", "lg", "xl", "2xl", "3xl",
                            "4xl",
                        ]),
                    ),
                    (
                        "color",
                        v(&["primary", "secondary", "disabled", "placeholder", "accent"]),
                    ),
                    ("weight", v(&["normal", "medium", "semibold", "bold"])),
                    ("justify", v(&["start", "center", "end"])),
                    ("hasStrikethrough", vec![json!(true)]),
                    ("maxLines", vec![json!(1)]),
                ]
            },
            render: render_text,
        },
        Family {
            name: "Heading",
            base: || json!({}),
            sweeps: || {
                vec![
                    ("level", (1..=6).map(|l| json!(l)).collect()),
                    ("type", v(&["display-1", "display-2", "display-3"])),
                    ("color", v(&["primary", "secondary", "accent"])),
                ]
            },
            render: render_heading,
        },
        Family {
            name: "Icon",
            base: || json!({"icon": "check"}),
            sweeps: || {
                vec![
                    (
                        "color",
                        IconColor::ALL
                            .iter()
                            .map(|c| json!(format!("{c:?}").to_lowercase()))
                            .collect(),
                    ),
                    ("size", v(&["xsm", "sm", "md", "lg"])),
                    ("icon", BUILT_IN_ICONS.iter().map(|n| json!(n)).collect()),
                ]
            },
            render: render_icon,
        },
        Family {
            name: "Spinner",
            base: || json!({}),
            sweeps: || {
                vec![
                    ("size", v(&["sm", "md", "lg", "xl"])),
                    ("shade", v(&["default", "subtle", "onMedia", "inherit"])),
                    ("label", v(&["Loading results"])),
                ]
            },
            render: render_spinner,
        },
        Family {
            name: "Link",
            base: || json!({}),
            sweeps: || {
                vec![
                    ("hasUnderline", vec![json!(true)]),
                    ("isDisabled", vec![json!(true)]),
                    ("isExternalLink", vec![json!(true)]),
                    ("isStandalone", vec![json!(true)]),
                    ("color", v(&["primary", "secondary", "accent"])),
                    ("weight", v(&["normal", "medium", "semibold", "bold"])),
                    ("size", v(&["sm", "base", "lg"])),
                ]
            },
            render: render_link,
        },
        Family {
            name: "Badge",
            base: || json!({}),
            sweeps: || {
                vec![(
                    "variant",
                    v(&[
                        "neutral", "info", "success", "warning", "error", "blue", "cyan", "green",
                        "orange", "pink", "purple", "red", "teal", "yellow",
                    ]),
                )]
            },
            render: render_badge,
        },
        Family {
            name: "Avatar",
            base: || json!({}),
            sweeps: || {
                vec![
                    ("size", v(&["xsm", "sm", "md", "lg", "xl"])),
                    ("shape", v(&["circle", "rounded", "square"])),
                    ("status", v(&["success", "neutral", "error"])),
                ]
            },
            render: render_avatar,
        },
        Family {
            name: "AvatarGroup",
            base: || json!({"overflow": 5}),
            sweeps: || {
                vec![
                    ("size", v(&["xsm", "sm", "md", "lg", "xl"])),
                    ("shape", v(&["circle", "rounded", "square"])),
                ]
            },
            render: render_avatar_group,
        },
        Family {
            name: "Kbd",
            base: || json!({}),
            sweeps: || {
                vec![(
                    "keys",
                    v(&["mod+k", "shift+enter", "esc", "ctrl+alt+delete"]),
                )]
            },
            render: render_kbd,
        },
        Family {
            name: "Divider",
            base: || json!({}),
            sweeps: || {
                vec![
                    ("orientation", v(&["horizontal", "vertical"])),
                    ("variant", v(&["subtle", "strong"])),
                    ("label", v(&["or"])),
                ]
            },
            render: render_divider,
        },
        Family {
            name: "Indicator",
            base: || json!({}),
            sweeps: || {
                vec![
                    ("kind", v(&["checkbox", "radio", "check"])),
                    ("state", v(&["checked", "unchecked", "indeterminate"])),
                    ("size", v(&["sm", "md"])),
                    ("isDisabled", vec![json!(true)]),
                ]
            },
            render: render_indicator,
        },
        Family {
            name: "VisuallyHidden",
            base: || json!({}),
            sweeps: Vec::new,
            render: render_visually_hidden,
        },
    ]
}
