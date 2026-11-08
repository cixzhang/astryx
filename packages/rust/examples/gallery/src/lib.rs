//! The Astryx gallery: every component in every state, as one [`App`] that
//! every shell runs. Arrows and Tab move focus, A activates, B resets the
//! last action. The theme selector and the dark mode switch at the top
//! change the theme at run time.

use astryx_core::lab::{
    FaceLayout, KeyboardAction, KeyboardKind, KeyboardState, KeyboardUpdate, OnScreenKeyboard,
};
use astryx_core::paged::Paged;
use astryx_core::*;

pub mod catalog;
pub mod images;
mod lab;
mod world;

/// Runs `gallery` with its sample images registered. Every shell starts the
/// gallery this way.
pub fn runtime(gallery: Gallery) -> Runtime<Gallery> {
    let mut rt = Runtime::new(gallery);
    for (id, data) in images::samples().into_iter().chain(world::images()) {
        rt.register_image(id, data);
    }
    rt
}

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
    SelectableCards,
    Kbd,
    Keyboard,
    Lab,
    World,
}

impl Section {
    pub const ALL: [Section; 20] = [
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
        Section::SelectableCards,
        Section::Kbd,
        Section::Keyboard,
        Section::Lab,
        Section::World,
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
            Section::SelectableCards => "SelectableCard",
            Section::Kbd => "Kbd",
            Section::Keyboard => "OnScreenKeyboard",
            Section::Lab => "Lab",
            Section::World => "World",
        }
    }

    /// What the section shows, in a few words.
    pub fn summary(self) -> &'static str {
        match self {
            Section::Typography => "Type styles, headings, colors, and truncation",
            Section::Stacks => "Gaps, alignment, wrapping, and scrolling",
            Section::Buttons => "Variants, sizes, elevation, loading, and disabled",
            Section::Cards => "Grounds, elevation, and padding",
            Section::Switches => "On and off, sizes, descriptions, and status",
            Section::ListItems => "Selectable, static, disabled, and with content",
            Section::Items => "Density, layout, and alignment",
            Section::Badges => "Status and palette colors",
            Section::Dividers => "Subtle, strong, labeled, vertical, full bleed",
            Section::Avatars => "Sizes, shapes, and status dots",
            Section::Icons => "Every built-in icon, sizes, and colors",
            Section::ProgressBars => "Variants, marks, and indeterminate",
            Section::TextInputs => "Editing, states, and status",
            Section::Selectors => "Variants, menus, and status",
            Section::Thumbnails => "Open, remove, loading, and disabled",
            Section::SelectableCards => "Selected, unselected, variants, and disabled",
            Section::Kbd => "Keyboard shortcuts, gamepad buttons, and key hints",
            Section::Keyboard => "Chord and grid keyboards for gamepads; L1 and R1 together switch",
            Section::Lab => "StatusBar, HoldButton, Image, and card choices (experimental)",
            Section::World => "Board, sprites, and a camera that follows focus (experimental)",
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
            Section::SelectableCards => "selectable-cards",
            Section::Kbd => "kbd",
            Section::Keyboard => "on-screen-keyboard",
            Section::Lab => "lab",
            Section::World => "world",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Theme(String),
    /// `"light"`, `"dark"`, or `"system"`.
    Mode(String),
    /// Moves through the picker's themes by this many steps (R1, L1).
    ShiftTheme(i32),
    /// Light, then dark, then system (Select).
    CycleMode,
    Toggle(usize, bool),
    Pressed(&'static str),
    Pick(String),
    SelectRow(usize),
    Remove(usize),
    Back,
    /// Opens a section page from the home screen.
    Open(Section),
    /// Returns to the home screen.
    Home,
    /// A text field's new value, by label.
    Field(&'static str, String),
    /// Focus moved to a thing on the World board, by id.
    WorldFocus(String),
    /// `"ltr"` or `"rtl"`.
    Direction(String),
    /// A selectable card toggled, by index.
    Card(usize, bool),
    /// A plan chosen in the card group.
    Plan(String),
    Keyboard(KeyboardUpdate),
}

/// The gallery's state.
#[derive(Clone, Debug)]
pub struct Gallery {
    pub theme: &'static Theme,
    /// `None` follows the person's light or dark preference.
    pub mode: Option<Mode>,
    pub direction: Direction,
    /// Shows one section alone, without the header, for screenshots.
    pub only: Option<Section>,
    /// The open section page; `None` is the home screen.
    pub page: Option<Section>,
    pub switches: [bool; 4],
    pub last: Option<&'static str>,
    pub fruit: Option<String>,
    pub selected_row: usize,
    pub removed: Vec<usize>,
    /// A shortcut just changed the theme, so the header names it.
    pub announced: bool,
    /// The text field values, by label.
    pub fields: std::collections::BTreeMap<&'static str, String>,
    /// The World board's focused thing, which the hero walks to.
    pub world_at: Option<String>,
    /// The selectable cards' selection, by index.
    pub cards: [bool; 4],
    /// The chosen plan in the Lab section's card group.
    pub plan: String,
    pub keyboard: KeyboardState,
    /// Where the gamepad's face buttons sit: Nintendo-style on the Brick.
    pub faces: FaceLayout,
    pub typed: String,
    /// What the keyboard last finished with: "Done" or "Cancelled".
    pub finished: Option<&'static str>,
    /// Themes outside the published set, such as pixel themes, listed
    /// before them in the picker and on R1 and L1.
    pub extra_themes: Vec<&'static Theme>,
    /// The heading at the top, "Astryx gallery" unless a shell names it.
    pub title: &'static str,
}

impl Default for Gallery {
    fn default() -> Self {
        Gallery {
            theme: Theme::ASTRYX,
            mode: None,
            direction: Direction::Ltr,
            only: None,
            page: None,
            switches: [false, true, false, true],
            last: None,
            fruit: Some("apple".into()),
            selected_row: 1,
            removed: Vec::new(),
            announced: false,
            fields: [
                ("Email", "ada@example.com"),
                ("Password", "hunter22"),
                ("Small", "sm"),
                ("Large", "lg"),
                ("Disabled", "Can't edit"),
                ("Read only", "Fixed value"),
                ("Loading", "Checking"),
                ("Username", "ada"),
                ("Phone", "555 0100"),
                ("Code", "4242"),
            ]
            .into_iter()
            .map(|(k, v)| (k, v.to_string()))
            .collect(),
            world_at: None,
            cards: [true, false, true, false],
            plan: "pro".into(),
            keyboard: KeyboardState::default(),
            faces: FaceLayout::default(),
            typed: String::new(),
            finished: None,
            extra_themes: Vec::new(),
            title: "Astryx gallery",
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

    pub fn with_direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    pub fn with_title(mut self, title: &'static str) -> Self {
        self.title = title;
        self
    }

    /// Adds `themes` to the picker, before the published ones.
    pub fn with_extra_themes(mut self, themes: impl IntoIterator<Item = &'static Theme>) -> Self {
        self.extra_themes.extend(themes);
        self
    }

    /// Every theme the picker offers, extra themes first.
    fn themes(&self) -> Vec<&'static Theme> {
        self.extra_themes
            .iter()
            .copied()
            .chain(Theme::ALL.iter().copied())
            .collect()
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
        if let Some(section) = self.only {
            return self.section_page(section, false);
        }
        let body = match self.page {
            Some(section) => self.section_page(section, true),
            None => self.home(),
        };
        // The header stays put while the page scrolls, so the theme
        // controls and the shortcut label are always in view.
        VStack::new()
            .child(
                VStack::new()
                    .padding(Space::S6)
                    .padding_block_end(Space::S0)
                    .child(self.header()),
            )
            .child(body)
            .into()
    }

    fn update(&mut self, msg: Msg) -> Command {
        self.announced = matches!(msg, Msg::ShiftTheme(_) | Msg::CycleMode);
        match msg {
            Msg::Theme(name) if name == self.theme.name => {}
            Msg::Theme(name) => {
                self.theme = self
                    .themes()
                    .into_iter()
                    .find(|t| t.name == name)
                    .unwrap_or(Theme::ASTRYX)
            }
            Msg::Mode(mode) => self.mode = mode_from(&mode),
            Msg::ShiftTheme(by) => {
                let themes = self.themes();
                let n = themes.len() as i32;
                let at = themes
                    .iter()
                    .position(|t| std::ptr::eq(*t, self.theme))
                    .unwrap_or(0) as i32;
                self.theme = themes[(at + by).rem_euclid(n) as usize];
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
            // The clear action sends an empty value.
            Msg::Pick(value) => self.fruit = (!value.is_empty()).then_some(value),
            Msg::SelectRow(i) => self.selected_row = i,
            Msg::Remove(i) => self.removed.push(i),
            Msg::Back => self.last = None,
            Msg::Open(section) => self.page = Some(section),
            Msg::Home => self.page = None,
            Msg::Direction(dir) => {
                self.direction = if dir == "rtl" {
                    Direction::Rtl
                } else {
                    Direction::Ltr
                }
            }
            Msg::WorldFocus(id) => self.world_at = Some(id),
            Msg::Card(i, on) => self.cards[i] = on,
            Msg::Plan(plan) => self.plan = plan,
            Msg::Keyboard(update) => {
                self.keyboard = update.state;
                self.typed = update.value;
                match update.action {
                    Some(KeyboardAction::Done) => self.finished = Some("Done"),
                    Some(KeyboardAction::Cancel) => self.finished = Some("Cancelled"),
                    // A real app would keep the layout as a preference here.
                    Some(KeyboardAction::Switched(_)) | None => {}
                }
            }
            Msg::Field(label, value) => {
                self.fields.insert(label, value);
            }
        }
        Command::None
    }

    fn direction(&self) -> Direction {
        self.direction
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
            // B leaves a section page; on the home screen it clears the last action.
            Input::B if self.page.is_some() => Some(Msg::Home),
            Input::B => Some(Msg::Back),
            _ => None,
        }
    }

    /// On the World board, the hero walks to whatever has focus.
    fn on_focus(&self, id: Option<&str>) -> Option<Msg> {
        let id = id.filter(|id| id.starts_with("world-"))?;
        (self.world_at.as_deref() != Some(id)).then(|| Msg::WorldFocus(id.to_string()))
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
        // A theme from outside the published set, such as a pixel theme,
        // is listed too, so the selector shows it.
        let listed = self.themes();
        let mut themes: Vec<SelectorOption> = listed
            .iter()
            .map(|t| SelectorOption::new(t.name).label(capitalize(t.name)))
            .collect();
        if !listed.iter().any(|t| std::ptr::eq(*t, self.theme)) {
            themes.push(SelectorOption::new(self.theme.name).label(capitalize(self.theme.name)));
        }
        let modes = [("light", "Light"), ("dark", "Dark"), ("system", "System")]
            .map(|(value, label)| SelectorOption::new(value).label(label));
        let mut title = VStack::new().gap(Space::S1).child(Heading::new(1, self.title)).child(caption(
            "Arrows and Tab move focus, A activates, B goes back. R1 and L1 change the theme, Select the mode.",
        ));
        title = title.child(caption(&format!(
            "Last action: {}",
            self.last.unwrap_or("none")
        )));
        let mut controls = HStack::new()
            .gap(Space::S4)
            .v_align(StackAlign::End)
            // Wide type, such as a pixel font, wraps the pickers instead of
            // cutting their values short.
            .wrap(StackWrap::Wrap)
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
            )
            .child(
                Selector::new(
                    "Direction",
                    [("ltr", "Left to right"), ("rtl", "Right to left")]
                        .map(|(v, l)| SelectorOption::new(v).label(l)),
                )
                .value(if self.direction.is_rtl() {
                    "rtl"
                } else {
                    "ltr"
                })
                .on_change(Msg::Direction)
                .id("direction"),
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

    /// A text field labeled `label` that edits the gallery's value for it.
    fn field(&self, label: &'static str) -> TextInput<Msg> {
        let value = self.fields.get(label).cloned().unwrap_or_default();
        TextInput::new(label, value).on_change(move |v| Msg::Field(label, v))
    }

    fn text_inputs(&self) -> Element<Msg> {
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
                    self.field("Name")
                        .placeholder("Ada Lovelace")
                        .description("As it appears on your badge"),
                    self.field("Email")
                        .r#type(TextInputType::Email)
                        .has_clear(true),
                    self.field("Password").r#type(TextInputType::Password),
                    self.field("Small").size(FieldSize::Sm),
                    self.field("Large").size(FieldSize::Lg),
                ]),
            ),
            group(
                "Disabled, read only, loading, required, optional",
                grid(vec![
                    self.field("Disabled")
                        .is_disabled(true)
                        .disabled_message("Ask an admin to change this"),
                    self.field("Read only").is_read_only(true),
                    self.field("Loading").is_loading(true),
                    self.field("Required")
                        .is_required(true)
                        .placeholder("Required field"),
                    self.field("Optional").is_optional(true),
                ]),
            ),
            group(
                "Status",
                grid(vec![
                    self.field("Username")
                        .status(Status::error("That name is taken")),
                    self.field("Phone")
                        .status(Status::warning("Check the area code")),
                    self.field("Code").status(Status::success("Code accepted")),
                ]),
            ),
        ]
        .gap(Space::S4)
        .into()
    }

    /// Every section as a row; A opens one.
    fn home(&self) -> Element<Msg> {
        // Rows go straight into the pager, so a page breaks between rows.
        let mut list = Paged::new()
            .gap(Space::S0_5)
            .padding(Space::S6)
            .id("home")
            .child(
                VStack::new()
                    .padding_block_end(Space::S2)
                    .child(Heading::new(2, "Components")),
            );
        for section in Section::ALL {
            list = list.child(
                ListItem::new(section.title())
                    .description(section.summary())
                    // A "go forward" chevron points toward inline end: the paired
                    // glyph swaps under RTL, so it mirrors exactly once.
                    .end_content(
                        Icon::new(if self.direction.is_rtl() {
                            "chevronLeft"
                        } else {
                            "chevronRight"
                        })
                        .color(IconColor::Secondary),
                    )
                    .on_click(Msg::Open(section))
                    .id(section.slug()),
            );
        }
        list.into()
    }

    /// One section as its own screen, laid out into pages, with a way back
    /// when it was opened from the home screen. A single section shown for
    /// goldens scrolls instead, so one image holds all of it.
    fn section_page(&self, section: Section, has_back: bool) -> Element<Msg> {
        // The World board needs the space left on screen for its camera,
        // so it does not page; nor does a single section shown for goldens.
        let back = || {
            HStack::new().child(
                Button::new("All components")
                    .variant(ButtonVariant::Ghost)
                    .on_click(Msg::Home)
                    .id("back"),
            )
        };
        if self.only.is_some() || section == Section::World {
            let mut page = VStack::new()
                .gap(Space::S3)
                .padding(Space::S6)
                .is_scrollable(true)
                .id(section.slug());
            if has_back {
                page = page.child(back());
            }
            return page
                .child(Heading::new(2, section.title()))
                .child(self.section_body(section))
                .into();
        }
        let mut page = Paged::new()
            .gap(Space::S3)
            .padding(Space::S6)
            .id(section.slug());
        if has_back {
            page = page.child(back());
        }
        page.child(Heading::new(2, section.title()))
            .child(self.section_body(section))
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
            Section::TextInputs => self.text_inputs(),
            Section::Selectors => self.selectors(),
            Section::Thumbnails => self.thumbnails(),
            Section::SelectableCards => self.selectable_cards(),
            Section::Kbd => kbd(),
            Section::Keyboard => self.keyboard(),
            Section::Lab => lab::lab(self),
            Section::World => world::world(self.world_at.as_deref()),
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
                    .child(
                        Switch::new("Disabled on", true)
                            .is_disabled(true)
                            .disabled_message("Managed by your organization")
                    )
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
        let mut contained = VStack::new().gap(Space::S2);
        for (i, label) in ["Sunflower", "Tomato", "Basil"].iter().enumerate() {
            contained = contained.child(
                ListItem::new(*label)
                    .description(["Tall, needs sun", "Water daily", "Grows in pots"][i])
                    .is_contained(true)
                    .is_selected(self.selected_row == i)
                    .on_click(Msg::SelectRow(i)),
            );
        }
        vstack![
            group("Selectable rows", list),
            group("Contained rows: a surface and border in every state", contained),
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
                    .child(
                        pick(Selector::new("Fruit", options.clone()))
                            .has_clear(true)
                            .on_change(Msg::Pick)
                    )
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

    fn selectable_cards(&self) -> Element<Msg> {
        let addon = |i: usize, name: &'static str, body: &'static str| {
            SelectableCard::new(name, self.cards[i])
                .on_change(move |on| Msg::Card(i, on))
                .child(
                    VStack::new()
                        .gap(Space::S1)
                        .child(Heading::new(4, name))
                        .child(Text::new(body).r#type(TextType::Supporting)),
                )
        };
        vstack![
            group(
                "Several can be on",
                row()
                    .child(addon(0, "Backups", "Daily, kept 30 days"))
                    .child(addon(1, "Analytics", "Weekly reports"))
                    .child(addon(2, "Support", "Replies within a day")),
            ),
            group(
                "Variants, elevation, and disabled",
                row()
                    .child(addon(3, "Blue", "Rings in its border color").variant(CardVariant::Blue))
                    .child(
                        SelectableCard::new("Green", true)
                            .variant(CardVariant::Green)
                            .on_change(|_| Msg::Pressed("green"))
                            .child(Text::new("Selected, green")),
                    )
                    .child(
                        SelectableCard::new("Raised", true)
                            .elevation(Elevation::Med)
                            .on_change(|_| Msg::Pressed("raised"))
                            .child(Text::new("With a shadow")),
                    )
                    .child(
                        SelectableCard::new("Disabled", false)
                            .is_disabled(true)
                            .on_change(|_| Msg::Pressed("disabled"))
                            .child(Text::new("Not available")),
                    ),
            ),
        ]
        .gap(Space::S4)
        .into()
    }

    fn thumbnails(&self) -> Element<Msg> {
        let mut r = row();
        r = r.child(
            Thumbnail::new()
                .src(images::BEACH)
                .alt("Beach")
                .label("beach.jpg"),
        );
        r = r.child(
            Thumbnail::new()
                .src(images::HILLS)
                .alt("Hills at sunset")
                .label("open.jpg")
                .on_click(Msg::Pressed("thumbnail")),
        );
        for (i, src) in [images::CITY, images::STICKER].into_iter().enumerate() {
            if !self.removed.contains(&i) {
                r = r.child(
                    Thumbnail::new()
                        .src(src)
                        .label(format!("photo-{i}.jpg"))
                        .on_remove(Msg::Remove(i)),
                );
            }
        }
        r = r.child(
            Thumbnail::new()
                .src(images::BEACH)
                .label("uploading.jpg")
                .is_loading(true),
        );
        r = r.child(
            Thumbnail::new()
                .src(images::CITY)
                .label("disabled.jpg")
                .is_disabled(true)
                .on_remove(Msg::Remove(9)),
        );
        let mut fallbacks = row();
        fallbacks = fallbacks.child(Thumbnail::new().label("no image"));
        fallbacks = fallbacks.child(Thumbnail::new().src(images::BROKEN).label("broken.jpg"));
        fallbacks = fallbacks.child(Thumbnail::new().src(images::PENDING).label("pending.jpg"));
        fallbacks = fallbacks.child(Thumbnail::new().label("loading.jpg").is_loading(true));
        vstack![
            group("Images: plain, open, remove, uploading, disabled", r),
            group(
                "No image, failed, still loading, loading without an image",
                fallbacks
            ),
        ]
        .gap(Space::S4)
        .into()
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
        // Each line starts with a Latin word, so only the region's direction
        // makes it read right to left in a right-to-left gallery.
        group(
            "Mixed scripts",
            VStack::new()
                .gap(Space::S1)
                .child(Text::new("Astryx היא מערכת עיצוב פתוחה."))
                .child(Text::new("Astryx هو نظام تصميم مفتوح.")),
        ),
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

impl Gallery {
    fn keyboard(&self) -> Element<Msg> {
        let grid = self.keyboard.kind == KeyboardKind::Grid;
        let finished = match self.finished {
            Some(how) => format!("{how}: \"{}\"", self.typed),
            None if grid => "Move with the D-pad, press A to type".to_string(),
            None => "Hold or tap a direction, then A, B, X, or Y".to_string(),
        };
        vstack![
            OnScreenKeyboard::new("Message", self.typed.clone(), self.keyboard.clone())
                .face_layout(self.faces)
                .on_change(Msg::Keyboard),
            Text::new(finished)
                .size(TextSize::Sm)
                .color(TextColor::Secondary),
        ]
        .gap(Space::S3)
        .into()
    }
}

fn kbd() -> Element<Msg> {
    let hint = |keys: Kbd, words: &str| {
        HStack::new()
            .gap(Space::S2)
            .v_align(StackAlign::Center)
            .child(keys)
            .child(Text::new(words))
    };
    vstack![
        group(
            "Keyboard shortcuts",
            row()
                .child(hint(Kbd::new("mod+k"), "Search"))
                .child(hint(Kbd::new("shift+enter"), "New line"))
                .child(hint(Kbd::new("esc"), "Close"))
                .child(hint(Kbd::new("up"), "Previous")),
        ),
        group(
            "Gamepad buttons",
            row()
                .child(Kbd::gamepad("A"))
                .child(Kbd::gamepad("B"))
                .child(Kbd::gamepad("X"))
                .child(Kbd::gamepad("Y"))
                .child(Kbd::gamepad("L1"))
                .child(Kbd::gamepad("R1"))
                .child(Kbd::gamepad("L2"))
                .child(Kbd::gamepad("R2"))
                .child(Kbd::gamepad("Start"))
                .child(Kbd::gamepad("Select"))
                .child(Kbd::gamepad("Menu"))
                .child(Kbd::gamepad("dpad")),
        ),
        group(
            "Key hints (experimental)",
            astryx_core::lab::KeyHints::new()
                .key("A", "Open")
                .key("B", "Back")
                .key("L1 R1", "Pages")
                .key("Select", "Mode"),
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
            "Scrollable HStack of avatars",
            // Buttons shrink to fit a row, as in Astryx; fixed-size avatars
            // show the scroll instead.
            HStack::new().gap(Space::S2).is_scrollable(true).children(
                [
                    "Ada Lovelace",
                    "Grace Hopper",
                    "Alan Turing",
                    "Katherine Johnson",
                    "Edsger Dijkstra",
                    "Barbara Liskov",
                    "Donald Knuth",
                    "Frances Allen",
                    "John Backus",
                    "Radia Perlman",
                    "Ken Thompson",
                    "Margaret Hamilton",
                    "Dennis Ritchie",
                    "Hedy Lamarr",
                    "Tim Berners-Lee",
                    "Shafi Goldwasser"
                ]
                .map(|name| Avatar::new()
                    .name(name)
                    .size(AvatarSize::Lg)
                    .on_click(Msg::Pressed("avatar"))
                    .id(name)),
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
        group(
            "Tooltips show while focused; B hides them",
            row()
                .child(
                    Button::new("Share")
                        .tooltip("Copies a link to this page")
                        .on_click(Msg::Pressed("share"))
                )
                .child(
                    Button::new("Publish")
                        .variant(ButtonVariant::Primary)
                        .is_disabled(true)
                        .tooltip("Add a title before publishing")
                        .on_click(Msg::Pressed("publish")),
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
    let mut photos = row();
    for s in [
        AvatarSize::Sm,
        AvatarSize::Md,
        AvatarSize::Lg,
        AvatarSize::Xl,
    ] {
        photos = photos.child(Avatar::new().name("Ada Lovelace").src(images::ADA).size(s));
    }
    for (shape, src) in [
        (AvatarShape::Rounded, images::GRACE),
        (AvatarShape::Square, images::ALAN),
    ] {
        photos = photos.child(
            Avatar::new()
                .name("Grace Hopper")
                .src(src)
                .size(AvatarSize::Lg)
                .shape(shape),
        );
    }
    photos = photos.child(
        Avatar::new()
            .name("Katherine Johnson")
            .src(images::KATHERINE)
            .size(AvatarSize::Lg)
            .status(AvatarStatusDot::new(AvatarStatusDotVariant::Success).label("Online")),
    );
    let fallbacks = row()
        .child(
            Avatar::new()
                .name("Alan Turing")
                .src(images::BROKEN)
                .fallback_src(images::ALAN)
                .size(AvatarSize::Lg),
        )
        .child(
            Avatar::new()
                .name("Alan Turing")
                .src(images::BROKEN)
                .size(AvatarSize::Lg),
        )
        .child(
            Avatar::new()
                .name("Alan Turing")
                .src(images::PENDING)
                .fallback_src(images::ALAN)
                .size(AvatarSize::Lg),
        )
        .child(Avatar::new().src(images::BROKEN).size(AvatarSize::Lg));
    vstack![
        group("Sizes", sizes),
        group("Shapes and the person symbol", shapes),
        group("Photos", photos),
        group(
            "Failed then fallback, failed, still loading, failed without a name",
            fallbacks
        ),
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
