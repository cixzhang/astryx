//! The browser shell: canvas sizing, the frame loop, input, and the mirror.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use astryx_core::{
    A11yAction, Command, DrawCmd, FrameInfo, Input, InputPhase, Point, Preferences, Renderer,
    Surface,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    AddEventListenerOptions, Document, Gamepad, GamepadButton, GamepadMappingType,
    HtmlCanvasElement, KeyboardEvent, ResizeObserver, ResizeObserverBoxOptions,
    ResizeObserverEntry, ResizeObserverOptions, ResizeObserverSize, Window,
};

use crate::focus::{focus_order, focused_id, hit_test, stepped_within};
use crate::fonts::{FontKey, FontPlan};
use crate::gamepad::{PadTracker, Snapshot};
use crate::keys::map_key;
use crate::mirror::Mirror;
use crate::prefs::{detect_preferences, QUERIES};
use astryx_render_gl::GlRenderer;

/// The canvas to draw into: a CSS selector or the element itself.
pub enum Target {
    Selector(String),
    Canvas(HtmlCanvasElement),
}

impl From<&str> for Target {
    fn from(selector: &str) -> Self {
        Target::Selector(selector.to_string())
    }
}

impl From<String> for Target {
    fn from(selector: String) -> Self {
        Target::Selector(selector)
    }
}

impl From<HtmlCanvasElement> for Target {
    fn from(canvas: HtmlCanvasElement) -> Self {
        Target::Canvas(canvas)
    }
}

/// Options for [`run_with`].
pub struct Options {
    /// The directory, relative to the page, that holds the font subsets
    /// `scripts/subset-fonts.py` writes. The default is `fonts/`. The wasm
    /// embeds no fonts, so with `None` text does not draw.
    pub fonts_url: Option<String>,
    /// The accessible name of the app. Assistive tech announces it when the
    /// app has no focusable nodes and the mirror root takes focus.
    pub label: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            fonts_url: Some("fonts/".into()),
            label: None,
        }
    }
}

/// Runs `surface` in a canvas with default [`Options`].
pub fn run(target: impl Into<Target>, surface: impl Surface + 'static) -> Result<(), JsValue> {
    run_with(target, surface, Options::default())
}

/// Runs `surface` in a canvas and draws the first frame. The app runs for
/// the life of the page.
pub fn run_with(
    target: impl Into<Target>,
    surface: impl Surface + 'static,
    options: Options,
) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let canvas = match target.into() {
        Target::Canvas(canvas) => canvas,
        Target::Selector(selector) => document
            .query_selector(&selector)?
            .ok_or_else(|| format!("astryx-web: nothing matches {selector}"))?
            .dyn_into()
            .map_err(|_| format!("astryx-web: {selector} is not a canvas"))?,
    };
    mark(&window, "astryx-start");
    start(window, document, canvas, Box::new(surface), options)
}

#[cfg(target_arch = "wasm32")]
fn create_renderer(canvas: &HtmlCanvasElement) -> Result<GlRenderer, JsValue> {
    use astryx_render_gl::astryx_raster::text::FontSet;
    // Faces arrive as they load; see `Shell::add_font`.
    GlRenderer::from_html_canvas(canvas, FontSet::new())
        .map_err(|e| format!("astryx-web: renderer: {e}").into())
}

#[cfg(not(target_arch = "wasm32"))]
fn create_renderer(_: &HtmlCanvasElement) -> Result<GlRenderer, JsValue> {
    Err("astryx-web runs only on wasm32".into())
}

/// Schemes `Command::OpenUrl` may open. Others, such as `javascript:`, could
/// run code with the page's origin or surprise the person.
fn is_openable_scheme(protocol: &str) -> bool {
    matches!(protocol, "http:" | "https:" | "mailto:" | "tel:")
}

/// Opens a link in a new tab with no `window.opener`, resolving relative
/// links against the page. Browsers allow this only during a key press or
/// click, so a gamepad press may be blocked as a pop-up.
fn open_url(window: &Window, url: &str) {
    let base = window.location().href().unwrap_or_default();
    let target = match web_sys::Url::new_with_base(url, &base) {
        Ok(parsed) if is_openable_scheme(&parsed.protocol()) => parsed.href(),
        _ => {
            web_sys::console::warn_1(&format!("astryx-web: not opening {url}").into());
            return;
        }
    };
    let opened =
        window.open_with_url_and_target_and_features(&target, "_blank", "noopener,noreferrer");
    if opened.is_err() {
        web_sys::console::warn_1(&format!("astryx-web: the browser blocked {target}").into());
    }
}

fn mark(window: &Window, name: &str) {
    if let Some(performance) = window.performance() {
        let _ = performance.mark(name);
    }
}

fn now(window: &Window) -> f64 {
    window.performance().map(|p| p.now()).unwrap_or(0.0)
}

struct Shell {
    window: Window,
    canvas: HtmlCanvasElement,
    surface: Box<dyn Surface>,
    renderer: GlRenderer,
    mirror: Mirror,
    width: u32,
    height: u32,
    scale: f32,
    /// The previous frame's time while frames run back to back.
    last_frame_ms: Option<f64>,
    dirty: bool,
    stopped: bool,
    frames: u64,
    pads: HashMap<u32, PadTracker>,
    /// Clicks before this time come from a key the shell already handled.
    suppress_click_until: f64,
    preferences: Preferences,
    runner: Weak<Runner>,
    font_base: Option<String>,
    font_plan: FontPlan,
    /// First-subset files still loading. The first frame waits for them.
    initial_fonts_pending: usize,
    /// The node a pointer pressed, activated if the pointer lifts over it.
    pointer_target: Option<u64>,
    cursor: &'static str,
}

impl Shell {
    fn frame_info(&self, dt: f32) -> FrameInfo {
        FrameInfo {
            width: self.width,
            height: self.height,
            scale: self.scale,
            dt,
        }
    }

    fn input(&mut self, input: Input, phase: InputPhase) {
        if self.stopped {
            return;
        }
        let command = self.surface.input(input, phase);
        self.handle(command);
    }

    fn text_input(&mut self, text: &str) {
        if self.stopped || text.is_empty() {
            return;
        }
        let command = self.surface.text_input(text);
        self.handle(command);
    }

    fn text_compose(&mut self, text: &str) {
        if self.stopped {
            return;
        }
        // The input method's own caret is not exposed; it sits at the end.
        self.surface.text_compose(text, text.len() as u32);
        self.dirty = true;
    }

    /// The focusable, enabled node under a pointer event, and its role.
    fn pointer_hit(&self, event: &web_sys::MouseEvent) -> Option<(u64, astryx_core::Role)> {
        let rect = self.canvas.get_bounding_client_rect();
        let x = event.client_x() as f64 - rect.left() - self.canvas.client_left() as f64;
        let y = event.client_y() as f64 - rect.top() - self.canvas.client_top() as f64;
        let tree = self.surface.accessibility()?;
        let node = hit_test(&tree, x as f32, y as f32)?;
        (!node.disabled).then_some((node.id, node.role))
    }

    /// Wheel and trackpad scrolling at a point, in logical points. Returns
    /// true when the app scrolled.
    fn scroll(&mut self, x: f32, y: f32, dx: f32, dy: f32) -> bool {
        let moved = self.surface.scroll(Point { x, y }, Point { x: dx, y: dy });
        if moved {
            self.dirty = true;
        }
        moved
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::None => {}
            // A page cannot close itself, so the app stops taking input and frames.
            Command::Quit => self.stopped = true,
            Command::OpenUrl(url) => open_url(&self.window, &url),
        }
        self.dirty = true;
    }

    fn focused(&self) -> Option<u64> {
        self.surface.accessibility().as_ref().and_then(focused_id)
    }

    /// Draws one frame and updates the mirror. With `follow_focus`, DOM focus
    /// follows app focus when the app already has DOM focus.
    fn render(&mut self, now_ms: f64, follow_focus: bool) {
        if self.width == 0 || self.height == 0 || self.initial_fonts_pending > 0 {
            return;
        }
        let dt = match self.last_frame_ms {
            Some(previous) => ((now_ms - previous) / 1000.0).clamp(0.0, 0.1) as f32,
            None => 1.0 / 60.0,
        };
        let had_focus = follow_focus && self.mirror.has_dom_focus();
        let info = self.frame_info(dt);
        for (id, data) in self.surface.take_image_updates() {
            self.renderer.set_image(id, data.as_ref());
        }
        let list = self.surface.frame(&info, self.renderer.measurer());
        self.renderer.render(&list, &info);
        self.request_fonts_for(&list.commands);
        self.mirror.sync(self.surface.accessibility().as_ref());
        if had_focus {
            self.mirror.focus_tab_stop();
        }
        self.last_frame_ms = self.surface.wants_frames().then_some(now_ms);
        self.dirty = false;
        self.frames += 1;
        let _ = self
            .mirror
            .root()
            .set_attribute("data-astryx-frames", &self.frames.to_string());
        if self.frames == 1 {
            mark(&self.window, "astryx-first-frame");
        }
    }

    /// Starts loading the font subsets that this frame's text needs.
    fn request_fonts_for(&mut self, commands: &[DrawCmd]) {
        if self.font_base.is_none() {
            return;
        }
        let mut keys = Vec::new();
        for cmd in commands {
            if let DrawCmd::Text { text, style, .. } = cmd {
                keys.extend(self.font_plan.request_for(text, style.weight));
            }
        }
        self.fetch_fonts(keys);
    }

    fn fetch_fonts(&self, keys: Vec<FontKey>) {
        let Some(base) = &self.font_base else {
            return;
        };
        for key in keys {
            let url = format!("{base}{}", self.font_plan.file_name(key));
            crate::fonts::fetch(url, self.runner.clone(), move |runner: &Runner, result| {
                runner.on_font(key, result)
            });
        }
    }

    /// Gives a loaded subset to the renderer. Each script subset is its own
    /// family, so it becomes a fallback after the subsets loaded before it.
    fn add_font(&mut self, key: FontKey, data: Vec<u8>) {
        if self.renderer.add_fallback_font(data).is_err() {
            let file = self.font_plan.file_name(key);
            web_sys::console::warn_1(&format!("astryx-web: {file} is not a font").into());
        }
    }

    fn a11y_action(&mut self, id: u64, action: A11yAction) {
        if self.stopped {
            return;
        }
        let command = self.surface.a11y_action(id, action);
        self.handle(command);
    }

    /// Sizes the canvas backing store. `physical` is the exact device-pixel
    /// size when the browser reports it.
    fn resize(&mut self, css_width: f64, css_height: f64, physical: Option<(u32, u32)>) {
        let dpr = self.window.device_pixel_ratio();
        let rounded = (
            (css_width * dpr).round().max(0.0) as u32,
            (css_height * dpr).round().max(0.0) as u32,
        );
        // The exact size differs from the rounded one by pixel snapping only.
        // Some zoom emulations report CSS pixels instead; trust devicePixelRatio then.
        let close = |exact: u32, rounded: u32| exact.abs_diff(rounded) <= 1;
        let (width, height) = match physical {
            Some((w, h)) if close(w, rounded.0) && close(h, rounded.1) => (w, h),
            _ => rounded,
        };
        if self.canvas.width() != width {
            self.canvas.set_width(width);
        }
        if self.canvas.height() != height {
            self.canvas.set_height(height);
        }
        self.width = width;
        self.height = height;
        self.scale = dpr as f32;
        self.place_mirror(css_width, css_height);
        self.dirty = true;
    }

    fn place_mirror(&self, css_width: f64, css_height: f64) {
        let root = self.mirror.root();
        // Measure the containing block's origin, then place the root on the
        // canvas content box relative to it.
        self.mirror.place(0.0, 0.0, css_width, css_height);
        let origin = root.get_bounding_client_rect();
        let rect = self.canvas.get_bounding_client_rect();
        let left = rect.left() + self.canvas.client_left() as f64 - origin.left();
        let top = rect.top() + self.canvas.client_top() as f64 - origin.top();
        self.mirror.place(left, top, css_width, css_height);
    }

    /// Re-reads the canvas size from layout.
    fn relayout(&mut self) {
        let rect = self.canvas.get_bounding_client_rect();
        let border_x = (self.canvas.offset_width() - self.canvas.client_width()) as f64;
        let border_y = (self.canvas.offset_height() - self.canvas.client_height()) as f64;
        self.resize(rect.width() - border_x, rect.height() - border_y, None);
    }

    /// Polls gamepads and feeds their inputs. Returns true while any pad is
    /// connected, so polling continues.
    fn poll_gamepads(&mut self, now_ms: f64) -> bool {
        let Ok(list) = self.window.navigator().get_gamepads() else {
            return false;
        };
        let mut seen = Vec::new();
        let mut events = Vec::new();
        for value in list.iter() {
            if value.is_null() || value.is_undefined() {
                continue;
            }
            let pad: Gamepad = value.unchecked_into();
            if !pad.connected() {
                continue;
            }
            let index = pad.index();
            seen.push(index);
            let buttons: Vec<bool> = pad
                .buttons()
                .iter()
                .map(|b| b.unchecked_into::<GamepadButton>().pressed())
                .collect();
            let axes: Vec<f64> = pad
                .axes()
                .iter()
                .map(|a| a.as_f64().unwrap_or(0.0))
                .collect();
            let snapshot = Snapshot {
                standard: pad.mapping() == GamepadMappingType::Standard,
                buttons: &buttons,
                axes: &axes,
            };
            events.extend(
                self.pads
                    .entry(index)
                    .or_default()
                    .update(now_ms, &snapshot),
            );
        }
        let gone: Vec<u32> = self
            .pads
            .keys()
            .copied()
            .filter(|index| !seen.contains(index))
            .collect();
        for index in gone {
            if let Some(mut pad) = self.pads.remove(&index) {
                events.extend(pad.release_all());
            }
        }
        for (input, phase) in events {
            self.input(input, phase);
        }
        !seen.is_empty()
    }
}

type FrameCallback = Closure<dyn FnMut(f64)>;

struct Runner {
    window: Window,
    shell: RefCell<Shell>,
    raf: RefCell<Option<FrameCallback>>,
    raf_pending: Cell<bool>,
}

impl Runner {
    /// Requests one animation frame unless one is already pending.
    fn schedule(&self) {
        if self.raf_pending.get() {
            return;
        }
        if let Some(callback) = self.raf.borrow().as_ref() {
            if self
                .window
                .request_animation_frame(callback.as_ref().unchecked_ref())
                .is_ok()
            {
                self.raf_pending.set(true);
            }
        }
    }

    fn on_frame(self: &Rc<Self>, now_ms: f64) {
        self.raf_pending.set(false);
        let keep_going = {
            // A frame can arrive while an event handler holds the shell.
            let Ok(mut shell) = self.shell.try_borrow_mut() else {
                self.schedule();
                return;
            };
            let pads = shell.poll_gamepads(now_ms);
            if shell.stopped {
                return;
            }
            if shell.dirty || shell.surface.wants_frames() {
                shell.render(now_ms, true);
            }
            shell.surface.wants_frames() || pads
        };
        if keep_going {
            self.schedule();
        }
    }

    fn key_input(&self, event: &KeyboardEvent, shell: &Shell) -> Option<Input> {
        // Keys during composition belong to the input method.
        if event.is_composing() || event.key() == "Process" {
            return None;
        }
        let command = event.ctrl_key() || event.alt_key() || event.meta_key();
        let in_text_field = shell.mirror.is_text_field(event.target().as_ref());
        map_key(&event.key(), event.shift_key(), command, in_text_field)
    }

    fn on_keydown(&self, event: &KeyboardEvent) {
        let Ok(mut shell) = self.shell.try_borrow_mut() else {
            return;
        };
        let Some(input) = self.key_input(event, &shell) else {
            return;
        };
        let time = now(&self.window);
        match input {
            Input::Next | Input::Previous => {
                let order = shell
                    .surface
                    .accessibility()
                    .as_ref()
                    .map(focus_order)
                    .unwrap_or_default();
                let before = shell.focused();
                shell.input(input, InputPhase::Pressed);
                shell.render(time, false);
                // At the end of the app's order the browser's own Tab moves
                // focus out of the canvas, so the app never traps the keyboard.
                if stepped_within(&order, before, shell.focused(), input == Input::Next) {
                    event.prevent_default();
                    shell.mirror.focus_tab_stop();
                }
            }
            _ => {
                event.prevent_default();
                if input == Input::A {
                    shell.suppress_click_until = f64::INFINITY;
                }
                shell.input(input, InputPhase::Pressed);
                drop(shell);
                self.schedule();
            }
        }
    }

    fn on_keyup(&self, event: &KeyboardEvent) {
        let Ok(mut shell) = self.shell.try_borrow_mut() else {
            return;
        };
        let Some(input) = self.key_input(event, &shell) else {
            return;
        };
        if input == Input::A {
            event.prevent_default();
            // Buttons fire `click` right after Enter or Space; the key already
            // reached the app.
            shell.suppress_click_until = now(&self.window) + 100.0;
        }
        shell.input(input, InputPhase::Released);
        drop(shell);
        self.schedule();
    }

    /// Typing, pasting, dictation, and on-screen keyboards in a mirror field
    /// arrive here. Text goes to the app and the field keeps the app's value.
    fn on_beforeinput(&self, event: &web_sys::InputEvent) {
        let Ok(mut shell) = self.shell.try_borrow_mut() else {
            return;
        };
        if !shell.mirror.is_text_field(event.target().as_ref()) {
            return;
        }
        let kind = event.input_type();
        match kind.as_str() {
            // Composition is not cancelable; it commits in `compositionend`.
            "insertCompositionText" => return,
            "insertText" | "insertReplacementText" | "insertFromYank" => {
                if let Some(text) = event.data() {
                    shell.text_input(&text);
                }
            }
            "insertFromPaste" | "insertFromDrop" => {
                let text = event.data().or_else(|| {
                    event
                        .data_transfer()
                        .and_then(|d| d.get_data("text/plain").ok())
                });
                if let Some(text) = text {
                    shell.text_input(&text);
                }
            }
            // On-screen keyboards delete without a Backspace key event.
            "deleteContentBackward" => {
                shell.input(Input::Backspace, InputPhase::Pressed);
                shell.input(Input::Backspace, InputPhase::Released);
            }
            "deleteContentForward" => {
                shell.input(Input::Delete, InputPhase::Pressed);
                shell.input(Input::Delete, InputPhase::Released);
            }
            _ => {}
        }
        event.prevent_default();
        drop(shell);
        self.schedule();
    }

    fn on_composition(&self, event: &web_sys::CompositionEvent) {
        let Ok(mut shell) = self.shell.try_borrow_mut() else {
            return;
        };
        if event.type_() == "compositionstart" {
            shell.mirror.set_composing(true);
            return;
        }
        if event.type_() == "compositionupdate" {
            shell.text_compose(&event.data().unwrap_or_default());
            drop(shell);
            self.schedule();
            return;
        }
        shell.text_compose("");
        if let Some(text) = event.data() {
            shell.text_input(&text);
        }
        shell.mirror.set_composing(false);
        drop(shell);
        self.schedule();
    }

    /// A press focuses the node under the pointer; lifting over the same node
    /// activates it, as a click does.
    fn on_pointerdown(&self, event: &web_sys::PointerEvent) {
        if event.pointer_type() == "mouse" && event.button() != 0 {
            return;
        }
        let Ok(mut shell) = self.shell.try_borrow_mut() else {
            return;
        };
        // Keeps DOM focus in the app and stops text selection on the canvas.
        event.prevent_default();
        let hit = shell.pointer_hit(event);
        shell.pointer_target = hit.map(|(id, _)| id);
        if let Some((id, _)) = hit {
            shell.a11y_action(id, A11yAction::Focus);
            shell.render(now(&self.window), false);
        }
        shell.mirror.focus_tab_stop();
        drop(shell);
        self.schedule();
    }

    fn on_pointerup(&self, event: &web_sys::PointerEvent) {
        let Ok(mut shell) = self.shell.try_borrow_mut() else {
            return;
        };
        let Some(target) = shell.pointer_target.take() else {
            return;
        };
        if shell.pointer_hit(event).map(|(id, _)| id) == Some(target) {
            shell.a11y_action(target, A11yAction::Activate);
            drop(shell);
            self.schedule();
        }
    }

    fn on_pointermove(&self, event: &web_sys::PointerEvent) {
        let Ok(mut shell) = self.shell.try_borrow_mut() else {
            return;
        };
        let cursor = match shell.pointer_hit(event) {
            Some((_, astryx_core::Role::TextInput)) => "text",
            Some(_) => "pointer",
            None => "default",
        };
        if cursor != shell.cursor {
            shell.cursor = cursor;
            let _ = shell.canvas.style().set_property("cursor", cursor);
        }
    }

    fn on_wheel(&self, event: &web_sys::WheelEvent) {
        // Ctrl with the wheel is the browser's zoom, and a trackpad pinch.
        if event.ctrl_key() {
            return;
        }
        let Ok(mut shell) = self.shell.try_borrow_mut() else {
            return;
        };
        let rect = shell.canvas.get_bounding_client_rect();
        let x = (event.client_x() as f64 - rect.left()) as f32;
        let y = (event.client_y() as f64 - rect.top()) as f32;
        let unit = match event.delta_mode() {
            web_sys::WheelEvent::DOM_DELTA_LINE => 16.0,
            web_sys::WheelEvent::DOM_DELTA_PAGE => rect.height() as f32,
            _ => 1.0,
        };
        let (dx, dy) = (event.delta_x() as f32 * unit, event.delta_y() as f32 * unit);
        if shell.scroll(x, y, dx, dy) {
            event.prevent_default();
            drop(shell);
            self.schedule();
        }
    }

    /// Activation from assistive tech arrives as a click on a mirror element.
    fn on_click(&self, event: &web_sys::Event) {
        let Ok(mut shell) = self.shell.try_borrow_mut() else {
            return;
        };
        let time = now(&self.window);
        if time < shell.suppress_click_until {
            return;
        }
        let Some(id) = event.target().and_then(|t| shell.mirror.node_id_of(&t)) else {
            return;
        };
        event.prevent_default();
        // Clicking also moves focus there, as a pointer click on a control does.
        shell.a11y_action(id, A11yAction::Focus);
        shell.a11y_action(id, A11yAction::Activate);
        shell.render(time, true);
    }

    /// DOM focus moved to a mirror element (Tab from outside, or a screen
    /// reader), so app focus follows.
    fn on_focusin(&self, event: &web_sys::Event) {
        let Ok(mut shell) = self.shell.try_borrow_mut() else {
            return;
        };
        let Some(id) = event.target().and_then(|t| shell.mirror.node_id_of(&t)) else {
            return;
        };
        if shell.focused() == Some(id) {
            return;
        }
        shell.a11y_action(id, A11yAction::Focus);
        shell.render(now(&self.window), true);
    }

    fn on_resize(&self, entries: &js_sys::Array) {
        let Some(entry) = entries
            .iter()
            .last()
            .map(|e| e.unchecked_into::<ResizeObserverEntry>())
        else {
            return;
        };
        let rect = entry.content_rect();
        // Safari has no device-pixel box; the shell then rounds CSS size times
        // devicePixelRatio.
        let device = js_sys::Reflect::get(&entry, &"devicePixelContentBoxSize".into())
            .ok()
            .filter(|v| !v.is_undefined())
            .map(|v| v.unchecked_into::<js_sys::Array>())
            .and_then(|sizes| sizes.get(0).dyn_into::<ResizeObserverSize>().ok())
            .map(|size| (size.inline_size() as u32, size.block_size() as u32));
        let Ok(mut shell) = self.shell.try_borrow_mut() else {
            return;
        };
        shell.resize(rect.width(), rect.height(), device);
        // Drawing now avoids one blank frame, since resizing clears the canvas.
        shell.render(now(&self.window), true);
    }

    fn on_preferences(&self) {
        let Ok(mut shell) = self.shell.try_borrow_mut() else {
            return;
        };
        let preferences = detect_preferences();
        if preferences == shell.preferences {
            return;
        }
        shell.preferences = preferences;
        shell.surface.set_preferences(&preferences);
        shell.dirty = true;
        drop(shell);
        self.schedule();
    }

    fn on_font(&self, key: FontKey, result: Result<Vec<u8>, JsValue>) {
        let Ok(mut shell) = self.shell.try_borrow_mut() else {
            return;
        };
        shell.font_plan.finish(key);
        match result {
            Ok(data) => shell.add_font(key, data),
            Err(err) => web_sys::console::warn_2(&"astryx-web: a font did not load".into(), &err),
        }
        // Only the first subset counts toward the first frame.
        if key.0 == 0 && shell.initial_fonts_pending > 0 {
            shell.initial_fonts_pending -= 1;
        }
        // Text re-lays out with the new face on the next frame.
        shell.dirty = true;
        drop(shell);
        self.schedule();
    }

    fn on_dpr_change(&self) {
        if let Ok(mut shell) = self.shell.try_borrow_mut() {
            shell.relayout();
            shell.render(now(&self.window), true);
        }
    }
}

/// Listens to a DOM event for the life of the page.
fn listen<E: JsCast + 'static>(
    target: &web_sys::EventTarget,
    name: &str,
    runner: &Rc<Runner>,
    handler: fn(&Runner, &E),
) -> Result<(), JsValue> {
    let runner = runner.clone();
    let closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
        handler(&runner, event.unchecked_ref::<E>());
    });
    target.add_event_listener_with_callback(name, closure.as_ref().unchecked_ref())?;
    closure.forget();
    Ok(())
}

/// Re-arms a one-shot listener for the current devicePixelRatio, since a
/// resolution media query only matches one ratio.
fn watch_dpr(runner: Rc<Runner>) {
    let query = format!("(resolution: {}dppx)", runner.window.device_pixel_ratio());
    let Ok(Some(list)) = runner.window.match_media(&query) else {
        return;
    };
    let next = runner.clone();
    let callback = Closure::once_into_js(move || {
        next.on_dpr_change();
        watch_dpr(next);
    });
    let options = AddEventListenerOptions::new();
    options.set_once(true);
    let _ = list.add_event_listener_with_callback_and_add_event_listener_options(
        "change",
        callback.unchecked_ref(),
        &options,
    );
}

/// True when `ResizeObserver` reports exact device-pixel sizes. Observing
/// an unknown box throws, so the shell checks first.
fn supports_device_pixel_box() -> bool {
    js_sys::Reflect::get(&js_sys::global(), &"ResizeObserverEntry".into())
        .and_then(|entry| js_sys::Reflect::get(&entry, &"prototype".into()))
        .and_then(|prototype| js_sys::Reflect::has(&prototype, &"devicePixelContentBoxSize".into()))
        .unwrap_or(false)
}

fn start(
    window: Window,
    document: Document,
    canvas: HtmlCanvasElement,
    mut surface: Box<dyn Surface>,
    options: Options,
) -> Result<(), JsValue> {
    let renderer = create_renderer(&canvas)?;
    mark(&window, "astryx-renderer-ready");
    let mirror = Mirror::new(&document, &canvas, options.label)?;
    // The mirror carries the semantics; the canvas is only pixels.
    canvas.set_attribute("aria-hidden", "true")?;

    let preferences = detect_preferences();
    surface.set_preferences(&preferences);
    let shell = Shell {
        window: window.clone(),
        canvas: canvas.clone(),
        surface,
        renderer,
        mirror,
        width: 0,
        height: 0,
        scale: 1.0,
        last_frame_ms: None,
        dirty: true,
        stopped: false,
        frames: 0,
        pads: HashMap::new(),
        suppress_click_until: 0.0,
        preferences,
        runner: Weak::new(),
        font_base: options.fonts_url.map(|url| {
            if url.is_empty() || url.ends_with('/') {
                url
            } else {
                url + "/"
            }
        }),
        font_plan: FontPlan::new(),
        initial_fonts_pending: 0,
        pointer_target: None,
        cursor: "default",
    };
    let runner = Rc::new(Runner {
        window: window.clone(),
        shell: RefCell::new(shell),
        raf: RefCell::new(None),
        raf_pending: Cell::new(false),
    });

    {
        let mut shell = runner.shell.borrow_mut();
        shell.runner = Rc::downgrade(&runner);
        if shell.font_base.is_some() {
            let initial = shell.font_plan.request_initial();
            shell.initial_fonts_pending = initial.len();
            shell.fetch_fonts(initial);
        }
    }

    let frame_runner = runner.clone();
    *runner.raf.borrow_mut() = Some(Closure::new(move |now_ms: f64| {
        frame_runner.on_frame(now_ms)
    }));

    let root: web_sys::EventTarget = runner.shell.borrow().mirror.root().clone().into();
    listen::<KeyboardEvent>(&root, "keydown", &runner, Runner::on_keydown)?;
    listen::<KeyboardEvent>(&root, "keyup", &runner, Runner::on_keyup)?;
    listen::<web_sys::Event>(&root, "click", &runner, Runner::on_click)?;
    listen::<web_sys::Event>(&root, "focusin", &runner, Runner::on_focusin)?;
    listen::<web_sys::InputEvent>(&root, "beforeinput", &runner, Runner::on_beforeinput)?;
    listen::<web_sys::CompositionEvent>(
        &root,
        "compositionstart",
        &runner,
        Runner::on_composition,
    )?;
    listen::<web_sys::CompositionEvent>(
        &root,
        "compositionupdate",
        &runner,
        Runner::on_composition,
    )?;
    listen::<web_sys::CompositionEvent>(&root, "compositionend", &runner, Runner::on_composition)?;
    listen::<web_sys::PointerEvent>(&canvas, "pointerdown", &runner, Runner::on_pointerdown)?;
    listen::<web_sys::PointerEvent>(&canvas, "pointerup", &runner, Runner::on_pointerup)?;
    listen::<web_sys::PointerEvent>(&canvas, "pointermove", &runner, Runner::on_pointermove)?;
    listen::<web_sys::PointerEvent>(&canvas, "pointercancel", &runner, |runner, _| {
        if let Ok(mut shell) = runner.shell.try_borrow_mut() {
            shell.pointer_target = None;
        }
    })?;
    listen::<web_sys::WheelEvent>(&canvas, "wheel", &runner, Runner::on_wheel)?;
    listen::<web_sys::Event>(&window, "gamepadconnected", &runner, |runner, _| {
        runner.schedule()
    })?;
    for query in QUERIES {
        if let Some(list) = window.match_media(query)? {
            listen::<web_sys::Event>(&list, "change", &runner, |runner, _| {
                runner.on_preferences()
            })?;
        }
    }
    watch_dpr(runner.clone());

    let resize_runner = runner.clone();
    let on_resize = Closure::<dyn FnMut(js_sys::Array)>::new(move |entries: js_sys::Array| {
        resize_runner.on_resize(&entries);
    });
    let observer = ResizeObserver::new(on_resize.as_ref().unchecked_ref())?;
    on_resize.forget();
    if supports_device_pixel_box() {
        let observe_options = ResizeObserverOptions::new();
        observe_options.set_box(ResizeObserverBoxOptions::DevicePixelContentBox);
        observer.observe_with_options(&canvas, &observe_options);
    } else {
        observer.observe(&canvas);
    }

    // Draw the first frame now instead of waiting for the observer.
    {
        let mut shell = runner.shell.borrow_mut();
        shell.relayout();
        shell.render(now(&window), false);
    }
    runner.schedule();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_openable_scheme;

    #[test]
    fn opens_only_web_mail_and_phone_links() {
        for ok in ["http:", "https:", "mailto:", "tel:"] {
            assert!(is_openable_scheme(ok), "{ok}");
        }
        for blocked in ["javascript:", "data:", "file:", "blob:", "vbscript:"] {
            assert!(!is_openable_scheme(blocked), "{blocked}");
        }
    }
}
