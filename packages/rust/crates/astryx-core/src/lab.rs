//! Experimental components that Astryx does not define yet. Their names,
//! props, and behavior may change, or move into the shared component set
//! once Astryx specifies them.
//!
//! - [`StatusBar`]: a device's Wi-Fi, Bluetooth, battery, and time, from
//!   `Preferences::device`.
//! - [`HoldButton`]: a Button that confirms only after a sustained press.
//! - [`Image`]: a full-width image with an aspect ratio and alt text.
//! - [`SelectableCardGroup`]: a single choice among `SelectableCard`s, as a
//!   radio group of cards.
//! - [`KeyHints`]: a row of gamepad keys and what they do.
//! - [`HoldCard`]: a `SelectableCard` that confirms on a held press, filling
//!   in the accent.
//! - [`OnScreenKeyboard`]: text entry with a gamepad, as a chord ring or a
//!   grid.

pub use crate::components::keyboard::{
    ChordKey, ChordLayer, ChordLayout, GridLayout, KeyboardAction, KeyboardKind, KeyboardState,
    KeyboardUpdate, OnScreenKeyboard, DIRECTIONS, FACE_BUTTONS,
};
use crate::components::text::{text_node, type_style};
use crate::components::{
    dim, len, radii, square, Button, ButtonSize, ButtonVariant, IconSize, SelectableCard,
};
use crate::contract::{
    Color, DeviceStatus, FontFamily, ImageFit, ImageId, PathCmd, Point, Radio, Role,
};
use crate::element::{
    base_props, Base, BoxDraw, Build, BuildCx, Draw, ImageDraw, Node, PathDraw, Semantics, Space,
};
use taffy::prelude::*;

// ---- StatusBar ----

/// A compact row of device status: the time, Wi-Fi, Bluetooth, and the
/// battery. It shows the shell's `Preferences::device`, and nothing when
/// the shell reports none. Assistive technology reads it as one line, such
/// as "Wi-Fi connected, battery 82% charging, 7:42 PM".
pub struct StatusBar {
    base: Base,
    device: Option<DeviceStatus>,
}

impl Default for StatusBar {
    fn default() -> Self {
        StatusBar::new()
    }
}

impl StatusBar {
    pub fn new() -> Self {
        StatusBar {
            base: Base::default(),
            device: None,
        }
    }
    /// Shows `device` instead of the shell's, as for previews.
    pub fn device(mut self, device: DeviceStatus) -> Self {
        self.device = Some(device);
        self
    }
}

base_props!(StatusBar);

/// `h:mm AM` on a 12-hour clock.
fn clock((hours, minutes): (u8, u8)) -> String {
    let h = match hours % 12 {
        0 => 12,
        h => h,
    };
    format!(
        "{h}:{:02} {}",
        minutes.min(59),
        if hours % 24 < 12 { "AM" } else { "PM" }
    )
}

/// The status as one sentence for assistive technology.
fn status_label(d: &DeviceStatus) -> String {
    let mut parts = vec![match d.wifi {
        Radio::Off => "Wi-Fi off".to_string(),
        Radio::On => "Wi-Fi on, not connected".to_string(),
        Radio::Connected => "Wi-Fi connected".to_string(),
    }];
    match d.bluetooth {
        Radio::Off => {}
        Radio::On => parts.push("Bluetooth on".into()),
        Radio::Connected => parts.push("Bluetooth connected".into()),
    }
    if let Some(p) = d.battery_percent {
        parts.push(format!(
            "battery {}%{}",
            p.min(100),
            if d.charging { " charging" } else { "" }
        ));
    }
    parts.push(clock(d.time));
    parts.join(", ")
}

const fn pt(x: f32, y: f32) -> Point {
    Point { x, y }
}

const WIFI: &[PathCmd] = &[
    PathCmd::MoveTo(pt(2.0, 9.0)),
    PathCmd::QuadTo(pt(12.0, 0.5), pt(22.0, 9.0)),
    PathCmd::MoveTo(pt(5.5, 12.5)),
    PathCmd::QuadTo(pt(12.0, 7.0), pt(18.5, 12.5)),
    PathCmd::MoveTo(pt(9.0, 16.0)),
    PathCmd::QuadTo(pt(12.0, 13.5), pt(15.0, 16.0)),
    PathCmd::MoveTo(pt(12.0, 19.5)),
    PathCmd::LineTo(pt(12.01, 19.5)),
];

const SLASH: &[PathCmd] = &[
    PathCmd::MoveTo(pt(3.0, 3.0)),
    PathCmd::LineTo(pt(21.0, 21.0)),
];

const BLUETOOTH: &[PathCmd] = &[
    PathCmd::MoveTo(pt(7.0, 7.0)),
    PathCmd::LineTo(pt(17.0, 17.0)),
    PathCmd::LineTo(pt(12.0, 22.0)),
    PathCmd::LineTo(pt(12.0, 2.0)),
    PathCmd::LineTo(pt(17.0, 7.0)),
    PathCmd::LineTo(pt(7.0, 17.0)),
];

const BOLT: &[PathCmd] = &[
    PathCmd::MoveTo(pt(13.0, 2.0)),
    PathCmd::LineTo(pt(4.0, 14.0)),
    PathCmd::LineTo(pt(11.0, 14.0)),
    PathCmd::LineTo(pt(10.0, 22.0)),
    PathCmd::LineTo(pt(20.0, 9.0)),
    PathCmd::LineTo(pt(13.0, 9.0)),
    PathCmd::Close,
];

/// 12-row bitmaps of the status icons, for pixel themes without their own.
const WIFI_BITS: &[&str] = &[
    "............",
    "..########..",
    ".##......##.",
    "#..######..#",
    "..##....##..",
    ".#..####..#.",
    "...##..##...",
    "....#..#....",
    "............",
    ".....##.....",
    ".....##.....",
    "............",
];
const WIFI_OFF_BITS: &[&str] = &[
    "#...........",
    ".##.#####...",
    ".###.....##.",
    "#..##.##...#",
    "..##.##.##..",
    ".#..#.##..#.",
    "...##..##...",
    "....#...##..",
    ".........##.",
    ".....##...##",
    ".....##....#",
    "............",
];
const BLUETOOTH_BITS: &[&str] = &[
    ".....#......",
    ".....##.....",
    ".....#.#....",
    "..#..#..#...",
    "...#.#.#....",
    "....###.....",
    "....###.....",
    "...#.#.#....",
    "..#..#..#...",
    ".....#.#....",
    ".....##.....",
    ".....#......",
];
const BOLT_BITS: &[&str] = &[
    "......##....",
    ".....##.....",
    "....###.....",
    "...###......",
    "..########..",
    "......###...",
    ".....###....",
    "....###.....",
    "....##......",
    "...##.......",
    "...#........",
    "............",
];

/// A status icon. Its name lets a theme with its own icon of that name
/// draw it instead; a pixel theme without one gets the built-in bitmap.
pub(crate) fn named_icon<Msg>(
    cx: &BuildCx<Msg>,
    (name, bits): (&'static str, &'static [&'static str]),
    paths: Vec<PathCmd>,
    side: f32,
    color: Color,
    filled: bool,
) -> Node<Msg> {
    if let (Some(cell), None) = (cx.theme.grid, cx.theme.icon(name)) {
        // Each icon pixel is a whole number of cells, as theme icons are.
        let n = bits.len() as f32;
        let pixel = ((side / (n * cell)).floor() * cell).max(cell / 2.0);
        let mut node = Node::new(Style {
            size: square(n * pixel),
            flex_shrink: 0.0,
            ..Style::default()
        })
        .draw(Draw::Bitmap(bits, color));
        node.icon = Some((name, n * pixel));
        return node;
    }
    let mut node = Node::new(Style {
        size: square(side),
        flex_shrink: 0.0,
        ..Style::default()
    })
    .draw(Draw::Path(PathDraw {
        commands: paths,
        view_box: 24.0,
        fill: filled.then_some(color),
        stroke: (!filled).then_some((color, 2.0)),
    }));
    node.icon = Some((name, side));
    node
}

impl<Msg> Build<Msg> for StatusBar {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let Some(device) = self.device.or_else(|| cx.prefs.device.clone()) else {
            return;
        };
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let style = type_style(t.font.label, FontFamily::Body);
        let side = IconSize::Sm.points();
        let sem = Semantics::new(Role::Text, status_label(&device));
        cx.key(self.base.id.as_deref());
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                gap: Size {
                    width: len(t.space.s2),
                    height: len(0.0),
                },
                padding: taffy::Rect {
                    left: len(t.space.s3),
                    right: len(t.space.s3),
                    top: len(t.space.s1),
                    bottom: len(t.space.s1),
                },
                ..Style::default()
            })
            .sem(sem),
        );
        cx.leaf(text_node(clock(device.time), style, c.text_primary.into()));
        cx.leaf(Node::new(Style {
            flex_grow: 1.0,
            ..Style::default()
        }));
        let (wifi, wifi_color) = match device.wifi {
            Radio::Off => ("wifiOff", c.icon_secondary),
            Radio::On => ("wifi", c.icon_secondary),
            Radio::Connected => ("wifi", c.icon_primary),
        };
        let mut paths = WIFI.to_vec();
        if device.wifi == Radio::Off {
            paths.extend_from_slice(SLASH);
        }
        let bits = if device.wifi == Radio::Off {
            WIFI_OFF_BITS
        } else {
            WIFI_BITS
        };
        let icon = named_icon(cx, (wifi, bits), paths, side, wifi_color.into(), false);
        cx.leaf(icon);
        if device.bluetooth != Radio::Off {
            let color = if device.bluetooth == Radio::Connected {
                c.icon_primary
            } else {
                c.icon_secondary
            };
            let icon = named_icon(
                cx,
                ("bluetooth", BLUETOOTH_BITS),
                BLUETOOTH.to_vec(),
                side,
                color.into(),
                false,
            );
            cx.leaf(icon);
        }
        if let Some(percent) = device.battery_percent {
            battery(cx, percent.min(100), device.charging, style);
        }
        cx.end();
    }
}

/// The battery: its percent, a gauge filled to it, and a bolt while
/// charging. The gauge turns to the error color at 20% or less unless
/// charging, and to the success color while charging.
fn battery<Msg>(
    cx: &mut BuildCx<Msg>,
    percent: u8,
    charging: bool,
    style: crate::contract::TextStyle,
) {
    let t = cx.tokens;
    let c = &t.color;
    cx.leaf(text_node(
        format!("{percent}%"),
        style,
        c.text_primary.into(),
    ));
    let fill: Color = if charging {
        c.icon_success
    } else if percent <= 20 {
        c.icon_error
    } else {
        c.icon_primary
    }
    .into();
    // Whole theme pixels under a pixel grid.
    let snap = |v: f32| match cx.theme.grid {
        Some(cell) => (v / cell).round().max(1.0) * cell,
        None => v,
    };
    let (w, h, line, gap) = (snap(22.0), snap(11.0), snap(1.5), snap(1.0));
    let inner = w - 2.0 * (line + gap);
    let level = match cx.theme.grid {
        Some(cell) => (inner * f32::from(percent) / 100.0 / cell).round() * cell,
        None => inner * f32::from(percent) / 100.0,
    };
    cx.begin(Node::new(Style {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::CENTER,
        flex_shrink: 0.0,
        ..Style::default()
    }));
    cx.begin(
        Node::new(Style {
            size: Size {
                width: dim(w),
                height: dim(h),
            },
            padding: taffy::Rect {
                left: len(line + gap),
                right: len(line + gap),
                top: len(line + gap),
                bottom: len(line + gap),
            },
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw {
            fill: Color::TRANSPARENT,
            radii: radii(snap(2.0) + line),
            border: Some((c.icon_primary.into(), line)),
            shadow: &[],
        })),
    );
    cx.leaf(
        Node::new(Style {
            size: Size {
                width: dim(level),
                height: Dimension::percent(1.0),
            },
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw::fill(fill, radii(snap(1.0))))),
    );
    cx.end();
    // The terminal nub.
    cx.leaf(
        Node::new(Style {
            size: Size {
                width: dim(line),
                height: dim(snap(h / 2.0)),
            },
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw::fill(c.icon_primary.into(), radii(0.0)))),
    );
    if charging {
        let icon = named_icon(cx, ("bolt", BOLT_BITS), BOLT.to_vec(), 12.0, fill, true);
        cx.leaf(icon);
    }
    cx.end();
}

// ---- HoldButton ----

/// A Button that confirms only after it is held: A held down (or a press
/// held on a pointer, once shells report pointer presses) fills it from
/// its inline start over `duration_ms`, 1 second unless set, and sends
/// `on_confirm` when full. Letting go early drains the fill and sends
/// nothing. Under reduced motion the fill moves in quarter steps.
/// Assistive technology hears "Hold to confirm"; its activate action, for
/// people who cannot hold, confirms at once.
pub struct HoldButton<Msg> {
    base: Base,
    label: String,
    duration_ms: f32,
    variant: ButtonVariant,
    size: ButtonSize,
    is_disabled: bool,
    on_confirm: Option<Msg>,
}

impl<Msg> HoldButton<Msg> {
    pub fn new(label: impl Into<String>) -> Self {
        HoldButton {
            base: Base::default(),
            label: label.into(),
            duration_ms: 1000.0,
            variant: ButtonVariant::Primary,
            size: ButtonSize::Md,
            is_disabled: false,
            on_confirm: None,
        }
    }
    /// How long the press is held, in milliseconds.
    pub fn duration_ms(mut self, ms: u32) -> Self {
        self.duration_ms = ms.max(1) as f32;
        self
    }
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    /// Sent once the hold completes.
    pub fn on_confirm(mut self, msg: Msg) -> Self {
        self.on_confirm = Some(msg);
        self
    }
}

base_props!(HoldButton<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for HoldButton<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let mut button = Button::new(self.label)
            .variant(self.variant)
            .size(self.size)
            .is_disabled(self.is_disabled)
            .is_hidden(self.base.is_hidden);
        if let Some(id) = self.base.id {
            button = button.id(id);
        }
        if let Some(msg) = self.on_confirm {
            button = button.on_click(msg);
        }
        button.hold = Some(self.duration_ms);
        cx.child(button.into());
    }
}

// ---- Image ----

/// An image as wide as its container, at an aspect ratio: the image's own
/// unless set, or 16:9 while it loads. `alt` describes it for assistive
/// technology; pass an empty string for a decorative image. While it
/// loads, or if it failed, the box shows a quiet placeholder.
pub struct Image {
    base: Base,
    image: ImageId,
    alt: String,
    fit: ImageFit,
    aspect_ratio: Option<f32>,
    pixel_perfect: bool,
}

impl Image {
    pub fn new(image: ImageId, alt: impl Into<String>) -> Self {
        Image {
            base: Base::default(),
            image,
            alt: alt.into(),
            fit: ImageFit::Cover,
            aspect_ratio: None,
            pixel_perfect: false,
        }
    }
    /// How the image fills its box: `Cover` (the default) crops, `Contain`
    /// shows all of it.
    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }
    /// Width over height, such as `16.0 / 9.0`.
    pub fn aspect_ratio(mut self, ratio: f32) -> Self {
        self.aspect_ratio = Some(ratio.max(0.01));
        self
    }
    /// Draws the image's pixels square, at the largest whole multiple of
    /// their size that fits (in a pixel theme's cells, else device
    /// pixels), for pixel art.
    pub fn pixel_perfect(mut self, on: bool) -> Self {
        self.pixel_perfect = on;
        self
    }
}

base_props!(Image);

impl<Msg> Build<Msg> for Image {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let size = cx.image_sizes.get(&self.image).copied();
        let ready = cx.images.get(&self.image).copied() == Some(true);
        let ratio = self
            .aspect_ratio
            .or(size.map(|(w, h)| w as f32 / h.max(1) as f32))
            .unwrap_or(16.0 / 9.0);
        let mut node = Node::new(Style {
            size: Size {
                width: Dimension::percent(1.0),
                height: Dimension::auto(),
            },
            aspect_ratio: Some(ratio),
            flex_shrink: 0.0,
            ..Style::default()
        });
        let corner = radii(t.radius.inner);
        node = if ready {
            node.draw(Draw::Image(ImageDraw {
                image: self.image,
                fit: self.fit,
                radii: corner,
                pixel: if self.pixel_perfect { size } else { None },
            }))
        } else {
            node.draw(Draw::Box(BoxDraw::fill(
                t.color.background_muted.into(),
                corner,
            )))
        };
        if !self.alt.trim().is_empty() {
            node = node.sem(Semantics::new(Role::Image, self.alt));
        }
        cx.key(self.base.id.as_deref());
        cx.leaf(node);
    }
}

// ---- SelectableCardGroup ----

/// One choice among cards, as a radio group: choosing a card selects it
/// and never deselects it. Next and Previous reach the group in one step,
/// on its chosen card (or its first when none is chosen); the arrows move
/// between cards by geometry, and A chooses the focused one.
pub struct SelectableCardGroup<Msg> {
    base: Base,
    label: String,
    value: Option<String>,
    on_change: Option<std::rc::Rc<dyn Fn(String) -> Msg>>,
    is_horizontal: bool,
    gap: Space,
    options: Vec<(String, SelectableCard<Msg>)>,
    hold_ms: Option<f32>,
}

impl<Msg> SelectableCardGroup<Msg> {
    /// A group named `label` for assistive technology.
    pub fn new(label: impl Into<String>) -> Self {
        SelectableCardGroup {
            base: Base::default(),
            label: label.into(),
            value: None,
            on_change: None,
            is_horizontal: false,
            gap: Space::S3,
            options: Vec::new(),
            hold_ms: None,
        }
    }
    /// The chosen card's value.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }
    /// Called with a card's value when it is chosen.
    pub fn on_change(mut self, on_change: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_change = Some(std::rc::Rc::new(on_change));
        self
    }
    /// Lays the cards out in a row that wraps, instead of a column.
    pub fn is_horizontal(mut self, on: bool) -> Self {
        self.is_horizontal = on;
        self
    }
    pub fn gap(mut self, gap: Space) -> Self {
        self.gap = gap;
        self
    }
    /// A card the group chooses as `value`. Its own selection is replaced
    /// by the group's.
    pub fn option(mut self, value: impl Into<String>, card: SelectableCard<Msg>) -> Self {
        self.options.push((value.into(), card));
        self
    }
    /// Choosing a card takes a press held this long, as on a [`HoldCard`].
    pub fn hold_ms(mut self, ms: u32) -> Self {
        self.hold_ms = Some(ms as f32);
        self
    }
}

// ---- HoldCard ----

/// A `SelectableCard` that changes only on a held press (`HoldCard`): the
/// card fills in the accent from inline start over the hold's time (in
/// quarter steps under reduced motion), letting go early empties it and
/// sends nothing, and a completed hold sends the card's change once. It
/// says "Hold to confirm"; assistive technology activates it at once.
/// Feedback cues: Progress at each quarter, Success at the end.
pub struct HoldCard<Msg> {
    card: SelectableCard<Msg>,
    duration_ms: f32,
}

impl<Msg> HoldCard<Msg> {
    /// `card` held for 1 second.
    pub fn new(card: SelectableCard<Msg>) -> Self {
        HoldCard {
            card,
            duration_ms: 1000.0,
        }
    }
    pub fn duration_ms(mut self, ms: u32) -> Self {
        self.duration_ms = ms as f32;
        self
    }
}

impl<Msg: Clone + 'static> From<HoldCard<Msg>> for crate::element::Element<Msg> {
    fn from(h: HoldCard<Msg>) -> Self {
        let mut card = h.card;
        card.hold = Some(h.duration_ms);
        card.into()
    }
}

base_props!(SelectableCardGroup<Msg>);

impl<Msg: Clone + 'static> Build<Msg> for SelectableCardGroup<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let gap = len(self.gap.points(cx.tokens));
        cx.key(self.base.id.as_deref());
        let ix = cx.begin(
            Node::new(Style {
                flex_direction: if self.is_horizontal {
                    FlexDirection::Row
                } else {
                    FlexDirection::Column
                },
                flex_wrap: if self.is_horizontal {
                    FlexWrap::Wrap
                } else {
                    FlexWrap::NoWrap
                },
                gap: Size {
                    width: gap,
                    height: gap,
                },
                ..Style::default()
            })
            .sem(Semantics::new(Role::RadioGroup, self.label)),
        );
        let group = cx.id_of(ix);
        for (value, mut card) in self.options {
            card.is_selected = self.value.as_deref() == Some(value.as_str());
            card.choice = self.on_change.as_ref().map(|f| f(value.clone()));
            card.hold = card.hold.or(self.hold_ms);
            let first = cx.nodes.len();
            cx.child(card.into());
            if let Some(node) = cx.nodes.get_mut(first) {
                node.group = Some(group);
            }
        }
        cx.end();
    }
}

// ---- KeyHints ----

/// A row of gamepad keys and what each does, such as "[A] Open  [B] Back",
/// with the same spacing on every screen. Keys take `Kbd::gamepad`'s
/// names; "L1 R1" shows either-or keys.
pub struct KeyHints {
    base: Base,
    hints: Vec<(String, String)>,
}

impl Default for KeyHints {
    fn default() -> Self {
        KeyHints::new()
    }
}

impl KeyHints {
    pub fn new() -> Self {
        KeyHints {
            base: Base::default(),
            hints: Vec::new(),
        }
    }
    /// A key (or keys) and what it does.
    pub fn key(mut self, keys: impl Into<String>, label: impl Into<String>) -> Self {
        self.hints.push((keys.into(), label.into()));
        self
    }
}

base_props!(KeyHints);

impl<Msg> Build<Msg> for KeyHints {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let style = type_style(t.font.supporting, FontFamily::Body);
        let row = |gap: f32| Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            gap: Size {
                width: len(gap),
                height: len(0.0),
            },
            flex_shrink: 0.0,
            ..Style::default()
        };
        cx.key(self.base.id.as_deref());
        cx.begin(Node::new(Style {
            flex_wrap: FlexWrap::Wrap,
            ..row(t.space.s4)
        }));
        for (keys, label) in self.hints {
            cx.begin(Node::new(row(t.space.s1_5)));
            Build::build(Box::new(crate::components::Kbd::gamepad(&keys)), cx);
            cx.leaf(text_node(label, style, t.color.text_primary.into()));
            cx.end();
        }
        cx.end();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_labels_read_as_one_sentence() {
        let d = DeviceStatus {
            wifi: Radio::Connected,
            battery_percent: Some(82),
            charging: true,
            time: (19, 42),
            ..DeviceStatus::default()
        };
        assert_eq!(
            status_label(&d),
            "Wi-Fi connected, battery 82% charging, 7:42 PM"
        );
        let d = DeviceStatus {
            bluetooth: Radio::On,
            battery_percent: None,
            time: (0, 5),
            ..DeviceStatus::default()
        };
        assert_eq!(status_label(&d), "Wi-Fi off, Bluetooth on, 12:05 AM");
        assert_eq!(clock((12, 0)), "12:00 PM");
    }
}
