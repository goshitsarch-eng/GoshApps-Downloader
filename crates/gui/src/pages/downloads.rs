//! Downloads page — active downloads (full cards) + paused/completed (compact rows).
//! Includes search, category filtering, and drag-drop reordering.

use adw::prelude::*;
use gosh_fetch_engine::types::{Download, DownloadState};
use std::cell::RefCell;
use std::rc::Rc;

use crate::engine_bridge::EngineBridge;
use crate::model::AppModel;
use crate::widgets::{download_card, compact_download_row};

pub fn build_downloads_page(model: &AppModel, bridge: &EngineBridge) -> gtk::Box {
    let page = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(0)
        .build();

    // Toolbar: search + category filter + batch actions
    let toolbar = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(8)
        .build();

    let search_entry = gtk::SearchEntry::builder()
        .placeholder_text("Search downloads...")
        .hexpand(true)
        .build();
    toolbar.append(&search_entry);

    // Category filter
    let categories = ["All", "Video", "Audio", "Documents", "Software", "Images"];
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
        if *cat == "All" {
            btn.set_active(true);
        }
        let category = cat.to_string();
        let active = active_category.clone();
        btn.connect_toggled(move |b| {
            if b.is_active() {
                *active.borrow_mut() = category.clone();
            }
        });
        category_box.append(&btn);
    }
    toolbar.append(&category_box);

    // Batch action buttons
    let pause_all_btn = gtk::Button::builder()
        .icon_name("media-playback-pause-symbolic")
        .tooltip_text("Pause All")
        .css_classes(["flat"])
        .build();
    {
        let bridge = bridge.clone();
        pause_all_btn.connect_clicked(move |_| bridge.pause_all());
    }
    toolbar.append(&pause_all_btn);

    let resume_all_btn = gtk::Button::builder()
        .icon_name("media-playback-start-symbolic")
        .tooltip_text("Resume All")
        .css_classes(["flat"])
        .build();
    {
        let bridge = bridge.clone();
        resume_all_btn.connect_clicked(move |_| bridge.resume_all());
    }
    toolbar.append(&resume_all_btn);

    page.append(&toolbar);

    // Scrolled content
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .build();

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(0)
        .build();

    // Active downloads section
    let active_header = gtk::Label::builder()
        .label("Active Downloads")
        .css_classes(["heading"])
        .xalign(0.0)
        .margin_start(12)
        .margin_top(8)
        .margin_bottom(4)
        .build();
    content.append(&active_header);

    let active_list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .margin_start(12)
        .margin_end(12)
        .build();
    content.append(&active_list);

    // Paused / Recent section
    let paused_header = gtk::Label::builder()
        .label("Paused & Recent")
        .css_classes(["heading"])
        .xalign(0.0)
        .margin_start(12)
        .margin_top(16)
        .margin_bottom(4)
        .build();
    content.append(&paused_header);

    let paused_list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(12)
        .build();
    content.append(&paused_list);

    // Empty state
    let empty_state = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .margin_top(48)
        .margin_bottom(48)
        .visible(false)
        .build();

    let empty_icon = gtk::Image::builder()
        .icon_name("folder-download-symbolic")
        .pixel_size(64)
        .css_classes(["dim-label"])
        .build();
    empty_state.append(&empty_icon);

    let empty_label = gtk::Label::builder()
        .label("No downloads yet")
        .css_classes(["title-2", "dim-label"])
        .build();
    empty_state.append(&empty_label);

    let empty_sub = gtk::Label::builder()
        .label("Add a download with Ctrl+N or the + button")
        .css_classes(["dim-label"])
        .build();
    empty_state.append(&empty_sub);

    content.append(&empty_state);
    scroll.set_child(Some(&content));
    page.append(&scroll);

    // Update the download lists when data changes
    {
        let active_list = active_list.clone();
        let paused_list = paused_list.clone();
        let active_header = active_header.clone();
        let paused_header = paused_header.clone();
        let empty_state = empty_state.clone();
        let search_entry = search_entry.clone();
        let active_category = active_category.clone();
        let model = model.clone();
        let bridge = bridge.clone();
        let model_inner = model.clone();

        model.connect_downloads_changed(move || {
            rebuild_download_lists(
                &active_list,
                &paused_list,
                &active_header,
                &paused_header,
                &empty_state,
                &model_inner,
                &bridge,
                &search_entry.text(),
                &active_category.borrow(),
            );
        });
    }

    // Re-filter on search change
    {
        let model = model.clone();
        let bridge = bridge.clone();
        let active_list = active_list.clone();
        let paused_list = paused_list.clone();
        let active_header = active_header.clone();
        let paused_header = paused_header.clone();
        let empty_state = empty_state.clone();
        let active_category = active_category.clone();
        search_entry.connect_search_changed(move |entry| {
            rebuild_download_lists(
                &active_list,
                &paused_list,
                &active_header,
                &paused_header,
                &empty_state,
                &model,
                &bridge,
                &entry.text(),
                &active_category.borrow(),
            );
        });
    }

    page
}

fn rebuild_download_lists(
    active_list: &gtk::ListBox,
    paused_list: &gtk::ListBox,
    active_header: &gtk::Label,
    paused_header: &gtk::Label,
    empty_state: &gtk::Box,
    model: &AppModel,
    bridge: &EngineBridge,
    search_text: &str,
    category: &str,
) {
    // Clear existing rows
    while let Some(child) = active_list.first_child() {
        active_list.remove(&child);
    }
    while let Some(child) = paused_list.first_child() {
        paused_list.remove(&child);
    }

    let downloads = model.downloads();
    let search_lower = search_text.to_lowercase();

    let filtered: Vec<&Download> = downloads.iter().filter(|d| {
        // Search filter
        if !search_lower.is_empty() {
            let name_match = d.name.to_lowercase().contains(&search_lower);
            let url_match = d.url.as_ref().map_or(false, |u| u.to_lowercase().contains(&search_lower));
            if !name_match && !url_match {
                return false;
            }
        }
        // Category filter
        if category != "All" {
            let ext = d.name.rsplit('.').next().unwrap_or("").to_lowercase();
            let matches = match category {
                "Video" => matches!(ext.as_str(), "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv"),
                "Audio" => matches!(ext.as_str(), "mp3" | "flac" | "wav" | "ogg" | "aac" | "m4a"),
                "Documents" => matches!(ext.as_str(), "pdf" | "doc" | "docx" | "txt" | "odt" | "epub"),
                "Software" => matches!(ext.as_str(), "exe" | "msi" | "deb" | "rpm" | "appimage" | "dmg" | "pkg"),
                "Images" => matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp"),
                _ => true,
            };
            if !matches {
                return false;
            }
        }
        true
    }).collect();

    if filtered.is_empty() {
        empty_state.set_visible(true);
        active_header.set_visible(false);
        paused_header.set_visible(false);
        return;
    }
    empty_state.set_visible(false);

    let mut has_active = false;
    let mut has_paused = false;

    // Collect active GIDs for reorder tracking
    let active_gids: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

    for dl in &filtered {
        match dl.status {
            DownloadState::Active | DownloadState::Waiting => {
                has_active = true;
                active_gids.borrow_mut().push(dl.gid.clone());
                let card = download_card::build_download_card(dl, bridge);
                let row = gtk::ListBoxRow::builder()
                    .child(&card)
                    .selectable(false)
                    .activatable(false)
                    .build();

                // Attach DragSource for reordering
                let drag_source = gtk::DragSource::new();
                drag_source.set_actions(gdk::DragAction::MOVE);
                let gid = dl.gid.clone();
                drag_source.connect_prepare(move |_src, _x, _y| {
                    Some(gdk::ContentProvider::for_value(&gid.to_value()))
                });
                row.add_controller(drag_source);

                active_list.append(&row);
            }
            _ => {
                has_paused = true;
                let compact = compact_download_row::build_compact_row(dl, bridge);
                let row = gtk::ListBoxRow::builder()
                    .child(&compact)
                    .selectable(false)
                    .activatable(false)
                    .build();
                paused_list.append(&row);
            }
        }
    }

    // Attach DropTarget to active_list for reorder drops
    if has_active {
        let drop_target = gtk::DropTarget::new(glib::Type::STRING, gdk::DragAction::MOVE);
        let model_for_drop = model.clone();
        let active_list_ref = active_list.clone();
        let active_gids_ref = active_gids.clone();
        drop_target.connect_drop(move |_target, value, _x, y| {
            let dragged_gid: String = match value.get::<String>() {
                Ok(g) => g,
                Err(_) => return false,
            };

            // Find the target row at the drop position
            let mut gids = active_gids_ref.borrow().clone();

            // Determine target index from y position
            let target_idx = {
                let mut idx = gids.len(); // default: append at end
                let mut row_opt = active_list_ref.first_child();
                let mut i = 0;
                while let Some(ref widget) = row_opt {
                    if let Some(row) = widget.downcast_ref::<gtk::ListBoxRow>() {
                        let row_height = row.height() as f64;
                        let row_mid = i as f64 * row_height + row_height / 2.0;
                        if y < row_mid {
                            idx = i;
                            break;
                        }
                    }
                    i += 1;
                    row_opt = widget.next_sibling();
                }
                idx
            };

            // Find source index
            let source_idx = match gids.iter().position(|g| g == &dragged_gid) {
                Some(i) => i,
                None => return false,
            };

            if source_idx == target_idx || (target_idx > 0 && source_idx == target_idx - 1) {
                return false; // No-op if dropping in same position
            }

            // Reorder the GID list
            let removed = gids.remove(source_idx);
            let insert_at = if target_idx > source_idx {
                target_idx - 1
            } else {
                target_idx
            };
            gids.insert(insert_at, removed);

            model_for_drop.set_gid_order(gids);
            true
        });
        active_list.add_controller(drop_target);
    }

    active_header.set_visible(has_active);
    paused_header.set_visible(has_paused);
}
