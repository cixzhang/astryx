//! The `settings-dialog` template: a settings window. A side navigation of
//! sections with a search field over every setting, and a panel of
//! settings cards; below 640 points it becomes one column with section
//! tabs. Shown inline, as the template is (a preview, not a modal).

use astryx_core::{App, Command, Element, Mode, Space, Theme, ThemeChoice};
use astryx_spike_foundations::ComponentSize;
use astryx_spike_ui::*;
use std::collections::BTreeMap;

/// The narrow layout's breakpoint.
pub const NARROW: f32 = 640.0;

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Panel(&'static str),
    Query(String),
    Toggle(&'static str, bool),
    Choose(&'static str, String),
    Text(&'static str, String),
    Noop,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Control {
    Switch,
    Select(&'static [(&'static str, &'static str)]),
    Text,
    Keys(&'static str),
    Button(&'static str),
}

struct Row {
    key: &'static str,
    title: &'static str,
    description: &'static str,
    icon: &'static str,
    control: Control,
}

struct Panel {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    icon: &'static str,
    cards: &'static [(Option<&'static str>, &'static [Row])],
}

const THEMES: &[(&str, &str)] = &[("system", "System"), ("light", "Light"), ("dark", "Dark")];
const DENSITY: &[(&str, &str)] = &[
    ("compact", "Compact"),
    ("balanced", "Balanced"),
    ("spacious", "Spacious"),
];
const FONTS: &[(&str, &str)] = &[("system", "System"), ("inter", "Inter"), ("serif", "Serif")];
const LANGS: &[(&str, &str)] = &[("en", "English"), ("es", "Español"), ("ja", "日本語")];
const PRESENCE: &[(&str, &str)] = &[
    ("available", "Available"),
    ("busy", "Busy"),
    ("away", "Away"),
];

macro_rules! row {
    ($key:literal, $title:literal, $desc:literal, $icon:literal, $c:expr) => {
        Row {
            key: $key,
            title: $title,
            description: $desc,
            icon: $icon,
            control: $c,
        }
    };
}

const GROUPS: &[(&str, &[Panel])] = &[
    (
        "Account",
        &[
            Panel {
                id: "profile",
                label: "Profile",
                description: "How you appear to people in this workspace.",
                icon: "user",
                cards: &[
                    (
                        None,
                        &[
                            row!(
                                "displayName",
                                "Display name",
                                "Shown on your comments and mentions.",
                                "user",
                                Control::Text
                            ),
                            row!(
                                "jobTitle",
                                "Job title",
                                "Shown on your profile card.",
                                "briefcase",
                                Control::Text
                            ),
                        ],
                    ),
                    (
                        Some("Presence"),
                        &[
                            row!(
                                "presence",
                                "Status",
                                "Who sees you as available.",
                                "status",
                                Control::Select(PRESENCE)
                            ),
                            row!(
                                "showLocalTime",
                                "Show local time",
                                "Lets people see your time of day.",
                                "clock",
                                Control::Switch
                            ),
                        ],
                    ),
                ],
            },
            Panel {
                id: "security",
                label: "Login & security",
                description: "Sign-in methods and the devices signed in right now.",
                icon: "lock",
                cards: &[(
                    None,
                    &[
                        row!(
                            "twoFactor",
                            "Two-step verification",
                            "Ask for a code when you sign in.",
                            "shield",
                            Control::Switch
                        ),
                        row!(
                            "password",
                            "Password",
                            "Last changed 3 months ago.",
                            "key",
                            Control::Button("Change")
                        ),
                    ],
                )],
            },
            Panel {
                id: "billing",
                label: "Billing",
                description: "Your plan, seats, and where invoices are sent.",
                icon: "creditCard",
                cards: &[(
                    None,
                    &[
                        row!(
                            "billingEmail",
                            "Billing email",
                            "Invoices go here.",
                            "mail",
                            Control::Text
                        ),
                        row!(
                            "emailInvoices",
                            "Email invoices",
                            "Send each invoice as a PDF.",
                            "document",
                            Control::Switch
                        ),
                    ],
                )],
            },
        ],
    ),
    (
        "Workspace",
        &[
            Panel {
                id: "appearance",
                label: "Appearance",
                description: "Theme and density for this browser.",
                icon: "paintBrush",
                cards: &[(
                    None,
                    &[
                        row!(
                            "theme",
                            "Theme",
                            "Match the system, or pick one.",
                            "sun",
                            Control::Select(THEMES)
                        ),
                        row!(
                            "font",
                            "Interface font",
                            "The face for menus and text.",
                            "text",
                            Control::Select(FONTS)
                        ),
                        row!(
                            "density",
                            "Density",
                            "How much fits on screen.",
                            "grid",
                            Control::Select(DENSITY)
                        ),
                        row!(
                            "reduceMotion",
                            "Reduce motion",
                            "Fewer animations.",
                            "pause",
                            Control::Switch
                        ),
                    ],
                )],
            },
            Panel {
                id: "notifications",
                label: "Notifications",
                description: "What reaches you, and where it arrives.",
                icon: "bell",
                cards: &[
                    (
                        Some("Email"),
                        &[
                            row!(
                                "emailMentions",
                                "Mentions",
                                "When someone mentions you.",
                                "atSymbol",
                                Control::Switch
                            ),
                            row!(
                                "emailComments",
                                "Comments",
                                "Replies to your threads.",
                                "chat",
                                Control::Switch
                            ),
                            row!(
                                "weeklyDigest",
                                "Weekly digest",
                                "A summary every Monday.",
                                "calendar",
                                Control::Switch
                            ),
                        ],
                    ),
                    (
                        Some("Desktop"),
                        &[
                            row!(
                                "desktopAlerts",
                                "Alerts",
                                "Show system notifications.",
                                "bell",
                                Control::Switch
                            ),
                            row!(
                                "notificationSounds",
                                "Sounds",
                                "Play a sound with alerts.",
                                "speaker",
                                Control::Switch
                            ),
                        ],
                    ),
                ],
            },
            Panel {
                id: "shortcuts",
                label: "Keyboard shortcuts",
                description: "Rebind or switch off any shortcut on this surface.",
                icon: "command",
                cards: &[(
                    None,
                    &[
                        row!(
                            "k-palette",
                            "Command palette",
                            "Search commands and files.",
                            "search",
                            Control::Keys("mod+k")
                        ),
                        row!(
                            "k-new",
                            "New item",
                            "Create in the current view.",
                            "plus",
                            Control::Keys("mod+n")
                        ),
                        row!(
                            "k-find",
                            "Find",
                            "Search this page.",
                            "search",
                            Control::Keys("mod+f")
                        ),
                    ],
                )],
            },
            Panel {
                id: "language",
                label: "Language & region",
                description: "Language, formats, and time zone.",
                icon: "globe",
                cards: &[(
                    None,
                    &[row!(
                        "language",
                        "Language",
                        "For menus and messages.",
                        "globe",
                        Control::Select(LANGS)
                    )],
                )],
            },
        ],
    ),
    (
        "Advanced",
        &[
            Panel {
                id: "privacy",
                label: "Privacy",
                description: "What you share, and what we keep.",
                icon: "shield",
                cards: &[(
                    None,
                    &[
                        row!(
                            "discoverable",
                            "Discoverable",
                            "People can find you by email.",
                            "search",
                            Control::Switch
                        ),
                        row!(
                            "readReceipts",
                            "Read receipts",
                            "Show when you have read a message.",
                            "eye",
                            Control::Switch
                        ),
                    ],
                )],
            },
            Panel {
                id: "integrations",
                label: "Connected apps",
                description: "Services with access to this workspace.",
                icon: "puzzle",
                cards: &[(
                    None,
                    &[row!(
                        "calendarApp",
                        "Calendar",
                        "Connected 2 weeks ago.",
                        "calendar",
                        Control::Button("Disconnect")
                    )],
                )],
            },
            Panel {
                id: "developer",
                label: "Developer",
                description: "API access and pre-release features.",
                icon: "code",
                cards: &[(
                    None,
                    &[
                        row!(
                            "apiAccess",
                            "API access",
                            "Allow personal access tokens.",
                            "key",
                            Control::Switch
                        ),
                        row!(
                            "earlyAccess",
                            "Early access",
                            "Try features before release.",
                            "sparkles",
                            Control::Switch
                        ),
                    ],
                )],
            },
        ],
    ),
];

fn panels() -> impl Iterator<Item = &'static Panel> {
    GROUPS.iter().flat_map(|(_, ps)| ps.iter())
}

fn panel(id: &str) -> &'static Panel {
    panels().find(|p| p.id == id).unwrap_or(&GROUPS[0].1[0])
}

pub struct SettingsDialog {
    pub panel: &'static str,
    pub query: String,
    pub flags: BTreeMap<&'static str, bool>,
    pub choices: BTreeMap<&'static str, String>,
    pub texts: BTreeMap<&'static str, String>,
    pub mode: Mode,
}

impl Default for SettingsDialog {
    fn default() -> Self {
        let flags = [
            ("showLocalTime", true),
            ("twoFactor", true),
            ("emailInvoices", true),
            ("emailMentions", true),
            ("emailComments", true),
            ("weeklyDigest", false),
            ("desktopAlerts", true),
            ("notificationSounds", false),
            ("reduceMotion", false),
            ("discoverable", true),
            ("readReceipts", true),
            ("apiAccess", false),
            ("earlyAccess", false),
        ]
        .into_iter()
        .collect();
        let choices = [
            ("presence", "available"),
            ("theme", "system"),
            ("font", "system"),
            ("density", "balanced"),
            ("language", "en"),
        ]
        .into_iter()
        .map(|(k, v)| (k, v.to_string()))
        .collect();
        let texts = [
            ("displayName", "Alex Johnson"),
            ("jobTitle", "Product designer"),
            ("billingEmail", "billing@example.com"),
        ]
        .into_iter()
        .map(|(k, v)| (k, v.to_string()))
        .collect();
        SettingsDialog {
            panel: "profile",
            query: String::new(),
            flags,
            choices,
            texts,
            mode: Mode::Light,
        }
    }
}

impl SettingsDialog {
    fn control(&self, r: &Row) -> Option<Element<Msg>> {
        let key = r.key;
        Some(match r.control {
            Control::Switch => Switch::new(r.title, *self.flags.get(key).unwrap_or(&false))
                .is_label_hidden(true)
                .on_change(move |on| Msg::Toggle(key, on))
                .into(),
            Control::Select(opts) => Selector::single(
                format!("setting-{key}"),
                r.title,
                opts.iter()
                    .map(|(v, l)| SelectorOption::new(*v, *l))
                    .collect(),
                self.choices.get(key).cloned(),
            )
            .is_label_hidden(true)
            .size(ComponentSize::Sm)
            .width(SizeValue::Points(192.0))
            .on_change(move |v: Vec<String>| {
                Msg::Choose(key, v.into_iter().next().unwrap_or_default())
            })
            .into(),
            Control::Text => {
                TextInput::new(r.title, self.texts.get(key).cloned().unwrap_or_default())
                    .is_label_hidden(true)
                    .size(ComponentSize::Sm)
                    .width(SizeValue::Points(192.0))
                    .on_change(move |s| Msg::Text(key, s))
                    .into()
            }
            Control::Keys(k) => Kbd::new(k).into(),
            Control::Button(words) => Button::new(words)
                .variant(ButtonVariant::Secondary)
                .size(ComponentSize::Sm)
                .on_click(Msg::Noop)
                .into(),
        })
    }

    fn row(&self, r: &Row) -> Element<Msg> {
        let text = VStack::new()
            .gap(Space::S0_5)
            .max_width(SizeValue::Points(420.0))
            .child(Text::new(r.title).r#type(TextType::Label))
            .child(
                Text::new(r.description)
                    .r#type(TextType::Supporting)
                    .color(TextColor::Secondary),
            );
        let mut line = HStack::new()
            .gap(Space::S3)
            .v_align(StackAxisAlign::Center)
            .padding(Space::S4)
            .child(StackItem::fill(
                HStack::new()
                    .gap(Space::S2)
                    .v_align(StackAxisAlign::Start)
                    .child(
                        VStack::new().padding_block_start(Space::S0_5).child(
                            Icon::new(r.icon)
                                .size(IconSize::Sm)
                                .color(IconColor::Secondary),
                        ),
                    )
                    .child(text),
            ));
        if let Some(c) = self.control(r) {
            line = line.child(c);
        }
        line.into()
    }

    fn card(&self, title: Option<&str>, rows: &[&Row]) -> Element<Msg> {
        let mut list = VStack::new();
        for (i, r) in rows.iter().enumerate() {
            if i > 0 {
                list = list.child(Divider::new());
            }
            list = list.child(self.row(r));
        }
        let mut v = VStack::new().gap(Space::S1_5);
        if let Some(t) = title {
            v = v.child(
                Text::new(t)
                    .r#type(TextType::Supporting)
                    .weight(TextWeight::Semibold)
                    .color(TextColor::Secondary),
            );
        }
        v.child(
            Card::new()
                .padding(Space::S0)
                .variant(CardVariant::Muted)
                .child(list),
        )
        .into()
    }

    fn body(&self) -> Element<Msg> {
        let mut v = VStack::new().gap(Space::S4);
        if self.query.trim().is_empty() {
            let p = panel(self.panel);
            v = v.child(
                VStack::new()
                    .gap(Space::S1)
                    .child(Heading::new(2, p.label))
                    .child(Text::new(p.description).color(TextColor::Secondary)),
            );
            for (title, rows) in p.cards {
                let rows: Vec<&Row> = rows.iter().collect();
                v = v.child(self.card(*title, &rows));
            }
        } else {
            let q = self.query.to_lowercase();
            let hits: Vec<(&Panel, &Row)> = panels()
                .flat_map(|p| {
                    p.cards
                        .iter()
                        .flat_map(move |(_, rows)| rows.iter().map(move |r| (p, r)))
                })
                .filter(|(_, r)| {
                    r.title.to_lowercase().contains(&q) || r.description.to_lowercase().contains(&q)
                })
                .collect();
            v = v.child(Heading::new(2, format!("Results for “{}”", self.query)));
            if hits.is_empty() {
                v = v.child(EmptyState::new("No settings found").description("Try another word."));
            }
            let mut list = List::new().label("Matching settings");
            for (p, r) in hits {
                list = list.child(
                    ListItem::new(r.title)
                        .description(format!("{} · {}", p.label, r.description))
                        .on_click(Msg::Panel(p.id)),
                );
            }
            v = v.child(list);
        }
        v.into()
    }

    fn search(&self) -> Element<Msg> {
        TextInput::new("Search settings", self.query.clone())
            .is_label_hidden(true)
            .size(ComponentSize::Sm)
            .start_icon(Icon::new("search"))
            .placeholder("Search settings")
            .has_clear(true)
            .on_change(Msg::Query)
            .into()
    }

    fn side_nav(&self) -> Element<Msg> {
        let mut nav = SideNav::new()
            .width(260.0)
            .label("Settings sections")
            .header(Heading::new(1, "Settings"))
            .top_content(self.search());
        for (group, ps) in GROUPS {
            let mut sec = SideNavSection::new(*group);
            for p in ps.iter() {
                sec = sec.child(
                    SideNavItem::new(p.label)
                        .icon(Icon::new(p.icon))
                        .is_selected(p.id == self.panel && self.query.is_empty())
                        .on_click(Msg::Panel(p.id)),
                );
            }
            nav = nav.child(sec);
        }
        nav.into()
    }

    fn narrow(&self) -> Element<Msg> {
        let mut tabs = HStack::new()
            .gap(Space::S1)
            .is_scrollable(true)
            .padding_inline(Space::S3)
            .padding_block(Space::S2);
        for p in panels() {
            tabs = tabs.child(
                Button::new(p.label)
                    .icon(Icon::new(p.icon))
                    .size(ComponentSize::Sm)
                    .variant(if p.id == self.panel {
                        ButtonVariant::Secondary
                    } else {
                        ButtonVariant::Ghost
                    })
                    .on_click(Msg::Panel(p.id)),
            );
        }
        VStack::new()
            .child(
                HStack::new()
                    .gap(Space::S2)
                    .v_align(StackAxisAlign::Center)
                    .padding(Space::S3)
                    .child(StackItem::fill(
                        Text::new("Settings").r#type(TextType::Label),
                    ))
                    .child(
                        Button::new("Close")
                            .variant(ButtonVariant::Ghost)
                            .size(ComponentSize::Sm)
                            .icon(Icon::new("close"))
                            .is_icon_only(true)
                            .on_click(Msg::Noop),
                    ),
            )
            .child(Divider::new())
            .child(
                VStack::new()
                    .padding_inline(Space::S3)
                    .padding_block(Space::S2)
                    .child(self.search()),
            )
            .child(tabs)
            .child(Divider::new())
            .child(
                VStack::new()
                    .padding(Space::S4)
                    .is_scrollable(true)
                    .child(self.body()),
            )
            .into()
    }
}

impl App for SettingsDialog {
    type Msg = Msg;

    fn view(&self) -> Element<Msg> {
        // The view reads the window width through the shell's size; the
        // runtime rebuilds on resize, so a narrow window gets one column.
        let shell: Element<Msg> = astryx_core::authoring::element(Responsive {
            wide: self.wide(),
            narrow: self.narrow(),
        });
        astryx_core::authoring::element(Page {
            child: Dialog::new(true, |_| Msg::Noop)
                .is_inline(true)
                .purpose(DialogPurpose::Form)
                .width(1120.0)
                .max_height(800.0)
                .label("Settings")
                .child(shell)
                .into(),
        })
    }

    fn update(&mut self, msg: Msg) -> Command {
        match msg {
            Msg::Panel(p) => {
                self.panel = p;
                self.query.clear();
            }
            Msg::Query(q) => self.query = q,
            Msg::Toggle(k, on) => {
                self.flags.insert(k, on);
            }
            Msg::Choose(k, v) => {
                self.choices.insert(k, v);
            }
            Msg::Text(k, v) => {
                self.texts.insert(k, v);
            }
            Msg::Noop => {}
        }
        Command::None
    }

    fn theme(&self) -> ThemeChoice {
        ThemeChoice {
            theme: Theme::NEUTRAL,
            mode: Some(self.mode),
        }
    }
}

impl SettingsDialog {
    fn wide(&self) -> Element<Msg> {
        Layout::new()
            .height(LayoutHeight::Fill)
            .start(LayoutPanel::new().has_divider(true).child(self.side_nav()))
            .content(
                LayoutContent::new()
                    .padding(Space::S6)
                    .is_scrollable(true)
                    .child(
                        HStack::new()
                            .gap(Space::S2)
                            .v_align(StackAxisAlign::Start)
                            .child(StackItem::fill(self.body()))
                            .child(
                                Button::new("Close settings")
                                    .variant(ButtonVariant::Ghost)
                                    .icon(Icon::new("close"))
                                    .is_icon_only(true)
                                    .on_click(Msg::Noop),
                            ),
                    ),
            )
            .into()
    }
}

/// The page: the viewport, with the dialog in its center.
struct Page {
    child: Element<Msg>,
}

impl astryx_core::authoring::Build<Msg> for Page {
    fn build(self: Box<Self>, cx: &mut astryx_core::authoring::BuildCx<Msg>) {
        let (vw, vh) = cx.viewport;
        cx.begin(astryx_core::authoring::Node::new(taffy::Style {
            flex_direction: taffy::FlexDirection::Column,
            align_items: taffy::AlignItems::CENTER,
            justify_content: taffy::JustifyContent::CENTER,
            size: taffy::Size {
                width: taffy::Dimension::length(vw.max(1.0)),
                height: taffy::Dimension::length(vh.max(1.0)),
            },
            padding: taffy::Rect::length(12.0),
            ..taffy::Style::default()
        }));
        cx.child(self.child);
        cx.end();
    }
}

/// Picks the two-column or one-column shell from the viewport width, and
/// sizes it as the template does: up to 800 points tall.
struct Responsive {
    wide: Element<Msg>,
    narrow: Element<Msg>,
}

impl astryx_core::authoring::Build<Msg> for Responsive {
    fn build(self: Box<Self>, cx: &mut astryx_core::authoring::BuildCx<Msg>) {
        let (vw, vh) = cx.viewport;
        let narrow = vw > 0.0 && vw < NARROW;
        let h = if vh > 0.0 {
            (vh - 32.0).min(800.0)
        } else {
            800.0
        };
        cx.begin(astryx_core::authoring::Node::new(taffy::Style {
            flex_direction: taffy::FlexDirection::Column,
            size: taffy::Size {
                width: taffy::Dimension::auto(),
                height: taffy::Dimension::length(h),
            },
            ..taffy::Style::default()
        }));
        cx.child(if narrow { self.narrow } else { self.wide });
        cx.end();
    }
}
