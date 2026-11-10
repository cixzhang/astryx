//! The status bar's worst case on the Brick: "12:59 PM", Wi-Fi, Bluetooth,
//! sounds off, 100% and charging, in Terrace light and dark. The time is
//! always one line; when the bar is short the percent goes first, then `PM`
//! turns into a small raised `p`. Set `ASTRYX_UPDATE_GOLDENS=1` to rewrite
//! the images after an intended change.

use astryx_core::lab::StatusBar;
use astryx_core::{
    App, Command, DeviceStatus, DrawCmd, Element, FrameInfo, Mode, Radio, Runtime, SizeValue,
    Surface, ThemeChoice, VStack,
};
use astryx_headless::astryx_render_soft::SoftRenderer;
use astryx_headless::golden::{assert_golden, Tolerance};
use astryx_headless::render_png;
use astryx_theme_pixel::PixelTheme;
use std::path::PathBuf;

/// The Brick's screen at its scale: 512 by 384 points.
const BRICK_WIDTH: f32 = 512.0;

fn worst() -> DeviceStatus {
    DeviceStatus {
        wifi: Radio::Connected,
        wifi_name: Some("Home".into()),
        bluetooth: Radio::Connected,
        battery_percent: Some(100),
        charging: true,
        time: (12, 59),
    }
}

struct Bar {
    width: f32,
    mode: Mode,
}

impl App for Bar {
    type Msg = ();
    fn view(&self) -> Element<()> {
        VStack::new()
            .width(SizeValue::Points(self.width))
            .child(StatusBar::new().device(worst()).sound_off(true))
            .into()
    }
    fn update(&mut self, _: ()) -> Command {
        Command::None
    }
    fn theme(&self) -> ThemeChoice {
        ThemeChoice::new(PixelTheme::Terrace.theme(), self.mode)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Shown {
    /// "12:59 PM" and "100%".
    Everything,
    /// The percent is gone.
    NoPercent,
    /// The percent is gone and `PM` is a small `p`.
    SmallP,
}

/// What the bar shows at `width`, checking on the way that the time is one
/// line with nothing cut short.
fn shown(width: f32, mode: Mode) -> Shown {
    let mut rt = Runtime::new(Bar { width, mode });
    let soft = SoftRenderer::new();
    let frame = FrameInfo {
        width: 1024,
        height: 80,
        scale: 2.0,
        dt: 0.0,
    };
    let list = rt.frame(&frame, soft.text());
    let texts: Vec<(String, f32, f32, f32)> = list
        .commands
        .iter()
        .filter_map(|c| match c {
            DrawCmd::Text { rect, text, .. } => Some((text.clone(), rect.x, rect.y, rect.width)),
            _ => None,
        })
        .collect();
    let find = |t: &str| texts.iter().find(|(s, ..)| s == t);
    let digits = find("12:59").unwrap_or_else(|| panic!("{width}: no digits in {texts:?}"));
    let half = find(" PM")
        .or_else(|| find("p"))
        .unwrap_or_else(|| panic!("{width}: no PM in {texts:?}"));
    assert_eq!(digits.2, half.2, "{width}: the time is one line: {texts:?}");
    assert!(
        half.1 >= digits.1 + digits.3 - 0.5,
        "{width}: PM follows the digits"
    );
    assert!(
        texts.iter().all(|(s, ..)| !s.ends_with('…')),
        "{width}: nothing is cut short: {texts:?}"
    );
    match (find("100%").is_some(), half.0 == " PM") {
        (true, true) => Shown::Everything,
        (false, true) => Shown::NoPercent,
        (false, false) => Shown::SmallP,
        (true, false) => panic!("{width}: PM shrank while the percent still shows"),
    }
}

#[test]
fn the_time_never_wraps_and_items_go_in_order_as_the_bar_narrows() {
    for mode in [Mode::Light, Mode::Dark] {
        assert_eq!(
            shown(BRICK_WIDTH, mode),
            Shown::Everything,
            "{mode:?} at the Brick's width"
        );
        let mut last = Shown::Everything;
        let mut seen = vec![last];
        let mut width = 260.0;
        while width >= 120.0 {
            let now = shown(width, mode);
            assert!(
                now >= last,
                "{mode:?}: {now:?} came back after {last:?} at {width}"
            );
            if now != last {
                seen.push(now);
            }
            last = now;
            width -= 2.0;
        }
        assert_eq!(
            seen,
            [Shown::Everything, Shown::NoPercent, Shown::SmallP],
            "{mode:?}"
        );
    }
}

fn golden(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens")
        .join(name)
}

#[test]
fn worst_case_bars_at_the_brick_width_and_short() {
    let tolerance = Tolerance {
        channel: 0,
        pixels: 0,
    };
    for (mode, name) in [(Mode::Light, "light"), (Mode::Dark, "dark")] {
        for (width, label) in [(BRICK_WIDTH, "full"), (220.0, "short"), (180.0, "shortest")] {
            let mut rt = Runtime::new(Bar { width, mode });
            let png = render_png(&mut rt as &mut dyn Surface, 512, 40, 2.0, &[]);
            assert_golden(
                &png,
                golden(&format!("terrace-status-worst-{label}-{name}@2x.png")),
                tolerance,
            );
        }
    }
}
