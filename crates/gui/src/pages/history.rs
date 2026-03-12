//! History page — table of completed downloads with search, filter, and actions.

use adw::prelude::*;
use gosh_fetch_engine::types::{Download, DownloadType};
use std::cell::RefCell;
use std::rc::Rc;

use crate::engine_bridge::EngineBridge;
use crate::model::AppModel;

pub fn build_history_page(model: &AppModel, bridge: &EngineBridge) -> gtk::Box {
    let page = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(0)
        .build();

    // Toolbar
    let toolbar = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(8)
        .build();

    let search_entry = gtk::SearchEntry::builder()
        .placeholder_text("Search history...")
        .hexpand(true)
        .build();
    toolbar.append(&search_entry);

    // Category filter
    let categories = ["All", "Documents", "Software", "Media", "Torrents"];
    let category_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(4)
        .css_classes(["linked"])
        .build();
    let active_category: Rc<RefCell<String>> = Rc::new(RefCell::new("All".to_string()));

    for cat in &categories {
        let btn = gtk::ToggleButton::builder()
            .label(*cat)
            .css_classes(["flat"])
            .build();
        if *cat == "All" { btn.set_active(true); }
        let category = cat.to_string();
        let active = active_category.clone();
        btn.connect_toggled(move |b| {
            if b.is_active() { *active.borrow_mut() = category.clone(); }
        });
        category_box.append(&btn);
    }
    toolbar.append(&category_box);

    // Clear all button
    let clear_btn = gtk::Button::builder()
        .label("Clear All")
        .css_classes(["destructive-action"])
        .build();
    {
        let bridge = bridge.clone();
        let page_ref = page.clone();
        clear_btn.connect_clicked(move |_| {
            let parent_window = page_ref.root().and_then(|r| r.downcast::<gtk::Window>().ok());
            let dialog = adw::MessageDialog::builder()
                .heading("Clear History")
                .body("Are you sure you want to clear all download history? This cannot be undone.")
                .modal(true)
                .build();
            if let Some(ref win) = parent_window {
                dialog.set_transient_for(Some(win));
            }
            dialog.add_response("cancel", "Cancel");
            dialog.add_response("clear", "Clear All");
            dialog.set_response_appearance("clear", adw::ResponseAppearance::Destructive);
            dialog.set_default_response(Some("cancel"));

            let bridge = bridge.clone();
            dialog.connect_response(None, move |_, response| {
                if response == "clear" {
                    bridge.clear_history();
                }
            });

            dialog.present();
        });
    }
    toolbar.append(&clear_btn);

    page.append(&toolbar);

    // History list
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .build();

    let list_box = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(12)
        .build();

    let empty = gtk::Label::builder()
        .label("No download history")
        .css_classes(["dim-label"])
        .margin_top(48)
        .margin_bottom(48)
        .build();
    list_box.set_placeholder(Some(&empty));

    scroll.set_child(Some(&list_box));
    page.append(&scroll);

    // Update list when history changes
    {
        let list_box = list_box.clone();
        let model = model.clone();
        let bridge = bridge.clone();
        let model_inner = model.clone();
        model.connect_history_changed(move || {
            rebuild_history_list(&list_box, &model_inner, &bridge);
        });
    }

    page
}

fn rebuild_history_list(list_box: &gtk::ListBox, model: &AppModel, bridge: &EngineBridge) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    let history = model.completed_history();
    for dl in history.iter() {
        let row = build_history_row(dl, bridge);
        list_box.append(&row);
    }
}

fn build_history_row(download: &Download, bridge: &EngineBridge) -> gtk::ListBoxRow {
    let hbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(8)
        .build();

    // Type icon
    let icon_name = match download.download_type {
        DownloadType::Http => "document-save-symbolic",
        DownloadType::Torrent | DownloadType::Magnet => "network-transmit-receive-symbolic",
    };
    let icon = gtk::Image::from_icon_name(icon_name);
    hbox.append(&icon);

    // Name + source domain
    let info_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();

    let name = gtk::Label::builder()
        .label(&download.name)
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    info_box.append(&name);

    if let Some(url) = &download.url {
        if let Ok(parsed) = url::Url::parse(url) {
            if let Some(domain) = parsed.host_str() {
                let domain_label = gtk::Label::builder()
                    .label(domain)
                    .xalign(0.0)
                    .css_classes(["caption", "dim-label"])
                    .build();
                info_box.append(&domain_label);
            }
        }
    }
    hbox.append(&info_box);

    // Size
    let size = if download.total_size > 0 {
        format_size(download.total_size)
    } else {
        "—".to_string()
    };
    let size_label = gtk::Label::builder()
        .label(&size)
        .css_classes(["caption"])
        .width_chars(10)
        .build();
    hbox.append(&size_label);

    // Date
    let date = download.completed_at.as_deref()
        .or(Some(&download.created_at))
        .unwrap_or("—");
    // Show just date portion
    let date_short = date.split('T').next().unwrap_or(date);
    let date_label = gtk::Label::builder()
        .label(date_short)
        .css_classes(["caption", "dim-label"])
        .width_chars(12)
        .build();
    hbox.append(&date_label);

    // Actions
    let open_folder_btn = gtk::Button::builder()
        .icon_name("folder-open-symbolic")
        .tooltip_text("Open Folder")
        .css_classes(["flat", "circular"])
        .build();
    {
        let path = download.save_path.clone();
        let bridge = bridge.clone();
        open_folder_btn.connect_clicked(move |_| {
            bridge.open_folder(path.clone());
        });
    }
    hbox.append(&open_folder_btn);

    let delete_btn = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .tooltip_text("Remove from history")
        .css_classes(["flat", "circular"])
        .build();
    {
        let gid = download.gid.clone();
        let bridge = bridge.clone();
        delete_btn.connect_clicked(move |_| {
            bridge.db_remove_download(&gid);
            // Refresh
            bridge.get_completed_history();
        });
    }
    hbox.append(&delete_btn);

    gtk::ListBoxRow::builder()
        .child(&hbox)
        .selectable(false)
        .activatable(false)
        .build()
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
