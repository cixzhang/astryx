//! Clicks and scrolls a real window with `xdotool` and checks that a press
//! focuses the node under the pointer, a release over the same node
//! activates it, a release elsewhere does not, and wheel notches arrive
//! through `Surface::scroll` as points. Needs a display and `xdotool`; run it
//! under Xvfb on headless machines. It is a plain `main` because winit wants
//! the main thread.

use std::process::Command as Process;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use astryx_core::{
    A11yAction, A11yNode, Color, Command, DisplayList, FrameInfo, Input, InputPhase, Point, Rect,
    Role, Surface, TextMeasure,
};
use astryx_desktop::pointer::POINTS_PER_LINE;
use astryx_desktop::{run_with, Options};

const TITLE: &str = "Astryx pointer test";

#[derive(Debug, PartialEq)]
enum Event {
    Action(u64, A11yAction),
    Scroll(Point, Point),
}

struct Buttons(Arc<Mutex<Vec<Event>>>);

fn button(id: u64, bounds: Rect) -> A11yNode {
    A11yNode {
        id,
        role: Role::Button,
        label: Some(format!("Button {id}")),
        description: None,
        value: None,
        bounds,
        focused: false,
        disabled: false,
        checked: None,
        selected: None,
        focusable: true,
        text_selection: None,
        secure: false,
        live: None,
        children: Vec::new(),
    }
}

impl Surface for Buttons {
    fn input(&mut self, input: Input, _: InputPhase) -> Command {
        if input == Input::B {
            Command::Quit
        } else {
            Command::None
        }
    }
    fn a11y_action(&mut self, id: u64, action: A11yAction) -> Command {
        self.0.lock().unwrap().push(Event::Action(id, action));
        Command::None
    }
    fn scroll(&mut self, at: Point, delta: Point) -> bool {
        self.0.lock().unwrap().push(Event::Scroll(at, delta));
        true
    }
    fn frame(&mut self, _: &FrameInfo, _: &dyn TextMeasure) -> DisplayList {
        DisplayList {
            clear: Color::rgba(1.0, 1.0, 1.0, 1.0),
            commands: Vec::new(),
        }
    }
    fn accessibility(&self) -> Option<A11yNode> {
        let mut root = button(1, Rect::new(0.0, 0.0, 320.0, 120.0));
        root.role = Role::Group;
        root.focusable = false;
        root.children = vec![
            button(2, Rect::new(20.0, 20.0, 100.0, 40.0)),
            button(3, Rect::new(180.0, 20.0, 100.0, 40.0)),
        ];
        Some(root)
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
        println!("window_pointer: skipped, no X display");
        return;
    }
    if xdotool(&["version"]).is_none() {
        println!("window_pointer: skipped, xdotool not installed");
        return;
    }
    std::thread::spawn(|| {
        let id = xdotool(&["search", "--sync", "--onlyvisible", "--name", TITLE])
            .and_then(|ids| ids.lines().next().map(str::to_owned))
            .expect("window appears");
        let _ = xdotool(&["windowfocus", "--sync", &id]);
        std::thread::sleep(Duration::from_millis(300));
        let at = |x: &str, y: &str| {
            xdotool(&["mousemove", "--window", &id, "--sync", x, y]).expect("xdotool moves");
            std::thread::sleep(Duration::from_millis(100));
        };
        // A click on button 2.
        at("70", "40");
        xdotool(&["click", "1"]).expect("xdotool clicks");
        // A press on button 2, released over button 3: no activation.
        at("70", "40");
        xdotool(&["mousedown", "1"]).expect("xdotool presses");
        at("230", "40");
        xdotool(&["mouseup", "1"]).expect("xdotool releases");
        // One wheel notch down (button 5), then one up (button 4), over button 3.
        xdotool(&["click", "5"]).expect("xdotool scrolls");
        std::thread::sleep(Duration::from_millis(100));
        xdotool(&["click", "4"]).expect("xdotool scrolls");
        std::thread::sleep(Duration::from_millis(200));
        xdotool(&["key", "Escape"]).expect("xdotool sends Escape");
    });
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(60));
        eprintln!("window_pointer: timed out");
        std::process::exit(1);
    });
    let log = Arc::new(Mutex::new(Vec::new()));
    let options = Options {
        title: TITLE.into(),
        size: (320, 120),
        ..Options::default()
    };
    run_with(Buttons(log.clone()), options).expect("desktop shell runs");
    let log = log.lock().unwrap();
    assert_eq!(
        log[..3],
        [
            Event::Action(2, A11yAction::Focus),
            Event::Action(2, A11yAction::Activate),
            Event::Action(2, A11yAction::Focus),
        ],
        "a click activates; a release over another node does not"
    );
    // winit on X11 reports a synthetic wheel click on both press and release,
    // so each notch arrives once or twice. The sign and size must be right.
    let at = Point { x: 230.0, y: 40.0 };
    let scrolls: Vec<f32> = log[3..]
        .iter()
        .map(|e| match e {
            Event::Scroll(p, d) if *p == at && d.x == 0.0 => d.y,
            other => panic!("unexpected event {other:?}"),
        })
        .collect();
    let down = scrolls
        .iter()
        .take_while(|y| **y == POINTS_PER_LINE)
        .count();
    let up = scrolls[down..]
        .iter()
        .filter(|y| **y == -POINTS_PER_LINE)
        .count();
    assert!(
        (1..=2).contains(&down) && (1..=2).contains(&up) && down + up == scrolls.len(),
        "one notch down, then one up, in points: {scrolls:?}"
    );
    println!("window_pointer: ok, {} events", log.len());
}
