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
        tether: Radio::Off,
    }
}

/// The most the bar ever holds: away from Wi-Fi while connecting through
/// the phone, so the dim phone shows beside the crossed-out Wi-Fi.
fn worst_tethering() -> DeviceStatus {
    DeviceStatus {
        wifi: Radio::On,
        wifi_name: None,
        tether: Radio::On,
        ..worst()
    }
}

struct Bar {
    width: f32,
    mode: Mode,
    device: DeviceStatus,
}

impl App for Bar {
    type Msg = ();
    fn view(&self) -> Element<()> {
        VStack::new()
            .width(SizeValue::Points(self.width))
            .child(StatusBar::new().device(self.device.clone()).sound_off(true))
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
fn shown(width: f32, mode: Mode, device: &DeviceStatus) -> Shown {
    let mut rt = Runtime::new(Bar {
        width,
        mode,
        device: device.clone(),
    });
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

/// Sweeps the bar from wide to narrow and returns the widths where the
/// percent goes and where PM turns into a small p, checking the order.
fn sweep(mode: Mode, device: &DeviceStatus) -> (f32, f32) {
    assert_eq!(
        shown(BRICK_WIDTH, mode, device),
        Shown::Everything,
        "{mode:?} at the Brick's width"
    );
    let mut last = Shown::Everything;
    let mut seen = vec![last];
    let (mut no_percent, mut small_p) = (0.0, 0.0);
    let mut width = 300.0;
    while width >= 120.0 {
        let now = shown(width, mode, device);
        assert!(
            now >= last,
            "{mode:?}: {now:?} came back after {last:?} at {width}"
        );
        if now != last {
            seen.push(now);
            match now {
                Shown::NoPercent => no_percent = width,
                Shown::SmallP => small_p = width,
                Shown::Everything => {}
            }
        }
        last = now;
        width -= 2.0;
    }
    assert_eq!(
        seen,
        [Shown::Everything, Shown::NoPercent, Shown::SmallP],
        "{mode:?}"
    );
    (no_percent, small_p)
}

#[test]
fn the_time_never_wraps_and_items_go_in_order_as_the_bar_narrows() {
    for mode in [Mode::Light, Mode::Dark] {
        assert_eq!(sweep(mode, &worst()), (236.0, 196.0), "{mode:?}");
    }
}

#[test]
fn with_the_phone_icon_too_the_order_holds_one_icon_sooner() {
    // The map's status card is 206 pt inside.
    const CARD: f32 = 206.0;
    for mode in [Mode::Light, Mode::Dark] {
        let (no_percent, small_p) = sweep(mode, &worst_tethering());
        let (plain_np, plain_sp) = sweep(mode, &worst());
        assert!(
            no_percent > plain_np && small_p > plain_sp,
            "one more icon drops things sooner"
        );
        assert_eq!((no_percent, small_p), (254.0, 214.0), "{mode:?}");
        assert_eq!(
            shown(CARD, mode, &worst_tethering()),
            Shown::SmallP,
            "{mode:?} on the card"
        );
        // Online through the phone, the phone stands in for Wi-Fi.
        let online = DeviceStatus {
            tether: Radio::Connected,
            ..worst_tethering()
        };
        assert_eq!(
            sweep(mode, &online),
            (236.0, 196.0),
            "{mode:?}: as wide as with Wi-Fi"
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
            let mut rt = Runtime::new(Bar {
                width,
                mode,
                device: worst(),
            });
            let png = render_png(&mut rt as &mut dyn Surface, 512, 40, 2.0, &[]);
            assert_golden(
                &png,
                golden(&format!("terrace-status-worst-{label}-{name}@2x.png")),
                tolerance,
            );
        }
    }
}

/// Away from Wi-Fi: connecting through the phone shows a dim phone beside
/// the crossed-out Wi-Fi; online through it, the phone stands in for Wi-Fi.
#[test]
fn online_through_the_phone_in_light_and_dark() {
    struct Tether(Mode);
    impl App for Tether {
        type Msg = ();
        fn view(&self) -> Element<()> {
            let bar = |wifi, tether| {
                StatusBar::new().device(DeviceStatus {
                    wifi,
                    bluetooth: Radio::Connected,
                    battery_percent: Some(64),
                    time: (9, 41),
                    tether,
                    ..DeviceStatus::default()
                })
            };
            VStack::new()
                .width(SizeValue::Points(BRICK_WIDTH))
                .child(bar(Radio::On, Radio::Off))
                .child(bar(Radio::On, Radio::On))
                .child(bar(Radio::On, Radio::Connected))
                .child(bar(Radio::Connected, Radio::Connected))
                .into()
        }
        fn update(&mut self, _: ()) -> Command {
            Command::None
        }
        fn theme(&self) -> ThemeChoice {
            ThemeChoice::new(PixelTheme::Terrace.theme(), self.0)
        }
    }
    let tolerance = Tolerance {
        channel: 0,
        pixels: 0,
    };
    for (mode, name) in [(Mode::Light, "light"), (Mode::Dark, "dark")] {
        let mut rt = Runtime::new(Tether(mode));
        let png = render_png(&mut rt as &mut dyn Surface, 512, 140, 2.0, &[]);
        assert_golden(
            &png,
            golden(&format!("terrace-status-tether-{name}@2x.png")),
            tolerance,
        );
    }
}
