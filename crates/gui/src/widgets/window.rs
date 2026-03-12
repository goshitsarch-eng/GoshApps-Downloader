//! Main application window with sidebar navigation and page stack.

use adw::prelude::*;
use gtk::{gio, glib};

use crate::engine_bridge::EngineBridge;
use crate::model::AppModel;
use crate::pages;
use crate::widgets;

/// Load saved window state from disk.
fn load_window_state() -> Option<(i32, i32, bool)> {
    let data_dir = dirs::data_dir()?;
    let path = data_dir.join("com.goshapps.downloader/window-state.json");
    let contents = std::fs::read_to_string(&path).ok()?;
    let val: serde_json::Value = serde_json::from_str(&contents).ok()?;
    let width = val.get("width")?.as_i64()? as i32;
    let height = val.get("height")?.as_i64()? as i32;
    let maximized = val.get("maximized")?.as_bool()?;
    Some((width, height, maximized))
}

/// Save current window state to disk.
fn save_window_state(window: &adw::ApplicationWindow) {
    if let Some(data_dir) = dirs::data_dir() {
        let dir = data_dir.join("com.goshapps.downloader");
        let _ = std::fs::create_dir_all(&dir);
        let (width, height) = window.default_size();
        let maximized = window.is_maximized();
        let state = serde_json::json!({
            "width": width,
            "height": height,
            "maximized": maximized,
        });
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            // Write to temp file then rename for atomicity
            let tmp_path = dir.join("window-state.json.tmp");
            if std::fs::write(&tmp_path, &json).is_ok() {
                let _ = std::fs::rename(&tmp_path, dir.join("window-state.json"));
            }
        }
    }
}

/// The main application window.
pub struct GoshFetchWindow;

impl GoshFetchWindow {
    pub fn new(app: &adw::Application, model: AppModel, bridge: EngineBridge) -> adw::ApplicationWindow {
        let (saved_w, saved_h, saved_maximized) = load_window_state()
            .unwrap_or((1200, 800, false));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Goshapps Downloader")
            .default_width(saved_w)
            .default_height(saved_h)
            .width_request(900)
            .height_request(600)
            .build();

        if saved_maximized {
            window.maximize();
        }

        // Set up keyboard shortcuts
        crate::shortcuts::setup_shortcuts(app);

        // Content stack for pages
        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .transition_duration(200)
            .hexpand(true)
            .vexpand(true)
            .build();

        // Build pages
        let downloads_page = pages::downloads::build_downloads_page(&model, &bridge);
        let history_page = pages::history::build_history_page(&model, &bridge);
        let statistics_page = pages::statistics::build_statistics_page(&model, &bridge);
        let scheduler_page = pages::scheduler::build_scheduler_page(&model, &bridge);
        let settings_page = pages::settings::build_settings_page(&model, &bridge);

        stack.add_named(&downloads_page, Some("downloads"));
        stack.add_named(&history_page, Some("history"));
        stack.add_named(&statistics_page, Some("statistics"));
        stack.add_named(&scheduler_page, Some("scheduler"));
        stack.add_named(&settings_page, Some("settings"));

        stack.set_visible_child_name("downloads");

        // Sidebar
        let sidebar = widgets::sidebar::build_sidebar(&model, &stack);

        // Status bar
        let status_bar = widgets::status_bar::build_status_bar(&model);

        // Header bar
        let header_bar = adw::HeaderBar::builder()
            .show_title(true)
            .build();

        // Add download button in header
        let add_button = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text("Add Download (Ctrl+N)")
            .css_classes(["suggested-action"])
            .build();
        {
            let model_clone = model.clone();
            let bridge_clone = bridge.clone();
            let window_ref = window.clone();
            add_button.connect_clicked(move |_| {
                crate::dialogs::add_download::show_add_download_dialog(
                    &window_ref,
                    &model_clone,
                    &bridge_clone,
                );
            });
        }
        header_bar.pack_start(&add_button);

        // Hamburger menu
        let app_menu = gio::Menu::new();
        app_menu.append(Some("Keyboard Shortcuts"), Some("win.show-shortcuts"));
        app_menu.append(Some("Preferences"), Some("win.open-settings"));
        app_menu.append(Some("About"), Some("win.show-about"));
        let menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&app_menu)
            .tooltip_text("Menu")
            .build();
        header_bar.pack_end(&menu_button);

        // Notification bell
        let notif_button = widgets::notification_dropdown::build_notification_button(&model);
        header_bar.pack_end(&notif_button);

        // Layout: split view with sidebar + content
        let split_view = adw::NavigationSplitView::builder()
            .min_sidebar_width(220.0)
            .max_sidebar_width(280.0)
            .build();

        // Sidebar navigation page
        let sidebar_page = adw::NavigationPage::builder()
            .title("Goshapps Downloader")
            .child(&sidebar)
            .build();

        // Content area: header + stack + status bar
        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let toolbar_view = adw::ToolbarView::builder().build();
        toolbar_view.add_top_bar(&header_bar);
        toolbar_view.set_content(Some(&stack));

        content_box.append(&toolbar_view);
        content_box.append(&status_bar);

        let content_page = adw::NavigationPage::builder()
            .title("Downloads")
            .child(&content_box)
            .build();

        split_view.set_sidebar(Some(&sidebar_page));
        split_view.set_content(Some(&content_page));

        window.set_content(Some(&split_view));

        // Set up window actions
        setup_window_actions(&window, &model, &bridge, &stack);

        // File drop support: accept .torrent files or URLs dropped onto the window
        {
            let drop_target = gtk::DropTarget::new(gio::File::static_type(), gdk::DragAction::COPY);
            let bridge_for_drop = bridge.clone();
            drop_target.connect_drop(move |_target, value, _x, _y| {
                if let Ok(file) = value.get::<gio::File>() {
                    if let Some(path) = file.path() {
                        let path_str = path.to_string_lossy().to_string();
                        if path_str.ends_with(".torrent") {
                            bridge_for_drop.add_torrent_file(path_str, None);
                        } else {
                            // Treat as a regular file URL
                            let uri = file.uri().to_string();
                            bridge_for_drop.add_download(uri, None);
                        }
                        return true;
                    }
                    // If no local path, try the URI (e.g. a URL dragged from a browser)
                    let uri = file.uri().to_string();
                    if !uri.is_empty() {
                        bridge_for_drop.add_download(uri, None);
                        return true;
                    }
                }
                false
            });
            window.add_controller(drop_target);
        }

        // Handle close-to-tray and window state persistence
        {
            let model_clone = model.clone();
            let bridge_clone = bridge.clone();
            window.connect_close_request(move |win| {
                // Save window state before closing/hiding
                save_window_state(win);

                if model_clone.close_to_tray() {
                    win.set_visible(false);
                    glib::Propagation::Stop
                } else {
                    bridge_clone.shutdown();
                    glib::Propagation::Proceed
                }
            });
        }

        // Show onboarding wizard on first run
        {
            let should_onboard = dirs::data_dir()
                .map(|d| !d.join("com.goshapps.downloader/onboarding-done").exists())
                .unwrap_or(false);
            if should_onboard {
                crate::dialogs::onboarding::show_onboarding(&window, &bridge);
            }
        }

        window
    }
}

fn setup_window_actions(
    window: &adw::ApplicationWindow,
    model: &AppModel,
    bridge: &EngineBridge,
    stack: &gtk::Stack,
) {
    // Action: new-download
    let action_new = gio::SimpleAction::new("new-download", None);
    {
        let model = model.clone();
        let bridge = bridge.clone();
        let window = window.clone();
        action_new.connect_activate(move |_, _| {
            crate::dialogs::add_download::show_add_download_dialog(
                &window,
                &model,
                &bridge,
            );
        });
    }
    window.add_action(&action_new);

    // Action: focus-search
    let action_search = gio::SimpleAction::new("focus-search", None);
    action_search.connect_activate(move |_, _| {
        // TODO: Focus the search entry on the current page
    });
    window.add_action(&action_search);

    // Action: open-settings
    let action_settings = gio::SimpleAction::new("open-settings", None);
    {
        let stack = stack.clone();
        action_settings.connect_activate(move |_, _| {
            stack.set_visible_child_name("settings");
        });
    }
    window.add_action(&action_settings);

    // Action: select-all
    let action_select_all = gio::SimpleAction::new("select-all", None);
    action_select_all.connect_activate(move |_, _| {
        // TODO: Select all downloads on the current page
    });
    window.add_action(&action_select_all);

    // Action: show-shortcuts (simple dialog showing keybindings)
    let action_shortcuts = gio::SimpleAction::new("show-shortcuts", None);
    {
        let window = window.clone();
        action_shortcuts.connect_activate(move |_, _| {
            let dialog = adw::MessageDialog::builder()
                .heading("Keyboard Shortcuts")
                .body(
                    "Ctrl+N    Add download\n\
                     Ctrl+K    Focus search\n\
                     Ctrl+,    Open settings\n\
                     Ctrl+A    Select all\n\
                     Ctrl+Q    Quit"
                )
                .modal(true)
                .transient_for(&window)
                .build();
            dialog.add_response("close", "Close");
            dialog.set_default_response(Some("close"));
            dialog.present();
        });
    }
    window.add_action(&action_shortcuts);

    // Action: show-about
    let action_about = gio::SimpleAction::new("show-about", None);
    {
        let window = window.clone();
        action_about.connect_activate(move |_, _| {
            let about = adw::AboutDialog::builder()
                .application_name("Goshapps Downloader")
                .application_icon("com.goshapps.downloader")
                .version("3.0.0")
                .developer_name("Gosh")
                .license_type(gtk::License::Agpl30)
                .website("https://github.com/goshitsarch-eng/Gosh-Fetch")
                .issue_url("https://github.com/goshitsarch-eng/Gosh-Fetch/issues")
                .build();
            about.add_credit_section(Some("Download Engine"), &["gosh-dl (MIT)"]);
            about.present(Some(&window));
        });
    }
    window.add_action(&action_about);
}
