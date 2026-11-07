//! The element tree apps build, and the node tree the runtime lays out and
//! paints.
//!
//! An [`Element`] is a component that has not been built yet: it holds props,
//! not colors. The runtime builds it against the current theme into a flat
//! list of [`Node`]s (pre-order), which carry concrete styles, colors, focus
//! behavior, and semantics.

use crate::contract::TextDirection;
use crate::contract::{
    Color, FontFamily, Live, PathCmd, Preferences, Radii, Role, TextAlign, TextStyle,
};
use crate::theme::{Direction, ThemeChoice};
use astryx_tokens::{ShadowLayer as TokenShadow, Theme, Tokens};
use taffy::Style;

/// A piece of UI built by a component, such as [`Button`](crate::Button) or
/// [`VStack`](crate::VStack). `Msg` is the app message its controls send.
pub struct Element<Msg> {
    pub(crate) build: Box<dyn Build<Msg>>,
}

impl<Msg: 'static> Element<Msg> {
    /// Paints this element and everything inside it with another theme or
    /// color mode, as a nested Astryx theme provider does.
    pub fn themed(self, choice: ThemeChoice) -> Self {
        Element {
            build: Box::new(Themed {
                choice,
                child: self,
            }),
        }
    }

    /// Lays out this element and everything inside it in `direction`, as an
    /// element with its own `dir` does on the web.
    pub fn direction(self, direction: Direction) -> Self {
        Element {
            build: Box::new(Directed {
                direction,
                child: self,
            }),
        }
    }
}

/// Builds one component into nodes.
pub(crate) trait Build<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>);
}

pub(crate) fn element<Msg, B: Build<Msg> + 'static>(b: B) -> Element<Msg> {
    Element { build: Box::new(b) }
}

struct Directed<Msg> {
    direction: Direction,
    child: Element<Msg>,
}

impl<Msg> Build<Msg> for Directed<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let saved = std::mem::replace(&mut cx.direction, self.direction);
        cx.begin(Node::new(Style {
            flex_direction: taffy::FlexDirection::Column,
            ..Style::default()
        }));
        cx.child(self.child);
        cx.end();
        cx.direction = saved;
    }
}

struct Themed<Msg> {
    choice: ThemeChoice,
    child: Element<Msg>,
}

impl<Msg> Build<Msg> for Themed<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let tokens = self.choice.tokens(&cx.prefs);
        let saved = std::mem::replace(&mut cx.tokens, tokens);
        let saved_theme = std::mem::replace(&mut cx.theme, self.choice.theme);
        cx.inherit.push(Inherited::root(tokens));
        // The scope paints its own ground, so a dark subtree reads as dark.
        let node = Node::new(Style {
            flex_direction: taffy::FlexDirection::Column,
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw::fill(
            tokens.color.background_body.into(),
            Radii::default(),
        )));
        cx.begin(node);
        cx.child(self.child);
        cx.end();
        cx.inherit.pop();
        cx.tokens = saved;
        cx.theme = saved_theme;
    }
}

/// A step on the Astryx spacing scale. `S0_5` is step 0.5, read from the
/// theme's `space.s0_5` token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Space {
    S0,
    S0_5,
    S1,
    S1_5,
    S2,
    S3,
    S4,
    S5,
    S6,
    S8,
    S10,
}

impl Space {
    /// The step's length in points under `tokens`.
    pub fn points(self, tokens: &Tokens) -> f32 {
        let s = &tokens.space;
        match self {
            Space::S0 => s.s0,
            Space::S0_5 => s.s0_5,
            Space::S1 => s.s1,
            Space::S1_5 => s.s1_5,
            Space::S2 => s.s2,
            Space::S3 => s.s3,
            Space::S4 => s.s4,
            Space::S5 => s.s5,
            Space::S6 => s.s6,
            Space::S8 => s.s8,
            Space::S10 => s.s10,
        }
    }
}

/// Astryx base props, shared by every component: `id` keeps the element's
/// identity (focus, accessibility id) stable when its siblings change, and
/// `is_hidden` removes it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Base {
    pub id: Option<String>,
    pub is_hidden: bool,
}

macro_rules! base_props {
    (@methods) => {
        /// Astryx base `id`: a stable identity for focus and accessibility.
        pub fn id(mut self, id: impl Into<String>) -> Self {
            self.base.id = Some(id.into());
            self
        }
        /// Astryx base `isHidden`: removes the element.
        pub fn is_hidden(mut self, is_hidden: bool) -> Self {
            self.base.is_hidden = is_hidden;
            self
        }
    };
    ($ty:ident<Msg>) => {
        impl<Msg> $ty<Msg> {
            $crate::element::base_props!(@methods);
        }
        impl<Msg: Clone + 'static> From<$ty<Msg>> for $crate::Element<Msg> {
            fn from(component: $ty<Msg>) -> Self {
                $crate::element::element(component)
            }
        }
    };
    ($ty:ident) => {
        impl $ty {
            $crate::element::base_props!(@methods);
        }
        impl<Msg: 'static> From<$ty> for $crate::Element<Msg> {
            fn from(component: $ty) -> Self {
                $crate::element::element(component)
            }
        }
    };
}
pub(crate) use base_props;

/// What a node paints for itself, before its children.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Draw {
    None,
    Box(BoxDraw),
    Text(TextDraw),
    Path(PathDraw),
    /// A theme's bitmap icon, one cell per character.
    Bitmap(&'static [&'static str], Color),
    Spinner(Color),
    Image(ImageDraw),
    /// A sprite: square pixels, possibly playing frames.
    Sprite(crate::world::SpriteDraw),
}

/// A registered image filling the node's box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ImageDraw {
    pub image: crate::contract::ImageId,
    pub fit: crate::contract::ImageFit,
    pub radii: Radii,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BoxDraw {
    pub fill: Color,
    pub radii: Radii,
    /// Color and width of a border drawn inside the node's edge.
    pub border: Option<(Color, f32)>,
    pub shadow: &'static [TokenShadow],
}

impl BoxDraw {
    pub fn fill(fill: Color, radii: Radii) -> Self {
        Self {
            fill,
            radii,
            border: None,
            shadow: &[],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextDraw {
    pub text: String,
    pub style: TextStyle,
    pub color: Color,
    pub align: TextAlign,
    /// Lines shown before an ellipsis; `None` shows every line.
    pub max_lines: Option<u32>,
    pub strikethrough: bool,
}

/// A vector shape in a square `view_box`, scaled to the node's box.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PathDraw {
    pub commands: Vec<PathCmd>,
    pub view_box: f32,
    pub fill: Option<Color>,
    pub stroke: Option<(Color, f32)>,
}

/// The focus ring and pressed overlay a node paints for the nearest
/// focusable node around it (or itself).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Indicator {
    pub ring: Option<Color>,
    pub radii: Radii,
    pub overlay: Option<Color>,
    /// A text field's focus: its border in the first color and a 2 point
    /// ring of the second inside it, in place of the outline ring.
    pub field: Option<(Color, Color)>,
}

impl Indicator {
    pub fn ring(ring: Color, radii: Radii, overlay: Option<Color>) -> Self {
        Indicator {
            ring: Some(ring),
            radii,
            overlay,
            field: None,
        }
    }
}

/// Makes a node a focus stop.
pub(crate) struct Interact<Msg> {
    /// Sent when the node is activated (A).
    pub on_activate: Option<Msg>,
    /// Disabled controls keep their focus stop but never activate.
    pub disabled: bool,
    /// Busy controls keep focus, ignore activation, and paint no press.
    pub busy: bool,
    pub popup: PopupAction,
    /// A link the shell opens after `on_activate`, already checked as safe.
    pub href: Option<String>,
    /// Makes the node a text field that takes typed text.
    pub edit: Option<Edit<Msg>>,
}

/// A text field's value and how to report a new one.
pub(crate) struct Edit<Msg> {
    pub value: String,
    pub on_change: std::rc::Rc<dyn Fn(String) -> Msg>,
    /// The index of the node that draws the value, for the caret.
    pub text_node: u32,
    /// Drawn masked, so the caret follows the mask.
    pub masked: bool,
    pub caret: Color,
}

/// What activating a control does to the open popup (a Selector's menu).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum PopupAction {
    #[default]
    None,
    /// Opens this control's popup, or closes it when open.
    Toggle,
    /// Closes the open popup and returns focus to its owner.
    Close,
}

impl<Msg> Interact<Msg> {
    pub fn new(on_activate: Option<Msg>, disabled: bool) -> Self {
        Interact {
            on_activate,
            disabled,
            busy: false,
            popup: PopupAction::None,
            href: None,
            edit: None,
        }
    }
    /// Opens `href` on activation, unless Astryx's navigation rules refuse it.
    pub fn href(mut self, href: Option<&str>) -> Self {
        self.href = href.and_then(safe_destination);
        self
    }
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }
}

/// What assistive technology reads for a node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Semantics {
    pub role: Role,
    pub label: Option<String>,
    pub value: Option<String>,
    pub description: Option<String>,
    pub checked: Option<bool>,
    pub selected: Option<bool>,
    /// The value is secret, such as a password.
    pub secure: bool,
    pub disabled: bool,
    /// Changes to the label are announced. Such a node is always in the
    /// accessibility tree, even inside a labeled control.
    pub live: Option<Live>,
}

impl Semantics {
    pub fn new(role: Role, label: impl Into<String>) -> Self {
        Self {
            role,
            label: Some(label.into()),
            value: None,
            description: None,
            checked: None,
            selected: None,
            secure: false,
            disabled: false,
            live: None,
        }
    }
}

/// An `href` as a link reads it, trimmed and without control characters,
/// or `None` for a script URL or an HTML `data:` URL, which Astryx's
/// navigation destinations refuse.
pub(crate) fn safe_destination(href: &str) -> Option<String> {
    let normalized: String = href
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .to_string();
    if normalized.is_empty() {
        return None;
    }
    let mut comparable = normalized.to_lowercase();
    if let Some(colon) = comparable.find(':') {
        let scheme = &comparable[..colon];
        let is_scheme = scheme
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c));
        if is_scheme {
            comparable = format!(
                "{}:{}",
                scheme,
                comparable[colon + 1..].trim_start_matches(' ')
            );
        }
    }
    let blocked = ["javascript:", "vbscript:", "data:text/html"]
        .iter()
        .any(|b| comparable.starts_with(b));
    (!blocked).then_some(normalized)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Axis {
    Horizontal,
    Vertical,
}

/// One built node. Children always follow their parent in the node list.
pub(crate) struct Node<Msg> {
    pub id: u64,
    pub parent: Option<u32>,
    pub children: Vec<u32>,
    pub style: Style,
    pub draw: Draw,
    pub opacity: f32,
    pub clip: Option<Radii>,
    pub scroll: Option<Axis>,
    pub interact: Option<Interact<Msg>>,
    pub indicator: Option<Indicator>,
    pub sem: Option<Semantics>,
    /// Paint position for a node taken out of flow (spinner over a label):
    /// centered in its parent instead of its laid-out place.
    pub centered: bool,
    /// Slides across its parent once per this many seconds, from fully
    /// before it to past its end (an indeterminate progress fill).
    pub slide: Option<f32>,
    /// Focus stays inside this node's subtree while it exists (an open menu).
    pub trap: bool,
    /// Takes focus when focus has to move into its trap.
    pub autofocus: bool,
    /// Words a tooltip shows while this focus stop has focus.
    pub tooltip: Option<String>,
    /// Built in a right-to-left region.
    pub rtl: bool,
    /// Scales down while pressed, as Astryx's standalone Button does.
    pub press_scale: bool,
    /// Moves and resizes smoothly when a rebuild places it elsewhere in its
    /// parent, as a CSS transition on its transform and size does.
    pub moves: bool,
    /// The pixel grid of the node's theme, in points.
    pub grid: Option<f32>,
    /// The built-in icon this node draws, and its side in points, so a
    /// theme can draw its own icon instead.
    pub icon: Option<(&'static str, f32)>,
    /// The element's `id` prop, when it has one.
    pub name: Option<String>,
    /// A `paged::Paged` viewport: its scroll offset rests on page starts.
    pub pager: bool,
    /// A `paged::CameraFollow` viewport, offset in both directions.
    pub camera: bool,
}

impl<Msg> Node<Msg> {
    pub fn new(style: Style) -> Self {
        Node {
            id: 0,
            parent: None,
            children: Vec::new(),
            style,
            draw: Draw::None,
            opacity: 1.0,
            clip: None,
            scroll: None,
            interact: None,
            indicator: None,
            sem: None,
            centered: false,
            slide: None,
            trap: false,
            autofocus: false,
            tooltip: None,
            rtl: false,
            press_scale: false,
            moves: false,
            grid: None,
            icon: None,
            name: None,
            pager: false,
            camera: false,
        }
    }
    pub fn draw(mut self, draw: Draw) -> Self {
        self.draw = draw;
        self
    }
    pub fn sem(mut self, sem: Semantics) -> Self {
        self.sem = Some(sem);
        self
    }
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }
    pub fn indicator(mut self, indicator: Indicator) -> Self {
        self.indicator = Some(indicator);
        self
    }
    pub fn interact(mut self, interact: Interact<Msg>) -> Self {
        self.interact = Some(interact);
        self
    }
    pub fn press_scale(mut self) -> Self {
        self.press_scale = true;
        self
    }
    pub fn moves(mut self) -> Self {
        self.moves = true;
        self
    }
    /// A tooltip shown while the node has focus. Its words join the node's
    /// description, unless they repeat its name.
    pub fn tooltip(mut self, tip: Option<String>) -> Self {
        let tip = tip.filter(|t| !t.trim().is_empty());
        if let (Some(t), Some(sem)) = (&tip, self.sem.as_mut()) {
            if sem.label.as_deref().map(str::trim) != Some(t.trim()) {
                sem.description = Some(match sem.description.take() {
                    Some(d) => format!("{d}, {t}"),
                    None => t.clone(),
                });
            }
        }
        self.tooltip = tip;
        self
    }
}

/// The text style and color `inherit` resolves to inside a component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Inherited {
    pub style: TextStyle,
    pub color: Color,
}

impl Inherited {
    pub fn root(tokens: &Tokens) -> Self {
        Inherited {
            style: crate::components::text::type_style(tokens.font.body, crate::FontFamily::Body),
            color: tokens.color.text_primary.into(),
        }
    }
}

/// Builds a popup once the view is built.
pub(crate) type Overlay<Msg> = Box<dyn FnOnce(&mut BuildCx<Msg>)>;

/// Builds elements into the node list.
pub(crate) struct BuildCx<Msg> {
    pub nodes: Vec<Node<Msg>>,
    pub tokens: &'static Tokens,
    /// The theme `tokens` come from, for what it adds beyond tokens.
    pub theme: &'static Theme,
    pub inherit: Vec<Inherited>,
    /// Something animates, so the shell keeps drawing frames.
    pub animating: bool,
    /// The person's preferences: color scheme, contrast, and motion.
    pub prefs: Preferences,
    /// The writing direction of the region being built.
    pub direction: Direction,
    /// The padding of the nearest container that publishes it (Card), for
    /// full-bleed children.
    pub container_padding: Vec<f32>,
    /// The control whose popup is open, and where it was last drawn.
    pub popup: Option<(u64, crate::contract::Rect)>,
    /// The drawable size in points, for placing popups.
    pub viewport: (f32, f32),
    /// Popups to build above everything else once the view is built.
    pub overlays: Vec<Overlay<Msg>>,
    /// The images the app registered: true when ready, false when failed.
    pub images: std::sync::Arc<std::collections::HashMap<crate::contract::ImageId, bool>>,
    open: Vec<u32>,
    key: Option<u64>,
    /// The `id` prop behind `key`, for the node `begin` opens next.
    name: Option<String>,
}

impl<Msg> BuildCx<Msg> {
    pub fn new(tokens: &'static Tokens) -> Self {
        BuildCx {
            nodes: Vec::new(),
            tokens,
            theme: Theme::ASTRYX,
            inherit: vec![Inherited::root(tokens)],
            animating: false,
            prefs: Preferences::default(),
            direction: Direction::Ltr,
            container_padding: Vec::new(),
            popup: None,
            viewport: (0.0, 0.0),
            overlays: Vec::new(),
            images: Default::default(),
            open: Vec::new(),
            key: None,
            name: None,
        }
    }

    /// Whether the image for `src` is ready (`Some(true)`), failed
    /// (`Some(false)`), or not registered yet, so still loading (`None`).
    pub fn image_state(&self, src: &str) -> Option<bool> {
        self.images.get(&crate::contract::ImageId::of(src)).copied()
    }

    pub fn inherited(&self) -> Inherited {
        *self.inherit.last().expect("root inherited style")
    }

    /// Gives the next node an identity from an Astryx base `id`.
    /// Keys the next node by a number, such as its slot in a list.
    pub fn key_slot(&mut self, slot: u64) {
        self.key = Some(slot);
        self.name = None;
    }

    pub fn key(&mut self, id: Option<&str>) {
        self.key = id.map(|id| fnv1a(id.as_bytes()));
        self.name = id.map(String::from);
    }

    /// Applies what the theme adds beyond tokens: its grid, fonts, and icons.
    fn apply_theme(&self, node: &mut Node<Msg>) {
        let theme = self.theme;
        node.grid = theme.grid;
        if let Draw::Text(t) = &mut node.draw {
            t.style = self.themed_text(t.style);
        }
        let Some((name, side)) = node.icon else {
            return;
        };
        let Some(icon) = theme.icon(name) else {
            return;
        };
        let color = match &node.draw {
            Draw::Path(p) => p.fill.or(p.stroke.map(|(c, _)| c)),
            _ => None,
        };
        node.draw = Draw::Bitmap(icon.rows, color.unwrap_or(Color::TRANSPARENT));
        if let Some(cell) = theme.grid {
            // The icon keeps its box. Its pixel is the largest whole number
            // of cells that fits, or a half or quarter cell for small boxes,
            // so it stays on the grid's device pixels.
            let n = icon.rows.len().max(1) as f32;
            let mut pixel = (side / (n * cell)).floor() * cell;
            if pixel == 0.0 {
                pixel = cell;
                while pixel * n > side && pixel > cell / 8.0 {
                    pixel /= 2.0;
                }
            }
            node.icon = Some((name, n * pixel));
        }
    }

    fn themed_text(&self, style: TextStyle) -> TextStyle {
        themed_text(self.theme, self.tokens, style)
    }

    /// Adds a node as a child of the open node and opens it.
    pub fn begin(&mut self, mut node: Node<Msg>) -> u32 {
        self.apply_theme(&mut node);
        let ix = self.nodes.len() as u32;
        let parent = self.open.last().copied();
        let (parent_id, sibling) = match parent {
            Some(p) => (
                self.nodes[p as usize].id,
                self.nodes[p as usize].children.len() as u64,
            ),
            None => (0, 0),
        };
        let local = self.key.take().map_or(sibling, |k| k ^ KEYED);
        node.name = self.name.take();
        node.id = mix(parent_id, local);
        node.parent = parent;
        if self.direction.is_rtl() {
            node.rtl = true;
            node.style.direction = taffy::Direction::Rtl;
        }
        // Text sets its paragraph direction from its region, so a Hebrew
        // label that starts with a Latin word still reads right to left.
        if let Draw::Text(t) = &mut node.draw {
            t.style.direction = Some(if node.rtl {
                TextDirection::Rtl
            } else {
                TextDirection::Ltr
            });
        }
        if let Some(p) = parent {
            self.nodes[p as usize].children.push(ix);
        }
        self.nodes.push(node);
        self.open.push(ix);
        ix
    }

    pub fn end(&mut self) {
        self.open.pop();
    }

    /// Marks that something animates, unless the person asked for reduced
    /// motion. Returns whether it may animate.
    pub fn animate(&mut self) -> bool {
        let on = !self.prefs.reduced_motion;
        self.animating |= on;
        on
    }

    /// A persistent, invisible region whose words assistive technology
    /// announces when they change, as an ARIA live region does.
    pub fn live_region(&mut self, words: Option<&str>, live: Live) {
        let sem = Semantics {
            label: words.map(str::to_string),
            live: Some(live),
            ..Semantics::new(Role::Text, "")
        };
        self.leaf(
            Node::new(Style {
                position: taffy::Position::Absolute,
                ..Style::default()
            })
            .sem(sem),
        );
    }

    pub fn rtl(&self) -> bool {
        self.direction.is_rtl()
    }

    /// The physical (left, right) pair for logical (start, end) values in
    /// the region being built.
    pub fn inline<T>(&self, start: T, end: T) -> (T, T) {
        if self.rtl() {
            (end, start)
        } else {
            (start, end)
        }
    }

    /// The id of the node `begin` returned.
    pub fn id_of(&self, ix: u32) -> u64 {
        self.nodes[ix as usize].id
    }

    /// Adds a node with no children.
    pub fn leaf(&mut self, node: Node<Msg>) -> u32 {
        let ix = self.begin(node);
        self.end();
        ix
    }

    pub fn child(&mut self, element: Element<Msg>) {
        element.build.build(self);
    }
}

/// Keeps keyed identities apart from sibling indexes.
const KEYED: u64 = 0x9e37_79b9_7f4a_7c15;

pub(crate) fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Combines a parent identity with a child's local identity (splitmix64).
pub(crate) fn mix(parent: u64, local: u64) -> u64 {
    let mut z = parent.wrapping_add(local.wrapping_add(1).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// A text style with the theme's font, and on a grid, sizes and line
/// heights in whole cells.
pub(crate) fn themed_text(
    theme: &'static Theme,
    tokens: &Tokens,
    mut style: TextStyle,
) -> TextStyle {
    let stack = match style.family {
        FontFamily::Body => tokens.font.family.body,
        FontFamily::Heading => tokens.font.family.heading,
        FontFamily::Code => tokens.font.family.code,
        FontFamily::Theme(_) => &[],
    };
    let shipped = stack
        .iter()
        .find_map(|name| theme.fonts.iter().find(|f| f.family == *name));
    if let Some(font) = shipped {
        style.family = FontFamily::Theme(font);
    }
    if let (Some(cell), FontFamily::Theme(font)) = (theme.grid, style.family) {
        if font.em_pixels > 0 && style.size > 0.0 {
            let step = cell * f32::from(font.em_pixels);
            let size = (style.size / step).round().max(1.0) * step;
            let line = (style.size * style.leading / cell).round() * cell;
            style.leading = line.max(size) / size;
            style.size = size;
        }
    }
    style
}
