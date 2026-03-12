//! Keyboard shortcut configuration.

use gtk::prelude::*;

/// Set up application-level keyboard shortcuts.
pub fn setup_shortcuts(app: &adw::Application) {
    // Ctrl+N → New download
    app.set_accels_for_action("win.new-download", &["<Control>n"]);
    // Ctrl+K → Focus search
    app.set_accels_for_action("win.focus-search", &["<Control>k"]);
    // Ctrl+, → Open settings
    app.set_accels_for_action("win.open-settings", &["<Control>comma"]);
    // Ctrl+A → Select all
    app.set_accels_for_action("win.select-all", &["<Control>a"]);
    // Ctrl+Q → Quit
    app.set_accels_for_action("app.quit", &["<Control>q"]);
}
