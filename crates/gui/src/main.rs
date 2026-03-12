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

fn main() -> ExitCode {
    env_logger::init();

    let app = app::GoshFetchApplication::new(APP_ID);
    app.run()
}
