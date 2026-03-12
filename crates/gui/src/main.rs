mod app;
mod engine_bridge;
mod model;
mod widgets;
mod pages;
mod dialogs;
mod shortcuts;

use adw::prelude::*;
use glib::ExitCode;

const APP_ID: &str = "com.goshapps.downloader";

/// Shared download category definitions for filtering across pages.
pub const DOWNLOAD_CATEGORIES: &[&str] = &["All", "Video", "Audio", "Documents", "Software", "Images"];

fn main() -> ExitCode {
    env_logger::init();

    let app = app::GoshFetchApplication::new(APP_ID);
    app.run()
}
