//! The Astryx gallery: every component in every state, as one [`App`] that
//! every shell runs. Arrows and Tab move focus, A activates, B resets the
//! last action. The theme selector and the dark mode switch at the top
//! change the theme at run time.

use astryx_core::*;

/// One part of the gallery, for showing a single component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Section {
    Typography,
    Stacks,
    Buttons,
    Cards,
    Switches,
    ListItems,
    Items,
    Badges,
    Dividers,
    Avatars,
    Icons,
    ProgressBars,
    TextInputs,
    Selectors,
    Thumbnails,
}

impl Section {
    pub const ALL: [Section; 15] = [
        Section::Typography,
        Section::Stacks,
        Section::Buttons,
        Section::Cards,
        Section::Switches,
        Section::ListItems,
        Section::Items,
        Section::Badges,
        Section::Dividers,
        Section::Avatars,
        Section::Icons,
        Section::ProgressBars,
        Section::TextInputs,
        Section::Selectors,
        Section::Thumbnails,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Section::Typography => "Text and Heading",
            Section::Stacks => "VStack and HStack",
            Section::Buttons => "Button",
            Section::Cards => "Card",
            Section::Switches => "Switch",
            Section::ListItems => "ListItem",
            Section::Items => "Item",
            Section::Badges => "Badge",
            Section::Dividers => "Divider",
            Section::Avatars => "Avatar",
            Section::Icons => "Icon",
            Section::ProgressBars => "ProgressBar",
            Section::TextInputs => "TextInput",
            Section::Selectors => "Selector",
            Section::Thumbnails => "Thumbnail",
        }
    }

    /// A file-friendly name, such as `"list-items"`.
    pub fn slug(self) -> &'static str {
        match self {
            Section::Typography => "typography",
            Section::Stacks => "stacks",
            Section::Buttons => "buttons",
            Section::Cards => "cards",
            Section::Switches => "switches",
            Section::ListItems => "list-items",
            Section::Items => "items",
            Section::Badges => "badges",
            Section::Dividers => "dividers",
            Section::Avatars => "avatars",
            Section::Icons => "icons",
            Section::ProgressBars => "progress-bars",
            Section::TextInputs => "text-inputs",
            Section::Selectors => "selectors",
            Section::Thumbnails => "thumbnails",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Theme(String),
    /// `"light"`, `"dark"`, or `"system"`.
    Mode(String),
    /// Moves through `Theme::ALL` by this many steps (R1, L1).
    ShiftTheme(i32),
    /// Light, then dark, then system (Select).
    CycleMode,
    Toggle(usize, bool),
    Pressed(&'static str),
    Pick(String),
    SelectRow(usize),
    Remove(usize),
    Back,
}

/// The gallery's state.
#[derive(Clone, Debug)]
pub struct Gallery {
    pub theme: &'static Theme,
    /// `None` follows the person's light or dark preference.
    pub mode: Option<Mode>,
    /// Shows one section instead of all of them.
    pub only: Option<Section>,
    pub switches: [bool; 4],
    pub last: Option<&'static str>,
    pub fruit: Option<String>,
    pub selected_row: usize,
    pub removed: Vec<usize>,
    /// A shortcut just changed the theme, so the header names it.
    pub announced: bool,
}

impl Default for Gallery {
    fn default() -> Self {
        Gallery {
            theme: Theme::ASTRYX,
            mode: None,
            only: None,
            switches: [false, true, false, true],
            last: None,
            fruit: Some("apple".into()),
            selected_row: 1,
            removed: Vec::new(),
            announced: false,
        }
    }
}

impl Gallery {
    pub fn new() -> Self {
        Self::default()
    }

    /// The gallery showing only `section`.
    pub fn section(section: Section) -> Self {
        Gallery {
            only: Some(section),
            ..Self::default()
        }
    }

    pub fn with_theme(mut self, theme: &'static Theme, mode: Mode) -> Self {
        self.theme = theme;
        self.mode = Some(mode);
        self
    }
}

impl App for Gallery {
    type Msg = Msg;

    fn view(&self) -> Element<Msg> {
        let sections = match self.only {
            Some(s) => vec![s],
            None => Section::ALL.to_vec(),
        };
        let mut page = VStack::new()
            .gap(Space::S6)
            .padding(Space::S6)
            .is_scrollable(true);
        for section in sections {
            page = page.child(
                VStack::new()
                    .gap(Space::S3)
                    .id(section.slug())
                    .child(Heading::new(2, section.title()))
                    .child(self.section_body(section)),
            );
        }
        if self.only.is_some() {
            return page.into();
        }
        // The header stays put while the sections scroll, so the theme
        // controls and the shortcut label are always in view.
        VStack::new()
            .child(
                VStack::new()
                    .padding(Space::S6)
                    .padding_block_end(Space::S0)
                    .child(self.header()),
            )
            .child(page)
            .into()
    }

    fn update(&mut self, msg: Msg) -> Command {
        self.announced = matches!(msg, Msg::ShiftTheme(_) | Msg::CycleMode);
        match msg {
            Msg::Theme(name) => self.theme = Theme::by_name(&name).unwrap_or(Theme::ASTRYX),
            Msg::Mode(mode) => self.mode = mode_from(&mode),
            Msg::ShiftTheme(by) => {
                let n = Theme::ALL.len() as i32;
                let at = Theme::ALL
                    .iter()
                    .position(|t| std::ptr::eq(*t, self.theme))
                    .unwrap_or(0) as i32;
                self.theme = Theme::ALL[(at + by).rem_euclid(n) as usize];
            }
            Msg::CycleMode => {
                self.mode = match self.mode {
                    Some(Mode::Light) => Some(Mode::Dark),
                    Some(Mode::Dark) => None,
                    None => Some(Mode::Light),
                }
            }
            Msg::Toggle(i, on) => self.switches[i] = on,
            Msg::Pressed(what) => self.last = Some(what),
            Msg::Pick(value) => self.fruit = Some(value),
            Msg::SelectRow(i) => self.selected_row = i,
            Msg::Remove(i) => self.removed.push(i),
            Msg::Back => self.last = None,
        }
        Command::None
    }

    fn theme(&self) -> ThemeChoice {
        ThemeChoice {
            theme: self.theme,
            mode: self.mode,
        }
    }

    /// D-pad devices switch themes from anywhere: R1 and L1 step through
    /// the themes, Select cycles the mode, B clears the last action.
    fn on_input(&self, input: Input) -> Option<Msg> {
        match input {
            Input::R1 => Some(Msg::ShiftTheme(1)),
            Input::L1 => Some(Msg::ShiftTheme(-1)),
            Input::Select => Some(Msg::CycleMode),
            Input::B => Some(Msg::Back),
            _ => None,
        }
    }
}

fn row() -> HStack<Msg> {
    HStack::new()
        .gap(Space::S3)
        .wrap(StackWrap::Wrap)
        .v_align(StackAlign::Center)
}

fn caption(text: &str) -> Text {
    Text::new(text).r#type(TextType::Supporting)
}

/// A captioned group of examples.
fn group(name: &str, content: impl Into<Element<Msg>>) -> VStack<Msg> {
    VStack::new()
        .gap(Space::S2)
        .child(caption(name))
        .child(content)
}

impl Gallery {
    fn header(&self) -> Element<Msg> {
        let themes: Vec<SelectorOption> = Theme::ALL
            .iter()
            .map(|t| SelectorOption::new(t.name).label(capitalize(t.name)))
            .collect();
        let modes = [("light", "Light"), ("dark", "Dark"), ("system", "System")]
            .map(|(value, label)| SelectorOption::new(value).label(label));
        let mut title = VStack::new().gap(Space::S1).child(Heading::new(1, "Astryx gallery")).child(caption(
            "Arrows and Tab move focus, A activates, B goes back. R1 and L1 change the theme, Select the mode.",
        ));
        title = title.child(caption(&format!(
            "Last action: {}",
            self.last.unwrap_or("none")
        )));
        let mut controls = HStack::new()
            .gap(Space::S4)
            .v_align(StackAlign::End)
            .child(
                Selector::new("Theme", themes)
                    .value(self.theme.name)
                    .on_change(Msg::Theme)
                    .id("theme"),
            )
            .child(
                Selector::new("Mode", modes)
                    .value(mode_value(self.mode))
                    .on_change(Msg::Mode)
                    .id("mode"),
            );
        if self.announced {
            let mode = match self.mode {
                Some(Mode::Light) => "light",
                Some(Mode::Dark) => "dark",
                None => "system mode",
            };
            controls = controls.child(
                VStack::new().padding_block_end(Space::S1_5).child(
                    Badge::new(format!("{}, {mode}", capitalize(self.theme.name)))
                        .variant(BadgeVariant::Info)
                        .id("announce"),
                ),
            );
        }
        Card::new()
            .elevation(Elevation::Low)
            .child(
                HStack::new()
                    .gap(Space::S6)
                    .h_align(StackJustify::Between)
                    .v_align(StackAlign::End)
                    .wrap(StackWrap::Wrap)
                    .child(title)
                    .child(controls),
            )
            .into()
    }

    fn section_body(&self, section: Section) -> Element<Msg> {
        match section {
            Section::Typography => typography(),
            Section::Stacks => stacks(),
            Section::Buttons => buttons(),
            Section::Cards => cards(),
            Section::Switches => self.switches(),
            Section::ListItems => self.list_items(),
            Section::Items => items(),
            Section::Badges => badges(),
            Section::Dividers => dividers(),
            Section::Avatars => avatars(),
            Section::Icons => icons(),
            Section::ProgressBars => progress_bars(),
            Section::TextInputs => text_inputs(),
            Section::Selectors => self.selectors(),
            Section::Thumbnails => self.thumbnails(),
        }
    }

    fn switches(&self) -> Element<Msg> {
        let s = self.switches;
        vstack![
            group(
                "Off and on, md and sm",
                row()
                    .child(Switch::new("Wi-Fi", s[0]).on_change(|v| Msg::Toggle(0, v)))
                    .child(Switch::new("Bluetooth", s[1]).on_change(|v| Msg::Toggle(1, v)))
                    .child(
                        Switch::new("Small", s[2])
                            .size(SwitchSize::Sm)
                            .on_change(|v| Msg::Toggle(2, v))
                    )
                    .child(
                        Switch::new("Small on", s[3])
                            .size(SwitchSize::Sm)
                            .on_change(|v| Msg::Toggle(3, v))
                    ),
            ),
            group(
                "Disabled, loading, hidden label",
                row()
                    .child(Switch::new("Disabled off", false).is_disabled(true))
                    .child(Switch::new("Disabled on", true).is_disabled(true))
                    .child(Switch::new("Loading", true).is_loading(true))
                    .child(Switch::new("Hidden label", true).is_label_hidden(true)),
            ),
            group(
                "Description, required, optional, status",
                VStack::new()
                    .gap(Space::S3)
                    .child(
                        Switch::new("Notifications", true)
                            .description("Alerts for new messages")
                            .is_required(true)
                    )
                    .child(Switch::new("Newsletter", false).is_optional(true))
                    .child(
                        Switch::new("Backups", false)
                            .status(Status::error("Turn on backups to continue"))
                    )
                    .child(
                        Switch::new("Sync", true)
                            .status(Status::warning("Syncing uses mobile data"))
                    )
                    .child(Switch::new("Updates", true).status(Status::success("Up to date"))),
            ),
        ]
        .gap(Space::S4)
        .into()
    }

    fn list_items(&self) -> Element<Msg> {
        let rows = ["Inbox", "Drafts", "Archive"];
        let mut list = VStack::new().gap(Space::S0_5);
        for (i, label) in rows.iter().enumerate() {
            list = list.child(
                ListItem::new(*label)
                    .description(format!("{} messages", 3 * i + 2))
                    .start_content(
                        Icon::new(["menu", "copy", "upload"][i]).color(IconColor::Secondary),
                    )
                    .is_selected(self.selected_row == i)
                    .on_click(Msg::SelectRow(i)),
            );
        }
        vstack![
            group("Selectable rows", list),
            group(
                "Static, disabled, end content",
                VStack::new()
                    .gap(Space::S0_5)
                    .child(ListItem::new("Static row"))
                    .child(ListItem::new("With a description").description("Supporting words under the label"))
                    .child(ListItem::new("Disabled row").description("Reachable, does nothing").is_disabled(true).on_click(Msg::Pressed("disabled row")))
                    .child(
                        ListItem::new("Ada Lovelace")
                            .description("Mathematician")
                            .start_content(Avatar::new().name("Ada Lovelace").size(AvatarSize::Sm))
                            .end_content(Badge::new("New").variant(BadgeVariant::Info)),
                    )
                    .child(
                        ListItem::new("Airplane mode")
                            .end_content(Switch::new("Airplane mode", self.switches[2]).is_label_hidden(true).on_change(|v| Msg::Toggle(2, v))),
                    )
                    .child(ListItem::new("A long label that runs past the width of the row and truncates with an ellipsis").description("A long description that also truncates to one line by default")),
            ),
        ]
        .gap(Space::S4)
        .into()
    }

    fn selectors(&self) -> Element<Msg> {
        let fruit = ["apple", "banana", "cherry", "durian"];
        let options: Vec<SelectorOption> = fruit
            .iter()
            .map(|f| {
                SelectorOption::new(*f)
                    .label(capitalize(f))
                    .disabled(*f == "durian")
            })
            .collect();
        let pick = |s: Selector<Msg>| match &self.fruit {
            Some(v) => s.value(v.clone()),
            None => s,
        };
        vstack![
            group(
                "Variants",
                row()
                    .child(pick(Selector::new("Fruit", options.clone())).on_change(Msg::Pick))
                    .child(
                        pick(Selector::new("Secondary", options.clone()))
                            .variant(SelectorVariant::Secondary)
                            .on_change(Msg::Pick)
                    )
                    .child(
                        pick(Selector::new("Ghost", options.clone()))
                            .variant(SelectorVariant::Ghost)
                            .on_change(Msg::Pick)
                    ),
            ),
            group(
                "Placeholder, disabled, loading, status",
                row()
                    .child(
                        Selector::new("Empty", options.clone())
                            .placeholder("Choose a fruit")
                            .description("Nothing picked yet")
                    )
                    .child(pick(Selector::new("Disabled", options.clone())).is_disabled(true))
                    .child(pick(Selector::new("Loading", options.clone())).is_loading(true))
                    .child(
                        pick(Selector::new("Required", options))
                            .is_required(true)
                            .status(Status::error("Pick a fruit"))
                    ),
            ),
        ]
        .gap(Space::S4)
        .into()
    }

    fn thumbnails(&self) -> Element<Msg> {
        let mut r = row();
        r = r.child(Thumbnail::new().label("beach.jpg"));
        r = r.child(
            Thumbnail::new()
                .label("open.jpg")
                .on_click(Msg::Pressed("thumbnail")),
        );
        for i in 0..2 {
            if !self.removed.contains(&i) {
                r = r.child(
                    Thumbnail::new()
                        .label(format!("photo-{i}.jpg"))
                        .on_remove(Msg::Remove(i)),
                );
            }
        }
        r = r.child(Thumbnail::new().label("loading.jpg").is_loading(true));
        r = r.child(
            Thumbnail::new()
                .label("disabled.jpg")
                .is_disabled(true)
                .on_remove(Msg::Remove(9)),
        );
        group("Plain, open, remove, loading, disabled", r).into()
    }
}

fn mode_from(value: &str) -> Option<Mode> {
    match value {
        "light" => Some(Mode::Light),
        "dark" => Some(Mode::Dark),
        _ => None,
    }
}

fn mode_value(mode: Option<Mode>) -> &'static str {
    match mode {
        Some(Mode::Light) => "light",
        Some(Mode::Dark) => "dark",
        None => "system",
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

fn typography() -> Element<Msg> {
    let types = [
        (TextType::Body, "Body"),
        (TextType::Large, "Large"),
        (TextType::Label, "Label"),
        (TextType::Supporting, "Supporting"),
        (TextType::Code, "Code"),
    ];
    let mut by_type = VStack::new().gap(Space::S1);
    for (t, name) in types {
        by_type = by_type.child(Text::new(format!("{name}: the quick brown fox")).r#type(t));
    }
    let colors = [
        (TextColor::Primary, "Primary"),
        (TextColor::Secondary, "Secondary"),
        (TextColor::Disabled, "Disabled"),
        (TextColor::Placeholder, "Placeholder"),
        (TextColor::Accent, "Accent"),
    ];
    let mut by_color = row();
    for (c, name) in colors {
        by_color = by_color.child(Text::new(name).color(c));
    }
    let weights = [
        (TextWeight::Normal, "Normal"),
        (TextWeight::Medium, "Medium"),
        (TextWeight::Semibold, "Semibold"),
        (TextWeight::Bold, "Bold"),
    ];
    let mut by_weight = row();
    for (w, name) in weights {
        by_weight = by_weight.child(Text::new(name).weight(w));
    }
    let mut headings = VStack::new().gap(Space::S1);
    for level in 1..=6 {
        headings = headings.child(Heading::new(level, format!("Heading {level}")));
    }
    vstack![
        group(
            "Display",
            VStack::new()
                .gap(Space::S1)
                .child(Heading::new(1, "Display 1").r#type(HeadingType::Display1))
                .child(Text::new("Display 2").r#type(TextType::Display2))
                .child(Text::new("Display 3").r#type(TextType::Display3)),
        ),
        group("Headings", headings),
        group("Types", by_type),
        group("Colors", by_color),
        group("Weights", by_weight),
        group(
            "Sizes, strikethrough, justify, max lines",
            VStack::new()
                .gap(Space::S1)
                .child(row().child(Text::new("2xs").size(TextSize::S2xs)).child(Text::new("sm").size(TextSize::Sm)).child(Text::new("xl").size(TextSize::Xl)).child(Text::new("3xl").size(TextSize::S3xl)))
                .child(Text::new("Struck through").has_strikethrough(true))
                .child(Text::new("Centered").justify(TextJustify::Center))
                .child(Text::new("End aligned").justify(TextJustify::End))
                .child(Text::new("One line only: this sentence is long enough that it cannot fit on a single line of the gallery and ends with an ellipsis instead of wrapping onto the next line").max_lines(1))
                .child(Text::new("Wrapping text flows onto as many lines as it needs. Astryx text wraps at the width its container offers, and its height is a whole number of lines of the type style's line height.")),
        ),
    ]
    .gap(Space::S4)
    .into()
}

fn swatch(label: &str) -> Card<Msg> {
    Card::new()
        .variant(CardVariant::Muted)
        .padding(Space::S2)
        .child(Text::new(label).r#type(TextType::Supporting))
}

fn stacks() -> Element<Msg> {
    let aligned = |j: StackJustify, name: &str| {
        group(
            name,
            Card::new().padding(Space::S2).child(
                HStack::new().gap(Space::S2).h_align(j).children([
                    swatch("A"),
                    swatch("B"),
                    swatch("C"),
                ]),
            ),
        )
    };
    vstack![
        group(
            "HStack gaps 1, 3, 6",
            VStack::new()
                .gap(Space::S2)
                .children([Space::S1, Space::S3, Space::S6].map(|g| {
                    HStack::new()
                        .gap(g)
                        .children([swatch("One"), swatch("Two"), swatch("Three")])
                }))
        ),
        aligned(StackJustify::Start, "hAlign start"),
        aligned(StackJustify::Center, "hAlign center"),
        aligned(StackJustify::Between, "hAlign between"),
        aligned(StackJustify::Evenly, "hAlign evenly"),
        group(
            "vAlign start, center, end, stretch",
            HStack::new().gap(Space::S3).children(
                [
                    StackAlign::Start,
                    StackAlign::Center,
                    StackAlign::End,
                    StackAlign::Stretch
                ]
                .map(|a| {
                    Card::new().padding(Space::S1).child(
                        HStack::new()
                            .gap(Space::S1)
                            .v_align(a)
                            .child(Card::new().padding(Space::S4).variant(CardVariant::Blue))
                            .child(swatch("x")),
                    )
                })
            ),
        ),
        group(
            "Wrap",
            Card::new().padding(Space::S2).child(
                HStack::new()
                    .gap(Space::S2)
                    .wrap(StackWrap::Wrap)
                    .children((1..=14).map(|i| swatch(&format!("Item {i}"))))
            )
        ),
        group(
            "Scrollable HStack",
            HStack::new().gap(Space::S2).is_scrollable(true).children(
                (1..=12).map(|i| Button::new(format!("Chip {i}")).id(format!("chip-{i}")))
            )
        ),
        group(
            "Padding block 4, inline 8",
            Card::new().padding(Space::S0).child(
                VStack::new()
                    .padding_block(Space::S4)
                    .padding_inline(Space::S8)
                    .child(swatch("Padded"))
            )
        ),
    ]
    .gap(Space::S4)
    .into()
}

fn buttons() -> Element<Msg> {
    let variants = [
        (ButtonVariant::Primary, "Primary"),
        (ButtonVariant::Secondary, "Secondary"),
        (ButtonVariant::Ghost, "Ghost"),
        (ButtonVariant::Destructive, "Destructive"),
    ];
    let state_row = |name: &'static str, f: fn(Button<Msg>) -> Button<Msg>| {
        let mut r = row();
        for (v, label) in variants {
            r = r.child(f(Button::new(label)
                .variant(v)
                .on_click(Msg::Pressed(label))));
        }
        group(name, r)
    };
    vstack![
        state_row("Variants", |b| b),
        state_row("Disabled (focusable, inert)", |b| b.is_disabled(true)),
        state_row("Loading", |b| b.is_loading(true)),
        group(
            "Sizes sm, md, lg",
            row()
                .child(
                    Button::new("Small")
                        .size(ButtonSize::Sm)
                        .on_click(Msg::Pressed("small"))
                )
                .child(
                    Button::new("Medium")
                        .size(ButtonSize::Md)
                        .on_click(Msg::Pressed("medium"))
                )
                .child(
                    Button::new("Large")
                        .size(ButtonSize::Lg)
                        .on_click(Msg::Pressed("large"))
                ),
        ),
        group(
            "Elevation low, med, high",
            row()
                .child(
                    Button::new("Low")
                        .elevation(Elevation::Low)
                        .on_click(Msg::Pressed("low"))
                )
                .child(
                    Button::new("Med")
                        .elevation(Elevation::Med)
                        .on_click(Msg::Pressed("med"))
                )
                .child(
                    Button::new("High")
                        .elevation(Elevation::High)
                        .on_click(Msg::Pressed("high"))
                ),
        ),
        group(
            "Width fill",
            Button::new("Fills the width")
                .variant(ButtonVariant::Primary)
                .width(ButtonWidth::Fill)
                .on_click(Msg::Pressed("fill"))
        ),
        group(
            "Interruptible loading, long label",
            row()
                .child(
                    Button::new("Cancel upload")
                        .is_loading(true)
                        .is_interruptible(true)
                        .on_click(Msg::Pressed("interrupt"))
                )
                .child(
                    HStack::new().child(
                        Button::new("A very long button label that truncates inside a narrow row")
                            .on_click(Msg::Pressed("long"))
                    )
                ),
        ),
    ]
    .gap(Space::S4)
    .into()
}

fn cards() -> Element<Msg> {
    let variants = [
        (CardVariant::Default, "Default"),
        (CardVariant::Transparent, "Transparent"),
        (CardVariant::Muted, "Muted"),
        (CardVariant::Blue, "Blue"),
        (CardVariant::Cyan, "Cyan"),
        (CardVariant::Gray, "Gray"),
        (CardVariant::Green, "Green"),
        (CardVariant::Orange, "Orange"),
        (CardVariant::Pink, "Pink"),
        (CardVariant::Purple, "Purple"),
        (CardVariant::Red, "Red"),
        (CardVariant::Teal, "Teal"),
        (CardVariant::Yellow, "Yellow"),
    ];
    let mut by_variant = row();
    for (v, name) in variants {
        by_variant = by_variant.child(Card::new().variant(v).child(Text::new(name)));
    }
    let mut by_elevation = row();
    for (e, name) in [
        (Elevation::None, "None"),
        (Elevation::Low, "Low"),
        (Elevation::Med, "Med"),
        (Elevation::High, "High"),
    ] {
        by_elevation = by_elevation.child(Card::new().elevation(e).child(Text::new(name)));
    }
    let mut by_padding = row();
    for (p, name) in [
        (Space::S0, "0"),
        (Space::S2, "2"),
        (Space::S4, "4 (default)"),
        (Space::S8, "8"),
    ] {
        by_padding = by_padding.child(Card::new().padding(p).child(swatch(name)));
    }
    vstack![
        group("Variants", by_variant),
        group("Elevation", by_elevation),
        group("Padding", by_padding),
        group(
            "Composed",
            Card::new().elevation(Elevation::Low).child(
                VStack::new()
                    .gap(Space::S2)
                    .child(Heading::new(3, "Storage"))
                    .child(Text::new("You have used 7.5 of 10 GB.").r#type(TextType::Supporting))
                    .child(
                        ProgressBar::new("Storage used")
                            .value(75.0)
                            .is_label_hidden(true)
                    )
                    .child(
                        HStack::new()
                            .gap(Space::S2)
                            .child(
                                Button::new("Upgrade")
                                    .variant(ButtonVariant::Primary)
                                    .on_click(Msg::Pressed("upgrade"))
                            )
                            .child(
                                Button::new("Manage")
                                    .variant(ButtonVariant::Ghost)
                                    .on_click(Msg::Pressed("manage"))
                            )
                    ),
            ),
        ),
    ]
    .gap(Space::S4)
    .into()
}

fn items() -> Element<Msg> {
    vstack![
        group(
            "Density compact, balanced, spacious",
            Card::new().padding(Space::S1).child(VStack::new().children([ItemDensity::Compact, ItemDensity::Balanced, ItemDensity::Spacious].map(|d| Item::new(format!("{d:?}")).description("Density").density(d).on_click(Msg::Pressed("density"))))),
        ),
        group(
            "Inline layout, highlighted, selected",
            Card::new().padding(Space::S1).child(
                VStack::new()
                    .child(Item::new("Inline").description("Description beside the label").layout(ItemLayout::Inline))
                    .child(Item::new("Highlighted").is_highlighted(true))
                    .child(Item::new("Selected").is_selected(true).end_content(Icon::new("check").size(IconSize::Sm))),
            ),
        ),
        group(
            "Align center, start, first line",
            Card::new().padding(Space::S1).child(VStack::new().children([(ItemAlign::Center, "Center"), (ItemAlign::Start, "Start"), (ItemAlign::FirstLine, "First line")].map(|(a, name)| {
                Item::new(format!("{name}: a label long enough to wrap onto a second line in this row, which shows how the start and end content line up with the words"))
                    .label_lines(2)
                    .align(a)
                    .start_content(Avatar::new().name(name).size(AvatarSize::Sm))
                    .end_content(Badge::new("3"))
            }))),
        ),
    ]
    .gap(Space::S4)
    .into()
}

fn badges() -> Element<Msg> {
    let variants = [
        (BadgeVariant::Neutral, "Neutral"),
        (BadgeVariant::Info, "Info"),
        (BadgeVariant::Success, "Success"),
        (BadgeVariant::Warning, "Warning"),
        (BadgeVariant::Error, "Error"),
        (BadgeVariant::Blue, "Blue"),
        (BadgeVariant::Cyan, "Cyan"),
        (BadgeVariant::Green, "Green"),
        (BadgeVariant::Orange, "Orange"),
        (BadgeVariant::Pink, "Pink"),
        (BadgeVariant::Purple, "Purple"),
        (BadgeVariant::Red, "Red"),
        (BadgeVariant::Teal, "Teal"),
        (BadgeVariant::Yellow, "Yellow"),
    ];
    let mut r = row();
    for (v, name) in variants {
        r = r.child(Badge::new(name).variant(v));
    }
    group("Variants", r).into()
}

fn dividers() -> Element<Msg> {
    vstack![
        group("Subtle", Divider::new()),
        group("Strong", Divider::new().variant(DividerVariant::Strong)),
        group("Label", Divider::new().label("or")),
        group(
            "Vertical",
            HStack::new()
                .gap(Space::S3)
                .child(Text::new("Left"))
                .child(Divider::new().orientation(DividerOrientation::Vertical))
                .child(Text::new("Middle"))
                .child(
                    Divider::new()
                        .orientation(DividerOrientation::Vertical)
                        .variant(DividerVariant::Strong)
                )
                .child(Text::new("Right")),
        ),
        group(
            "Full bleed in a card",
            Card::new().child(
                VStack::new()
                    .gap(Space::S3)
                    .child(Text::new("Above"))
                    .child(Divider::new().is_full_bleed(true))
                    .child(Text::new("Below"))
            ),
        ),
    ]
    .gap(Space::S4)
    .into()
}

fn avatars() -> Element<Msg> {
    let mut sizes = row();
    for s in [
        AvatarSize::Xsm,
        AvatarSize::Sm,
        AvatarSize::Md,
        AvatarSize::Lg,
        AvatarSize::Xl,
    ] {
        sizes = sizes.child(Avatar::new().name("Grace Hopper").size(s));
    }
    let mut shapes = row();
    for s in [
        AvatarShape::Circle,
        AvatarShape::Rounded,
        AvatarShape::Square,
    ] {
        shapes = shapes.child(
            Avatar::new()
                .name("Alan Turing")
                .size(AvatarSize::Lg)
                .shape(s),
        );
    }
    shapes = shapes.child(Avatar::new().size(AvatarSize::Lg));
    let mut status = row();
    for (v, label) in [
        (AvatarStatusDotVariant::Success, "Online"),
        (AvatarStatusDotVariant::Neutral, "Away"),
        (AvatarStatusDotVariant::Error, "Busy"),
    ] {
        for size in [AvatarSize::Md, AvatarSize::Lg, AvatarSize::Xl] {
            status = status.child(
                Avatar::new()
                    .name("Ada Lovelace")
                    .size(size)
                    .status(AvatarStatusDot::new(v).label(label)),
            );
        }
    }
    status = status.child(
        Avatar::new()
            .name("Rounded Status")
            .size(AvatarSize::Lg)
            .shape(AvatarShape::Rounded)
            .status(AvatarStatusDot::new(AvatarStatusDotVariant::Success).label("Online")),
    );
    vstack![
        group("Sizes", sizes),
        group("Shapes and the person symbol", shapes),
        group("Status dots", status),
        group(
            "Pressable",
            row().child(
                Avatar::new()
                    .name("Katherine Johnson")
                    .size(AvatarSize::Lg)
                    .on_click(Msg::Pressed("avatar"))
            )
        ),
    ]
    .gap(Space::S4)
    .into()
}

fn icons() -> Element<Msg> {
    let mut all = row();
    for name in BUILT_IN_ICONS {
        all = all.child(
            VStack::new()
                .gap(Space::S1)
                .h_align(StackAlign::Center)
                .child(Icon::new(name).color(IconColor::Primary))
                .child(caption(name)),
        );
    }
    let mut sizes = row();
    for s in [IconSize::Xsm, IconSize::Sm, IconSize::Md, IconSize::Lg] {
        sizes = sizes.child(Icon::new("info").size(s).color(IconColor::Accent));
    }
    let mut colors = row();
    for c in [
        IconColor::Primary,
        IconColor::Secondary,
        IconColor::Disabled,
        IconColor::Accent,
        IconColor::Info,
        IconColor::Success,
        IconColor::Warning,
        IconColor::Error,
    ] {
        colors = colors.child(Icon::new("success").color(c).label(format!("{c:?}")));
    }
    vstack![
        group("Built in", all),
        group("Sizes", sizes),
        group("Colors", colors)
    ]
    .gap(Space::S4)
    .into()
}

fn progress_bars() -> Element<Msg> {
    let variants = [
        (ProgressBarVariant::Accent, "Accent"),
        (ProgressBarVariant::Success, "Success"),
        (ProgressBarVariant::Warning, "Warning"),
        (ProgressBarVariant::Error, "Error"),
        (ProgressBarVariant::Neutral, "Neutral"),
    ];
    let mut by_variant = VStack::new().gap(Space::S3);
    for (i, (v, name)) in variants.into_iter().enumerate() {
        by_variant = by_variant.child(
            ProgressBar::new(name)
                .value(20.0 * (i as f64 + 1.0))
                .variant(v)
                .has_value_label(true),
        );
    }
    vstack![
        group("Variants with value labels", by_variant),
        group(
            "Empty, full, marks, hidden label, disabled, indeterminate",
            VStack::new()
                .gap(Space::S3)
                .child(ProgressBar::new("Empty").value(0.0).has_value_label(true))
                .child(ProgressBar::new("Full").value(100.0).has_value_label(true))
                .child(ProgressBar::new("With marks").value(60.0).marks([
                    ProgressBarMark {
                        value: 25.0,
                        label: "Start".into()
                    },
                    ProgressBarMark {
                        value: 75.0,
                        label: "Goal".into()
                    }
                ]))
                .child(
                    ProgressBar::new("Hidden label")
                        .value(40.0)
                        .is_label_hidden(true)
                )
                .child(
                    ProgressBar::new("Disabled")
                        .value(40.0)
                        .is_disabled(true)
                        .has_value_label(true)
                )
                .child(ProgressBar::new("Indeterminate").is_indeterminate(true)),
        ),
    ]
    .gap(Space::S4)
    .into()
}

fn text_inputs() -> Element<Msg> {
    let grid = |items: Vec<TextInput<Msg>>| {
        HStack::new()
            .gap(Space::S4)
            .wrap(StackWrap::Wrap)
            .children(items.into_iter().map(|i| VStack::new().child(i)))
    };
    vstack![
        group(
            "Empty, filled, password, sizes",
            grid(vec![
                TextInput::new("Name", "")
                    .placeholder("Ada Lovelace")
                    .description("As it appears on your badge"),
                TextInput::new("Email", "ada@example.com")
                    .r#type(TextInputType::Email)
                    .has_clear(true)
                    .on_change(|_| Msg::Pressed("clear")),
                TextInput::new("Password", "hunter22").r#type(TextInputType::Password),
                TextInput::new("Small", "sm").size(FieldSize::Sm),
                TextInput::new("Large", "lg").size(FieldSize::Lg),
            ]),
        ),
        group(
            "Disabled, read only, loading, required, optional",
            grid(vec![
                TextInput::new("Disabled", "Can't edit").is_disabled(true),
                TextInput::new("Read only", "Fixed value").is_read_only(true),
                TextInput::new("Loading", "Checking").is_loading(true),
                TextInput::new("Required", "")
                    .is_required(true)
                    .placeholder("Required field"),
                TextInput::new("Optional", "").is_optional(true),
            ]),
        ),
        group(
            "Status",
            grid(vec![
                TextInput::new("Username", "ada").status(Status::error("That name is taken")),
                TextInput::new("Phone", "555 0100").status(Status::warning("Check the area code")),
                TextInput::new("Code", "4242").status(Status::success("Code accepted")),
            ]),
        ),
    ]
    .gap(Space::S4)
    .into()
}
