//! The shared contract of Astryx Rust.
//!
//! Every other crate depends on these types: components produce a
//! [`DisplayList`], renderers draw it, shells feed [`Input`] and [`FrameInfo`]
//! in and hand a [`TextMeasure`] to layout. The orchestrator owns this file.
//! Lanes add their own modules next to it and re-export them from `lib.rs`.

/// A point or size in logical points. One point is one token unit.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

/// An axis-aligned rectangle in logical points, origin at the top left.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }
    pub fn center(&self) -> Point {
        Point { x: self.x + self.width / 2.0, y: self.y + self.height / 2.0 }
    }
    pub fn inset(&self, by: f32) -> Self {
        Self::new(self.x + by, self.y + by, (self.width - 2.0 * by).max(0.0), (self.height - 2.0 * by).max(0.0))
    }
}

/// Straight (not premultiplied) sRGB color, each channel 0..=1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }
}

/// Corner radii in points: top-left, top-right, bottom-right, bottom-left.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Radii(pub [f32; 4]);

impl Radii {
    pub const fn all(r: f32) -> Self {
        Self([r; 4])
    }
}

/// One layer of a box shadow, as CSS `box-shadow` draws it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowLayer {
    pub x: f32,
    pub y: f32,
    pub blur: f32,
    pub spread: f32,
    pub inset: bool,
    pub color: Color,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontFamily {
    Body,
    Heading,
    Code,
}

/// Everything a renderer and a text measurer need to set one run of text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    pub family: FontFamily,
    /// Font size in points.
    pub size: f32,
    /// CSS weight, 100..=900.
    pub weight: u16,
    /// Line height as a multiple of `size`.
    pub leading: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlign {
    #[default]
    Start,
    Center,
    End,
}

/// One drawing step. Coordinates are logical points; the renderer applies
/// the frame scale.
#[derive(Clone, Debug, PartialEq)]
pub enum DrawCmd {
    /// A filled and optionally stroked rounded rectangle.
    Rect { rect: Rect, radii: Radii, fill: Color, stroke: Option<(Color, f32)> },
    /// Shadow layers for a rounded rectangle. Emitted before the rectangle.
    Shadow { rect: Rect, radii: Radii, layers: Vec<ShadowLayer> },
    /// Text laid out inside `rect`: wraps at `rect.width`, first baseline
    /// placed from the top of `rect` by the style's line height.
    Text { rect: Rect, text: String, style: TextStyle, color: Color, align: TextAlign },
    /// A stroked line, for dividers and icons.
    Line { from: Point, to: Point, color: Color, width: f32 },
    /// A vector path in points, for icons. `fill` or `stroke` may be absent.
    Path { commands: Vec<PathCmd>, fill: Option<Color>, stroke: Option<(Color, f32)> },
    /// Clip all following commands to `rect` until the matching `PopClip`.
    PushClip { rect: Rect, radii: Radii },
    PopClip,
    /// Multiply the alpha of all following commands until `PopOpacity`.
    PushOpacity(f32),
    PopOpacity,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCmd {
    MoveTo(Point),
    LineTo(Point),
    QuadTo(Point, Point),
    CubicTo(Point, Point, Point),
    Close,
}

/// An ordered list of draw commands for one frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DisplayList {
    /// The color to clear the frame to before drawing.
    pub clear: Color,
    pub commands: Vec<DrawCmd>,
}

/// Measures text the same way the renderer will draw it. Each renderer
/// supplies one, so layout and pixels agree.
pub trait TextMeasure {
    /// The size of `text` wrapped at `max_width` (no wrapping when `None`).
    fn measure(&self, text: &str, style: &TextStyle, max_width: Option<f32>) -> Size;
}

/// Draws display lists. GPU renderers draw into the shell's current GL
/// context; software renderers draw into their own pixel buffer.
pub trait Renderer {
    fn measurer(&self) -> &dyn TextMeasure;
    fn render(&mut self, list: &DisplayList, frame: &FrameInfo);
}

/// One logical input. Shells map keys, gamepads, and device buttons to these.
/// A confirms, B goes back, as on a gamepad.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Input {
    Up,
    Down,
    Left,
    Right,
    /// Move focus forward / backward in reading order (Tab, Shift+Tab).
    Next,
    Previous,
    /// Confirm / activate the focused element (Enter, Space, A).
    A,
    /// Back / cancel (Escape, B).
    B,
    X,
    Y,
    L1,
    R1,
    L2,
    R2,
    Start,
    Select,
    Menu,
}

/// Whether a button went down or up. Key repeats arrive as extra `Pressed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputPhase {
    Pressed,
    Released,
}

/// What a shell tells the runtime about one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameInfo {
    /// Drawable size in physical pixels.
    pub width: u32,
    pub height: u32,
    /// Physical pixels per logical point.
    pub scale: f32,
    /// Seconds since the previous frame.
    pub dt: f32,
}

impl FrameInfo {
    /// The drawable size in logical points.
    pub fn logical_size(&self) -> Size {
        Size { width: self.width as f32 / self.scale, height: self.height as f32 / self.scale }
    }
}

/// What an app asks its shell to do after an update.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Command {
    #[default]
    None,
    Quit,
}

/// The accessibility role of a node, named after ARIA roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Group,
    Text,
    Heading(u8),
    Button,
    Switch,
    Checkbox,
    ProgressBar,
    TextInput,
    List,
    ListItem,
    Image,
}

/// One node of the accessibility tree a frame produces. Shells bridge it to
/// the platform (AccessKit on desktop, a DOM mirror on the web).
#[derive(Clone, Debug, PartialEq)]
pub struct A11yNode {
    /// Stable across frames for the same element.
    pub id: u64,
    pub role: Role,
    pub label: Option<String>,
    pub value: Option<String>,
    pub bounds: Rect,
    pub focused: bool,
    pub disabled: bool,
    /// `Some(true|false)` for toggles.
    pub checked: Option<bool>,
    pub children: Vec<A11yNode>,
}

/// What a shell drives, once per frame. The core's runtime implements it for
/// any app; shells and renderers never see app or element types. Test shells
/// can implement it directly with a fixed display list.
pub trait Surface {
    /// Feed one input. Returns what the app asked for.
    fn input(&mut self, input: Input, phase: InputPhase) -> Command;
    /// Lay out and paint one frame.
    fn frame(&mut self, frame: &FrameInfo, text: &dyn TextMeasure) -> DisplayList;
    /// The accessibility tree of the last frame.
    fn accessibility(&self) -> Option<A11yNode>;
    /// True while something animates, so the shell keeps drawing frames.
    fn wants_frames(&self) -> bool;
}
