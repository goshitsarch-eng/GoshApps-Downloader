//! Compact download row for paused/completed downloads.

use adw::prelude::*;
use gosh_fetch_engine::types::{Download, DownloadState};

use crate::engine_bridge::EngineBridge;

pub fn build_compact_row(download: &Download, bridge: &EngineBridge) -> gtk::Box {
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .margin_start(12)
        .margin_end(12)
        .margin_top(4)
        .margin_bottom(4)
        .build();

    // Status icon
    let icon_name = match download.status {
        DownloadState::Paused => "media-playback-pause-symbolic",
        DownloadState::Complete => "emblem-ok-symbolic",
        DownloadState::Error => "dialog-error-symbolic",
        DownloadState::Waiting => "content-loading-symbolic",
        _ => "document-save-symbolic",
    };
    let icon = gtk::Image::from_icon_name(icon_name);
    row.append(&icon);

    // Name
    let name = gtk::Label::builder()
        .label(&download.name)
        .hexpand(true)
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    row.append(&name);

    // Size
    let size = if download.total_size > 0 {
        format_size(download.total_size)
    } else {
        "—".to_string()
    };
    let size_label = gtk::Label::builder()
        .label(&size)
        .css_classes(["caption", "dim-label"])
        .build();
    row.append(&size_label);

    // Action button
    match download.status {
        DownloadState::Paused | DownloadState::Error => {
            let resume_btn = gtk::Button::builder()
                .icon_name("media-playback-start-symbolic")
                .css_classes(["flat", "circular"])
                .build();
            let gid = download.gid.clone();
            let bridge = bridge.clone();
            resume_btn.connect_clicked(move |_| {
                bridge.resume_download(&gid);
            });
            row.append(&resume_btn);
        }
        _ => {}
    }

    let remove_btn = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .css_classes(["flat", "circular"])
        .build();
    {
        let gid = download.gid.clone();
        let bridge = bridge.clone();
        remove_btn.connect_clicked(move |_| {
            bridge.remove_download(&gid, false);
        });
    }
    row.append(&remove_btn);

    row
}

fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * KB;
    const GB: f64 = 1024.0 * MB;
    let b = bytes as f64;
    if b >= GB { format!("{:.2} GB", b / GB) }
    else if b >= MB { format!("{:.1} MB", b / MB) }
    else if b >= KB { format!("{:.1} KB", b / KB) }
    else { format!("{} B", bytes) }
}
