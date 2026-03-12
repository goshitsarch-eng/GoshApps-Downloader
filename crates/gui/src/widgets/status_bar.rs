//! Status bar at the bottom of the window showing speeds and connection status.

use adw::prelude::*;

use crate::model::AppModel;

pub fn build_status_bar(model: &AppModel) -> gtk::Box {
    let bar = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(16)
        .margin_start(12)
        .margin_end(12)
        .margin_top(4)
        .margin_bottom(4)
        .css_classes(["toolbar", "status-bar"])
        .build();

    // Download speed
    let dl_icon = gtk::Image::from_icon_name("go-down-symbolic");
    let dl_label = gtk::Label::builder()
        .label("0 B/s")
        .css_classes(["caption"])
        .build();
    bar.append(&dl_icon);
    bar.append(&dl_label);

    // Upload speed
    let ul_icon = gtk::Image::from_icon_name("go-up-symbolic");
    let ul_label = gtk::Label::builder()
        .label("0 B/s")
        .css_classes(["caption"])
        .build();
    bar.append(&ul_icon);
    bar.append(&ul_label);

    // Active downloads count
    let active_label = gtk::Label::builder()
        .label("0 active")
        .css_classes(["caption", "dim-label"])
        .build();
    bar.append(&active_label);

    // Spacer
    let spacer = gtk::Box::builder().hexpand(true).build();
    bar.append(&spacer);

    // Connection status
    let conn_icon = gtk::Image::from_icon_name("network-transmit-receive-symbolic");
    let conn_label = gtk::Label::builder()
        .label("Online")
        .css_classes(["caption"])
        .build();
    bar.append(&conn_icon);
    bar.append(&conn_label);

    // Update stats on change
    {
        let dl_label = dl_label.clone();
        let ul_label = ul_label.clone();
        let active_label = active_label.clone();
        let conn_label = conn_label.clone();
        let conn_icon = conn_icon.clone();
        let model = model.clone();
        let model_for_closure = model.clone();
        model.connect_stats_changed(move || {
            let model = &model_for_closure;
            dl_label.set_label(&format_speed(model.download_speed()));
            ul_label.set_label(&format_speed(model.upload_speed()));
            let active = model.num_active();
            active_label.set_label(&format!("{} active", active));
            if model.is_connected() {
                conn_label.set_label("Online");
                conn_icon.set_icon_name(Some("network-transmit-receive-symbolic"));
            } else {
                conn_label.set_label("Offline");
                conn_icon.set_icon_name(Some("network-offline-symbolic"));
            }
        });
    }

    bar
}

pub fn format_speed(bytes_per_sec: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * KB;
    const GB: f64 = 1024.0 * MB;

    let b = bytes_per_sec as f64;
    if b >= GB {
        format!("{:.1} GB/s", b / GB)
    } else if b >= MB {
        format!("{:.1} MB/s", b / MB)
    } else if b >= KB {
        format!("{:.1} KB/s", b / KB)
    } else {
        format!("{} B/s", bytes_per_sec)
    }
}
