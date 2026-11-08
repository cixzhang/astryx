//! A catalog of component demos built from data, for apps that show live
//! Astryx components from declarative JSON (such as feature pages).
//!
//! Every component lists its variants and an allowlist of typed, bounded
//! props. [`validate`] checks a props object against it; [`build`] makes
//! the element, with interactive state the caller keeps in a [`DemoState`].
//! Props are plain data: text is at most a few dozen characters, with no
//! links and no control characters.
//!
//! ```
//! use astryx_gallery::catalog::{self, DemoState};
//! let props = serde_json::json!({"label": "Save", "is_loading": true});
//! let element = catalog::build::<()>(
//!     "Button",
//!     Some("primary"),
//!     props.as_object().unwrap(),
//!     &DemoState::default(),
//!     |_| (),
//! );
//! assert!(element.is_ok());
//! ```

use astryx_core::lab;
use astryx_core::*;
use serde_json::{Map, Value};
use std::fmt;

/// What a prop holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropKind {
    /// Plain text of at most `max` characters.
    Text {
        max: usize,
    },
    Bool,
    /// A whole number from `min` to `max`.
    Int {
        min: i64,
        max: i64,
    },
    /// One of these names.
    Enum(&'static [&'static str]),
    /// Up to `max_items` texts of at most `item_max` characters each.
    TextList {
        max_items: usize,
        item_max: usize,
    },
}

/// A prop's value when the props leave it out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropDefault {
    Text(&'static str),
    Bool(bool),
    Int(i64),
    Enum(&'static str),
    TextList(&'static [&'static str]),
}

/// One allowed prop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PropSpec {
    pub name: &'static str,
    pub kind: PropKind,
    pub required: bool,
    pub default: PropDefault,
}

/// One component: its variants (the first is the default) and its props.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComponentSpec {
    pub name: &'static str,
    pub variants: &'static [&'static str],
    pub props: &'static [PropSpec],
}

/// Why props were refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogError {
    UnknownComponent(String),
    UnknownVariant {
        component: &'static str,
        variant: String,
    },
    UnknownProp {
        component: &'static str,
        prop: String,
    },
    MissingProp {
        component: &'static str,
        prop: &'static str,
    },
    /// The value does not fit the prop: `expected` says what does.
    BadValue {
        component: &'static str,
        prop: &'static str,
        expected: String,
    },
    /// The props JSON is not an object.
    NotAnObject(String),
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CatalogError::UnknownComponent(c) => write!(f, "no component named {c:?}"),
            CatalogError::UnknownVariant { component, variant } => {
                write!(f, "{component} has no variant {variant:?}")
            }
            CatalogError::UnknownProp { component, prop } => {
                write!(f, "{component} takes no prop {prop:?}")
            }
            CatalogError::MissingProp { component, prop } => {
                write!(f, "{component} needs {prop:?}")
            }
            CatalogError::BadValue {
                component,
                prop,
                expected,
            } => write!(f, "{component}.{prop} must be {expected}"),
            CatalogError::NotAnObject(why) => write!(f, "props must be a JSON object: {why}"),
        }
    }
}

impl std::error::Error for CatalogError {}

const LABEL: usize = 60;

const fn text(name: &'static str, default: &'static str) -> PropSpec {
    PropSpec {
        name,
        kind: PropKind::Text { max: LABEL },
        required: false,
        default: PropDefault::Text(default),
    }
}

const fn flag(name: &'static str, default: bool) -> PropSpec {
    PropSpec {
        name,
        kind: PropKind::Bool,
        required: false,
        default: PropDefault::Bool(default),
    }
}

const fn choice(
    name: &'static str,
    values: &'static [&'static str],
    default: &'static str,
) -> PropSpec {
    PropSpec {
        name,
        kind: PropKind::Enum(values),
        required: false,
        default: PropDefault::Enum(default),
    }
}

const SIZES: &[&str] = &["sm", "md", "lg"];
const RADIOS: &[&str] = &["off", "on", "connected"];
/// The gallery's sample images, by name. Apps register them with
/// `astryx_gallery::images::samples()`.
const SAMPLE_IMAGES: &[&str] = &["beach", "hills", "city", "sticker"];
const BADGES: &[&str] = &[
    "neutral", "info", "success", "warning", "error", "blue", "cyan", "green", "orange", "pink",
    "purple", "red",
];
const CARDS: &[&str] = &[
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
];
const TEXT_TYPES: &[&str] = &[
    "body",
    "large",
    "label",
    "supporting",
    "code",
    "display-2",
    "display-3",
];
const WEIGHTS: &[&str] = &["normal", "medium", "semibold", "bold"];
const TEXT_COLORS: &[&str] = &["primary", "secondary", "disabled", "placeholder", "accent"];
const ICON_COLORS: &[&str] = &[
    "primary",
    "secondary",
    "tertiary",
    "disabled",
    "accent",
    "info",
    "success",
    "error",
    "warning",
];

/// Every component the catalog builds.
pub const COMPONENTS: &[ComponentSpec] = &[
    ComponentSpec {
        name: "Button",
        variants: &["primary", "secondary", "ghost", "destructive"],
        props: &[
            text("label", "Button"),
            choice("size", SIZES, "md"),
            flag("is_loading", false),
            flag("is_disabled", false),
            text("tooltip", ""),
        ],
    },
    ComponentSpec {
        name: "Switch",
        variants: &["default"],
        props: &[
            text("label", "Switch"),
            flag("value", false),
            choice("size", &["sm", "md"], "md"),
            flag("is_disabled", false),
            text("description", ""),
        ],
    },
    ComponentSpec {
        name: "Selector",
        variants: &["input", "secondary", "ghost"],
        props: &[
            text("label", "Fruit"),
            PropSpec {
                name: "options",
                kind: PropKind::TextList {
                    max_items: 6,
                    item_max: LABEL,
                },
                required: false,
                default: PropDefault::TextList(&["Apple", "Banana", "Cherry"]),
            },
            text("value", ""),
            text("placeholder", "Choose one"),
            flag("is_disabled", false),
            flag("has_clear", false),
        ],
    },
    ComponentSpec {
        name: "Badge",
        variants: BADGES,
        props: &[text("label", "Badge")],
    },
    ComponentSpec {
        name: "ProgressBar",
        variants: &["accent", "success", "warning", "error", "neutral"],
        props: &[
            text("label", "Progress"),
            PropSpec {
                name: "value",
                kind: PropKind::Int { min: 0, max: 100 },
                required: false,
                default: PropDefault::Int(40),
            },
            flag("is_indeterminate", false),
            flag("has_value_label", false),
        ],
    },
    ComponentSpec {
        name: "ListItem",
        variants: &["default"],
        props: &[
            text("label", "List item"),
            text("description", ""),
            flag("is_selected", false),
            flag("is_contained", false),
        ],
    },
    ComponentSpec {
        name: "Avatar",
        variants: &["default"],
        props: &[
            text("name", "Ada Lovelace"),
            choice("size", &["xsm", "sm", "md", "lg", "xl"], "md"),
            choice("shape", &["circle", "rounded", "square"], "circle"),
            choice("status", &["none", "success", "neutral", "error"], "none"),
        ],
    },
    ComponentSpec {
        name: "Text",
        variants: &["default"],
        props: &[
            text("text", "The quick brown fox"),
            choice("type", TEXT_TYPES, "body"),
            choice("weight", WEIGHTS, "normal"),
            choice("color", TEXT_COLORS, "primary"),
        ],
    },
    ComponentSpec {
        name: "Heading",
        variants: &["2", "1", "3", "4", "5", "6"],
        props: &[text("text", "Heading")],
    },
    ComponentSpec {
        name: "Divider",
        variants: &["subtle", "strong"],
        props: &[text("label", "")],
    },
    ComponentSpec {
        name: "Icon",
        variants: &["md", "xsm", "sm", "lg"],
        props: &[
            choice("icon", &BUILT_IN_ICONS, "check"),
            choice("color", ICON_COLORS, "primary"),
        ],
    },
    ComponentSpec {
        name: "OnScreenKeyboard",
        variants: &["chord", "grid"],
        props: &[text("label", "Message"), text("value", "Hello")],
    },
    ComponentSpec {
        name: "Kbd",
        variants: &["gamepad", "keyboard"],
        props: &[text("keys", "B")],
    },
    ComponentSpec {
        name: "SelectableCard",
        variants: CARDS,
        props: &[
            text("label", "Pro plan"),
            text("title", "Pro plan"),
            text("body", "Every garden, unlimited plots"),
            flag("is_selected", false),
            flag("is_disabled", false),
        ],
    },
    ComponentSpec {
        name: "HoldCard",
        variants: &["default"],
        props: &[
            text("label", "Send the plan"),
            text("title", "Send to Ada"),
            text("body", "Hold A to send"),
            flag("is_selected", false),
        ],
    },
    ComponentSpec {
        name: "ChoiceTile",
        variants: &["default"],
        props: &[
            text("title", "Pro"),
            text("body", "Every garden, unlimited plots"),
            flag("is_selected", false),
            flag("is_recommended", false),
            choice(
                "image",
                &["none", "beach", "hills", "city", "sticker"],
                "none",
            ),
        ],
    },
    ComponentSpec {
        name: "HoldButton",
        variants: &["primary", "secondary", "ghost", "destructive"],
        props: &[
            text("label", "Hold to confirm"),
            PropSpec {
                name: "duration_ms",
                kind: PropKind::Int {
                    min: 300,
                    max: 3000,
                },
                required: false,
                default: PropDefault::Int(1000),
            },
            flag("is_disabled", false),
        ],
    },
    ComponentSpec {
        name: "StatusBar",
        variants: &["default"],
        props: &[
            choice("wifi", RADIOS, "connected"),
            choice("bluetooth", RADIOS, "off"),
            flag("has_battery", true),
            PropSpec {
                name: "battery",
                kind: PropKind::Int { min: 0, max: 100 },
                required: false,
                default: PropDefault::Int(82),
            },
            flag("charging", false),
            PropSpec {
                name: "hour",
                kind: PropKind::Int { min: 0, max: 23 },
                required: false,
                default: PropDefault::Int(19),
            },
            PropSpec {
                name: "minute",
                kind: PropKind::Int { min: 0, max: 59 },
                required: false,
                default: PropDefault::Int(42),
            },
        ],
    },
    ComponentSpec {
        name: "Image",
        variants: &["default"],
        props: &[
            choice("image", SAMPLE_IMAGES, "beach"),
            text("alt", "A beach at sunset"),
            choice("fit", &["cover", "contain"], "cover"),
            choice("aspect", &["16:9", "4:3", "3:2", "1:1"], "16:9"),
            flag("pixel_perfect", false),
        ],
    },
    ComponentSpec {
        name: "Card",
        variants: CARDS,
        props: &[
            text("text", "Card content"),
            choice("padding", &["s2", "s4", "s6"], "s4"),
        ],
    },
];

/// The catalog: every component, its variants, and its allowed props.
pub fn list() -> &'static [ComponentSpec] {
    COMPONENTS
}

/// The catalog as JSON, so tools in other languages can validate against
/// it: `{"components": [{"name", "variants", "props": [{"name", "kind",
/// "required", "default", ...bounds}]}]}`.
pub fn spec_json() -> String {
    let components: Vec<Value> = COMPONENTS
        .iter()
        .map(|c| {
            let props: Vec<Value> = c
                .props
                .iter()
                .map(|p| {
                    let mut v = match p.kind {
                        PropKind::Text { max } => serde_json::json!({"kind": "text", "max": max}),
                        PropKind::Bool => serde_json::json!({"kind": "bool"}),
                        PropKind::Int { min, max } => serde_json::json!({"kind": "int", "min": min, "max": max}),
                        PropKind::Enum(values) => serde_json::json!({"kind": "enum", "values": values}),
                        PropKind::TextList { max_items, item_max } => {
                            serde_json::json!({"kind": "text_list", "max_items": max_items, "item_max": item_max})
                        }
                    };
                    let default = match p.default {
                        PropDefault::Text(t) | PropDefault::Enum(t) => Value::from(t),
                        PropDefault::Bool(b) => Value::from(b),
                        PropDefault::Int(i) => Value::from(i),
                        PropDefault::TextList(items) => Value::from(items.to_vec()),
                    };
                    let object = v.as_object_mut().expect("an object");
                    object.insert("name".into(), p.name.into());
                    object.insert("required".into(), p.required.into());
                    object.insert("default".into(), default);
                    v
                })
                .collect();
            serde_json::json!({"name": c.name, "variants": c.variants, "props": props})
        })
        .collect();
    serde_json::to_string_pretty(&serde_json::json!({ "components": components }))
        .expect("serializable")
}

/// A checked prop value.
#[derive(Clone, Debug, PartialEq)]
enum Checked {
    Text(String),
    Bool(bool),
    Int(i64),
    Enum(&'static str),
    TextList(Vec<String>),
}

/// The checked props of one component, defaults filled in.
struct Props {
    values: Vec<(&'static str, Checked)>,
}

impl Props {
    fn get(&self, name: &str) -> &Checked {
        &self
            .values
            .iter()
            .find(|(n, _)| *n == name)
            .expect("a listed prop")
            .1
    }
    fn text(&self, name: &str) -> String {
        match self.get(name) {
            Checked::Text(t) => t.clone(),
            _ => String::new(),
        }
    }
    fn flag(&self, name: &str) -> bool {
        matches!(self.get(name), Checked::Bool(true))
    }
    fn int(&self, name: &str) -> i64 {
        match self.get(name) {
            Checked::Int(i) => *i,
            _ => 0,
        }
    }
    fn choice(&self, name: &str) -> &'static str {
        match self.get(name) {
            Checked::Enum(e) => e,
            _ => "",
        }
    }
    fn list(&self, name: &str) -> Vec<String> {
        match self.get(name) {
            Checked::TextList(items) => items.clone(),
            _ => Vec::new(),
        }
    }
}

/// Plain text: within `max` characters, with no links and no control
/// characters.
fn plain(s: &str, max: usize) -> bool {
    s.chars().count() <= max
        && !s.chars().any(char::is_control)
        && !s.contains("://")
        && !s.to_ascii_lowercase().contains("www.")
}

fn check(
    spec: &'static ComponentSpec,
    variant: Option<&str>,
    props: &Map<String, Value>,
) -> Result<(&'static str, Props), CatalogError> {
    let variant = match variant {
        None => spec.variants[0],
        Some(v) => spec
            .variants
            .iter()
            .copied()
            .find(|&known| known == v)
            .ok_or_else(|| CatalogError::UnknownVariant {
                component: spec.name,
                variant: v.to_string(),
            })?,
    };
    if let Some(unknown) = props
        .keys()
        .find(|k| !spec.props.iter().any(|p| p.name == k.as_str()))
    {
        return Err(CatalogError::UnknownProp {
            component: spec.name,
            prop: unknown.clone(),
        });
    }
    let mut values = Vec::new();
    for p in spec.props {
        let bad = |expected: String| CatalogError::BadValue {
            component: spec.name,
            prop: p.name,
            expected,
        };
        let value = match props.get(p.name) {
            None if p.required => {
                return Err(CatalogError::MissingProp {
                    component: spec.name,
                    prop: p.name,
                })
            }
            None => match p.default {
                PropDefault::Text(t) => Checked::Text(t.into()),
                PropDefault::Bool(b) => Checked::Bool(b),
                PropDefault::Int(i) => Checked::Int(i),
                PropDefault::Enum(e) => Checked::Enum(e),
                PropDefault::TextList(items) => {
                    Checked::TextList(items.iter().map(|s| s.to_string()).collect())
                }
            },
            Some(v) => match p.kind {
                PropKind::Text { max } => match v.as_str() {
                    Some(s) if plain(s, max) => Checked::Text(s.into()),
                    _ => {
                        return Err(bad(format!(
                            "plain text of at most {max} characters, with no links"
                        )))
                    }
                },
                PropKind::Bool => {
                    Checked::Bool(v.as_bool().ok_or_else(|| bad("true or false".into()))?)
                }
                PropKind::Int { min, max } => match v.as_i64() {
                    Some(i) if (min..=max).contains(&i) => Checked::Int(i),
                    _ => return Err(bad(format!("a whole number from {min} to {max}"))),
                },
                PropKind::Enum(values) => {
                    match v.as_str().and_then(|s| values.iter().find(|&&e| e == s)) {
                        Some(e) => Checked::Enum(e),
                        None => return Err(bad(format!("one of {}", values.join(", ")))),
                    }
                }
                PropKind::TextList {
                    max_items,
                    item_max,
                } => {
                    let items = v.as_array().filter(|a| a.len() <= max_items);
                    let items: Option<Vec<String>> = items.and_then(|a| {
                        a.iter()
                            .map(|i| i.as_str().filter(|s| plain(s, item_max)).map(String::from))
                            .collect()
                    });
                    match items {
                        Some(items) => Checked::TextList(items),
                        None => {
                            return Err(bad(format!(
                                "a list of at most {max_items} plain texts of at most {item_max} characters"
                            )))
                        }
                    }
                }
            },
        };
        values.push((p.name, value));
    }
    Ok((variant, Props { values }))
}

fn spec(name: &str) -> Result<&'static ComponentSpec, CatalogError> {
    COMPONENTS
        .iter()
        .find(|c| c.name == name)
        .ok_or_else(|| CatalogError::UnknownComponent(name.to_string()))
}

/// Checks `props` for component `name` and `variant` (the first variant
/// when `None`): every prop listed, every value in bounds.
pub fn validate(
    name: &str,
    variant: Option<&str>,
    props: &Map<String, Value>,
) -> Result<(), CatalogError> {
    check(spec(name)?, variant, props).map(|_| ())
}

/// Parses `props_json` and calls [`validate`].
pub fn validate_json(
    name: &str,
    variant: Option<&str>,
    props_json: &str,
) -> Result<(), CatalogError> {
    validate(name, variant, &parse(props_json)?)
}

fn parse(props_json: &str) -> Result<Map<String, Value>, CatalogError> {
    match serde_json::from_str::<Value>(props_json) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => Err(CatalogError::NotAnObject("not an object".into())),
        Err(e) => Err(CatalogError::NotAnObject(e.to_string())),
    }
}

/// What a demo remembers between rebuilds: the caller keeps it and updates
/// it from [`DemoEvent`]s.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DemoState {
    /// The Switch's value once toggled; until then, its `value` prop.
    pub toggled: Option<bool>,
    /// The Selector's choice once made; until then, its `value` prop.
    pub selected: Option<String>,
    /// How many times the Button was pressed.
    pub presses: u32,
}

impl DemoState {
    /// The state after `event`.
    pub fn apply(&mut self, event: &DemoEvent) {
        match event {
            DemoEvent::Toggled(on) => self.toggled = Some(*on),
            DemoEvent::Selected(value) => self.selected = Some(value.clone()),
            DemoEvent::Pressed => self.presses += 1,
        }
    }
}

/// What a person did in a demo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DemoEvent {
    Toggled(bool),
    Selected(String),
    Pressed,
}

/// Builds component `name` in `variant` (the first when `None`) from
/// checked `props`, showing `state`; `on` turns what a person does into the
/// app's message.
pub fn build<Msg: Clone + 'static>(
    name: &str,
    variant: Option<&str>,
    props: &Map<String, Value>,
    state: &DemoState,
    on: impl Fn(DemoEvent) -> Msg + Clone + 'static,
) -> Result<Element<Msg>, CatalogError> {
    let (variant, p) = check(spec(name)?, variant, props)?;
    let size = |s: &str| match s {
        "sm" => ButtonSize::Sm,
        "lg" => ButtonSize::Lg,
        _ => ButtonSize::Md,
    };
    Ok(match name {
        "Button" => {
            let mut b = Button::new(p.text("label"))
                .variant(match variant {
                    "secondary" => ButtonVariant::Secondary,
                    "ghost" => ButtonVariant::Ghost,
                    "destructive" => ButtonVariant::Destructive,
                    _ => ButtonVariant::Primary,
                })
                .size(size(p.choice("size")))
                .is_loading(p.flag("is_loading"))
                .is_disabled(p.flag("is_disabled"))
                .on_click(on(DemoEvent::Pressed));
            let tooltip = p.text("tooltip");
            if !tooltip.is_empty() {
                b = b.tooltip(tooltip);
            }
            b.into()
        }
        "Switch" => {
            let value = state.toggled.unwrap_or(p.flag("value"));
            let mut s = Switch::new(p.text("label"), value)
                .size(if p.choice("size") == "sm" {
                    SwitchSize::Sm
                } else {
                    SwitchSize::Md
                })
                .is_disabled(p.flag("is_disabled"))
                .on_change(move |v| on(DemoEvent::Toggled(v)));
            let description = p.text("description");
            if !description.is_empty() {
                s = s.description(description);
            }
            s.into()
        }
        "Selector" => {
            let options: Vec<SelectorOption> = p
                .list("options")
                .into_iter()
                .map(|o| SelectorOption::new(o.clone()).label(o))
                .collect();
            let chosen = state.selected.clone().unwrap_or_else(|| p.text("value"));
            let mut s = Selector::new(p.text("label"), options)
                .variant(match variant {
                    "secondary" => SelectorVariant::Secondary,
                    "ghost" => SelectorVariant::Ghost,
                    _ => SelectorVariant::Input,
                })
                .placeholder(p.text("placeholder"))
                .is_disabled(p.flag("is_disabled"))
                .has_clear(p.flag("has_clear"))
                .on_change(move |v: String| on(DemoEvent::Selected(v)));
            if !chosen.is_empty() {
                s = s.value(chosen);
            }
            s.into()
        }
        "Badge" => Badge::new(p.text("label"))
            .variant(match variant {
                "info" => BadgeVariant::Info,
                "success" => BadgeVariant::Success,
                "warning" => BadgeVariant::Warning,
                "error" => BadgeVariant::Error,
                "blue" => BadgeVariant::Blue,
                "cyan" => BadgeVariant::Cyan,
                "green" => BadgeVariant::Green,
                "orange" => BadgeVariant::Orange,
                "pink" => BadgeVariant::Pink,
                "purple" => BadgeVariant::Purple,
                "red" => BadgeVariant::Red,
                _ => BadgeVariant::Neutral,
            })
            .into(),
        "ProgressBar" => ProgressBar::new(p.text("label"))
            .value(p.int("value") as f64)
            .variant(match variant {
                "success" => ProgressBarVariant::Success,
                "warning" => ProgressBarVariant::Warning,
                "error" => ProgressBarVariant::Error,
                "neutral" => ProgressBarVariant::Neutral,
                _ => ProgressBarVariant::Accent,
            })
            .is_indeterminate(p.flag("is_indeterminate"))
            .has_value_label(p.flag("has_value_label"))
            .into(),
        "ListItem" => {
            let mut item = ListItem::new(p.text("label"))
                .is_selected(p.flag("is_selected"))
                .is_contained(p.flag("is_contained"));
            let description = p.text("description");
            if !description.is_empty() {
                item = item.description(description);
            }
            item.into()
        }
        "Avatar" => {
            let mut a = Avatar::new()
                .name(p.text("name"))
                .size(match p.choice("size") {
                    "xsm" => AvatarSize::Xsm,
                    "sm" => AvatarSize::Sm,
                    "lg" => AvatarSize::Lg,
                    "xl" => AvatarSize::Xl,
                    _ => AvatarSize::Md,
                })
                .shape(match p.choice("shape") {
                    "rounded" => AvatarShape::Rounded,
                    "square" => AvatarShape::Square,
                    _ => AvatarShape::Circle,
                });
            let dot = match p.choice("status") {
                "success" => Some(AvatarStatusDotVariant::Success),
                "neutral" => Some(AvatarStatusDotVariant::Neutral),
                "error" => Some(AvatarStatusDotVariant::Error),
                _ => None,
            };
            if let Some(dot) = dot {
                a = a.status(AvatarStatusDot::new(dot));
            }
            a.into()
        }
        "Text" => Text::new(p.text("text"))
            .r#type(match p.choice("type") {
                "large" => TextType::Large,
                "label" => TextType::Label,
                "supporting" => TextType::Supporting,
                "code" => TextType::Code,
                "display-2" => TextType::Display2,
                "display-3" => TextType::Display3,
                _ => TextType::Body,
            })
            .weight(match p.choice("weight") {
                "medium" => TextWeight::Medium,
                "semibold" => TextWeight::Semibold,
                "bold" => TextWeight::Bold,
                _ => TextWeight::Normal,
            })
            .color(match p.choice("color") {
                "secondary" => TextColor::Secondary,
                "disabled" => TextColor::Disabled,
                "placeholder" => TextColor::Placeholder,
                "accent" => TextColor::Accent,
                _ => TextColor::Primary,
            })
            .into(),
        "Heading" => Heading::new(variant.parse().unwrap_or(2), p.text("text")).into(),
        "Divider" => {
            let mut d = Divider::new().variant(if variant == "strong" {
                DividerVariant::Strong
            } else {
                DividerVariant::Subtle
            });
            let label = p.text("label");
            if !label.is_empty() {
                d = d.label(label);
            }
            d.into()
        }
        "Icon" => Icon::new(p.choice("icon"))
            .size(match variant {
                "xsm" => IconSize::Xsm,
                "sm" => IconSize::Sm,
                "lg" => IconSize::Lg,
                _ => IconSize::Md,
            })
            .color(match p.choice("color") {
                "secondary" => IconColor::Secondary,
                "tertiary" => IconColor::Tertiary,
                "disabled" => IconColor::Disabled,
                "accent" => IconColor::Accent,
                "info" => IconColor::Info,
                "success" => IconColor::Success,
                "error" => IconColor::Error,
                "warning" => IconColor::Warning,
                _ => IconColor::Primary,
            })
            .into(),
        "Card" => Card::new()
            .variant(card_variant(variant))
            .padding(match p.choice("padding") {
                "s2" => Space::S2,
                "s6" => Space::S6,
                _ => Space::S4,
            })
            .child(Text::new(p.text("text")))
            .into(),
        // A picture of the keyboard: typing needs the app to keep its state.
        "OnScreenKeyboard" => {
            let kind = if variant == "grid" {
                lab::KeyboardKind::Grid
            } else {
                lab::KeyboardKind::Chord
            };
            lab::OnScreenKeyboard::new(
                p.text("label"),
                p.text("value"),
                lab::KeyboardState::new(kind),
            )
            .into()
        }
        "Kbd" => match variant {
            "keyboard" => Kbd::new(&p.text("keys")).into(),
            _ => Kbd::gamepad(&p.text("keys")).into(),
        },
        "HoldCard" => {
            let selected = state.toggled.unwrap_or(p.flag("is_selected"));
            lab::HoldCard::new(
                SelectableCard::new(p.text("label"), selected)
                    .on_change(move |v| on(DemoEvent::Toggled(v)))
                    .child(
                        VStack::new()
                            .gap(Space::S1)
                            .child(Heading::new(4, p.text("title")))
                            .child(Text::new(p.text("body")).r#type(TextType::Supporting)),
                    ),
            )
            .into()
        }
        "SelectableCard" => {
            let selected = state.toggled.unwrap_or(p.flag("is_selected"));
            SelectableCard::new(p.text("label"), selected)
                .variant(card_variant(variant))
                .is_disabled(p.flag("is_disabled"))
                .on_change(move |v| on(DemoEvent::Toggled(v)))
                .child(
                    VStack::new()
                        .gap(Space::S1)
                        .child(Heading::new(4, p.text("title")))
                        .child(Text::new(p.text("body")).r#type(TextType::Supporting)),
                )
                .into()
        }
        "ChoiceTile" => {
            let selected = state.toggled.unwrap_or(p.flag("is_selected"));
            let mut title = HStack::new()
                .gap(Space::S2)
                .v_align(StackAlign::Center)
                .child(Heading::new(4, p.text("title")));
            if p.flag("is_recommended") {
                title = title.child(Badge::new("Recommended").variant(BadgeVariant::Info));
            }
            let mut body = VStack::new().gap(Space::S1);
            if let Some(src) = sample_image(p.choice("image")) {
                body = body.child(lab::Image::new(ImageId::of(src), "").aspect_ratio(16.0 / 9.0));
            }
            body = body
                .child(title)
                .child(Text::new(p.text("body")).r#type(TextType::Supporting));
            SelectableCard::new(p.text("title"), selected)
                .on_change(move |v| on(DemoEvent::Toggled(v)))
                .child(body)
                .into()
        }
        "HoldButton" => lab::HoldButton::new(p.text("label"))
            .variant(match variant {
                "secondary" => ButtonVariant::Secondary,
                "ghost" => ButtonVariant::Ghost,
                "destructive" => ButtonVariant::Destructive,
                _ => ButtonVariant::Primary,
            })
            .duration_ms(p.int("duration_ms") as u32)
            .is_disabled(p.flag("is_disabled"))
            .on_confirm(on(DemoEvent::Pressed))
            .into(),
        "StatusBar" => {
            let radio = |name: &str| match p.choice(name) {
                "on" => Radio::On,
                "connected" => Radio::Connected,
                _ => Radio::Off,
            };
            lab::StatusBar::new()
                .device(DeviceStatus {
                    wifi: radio("wifi"),
                    wifi_name: None,
                    bluetooth: radio("bluetooth"),
                    battery_percent: p.flag("has_battery").then(|| p.int("battery") as u8),
                    charging: p.flag("charging"),
                    time: (p.int("hour") as u8, p.int("minute") as u8),
                })
                .into()
        }
        "Image" => {
            let src = sample_image(p.choice("image")).unwrap_or(crate::images::BEACH);
            let ratio = match p.choice("aspect") {
                "4:3" => 4.0 / 3.0,
                "3:2" => 1.5,
                "1:1" => 1.0,
                _ => 16.0 / 9.0,
            };
            lab::Image::new(ImageId::of(src), p.text("alt"))
                .fit(if p.choice("fit") == "contain" {
                    ImageFit::Contain
                } else {
                    ImageFit::Cover
                })
                .aspect_ratio(ratio)
                .pixel_perfect(p.flag("pixel_perfect"))
                .into()
        }
        _ => return Err(CatalogError::UnknownComponent(name.to_string())),
    })
}

fn sample_image(name: &str) -> Option<&'static str> {
    use crate::images::{BEACH, CITY, HILLS, STICKER};
    match name {
        "beach" => Some(BEACH),
        "hills" => Some(HILLS),
        "city" => Some(CITY),
        "sticker" => Some(STICKER),
        _ => None,
    }
}

fn card_variant(name: &str) -> CardVariant {
    match name {
        "transparent" => CardVariant::Transparent,
        "muted" => CardVariant::Muted,
        "blue" => CardVariant::Blue,
        "cyan" => CardVariant::Cyan,
        "gray" => CardVariant::Gray,
        "green" => CardVariant::Green,
        "orange" => CardVariant::Orange,
        "pink" => CardVariant::Pink,
        "purple" => CardVariant::Purple,
        "red" => CardVariant::Red,
        "teal" => CardVariant::Teal,
        _ => CardVariant::Default,
    }
}

/// Parses `props_json` and calls [`build`].
pub fn build_json<Msg: Clone + 'static>(
    name: &str,
    variant: Option<&str>,
    props_json: &str,
    state: &DemoState,
    on: impl Fn(DemoEvent) -> Msg + Clone + 'static,
) -> Result<Element<Msg>, CatalogError> {
    build(name, variant, &parse(props_json)?, state, on)
}
