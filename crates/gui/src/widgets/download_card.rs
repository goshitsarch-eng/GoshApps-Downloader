//! Download card widget showing progress, speed, ETA, and action buttons.

use adw::prelude::*;
use gtk::{gdk, gio};
use gosh_fetch_engine::types::{Download, DownloadState, DownloadType};

use crate::engine_bridge::EngineBridge;
use crate::widgets::status_bar::format_speed;

pub fn build_download_card(download: &Download, bridge: &EngineBridge) -> gtk::Box {
    let status_class = match download.status {
        DownloadState::Active => "download-status-active",
        DownloadState::Paused => "download-status-paused",
        DownloadState::Error => "download-status-error",
        DownloadState::Complete => "download-status-complete",
        _ => "card",
    };
    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(8)
        .css_classes(["card", status_class])
        .build();

    // Top row: drag handle + icon + name + type badge
    let top_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();

    // Drag handle for reordering
    let drag_handle = gtk::Image::builder()
        .icon_name("list-drag-handle-symbolic")
        .tooltip_text("Drag to reorder")
        .css_classes(["dim-label"])
        .build();
    top_row.append(&drag_handle);

    let icon_name = match download.download_type {
        DownloadType::Http => "document-save-symbolic",
        DownloadType::Torrent => "network-transmit-receive-symbolic",
        DownloadType::Magnet => "emblem-shared-symbolic",
    };
    let icon = gtk::Image::from_icon_name(icon_name);
    top_row.append(&icon);

    let name_label = gtk::Label::builder()
        .label(&download.name)
        .hexpand(true)
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::Middle)
        .css_classes(["heading"])
        .build();
    top_row.append(&name_label);

    // Type badge
    let type_label = gtk::Label::builder()
        .label(&download.download_type.to_string().to_uppercase())
        .css_classes(["caption", "badge"])
        .build();
    top_row.append(&type_label);

    card.append(&top_row);

    // Progress bar
    let progress = if download.total_size > 0 {
        download.completed_size as f64 / download.total_size as f64
    } else {
        0.0
    };
    let progress_bar = gtk::ProgressBar::builder()
        .fraction(progress)
        .show_text(true)
        .text(&format!("{:.1}%", progress * 100.0))
        .build();
    match download.status {
        DownloadState::Active => progress_bar.add_css_class("active"),
        DownloadState::Paused => progress_bar.add_css_class("paused"),
        _ => {}
    }
    card.append(&progress_bar);

    // Info row: size + speed + ETA
    let info_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(16)
        .build();

    let size_text = format!(
        "{} / {}",
        format_size(download.completed_size),
        if download.total_size > 0 { format_size(download.total_size) } else { "Unknown".to_string() }
    );
    let size_label = gtk::Label::builder()
        .label(&size_text)
        .css_classes(["caption"])
        .build();
    info_row.append(&size_label);

    let percent_label = gtk::Label::builder()
        .label(&format!("{:.1}%", progress * 100.0))
        .css_classes(["caption"])
        .build();
    info_row.append(&percent_label);

    let spacer = gtk::Box::builder().hexpand(true).build();
    info_row.append(&spacer);

    if download.status == DownloadState::Active {
        let speed_label = gtk::Label::builder()
            .label(&format_speed(download.download_speed))
            .css_classes(["caption"])
            .build();
        info_row.append(&speed_label);

        let eta = format_eta(download.total_size, download.completed_size, download.download_speed);
        let eta_label = gtk::Label::builder()
            .label(&eta)
            .css_classes(["caption", "dim-label"])
            .build();
        info_row.append(&eta_label);
    } else {
        let status_label = gtk::Label::builder()
            .label(&format_status(download.status))
            .css_classes(["caption"])
            .build();
        info_row.append(&status_label);
    }

    card.append(&info_row);

    // Action buttons
    let action_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(4)
        .halign(gtk::Align::End)
        .build();

    match download.status {
        DownloadState::Active => {
            let pause_btn = gtk::Button::builder()
                .icon_name("media-playback-pause-symbolic")
                .tooltip_text("Pause")
                .css_classes(["flat", "circular"])
                .build();
            let gid = download.gid.clone();
            let bridge = bridge.clone();
            pause_btn.connect_clicked(move |_| {
                bridge.pause_download(&gid);
            });
            action_row.append(&pause_btn);
        }
        DownloadState::Paused | DownloadState::Error => {
            let resume_btn = gtk::Button::builder()
                .icon_name("media-playback-start-symbolic")
                .tooltip_text("Resume")
                .css_classes(["flat", "circular"])
                .build();
            let gid = download.gid.clone();
            let bridge = bridge.clone();
            resume_btn.connect_clicked(move |_| {
                bridge.resume_download(&gid);
            });
            action_row.append(&resume_btn);
        }
        _ => {}
    }

    // Remove button
    let remove_btn = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .tooltip_text("Remove")
        .css_classes(["flat", "circular"])
        .build();
    {
        let gid = download.gid.clone();
        let bridge = bridge.clone();
        remove_btn.connect_clicked(move |_| {
            bridge.remove_download(&gid, false);
        });
    }
    action_row.append(&remove_btn);

    card.append(&action_row);

    // Right-click context menu
    let menu = gio::Menu::new();
    match download.status {
        DownloadState::Active => {
            menu.append(Some("Pause"), Some("download.pause"));
        }
        DownloadState::Paused | DownloadState::Error => {
            menu.append(Some("Resume"), Some("download.resume"));
        }
        _ => {}
    }
    menu.append(Some("Open Folder"), Some("download.open-folder"));
    menu.append(Some("Copy URL"), Some("download.copy-url"));
    menu.append(Some("Remove"), Some("download.remove"));

    let popover = gtk::PopoverMenu::from_model(Some(&menu));
    popover.set_parent(&card);

    // Context menu actions
    let action_group = gio::SimpleActionGroup::new();

    let pause_action = gio::SimpleAction::new("pause", None);
    {
        let gid = download.gid.clone();
        let bridge = bridge.clone();
        pause_action.connect_activate(move |_, _| bridge.pause_download(&gid));
    }
    action_group.add_action(&pause_action);

    let resume_action = gio::SimpleAction::new("resume", None);
    {
        let gid = download.gid.clone();
        let bridge = bridge.clone();
        resume_action.connect_activate(move |_, _| bridge.resume_download(&gid));
    }
    action_group.add_action(&resume_action);

    let open_action = gio::SimpleAction::new("open-folder", None);
    {
        let path = download.save_path.clone();
        let bridge = bridge.clone();
        open_action.connect_activate(move |_, _| bridge.open_folder(path.clone()));
    }
    action_group.add_action(&open_action);

    let copy_action = gio::SimpleAction::new("copy-url", None);
    {
        let url = download.url.clone().unwrap_or_default();
        copy_action.connect_activate(move |_, _| {
            if let Some(display) = gdk::Display::default() {
                display.clipboard().set_text(&url);
            }
        });
    }
    action_group.add_action(&copy_action);

    let remove_action = gio::SimpleAction::new("remove", None);
    {
        let gid = download.gid.clone();
        let bridge = bridge.clone();
        remove_action.connect_activate(move |_, _| bridge.remove_download(&gid, false));
    }
    action_group.add_action(&remove_action);

    card.insert_action_group("download", Some(&action_group));

    // Right-click gesture
    let click = gtk::GestureClick::builder()
        .button(3) // right-click
        .build();
    {
        let popover = popover.clone();
        click.connect_pressed(move |gesture, _, x, y| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
            popover.popup();
        });
    }
    card.add_controller(click);

    card
}

fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * KB;
    const GB: f64 = 1024.0 * MB;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

fn format_eta(total: u64, completed: u64, speed: u64) -> String {
    if speed == 0 || total <= completed {
        return String::new();
    }
    let remaining = total - completed;
    let seconds = remaining / speed;
    if seconds >= 3600 {
        format!("{}h {}m left", seconds / 3600, (seconds % 3600) / 60)
    } else if seconds >= 60 {
        format!("{}m {}s left", seconds / 60, seconds % 60)
    } else {
        format!("{}s left", seconds)
    }
}

fn format_status(status: DownloadState) -> String {
    match status {
        DownloadState::Active => "Downloading".to_string(),
        DownloadState::Waiting => "Queued".to_string(),
        DownloadState::Paused => "Paused".to_string(),
        DownloadState::Complete => "Complete".to_string(),
        DownloadState::Error => "Error".to_string(),
        DownloadState::Removed => "Removed".to_string(),
    }
}
