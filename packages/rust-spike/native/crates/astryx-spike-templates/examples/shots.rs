//! Renders both templates at desktop and phone sizes:
//! `cargo run -p astryx-spike-templates --example shots -- <out-dir>`.

use astryx_core::Runtime;
use astryx_spike_templates::{SettingsDialog, ShellNav};

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "template-shots".into());
    std::fs::create_dir_all(&out).unwrap();
    for (w, h, tag) in [(1280u32, 800u32, "desktop"), (390, 844, "phone")] {
        let mut rt = Runtime::new(ShellNav::default()).hide_focus_until_keyboard();
        let png = astryx_headless::render_png(&mut rt, w, h, 1.0, &[]);
        std::fs::write(format!("{out}/shell-nav-{tag}.png"), png).unwrap();
        let mut rt = Runtime::new(SettingsDialog::default()).hide_focus_until_keyboard();
        let png = astryx_headless::render_png(&mut rt, w, h, 1.0, &[]);
        std::fs::write(format!("{out}/settings-dialog-{tag}.png"), png).unwrap();
    }
}
