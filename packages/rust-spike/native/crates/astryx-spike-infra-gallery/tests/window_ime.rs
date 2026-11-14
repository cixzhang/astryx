//! A real input method composing into the spike's text fields in a real
//! window: ibus with its Japanese engine (anthy) over XIM, under Xvfb.
//! "ka" composes "か" inline in a typeahead and in the multi-line chat
//! composer; Return commits it, and only the next Return sends. Run under
//! packages/rust/crates/astryx-desktop/scripts/ime-session.sh; otherwise it
//! says why it skipped. A plain `main`, as winit wants the main thread.

use std::process::Command as Process;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use astryx_core::contract::{
    A11yAction, A11yNode, Command, DisplayList, DrawCmd, FrameInfo, Input, InputPhase, Key,
    KeyEvent, KeyOutcome, Point, PointerEvent, PointerOutcome, Role, Surface, TextMeasure,
};
use astryx_core::{App, Element, Runtime, VStack};
use astryx_desktop::{run_with, Options};
use astryx_spike_ui::{static_source, ChatComposer, Entry, Typeahead};

const TITLE: &str = "Astryx spike IME test";

#[derive(Clone, Debug)]
enum Msg {
    Pick,
    Send(String),
}

struct Form {
    composer: bool,
    sent: Vec<String>,
}

impl App for Form {
    type Msg = Msg;
    fn view(&self) -> Element<Msg> {
        let field: Element<Msg> = if self.composer {
            ChatComposer::new("chat", Msg::Send).into()
        } else {
            Typeahead::new(
                "fruit",
                "Fruit",
                static_source(vec![Entry::new("kaki", "かき")]),
                None,
            )
            .on_change(|_| Msg::Pick)
            .into()
        };
        VStack::new().child(field).into()
    }
    fn update(&mut self, msg: Msg) -> Command {
        if let Msg::Send(text) = msg {
            self.sent.push(text);
        }
        Command::None
    }
}

#[derive(Default)]
struct Seen {
    /// Frames that drew "か" while the input method composed.
    inline: u32,
    /// The caret bounds the tree carried while composing.
    caret: Option<astryx_core::Rect>,
    fruit: String,
    message: String,
    /// The focused node's label, each time it changes.
    focus: Vec<String>,
    sent: Vec<String>,
}

/// The runtime, recording what the window drew and holds.
struct Recording {
    rt: Runtime<Form>,
    seen: Arc<Mutex<Seen>>,
    composing: bool,
}

fn find<'a>(n: &'a A11yNode, role: Role, label: &str) -> Option<&'a A11yNode> {
    if n.role == role && n.label.as_deref() == Some(label) {
        return Some(n);
    }
    n.children.iter().find_map(|c| find(c, role, label))
}

impl Surface for Recording {
    fn input(&mut self, input: Input, phase: InputPhase) -> Command {
        self.rt.input(input, phase)
    }
    fn text_input(&mut self, text: &str) -> Command {
        self.rt.text_input(text)
    }
    fn text_compose(&mut self, text: &str, cursor: u32) {
        self.composing = !text.is_empty();
        self.rt.text_compose(text, cursor)
    }
    fn key(&mut self, event: KeyEvent) -> KeyOutcome {
        // F12 ends the test.
        if event.key == Key::F(12) {
            return KeyOutcome {
                command: Command::Quit,
                handled: true,
            };
        }
        self.rt.key(event)
    }
    fn pointer(&mut self, event: PointerEvent) -> PointerOutcome {
        self.rt.pointer(event)
    }
    fn a11y_action(&mut self, id: u64, action: A11yAction) -> Command {
        self.rt.a11y_action(id, action)
    }
    fn pointer_move(&mut self, at: Option<Point>) {
        self.rt.pointer_move(at)
    }
    fn frame(&mut self, frame: &FrameInfo, text: &dyn TextMeasure) -> DisplayList {
        let list = self.rt.frame(frame, text);
        let mut seen = self.seen.lock().unwrap();
        let tree = self.rt.accessibility();
        if self.composing {
            let drew = list
                .commands
                .iter()
                .any(|c| matches!(c, DrawCmd::Text { text, .. } if text.contains('か')));
            seen.inline += u32::from(drew);
            seen.caret = tree
                .as_ref()
                .and_then(|t| find_focused(t))
                .and_then(|n| n.state.caret_bounds);
        }
        if let Some(t) = &tree {
            if let Some(n) = find(t, Role::ComboBox, "Fruit") {
                seen.fruit = n.value.clone().unwrap_or_default();
            }
            if let Some(n) = find(t, Role::TextInput, "Message input") {
                seen.message = n.value.clone().unwrap_or_default();
            }
        }
        seen.sent = self.rt.app().sent.clone();
        let focused = tree
            .as_ref()
            .and_then(|t| find_focused(t))
            .map(|n| format!("{:?} {}", n.role, n.label.clone().unwrap_or_default()))
            .unwrap_or_default();
        if seen.focus.last() != Some(&focused) {
            seen.focus.push(focused);
        }
        list
    }
    fn accessibility(&self) -> Option<A11yNode> {
        self.rt.accessibility()
    }
    fn wants_frames(&self) -> bool {
        self.rt.wants_frames()
    }
    fn take_command(&mut self) -> Command {
        self.rt.take_command()
    }
}

fn find_focused(n: &A11yNode) -> Option<&A11yNode> {
    if n.focused {
        return Some(n);
    }
    n.children.iter().find_map(find_focused)
}

fn xdotool(args: &[&str]) -> Option<String> {
    let out = Process::new("xdotool").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

fn ibus_engine() -> Option<String> {
    let out = Process::new("ibus").arg("engine").output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

fn main() {
    if std::env::var_os("DISPLAY").is_none() {
        println!("window_ime: skipped, no X display");
        return;
    }
    if xdotool(&["version"]).is_none() {
        println!("window_ime: skipped, xdotool not installed");
        return;
    }
    if std::env::var("XMODIFIERS").ok().as_deref() != Some("@im=ibus")
        || ibus_engine().as_deref() != Some("anthy")
    {
        println!(
            "window_ime: skipped, no ibus session with the anthy engine (run under ime-session.sh)"
        );
        return;
    }
    // One window per field: winit runs one event loop per process.
    if let Some(field) = std::env::args().nth(1) {
        run_field(field == "composer");
        return;
    }
    for field in ["typeahead", "composer"] {
        let me = std::env::current_exe().expect("test binary");
        let status = Process::new(me).arg(field).status().expect("run");
        assert!(status.success(), "window_ime {field} failed");
    }
    println!("window_ime: ok");
}

fn run_field(composer: bool) {
    let seen = Arc::new(Mutex::new(Seen::default()));
    std::thread::spawn(move || {
        let id = xdotool(&["search", "--sync", "--onlyvisible", "--name", TITLE])
            .and_then(|ids| ids.lines().next().map(str::to_owned))
            .expect("window appears");
        let _ = xdotool(&["mousemove", "--window", &id, "5", "5"]);
        let _ = xdotool(&["windowfocus", "--sync", &id]);
        let pause = || std::thread::sleep(Duration::from_millis(700));
        let keys = |k: &str| {
            xdotool(&["key", k]).expect("xdotool");
        };
        pause();
        // The field has focus: compose "ka" ("か"), commit with Return.
        xdotool(&["type", "--delay", "120", "ka"]).expect("xdotool");
        pause();
        keys("Return");
        pause();
        if composer {
            // Only this Return sends.
            keys("Return");
            pause();
        }
        keys("F12");
    });
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(60));
        eprintln!("window_ime: timed out");
        std::process::exit(1);
    });
    let surface = Recording {
        rt: Runtime::new(Form {
            composer,
            sent: Vec::new(),
        }),
        seen: seen.clone(),
        composing: false,
    };
    let options = Options {
        title: TITLE.into(),
        size: (480, 360),
        ..Options::default()
    };
    let _ = run_with(surface, options);
    let seen = seen.lock().unwrap();
    let what = if composer { "composer" } else { "typeahead" };
    println!(
        "window_ime {what}: inline frames {}, caret {:?}, fruit {:?}, message {:?}, sent {:?}, focus {:?}",
        seen.inline, seen.caret, seen.fruit, seen.message, seen.sent, seen.focus
    );
    assert!(seen.inline > 0, "{what}: the composition drew inline");
    assert!(
        seen.caret.is_some(),
        "{what}: the tree carried the caret for the candidate window"
    );
    if composer {
        assert_eq!(
            seen.sent,
            ["か"],
            "the commit's Return did not send; the next one did"
        );
    } else {
        assert_eq!(seen.fruit, "か", "Return committed into the typeahead");
    }
}
