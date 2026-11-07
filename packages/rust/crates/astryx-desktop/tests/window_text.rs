//! Types text into a focused text field in a real window with `xdotool` and
//! checks that it arrives through `Surface::text_input`, while Space does not
//! also confirm. Needs a display and `xdotool`; run it under Xvfb on headless
//! machines. It is a plain `main` because winit wants the main thread.

use std::process::Command as Process;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use astryx_core::{
    A11yNode, Color, Command, DisplayList, FrameInfo, Input, InputPhase, Rect, Role, Surface,
    TextMeasure,
};
use astryx_desktop::{run_with, Options};

const TITLE: &str = "Astryx text test";

#[derive(Default)]
struct Log {
    text: String,
    inputs: Vec<Input>,
}

struct Field(Arc<Mutex<Log>>);

impl Surface for Field {
    fn input(&mut self, input: Input, phase: InputPhase) -> Command {
        if phase == InputPhase::Pressed {
            self.0.lock().unwrap().inputs.push(input);
        }
        if input == Input::B {
            Command::Quit
        } else {
            Command::None
        }
    }
    fn text_input(&mut self, text: &str) -> Command {
        self.0.lock().unwrap().text.push_str(text);
        Command::None
    }
    fn frame(&mut self, _: &FrameInfo, _: &dyn TextMeasure) -> DisplayList {
        DisplayList {
            clear: Color::rgba(1.0, 1.0, 1.0, 1.0),
            commands: Vec::new(),
        }
    }
    fn accessibility(&self) -> Option<A11yNode> {
        Some(A11yNode {
            text_selection: None,
            secure: false,
            id: 1,
            role: Role::TextInput,
            label: Some("Name".into()),
            value: None,
            description: None,
            bounds: Rect::new(20.0, 20.0, 200.0, 32.0),
            focused: true,
            focusable: true,
            disabled: false,
            checked: None,
            selected: None,
            live: None,
            children: Vec::new(),
        })
    }
    fn wants_frames(&self) -> bool {
        false
    }
}

fn xdotool(args: &[&str]) -> Option<String> {
    let out = Process::new("xdotool").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

fn main() {
    if std::env::var_os("DISPLAY").is_none() {
        println!("window_text: skipped, no X display");
        return;
    }
    if xdotool(&["version"]).is_none() {
        println!("window_text: skipped, xdotool not installed");
        return;
    }
    std::thread::spawn(|| {
        let id = xdotool(&["search", "--sync", "--onlyvisible", "--name", TITLE])
            .and_then(|ids| ids.lines().next().map(str::to_owned))
            .expect("window appears");
        let _ = xdotool(&["mousemove", "--window", &id, "20", "20"]);
        let _ = xdotool(&["windowfocus", "--sync", &id]);
        std::thread::sleep(Duration::from_millis(300));
        xdotool(&["type", "--delay", "60", "Hé llo"]).expect("xdotool types");
        xdotool(&[
            "key",
            "--delay",
            "60",
            "ctrl+a",
            "Left",
            "BackSpace",
            "Home",
            "Delete",
            "End",
            "Escape",
        ])
        .expect("xdotool sends keys");
    });
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(60));
        eprintln!("window_text: timed out");
        std::process::exit(1);
    });
    let log = Arc::new(Mutex::new(Log::default()));
    let options = Options {
        title: TITLE.into(),
        size: (320, 120),
        ..Options::default()
    };
    run_with(Field(log.clone()), options).expect("desktop shell runs");
    let log = log.lock().unwrap();
    assert_eq!(log.text, "Hé llo", "typed text");
    assert_eq!(
        log.inputs,
        vec![
            Input::Left,
            Input::Backspace,
            Input::Home,
            Input::Delete,
            Input::End,
            Input::B
        ],
        "Space types a space instead of confirming; editing keys arrive as inputs"
    );
    println!("window_text: ok, typed {:?}", log.text);
}
