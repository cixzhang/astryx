//! Types real keys into a real window with `xdotool` and checks that the
//! surface sees the mapped inputs. Needs a display and `xdotool`; run it
//! under Xvfb on headless machines (`xvfb-run cargo test -p astryx-desktop`).
//! It is a plain `main` because winit wants the main thread.

use std::process::Command as Process;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use astryx_core::{
    A11yNode, Command, DisplayList, FrameInfo, Input, InputPhase, Role, Surface, TextMeasure,
};
use astryx_desktop::{run_with, Options};
use astryx_headless::fixtures::{FixtureSurface, FIXTURE_SIZE};

const TITLE: &str = "Astryx key test";

struct Recording {
    inner: FixtureSurface,
    log: Arc<Mutex<Vec<Input>>>,
    tree: Arc<Mutex<Option<A11yNode>>>,
}

impl Surface for Recording {
    fn input(&mut self, input: Input, phase: InputPhase) -> Command {
        if phase == InputPhase::Pressed {
            self.log.lock().unwrap().push(input);
        }
        let command = self.inner.input(input, phase);
        *self.tree.lock().unwrap() = self.inner.accessibility();
        command
    }
    fn frame(&mut self, frame: &FrameInfo, text: &dyn TextMeasure) -> DisplayList {
        self.inner.frame(frame, text)
    }
    fn accessibility(&self) -> Option<A11yNode> {
        self.inner.accessibility()
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
        println!("window_keys: skipped, no X display");
        return;
    }
    if xdotool(&["version"]).is_none() {
        println!("window_keys: skipped, xdotool not installed");
        return;
    }
    std::thread::spawn(|| {
        let id = xdotool(&["search", "--sync", "--onlyvisible", "--name", TITLE])
            .and_then(|ids| ids.lines().next().map(str::to_owned))
            .expect("window appears");
        // Without a window manager, keyboard focus follows the pointer.
        let _ = xdotool(&["mousemove", "--window", &id, "20", "20"]);
        let _ = xdotool(&["windowfocus", "--sync", &id]);
        std::thread::sleep(Duration::from_millis(300));
        xdotool(&[
            "key",
            "--delay",
            "60",
            "Tab",
            "Tab",
            "Return",
            "shift+Tab",
            "space",
            "Escape",
        ])
        .expect("xdotool sends keys");
    });
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(60));
        eprintln!("window_keys: timed out waiting for keys");
        std::process::exit(1);
    });

    let log = Arc::new(Mutex::new(Vec::new()));
    let tree = Arc::new(Mutex::new(None));
    let surface = Recording {
        inner: FixtureSurface::new("gallery").unwrap(),
        log: log.clone(),
        tree: tree.clone(),
    };
    let options = Options {
        title: TITLE.into(),
        size: FIXTURE_SIZE,
        ..Options::default()
    };
    run_with(surface, options).expect("desktop shell runs");

    let log = log.lock().unwrap().clone();
    use Input::*;
    assert_eq!(
        log,
        vec![Next, Next, A, Previous, A, B],
        "inputs seen by the surface"
    );
    let tree = tree.lock().unwrap().clone().expect("tree");
    let switch = tree
        .children
        .iter()
        .find(|n| n.role == Role::Switch)
        .unwrap();
    assert_eq!(switch.checked, Some(true), "Enter on the switch toggled it");
    let cancel = tree
        .children
        .iter()
        .find(|n| n.label.as_deref() == Some("Cancel"))
        .unwrap();
    assert!(cancel.focused, "Shift+Tab moved focus back to Cancel");
    println!("window_keys: ok, inputs {log:?}");
}
