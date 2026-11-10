//! The spike gallery: every component family in every visual state.
//!
//! Two modes:
//! - One case (`Gallery::case`): exactly one component in a box named
//!   "case", padded 16 points on the page ground, for screenshots.
//! - Browse (`Gallery::new`): a family list and every case of the chosen
//!   family, with light/dark and LTR/RTL switches.

pub mod a11y_json;
pub mod families;
pub mod props;

use astryx_core::authoring::{Base, BoxDraw, Build, BuildCx, Draw, Node, Semantics};
use astryx_core::contract::{Command, Role};
use astryx_core::{App, Direction, Element, Mode, Space, ThemeChoice};
use astryx_spike_ui::*;
use families::{Case, Family};
use props::Props;

/// Gallery messages. Components in cases send `Noop` or report their state
/// so interactive cases can be driven.
#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Noop,
    Family(usize),
    ToggleScheme,
    ToggleDirection,
}

pub struct Gallery {
    families: Vec<Family>,
    family: usize,
    case: Option<Case>,
    mode: Mode,
    direction: Direction,
    theme: &'static astryx_core::Theme,
}

impl Default for Gallery {
    fn default() -> Self {
        Self::new()
    }
}

impl Gallery {
    pub fn new() -> Self {
        Gallery {
            families: families::all(),
            family: 0,
            case: None,
            mode: Mode::Light,
            direction: Direction::Ltr,
            theme: astryx_core::Theme::NEUTRAL,
        }
    }

    /// One case by id, or `None` when no family has it.
    pub fn case(id: &str) -> Option<Self> {
        let (case, _) = families::find_case(id)?;
        Some(Gallery {
            case: Some(case),
            ..Self::new()
        })
    }

    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// A theme by name (`neutral`, `astryx`, `butter` …); unknown names keep
    /// the default, `neutral`.
    pub fn with_theme(mut self, name: &str) -> Self {
        if let Some(t) = astryx_core::Theme::ALL.iter().find(|t| t.name == name) {
            self.theme = t;
        }
        self
    }

    pub fn with_direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    fn render(&self, case: &Case) -> Element<Msg> {
        let render = self
            .families
            .iter()
            .find(|f| f.name == case.family)
            .map(|f| f.render)
            .expect("family of case");
        render(Props(&case.props))
    }
}

/// The page ground: the theme's body color, filling the window.
struct Page {
    base: Base,
    child: Option<Element<Msg>>,
    padding: f32,
    name: Option<&'static str>,
}

impl Build<Msg> for Page {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let _ = &self.base;
        // Case pages sit on the surface color, as the Storybook baseline does.
        let ground = if self.name.is_some() {
            cx.tokens.color.background_surface.into()
        } else {
            cx.tokens.color.background_body.into()
        };
        let node = Node::new(taffy::Style {
            flex_direction: taffy::FlexDirection::Column,
            align_items: if self.name.is_some() {
                taffy::AlignItems::FLEX_START
            } else {
                taffy::AlignItems::STRETCH
            },
            size: taffy::Size {
                width: taffy::Dimension::percent(1.0),
                height: taffy::Dimension::percent(1.0),
            },
            padding: taffy::Rect {
                left: taffy::LengthPercentage::length(self.padding),
                right: taffy::LengthPercentage::length(self.padding),
                top: taffy::LengthPercentage::length(self.padding),
                bottom: taffy::LengthPercentage::length(self.padding),
            },
            ..taffy::Style::default()
        })
        .draw(Draw::Box(BoxDraw::fill(
            ground,
            astryx_core::contract::Radii::all(0.0),
        )));
        cx.begin(node);
        if let Some(name) = self.name {
            // The case box hugs the component, with 16 points of ground
            // around it, so a capture can crop to it.
            cx.begin(
                Node::new(taffy::Style {
                    flex_direction: taffy::FlexDirection::Column,
                    align_items: taffy::AlignItems::FLEX_START,
                    padding: taffy::Rect {
                        left: taffy::LengthPercentage::length(16.0),
                        right: taffy::LengthPercentage::length(16.0),
                        top: taffy::LengthPercentage::length(16.0),
                        bottom: taffy::LengthPercentage::length(16.0),
                    },
                    ..taffy::Style::default()
                })
                .sem(Semantics::new(Role::Group, name)),
            );
            if let Some(c) = self.child {
                cx.child(c);
            }
            cx.end();
        } else if let Some(c) = self.child {
            cx.child(c);
        }
        cx.end();
    }
}

fn page(child: Element<Msg>, padding: f32, name: Option<&'static str>) -> Element<Msg> {
    astryx_core::authoring::element(Page {
        base: Base::default(),
        child: Some(child),
        padding,
        name,
    })
}

impl App for Gallery {
    type Msg = Msg;

    fn view(&self) -> Element<Msg> {
        if let Some(case) = &self.case {
            // The QA harness reads the box named "case".
            return page(self.render(case), 0.0, Some("case"));
        }
        let family = &self.families[self.family];
        let nav = VStack::new()
            .gap(Space::S0_5)
            .padding(Space::S2)
            .width(SizeValue::Points(200.0))
            .children(self.families.iter().enumerate().map(|(i, f)| {
                Link::new(f.name)
                    .is_standalone(true)
                    .color(if i == self.family {
                        TextColor::Accent
                    } else {
                        TextColor::Primary
                    })
                    .on_click(Msg::Family(i))
            }));
        let cases = families::cases(family);
        let grid = Grid::new(GridColumns::from(240.0))
            .gap(Space::S4)
            .children(cases.iter().map(|c| {
                Card::new().padding(Space::S3).child(
                    VStack::new()
                        .gap(Space::S2)
                        .child(Text::new(c.id.clone()).r#type(TextType::Supporting))
                        .child(self.render(c)),
                )
            }));
        let header = HStack::new()
            .gap(Space::S3)
            .v_align(StackAxisAlign::Center)
            .child(Heading::new(
                1,
                format!("{} ({} cases)", family.name, cases.len()),
            ))
            .child(
                Link::new("Light / dark")
                    .is_standalone(true)
                    .on_click(Msg::ToggleScheme),
            )
            .child(
                Link::new("LTR / RTL")
                    .is_standalone(true)
                    .on_click(Msg::ToggleDirection),
            );
        let body = Layout::new()
            .header(LayoutHeader::new().has_divider(true).child(header))
            .start(LayoutPanel::new().width(220.0).has_divider(true).child(nav))
            .content(LayoutContent::new().child(grid));
        page(body.into(), 0.0, None)
    }

    fn update(&mut self, msg: Msg) -> Command {
        match msg {
            Msg::Noop => {}
            Msg::Family(i) => self.family = i.min(self.families.len() - 1),
            Msg::ToggleScheme => self.mode = self.mode.toggled(),
            Msg::ToggleDirection => {
                self.direction = if self.direction.is_rtl() {
                    Direction::Ltr
                } else {
                    Direction::Rtl
                }
            }
        }
        Command::None
    }

    fn theme(&self) -> ThemeChoice {
        ThemeChoice {
            theme: self.theme,
            mode: Some(self.mode),
        }
    }

    fn direction(&self) -> Direction {
        self.direction
    }
}

/// Every case id in the gallery, in family order.
/// A repeated id keeps its first family.
pub fn all_case_ids() -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    families::all()
        .iter()
        .flat_map(|f| families::cases(f).into_iter().map(|c| c.id))
        .filter(|id| seen.insert(id.clone()))
        .collect()
}
