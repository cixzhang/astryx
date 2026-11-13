//! Gallery screens for the spike's infra rows, one case per screen, on the
//! QA harness's conventions: one case in a box named "case", 16 points of
//! padding on the page ground.
//!
//! Cases: `infra-theme`, `infra-i18n-en`, `infra-i18n-ar`, `infra-i18n-fr`,
//! `infra-focus-menu`, `infra-layers-popover-dialog`, `infra-layers-tooltip`,
//! `infra-layers-context-menu`, `infra-motion-switch`,
//! `infra-motion-collapsible`, `infra-gestures-sheet`,
//! `infra-gestures-reorder`, `infra-virtual-10k`.

pub mod a11y_json;
pub mod settled;

use astryx_core::authoring::{
    element, BoxDraw, Build, BuildCx, Draw, KeyResponse, Node, Semantics,
};
use astryx_core::motion::Channel;
use astryx_core::{
    App, Button, Color, Command, Direction, Element, HStack, InputPhase, Key, KeyEvent, Mode,
    Point, Radii, Role, SizeValue, Space, Text, ThemeChoice, VStack,
};
use astryx_spike_foundations::focus::{Key as NavKey, RovingFocus, Typeahead};
use astryx_spike_foundations::i18n::{DateStyle, DateValue};
use astryx_spike_foundations::{args, provide_i18n, I18n};
use astryx_spike_gestures::runtime::{Drag, SnapDrag};
use astryx_spike_gestures::{Axis, PanEvent};
use astryx_spike_layers::placement::{Align, Placement, Side};
use astryx_spike_layers::Layer;
use astryx_spike_virtual::{Kind, VirtualList};
use std::rc::Rc;

/// Every case id, in browse order.
pub const CASES: [&str; 13] = [
    "infra-theme",
    "infra-i18n-en",
    "infra-i18n-ar",
    "infra-i18n-fr",
    "infra-focus-menu",
    "infra-layers-popover-dialog",
    "infra-layers-tooltip",
    "infra-layers-context-menu",
    "infra-motion-switch",
    "infra-motion-collapsible",
    "infra-gestures-sheet",
    "infra-gestures-reorder",
    "infra-virtual-10k",
];

#[derive(Clone, Debug)]
pub enum Msg {
    Case(usize),
    Toggle,
    Expand,
    Pan(PanEvent),
    Close,
    Noop,
}

pub struct Gallery {
    case: usize,
    /// One case only, for capture.
    single: bool,
    mode: Mode,
    direction: Direction,
    theme: &'static astryx_core::Theme,
    on: bool,
    expanded: bool,
    sheet: SnapDrag,
    open: bool,
}

impl Default for Gallery {
    fn default() -> Self {
        Self::new()
    }
}

impl Gallery {
    pub fn new() -> Self {
        Gallery {
            case: 0,
            single: false,
            mode: Mode::Light,
            direction: Direction::Ltr,
            theme: astryx_core::Theme::NEUTRAL,
            on: false,
            expanded: true,
            // Room above the top point for its rubber band.
            sheet: SnapDrag::new(vec![96.0, 280.0, 440.0], 1),
            open: true,
        }
    }

    /// One case by id.
    pub fn case(id: &str) -> Option<Self> {
        let case = CASES.iter().position(|c| *c == id)?;
        Some(Gallery {
            case,
            single: true,
            ..Self::new()
        })
    }

    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    pub fn with_theme(mut self, name: &str) -> Self {
        if let Some(t) = astryx_core::Theme::ALL.iter().find(|t| t.name == name) {
            self.theme = t;
        }
        self
    }

    fn screen(&self) -> Element<Msg> {
        match CASES[self.case] {
            "infra-theme" => theme_screen(),
            "infra-i18n-en" => i18n_screen("en"),
            "infra-i18n-ar" => i18n_screen("ar-SA"),
            "infra-i18n-fr" => i18n_screen("fr-FR"),
            "infra-focus-menu" => element(Menu),
            "infra-layers-popover-dialog" => self.popover_dialog(),
            "infra-layers-tooltip" => tooltip_screen(),
            "infra-layers-context-menu" => context_menu_screen(),
            "infra-motion-switch" => self.switch_screen(),
            "infra-motion-collapsible" => self.collapsible_screen(),
            "infra-gestures-sheet" => self.sheet_screen(),
            "infra-gestures-reorder" => reorder_screen(),
            _ => virtual_screen(),
        }
    }

    fn popover_dialog(&self) -> Element<Msg> {
        let mut body = VStack::new()
            .gap(Space::S2)
            .padding(Space::S4)
            .child(Text::new("Settings"))
            .child(Button::new("Options").id("options"))
            .child(Button::new("Save").on_click(Msg::Close));
        if self.open {
            body = body.child(
                Layer::popover(
                    "options",
                    surface(
                        VStack::new()
                            .padding(Space::S2)
                            .child(Button::new("Duplicate"))
                            .child(Button::new("Archive"))
                            .into(),
                    ),
                )
                .label("Options")
                .placement(Placement::new(Side::Below, Align::Start).offset(4.0))
                .on_dismiss(|_| Msg::Close),
            );
        }
        VStack::new()
            .child(Text::new("Page behind the dialog"))
            .child(Button::new("Behind"))
            .child(
                Layer::dialog(surface(body.into()))
                    .label("Settings")
                    .backdrop(Color::rgba(0.0, 0.0, 0.0, 0.4))
                    .on_dismiss(|_| Msg::Noop),
            )
            .into()
    }

    fn switch_screen(&self) -> Element<Msg> {
        VStack::new()
            .gap(Space::S2)
            .child(element(Thumb { on: self.on }))
            .child(Button::new(if self.on { "Turn off" } else { "Turn on" }).on_click(Msg::Toggle))
            .into()
    }

    fn collapsible_screen(&self) -> Element<Msg> {
        VStack::new()
            .gap(Space::S2)
            .child(
                Button::new(if self.expanded { "Collapse" } else { "Expand" })
                    .on_click(Msg::Expand),
            )
            .child(element(Collapse {
                open: self.expanded,
            }))
            .child(Text::new("Content below reflows"))
            .into()
    }

    fn sheet_screen(&self) -> Element<Msg> {
        let list = VStack::new()
            .id("sheet-list")
            .is_scrollable(true)
            .height(SizeValue::Points(220.0))
            .children((0..40).map(|i| Text::new(format!("Sheet row {i}"))));
        let sheet = Drag::new(
            surface(
                VStack::new()
                    .padding(Space::S3)
                    .gap(Space::S2)
                    .child(Text::new("Drag the sheet"))
                    .child(list)
                    .into(),
            ),
            Axis::Y,
            |e| Some(Msg::Pan(e)),
        )
        .id("sheet");
        element(Offset {
            y: self.sheet.offset,
            release_velocity: self.sheet.release_velocity,
            child: Some(sheet.into()),
        })
    }
}

impl App for Gallery {
    type Msg = Msg;

    fn view(&self) -> Element<Msg> {
        let screen = self.screen();
        let page = element(Page {
            name: self.single.then_some("case"),
            child: Some(screen),
        });
        if self.single {
            return page;
        }
        let nav = VStack::new()
            .gap(Space::S0_5)
            .padding(Space::S2)
            .width(SizeValue::Points(240.0))
            .children(
                CASES
                    .iter()
                    .enumerate()
                    .map(|(i, c)| Button::new(*c).on_click(Msg::Case(i))),
            );
        HStack::new().child(nav).child(page).into()
    }

    fn update(&mut self, msg: Msg) -> Command {
        match msg {
            Msg::Case(i) => self.case = i,
            Msg::Toggle => self.on = !self.on,
            Msg::Expand => self.expanded = !self.expanded,
            Msg::Pan(e) => {
                self.sheet.pan(&e);
            }
            Msg::Close => self.open = false,
            Msg::Noop => {}
        }
        Command::None
    }

    fn theme(&self) -> ThemeChoice {
        ThemeChoice::new(self.theme, self.mode)
    }

    fn direction(&self) -> Direction {
        self.direction
    }
}

/// The page: 16 points of padding on the page ground, named for the harness.
struct Page {
    name: Option<&'static str>,
    child: Option<Element<Msg>>,
}

impl Build<Msg> for Page {
    fn build(mut self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let pad = taffy::LengthPercentage::length(16.0);
        let node = Node::new(taffy::Style {
            flex_direction: taffy::FlexDirection::Column,
            flex_grow: 1.0,
            padding: taffy::Rect {
                left: pad,
                right: pad,
                top: pad,
                bottom: pad,
            },
            ..Default::default()
        })
        .sem(Semantics::role(Role::Group).with_state(|_| {}))
        .draw(Draw::Box(BoxDraw::fill(
            cx.tokens.color.background_body.into(),
            Radii::default(),
        )));
        let mut node = node;
        if let Some(sem) = node.sem.as_mut() {
            sem.label = self.name.map(String::from);
        }
        cx.begin(node);
        if let Some(c) = self.child.take() {
            cx.child(c);
        }
        cx.end();
    }
}

/// A card-like surface for layer content.
fn surface(child: Element<Msg>) -> Element<Msg> {
    element(Surface { child: Some(child) })
}

struct Surface {
    child: Option<Element<Msg>>,
}

impl Build<Msg> for Surface {
    fn build(mut self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let node = Node::new(taffy::Style {
            flex_direction: taffy::FlexDirection::Column,
            ..Default::default()
        })
        .draw(Draw::Box(BoxDraw {
            border: Some((t.color.border.into(), t.size.border_width)),
            ..BoxDraw::fill(
                t.color.background_popover.into(),
                Radii::all(t.radius.container),
            )
        }));
        cx.begin(node);
        if let Some(c) = self.child.take() {
            cx.child(c);
        }
        cx.end();
    }
}

fn theme_screen() -> Element<Msg> {
    let swatch =
        |name: &'static str, pick: fn(&astryx_tokens::Tokens) -> [f32; 4]| -> Element<Msg> {
            HStack::new()
                .gap(Space::S2)
                .child(element(Swatch(pick)))
                .child(Text::new(name))
                .into()
        };
    VStack::new()
        .gap(Space::S2)
        .child(Text::new("Theme tokens (brand, mode)"))
        .child(swatch("accent", |t| t.color.accent))
        .child(swatch("background card", |t| t.color.background_card))
        .child(swatch("text primary", |t| t.color.text_primary))
        .child(swatch("border", |t| t.color.border))
        .child(swatch("background error inverted", |t| {
            t.color.background_error_inverted
        }))
        .child(Button::new("Primary action"))
        .into()
}

struct Swatch(fn(&astryx_tokens::Tokens) -> [f32; 4]);

impl Build<Msg> for Swatch {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let c = (self.0)(cx.tokens);
        cx.leaf(
            Node::new(taffy::Style {
                size: taffy::Size {
                    width: taffy::Dimension::length(24.0),
                    height: taffy::Dimension::length(24.0),
                },
                ..Default::default()
            })
            .draw(Draw::Box(BoxDraw::fill(
                c.into(),
                Radii::all(cx.tokens.radius.element),
            ))),
        );
    }
}

fn i18n_screen(tag: &str) -> Element<Msg> {
    let i18n = I18n::new(tag);
    let mut lines: Vec<Element<Msg>> =
        vec![Text::new(format!("Locale {} ({:?})", i18n.locale().tag, i18n.dir())).into()];
    for n in [0, 1, 2, 3, 11, 100] {
        lines.push(Text::new(i18n.t("@astryx.pagination.nextBy", &args! {"step" => n})).into());
    }
    lines.push(Text::new(i18n.number(1_234_567.891)).into());
    lines.push(Text::new(i18n.date(&DateValue::ymd(2026, 11, 9), DateStyle::Long)).into());
    lines.push(Text::new(i18n.list(&["A", "B", "C"])).into());
    lines.push(Text::new(i18n.t("@astryx.alertDialog.cancel", &args! {})).into());
    provide_i18n(VStack::new().gap(Space::S1).children(lines).into(), i18n)
}

const ITEMS: [&str; 5] = ["Copy", "Cut", "Paste", "Paste and match style", "Delete"];

/// A menu with roving focus and typeahead.
struct Menu;

#[derive(Clone, Debug, Default)]
struct MenuState {
    active: usize,
    typeahead: Typeahead,
    clock: f64,
}

impl Build<Msg> for Menu {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        cx.key(Some("menu"));
        let state = cx.local_handle::<MenuState>();
        let focus = cx.focus_requests();
        let mut node = Node::new(taffy::Style {
            flex_direction: taffy::FlexDirection::Column,
            ..Default::default()
        })
        .sem(Semantics::new(Role::Menu, "Edit"));
        node.on_key = Some(Rc::new(move |e: &KeyEvent| {
            if e.phase != InputPhase::Pressed {
                return KeyResponse::Pass;
            }
            let mut s = state.get();
            let roving = RovingFocus::vertical();
            let next = match &e.key {
                Key::Up => roving.next(ITEMS.len(), Some(s.active), NavKey::Up, |_| false),
                Key::Down => roving.next(ITEMS.len(), Some(s.active), NavKey::Down, |_| false),
                Key::Home => roving.next(ITEMS.len(), Some(s.active), NavKey::Home, |_| false),
                Key::End => roving.next(ITEMS.len(), Some(s.active), NavKey::End, |_| false),
                Key::Character(c) => {
                    s.clock += 100.0;
                    let clock = s.clock;
                    s.typeahead
                        .input(c, clock, &ITEMS, Some(s.active), |_| false)
                }
                _ => return KeyResponse::Pass,
            };
            if let Some(i) = next {
                s.active = i;
                focus.request(format!("menu-item-{i}"));
            }
            state.set(s);
            KeyResponse::Handled
        }));
        cx.key(Some("menu"));
        cx.begin(node);
        for (i, label) in ITEMS.iter().enumerate() {
            cx.child(Button::new(*label).id(format!("menu-item-{i}")).into());
        }
        cx.end();
    }
}

fn tooltip_screen() -> Element<Msg> {
    VStack::new()
        .padding(Space::S10)
        .child(
            VStack::new()
                .padding(Space::S10)
                .child(Button::new("Save").id("save"))
                .child(Layer::tooltip(
                    "save",
                    surface(
                        VStack::new()
                            .padding(Space::S1)
                            .child(Text::new("Save your changes"))
                            .into(),
                    ),
                )),
        )
        .into()
}

fn context_menu_screen() -> Element<Msg> {
    VStack::new()
        .child(Text::new("Right click opens the menu at the pointer"))
        .child(
            Layer::menu_at(
                Point { x: 120.0, y: 80.0 },
                surface(
                    VStack::new()
                        .padding(Space::S1)
                        .child(Button::new("Open"))
                        .child(Button::new("Rename"))
                        .child(Button::new("Delete"))
                        .into(),
                ),
            )
            .label("Row actions"),
        )
        .into()
}

/// A Switch-like thumb on the stateChange spring.
struct Thumb {
    on: bool,
}

impl Build<Msg> for Thumb {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let motion = astryx_spike_motion::tokens(cx);
        let track = Node::new(taffy::Style {
            size: taffy::Size {
                width: taffy::Dimension::length(44.0),
                height: taffy::Dimension::length(24.0),
            },
            padding: taffy::Rect {
                left: taffy::LengthPercentage::length(2.0),
                right: taffy::LengthPercentage::length(2.0),
                top: taffy::LengthPercentage::length(2.0),
                bottom: taffy::LengthPercentage::length(2.0),
            },
            ..Default::default()
        })
        .draw(Draw::Box(BoxDraw::fill(
            if self.on {
                t.color.accent.into()
            } else {
                t.color.border_emphasized.into()
            },
            Radii::all(12.0),
        )));
        cx.key(Some("thumb-track"));
        cx.begin(track);
        cx.key(Some("thumb"));
        cx.leaf(
            Node::new(taffy::Style {
                size: taffy::Size {
                    width: taffy::Dimension::length(20.0),
                    height: taffy::Dimension::length(20.0),
                },
                ..Default::default()
            })
            .draw(Draw::Box(BoxDraw::fill(
                Color::rgba(1.0, 1.0, 1.0, 1.0),
                Radii::all(10.0),
            )))
            .animate(astryx_spike_motion::state_change(
                motion,
                Channel::TranslateX,
                if self.on { 20.0 } else { 0.0 },
            )),
        );
        cx.end();
    }
}

/// A Collapsible body: its height animates; closed content is inert.
struct Collapse {
    open: bool,
}

impl Build<Msg> for Collapse {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let motion = astryx_spike_motion::tokens(cx);
        let mut node = Node::new(taffy::Style {
            flex_direction: taffy::FlexDirection::Column,
            max_size: taffy::Size {
                width: taffy::LengthPercentageAuto::auto(),
                height: if self.open {
                    taffy::LengthPercentageAuto::auto()
                } else {
                    taffy::LengthPercentageAuto::length(0.0)
                },
            },
            ..Default::default()
        });
        node.resize = Some(astryx_spike_motion::collapse(motion));
        node.clip = Some(Radii::default());
        node.inert = !self.open;
        cx.key(Some("collapse"));
        cx.begin(node);
        for i in 0..4 {
            cx.child(Text::new(format!("Collapsible line {i}")).into());
        }
        cx.end();
    }
}

/// Moves its child down by `y` points (a sheet at its snap point).
/// A sheet `y` points down the screen: a spacer above it follows the
/// finger at once and settles on the snap point on the settle spring from
/// the release speed. The spacer is laid out, so the sheet's hit area and
/// accessibility bounds move with it.
struct Offset {
    y: f32,
    release_velocity: Option<f32>,
    child: Option<Element<Msg>>,
}

impl Build<Msg> for Offset {
    fn build(mut self: Box<Self>, cx: &mut BuildCx<Msg>) {
        cx.begin(Node::new(taffy::Style {
            flex_direction: taffy::FlexDirection::Column,
            ..Default::default()
        }));
        let mut spacer = Node::new(taffy::Style {
            size: taffy::Size {
                width: taffy::Dimension::auto(),
                height: taffy::Dimension::length(self.y.max(0.0)),
            },
            flex_shrink: 0.0,
            ..Default::default()
        });
        spacer.resize = Some(astryx_spike_motion::settle(
            astryx_spike_motion::tokens(cx),
            self.release_velocity,
        ));
        cx.leaf(spacer);
        if let Some(c) = self.child.take() {
            cx.child(c);
        }
        cx.end();
    }
}

fn reorder_screen() -> Element<Msg> {
    VStack::new()
        .gap(Space::S1)
        .child(Text::new("Reorder: Space grabs, arrows move, Space drops"))
        .children((1..=5).map(|i| Button::new(format!("Item {i}"))))
        .into()
}

fn virtual_screen() -> Element<Msg> {
    VirtualList::new("big", 10_000, 32.0, |i| {
        Text::new(if i % 4 == 0 {
            format!(
                "Row {} of 10,000, a longer row that wraps onto a second line",
                i + 1
            )
        } else {
            format!("Row {} of 10,000", i + 1)
        })
        .into()
    })
    .kind(Kind::Grid)
    .label("10,000 rows")
    .height(480.0)
    .into()
}
