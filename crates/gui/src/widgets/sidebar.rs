//! Navigation sidebar with download counts and storage widget.

use adw::prelude::*;
use gtk::glib;

use crate::model::AppModel;

/// Build the sidebar navigation widget.
pub fn build_sidebar(model: &AppModel, stack: &gtk::Stack) -> gtk::Box {
    let sidebar_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(["sidebar"])
        .build();

    // Logo / App name
    let logo_label = gtk::Label::builder()
        .label("Goshapps Downloader")
        .css_classes(["title-2"])
        .margin_top(16)
        .margin_bottom(8)
        .build();
    sidebar_box.append(&logo_label);

    let version_label = gtk::Label::builder()
        .label("v3.0.0")
        .css_classes(["caption", "dim-label"])
        .margin_bottom(16)
        .build();
    sidebar_box.append(&version_label);

    // Navigation list
    let nav_list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .css_classes(["navigation-sidebar"])
        .vexpand(true)
        .build();

    // Navigation items
    let nav_items = vec![
        ("downloads", "view-list-symbolic", "Downloads"),
        ("history", "document-open-recent-symbolic", "History"),
        ("statistics", "utilities-system-monitor-symbolic", "Statistics"),
        ("scheduler", "x-office-calendar-symbolic", "Scheduler"),
        ("settings", "emblem-system-symbolic", "Settings"),
    ];

    for (page_name, icon, label) in &nav_items {
        let row = build_nav_row(icon, label);
        row.set_widget_name(page_name);
        nav_list.append(&row);
    }

    // Connect navigation
    {
        let stack = stack.clone();
        nav_list.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                let name = row.widget_name();
                stack.set_visible_child_name(&name);
            }
        });
    }

    // Select the first row by default
    if let Some(first) = nav_list.row_at_index(0) {
        nav_list.select_row(Some(&first));
    }

    sidebar_box.append(&nav_list);

    // Download count badges - update them when downloads change
    {
        let nav_list_ref = nav_list.clone();
        let model = model.clone();
        let model_inner = model.clone();
        model.connect_downloads_changed(move || {
            update_nav_badges(&nav_list_ref, &model_inner);
        });
    }

    // Storage widget at bottom
    let storage_box = build_storage_widget(model);
    sidebar_box.append(&storage_box);

    sidebar_box
}

fn build_nav_row(icon_name: &str, label_text: &str) -> gtk::ListBoxRow {
    let hbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(8)
        .build();

    let icon = gtk::Image::from_icon_name(icon_name);
    hbox.append(&icon);

    let label = gtk::Label::builder()
        .label(label_text)
        .hexpand(true)
        .xalign(0.0)
        .build();
    hbox.append(&label);

    // Badge (hidden by default)
    let badge = gtk::Label::builder()
        .css_classes(["badge"])
        .visible(false)
        .build();
    badge.set_widget_name("badge");
    hbox.append(&badge);

    let row = gtk::ListBoxRow::builder()
        .child(&hbox)
        .build();

    row
}

fn update_nav_badges(nav_list: &gtk::ListBox, model: &AppModel) {
    let downloads = model.downloads();
    let active_count = downloads.iter().filter(|d| {
        d.status == gosh_fetch_engine::types::DownloadState::Active
    }).count();

    // Update the downloads row badge
    if let Some(row) = nav_list.row_at_index(0) {
        if let Some(hbox) = row.child().and_then(|c| c.downcast::<gtk::Box>().ok()) {
            // Find the badge label (third child)
            if let Some(badge) = hbox.last_child().and_then(|c| c.downcast::<gtk::Label>().ok()) {
                if badge.widget_name() == "badge" {
                    if active_count > 0 {
                        badge.set_label(&active_count.to_string());
                        badge.set_visible(true);
                    } else {
                        badge.set_visible(false);
                    }
                }
            }
        }
    }
}

fn build_storage_widget(_model: &AppModel) -> gtk::Box {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(16)
        .build();

    let label = gtk::Label::builder()
        .label("Storage")
        .css_classes(["caption", "dim-label"])
        .xalign(0.0)
        .build();
    vbox.append(&label);

    let level_bar = gtk::LevelBar::builder()
        .min_value(0.0)
        .max_value(1.0)
        .value(0.0)
        .build();
    vbox.append(&level_bar);

    let info_label = gtk::Label::builder()
        .label("Checking...")
        .css_classes(["caption"])
        .xalign(0.0)
        .build();
    vbox.append(&info_label);

    // Update storage info periodically
    {
        let level_bar = level_bar.clone();
        let info_label = info_label.clone();
        glib::timeout_add_seconds_local(10, move || {
            if let Some(download_dir) = dirs::download_dir() {
                if let Some(stat) = nix_statvfs(&download_dir) {
                    let total = stat.0;
                    let free = stat.1;
                    let used_fraction = if total > 0 {
                        1.0 - (free as f64 / total as f64)
                    } else {
                        0.0
                    };
                    level_bar.set_value(used_fraction);
                    info_label.set_label(&format!(
                        "{} free of {}",
                        format_bytes(free),
                        format_bytes(total)
                    ));
                }
            }
            glib::ControlFlow::Continue
        });
    }

    vbox
}

/// Get filesystem stats (total, free) in bytes using libc::statvfs.
fn nix_statvfs(path: &std::path::Path) -> Option<(u64, u64)> {
    use std::ffi::CString;
    let c_path = CString::new(path.to_str()?).ok()?;
    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
            let total = stat.f_blocks as u64 * stat.f_frsize as u64;
            let free = stat.f_bavail as u64 * stat.f_frsize as u64;
            Some((total, free))
        } else {
            None
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
