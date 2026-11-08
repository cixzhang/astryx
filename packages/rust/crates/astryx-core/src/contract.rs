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
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub fn center(&self) -> Point {
        Point {
            x: self.x + self.width / 2.0,
            y: self.y + self.height / 2.0,
        }
    }
    pub fn inset(&self, by: f32) -> Self {
        Self::new(
            self.x + by,
            self.y + by,
            (self.width - 2.0 * by).max(0.0),
            (self.height - 2.0 * by).max(0.0),
        )
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
    pub const TRANSPARENT: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
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

/// An image the app registers with the runtime. The app picks the number and
/// keeps it stable; [`ImageId::of`] derives one from a source string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ImageId(pub u64);

impl ImageId {
    /// A stable id for a source string such as a component's `src`
    /// (64-bit FNV-1a), so the same string always names the same image.
    pub const fn of(src: &str) -> ImageId {
        let bytes = src.as_bytes();
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut i = 0;
        while i < bytes.len() {
            hash ^= bytes[i] as u64;
            hash = hash.wrapping_mul(0x0100_0000_01b3);
            i += 1;
        }
        ImageId(hash)
    }
}

/// How an image fills its box, as CSS `object-fit` does, centered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ImageFit {
    /// Covers the box and crops the overflow.
    #[default]
    Cover,
    /// Fits inside the box, leaving the rest empty.
    Contain,
    /// Stretches to the box.
    Fill,
}

impl ImageFit {
    /// Where an image of `width` x `height` pixels lands for this fit in
    /// `rect`. Renderers draw it there, clipped to `rect` and its radii.
    pub fn place(self, rect: Rect, width: u32, height: u32) -> Rect {
        let (iw, ih) = (width.max(1) as f32, height.max(1) as f32);
        let scale = match self {
            ImageFit::Fill => return rect,
            ImageFit::Cover => (rect.width / iw).max(rect.height / ih),
            ImageFit::Contain => (rect.width / iw).min(rect.height / ih),
        };
        let (w, h) = (iw * scale, ih * scale);
        Rect::new(
            rect.x + (rect.width - w) / 2.0,
            rect.y + (rect.height - h) / 2.0,
            w,
            h,
        )
    }
}

/// Decoded pixels: straight-alpha sRGB RGBA8, row by row, top row first.
#[derive(Clone, PartialEq, Eq)]
pub struct ImageData {
    pub width: u32,
    pub height: u32,
    pub rgba: std::sync::Arc<[u8]>,
}

impl std::fmt::Debug for ImageData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ImageData({}x{})", self.width, self.height)
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

pub use astryx_tokens::ThemeFont;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontFamily {
    Body,
    Heading,
    Code,
    /// A family the theme ships. The text engine loads its files the first
    /// time it sees them, tries it first, and falls back to its other
    /// fonts for characters it lacks.
    Theme(&'static ThemeFont),
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
    /// The paragraph direction. `None` takes it from the first strong
    /// letter, as the Unicode bidirectional algorithm does by default.
    pub direction: Option<TextDirection>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextDirection {
    Ltr,
    Rtl,
}

/// Physical alignment: `Start` is left and `End` is right in every
/// direction. The runtime mirrors alignment for right-to-left regions before
/// it paints, so renderers never flip it again.
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
    /// A filled and optionally stroked rounded rectangle. The stroke is
    /// drawn inside the rectangle, like a CSS border.
    Rect {
        rect: Rect,
        radii: Radii,
        fill: Color,
        stroke: Option<(Color, f32)>,
    },
    /// Shadow layers for a rounded rectangle. Emitted before the rectangle.
    Shadow {
        rect: Rect,
        radii: Radii,
        layers: Vec<ShadowLayer>,
    },
    /// Text laid out inside `rect`, drawn the CSS way:
    /// - It wraps at `rect.width`. A word longer than the box overflows; it
    ///   does not break.
    /// - Each line is `size * leading` tall. The space between the line
    ///   height and the font height is split evenly above and below.
    /// - Empty text measures 0x0.
    Text {
        rect: Rect,
        text: String,
        style: TextStyle,
        color: Color,
        align: TextAlign,
    },
    /// A stroked line with butt caps, for dividers.
    Line {
        from: Point,
        to: Point,
        color: Color,
        width: f32,
    },
    /// A vector path in points, for icons. `fill` or `stroke` may be absent.
    /// Strokes use round caps and round joins.
    Path {
        commands: Vec<PathCmd>,
        fill: Option<Color>,
        stroke: Option<(Color, f32)>,
    },
    /// A registered image drawn into `rect` with `fit`, clipped to `rect` and
    /// `radii`. An image the renderer does not have draws nothing.
    Image {
        rect: Rect,
        radii: Radii,
        image: ImageId,
        fit: ImageFit,
    },
    /// Clip all following commands to `rect` until the matching `PopClip`.
    PushClip {
        rect: Rect,
        radii: Radii,
    },
    PopClip,
    /// Multiply the alpha of all following commands until `PopOpacity`.
    PushOpacity(f32),
    PopOpacity,
    /// Scale following commands about `origin`, then translate them by `translate`,
    /// until `PopTransform`. Used for press feedback and motion.
    PushTransform {
        origin: Point,
        scale: f32,
        translate: Point,
    },
    PopTransform,
    /// Draw the following commands on a pixel grid of `cell` points until
    /// `PopPixelGrid`: nothing is antialiased, and text origins and
    /// baselines snap to whole cells. Pixel themes wrap their content in it.
    PushPixelGrid {
        cell: f32,
    },
    PopPixelGrid,
    /// Light the commands up to `PopLighting`, as a scene at dusk, at night
    /// or under a canopy. They draw as usual; then a light map at a quarter
    /// of the target's resolution is multiplied over the whole target. The
    /// map starts at `ambient` mixed with white by `strength`, may be
    /// darkened by a `dapple`, and each `Light` inside adds its color.
    /// `Emissive` rects inside draw after the map and are not darkened.
    /// Strength 0 skips the map. Commands after `PopLighting`, such as the
    /// UI, are not lit. Lighting does not nest.
    PushLighting(Lighting),
    /// A light of the enclosing lighting pass. Open transforms move and
    /// scale its center and radii like any shape.
    Light(Light),
    /// A rect drawn after the light map of the enclosing lighting pass, not
    /// darkened: a lit window, a glow. Open transforms apply to it.
    Emissive {
        rect: Rect,
        color: Color,
    },
    PopLighting,
}

/// See [`DrawCmd::PushLighting`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lighting {
    /// The color unlit areas are multiplied by at full strength. Alpha is
    /// ignored.
    pub ambient: Color,
    /// 0 leaves the scene as it is, 1 multiplies it by `ambient`.
    pub strength: f32,
    /// Seconds, for flicker, pulse and dapple motion. Hold it still to
    /// pause them, for example under reduced motion.
    pub time: f32,
    pub dapple: Option<Dapple>,
}

/// Leaf shade: a tileable noise pattern scrolled over the light map and
/// multiplied in, in a few flat bands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dapple {
    /// Points per noise cell: the size of a patch of light or shade.
    pub cell: f32,
    /// Points per second the pattern moves.
    pub velocity: Point,
    /// Flat levels, 2 or 3.
    pub bands: u8,
    /// How much the darkest band darkens, 0 to 1, scaled by the strength.
    pub depth: f32,
}

/// A light: an ellipse whose color falls off from its center as
/// `(1 - d)^2`, in four flat bands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    pub center: Point,
    pub radius_x: f32,
    pub radius_y: f32,
    /// Alpha is ignored.
    pub color: Color,
    /// 0 to 1.5.
    pub intensity: f32,
    pub flicker: Flicker,
    pub pulse: Pulse,
}

/// Fast random-looking change in intensity, as a flame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Flicker {
    /// The largest change, as a fraction of the intensity (0 to 0.15).
    pub amplitude: f32,
    pub hz: f32,
}

/// Slow breathing of intensity. A period of 0 is steady.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pulse {
    /// Seconds per breath.
    pub period: f32,
    /// The lowest intensity, as a fraction of the full one.
    pub min: f32,
}

impl Lighting {
    /// The standard lighting of a time of day at `time` seconds: none by
    /// day, a warm multiply at dusk, dark blue at night, when lights show.
    pub fn for_phase(phase: DayPhase, time: f32) -> Option<Lighting> {
        let (ambient, strength) = match phase {
            DayPhase::Day => return None,
            DayPhase::Dusk => (
                Color::rgba(
                    0xf6 as f32 / 255.0,
                    0xb3 as f32 / 255.0,
                    0x8a as f32 / 255.0,
                    1.0,
                ),
                0.35,
            ),
            DayPhase::Night => (
                Color::rgba(
                    0x12 as f32 / 255.0,
                    0x18 as f32 / 255.0,
                    0x38 as f32 / 255.0,
                    1.0,
                ),
                0.84,
            ),
        };
        Some(Lighting {
            ambient,
            strength,
            time,
            dapple: None,
        })
    }
}

/// One stop of the day's light: from `minute`, tint toward `rgb` at
/// `strength`. Colors are Terrace palette swatches.
const DAY: [(f32, u32, f32); 12] = [
    (4.0 * 60.0 + 45.0, 0x30569b, 0.70),  // night: blue-3
    (5.0 * 60.0 + 45.0, 0xe08e79, 0.35),  // dawn: rose-3
    (6.0 * 60.0 + 30.0, 0xfdefbd, 0.18),  // morning: straw-1
    (7.0 * 60.0, 0xfdefbd, 0.0),          // day
    (16.0 * 60.0 + 30.0, 0xeac086, 0.0),  // day
    (17.0 * 60.0, 0xeac086, 0.22),        // golden hour: straw-3
    (17.0 * 60.0 + 30.0, 0xeac086, 0.30), // golden hour
    (18.0 * 60.0 + 15.0, 0xe08e79, 0.38), // sunset: rose-3
    (19.0 * 60.0, 0x542437, 0.50),        // sunset: ink-plum
    (19.0 * 60.0 + 45.0, 0x30569b, 0.60), // dusk: blue-3
    (20.0 * 60.0 + 30.0, 0x30569b, 0.70), // night
    (24.0 * 60.0 + 4.0 * 60.0 + 45.0, 0x30569b, 0.70),
];

/// Steps between two stops of the day. The light holds still between
/// steps, like the art, and each step is about two 8-bit levels at most.
const DAY_STEPS: f32 = 32.0;

impl Lighting {
    /// The light at `minutes` after local midnight, at `time` seconds:
    /// night, dawn, day, golden hour, sunset and dusk blend into each other
    /// in small eased steps. `None` by day.
    pub fn at(minutes: f32, time: f32) -> Option<Lighting> {
        let m = minutes.rem_euclid(24.0 * 60.0);
        // Before the first stop, the night that began the evening before.
        let m = if m < DAY[0].0 { m + 24.0 * 60.0 } else { m };
        let i = DAY
            .windows(2)
            .position(|w| m < w[1].0)
            .unwrap_or(DAY.len() - 2);
        let ((m0, c0, s0), (m1, c1, s1)) = (DAY[i], DAY[i + 1]);
        let f = ((m - m0) / (m1 - m0)).clamp(0.0, 1.0);
        let f = (f * DAY_STEPS).floor() / DAY_STEPS;
        let f = f * f * (3.0 - 2.0 * f);
        let strength = s0 + (s1 - s0) * f;
        if strength <= 0.0 {
            return None;
        }
        let channel = |c: u32, shift: u32| ((c >> shift) & 255) as f32 / 255.0;
        let mix = |shift| channel(c0, shift) + (channel(c1, shift) - channel(c0, shift)) * f;
        Some(Lighting {
            ambient: Color::rgba(mix(16), mix(8), mix(0), 1.0),
            strength,
            time,
            dapple: None,
        })
    }

    /// How bright lamps and lanterns are at `minutes` after midnight, 0 to
    /// 1: off by day, fading in through dusk and out through dawn.
    pub fn lamp_level(minutes: f32) -> f32 {
        let m = minutes.rem_euclid(24.0 * 60.0);
        let ramp = |from: f32, to: f32| {
            let f = ((m - from) / (to - from)).clamp(0.0, 1.0);
            f * f * (3.0 - 2.0 * f)
        };
        // Up from 17:30 to 19:30, down from 05:30 to 07:00.
        ramp(17.5 * 60.0, 19.5 * 60.0).max(1.0 - ramp(5.5 * 60.0, 7.0 * 60.0))
    }
}

impl Light {
    /// The intensity at `time` seconds, with flicker and pulse applied.
    pub fn intensity_at(&self, time: f32) -> f32 {
        use std::f32::consts::TAU;
        let mut k = 1.0;
        if self.flicker.amplitude > 0.0 && self.flicker.hz > 0.0 {
            // Two unrelated waves, offset by position so neighbors differ.
            let phase = self.center.x * 0.37 + self.center.y * 0.71;
            let t = TAU * self.flicker.hz * time + phase;
            k *= 1.0 + self.flicker.amplitude * (0.6 * t.sin() + 0.4 * (2.7 * t + 1.3).sin());
        }
        if self.pulse.period > 0.0 {
            let wave = 0.5 + 0.5 * (TAU * time / self.pulse.period).cos();
            k *= self.pulse.min + (1.0 - self.pulse.min) * wave;
        }
        (self.intensity * k).max(0.0)
    }
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
    /// Changes whenever earlier measurements may be wrong, for example after
    /// a font face loads. Layout caches compare it before reusing sizes.
    fn generation(&self) -> u64 {
        0
    }
}

/// Draws display lists. GPU renderers draw into the shell's current GL
/// context; software renderers draw into their own pixel buffer.
pub trait Renderer {
    fn measurer(&self) -> &dyn TextMeasure;
    fn render(&mut self, list: &DisplayList, frame: &FrameInfo);
    /// Adds or replaces an image; `None` frees it.
    fn set_image(&mut self, id: ImageId, data: Option<&ImageData>) {
        let _ = (id, data);
    }
    /// Starts a transition from `from`, the frame on screen now, to the
    /// frames rendered next. Renderers without transitions show the new
    /// frame at once.
    fn begin_transition(&mut self, transition: &Transition, from: &DisplayList, frame: &FrameInfo) {
        let _ = (transition, from, frame);
    }
    /// A transition is running: shells keep drawing frames until it ends.
    fn transitioning(&self) -> bool {
        false
    }
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
    ///
    /// Face buttons are named by their printed label, not their place:
    /// on an Xbox-style pad A is at the bottom, B right, X left, and Y top;
    /// on a Nintendo-style pad, such as the TrimUI Brick, A is right, B
    /// bottom, X top, and Y left.
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
    /// Text editing keys. Shells send them from a keyboard; the runtime uses
    /// them only while a text field has focus. In a focused text field,
    /// Left and Right move the caret, and Up, Down, Next, and Previous still
    /// move focus.
    Backspace,
    Delete,
    Home,
    End,
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
        Size {
            width: self.width as f32 / self.scale,
            height: self.height as f32 / self.scale,
        }
    }
}

/// What an app asks its shell to do after an update.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Command {
    #[default]
    None,
    Quit,
    /// Open a link with the platform: the browser on desktop, a new tab or
    /// navigation on the web.
    OpenUrl(String),
    /// Show the change to the next frame as a transition from the frame on
    /// screen now. Shells show the new frame at once under reduced motion,
    /// and still play the sound unless sound is off.
    Transition(Transition),
    /// Play a registered sound once, mixed with any already playing. Silent
    /// when sound is off.
    PlaySound(SoundId),
    /// Several commands, carried out in order.
    Batch(Vec<Command>),
}

impl Command {
    /// This command, then `next`: the one that is not `None` when the other
    /// is, else both in a batch.
    pub fn and(self, next: Command) -> Command {
        match (self, next) {
            (Command::None, c) | (c, Command::None) => c,
            (Command::Batch(mut a), Command::Batch(b)) => {
                a.extend(b);
                Command::Batch(a)
            }
            (Command::Batch(mut a), c) => {
                a.push(c);
                Command::Batch(a)
            }
            (a, b) => Command::Batch(vec![a, b]),
        }
    }
}

/// See [`Command::Transition`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    pub kind: TransitionKind,
    pub duration_ms: u32,
    pub easing: Easing,
    /// A sound to play as it starts.
    pub sound: Option<SoundId>,
}

/// How the new frame replaces the old one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransitionKind {
    /// The old frame fades into the new one.
    Fade,
    /// The new frame uncovers from this edge across the screen.
    Wipe(Edge),
    /// A doorway opens from the center: the new frame shows inside a
    /// rectangle inset from half the screen to nothing.
    Door,
    /// A circle opening from this point, in points, shows the new frame.
    Iris(Point),
    /// The new frame slides in from this offset, in points, and fades in.
    Slide(Point),
    /// The new frame appears in square cells of this many points, in a
    /// fixed random order.
    PixelDissolve(f32),
    /// The old frame twists about the center as it fades into the new one.
    Swirl,
    /// Two white flashes over the old frame in the first half, then bars
    /// close from the top and bottom, then the new frame.
    Encounter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

/// Progress over time, as CSS timing functions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    /// Jumps in this many equal steps, as CSS `steps(n)`.
    Steps(u8),
    /// Passes the end once and settles back.
    Overshoot,
}

impl Easing {
    /// Progress at time `t`, both from 0 to 1.
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::EaseIn => t * t * t,
            Easing::EaseOut => 1.0 - (1.0 - t).powi(3),
            Easing::Steps(n) => {
                let n = f32::from(n.max(1));
                ((t * n).floor() / n).min(1.0)
            }
            Easing::Overshoot => {
                // easeOutBack: about 10% past the end at 70% of the time.
                let (c1, c3) = (1.70158, 2.70158);
                let u = t - 1.0;
                1.0 + c3 * u * u * u + c1 * u * u
            }
        }
    }
}

/// A sound an app registers with [`Runtime::register_sound`](crate::Runtime::register_sound).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SoundId(pub u32);

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
    Separator,
    /// A control that opens a list of options, such as a select.
    ComboBox,
    /// The open list of a `ComboBox`.
    ListBox,
    /// One choice in a `ListBox`. `selected` tells whether it is chosen.
    ListBoxOption,
    /// A set of choices of which one is chosen at a time.
    RadioGroup,
    /// One choice in a `RadioGroup`. `checked` tells whether it is chosen.
    Radio,
}

/// One node of the accessibility tree a frame produces. Shells bridge it to
/// the platform (AccessKit on desktop, a DOM mirror on the web).
#[derive(Clone, Debug, PartialEq)]
pub struct A11yNode {
    /// Stable across frames for the same element.
    pub id: u64,
    pub role: Role,
    pub label: Option<String>,
    /// Extra help text, as ARIA `aria-description`.
    pub description: Option<String>,
    pub value: Option<String>,
    pub bounds: Rect,
    pub focused: bool,
    pub disabled: bool,
    /// `Some(true|false)` for toggles.
    pub checked: Option<bool>,
    /// `Some(true|false)` for choices that can be selected.
    pub selected: Option<bool>,
    /// True when the node takes keyboard focus.
    pub focusable: bool,
    /// For text fields: the caret or selection as `(anchor, focus)` in
    /// UTF-8 byte offsets into `value`. Equal values mean a caret.
    pub text_selection: Option<(u32, u32)>,
    /// For text fields: the value is secret, such as a password. Platforms
    /// must not speak or copy it.
    pub secure: bool,
    /// The node's content reads right to left. Accessibility direction must
    /// match the visual direction.
    pub rtl: bool,
    /// Set when changes to the node's text must be announced.
    pub live: Option<Live>,
    pub children: Vec<A11yNode>,
}

/// How assistive technology announces changes, as ARIA `aria-live`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Live {
    /// Announce when the person is idle.
    Polite,
    /// Announce at once, interrupting. Use only for urgent messages.
    Assertive,
}

/// The color scheme the person asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorScheme {
    #[default]
    Light,
    Dark,
}

/// Where a gamepad's face buttons sit. Inputs are named by printed label
/// (see `Input::A`), so components that draw buttons by place read this.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FaceLayout {
    /// No gamepad seen yet. Components treat it as `Nintendo`, the Brick's.
    #[default]
    Unknown,
    /// A right, B bottom, X top, Y left, as on the TrimUI Brick and
    /// Nintendo pads.
    Nintendo,
    /// A bottom, B right, X left, Y top, as on Xbox-style pads.
    Xbox,
}

/// What the person asked their OS or browser for. Shells read it from the
/// platform and pass changes to `Surface::set_preferences`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Preferences {
    pub reduced_motion: bool,
    pub color_scheme: ColorScheme,
    pub more_contrast: bool,
    /// The platform forces its own colors, such as Windows contrast themes.
    pub forced_colors: bool,
    /// The primary pointer is coarse, such as a touch screen. Reduced motion
    /// then cross-fades instead of changing at once.
    pub coarse_pointer: bool,
    /// The time of day where the person is, for shells that know it.
    pub day_phase: DayPhase,
    /// Minutes since local midnight, for scenes lit to follow the day
    /// smoothly. Shells that read a clock set it, refreshed every minute;
    /// others leave `None`.
    pub day_time: Option<f32>,
    /// How long since the person last touched the device, for shells that
    /// save power. Apps may calm motion while `Dim`.
    pub idle: Idle,
    /// The person turned sounds off.
    pub sound_off: bool,
    /// The face-button layout of the gamepad in use. The Brick shell sets
    /// `Nintendo`; desktop and web set it from the pad that last sent
    /// input.
    pub face_layout: FaceLayout,
    /// The device's radios, battery and clock, for a status bar. Shells
    /// that read them set it, refreshed every 30 seconds; others leave
    /// `None`.
    pub device: Option<DeviceStatus>,
}

/// What a status bar shows. See [`Preferences::device`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeviceStatus {
    pub wifi: Radio,
    /// The network Wi-Fi is connected to, when known.
    pub wifi_name: Option<String>,
    pub bluetooth: Radio,
    /// Battery charge, 0 to 100; `None` without a battery.
    pub battery_percent: Option<u8>,
    /// Plugged in and charging, or full on power.
    pub charging: bool,
    /// Local time, hours 0 to 23 and minutes.
    pub time: (u8, u8),
}

/// A radio's state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Radio {
    #[default]
    Off,
    /// On, not connected.
    On,
    Connected,
}

/// Whether the person is using the device. Shells that save power set it;
/// others leave `Active`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Idle {
    #[default]
    Active,
    /// No input for a while: the shell draws at a low frame rate.
    Dim,
    /// No input for longer: the shell draws nothing until input arrives.
    Asleep,
}

/// The time of day, for scenes lit to match it. Shells that read a clock
/// set it; others leave `Day`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DayPhase {
    #[default]
    Day,
    /// Sunrise and sunset.
    Dusk,
    Night,
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
    /// An action from assistive technology on the node with this `A11yNode`
    /// id, for example a screen reader moving focus or clicking.
    fn a11y_action(&mut self, id: u64, action: A11yAction) -> Command {
        let _ = (id, action);
        Command::None
    }
    /// Text the person typed or committed from an input method, for the
    /// focused text field.
    fn text_input(&mut self, text: &str) -> Command {
        let _ = text;
        Command::None
    }
    /// Text an input method is composing but has not committed, with the
    /// cursor as a UTF-8 byte offset into it. Empty text ends composing.
    /// Committed text still arrives through `text_input`.
    fn text_compose(&mut self, text: &str, cursor: u32) {
        let _ = (text, cursor);
    }
    /// Wheel, trackpad, or touch scrolling at `at`, by `delta` (logical
    /// points, positive y scrolls content up). Returns true when a scroll
    /// container moved, so the shell keeps the event from the page.
    fn scroll(&mut self, at: Point, delta: Point) -> bool {
        let _ = (at, delta);
        false
    }
    /// Where a mouse or pen pointer is, in logical points, or `None` when
    /// it left the window. Shells send it as the pointer moves so the
    /// element under it can show hover; touch never sends it.
    fn pointer_move(&mut self, at: Option<Point>) {
        let _ = at;
    }
    /// A command the app returned outside an input, such as while a held
    /// press completes during a frame. Shells take it after each frame.
    fn take_command(&mut self) -> Command {
        Command::None
    }
    /// Images added, replaced, or freed (`None`) since the last call. Shells
    /// drain it every frame, before drawing, into their renderer.
    fn take_image_updates(&mut self) -> Vec<(ImageId, Option<ImageData>)> {
        Vec::new()
    }
    /// Sounds added, replaced, or freed (`None`) since the last call, as
    /// WAV file bytes. Shells drain it into their audio output.
    fn take_sound_updates(&mut self) -> Vec<(SoundId, Option<std::sync::Arc<[u8]>>)> {
        Vec::new()
    }
    /// The person's preferences changed, or are known for the first time.
    fn set_preferences(&mut self, preferences: &Preferences) {
        let _ = preferences;
    }
}

/// What assistive technology can ask a node to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A11yAction {
    /// Move app focus to the node.
    Focus,
    /// Activate the node, as `Input::A` does for the focused node. Assistive
    /// technology sends it; it acts at once, even on controls that confirm
    /// only after a held press.
    Activate,
    /// A pointer went down on the node: focus it and start a press, as
    /// `Input::A` going down does.
    PressStart,
    /// The pointer that started a press came up over the same node: end the
    /// press, as `Input::A` coming up does, which activates it (a control
    /// that confirms on a held press does so only if it was held long
    /// enough).
    PressEnd,
    /// The pointer came up elsewhere: end the press without activating.
    /// Surfaces that do not track presses treat `PressStart` as `Focus`,
    /// `PressEnd` as `Activate`, and ignore `PressCancel`.
    PressCancel,
}
