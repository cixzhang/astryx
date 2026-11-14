//! A real input method composing into a real window: ibus with its Japanese
//! engine (anthy) over XIM, under Xvfb. Typing "ka" composes "か" inline;
//! Return commits it into the field. Needs a display, `xdotool`, a running
//! `ibus-daemon --xim` with the anthy engine selected and `XMODIFIERS` set
//! (`scripts/ime-session.sh` starts one); otherwise it says why it skipped.
//! A plain `main` because winit wants the main thread.

use std::process::Command as Process;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use astryx_core::*;
use astryx_desktop::{run_with, Options};

const TITLE: &str = "Astryx IME test";

struct Field {
    value: String,
}

impl App for Field {
    type Msg = String;
    fn view(&self) -> Element<String> {
        VStack::new()
            .padding(Space::S4)
            .child(TextInput::new("Name", self.value.clone()).on_change(|v| v))
            .into()
    }
    fn update(&mut self, v: String) -> Command {
        self.value = v;
        Command::None
    }
}

/// The runtime, recording the text calls the shell makes and the value.
struct Recording {
    rt: Runtime<Field>,
    log: Arc<Mutex<Vec<String>>>,
    value: Arc<Mutex<String>>,
}

impl Surface for Recording {
    fn input(&mut self, input: Input, phase: InputPhase) -> Command {
        // Escape ends the test.
        if input == Input::B {
            return Command::Quit;
        }
        self.rt.input(input, phase)
    }
    fn text_input(&mut self, text: &str) -> Command {
        self.log.lock().unwrap().push(format!("input {text}"));
        let c = self.rt.text_input(text);
        *self.value.lock().unwrap() = self.rt.app().value.clone();
        c
    }
    fn text_compose(&mut self, text: &str, cursor: u32) {
        self.log
            .lock()
            .unwrap()
            .push(format!("compose {text} {cursor}"));
        self.rt.text_compose(text, cursor)
    }
    fn a11y_action(&mut self, id: u64, action: A11yAction) -> Command {
        self.rt.a11y_action(id, action)
    }
    fn pointer_move(&mut self, at: Option<Point>) {
        self.rt.pointer_move(at)
    }
    fn frame(&mut self, frame: &FrameInfo, text: &dyn TextMeasure) -> DisplayList {
        self.rt.frame(frame, text)
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
            "window_ime: skipped, no ibus session with the anthy engine (run under scripts/ime-session.sh)"
        );
        return;
    }
    let log = Arc::new(Mutex::new(Vec::new()));
    let value = Arc::new(Mutex::new(String::new()));
    std::thread::spawn(|| {
        let id = xdotool(&["search", "--sync", "--onlyvisible", "--name", TITLE])
            .and_then(|ids| ids.lines().next().map(str::to_owned))
            .expect("window appears");
        let _ = xdotool(&["mousemove", "--window", &id, "40", "40"]);
        let _ = xdotool(&["windowfocus", "--sync", &id]);
        std::thread::sleep(Duration::from_millis(500));
        // Focus the field (Tab), which turns the input method on.
        xdotool(&["key", "Tab"]).expect("xdotool");
        std::thread::sleep(Duration::from_millis(700));
        xdotool(&["type", "--delay", "120", "ka"]).expect("xdotool");
        std::thread::sleep(Duration::from_millis(700));
        xdotool(&["key", "Return"]).expect("xdotool");
        std::thread::sleep(Duration::from_millis(700));
        xdotool(&["key", "Escape"]).expect("xdotool");
    });
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(60));
        eprintln!("window_ime: timed out");
        std::process::exit(1);
    });
    let surface = Recording {
        rt: Runtime::new(Field {
            value: String::new(),
        }),
        log: log.clone(),
        value: value.clone(),
    };
    let options = Options {
        title: TITLE.into(),
        size: (360, 160),
        ..Options::default()
    };
    let _ = run_with(surface, options);
    let log = log.lock().unwrap().clone();
    let value = value.lock().unwrap().clone();
    println!("window_ime: calls {log:?}, value {value:?}");
    assert!(
        log.iter().any(|l| l == "compose か 3"),
        "the composition reached the surface inline: {log:?}"
    );
    assert_eq!(value, "か", "Return committed it: {log:?}");
    println!("window_ime: ok");
}
