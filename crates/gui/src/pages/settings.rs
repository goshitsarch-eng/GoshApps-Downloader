//! Settings page — AdwPreferencesPage with tabs for General, Network, BitTorrent, Appearance, About.

use adw::prelude::*;
use gosh_fetch_engine::db::Settings;
use std::cell::RefCell;
use std::rc::Rc;

use crate::engine_bridge::EngineBridge;
use crate::model::AppModel;

pub fn build_settings_page(_model: &AppModel, bridge: &EngineBridge) -> gtk::Box {
    let page = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .build();

    let prefs_page = adw::PreferencesPage::builder().build();

    // Current settings (loaded from engine)
    let settings: Rc<RefCell<Settings>> = Rc::new(RefCell::new(Settings::default()));

    // ===== GENERAL =====
    let general_group = adw::PreferencesGroup::builder()
        .title("General")
        .build();

    // Download path
    let download_path_row = adw::EntryRow::builder()
        .title("Download Path")
        .text(&settings.borrow().download_path)
        .build();
    general_group.add(&download_path_row);

    // Notifications toggle
    let notifications_row = adw::SwitchRow::builder()
        .title("Enable Notifications")
        .subtitle("Show desktop notifications for download events")
        .active(settings.borrow().enable_notifications)
        .build();
    general_group.add(&notifications_row);

    // Close to tray
    let tray_row = adw::SwitchRow::builder()
        .title("Close to Tray")
        .subtitle("Minimize to system tray instead of quitting")
        .active(settings.borrow().close_to_tray)
        .build();
    general_group.add(&tray_row);

    // Delete files on remove
    let delete_row = adw::SwitchRow::builder()
        .title("Delete Files on Remove")
        .subtitle("Also delete downloaded files when removing a download")
        .active(settings.borrow().delete_files_on_remove)
        .build();
    general_group.add(&delete_row);

    // User agent
    let ua_row = adw::EntryRow::builder()
        .title("User Agent")
        .text(&settings.borrow().user_agent)
        .build();
    general_group.add(&ua_row);

    prefs_page.add(&general_group);

    // ===== NETWORK =====
    let network_group = adw::PreferencesGroup::builder()
        .title("Network")
        .build();

    let concurrent_row = adw::SpinRow::builder()
        .title("Max Concurrent Downloads")
        .subtitle("Maximum number of simultaneous downloads")
        .adjustment(&gtk::Adjustment::new(
            settings.borrow().max_concurrent_downloads as f64,
            1.0, 20.0, 1.0, 5.0, 0.0,
        ))
        .build();
    network_group.add(&concurrent_row);

    let connections_row = adw::SpinRow::builder()
        .title("Connections per Server")
        .adjustment(&gtk::Adjustment::new(
            settings.borrow().max_connections_per_server as f64,
            1.0, 16.0, 1.0, 4.0, 0.0,
        ))
        .build();
    network_group.add(&connections_row);

    let split_row = adw::SpinRow::builder()
        .title("Split Count")
        .subtitle("Number of segments per download")
        .adjustment(&gtk::Adjustment::new(
            settings.borrow().split_count as f64,
            1.0, 32.0, 1.0, 4.0, 0.0,
        ))
        .build();
    network_group.add(&split_row);

    // Speed limits
    let dl_limit_row = adw::SpinRow::builder()
        .title("Download Speed Limit (KB/s)")
        .subtitle("0 = unlimited")
        .adjustment(&gtk::Adjustment::new(
            (settings.borrow().download_speed_limit / 1024) as f64,
            0.0, 1_000_000.0, 100.0, 1000.0, 0.0,
        ))
        .build();
    network_group.add(&dl_limit_row);

    let ul_limit_row = adw::SpinRow::builder()
        .title("Upload Speed Limit (KB/s)")
        .subtitle("0 = unlimited")
        .adjustment(&gtk::Adjustment::new(
            (settings.borrow().upload_speed_limit / 1024) as f64,
            0.0, 1_000_000.0, 100.0, 1000.0, 0.0,
        ))
        .build();
    network_group.add(&ul_limit_row);

    // Proxy
    let proxy_row = adw::EntryRow::builder()
        .title("Proxy URL")
        .text(&settings.borrow().proxy_url)
        .build();
    network_group.add(&proxy_row);

    // Timeouts
    let connect_timeout_row = adw::SpinRow::builder()
        .title("Connect Timeout (seconds)")
        .adjustment(&gtk::Adjustment::new(
            settings.borrow().connect_timeout as f64,
            5.0, 300.0, 5.0, 30.0, 0.0,
        ))
        .build();
    network_group.add(&connect_timeout_row);

    let read_timeout_row = adw::SpinRow::builder()
        .title("Read Timeout (seconds)")
        .adjustment(&gtk::Adjustment::new(
            settings.borrow().read_timeout as f64,
            5.0, 600.0, 5.0, 30.0, 0.0,
        ))
        .build();
    network_group.add(&read_timeout_row);

    let retries_row = adw::SpinRow::builder()
        .title("Max Retries")
        .adjustment(&gtk::Adjustment::new(
            settings.borrow().max_retries as f64,
            0.0, 20.0, 1.0, 3.0, 0.0,
        ))
        .build();
    network_group.add(&retries_row);

    // Allocation mode
    let alloc_row = adw::ComboRow::builder()
        .title("File Allocation Mode")
        .model(&gtk::StringList::new(&["None", "Sparse", "Full"]))
        .build();
    let alloc_idx = match settings.borrow().allocation_mode.as_str() {
        "sparse" => 1,
        "full" => 2,
        _ => 0,
    };
    alloc_row.set_selected(alloc_idx);
    network_group.add(&alloc_row);

    prefs_page.add(&network_group);

    // ===== BITTORRENT =====
    let bt_group = adw::PreferencesGroup::builder()
        .title("BitTorrent")
        .build();

    let dht_row = adw::SwitchRow::builder()
        .title("Enable DHT")
        .subtitle("Distributed Hash Table for peer discovery")
        .active(settings.borrow().bt_enable_dht)
        .build();
    bt_group.add(&dht_row);

    let pex_row = adw::SwitchRow::builder()
        .title("Enable PEX")
        .subtitle("Peer Exchange protocol")
        .active(settings.borrow().bt_enable_pex)
        .build();
    bt_group.add(&pex_row);

    let lpd_row = adw::SwitchRow::builder()
        .title("Enable LPD")
        .subtitle("Local Peer Discovery")
        .active(settings.borrow().bt_enable_lpd)
        .build();
    bt_group.add(&lpd_row);

    let max_peers_row = adw::SpinRow::builder()
        .title("Max Peers")
        .adjustment(&gtk::Adjustment::new(
            settings.borrow().bt_max_peers as f64,
            1.0, 200.0, 5.0, 20.0, 0.0,
        ))
        .build();
    bt_group.add(&max_peers_row);

    let seed_ratio_row = adw::SpinRow::builder()
        .title("Seed Ratio")
        .subtitle("Stop seeding after reaching this ratio")
        .adjustment(&gtk::Adjustment::new(
            settings.borrow().bt_seed_ratio,
            0.0, 100.0, 0.1, 1.0, 0.0,
        ))
        .digits(1)
        .build();
    bt_group.add(&seed_ratio_row);

    let auto_tracker_row = adw::SwitchRow::builder()
        .title("Auto-Update Trackers")
        .subtitle("Fetch updated tracker lists periodically")
        .active(settings.borrow().auto_update_trackers)
        .build();
    bt_group.add(&auto_tracker_row);

    prefs_page.add(&bt_group);

    // ===== APPEARANCE =====
    let appearance_group = adw::PreferencesGroup::builder()
        .title("Appearance")
        .build();

    let theme_row = adw::ComboRow::builder()
        .title("Theme")
        .model(&gtk::StringList::new(&["System", "Light", "Dark"]))
        .build();
    let theme_idx = match settings.borrow().theme.as_str() {
        "light" => 1,
        "dark" => 2,
        _ => 0,
    };
    theme_row.set_selected(theme_idx);
    {
        theme_row.connect_selected_notify(move |row| {
            let manager = adw::StyleManager::default();
            match row.selected() {
                0 => manager.set_color_scheme(adw::ColorScheme::Default),
                1 => manager.set_color_scheme(adw::ColorScheme::ForceLight),
                2 => manager.set_color_scheme(adw::ColorScheme::ForceDark),
                _ => {}
            }
        });
    }
    appearance_group.add(&theme_row);

    prefs_page.add(&appearance_group);

    scroll.set_child(Some(&prefs_page));
    page.append(&scroll);

    // Save button
    let save_bar = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(12)
        .build();

    let spacer = gtk::Box::builder().hexpand(true).build();
    save_bar.append(&spacer);

    let reset_btn = gtk::Button::builder()
        .label("Reset Defaults")
        .css_classes(["flat"])
        .build();
    save_bar.append(&reset_btn);

    let save_btn = gtk::Button::builder()
        .label("Save Settings")
        .css_classes(["suggested-action"])
        .margin_start(8)
        .build();
    {
        let bridge = bridge.clone();
        let download_path_row = download_path_row.clone();
        let notifications_row = notifications_row.clone();
        let tray_row = tray_row.clone();
        let delete_row = delete_row.clone();
        let ua_row = ua_row.clone();
        let concurrent_row = concurrent_row.clone();
        let connections_row = connections_row.clone();
        let split_row = split_row.clone();
        let dl_limit_row = dl_limit_row.clone();
        let ul_limit_row = ul_limit_row.clone();
        let proxy_row = proxy_row.clone();
        let connect_timeout_row = connect_timeout_row.clone();
        let read_timeout_row = read_timeout_row.clone();
        let retries_row = retries_row.clone();
        let alloc_row = alloc_row.clone();
        let dht_row = dht_row.clone();
        let pex_row = pex_row.clone();
        let lpd_row = lpd_row.clone();
        let max_peers_row = max_peers_row.clone();
        let seed_ratio_row = seed_ratio_row.clone();
        let auto_tracker_row = auto_tracker_row.clone();
        let theme_row = theme_row.clone();

        save_btn.connect_clicked(move |_| {
            let alloc_mode = match alloc_row.selected() {
                1 => "sparse",
                2 => "full",
                _ => "none",
            }.to_string();

            let theme = match theme_row.selected() {
                1 => "light",
                2 => "dark",
                _ => "system",
            }.to_string();

            let new_settings = Settings {
                download_path: download_path_row.text().to_string(),
                max_concurrent_downloads: concurrent_row.value() as u32,
                max_connections_per_server: connections_row.value() as u32,
                split_count: split_row.value() as u32,
                download_speed_limit: (dl_limit_row.value() as u64) * 1024,
                upload_speed_limit: (ul_limit_row.value() as u64) * 1024,
                user_agent: ua_row.text().to_string(),
                enable_notifications: notifications_row.is_active(),
                close_to_tray: tray_row.is_active(),
                theme,
                bt_enable_dht: dht_row.is_active(),
                bt_enable_pex: pex_row.is_active(),
                bt_enable_lpd: lpd_row.is_active(),
                bt_max_peers: max_peers_row.value() as u32,
                bt_seed_ratio: seed_ratio_row.value(),
                auto_update_trackers: auto_tracker_row.is_active(),
                delete_files_on_remove: delete_row.is_active(),
                proxy_url: proxy_row.text().to_string(),
                connect_timeout: connect_timeout_row.value() as u64,
                read_timeout: read_timeout_row.value() as u64,
                max_retries: retries_row.value() as u32,
                allocation_mode: alloc_mode,
            };

            bridge.update_settings(new_settings.clone());
            bridge.apply_settings(new_settings);
        });
    }
    save_bar.append(&save_btn);

    page.append(&save_bar);

    // About button
    let about_btn = gtk::Button::builder()
        .label("About Goshapps Downloader")
        .css_classes(["flat"])
        .margin_start(12)
        .margin_bottom(12)
        .halign(gtk::Align::Start)
        .build();
    {
        let page_ref = page.clone();
        about_btn.connect_clicked(move |_| {
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
            about.present(Some(&page_ref));
        });
    }
    page.append(&about_btn);

    page
}
