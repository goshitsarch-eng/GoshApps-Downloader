//! Add Download dialog — supports URL/magnet links and .torrent files with advanced options.

use adw::prelude::*;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;

use crate::engine_bridge::EngineBridge;
use crate::model::AppModel;

pub fn show_add_download_dialog(
    parent: &impl IsA<gtk::Widget>,
    _model: &AppModel,
    bridge: &EngineBridge,
) {
    let parent_window = parent.root().and_then(|r| r.downcast::<gtk::Window>().ok());
    let dialog = adw::Window::builder()
        .title("Add Download")
        .default_width(520)
        .default_height(500)
        .modal(true)
        .build();
    if let Some(ref win) = parent_window {
        dialog.set_transient_for(Some(win));
    }

    let toolbar_view = adw::ToolbarView::new();

    let header = adw::HeaderBar::builder()
        .show_title(true)
        .build();
    toolbar_view.add_top_bar(&header);

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_start(16)
        .margin_end(16)
        .margin_top(12)
        .margin_bottom(16)
        .build();

    // Mode toggle: Link vs Torrent File
    let mode_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(4)
        .halign(gtk::Align::Center)
        .css_classes(["linked"])
        .build();

    let link_btn = gtk::ToggleButton::builder()
        .label("URL / Magnet")
        .active(true)
        .build();
    let torrent_btn = gtk::ToggleButton::builder()
        .label("Torrent File")
        .group(&link_btn)
        .build();
    mode_box.append(&link_btn);
    mode_box.append(&torrent_btn);
    content.append(&mode_box);

    // --- Link mode ---
    let link_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .build();

    let url_label = gtk::Label::builder()
        .label("Enter URL(s) or magnet link(s), one per line:")
        .xalign(0.0)
        .css_classes(["caption"])
        .build();
    link_box.append(&url_label);

    let url_scroll = gtk::ScrolledWindow::builder()
        .height_request(120)
        .build();
    let url_text_view = gtk::TextView::builder()
        .wrap_mode(gtk::WrapMode::WordChar)
        .css_classes(["card"])
        .build();
    url_scroll.set_child(Some(&url_text_view));
    link_box.append(&url_scroll);

    // Paste button
    let paste_btn = gtk::Button::builder()
        .label("Paste from Clipboard")
        .css_classes(["flat"])
        .halign(gtk::Align::Start)
        .build();
    {
        let url_text_view = url_text_view.clone();
        paste_btn.connect_clicked(move |btn| {
            let display = btn.display();
            let clipboard = display.clipboard();
            let buffer = url_text_view.buffer();
            clipboard.read_text_async(gio::Cancellable::NONE, glib::clone!(
                #[weak] buffer,
                move |result| {
                    if let Ok(Some(text)) = result {
                        buffer.set_text(&text);
                    }
                }
            ));
        });
    }
    link_box.append(&paste_btn);
    content.append(&link_box);

    // --- Torrent file mode ---
    let torrent_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .visible(false)
        .build();

    let torrent_path: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

    let torrent_label = gtk::Label::builder()
        .label("No file selected")
        .css_classes(["dim-label"])
        .build();
    torrent_box.append(&torrent_label.clone());

    let browse_btn = gtk::Button::builder()
        .label("Browse for .torrent file")
        .css_classes(["flat"])
        .build();
    {
        let torrent_label = torrent_label.clone();
        let torrent_path = torrent_path.clone();
        let dialog_ref = dialog.clone();
        browse_btn.connect_clicked(move |_| {
            let file_dialog = gtk::FileDialog::builder()
                .title("Select Torrent File")
                .build();
            let filter = gtk::FileFilter::new();
            filter.add_pattern("*.torrent");
            filter.set_name(Some("Torrent Files"));
            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            file_dialog.set_filters(Some(&filters));

            let torrent_label = torrent_label.clone();
            let torrent_path = torrent_path.clone();
            file_dialog.open(
                Some(&dialog_ref),
                gio::Cancellable::NONE,
                move |result| {
                    if let Ok(file) = result {
                        if let Some(path) = file.path() {
                            let path_str = path.to_string_lossy().to_string();
                            torrent_label.set_label(&path_str);
                            *torrent_path.borrow_mut() = path_str;
                        }
                    }
                },
            );
        });
    }
    torrent_box.append(&browse_btn);
    content.append(&torrent_box);

    // Mode toggle handler
    {
        let link_box = link_box.clone();
        let torrent_box = torrent_box.clone();
        link_btn.connect_toggled(move |btn| {
            link_box.set_visible(btn.is_active());
            torrent_box.set_visible(!btn.is_active());
        });
    }

    // --- Advanced options (collapsed by default) ---
    let _expander = adw::ExpanderRow::builder()
        .title("Advanced Options")
        .show_enable_switch(false)
        .build();

    let advanced_group = adw::PreferencesGroup::new();

    // Save directory
    let dir_row = adw::EntryRow::builder()
        .title("Save Directory")
        .build();
    advanced_group.add(&dir_row);

    // Output filename
    let filename_row = adw::EntryRow::builder()
        .title("Rename File")
        .build();
    advanced_group.add(&filename_row);

    // Priority
    let priority_row = adw::ComboRow::builder()
        .title("Priority")
        .model(&gtk::StringList::new(&["Normal", "Low", "High", "Critical"]))
        .build();
    advanced_group.add(&priority_row);

    // Connections
    let connections_row = adw::SpinRow::builder()
        .title("Connections")
        .adjustment(&gtk::Adjustment::new(8.0, 1.0, 32.0, 1.0, 4.0, 0.0))
        .build();
    advanced_group.add(&connections_row);

    // Speed limit
    let speed_row = adw::SpinRow::builder()
        .title("Speed Limit (KB/s)")
        .subtitle("0 = unlimited")
        .adjustment(&gtk::Adjustment::new(0.0, 0.0, 1_000_000.0, 100.0, 1000.0, 0.0))
        .build();
    advanced_group.add(&speed_row);

    // Checksum
    let checksum_row = adw::EntryRow::builder()
        .title("Checksum (sha256:hex or md5:hex)")
        .build();
    advanced_group.add(&checksum_row);

    // Sequential (for torrents)
    let sequential_row = adw::SwitchRow::builder()
        .title("Sequential Download")
        .subtitle("Download pieces in order (useful for streaming)")
        .active(false)
        .build();
    advanced_group.add(&sequential_row);

    // Custom headers
    let headers_row = adw::EntryRow::builder()
        .title("Custom Headers (one per line: Key: Value)")
        .build();
    advanced_group.add(&headers_row);

    // Wrap advanced_group in an expander
    let advanced_expander = gtk::Expander::builder()
        .label("Advanced Options")
        .build();
    advanced_expander.set_child(Some(&advanced_group));
    content.append(&advanced_expander);

    // --- Action buttons ---
    let button_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .margin_top(12)
        .halign(gtk::Align::End)
        .build();

    let cancel_btn = gtk::Button::builder()
        .label("Cancel")
        .build();
    {
        let dialog = dialog.clone();
        cancel_btn.connect_clicked(move |_| dialog.close());
    }
    button_box.append(&cancel_btn);

    let add_btn = gtk::Button::builder()
        .label("Add Download")
        .css_classes(["suggested-action"])
        .build();
    {
        let dialog = dialog.clone();
        let bridge = bridge.clone();
        let url_text_view = url_text_view.clone();
        let link_btn = link_btn.clone();
        let torrent_path = torrent_path.clone();
        let dir_row = dir_row.clone();
        let filename_row = filename_row.clone();
        let priority_row = priority_row.clone();
        let connections_row = connections_row.clone();
        let speed_row = speed_row.clone();
        let checksum_row = checksum_row.clone();
        let sequential_row = sequential_row.clone();

        add_btn.connect_clicked(move |_| {
            let priority = match priority_row.selected() {
                1 => "low",
                2 => "high",
                3 => "critical",
                _ => "normal",
            }.to_string();

            let speed_limit = speed_row.value() as u64;
            let speed_limit_str = if speed_limit > 0 {
                Some(format!("{}K", speed_limit))
            } else {
                None
            };

            let options = gosh_fetch_engine::types::DownloadOptions {
                dir: {
                    let t = dir_row.text().to_string();
                    if t.is_empty() { None } else { Some(t) }
                },
                out: {
                    let t = filename_row.text().to_string();
                    if t.is_empty() { None } else { Some(t) }
                },
                split: Some(connections_row.value().to_string()),
                priority: Some(priority),
                max_download_limit: speed_limit_str,
                checksum: {
                    let t = checksum_row.text().to_string();
                    if t.is_empty() { None } else { Some(t) }
                },
                sequential: if sequential_row.is_active() { Some(true) } else { None },
                ..Default::default()
            };

            if link_btn.is_active() {
                // URL/Magnet mode
                let buffer = url_text_view.buffer();
                let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
                let urls: Vec<String> = text.lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();

                for url in urls {
                    if url.to_lowercase().starts_with("magnet:") {
                        bridge.add_magnet(url, Some(options.clone()));
                    } else {
                        bridge.add_download(url, Some(options.clone()));
                    }
                }
            } else {
                // Torrent file mode
                let path = torrent_path.borrow().clone();
                if !path.is_empty() {
                    bridge.add_torrent_file(path, Some(options));
                }
            }

            dialog.close();
        });
    }
    button_box.append(&add_btn);

    content.append(&button_box);

    toolbar_view.set_content(Some(&content));
    dialog.set_content(Some(&toolbar_view));
    dialog.present();
}
