//! Apps receive the device's status through `on_preferences`, once per
//! change.

use astryx_core::*;

#[derive(Default)]
struct Bar {
    shown: Option<DeviceStatus>,
    updates: usize,
}

impl App for Bar {
    type Msg = Option<DeviceStatus>;
    fn view(&self) -> Element<Self::Msg> {
        let words = match &self.shown {
            Some(d) => format!(
                "{:02}:{:02} {:?} {}%",
                d.time.0,
                d.time.1,
                d.wifi,
                d.battery_percent.unwrap_or(0)
            ),
            None => "no status".into(),
        };
        Text::new(words).into()
    }
    fn update(&mut self, msg: Self::Msg) -> Command {
        self.shown = msg;
        self.updates += 1;
        Command::None
    }
    fn on_preferences(&self, preferences: &Preferences) -> Option<Self::Msg> {
        (preferences.device != self.shown).then(|| preferences.device.clone())
    }
}

#[test]
fn device_status_reaches_the_app_on_change() {
    let mut rt = Runtime::new(Bar::default());
    let status = DeviceStatus {
        wifi: Radio::Connected,
        wifi_name: Some("home".into()),
        bluetooth: Radio::Off,
        battery_percent: Some(87),
        charging: true,
        time: (21, 40),
        tether: Radio::Off,
    };
    let prefs = Preferences {
        device: Some(status.clone()),
        ..Preferences::default()
    };
    rt.set_preferences(&prefs);
    assert_eq!(rt.app().shown.as_ref(), Some(&status));
    // The same status again is no change.
    rt.set_preferences(&prefs);
    assert_eq!(rt.app().updates, 1);
    let later = DeviceStatus {
        time: (21, 41),
        battery_percent: Some(86),
        ..status
    };
    rt.set_preferences(&Preferences {
        device: Some(later.clone()),
        ..Preferences::default()
    });
    assert_eq!(rt.app().shown, Some(later));
    assert_eq!(rt.app().updates, 2);
    // Shells without a status leave it `None`.
    assert_eq!(Preferences::default().device, None);
}
