//! The Elm-style app model and the runtime that drives it as a [`Surface`].

use crate::contract::{
    A11yAction, A11yNode, Color, Command, DisplayList, DrawCmd, FontFamily, FrameInfo, ImageData,
    ImageId, Input, InputPhase, Live, Point, PointerEvent, PointerOutcome, PointerPhase,
    Preferences, Radii, Rect, Role, Size, SoundId, Surface, TextAlign, TextDirection, TextMeasure,
};
use crate::element::{Axis, BuildCx, Draw, Edit, Element, Node, PopupAction, Semantics};
use crate::layer::{DismissReason, LayerAnchor, PlaceInput};
use crate::layout::{self, Layout};
use crate::paint::{self, PaintState, FOCUS_RING_OFFSET, FOCUS_RING_WIDTH};
use crate::theme::{Direction, ThemeChoice};
use crate::{a11y, focus, paged};
use astryx_motion::gesture::{
    Page, PagingGesture, Phase, PressEvent, PressGesture, PressHighlight, RepeatGesture,
    DPAD_REPEAT,
};
use astryx_motion::tokens::{self as motion_tokens, MotionTokens};
use astryx_motion::{Animated, Change, Motion, MotionPolicy, Pointer};
use std::collections::{HashMap, HashSet};
use taffy::{FlexDirection, Style};

/// An Astryx app: state, a view of it, and an update for each message.
///
/// ```
/// use astryx_core::{App, Button, Command, Element, Text, VStack};
///
/// struct Counter(u32);
///
/// #[derive(Clone)]
/// enum Msg {
///     Add,
/// }
///
/// impl App for Counter {
///     type Msg = Msg;
///     fn view(&self) -> Element<Msg> {
///         VStack::new()
///             .child(Text::new(format!("Count: {}", self.0)))
///             .child(Button::new("Add").on_click(Msg::Add))
///             .into()
///     }
///     fn update(&mut self, msg: Msg) -> Command {
///         match msg {
///             Msg::Add => self.0 += 1,
///         }
///         Command::None
///     }
/// }
/// ```
pub trait App {
    type Msg: Clone + 'static;

    fn view(&self) -> Element<Self::Msg>;

    fn update(&mut self, msg: Self::Msg) -> Command;

    /// The theme and color mode. Read after every update, so switching it is
    /// an ordinary message.
    fn theme(&self) -> ThemeChoice {
        ThemeChoice::default()
    }

    /// The writing direction of the whole app. Read after every update, so
    /// switching it is an ordinary message.
    fn direction(&self) -> Direction {
        Direction::Ltr
    }

    /// Inputs focus does not use: B (back), X, Y, the shoulder buttons,
    /// Start, Select, and Menu, on press. Return a message to handle one.
    fn on_input(&self, input: Input) -> Option<Self::Msg> {
        let _ = input;
        None
    }

    /// Experimental. Called after focus moves, by input, an accessibility
    /// action, or a rebuild, with the `id` prop of the focused element (or
    /// its nearest ancestor that has one), or `None` when nothing has
    /// focus. Return a message to follow focus, such as to show details of
    /// the focused item. A message returned during a frame cannot return a
    /// command.
    fn on_focus(&self, id: Option<&str>) -> Option<Self::Msg> {
        let _ = id;
        None
    }

    /// Called when the shell passes new preferences, such as the color
    /// scheme or the time of day (`day_phase`). Return a message to keep
    /// what the app needs, such as lighting a scene for night. Themes that
    /// follow the system scheme switch on their own.
    fn on_preferences(&self, preferences: &Preferences) -> Option<Self::Msg> {
        let _ = preferences;
        None
    }

    /// Called with a cue for each interaction the person makes, so the app
    /// can answer with a sound or a rumble. Components stay silent; return
    /// a message to play something.
    fn on_feedback(&self, kind: FeedbackKind) -> Option<Self::Msg> {
        let _ = kind;
        None
    }
}

/// What just happened, for an app to answer with sound or rumble.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FeedbackKind {
    /// Focus moved, a list scrolled, or a keyboard group or layer changed.
    Move,
    /// A control was pressed or a key typed.
    Select,
    /// B closed a layer or went back, or a keyboard was cancelled.
    Back,
    /// An action failed. The core sends none; components and apps may.
    Error,
    /// Nothing happened: a disabled control was pressed, focus is at an
    /// edge, or there is nothing to delete.
    Blocked,
    /// A held press passed a quarter of its time.
    Progress,
    /// A held press completed, or a keyboard finished.
    Success,
}

/// The overlap of two rects, if any.
fn intersect(a: Rect, b: Rect) -> Option<Rect> {
    let (x0, y0) = (a.x.max(b.x), a.y.max(b.y));
    let (x1, y1) = (
        (a.x + a.width).min(b.x + b.width),
        (a.y + a.height).min(b.y + b.height),
    );
    (x1 > x0 && y1 > y0).then(|| Rect::new(x0, y0, x1 - x0, y1 - y0))
}

/// A pager's pages: each page's offset and the height it shows.
#[derive(Clone, Debug, Default)]
struct Pager {
    pages: Vec<(f32, f32)>,
    page: usize,
    /// A wheel moved it off a page start; the D-pad snaps it back.
    free: bool,
}

/// Pages with more dots than this show "Page X of Y" instead.
const MAX_PAGE_DOTS: usize = 12;

/// A press on a hold-to-confirm control, from when A went down.
#[derive(Clone, Copy, Debug)]
struct Hold {
    id: u64,
    started_ms: f64,
    duration_ms: f32,
    /// It reached its end and activated; it stays full until released.
    done: bool,
    /// The quarters of its time already cued.
    quarter: u8,
}

/// Astryx's press scale for a standalone Button.
const PRESS_SCALE: f32 = 0.98;

/// A moving node: where it is shown, relative to its parent, and its width.
#[derive(Clone, Copy, Debug)]
struct Moving {
    shown: Animated<[f32; 3]>,
}

/// Astryx's widest tooltip, in points.
const TOOLTIP_MAX_WIDTH: f32 = 300.0;

/// How long the phases of the last frame took, for profiling shells.
/// All zero on targets without a clock (WebAssembly).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameTimings {
    pub rebuild: std::time::Duration,
    pub layout: std::time::Duration,
    pub a11y: std::time::Duration,
    pub paint: std::time::Duration,
    /// Whether the frame recomputed layout; a rebuild that changes only
    /// colors, such as most theme and mode switches, keeps the last one.
    pub laid_out: bool,
}

/// A stopwatch that reads zero where `Instant` is unavailable.
struct Stopwatch(#[cfg(not(target_arch = "wasm32"))] std::time::Instant);

impl Stopwatch {
    fn start() -> Self {
        Stopwatch(
            #[cfg(not(target_arch = "wasm32"))]
            std::time::Instant::now(),
        )
    }

    /// Time since the last lap, restarting the watch.
    fn lap(&mut self) -> std::time::Duration {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let now = std::time::Instant::now();
            let d = now - self.0;
            self.0 = now;
            d
        }
        #[cfg(target_arch = "wasm32")]
        std::time::Duration::ZERO
    }
}

/// How many vanished focus targets the runtime remembers.
const LOST_FOCUS_MEMORY: usize = 16;

/// A scroll indicator: fully shown until `hold_until_ms`, then fading.
#[derive(Clone, Copy, Debug)]
struct ScrollIndicator {
    opacity: Animated<f32>,
    hold_until_ms: f64,
}

/// How long the scroll indicator stays fully shown after a scroll.
const INDICATOR_HOLD_MS: f64 = 1000.0;

/// Runs an [`App`]: builds its view, lays it out, moves focus, and paints.
pub struct Runtime<A: App> {
    app: A,
    theme: ThemeChoice,
    nodes: Vec<Node<A::Msg>>,
    layout: Option<Layout>,
    stale: bool,
    relayout: bool,
    viewport: (f32, f32),
    /// Focus stops in reading order: node index and id.
    stops: Vec<(usize, u64)>,
    focused: Option<u64>,
    /// The focused box, kept to place focus when its element goes away.
    focused_box: Option<Rect>,
    pressed: Option<u64>,
    prefs: Preferences,
    direction: Direction,
    /// Caret positions of text fields, in characters, by node id.
    carets: HashMap<u64, usize>,
    measure_generation: u64,
    /// A field's value typed since the last rebuild.
    typed: Option<(u64, String)>,
    /// Text an input method is composing in a field: the field's id, the
    /// text, and the cursor as a byte offset into it.
    composing: Option<(u64, String, u32)>,
    /// The control whose popup is open.
    popup: Option<u64>,
    /// Focused elements that went away, most recent last.
    lost: Vec<u64>,
    /// The scroll container around the focused element.
    focus_container: Option<u64>,
    /// The focus stop whose tooltip B dismissed; it stays hidden until
    /// focus moves.
    tooltip_dismissed: Option<u64>,
    /// Milliseconds since the runtime started, from frame times; drives
    /// animations and gestures.
    motion_ms: f64,
    policy: MotionPolicy,
    /// The offset each scroll container shows, moving toward `scroll`.
    shown: HashMap<u64, Animated<f32>>,
    scroll_indicators: HashMap<u64, ScrollIndicator>,
    /// A press on the focused element.
    press: PressGesture,
    /// The pressed overlay's strength.
    highlight: PressHighlight,
    /// D-pad hold-to-repeat.
    repeats: RepeatGesture<Input>,
    /// L2 and R2.
    paging: PagingGesture,
    /// A hold in progress, and the fill draining after a hold let go early.
    hold: Option<Hold>,
    hold_release: Option<(u64, Animated<f32>)>,
    /// The press scale, and the element it applies to until it is back at 1.
    press_scale: Animated<f32>,
    scaled: Option<u64>,
    /// Offset from its target place (x, y) and size, for each moving node.
    moving: HashMap<u64, Moving>,
    /// Each pager's pages and the page it shows, by node id.
    pagers: HashMap<u64, Pager>,
    /// The pager whose page has no focus stop, while focus rests on it.
    bare_page: Option<u64>,
    /// The view was rebuilt since the last frame.
    rebuilt: bool,
    /// Node timers already sent, by node id and time.
    fired_timers: HashSet<(u64, u64)>,
    /// Where the mouse or pen pointer is, if over the window.
    pointer: Option<Point>,
    /// Commands from cues and frames not yet handed to the shell.
    pending: Command,
    /// The focus last reported to `App::on_focus`.
    reported_focus: Option<Option<u64>>,
    /// Each camera's offset (x, y), moving toward its target.
    cameras: HashMap<u64, Animated<(f32, f32)>>,
    scroll: HashMap<u64, f32>,
    reveal: bool,
    /// Focus or press changed since the last frame.
    repaint: bool,
    animating: bool,
    /// Something that animates is on screen in the last frame.
    animating_visible: bool,
    time: f32,
    a11y: Option<A11yNode>,
    timings: FrameTimings,
    /// Registered images: true when ready, false when failed.
    images: std::sync::Arc<HashMap<ImageId, bool>>,
    image_sizes: std::sync::Arc<HashMap<ImageId, (u32, u32)>>,
    /// Image changes the shell has not taken yet.
    image_updates: Vec<(ImageId, Option<ImageData>)>,
    sound_updates: Vec<(SoundId, Option<std::sync::Arc<[u8]>>)>,
    /// Open layers by root id, with the focus they restore when they close.
    layer_openers: HashMap<u64, Option<u64>>,
    /// Hover tracking for nodes with a hover spec, by node id.
    hovers: HashMap<u64, HoverTrack>,
    /// Focus last moved by a pointer press, so it is not focus-visible.
    focus_by_pointer: bool,
    /// Animated paint channels by node id and channel.
    anims: HashMap<(u64, crate::motion::Channel), Animated<f32>>,
    /// Exit copies playing out.
    exits: Vec<ExitCopy>,
    /// How each node with an exit spec was last drawn, by node id.
    captured: HashMap<u64, (Vec<DrawCmd>, Rect)>,
    /// Height animations by node id.
    resizes: HashMap<u64, ResizeTrack>,
}

/// A closed node's last drawing, leaving.
struct ExitCopy {
    id: u64,
    commands: Vec<DrawCmd>,
    rect: Rect,
    opacity: Animated<f32>,
    scale: Animated<f32>,
    translate: Animated<(f32, f32)>,
}

/// A node's natural height and the height it is drawn at.
struct ResizeTrack {
    natural: f32,
    shown: Animated<f32>,
}

/// Where the pointer has been relative to a node that reports hover.
#[derive(Clone, Copy, Debug, Default)]
struct HoverTrack {
    /// What the app was last told.
    on: bool,
    /// Since when the pointer has been inside (or outside) without a
    /// report, in runtime milliseconds.
    inside_since: Option<f64>,
    outside_since: Option<f64>,
}

impl<A: App> Runtime<A> {
    pub fn new(app: A) -> Self {
        let theme = app.theme();
        Runtime {
            app,
            theme,
            nodes: Vec::new(),
            layout: None,
            stale: true,
            relayout: true,
            viewport: (0.0, 0.0),
            stops: Vec::new(),
            focused: None,
            focused_box: None,
            pressed: None,
            prefs: Preferences::default(),
            direction: Direction::Ltr,
            carets: HashMap::new(),
            measure_generation: 0,
            typed: None,
            composing: None,
            popup: None,
            lost: Vec::new(),
            focus_container: None,
            tooltip_dismissed: None,
            motion_ms: 0.0,
            policy: MotionPolicy::default(),
            shown: HashMap::new(),
            scroll_indicators: HashMap::new(),
            press: PressGesture::new(),
            highlight: PressHighlight::new(),
            repeats: RepeatGesture::new(DPAD_REPEAT),
            paging: PagingGesture::new(),
            hold: None,
            hold_release: None,
            press_scale: Animated::new(1.0),
            scaled: None,
            moving: HashMap::new(),
            pagers: HashMap::new(),
            bare_page: None,
            reported_focus: None,
            rebuilt: false,
            fired_timers: HashSet::new(),
            pointer: None,
            pending: Command::None,
            cameras: HashMap::new(),
            scroll: HashMap::new(),
            reveal: false,
            repaint: true,
            animating: false,
            animating_visible: false,
            time: 0.0,
            a11y: None,
            timings: FrameTimings::default(),
            images: Default::default(),
            image_sizes: Default::default(),
            image_updates: Vec::new(),
            sound_updates: Vec::new(),
            layer_openers: HashMap::new(),
            hovers: HashMap::new(),
            focus_by_pointer: false,
            anims: HashMap::new(),
            exits: Vec::new(),
            captured: HashMap::new(),
            resizes: HashMap::new(),
        }
    }

    /// Hides the focus ring until the person uses the keyboard, as browsers
    /// do for a page that has just loaded (`:focus-visible`). Pointer-first
    /// shells (desktop, web) and screenshot harnesses use it; focus-first
    /// devices keep the ring from the start.
    pub fn hide_focus_until_keyboard(mut self) -> Self {
        self.focus_by_pointer = true;
        self
    }

    /// Registers the image for `id`: `Some` makes it ready, replacing any
    /// earlier one; `None` marks it failed, so components fall back (an
    /// Avatar to `fallback_src`, then initials; a Thumbnail to its
    /// placeholder). The pixels go to the renderer on the next frame
    /// through [`Surface::take_image_updates`]; the runtime keeps no copy.
    /// Use [`ImageId::of`] for an image a component names by `src`.
    pub fn register_image(&mut self, id: ImageId, data: Option<ImageData>) {
        std::sync::Arc::make_mut(&mut self.images).insert(id, data.is_some());
        let sizes = std::sync::Arc::make_mut(&mut self.image_sizes);
        match &data {
            Some(d) => sizes.insert(id, (d.width, d.height)),
            None => sizes.remove(&id),
        };
        self.image_updates.retain(|(i, _)| *i != id);
        self.image_updates.push((id, data));
        self.stale = true;
    }

    /// Adds or replaces a sound (WAV file bytes) the app plays with
    /// [`Command::PlaySound`] or a transition; `None` frees it. The bytes go
    /// to the shell through [`Surface::take_sound_updates`].
    pub fn register_sound(&mut self, id: SoundId, wav: Option<std::sync::Arc<[u8]>>) {
        self.sound_updates.retain(|(i, _)| *i != id);
        self.sound_updates.push((id, wav));
    }

    /// [`Self::register_sound`] as a builder.
    pub fn with_sound(mut self, id: SoundId, wav: std::sync::Arc<[u8]>) -> Self {
        self.register_sound(id, Some(wav));
        self
    }

    /// [`Self::register_image`] as a builder.
    pub fn with_image(mut self, id: ImageId, data: Option<ImageData>) -> Self {
        self.register_image(id, data);
        self
    }

    /// Forgets the image for `id` and frees it in the renderer; components
    /// treat it as loading again.
    pub fn forget_image(&mut self, id: ImageId) {
        if std::sync::Arc::make_mut(&mut self.images)
            .remove(&id)
            .is_some()
        {
            self.image_updates.retain(|(i, _)| *i != id);
            self.image_updates.push((id, None));
            self.stale = true;
        }
    }

    pub fn app(&self) -> &A {
        &self.app
    }

    /// Changes app state from outside a message; the next frame rebuilds.
    pub fn app_mut(&mut self) -> &mut A {
        self.stale = true;
        &mut self.app
    }

    /// Sends the node timers whose time has come, each once.
    fn fire_timers(&mut self, now: f64) -> Option<Command> {
        let due: Vec<(u64, f64, A::Msg)> = self
            .nodes
            .iter()
            .filter_map(|n| n.timer.as_ref().map(|(at, msg)| (n.id, *at, msg)))
            .filter(|&(id, at, _)| at <= now && !self.fired_timers.contains(&(id, at.to_bits())))
            .map(|(id, at, msg)| (id, at, msg.clone()))
            .collect();
        let mut command = None;
        for (id, at, msg) in due {
            self.fired_timers.insert((id, at.to_bits()));
            command = Some(self.send(msg));
        }
        command
    }

    fn timer_pending(&self) -> bool {
        self.nodes.iter().any(|n| {
            n.timer
                .as_ref()
                .is_some_and(|(at, _)| !self.fired_timers.contains(&(n.id, at.to_bits())))
        })
    }

    /// Gives an input to the focused node that captures input, such as an
    /// on-screen keyboard. `None` when nothing captured it.
    fn capture(&mut self, input: Input, phase: InputPhase) -> Option<Command> {
        if matches!(input, Input::Next | Input::Previous) {
            return None;
        }
        let id = self.focused?;
        let node = self.nodes.iter().find(|n| n.id == id)?;
        let capture = node.capture.clone()?;
        self.repaint = true;
        // The keyboard edits at the end of the text, so the caret goes there.
        self.carets.remove(&id);
        let (msg, cue) = capture(input, phase, self.motion_ms);
        let command = match msg {
            Some(msg) => self.send(msg),
            None => Command::None,
        };
        if let Some(kind) = cue {
            self.cue(kind);
        }
        Some(command)
    }

    /// Hands a feedback cue to the app. A command it returns joins the
    /// input's, or waits for `take_command` when the cue came in a frame.
    fn cue(&mut self, kind: FeedbackKind) {
        if let Some(msg) = self.app.on_feedback(kind) {
            let command = self.send(msg);
            self.pending = std::mem::take(&mut self.pending).and(command);
        }
    }

    /// Tells the app where focus went, once per change.
    fn report_focus(&mut self) -> Command {
        if self.reported_focus == Some(self.focused) {
            return Command::None;
        }
        self.reported_focus = Some(self.focused);
        let name = self.focused.and_then(|id| {
            let mut ix = Some(self.nodes.iter().position(|n| n.id == id)?);
            while let Some(i) = ix {
                if let Some(name) = &self.nodes[i].name {
                    return Some(name.clone());
                }
                ix = self.nodes[i].parent.map(|p| p as usize);
            }
            None
        });
        match self.app.on_focus(name.as_deref()) {
            Some(msg) => self.send(msg),
            None => Command::None,
        }
    }

    /// Sends a message as a control would.
    pub fn send(&mut self, msg: A::Msg) -> Command {
        let command = self.app.update(msg);
        self.stale = true;
        command
    }

    /// The preferences the shell passed last.
    pub fn preferences(&self) -> &Preferences {
        &self.prefs
    }

    /// Phase timings of the last frame.
    pub fn timings(&self) -> FrameTimings {
        self.timings
    }

    /// The accessibility id of the focused element.
    pub fn focused(&self) -> Option<u64> {
        self.focused
    }

    fn rebuild(&mut self) {
        self.theme = self.app.theme();
        let tokens = self.theme.tokens(&self.prefs);
        let mut cx = BuildCx::new(tokens);
        cx.theme = self.theme.theme;
        cx.prefs = self.prefs.clone();
        cx.viewport = self.viewport;
        self.direction = self.app.direction();
        cx.direction = self.direction;
        cx.images = self.images.clone();
        cx.image_sizes = self.image_sizes.clone();
        cx.now_ms = self.motion_ms;
        if let Some(layout) = &self.layout {
            cx.last_rects = std::sync::Arc::new(
                self.nodes
                    .iter()
                    .enumerate()
                    .filter_map(|(ix, n)| Some((n.name.clone()?, *layout.rects.get(ix)?)))
                    .collect(),
            );
        }
        cx.popup = self.popup.and_then(|id| {
            let ix = self.nodes.iter().position(|n| n.id == id)?;
            Some((id, self.layout.as_ref()?.rects.get(ix).copied()?))
        });
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            ..Style::default()
        }));
        cx.child(self.app.view());
        // Popups go last, so they paint above the view and are not clipped
        // by it. A layer opened inside a layer queues another overlay, which
        // goes after, so it stacks above.
        while !cx.overlays.is_empty() {
            for overlay in std::mem::take(&mut cx.overlays) {
                overlay(&mut cx);
            }
        }
        cx.end();
        self.animating = cx.animating;
        let old = std::mem::replace(&mut self.nodes, cx.nodes);
        self.typed = None;
        // The app's root may shrink to the window, so a scrolling view inside
        // it scrolls instead of growing past the bottom edge.
        if let Some(&first) = self.nodes.first().and_then(|root| root.children.first()) {
            let min = &mut self.nodes[first as usize].style.min_size.height;
            if min.is_auto() {
                *min = taffy::LengthPercentageAuto::length(0.0);
            }
        }
        if let Some(id) = self.popup {
            if !self.nodes.iter().any(|n| n.trap) || !self.nodes.iter().any(|n| n.id == id) {
                self.popup = None;
            }
        }
        self.track_layers(&old);
        self.sync_motion(&old);
        self.stale = false;
        self.rebuilt = true;
        // A rebuild that changes only paint, such as a switch to a theme
        // with the same sizes, keeps the layout it already has; one that
        // changes a few boxes lays out only what they touch.
        let change = match self.layout.as_mut() {
            Some(layout) => layout.update(&old, &self.nodes),
            None => layout::Change::Shape,
        };
        match change {
            layout::Change::None => {}
            layout::Change::Some => self.relayout = true,
            layout::Change::Shape => {
                self.layout = Some(Layout::new(&self.nodes));
                self.relayout = true;
            }
        }
    }

    /// True when a spinner or sliding fill is inside the viewport and not
    /// clipped away by a scroll container or clip, so off-screen animations
    /// cost no frames.
    fn animation_visible(&self) -> bool {
        if !self.animating {
            return false;
        }
        let Some(layout) = &self.layout else {
            return false;
        };
        let intersect = |a: Rect, b: Rect| {
            let x0 = a.x.max(b.x);
            let y0 = a.y.max(b.y);
            let x1 = (a.x + a.width).min(b.x + b.width);
            let y1 = (a.y + a.height).min(b.y + b.height);
            (x1 > x0 && y1 > y0).then(|| Rect::new(x0, y0, x1 - x0, y1 - y0))
        };
        let viewport = Rect::new(0.0, 0.0, self.viewport.0, self.viewport.1);
        self.nodes.iter().enumerate().any(|(ix, node)| {
            let plays = match &node.draw {
                Draw::Spinner(_) => true,
                Draw::Paint(p) => p.plays,
                Draw::Sprite(s) => s.frames > 1,
                Draw::Light(l) => l.plays(),
                _ => false,
            };
            if !plays && node.slide.is_none() {
                return false;
            }
            let area = |r: Rect| match &node.draw {
                Draw::Light(l) => l.reach(r),
                _ => r,
            };
            let Some(mut visible) = layout
                .rects
                .get(ix)
                .and_then(|&r| intersect(area(r), viewport))
            else {
                return false;
            };
            let mut ancestor = node.parent;
            while let Some(a) = ancestor {
                let a = a as usize;
                if self.nodes[a].clip.is_some() || self.nodes[a].scroll.is_some() {
                    match intersect(visible, layout.rects[a]) {
                        Some(r) => visible = r,
                        None => return false,
                    }
                }
                ancestor = self.nodes[a].parent;
            }
            true
        })
    }

    fn stop_index(&self, id: Option<u64>) -> Option<usize> {
        id.and_then(|id| self.stops.iter().position(|&(_, s)| s == id))
    }

    fn stop_rects(&self) -> Vec<Rect> {
        let Some(layout) = &self.layout else {
            return Vec::new();
        };
        // Stops on other pages of a pager are out of reach of the arrows.
        self.stops
            .iter()
            .map(|&(ix, _)| match self.pager_of(ix) {
                Some(p) if !self.on_current_page(p, ix) => Rect::new(0.0, 0.0, 0.0, 0.0),
                _ => layout.rects[ix],
            })
            .collect()
    }

    /// The nearest pager around node `ix`.
    fn pager_of(&self, ix: usize) -> Option<usize> {
        let mut ancestor = self.nodes[ix].parent;
        while let Some(a) = ancestor {
            if self.nodes[a as usize].pager {
                return Some(a as usize);
            }
            ancestor = self.nodes[a as usize].parent;
        }
        None
    }

    /// Node `ix`'s extent inside pager `p`, from the pager's top edge.
    fn page_span(&self, p: usize, ix: usize) -> (f32, f32) {
        let layout = self.layout.as_ref().expect("layout");
        let (view, b) = (layout.boxes[p], layout.boxes[ix]);
        (b.y - view.y, b.y + b.height - view.y)
    }

    fn on_current_page(&self, p: usize, ix: usize) -> bool {
        let Some(pager) = self.pagers.get(&self.nodes[p].id) else {
            return true;
        };
        if pager.free {
            return true;
        }
        let (top, bottom) = self.page_span(p, ix);
        pager
            .pages
            .get(pager.page)
            .is_none_or(|&page| paged::on_page(page, top, bottom))
    }

    /// The focus stops of pager `p` on page `page`, in reading order.
    fn stops_on_page(&self, p: usize, page: usize) -> Vec<usize> {
        let Some(&range) = self
            .pagers
            .get(&self.nodes[p].id)
            .and_then(|g| g.pages.get(page))
        else {
            return Vec::new();
        };
        (0..self.stops.len())
            .filter(|&s| {
                let ix = self.stops[s].0;
                self.pager_of(ix) == Some(p) && {
                    let (top, bottom) = self.page_span(p, ix);
                    paged::on_page(range, top, bottom)
                }
            })
            .collect()
    }

    /// Recomputes every pager's pages after layout, keeps each on its page
    /// (or the page holding its old offset), and names the page for
    /// assistive technology.
    fn update_pagers(&mut self) {
        let Some(layout) = self.layout.as_mut() else {
            return;
        };
        let mut seen = HashSet::new();
        for ix in 0..self.nodes.len() {
            if !self.nodes[ix].pager {
                continue;
            }
            let id = self.nodes[ix].id;
            seen.insert(id);
            let pages = paged::pages(&self.nodes, &layout.boxes, ix, layout.padding_block(ix));
            let pager = self.pagers.entry(id).or_default();
            pager.page = pager.page.min(pages.len() - 1);
            let last = pages.last().map_or(0.0, |p| p.0);
            // The last page starts at its first item, which may be past the
            // content's plain scroll end.
            layout.overflow[ix] = layout.overflow[ix].max(last);
            if !pager.free && self.scroll.get(&id).copied().unwrap_or(0.0) != pages[pager.page].0 {
                self.scroll.insert(id, pages[pager.page].0);
                self.shown.remove(&id);
            }
            pager.pages = pages;
            let label = (pager.pages.len() > 1)
                .then(|| format!("Page {} of {}", pager.page + 1, pager.pages.len()));
            if let Some(&strip) = self.nodes[ix]
                .parent
                .and_then(|o| self.nodes[o as usize].children.get(1))
            {
                self.nodes[strip as usize].sem = label.map(|l| {
                    let mut sem = Semantics::new(Role::Text, l);
                    sem.live = Some(Live::Polite);
                    sem
                });
            }
        }
        self.pagers.retain(|id, _| seen.contains(id));
        if self.bare_page.is_some_and(|id| !seen.contains(&id)) {
            self.bare_page = None;
        }
        layout.place(&self.nodes, &self.scroll);
    }

    /// Shows page `page` of pager `p`, sliding to it with `smooth`.
    fn set_page(&mut self, p: usize, page: usize, smooth: bool) {
        let id = self.nodes[p].id;
        let Some(pager) = self.pagers.get_mut(&id) else {
            return;
        };
        let Some(&(offset, _)) = pager.pages.get(page) else {
            return;
        };
        pager.page = page;
        pager.free = false;
        self.repaint = true;
        self.update_page_label(p);
        self.set_scroll(p, offset, smooth);
    }

    fn update_page_label(&mut self, p: usize) {
        let Some(pager) = self.pagers.get(&self.nodes[p].id) else {
            return;
        };
        let label = format!("Page {} of {}", pager.page + 1, pager.pages.len());
        if let Some(&strip) = self.nodes[p]
            .parent
            .and_then(|o| self.nodes[o as usize].children.get(1))
        {
            if let Some(sem) = self.nodes[strip as usize].sem.as_mut() {
                sem.label = Some(label);
            }
        }
    }

    /// Flips pager `p` to `page` and puts focus on its first stop, or its
    /// last when going back; a page without stops holds focus itself.
    fn flip(&mut self, p: usize, page: usize, first: bool) {
        self.set_page(p, page, true);
        let stops = self.stops_on_page(p, page);
        let pick = if first { stops.first() } else { stops.last() };
        match pick {
            Some(&stop) => self.set_focus(stop),
            None => {
                self.end_press();
                self.focused = None;
                self.bare_page = Some(self.nodes[p].id);
            }
        }
    }

    /// The pager focus is in: around the focused stop, or the one whose
    /// page without stops holds focus.
    fn focus_pager(&self) -> Option<usize> {
        match self.stop_index(self.focused) {
            Some(stop) => self.pager_of(self.stops[stop].0),
            None => self
                .bare_page
                .and_then(|id| self.nodes.iter().position(|n| n.id == id)),
        }
    }

    /// An arrow inside a pager. Focus moves within the page; past the last
    /// stop in that direction the page flips: Down and Up always, Right and
    /// Left from the page's last or first stop.
    fn paged_directional(&mut self, p: usize, input: Input) -> Command {
        let id = self.nodes[p].id;
        if self.pagers.get(&id).is_some_and(|g| g.free) {
            // After a wheel, the D-pad first returns to the page focus is on.
            let page = match self.stop_index(self.focused) {
                Some(stop) => {
                    let top = self.page_span(p, self.stops[stop].0).0;
                    paged::page_at(&self.pagers[&id].pages, top)
                }
                None => paged::page_at(
                    &self.pagers[&id].pages,
                    self.scroll.get(&id).copied().unwrap_or(0.0),
                ),
            };
            self.set_page(p, page, true);
        }
        let Some(pager) = self.pagers.get(&id) else {
            return Command::None;
        };
        let (page, count) = (pager.page, pager.pages.len());
        let rects = self.stop_rects();
        let current = self.stop_index(self.focused);
        let candidate = current.and_then(|c| focus::navigate(&rects, Some(c), input));
        if let Some(s) = candidate.filter(|&s| self.pager_of(self.stops[s].0) == Some(p)) {
            self.set_focus(s);
            self.cue(FeedbackKind::Move);
            return Command::None;
        }
        let on_page = self.stops_on_page(p, page);
        let flips = match input {
            Input::Down | Input::Up => true,
            Input::Right => current.is_none() || current == on_page.last().copied(),
            _ => current.is_none() || current == on_page.first().copied(),
        };
        let forward = matches!(input, Input::Down | Input::Right);
        let target = if forward {
            (page + 1 < count).then_some(page + 1)
        } else {
            page.checked_sub(1)
        };
        match (flips, target) {
            (true, Some(next)) => {
                self.flip(p, next, forward);
                self.cue(FeedbackKind::Move);
            }
            _ => match candidate {
                Some(s) => {
                    self.set_focus(s);
                    self.reveal_focus(true);
                    self.cue(FeedbackKind::Move);
                }
                // At the first or last page with nowhere to go.
                None => self.cue(FeedbackKind::Blocked),
            },
        }
        Command::None
    }

    /// Page dots (or "Page X of Y") under each pager, and the height each
    /// pager's page shows, so no part of the next page peeks in.
    fn pager_paint(&self, rects: &[Rect]) -> (HashMap<usize, (f32, f32)>, Vec<DrawCmd>) {
        let mut clips = HashMap::new();
        let mut marks = Vec::new();
        let t = self.theme.tokens(&self.prefs);
        let grid = self.theme.theme.grid;
        if let Some(cell) = grid {
            marks.push(DrawCmd::PushPixelGrid { cell });
        }
        for (ix, node) in self.nodes.iter().enumerate() {
            let Some(pager) = node.pager.then(|| self.pagers.get(&node.id)).flatten() else {
                continue;
            };
            let sliding = self.shown.get(&node.id).is_some_and(|a| a.is_animating());
            if !pager.free && !sliding {
                if let Some(&(offset, h)) = pager.pages.get(pager.page) {
                    // Above a later page's first item is the end of the page
                    // before it, which stays hidden too.
                    let top = match pager.page.checked_sub(1) {
                        Some(prev) => {
                            let (o, ph) = pager.pages[prev];
                            (o + ph - offset).max(0.0)
                        }
                        None => 0.0,
                    };
                    clips.insert(ix, (top, h));
                }
            }
            let count = pager.pages.len();
            let Some(&strip) = node
                .parent
                .and_then(|o| self.nodes[o as usize].children.get(1))
            else {
                continue;
            };
            if count < 2 {
                continue;
            }
            let strip = rects[strip as usize];
            let page = if pager.free {
                paged::page_at(&pager.pages, self.shown_offset(node.id))
            } else {
                pager.page
            };
            if count <= MAX_PAGE_DOTS {
                // Astryx Pagination's small dots: neutral, the current one
                // accent, a step apart, in reading order.
                let (side, gap) = (t.space.s1_5, t.space.s1);
                let width = count as f32 * side + (count - 1) as f32 * gap;
                let x0 = strip.x + (strip.width - width) / 2.0;
                let y = strip.y + (strip.height - side) / 2.0;
                for i in 0..count {
                    let slot = if node.rtl { count - 1 - i } else { i };
                    let fill: Color = if i == page {
                        t.color.accent.into()
                    } else {
                        t.color.neutral.into()
                    };
                    let rect = Rect::new(x0 + slot as f32 * (side + gap), y, side, side);
                    let radii = Radii::all(side / 2.0);
                    match grid {
                        // Stepped dots, as a pixel circle that small draws.
                        Some(cell) => crate::pixel::push_rects(
                            &mut marks,
                            crate::pixel::fill(crate::pixel::snap_rect(rect, cell), radii, cell),
                            fill,
                        ),
                        None => marks.push(DrawCmd::Rect {
                            rect,
                            radii,
                            fill,
                            stroke: None,
                        }),
                    }
                }
            } else {
                let mut style = crate::element::themed_text(
                    self.theme.theme,
                    t,
                    crate::components::text::type_style(t.font.supporting, FontFamily::Body),
                );
                style.direction = Some(if node.rtl {
                    TextDirection::Rtl
                } else {
                    TextDirection::Ltr
                });
                marks.push(DrawCmd::Text {
                    rect: strip,
                    text: format!("Page {} of {}", page + 1, count),
                    style,
                    color: t.color.text_secondary.into(),
                    align: TextAlign::Center,
                });
            }
        }
        if grid.is_some() {
            if marks.len() == 1 {
                marks.clear();
            } else {
                marks.push(DrawCmd::PopPixelGrid);
            }
        }
        (clips, marks)
    }

    /// Moves each camera so the focused item inside it stays out of the
    /// quarter of the viewport nearest each edge, on the theme's medium
    /// move motion (instant under reduced motion), within the content.
    fn follow_cameras(&mut self) {
        let Some(layout) = self.layout.as_ref() else {
            return;
        };
        let focused = self.stop_index(self.focused).map(|s| self.stops[s].0);
        let tokens = self.motion_tokens();
        let motion = self.policy.travel(
            Change::Transition,
            Motion::curve(tokens.durations.medium, tokens.ease_move),
            tokens,
        );
        let mut seen = HashSet::new();
        for ix in 0..self.nodes.len() {
            if !self.nodes[ix].camera {
                continue;
            }
            let id = self.nodes[ix].id;
            seen.insert(id);
            let view = layout.boxes[ix];
            let mut inside = Vec::new();
            let mut stack: Vec<usize> = self.nodes[ix]
                .children
                .iter()
                .map(|&c| c as usize)
                .collect();
            while let Some(n) = stack.pop() {
                inside.push(n);
                stack.extend(self.nodes[n].children.iter().map(|&c| c as usize));
            }
            let (right, bottom) = inside.iter().fold((0.0f32, 0.0f32), |(r, b), &n| {
                let bx = layout.boxes[n];
                (
                    r.max(bx.x + bx.width - view.x),
                    b.max(bx.y + bx.height - view.y),
                )
            });
            // The part of the camera on screen: its own box, cut by the
            // window and by every clipping box around it, such as when a
            // parent could not shrink it to the space its siblings leave.
            let painted = layout.rects[ix];
            let window = Rect::new(0.0, 0.0, self.viewport.0, self.viewport.1);
            let mut shown = intersect(painted, window);
            let mut ancestor = self.nodes[ix].parent;
            while let Some(a) = ancestor {
                let n = &self.nodes[a as usize];
                if n.clip.is_some() || n.scroll.is_some() || n.camera {
                    shown = shown.and_then(|r| intersect(r, layout.rects[a as usize]));
                }
                ancestor = n.parent;
            }
            let camera = self
                .cameras
                .entry(id)
                .or_insert_with(|| Animated::new((0.0, 0.0)));
            let (Some(shown), Some(f)) = (shown, focused.filter(|f| inside.contains(f))) else {
                continue;
            };
            // The band on screen, measured from the camera box's edges.
            let band_x = (shown.x - painted.x, shown.x - painted.x + shown.width);
            let band_y = (shown.y - painted.y, shown.y - painted.y + shown.height);
            let b = layout.boxes[f];
            let (mut tx, mut ty) = camera.target();
            let follow = |t: &mut f32, lo: f32, hi: f32, band: (f32, f32), end: f32| {
                let zone = (band.1 - band.0) / 4.0;
                if lo - *t < band.0 + zone {
                    *t = lo - band.0 - zone;
                } else if hi - *t > band.1 - zone {
                    *t = hi - (band.1 - zone);
                }
                *t = t.clamp(0.0, (end - band.1).max(0.0));
            };
            follow(&mut tx, b.x - view.x, b.x + b.width - view.x, band_x, right);
            follow(
                &mut ty,
                b.y - view.y,
                b.y + b.height - view.y,
                band_y,
                bottom,
            );
            if (tx, ty) != camera.target() {
                camera.animate_to((tx, ty), motion);
            }
        }
        self.cameras.retain(|id, _| seen.contains(id));
        let offsets: HashMap<u64, (f32, f32)> = self
            .cameras
            .iter()
            .map(|(&id, a)| (id, a.value()))
            .collect();
        let layout = self.layout.as_mut().expect("layout");
        if !offsets.is_empty() || !layout.camera.is_empty() {
            layout.camera = offsets;
            layout.place(&self.nodes, &self.scroll);
        }
    }

    fn set_focus(&mut self, stop: usize) {
        self.bare_page = None;
        let (ix, id) = self.stops[stop];
        if self.focused != Some(id) {
            // Moving focus ends a press without activating.
            self.end_press();
            self.tooltip_dismissed = None;
            self.composing = None;
        }
        self.focused = Some(id);
        self.focused_box = self.layout.as_ref().map(|l| l.boxes[ix]);
        self.focus_container = self.scroll_ancestor(ix).map(|a| self.nodes[a].id);
        self.reveal = true;
    }

    /// The nearest scroll container around node `ix`.
    fn scroll_ancestor(&self, ix: usize) -> Option<usize> {
        let mut ancestor = self.nodes[ix].parent;
        while let Some(a) = ancestor {
            if self.nodes[a as usize].scroll.is_some() {
                return Some(a as usize);
            }
            ancestor = self.nodes[a as usize].parent;
        }
        None
    }

    /// Keeps focus on a live stop after a rebuild: the same id when it
    /// survives; else an earlier focus target that came back; else, when
    /// the scrolling page around focus was replaced, the first stop of the
    /// new page; else the stop nearest to where focus was.
    fn settle_focus(&mut self) {
        let trapped = |nodes: &[Node<A::Msg>], mut ix: usize| loop {
            if nodes[ix].trap {
                return true;
            }
            match nodes[ix].parent {
                Some(p) => ix = p as usize,
                None => return false,
            }
        };
        // Only the topmost trap holds focus: a popover opened over a dialog
        // takes it from the dialog.
        let top_trap = self.nodes.iter().rposition(|n| n.trap);
        let in_top_trap = |nodes: &[Node<A::Msg>], mut ix: usize| loop {
            if Some(ix) == top_trap {
                return true;
            }
            match nodes[ix].parent {
                Some(p) => ix = p as usize,
                None => return false,
            }
        };
        let _ = trapped;
        let inert = |nodes: &[Node<A::Msg>], mut ix: usize| loop {
            if nodes[ix].inert {
                return true;
            }
            match nodes[ix].parent {
                Some(p) => ix = p as usize,
                None => return false,
            }
        };
        self.stops = self
            .nodes
            .iter()
            .enumerate()
            .filter(|&(i, n)| {
                n.interact.is_some()
                    && (top_trap.is_none() || in_top_trap(&self.nodes, i))
                    && !inert(&self.nodes, i)
            })
            .map(|(i, n)| (i, n.id))
            .collect();
        if self.stops.is_empty() {
            self.focused = None;
            return;
        }
        // Focus rests on a page that has no stops.
        if self.focused.is_none() && self.bare_page.is_some() {
            return;
        }
        if let Some(stop) = self.stop_index(self.focused) {
            self.focused_box = self.layout.as_ref().map(|l| l.boxes[self.stops[stop].0]);
            return;
        }
        // A held direction stops when its element goes away.
        self.repeats.cancel();
        // Focus that went away with its element comes back to it when the
        // element returns, as going back to a list returns to its row.
        if let Some(gone) = self.focused {
            self.lost.retain(|&id| id != gone);
            self.lost.push(gone);
            if self.lost.len() > LOST_FOCUS_MEMORY {
                self.lost.remove(0);
            }
        }
        if let Some(at) = self
            .lost
            .iter()
            .rposition(|&id| self.stop_index(Some(id)).is_some())
        {
            let id = self.lost.remove(at);
            let stop = self.stop_index(Some(id)).expect("checked above");
            self.set_focus(stop);
            return;
        }
        let page_gone = self
            .focus_container
            .is_some_and(|id| !self.nodes.iter().any(|n| n.id == id));
        if page_gone {
            if let Some(stop) = self
                .stops
                .iter()
                .position(|&(ix, _)| self.scroll_ancestor(ix).is_some())
            {
                self.set_focus(stop);
                return;
            }
        }
        if let Some(stop) = self
            .stops
            .iter()
            .position(|&(ix, _)| self.nodes[ix].autofocus)
        {
            self.set_focus(stop);
            return;
        }
        let nearest = match (self.focused_box, &self.layout) {
            (Some(old), Some(layout)) => {
                let c = old.center();
                let dist = |&(ix, _): &(usize, u64)| {
                    let d = layout.boxes[ix].center();
                    (d.x - c.x).powi(2) + (d.y - c.y).powi(2)
                };
                (0..self.stops.len())
                    .min_by(|&a, &b| dist(&self.stops[a]).total_cmp(&dist(&self.stops[b])))
                    .unwrap_or(0)
            }
            _ => 0,
        };
        self.set_focus(nearest);
    }

    /// The painted rect of the focused element.
    fn focused_rect(&self) -> Option<Rect> {
        let id = self.focused?;
        let ix = self.nodes.iter().position(|n| n.id == id)?;
        self.layout.as_ref()?.rects.get(ix).copied()
    }

    /// Scrolls the nearest scroll container around the focused element so
    /// the element is drawn where it was at `old`, then makes sure it is in
    /// view.
    fn keep_in_place(&mut self, old: Rect) {
        let Some(new) = self.focused_rect() else {
            return;
        };
        let Some(stop) = self.stop_index(self.focused) else {
            return;
        };
        let mut ancestor = self.nodes[self.stops[stop].0].parent;
        while let Some(a) = ancestor {
            let a = a as usize;
            // A pager stays on its page; the page holding focus shows.
            if let (Some(axis), false) = (self.nodes[a].scroll, self.nodes[a].pager) {
                let id = self.nodes[a].id;
                let delta = if axis == Axis::Vertical {
                    new.y - old.y
                } else {
                    new.x - old.x
                } * self.scroll_sign(a);
                let layout = self.layout.as_mut().expect("layout");
                let next = (self.scroll.get(&id).copied().unwrap_or(0.0) + delta)
                    .clamp(0.0, layout.overflow[a]);
                self.scroll.insert(id, next);
                self.shown.remove(&id);
                layout.place(&self.nodes, &self.scroll);
                break;
            }
            ancestor = self.nodes[a].parent;
        }
        self.reveal = true;
    }

    /// Scrolls every scroll container around the focused element so the
    /// element and its ring are in view.
    fn reveal_focus(&mut self, smooth: bool) {
        let Some(stop) = self.stop_index(self.focused) else {
            return;
        };
        let target = self.stops[stop].0;
        let margin = FOCUS_RING_OFFSET + FOCUS_RING_WIDTH;
        let mut ancestor = self.nodes[target].parent;
        while let Some(a) = ancestor {
            let a = a as usize;
            if self.nodes[a].pager {
                let top = self.page_span(a, target).0;
                if let Some(pager) = self.pagers.get(&self.nodes[a].id) {
                    let page = paged::page_at(&pager.pages, top);
                    if pager.free || page != pager.page {
                        self.set_page(a, page, smooth);
                    }
                }
            } else if let Some(axis) = self.nodes[a].scroll {
                let layout = self.layout.as_mut().expect("layout");
                layout.place(&self.nodes, &self.scroll);
                let (view, item) = (layout.rects[a], layout.rects[target]);
                let (v0, v1, i0, i1) = match axis {
                    Axis::Vertical => (
                        view.y,
                        view.y + view.height,
                        item.y - margin,
                        item.y + item.height + margin,
                    ),
                    Axis::Horizontal => (
                        view.x,
                        view.x + view.width,
                        item.x - margin,
                        item.x + item.width + margin,
                    ),
                };
                let current = self.scroll.get(&self.nodes[a].id).copied().unwrap_or(0.0);
                let delta = if i0 < v0 {
                    i0 - v0
                } else if i1 > v1 {
                    (i1 - v1).min(i0 - v0)
                } else {
                    0.0
                };
                self.set_scroll(a, current + delta * self.scroll_sign(a), smooth);
            }
            ancestor = self.nodes[a].parent;
        }
    }

    /// How a scroll offset moves content on screen: 1 when a larger offset
    /// moves content up or left, -1 for a right-to-left row, whose larger
    /// offsets move content right.
    fn scroll_sign(&self, ix: usize) -> f32 {
        if self.nodes[ix].rtl && self.nodes[ix].scroll == Some(Axis::Horizontal) {
            -1.0
        } else {
            1.0
        }
    }

    /// The scroll offset a container shows now, part way through a scroll.
    fn shown_offset(&self, id: u64) -> f32 {
        match self.shown.get(&id) {
            // On a pixel grid, scrolls and page slides move by whole cells.
            Some(a) => match self.theme.theme.grid {
                Some(cell) => crate::pixel::snap(a.value(), cell),
                None => a.value(),
            },
            None => self.scroll.get(&id).copied().unwrap_or(0.0),
        }
    }

    /// The theme's motion tokens.
    fn motion_tokens(&self) -> &'static MotionTokens {
        MotionTokens::by_name(self.theme.theme.name).unwrap_or(motion_tokens::DEFAULT)
    }

    /// Shows container `id`'s indicator fully for a while, then fades it.
    fn show_indicator(&mut self, id: u64) {
        let hold_until_ms = self.motion_ms + INDICATOR_HOLD_MS;
        let indicator = self.scroll_indicators.entry(id).or_insert(ScrollIndicator {
            opacity: Animated::new(1.0),
            hold_until_ms,
        });
        indicator.opacity.set(1.0);
        indicator.hold_until_ms = hold_until_ms;
    }

    /// Scrolls container `ix` to `offset`. With `smooth` it moves on the
    /// theme's move curve (instant under reduced motion) and shows the
    /// indicator; otherwise it jumps, as for focus on the first frame.
    /// Returns false when it is already there.
    fn set_scroll(&mut self, ix: usize, offset: f32, smooth: bool) -> bool {
        let Some(layout) = self.layout.as_mut() else {
            return false;
        };
        let id = self.nodes[ix].id;
        let target = offset.clamp(0.0, layout.overflow[ix]);
        let current = self.scroll.get(&id).copied().unwrap_or(0.0);
        if (target - current).abs() < 0.5 {
            return false;
        }
        let from = self.shown_offset(id);
        self.scroll.insert(id, target);
        if smooth {
            let tokens = self.motion_tokens();
            // A page flip slides a whole page, a longer travel than a step.
            let pager = self.nodes[ix].pager;
            let duration = if pager {
                tokens.durations.medium
            } else {
                tokens.durations.fast
            };
            let motion = self.policy.travel(
                Change::Transition,
                Motion::curve(duration, tokens.ease_move),
                tokens,
            );
            self.shown
                .entry(id)
                .or_insert_with(|| Animated::new(from))
                .animate_to(target, motion);
            if !pager {
                self.show_indicator(id);
            }
        } else {
            self.shown.remove(&id);
        }
        if let Some(layout) = self.layout.as_mut() {
            layout.place(&self.nodes, &self.scroll);
        }
        true
    }

    /// Ends a press on the focused element without activating it.
    fn end_press(&mut self) {
        self.release_hold();
        let tokens = self.motion_tokens();
        for event in self.press.cancel() {
            self.highlight.apply(event, tokens, &self.policy, false);
            self.press_scale.animate_to(
                1.0,
                Motion::curve(tokens.durations.fast, tokens.ease_standard),
            );
        }
    }

    /// The stop Next or Previous moves to. A group of stops, such as a
    /// single-choice group, is one step: it is entered at its entry stop
    /// (the chosen one), or at its first stop going forward and its last
    /// going back when none is chosen, and left in one step.
    fn next_in_order(&self, input: Input) -> Option<usize> {
        let rects = self.stop_rects();
        let group = |s: usize| self.nodes[self.stops[s].0].group;
        let current = self.stop_index(self.focused);
        let from = current.and_then(group);
        let forward = input == Input::Next;
        let mut at = current;
        for _ in 0..self.stops.len() {
            let s = focus::navigate(&rects, at, input)?;
            at = Some(s);
            let Some(g) = group(s) else {
                return Some(s);
            };
            if Some(g) == from {
                continue;
            }
            let members: Vec<usize> = (0..self.stops.len())
                .filter(|&m| group(m) == Some(g))
                .collect();
            let entry = members
                .iter()
                .copied()
                .find(|&m| self.nodes[self.stops[m].0].group_entry);
            let edge = if forward {
                members.first()
            } else {
                members.last()
            };
            if Some(s) == entry.or(edge.copied()) {
                return Some(s);
            }
        }
        None
    }

    fn is_hold(&self, id: Option<u64>) -> bool {
        id.and_then(|id| self.nodes.iter().find(|n| n.id == id))
            .and_then(|n| n.interact.as_ref())
            .is_some_and(|i| i.hold.is_some())
    }

    /// The hold's fill now, 0 to 1, for the node it fills.
    fn hold_fill(&self) -> Option<(u64, f32)> {
        if let Some(h) = self.hold {
            let p = if h.done {
                1.0
            } else {
                ((self.motion_ms - h.started_ms) as f32 / h.duration_ms).clamp(0.0, 1.0)
            };
            return Some((h.id, p));
        }
        self.hold_release.as_ref().map(|(id, a)| (*id, a.value()))
    }

    /// Ends a hold. One let go before its end drains its fill on the
    /// theme's exit motion (at once under reduced motion) and does nothing.
    fn release_hold(&mut self) {
        let Some((id, fill)) = self.hold_fill().filter(|_| self.hold.is_some()) else {
            return;
        };
        let done = self.hold.take().is_some_and(|h| h.done);
        let tokens = self.motion_tokens();
        let drain = self.policy.travel(
            Change::Transition,
            Motion::curve(tokens.durations.exit, tokens.ease_exit),
            tokens,
        );
        let mut a = Animated::new(if done { 0.0 } else { fill });
        a.animate_to(0.0, drain);
        self.hold_release = a.is_animating().then_some((id, a));
        self.repaint = true;
    }

    /// A press event on the focused element: show it, activate on release.
    fn press_event(&mut self, event: PressEvent) -> Command {
        let tokens = self.motion_tokens();
        self.highlight.apply(event, tokens, &self.policy, false);
        // Button's press scale moves on its fast, standard transition, and
        // never under reduced motion (Button FR15, reduced-motion DR1).
        let scale_motion = Motion::curve(tokens.durations.fast, tokens.ease_standard);
        match event {
            PressEvent::Down => {
                self.pressed = self.focused;
                let hold = self.focused.and_then(|id| {
                    let n = self.nodes.iter().find(|n| n.id == id)?;
                    let i = n.interact.as_ref().filter(|i| !i.disabled && !i.busy)?;
                    Some((id, i.hold?))
                });
                if let Some((id, duration_ms)) = hold {
                    self.hold_release = None;
                    self.hold = Some(Hold {
                        id,
                        started_ms: self.motion_ms,
                        duration_ms,
                        done: false,
                        quarter: 0,
                    });
                }
                let scales = self
                    .focused
                    .and_then(|id| self.nodes.iter().find(|n| n.id == id))
                    .is_some_and(|n| {
                        n.press_scale && n.interact.as_ref().is_some_and(|i| !i.disabled && !i.busy)
                    });
                if scales && !self.policy.reduces(tokens) {
                    self.scaled = self.focused;
                    self.press_scale.animate_to(PRESS_SCALE, scale_motion);
                }
                Command::None
            }
            PressEvent::Up => {
                self.press_scale.animate_to(1.0, scale_motion);
                self.release_hold();
                Command::None
            }
            // A hold-to-confirm control activates when its hold ends, not on
            // release.
            PressEvent::Activate if self.is_hold(self.pressed) => Command::None,
            PressEvent::Activate => match self.pressed {
                Some(id) if Some(id) == self.focused => self.activate(id),
                _ => Command::None,
            },
            // The contract has no context request yet, so a long press is a
            // press that does not activate.
            PressEvent::LongPress => Command::None,
        }
    }

    /// The scroll container the arrows and paging move along `axis`: the
    /// nearest one around the focused element, else the first one that can
    /// scroll (inside an open popup, only its own).
    fn scroll_container(&self, axis: Axis) -> Option<usize> {
        let layout = self.layout.as_ref()?;
        let fits = |ix: usize| self.nodes[ix].scroll == Some(axis) && !self.nodes[ix].pager;
        if let Some(stop) = self.stop_index(self.focused) {
            let mut ancestor = self.nodes[self.stops[stop].0].parent;
            while let Some(a) = ancestor {
                if fits(a as usize) {
                    return Some(a as usize);
                }
                ancestor = self.nodes[a as usize].parent;
            }
        }
        if self.nodes.iter().any(|n| n.trap) {
            return None;
        }
        (0..self.nodes.len()).find(|&ix| fits(ix) && layout.overflow[ix] > 0.0)
    }

    /// An arrow press. Focus moves to the next element in that direction
    /// when it is in view and close; otherwise the nearest scroll container
    /// scrolls a third of its viewport so content between focus stops can
    /// be read. At the end of the content, focus moves on.
    fn directional(&mut self, input: Input) -> Command {
        if let Some(p) = self.focus_pager() {
            return self.paged_directional(p, input);
        }
        let rects = self.stop_rects();
        let current = self.stop_index(self.focused);
        let candidate = focus::navigate(&rects, current, input);
        let axis = if matches!(input, Input::Up | Input::Down) {
            Axis::Vertical
        } else {
            Axis::Horizontal
        };
        let forward = matches!(input, Input::Down | Input::Right);
        if let Some(c) = self.scroll_container(axis) {
            let layout = self.layout.as_ref().expect("layout");
            let view = layout.rects[c];
            let (v0, v1) = match axis {
                Axis::Vertical => (view.y, view.y + view.height),
                Axis::Horizontal => (view.x, view.x + view.width),
            };
            let span = v1 - v0;
            let span_of = |r: Rect| match axis {
                Axis::Vertical => (r.y, r.y + r.height),
                Axis::Horizontal => (r.x, r.x + r.width),
            };
            let near = candidate.is_some_and(|s| {
                let (c0, c1) = span_of(rects[s]);
                // Distance is measured from the focused element, or from the
                // viewport's edge once focus has scrolled out of view.
                let (f0, f1) = current.map_or((v0, v0), |f| span_of(rects[f]));
                if forward {
                    let from = f1.clamp(v0, v1);
                    c0 >= v0 - 0.5 && c0 < v1 && c0 <= from + span / 3.0
                } else {
                    let from = f0.clamp(v0, v1);
                    c1 <= v1 + 0.5 && c1 > v0 && c1 >= from - span / 3.0
                }
            });
            if !near {
                let offset = self.scroll.get(&self.nodes[c].id).copied().unwrap_or(0.0);
                let step = if forward { span / 3.0 } else { -span / 3.0 };
                if self.set_scroll(c, offset + step * self.scroll_sign(c), true) {
                    self.cue(FeedbackKind::Move);
                    return Command::None;
                }
            }
        }
        if let Some(stop) = candidate {
            self.set_focus(stop);
            self.reveal_focus(true);
            self.cue(FeedbackKind::Move);
        } else if current.is_some() {
            self.cue(FeedbackKind::Blocked);
        }
        Command::None
    }

    /// L2 and R2 scroll the nearest vertical scroll container by a viewport,
    /// keeping a little of the last one in view. Focus that scrolls out of
    /// view moves to the first focus stop in view.
    fn page(&mut self, page: Page) -> Command {
        // L2 and R2 flip the pager focus is in, or the first one on screen.
        if let Some(p) = self
            .focus_pager()
            .or_else(|| self.nodes.iter().position(|n| n.pager))
        {
            let id = self.nodes[p].id;
            if let Some(pager) = self.pagers.get(&id) {
                let target = match page {
                    Page::Next => (pager.page + 1 < pager.pages.len()).then_some(pager.page + 1),
                    Page::Previous => pager.page.checked_sub(1),
                };
                match target {
                    Some(target) => {
                        self.flip(p, target, true);
                        self.cue(FeedbackKind::Move);
                    }
                    None => self.cue(FeedbackKind::Blocked),
                }
            }
            return Command::None;
        }
        let Some(c) = self.scroll_container(Axis::Vertical) else {
            return Command::None;
        };
        let overlap = self.theme.tokens(&self.prefs).space.s6;
        let view = self.layout.as_ref().expect("layout").rects[c];
        let offset = self.scroll.get(&self.nodes[c].id).copied().unwrap_or(0.0);
        let step = (view.height - overlap).max(view.height / 2.0);
        let next = if page == Page::Next {
            offset + step
        } else {
            offset - step
        };
        if !self.set_scroll(c, next, true) {
            return Command::None;
        }
        let rects = self.stop_rects();
        let inside = |r: Rect| r.y >= view.y - 0.5 && r.y + r.height <= view.y + view.height + 0.5;
        let focus_in_view = self
            .stop_index(self.focused)
            .is_some_and(|f| inside(rects[f]));
        if !focus_in_view {
            if let Some(stop) = (0..rects.len()).find(|&s| inside(rects[s])) {
                self.set_focus(stop);
            }
        }
        Command::None
    }

    /// Starts moving nodes toward the places the last layout gave them, on
    /// the theme's fast, standard transition; a spring under a touch
    /// presentation, as Switch's thumb settles; instant under reduced motion.
    fn update_moving(&mut self) {
        let Some(layout) = self.layout.as_ref() else {
            return;
        };
        let tokens = self.motion_tokens();
        let motion = match self.policy.pointer {
            Pointer::Fine => Motion::curve(tokens.durations.fast, tokens.ease_standard),
            Pointer::Coarse => {
                Motion::Spring(tokens.spring(motion_tokens::springs::STATE_CHANGE).spring())
            }
        };
        let motion = self.policy.travel(Change::Transition, motion, tokens);
        let mut seen = HashSet::new();
        for (ix, node) in self.nodes.iter().enumerate() {
            let (true, Some(p)) = (node.moves, node.parent) else {
                continue;
            };
            let (r, parent) = (layout.rects[ix], layout.rects[p as usize]);
            let target = [r.x - parent.x, r.y - parent.y, r.width];
            seen.insert(node.id);
            match self.moving.get_mut(&node.id) {
                Some(m) if m.shown.target() != target => m.shown.animate_to(target, motion),
                Some(_) => {}
                None => {
                    self.moving.insert(
                        node.id,
                        Moving {
                            shown: Animated::new(target),
                        },
                    );
                }
            }
        }
        self.moving.retain(|id, _| seen.contains(id));
    }

    /// Paint transforms: the press scale, and moving nodes on their way.
    fn transforms(&self, rects: &[Rect]) -> HashMap<usize, paint::Transform> {
        let mut out = HashMap::new();
        let index = |id: u64| self.nodes.iter().position(|n| n.id == id);
        if let Some(ix) = self.scaled.and_then(index) {
            let r = rects[ix];
            out.insert(
                ix,
                paint::Transform {
                    origin: r.center(),
                    scale: self.press_scale.value(),
                    translate: Point { x: 0.0, y: 0.0 },
                },
            );
        }
        for (&id, m) in &self.moving {
            if !m.shown.is_animating() {
                continue;
            }
            let Some(ix) = index(id) else { continue };
            let [x, y, w] = m.shown.value();
            let [tx, ty, tw] = m.shown.target();
            let r = rects[ix];
            out.insert(
                ix,
                paint::Transform {
                    origin: r.center(),
                    scale: if tw > 0.0 { w / tw } else { 1.0 },
                    translate: Point {
                        x: x - tx,
                        y: y - ty,
                    },
                },
            );
        }
        out
    }

    /// The focused element's tooltip words, unless B dismissed them.
    fn tooltip_text(&self) -> Option<&str> {
        let id = self.focused?;
        if self.tooltip_dismissed == Some(id) {
            return None;
        }
        self.nodes.iter().find(|n| n.id == id)?.tooltip.as_deref()
    }

    /// The tooltip bubble for the focused element: above it with Astryx's
    /// `space.s1` offset, below it when there is no room above, and slid
    /// sideways to stay `space.s4` inside the window. It is at most 300
    /// points wide, as Astryx's tooltip is.
    fn tooltip_paint(&self, rects: &[Rect], text: &dyn TextMeasure) -> Option<paint::Tooltip> {
        let words = self.tooltip_text()?;
        let id = self.focused?;
        let ix = self.nodes.iter().position(|n| n.id == id)?;
        let anchor = rects[ix];
        let t = self.theme.tokens(&self.prefs);
        let mut style = crate::element::themed_text(
            self.theme.theme,
            t,
            crate::components::text::type_style(t.font.body, FontFamily::Body),
        );
        style.direction = Some(if self.nodes[ix].rtl {
            TextDirection::Rtl
        } else {
            TextDirection::Ltr
        });
        let (pad_x, pad_y) = (t.space.s2, t.space.s1);
        let measured = text.measure(words, &style, Some(TOOLTIP_MAX_WIDTH - 2.0 * pad_x));
        let line = style.size * style.leading;
        let lines = (measured.height / line).round().max(1.0);
        let inner = Size {
            width: (measured.width - 0.01).ceil(),
            height: (lines * line - 0.01).ceil(),
        };
        let (w, h) = (inner.width + 2.0 * pad_x, inner.height + 2.0 * pad_y);
        let (vw, _) = self.viewport;
        let (gap, margin) = (t.space.s1, t.space.s4);
        let y = if anchor.y - gap - h >= margin {
            anchor.y - gap - h
        } else {
            anchor.y + anchor.height + gap
        };
        let centered = anchor.x + (anchor.width - w) / 2.0;
        let x = centered.min(vw - margin - w).max(margin);
        let rect = Rect::new(x.round(), y.round(), w, h);
        Some(paint::Tooltip {
            rect,
            radii: Radii::all(t.radius.container),
            fill: t.color.text_primary.into(),
            text: words.to_string(),
            text_rect: Rect::new(rect.x + pad_x, rect.y + pad_y, inner.width, inner.height),
            style,
            color: t.color.background_surface.into(),
        })
    }

    /// Thumbs for the scroll containers that scrolled recently.
    fn indicators(&self, rects: &[Rect]) -> Vec<paint::Indicator> {
        let Some(layout) = self.layout.as_ref() else {
            return Vec::new();
        };
        let tokens = self.theme.tokens(&self.prefs);
        // Astryx's scrollbar thumb is the neutral color on a clear track.
        let (thickness, inset) = (tokens.space.s1, tokens.space.s0_5);
        let mut out = Vec::new();
        for (ix, node) in self.nodes.iter().enumerate() {
            let (Some(axis), Some(indicator)) = (node.scroll, self.scroll_indicators.get(&node.id))
            else {
                continue;
            };
            let overflow = layout.overflow[ix];
            let opacity = indicator.opacity.value();
            if overflow <= 0.0 || opacity <= 0.0 {
                continue;
            }
            let view = rects[ix];
            let offset = self.shown_offset(node.id);
            let span = if axis == Axis::Vertical {
                view.height
            } else {
                view.width
            };
            let length = (span * span / (span + overflow))
                .max(tokens.space.s6)
                .min(span);
            let start = (span - length) * (offset / overflow);
            // Under RTL the vertical thumb sits on the left, the inline end,
            // and the horizontal one starts at the right.
            let rtl = node.rtl;
            let thumb = match axis {
                Axis::Vertical if rtl => {
                    Rect::new(view.x + inset, view.y + start, thickness, length)
                }
                Axis::Vertical => Rect::new(
                    view.x + view.width - inset - thickness,
                    view.y + start,
                    thickness,
                    length,
                ),
                Axis::Horizontal if rtl => Rect::new(
                    view.x + view.width - start - length,
                    view.y + view.height - inset - thickness,
                    length,
                    thickness,
                ),
                Axis::Horizontal => Rect::new(
                    view.x + start,
                    view.y + view.height - inset - thickness,
                    length,
                    thickness,
                ),
            };
            out.push(paint::Indicator {
                thumb,
                opacity,
                color: tokens.color.neutral.into(),
            });
        }
        out
    }

    /// Moves every animation and gesture to `motion_ms`, and runs what a
    /// held button does over time (repeats, paging, long press).
    fn tick_motion(&mut self, dt: f32) -> Command {
        let tokens = self.motion_tokens();
        self.shown.retain(|_, a| a.tick(dt));
        let now = self.motion_ms;
        for indicator in self.scroll_indicators.values_mut() {
            if now >= indicator.hold_until_ms && indicator.opacity.target() > 0.0 {
                let fade = Motion::curve(tokens.durations.fast, tokens.ease_exit);
                indicator
                    .opacity
                    .animate_to(0.0, self.policy.travel(Change::Transition, fade, tokens));
                // The fade starts at the end of the hold, not at this frame's start.
                indicator
                    .opacity
                    .tick(((now - indicator.hold_until_ms) / 1000.0).min(f64::from(dt)) as f32);
            } else {
                indicator.opacity.tick(dt);
            }
        }
        self.scroll_indicators
            .retain(|_, i| i.opacity.value() > 0.0 || i.opacity.is_animating());
        self.highlight.tick(dt);
        if !self.press.is_down() && !self.highlight.wants_frames() {
            self.pressed = None;
        }
        if let Some((_, a)) = self.hold_release.as_mut() {
            if !a.tick(dt) {
                self.hold_release = None;
            }
        }
        self.press_scale.tick(dt);
        if !self.press_scale.is_animating() && self.press_scale.value() == 1.0 {
            self.scaled = None;
        }
        for m in self.moving.values_mut() {
            m.shown.tick(dt);
        }
        for camera in self.cameras.values_mut() {
            camera.tick(dt);
        }
        for a in self.anims.values_mut() {
            a.tick(dt);
        }
        for e in &mut self.exits {
            e.opacity.tick(dt);
            e.scale.tick(dt);
            e.translate.tick(dt);
        }
        self.exits.retain(|e| {
            e.opacity.is_animating() || e.scale.is_animating() || e.translate.is_animating()
        });
        for r in self.resizes.values_mut() {
            if r.shown.tick(dt) {
                self.relayout = true;
            }
        }
        let mut command = Command::None;
        for event in self.press.tick(now) {
            command = self.press_event(event);
        }
        command = self.fire_timers(now).unwrap_or(command);
        // A cue at each quarter of a hold's time, before the end.
        if let Some(h) = self.hold.filter(|h| !h.done) {
            let quarter = (((now - h.started_ms) as f32 / h.duration_ms) * 4.0)
                .floor()
                .clamp(0.0, 3.0) as u8;
            if quarter > h.quarter {
                self.hold = Some(Hold { quarter, ..h });
                self.cue(FeedbackKind::Progress);
            }
        }
        let held_out = self
            .hold
            .filter(|h| !h.done && (now - h.started_ms) as f32 >= h.duration_ms);
        if let Some(h) = held_out {
            self.hold = Some(Hold { done: true, ..h });
            self.repaint = true;
            if self.focused == Some(h.id) {
                command = self.activate(h.id);
            }
        }
        if let Some(input) = self.repeats.poll(now) {
            if !(matches!(input, Input::Left | Input::Right) && self.move_caret(input)) {
                command = self.directional(input);
            }
        }
        if let Some(page) = self.paging.poll(now) {
            self.page(page);
        }
        command
    }

    fn activate(&mut self, id: u64) -> Command {
        let Some(node) = self.nodes.iter().find(|n| n.id == id) else {
            return Command::None;
        };
        let Some(interact) = &node.interact else {
            return Command::None;
        };
        if interact.disabled || interact.busy {
            self.cue(FeedbackKind::Blocked);
            return Command::None;
        }
        let completed_hold =
            interact.hold.is_some() && self.hold.is_some_and(|h| h.id == id && h.done);
        let msg = interact.on_activate.clone();
        let href = interact.href.clone();
        let popup = interact.popup;
        self.cue(if completed_hold {
            FeedbackKind::Success
        } else {
            FeedbackKind::Select
        });
        let (msg, href) = (msg, href);
        let interact_popup = popup;
        match interact_popup {
            PopupAction::None => {}
            PopupAction::Toggle => {
                self.popup = if self.popup == Some(id) {
                    None
                } else {
                    Some(id)
                };
                self.stale = true;
            }
            PopupAction::Close => self.close_popup(),
        }
        let command = match msg {
            Some(msg) => self.send(msg),
            None => Command::None,
        };
        // A link opens after its press handler, unless the app asked for
        // something else.
        match (command, href) {
            (Command::None, Some(href)) => Command::OpenUrl(href),
            (command, _) => command,
        }
    }

    /// The focused text field: its node index and edit binding.
    fn focused_edit(&self) -> Option<(usize, &Edit<A::Msg>)> {
        let id = self.focused?;
        let ix = self.nodes.iter().position(|n| n.id == id)?;
        let interact = self.nodes[ix].interact.as_ref()?;
        if interact.disabled {
            return None;
        }
        Some((ix, interact.edit.as_ref()?))
    }

    /// The focused field's value: the one typed since the last rebuild, or
    /// the one its view shows.
    fn field_value(&self, id: u64, edit: &Edit<A::Msg>) -> String {
        match &self.typed {
            Some((typed_id, value)) if *typed_id == id => value.clone(),
            _ => edit.value.clone(),
        }
    }

    /// The caret of the focused field, in characters, within its value.
    fn caret(&self, id: u64, value: &str) -> usize {
        let len = value.chars().count();
        self.carets.get(&id).copied().unwrap_or(len).min(len)
    }

    /// Moves the caret of a focused text field one character. Returns false
    /// at either end, so the arrow moves focus instead.
    fn move_caret(&mut self, input: Input) -> bool {
        let Some((ix, edit)) = self.focused_edit() else {
            return false;
        };
        let id = self.nodes[ix].id;
        let value = self.field_value(id, edit);
        let caret = self.caret(id, &value);
        let len = value.chars().count();
        let next = match input {
            Input::Left if caret > 0 => caret - 1,
            Input::Right if caret < len => caret + 1,
            Input::Home => 0,
            Input::End => len,
            _ => return false,
        };
        self.carets.insert(id, next);
        true
    }

    /// Where to draw the focused field's caret, and how far its text
    /// scrolls so the caret stays inside the field.
    fn caret_paint(&self, text: &dyn TextMeasure) -> Option<paint::Caret> {
        let (ix, edit) = self.focused_edit()?;
        let layout = self.layout.as_ref()?;
        let node = &self.nodes[edit.text_node as usize];
        let crate::element::Draw::Text(t) = &node.draw else {
            return None;
        };
        let id = self.nodes[ix].id;
        let value = self.field_value(id, edit);
        let caret = self.caret(id, &value);
        let shown = |s: &str| {
            if edit.masked {
                "\u{2022}".repeat(s.chars().count())
            } else {
                s.to_string()
            }
        };
        let prefix = shown(&value.chars().take(caret).collect::<String>());
        let suffix = shown(&value.chars().skip(caret).collect::<String>());
        let width = |s: &str| text.measure(s, &t.style, None).width;
        // Composing text shows inline at the caret, underlined, with the
        // caret at the input method's cursor.
        let composing = match &self.composing {
            Some((cid, pre, cursor)) if *cid == id => Some((shown(pre), *cursor as usize)),
            _ => None,
        };
        let (x, override_text, underline) = match &composing {
            Some((pre, cursor)) => {
                let cut = pre
                    .char_indices()
                    .map(|(i, _)| i)
                    .chain([pre.len()])
                    .find(|&i| i >= *cursor)
                    .unwrap_or(pre.len());
                let all = format!("{prefix}{pre}{suffix}");
                let start = width(&prefix);
                (
                    width(&format!("{prefix}{}", &pre[..cut])),
                    Some(all),
                    Some((start, width(&format!("{prefix}{pre}")))),
                )
            }
            None => (width(&prefix), None, None),
        };
        let area = node
            .parent
            .map_or(layout.rects[ix], |p| layout.rects[p as usize]);
        let caret_width = 1.0;
        let rect = layout.rects[edit.text_node as usize];
        // The value slides so the caret stays inside the field: left past
        // its end, or right past its start, where a right-to-left field's
        // long value overflows.
        let at = rect.x + x;
        let shift = if at + caret_width > area.x + area.width {
            at + caret_width - (area.x + area.width)
        } else if at < area.x {
            at - area.x
        } else {
            0.0
        };
        let line = (t.style.size * t.style.leading - 0.01).ceil();
        let top = rect.y + (rect.height - line) / 2.0;
        Some(paint::Caret {
            text_node: edit.text_node as usize,
            shift,
            rect: Rect::new((rect.x + x - shift).round(), top, caret_width, line),
            color: edit.caret,
            text: override_text.map(|s| {
                let w = width(&s);
                (s, w)
            }),
            underline: underline
                .map(|(a, b)| Rect::new(rect.x + a - shift, top + line - 2.0, b - a, 1.0)),
        })
    }

    /// Gives the focused field's accessibility node its caret, as a byte
    /// offset into the value it exposes.
    fn mark_text_selection(&mut self) {
        let Some((ix, edit)) = self.focused_edit() else {
            return;
        };
        let id = self.nodes[ix].id;
        let value = self.field_value(id, edit);
        let caret = self.caret(id, &value);
        let bytes: usize = if edit.masked {
            caret * '\u{2022}'.len_utf8()
        } else {
            value.chars().take(caret).map(char::len_utf8).sum()
        };
        fn find(n: &mut A11yNode, id: u64) -> Option<&mut A11yNode> {
            if n.id == id {
                return Some(n);
            }
            n.children.iter_mut().find_map(|c| find(c, id))
        }
        if let Some(node) = self.a11y.as_mut().and_then(|root| find(root, id)) {
            node.text_selection = Some((bytes as u32, bytes as u32));
        }
    }

    /// `motion` for `channel` under the person's and the theme's motion
    /// settings: travel lands at once under Reduce Motion; opacity may
    /// cross-fade.
    fn plan_motion(
        &self,
        channel: Option<crate::motion::Channel>,
        motion: Motion,
        change: Change,
    ) -> Motion {
        use astryx_motion::Plan;
        match self.policy.plan(change, self.motion_tokens()) {
            Plan::Animate => motion,
            Plan::Instant => Motion::Instant,
            Plan::CrossFade {
                duration_ms,
                easing,
            } if channel == Some(crate::motion::Channel::Opacity) => {
                Motion::curve(duration_ms, easing)
            }
            Plan::CrossFade { .. } => Motion::Instant,
        }
    }

    /// Brings animations up to date with a rebuild from `old`: new targets
    /// retarget, new nodes enter, and nodes with an exit spec that went
    /// away leave through an exit copy.
    fn sync_motion(&mut self, old: &[Node<A::Msg>]) {
        let mut live = HashSet::new();
        let mut plans = Vec::new();
        for node in &self.nodes {
            for a in &node.anims {
                let key = (node.id, a.channel);
                live.insert(key);
                plans.push((key, *a));
            }
        }
        for (key, a) in plans {
            let motion = self.plan_motion(Some(a.channel), a.motion, a.change);
            match self.anims.get_mut(&key) {
                None => {
                    let mut v = Animated::new(a.from.unwrap_or(a.to));
                    if a.from.is_some() {
                        v.animate_to(a.to, motion);
                    }
                    self.anims.insert(key, v);
                }
                Some(v) if v.target() != a.to => v.animate_to(a.to, motion),
                Some(_) => {}
            }
        }
        self.anims.retain(|k, _| live.contains(k));
        let ids: HashSet<u64> = self.nodes.iter().map(|n| n.id).collect();
        for node in old {
            let Some(exit) = node.exit else { continue };
            if ids.contains(&node.id) {
                continue;
            }
            let Some((commands, rect)) = self.captured.remove(&node.id) else {
                continue;
            };
            // One copy per node: a second exit replaces the first.
            self.exits.retain(|e| e.id != node.id);
            let fade = self.plan_motion(
                Some(crate::motion::Channel::Opacity),
                exit.motion,
                exit.change,
            );
            let travel = self.plan_motion(None, exit.motion, exit.change);
            let mut opacity = Animated::new(1.0);
            opacity.animate_to(exit.opacity, fade);
            let mut scale = Animated::new(1.0);
            scale.animate_to(exit.scale, travel);
            let mut translate = Animated::new((0.0, 0.0));
            translate.animate_to(exit.translate, travel);
            if opacity.is_animating() || scale.is_animating() || translate.is_animating() {
                self.exits.push(ExitCopy {
                    id: node.id,
                    commands,
                    rect,
                    opacity,
                    scale,
                    translate,
                });
            }
        }
        self.captured.retain(|id, _| ids.contains(id));
    }

    /// Animates the heights of nodes with a resize spec toward their newly
    /// laid-out heights. Returns true when it laid out again at animated
    /// heights.
    fn resize_layout(&mut self, text: &dyn TextMeasure) -> bool {
        let Some(layout) = self.layout.as_ref() else {
            return false;
        };
        let mut overrides = Vec::new();
        let mut live = HashSet::new();
        let mut starts = Vec::new();
        for (ix, node) in self.nodes.iter().enumerate() {
            let Some(resize) = node.resize else { continue };
            live.insert(node.id);
            let natural = layout.boxes.get(ix).map_or(0.0, |b| b.height);
            starts.push((ix, node.id, natural, resize));
        }
        for (ix, id, natural, resize) in starts {
            let motion = self.plan_motion(None, resize.motion, resize.change);
            let track = self.resizes.entry(id).or_insert(ResizeTrack {
                natural,
                shown: Animated::new(natural),
            });
            if (track.natural - natural).abs() > 0.25 {
                track.natural = natural;
                track.shown.animate_to(natural, motion);
            }
            if track.shown.is_animating() {
                overrides.push((ix, track.shown.value()));
            }
        }
        self.resizes.retain(|id, _| live.contains(id));
        if overrides.is_empty() {
            return false;
        }
        let layout = self.layout.as_mut().expect("layout");
        for &(ix, h) in &overrides {
            layout.override_height(&self.nodes, ix, Some(h));
        }
        layout.compute(&self.nodes, self.viewport, text);
        // The next frame measures the natural heights again.
        for &(ix, _) in &overrides {
            layout.override_height(&self.nodes, ix, None);
        }
        true
    }

    /// The topmost layer root whose spec passes `f`.
    fn top_layer(&self, f: impl Fn(&crate::layer::LayerSpec<A::Msg>) -> bool) -> Option<usize> {
        self.nodes
            .iter()
            .rposition(|n| n.layer.as_ref().is_some_and(&f))
    }

    /// Sends layer `ix`'s dismissal message for `reason`.
    fn dismiss(&mut self, ix: usize, reason: DismissReason) -> Command {
        let msg = self.nodes[ix]
            .layer
            .as_ref()
            .and_then(|l| l.on_dismiss.as_ref())
            .map(|f| f(reason));
        match msg {
            Some(msg) => self.send(msg),
            None => Command::None,
        }
    }

    /// Notes layers that opened and closed in a rebuild from `old`: a new
    /// layer remembers the focus it opened over; a closed layer that held
    /// focus gives it back.
    fn track_layers(&mut self, old: &[Node<A::Msg>]) {
        let now: HashSet<u64> = self
            .nodes
            .iter()
            .filter(|n| n.layer.is_some())
            .map(|n| n.id)
            .collect();
        for n in self.nodes.iter().filter(|n| n.layer.is_some()) {
            self.layer_openers.entry(n.id).or_insert(self.focused);
        }
        let closed: Vec<u64> = self
            .layer_openers
            .keys()
            .copied()
            .filter(|id| !now.contains(id))
            .collect();
        for id in closed {
            let opener = self.layer_openers.remove(&id).flatten();
            let restore = old
                .iter()
                .find(|n| n.id == id)
                .and_then(|n| n.layer.as_ref())
                .is_some_and(|l| l.restore_focus);
            let focus_gone = self
                .focused
                .is_none_or(|f| !self.nodes.iter().any(|n| n.id == f));
            if restore && focus_gone {
                if let Some(opener) = opener.filter(|o| self.nodes.iter().any(|n| n.id == *o)) {
                    self.focused = Some(opener);
                    self.reveal = true;
                }
            }
        }
    }

    /// Places every layer against its anchor, in order, so a layer anchored
    /// inside another layer follows it.
    fn place_layers(&mut self) {
        let Some(layout) = self.layout.as_mut() else {
            return;
        };
        if !self.nodes.iter().any(|n| n.layer.is_some()) {
            if !layout.layers.is_empty() {
                layout.layers.clear();
                layout.place(&self.nodes, &self.scroll);
            }
            return;
        }
        layout.layers.clear();
        layout.place(&self.nodes, &self.scroll);
        let base = layout.rects.clone();
        let viewport = Rect::new(0.0, 0.0, self.viewport.0, self.viewport.1);
        let mut offsets: HashMap<u64, (f32, f32)> = HashMap::new();
        // The offset of the layer around node `ix`, if one was placed.
        let enclosing = |nodes: &[Node<A::Msg>], offsets: &HashMap<u64, (f32, f32)>, ix: usize| {
            let mut at = Some(ix);
            while let Some(i) = at {
                if nodes[i].layer.is_some() {
                    return offsets.get(&nodes[i].id).copied().unwrap_or((0.0, 0.0));
                }
                at = nodes[i].parent.map(|p| p as usize);
            }
            (0.0, 0.0)
        };
        for (ix, node) in self.nodes.iter().enumerate() {
            let Some(spec) = &node.layer else {
                continue;
            };
            let anchor_ix = match &spec.anchor {
                LayerAnchor::None | LayerAnchor::Point(_) => None,
                LayerAnchor::Name(name) => self
                    .nodes
                    .iter()
                    .position(|n| n.name.as_deref() == Some(name.as_str())),
                LayerAnchor::Id(id) => self.nodes.iter().position(|n| n.id == *id),
            };
            let anchor = match &spec.anchor {
                LayerAnchor::Point(p) => Some(Rect::new(p.x, p.y, 0.0, 0.0)),
                _ => anchor_ix.map(|a| {
                    let (dx, dy) = enclosing(&self.nodes, &offsets, a);
                    let r = base[a];
                    Rect::new(r.x + dx, r.y + dy, r.width, r.height)
                }),
            };
            let size = Size {
                width: base[ix].width,
                height: base[ix].height,
            };
            let at = (spec.place)(&PlaceInput {
                anchor,
                size,
                viewport,
                rtl: node.rtl,
            });
            offsets.insert(node.id, (at.x, at.y));
        }
        layout.layers = offsets;
        layout.place(&self.nodes, &self.scroll);
    }

    /// Reports hover changes for nodes with a hover spec, after their
    /// delays. A layer anchored to such a node counts as inside it.
    fn track_hover(&mut self, rects: &[Rect]) {
        let now = self.motion_ms;
        let focus_visible = !self.focus_by_pointer;
        let mut sends = Vec::new();
        let mut seen = HashSet::new();
        for (ix, node) in self.nodes.iter().enumerate() {
            let Some(spec) = &node.hover else {
                continue;
            };
            seen.insert(node.id);
            let over = |r: Rect| self.pointer.is_some_and(|p| contains(r, p));
            let mut inside = rects.get(ix).is_some_and(|&r| over(r));
            if !inside {
                inside = self.nodes.iter().enumerate().any(|(lx, l)| {
                    l.layer.as_ref().is_some_and(|spec| match &spec.anchor {
                        LayerAnchor::Id(id) => *id == node.id,
                        LayerAnchor::Name(n) => node.name.as_deref() == Some(n.as_str()),
                        _ => false,
                    }) && rects.get(lx).is_some_and(|&r| over(r))
                });
            }
            let focused = spec.focus && focus_visible && self.focus_within(ix);
            let track = self.hovers.entry(node.id).or_default();
            if focused {
                track.inside_since = None;
                track.outside_since = None;
                if !track.on {
                    track.on = true;
                    sends.push((spec.on_change)(true));
                }
                continue;
            }
            if inside {
                track.outside_since = None;
                let since = *track.inside_since.get_or_insert(now);
                if !track.on && now - since >= spec.enter_ms {
                    track.on = true;
                    sends.push((spec.on_change)(true));
                }
            } else {
                track.inside_since = None;
                let since = *track.outside_since.get_or_insert(now);
                if track.on && now - since >= spec.leave_ms {
                    track.on = false;
                    sends.push((spec.on_change)(false));
                }
            }
        }
        self.hovers.retain(|id, _| seen.contains(id));
        for msg in sends {
            let command = self.send(msg);
            self.pending = std::mem::take(&mut self.pending).and(command);
        }
    }

    /// Whether focus is on node `ix` or inside it.
    fn focus_within(&self, ix: usize) -> bool {
        let Some(f) = self.focused else {
            return false;
        };
        let Some(mut at) = self.nodes.iter().position(|n| n.id == f) else {
            return false;
        };
        loop {
            if at == ix {
                return true;
            }
            match self.nodes[at].parent {
                Some(p) => at = p as usize,
                None => return false,
            }
        }
    }

    /// Whether a hover report is waiting on its delay.
    fn hover_pending(&self) -> bool {
        self.hovers
            .values()
            .any(|t| (t.on && t.outside_since.is_some()) || (!t.on && t.inside_since.is_some()))
    }

    /// Closes the open popup and puts focus back on its owner.
    fn close_popup(&mut self) {
        if let Some(owner) = self.popup.take() {
            self.focused = Some(owner);
            self.reveal = true;
            self.stale = true;
        }
    }
}

impl<A: App> Runtime<A> {
    fn handle_input(&mut self, input: Input, phase: InputPhase) -> Command {
        self.repaint = true;
        if phase == InputPhase::Pressed
            && matches!(
                input,
                Input::Up
                    | Input::Down
                    | Input::Left
                    | Input::Right
                    | Input::Next
                    | Input::Previous
            )
        {
            self.focus_by_pointer = false;
        }
        match (input, phase) {
            (Input::A, _) => {
                let mut command = Command::None;
                for event in self.press.input(gesture_phase(phase), self.motion_ms) {
                    let c = self.press_event(event);
                    if c != Command::None {
                        command = c;
                    }
                }
                command
            }
            (Input::Left | Input::Right, InputPhase::Pressed)
                if self.focused_edit().is_some()
                    && self
                        .repeats
                        .input(input, Phase::Pressed, self.motion_ms)
                        .is_some() =>
            {
                if !self.move_caret(input) {
                    return self.directional(input);
                }
                Command::None
            }
            (Input::Home | Input::End, InputPhase::Pressed) if self.move_caret(input) => {
                Command::None
            }
            (Input::Backspace, InputPhase::Pressed) if self.focused_edit().is_some() => {
                self.text_input("\u{8}")
            }
            (Input::Delete, InputPhase::Pressed) if self.focused_edit().is_some() => {
                self.text_input("\u{7f}")
            }
            (Input::Up | Input::Down | Input::Left | Input::Right, _) => {
                match self
                    .repeats
                    .input(input, gesture_phase(phase), self.motion_ms)
                {
                    Some(input) => self.directional(input),
                    None => Command::None,
                }
            }
            (Input::L2 | Input::R2, _) => {
                let page = if input == Input::R2 {
                    Page::Next
                } else {
                    Page::Previous
                };
                match self
                    .paging
                    .input(page, gesture_phase(phase), self.motion_ms)
                {
                    Some(page) => self.page(page),
                    None => Command::None,
                }
            }
            (Input::Next | Input::Previous, InputPhase::Pressed) => {
                if let Some(stop) = self.next_in_order(input) {
                    self.set_focus(stop);
                    self.reveal_focus(true);
                    self.cue(FeedbackKind::Move);
                }
                Command::None
            }
            (Input::B, InputPhase::Pressed) if self.top_layer(|l| l.escape_dismiss).is_some() => {
                let ix = self.top_layer(|l| l.escape_dismiss).expect("checked");
                self.cue(FeedbackKind::Back);
                self.dismiss(ix, DismissReason::Escape)
            }
            (Input::B, InputPhase::Pressed) if self.popup.is_some() => {
                self.close_popup();
                self.cue(FeedbackKind::Back);
                Command::None
            }
            // A shown tooltip takes B first, as the topmost layer takes Escape.
            (Input::B, InputPhase::Pressed) if self.tooltip_text().is_some() => {
                self.tooltip_dismissed = self.focused;
                self.cue(FeedbackKind::Back);
                Command::None
            }
            (_, InputPhase::Pressed) => match self.app.on_input(input) {
                Some(msg) => {
                    if input == Input::B {
                        self.cue(FeedbackKind::Back);
                    }
                    self.send(msg)
                }
                None => Command::None,
            },
            _ => Command::None,
        }
    }

    fn handle_a11y_action(&mut self, id: u64, action: A11yAction) -> Command {
        self.repaint = true;
        self.repeats.cancel();
        if action == A11yAction::PressCancel {
            self.end_press();
            self.pressed = None;
            return Command::None;
        }
        let Some(stop) = self.stop_index(Some(id)) else {
            return Command::None;
        };
        if self.focused != Some(id) {
            self.set_focus(stop);
            self.reveal_focus(true);
        }
        match action {
            A11yAction::Focus | A11yAction::PressCancel => Command::None,
            A11yAction::Activate => self.activate(id),
            // A pointer press runs the same press as A, so press feedback,
            // long presses, and holds behave alike.
            A11yAction::PressStart => {
                self.focus_by_pointer = true;
                self.handle_input(Input::A, InputPhase::Pressed)
            }
            A11yAction::PressEnd => self.handle_input(Input::A, InputPhase::Released),
        }
    }
}

impl<A: App> Surface for Runtime<A> {
    fn pointer(&mut self, event: PointerEvent) -> PointerOutcome {
        // Hover stays on `pointer_move`; this path carries presses.
        if event.phase != PointerPhase::Down {
            return PointerOutcome::default();
        }
        // A press outside the topmost layer closes it, if it closes on
        // outside presses, and goes no further: the same press neither
        // reaches the page nor closes the layer under it.
        let Some(top) = self.top_layer(|_| true) else {
            return PointerOutcome::default();
        };
        let inside = self
            .layout
            .as_ref()
            .and_then(|l| l.rects.get(top))
            .is_some_and(|r| contains(*r, event.at));
        let spec = self.nodes[top].layer.as_ref().expect("layer");
        if inside {
            return PointerOutcome::default();
        }
        if spec.light_dismiss && spec.on_dismiss.is_some() {
            let command = self.dismiss(top, DismissReason::OutsidePress);
            let command = command.and(std::mem::take(&mut self.pending));
            return PointerOutcome {
                command,
                handled: true,
            };
        }
        // A modal layer's page is inert.
        PointerOutcome {
            command: Command::None,
            handled: spec.modal,
        }
    }

    fn pointer_move(&mut self, at: Option<Point>) {
        if self.pointer != at {
            self.pointer = at;
            self.repaint = true;
        }
    }

    fn input(&mut self, input: Input, phase: InputPhase) -> Command {
        let command = match self.capture(input, phase) {
            Some(command) => command,
            None => self.handle_input(input, phase),
        };
        let followed = self.report_focus();
        let command = if command == Command::None {
            followed
        } else {
            command
        };
        command.and(std::mem::take(&mut self.pending))
    }

    fn take_command(&mut self) -> Command {
        std::mem::take(&mut self.pending)
    }

    fn a11y_action(&mut self, id: u64, action: A11yAction) -> Command {
        let command = self.handle_a11y_action(id, action);
        let followed = self.report_focus();
        let command = if command == Command::None {
            followed
        } else {
            command
        };
        command.and(std::mem::take(&mut self.pending))
    }

    fn frame(&mut self, frame: &FrameInfo, text: &dyn TextMeasure) -> DisplayList {
        // Under reduced motion spinners hold still.
        if !self.prefs.reduced_motion {
            self.time += frame.dt;
        }
        self.motion_ms += f64::from(frame.dt) * 1000.0;
        self.repaint = false;
        // A command from a long press or repeat has no caller to return to
        // in a frame; the effect lands in the app's state.
        let command = self.tick_motion(frame.dt);
        self.pending = std::mem::take(&mut self.pending).and(command);
        let size = frame.logical_size();
        let viewport = (size.width, size.height);
        let mut watch = Stopwatch::start();
        self.timings = FrameTimings::default();
        // Where the focused element is drawn now, so a rebuild that moves
        // it (a theme with other sizes) can keep it in place.
        let anchor = self.focused_rect().zip(self.focused);
        if self.stale || self.app.theme() != self.theme || self.app.direction() != self.direction {
            self.rebuild();
        }
        // New font faces can change every measurement, so cached sizes go.
        if text.generation() != self.measure_generation {
            self.measure_generation = text.generation();
            self.layout = Some(Layout::new(&self.nodes));
            self.relayout = true;
        }
        self.timings.rebuild = watch.lap();
        let laid_out = self.relayout || viewport != self.viewport;
        if laid_out {
            self.viewport = viewport;
            self.layout
                .as_mut()
                .expect("layout")
                .compute(&self.nodes, viewport, text);
            self.relayout = false;
            if self.resize_layout(text) {
                // Lay out again next frame, while the height moves.
                self.relayout = true;
            }
            self.layout
                .as_mut()
                .expect("layout")
                .place(&self.nodes, &self.scroll);
        }
        // New nodes need their page labels and focus stops even when the
        // geometry they share with the old ones is kept.
        if laid_out || std::mem::take(&mut self.rebuilt) {
            self.update_pagers();
            self.settle_focus();
        }
        self.timings.laid_out = laid_out;
        if laid_out {
            // Only the same element stays in place; new focus is revealed.
            if let Some((old, _)) = anchor.filter(|&(_, id)| self.focused == Some(id)) {
                self.keep_in_place(old);
            }
        }
        if std::mem::take(&mut self.reveal) {
            self.reveal_focus(false);
        }
        self.follow_cameras();
        self.place_layers();
        // A rebuild can move focus; the app hears of it too.
        let _ = self.report_focus();
        self.update_moving();
        self.animating_visible = self.animation_visible();
        self.timings.layout = watch.lap();
        let modal = self
            .nodes
            .iter()
            .rposition(|n| n.layer.as_ref().is_some_and(|l| l.modal));
        self.a11y = a11y::tree(
            &self.nodes,
            &self.layout.as_ref().expect("layout").rects,
            self.focused,
            modal,
        );
        self.timings.a11y = watch.lap();
        self.mark_text_selection();
        let layout = self.layout.as_ref().expect("layout");
        let rects = if self.shown.is_empty() {
            layout.rects.clone()
        } else {
            let shown: HashMap<u64, f32> = self
                .scroll
                .keys()
                .map(|&id| (id, self.shown_offset(id)))
                .collect();
            layout.placed(&self.nodes, &shown)
        };
        self.track_hover(&rects);
        let caret = self.caret_paint(text);
        let indicators = self.indicators(&rects);
        let tooltip = self.tooltip_paint(&rects, text);
        let mut transforms = self.transforms(&rects);
        let mut opacities = HashMap::new();
        if !self.anims.is_empty() {
            let index: HashMap<u64, usize> = self
                .nodes
                .iter()
                .enumerate()
                .map(|(i, n)| (n.id, i))
                .collect();
            use crate::motion::Channel;
            for (&(id, channel), a) in &self.anims {
                let Some(&ix) = index.get(&id) else { continue };
                let v = a.value();
                if channel == Channel::Opacity {
                    if v < 1.0 {
                        opacities.insert(ix, v.max(0.0));
                    }
                    continue;
                }
                let identity = match channel {
                    Channel::Scale => v == 1.0,
                    _ => v == 0.0,
                };
                if identity {
                    continue;
                }
                let t = transforms.entry(ix).or_insert(paint::Transform {
                    origin: rects[ix].center(),
                    scale: 1.0,
                    translate: Point { x: 0.0, y: 0.0 },
                });
                match channel {
                    Channel::TranslateX => t.translate.x += v,
                    Channel::TranslateY => t.translate.y += v,
                    Channel::Scale => t.scale *= v,
                    Channel::Opacity => {}
                }
            }
        }
        let exits = self
            .exits
            .iter()
            .map(|e| {
                let (x, y) = e.translate.value();
                (
                    e.commands.clone(),
                    e.rect,
                    e.opacity.value().clamp(0.0, 1.0),
                    e.scale.value(),
                    Point { x, y },
                )
            })
            .collect();
        let (page_clips, page_marks) = self.pager_paint(&rects);
        // The topmost enabled focus stop under the pointer shows hover.
        let hovered = self.pointer.and_then(|p| {
            self.stops.iter().rev().find_map(|&(ix, _)| {
                let r = rects.get(ix)?;
                let enabled = self.nodes[ix]
                    .interact
                    .as_ref()
                    .is_some_and(|i| !i.disabled && !i.busy);
                let inside =
                    p.x >= r.x && p.x < r.x + r.width && p.y >= r.y && p.y < r.y + r.height;
                (enabled && inside).then_some(self.nodes[ix].id)
            })
        });
        let backdrop = modal.and_then(|ix| {
            let color = self.nodes[ix].layer.as_ref()?.backdrop?;
            Some((ix, color))
        });
        let state = PaintState {
            focus_visible: !self.focus_by_pointer || self.focused_edit().is_some(),
            backdrop,
            focused: self.focused,
            pressed: self.pressed,
            hovered,
            hover_fill: self.theme.tokens(&self.prefs).color.overlay_hover.into(),
            hover_border: self
                .theme
                .tokens(&self.prefs)
                .color
                .border_emphasized
                .into(),
            press_strength: self.highlight.strength(),
            hold: self.hold_fill().and_then(|(id, p)| {
                let ix = self.nodes.iter().position(|n| n.id == id)?;
                // Under reduced motion the fill moves in quarter steps.
                let p = if self.policy.reduces(self.motion_tokens()) {
                    (p * 4.0).floor() / 4.0
                } else {
                    p
                };
                Some((ix, p))
            }),
            time: self.time,
            scale: frame.scale,
            caret,
            indicators,
            tooltip,
            transforms,
            page_clips,
            page_marks,
            opacities,
            exits,
        };
        let layout = self.layout.as_ref().expect("layout");
        let mut captures = Vec::new();
        let list = paint::paint(
            &self.nodes,
            layout,
            &rects,
            &state,
            self.theme.tokens(&self.prefs).color.background_body.into(),
            &mut captures,
        );
        for (ix, start, end) in captures {
            self.captured.insert(
                self.nodes[ix].id,
                (list.commands[start..end].to_vec(), rects[ix]),
            );
        }
        self.timings.paint = watch.lap();
        list
    }

    fn scroll(&mut self, at: Point, delta: Point) -> bool {
        self.repaint = true;
        let Some(layout) = self.layout.as_ref() else {
            return false;
        };
        let inside =
            |r: Rect| at.x >= r.x && at.x < r.x + r.width && at.y >= r.y && at.y < r.y + r.height;
        // The innermost scroll container under the point comes last in
        // reading order; its ancestors follow it outward.
        let mut chain: Vec<usize> = (0..self.nodes.len())
            .filter(|&ix| self.nodes[ix].scroll.is_some() && inside(layout.rects[ix]))
            .collect();
        chain.reverse();
        for ix in chain {
            let along = match self.nodes[ix].scroll {
                Some(Axis::Vertical) => delta.y,
                _ => delta.x * self.scroll_sign(ix),
            };
            if along == 0.0 {
                continue;
            }
            let id = self.nodes[ix].id;
            let current = self.scroll.get(&id).copied().unwrap_or(0.0);
            let target =
                (current + along).clamp(0.0, self.layout.as_ref().expect("layout").overflow[ix]);
            if (target - current).abs() < 0.01 {
                continue;
            }
            // Wheels and trackpads already move smoothly, so the offset
            // follows them directly; the indicator still shows.
            self.scroll.insert(id, target);
            self.shown.remove(&id);
            self.show_indicator(id);
            if let Some(pager) = self.pagers.get_mut(&id) {
                pager.free = true;
            }
            if let Some(layout) = self.layout.as_mut() {
                layout.place(&self.nodes, &self.scroll);
            }
            return true;
        }
        false
    }

    fn text_compose(&mut self, text: &str, cursor: u32) {
        self.repaint = true;
        let Some((ix, _)) = self.focused_edit() else {
            return;
        };
        let id = self.nodes[ix].id;
        self.composing =
            (!text.is_empty()).then(|| (id, text.to_string(), cursor.min(text.len() as u32)));
    }

    fn text_input(&mut self, text: &str) -> Command {
        self.repaint = true;
        self.composing = None;
        let Some((ix, edit)) = self.focused_edit() else {
            return Command::None;
        };
        let id = self.nodes[ix].id;
        let (old, on_change) = (self.field_value(id, edit), edit.on_change.clone());
        let mut caret = self.caret(id, &old);
        let mut value: Vec<char> = old.chars().collect();
        for ch in text.chars() {
            match ch {
                // Backspace and Delete arrive as their control characters.
                '\u{8}' if caret > 0 => {
                    caret -= 1;
                    value.remove(caret);
                }
                '\u{7f}' if caret < value.len() => {
                    value.remove(caret);
                }
                // A single-line field takes no other control characters.
                c if c.is_control() => {}
                c => {
                    value.insert(caret, c);
                    caret += 1;
                }
            }
        }
        let value: String = value.into_iter().collect();
        self.carets.insert(id, caret);
        if value == old {
            return Command::None;
        }
        let msg = on_change(value.clone());
        // Keystrokes before the next frame build on this value.
        self.typed = Some((id, value));
        self.send(msg)
    }

    fn take_image_updates(&mut self) -> Vec<(ImageId, Option<ImageData>)> {
        std::mem::take(&mut self.image_updates)
    }

    fn take_sound_updates(&mut self) -> Vec<(SoundId, Option<std::sync::Arc<[u8]>>)> {
        std::mem::take(&mut self.sound_updates)
    }

    fn set_preferences(&mut self, preferences: &Preferences) {
        if *preferences != self.prefs {
            self.prefs = preferences.clone();
            self.policy = MotionPolicy {
                prefers_reduced_motion: preferences.reduced_motion,
                pointer: if preferences.coarse_pointer {
                    Pointer::Coarse
                } else {
                    Pointer::Fine
                },
            };
            self.stale = true;
            if let Some(msg) = self.app.on_preferences(preferences) {
                // Preferences change between frames, with no caller to take
                // a command; the effect lands in the app's state.
                let _ = self.send(msg);
            }
        }
    }

    fn accessibility(&self) -> Option<A11yNode> {
        self.a11y.clone()
    }

    fn wants_frames(&self) -> bool {
        self.animating_visible
            || self.stale
            || self.repaint
            || !self.shown.is_empty()
            || !self.scroll_indicators.is_empty()
            || self.highlight.wants_frames()
            || self.press_scale.is_animating()
            || self.hold.is_some_and(|h| !h.done)
            || self.timer_pending()
            || self.hold_release.is_some()
            || self.moving.values().any(|m| m.shown.is_animating())
            || self.cameras.values().any(|c| c.is_animating())
            || self.press.wants_frames()
            || self.repeats.wants_frames()
            || self.paging.wants_frames()
            || self.hover_pending()
            || self.anims.values().any(|a| a.is_animating())
            || !self.exits.is_empty()
            || self.resizes.values().any(|r| r.shown.is_animating())
    }
}

fn gesture_phase(phase: InputPhase) -> Phase {
    match phase {
        InputPhase::Pressed => Phase::Pressed,
        InputPhase::Released => Phase::Released,
    }
}

fn contains(r: Rect, p: Point) -> bool {
    p.x >= r.x && p.x < r.x + r.width && p.y >= r.y && p.y < r.y + r.height
}
