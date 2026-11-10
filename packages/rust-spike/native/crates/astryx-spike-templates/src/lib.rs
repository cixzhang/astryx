//! Astryx page templates on the Rust spike stack: `shell-nav` (an editor
//! frame with menus, a file tree and a command palette) and
//! `settings-dialog` (a settings window with sections, search and panels).

pub mod settings_dialog;
pub mod shell_nav;

pub use settings_dialog::SettingsDialog;
pub use shell_nav::ShellNav;
