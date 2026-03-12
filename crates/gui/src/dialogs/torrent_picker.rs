//! Torrent file picker dialog — tree view for multi-file torrent selection.

use adw::prelude::*;
use gosh_fetch_engine::types::TorrentInfo;

#[allow(dead_code)]
pub fn show_torrent_picker(
    parent: &impl IsA<gtk::Widget>,
    info: &TorrentInfo,
    on_select: impl Fn(Vec<usize>) + 'static,
) {
    let parent_window = parent.root().and_then(|r| r.downcast::<gtk::Window>().ok());
    let dialog = adw::Window::builder()
        .title(&format!("Select Files - {}", info.name))
        .default_width(600)
        .default_height(500)
        .modal(true)
        .build();
    if let Some(ref win) = parent_window {
        dialog.set_transient_for(Some(win));
    }

    let toolbar_view = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    toolbar_view.add_top_bar(&header);

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(12)
        .build();

    // Summary
    let summary = gtk::Label::builder()
        .label(&format!(
            "{} files, {} total",
            info.files.len(),
            format_size(info.total_size)
        ))
        .css_classes(["caption", "dim-label"])
        .xalign(0.0)
        .build();
    content.append(&summary);

    // Select all / none buttons
    let action_bar = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();

    let select_all_btn = gtk::Button::builder()
        .label("Select All")
        .css_classes(["flat"])
        .build();
    action_bar.append(&select_all_btn);

    let select_none_btn = gtk::Button::builder()
        .label("Select None")
        .css_classes(["flat"])
        .build();
    action_bar.append(&select_none_btn);
    content.append(&action_bar);

    // File list
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .build();

    let list_box = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    // Track selected indices
    let selected: std::rc::Rc<std::cell::RefCell<Vec<bool>>> =
        std::rc::Rc::new(std::cell::RefCell::new(vec![true; info.files.len()]));

    for file in &info.files {
        let row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(4)
            .margin_bottom(4)
            .build();

        let check = gtk::CheckButton::builder()
            .active(true)
            .build();
        {
            let selected = selected.clone();
            let idx = file.index;
            check.connect_toggled(move |btn| {
                if let Some(s) = selected.borrow_mut().get_mut(idx) {
                    *s = btn.is_active();
                }
            });
        }
        row.append(&check);

        let name = gtk::Label::builder()
            .label(&file.path)
            .hexpand(true)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::Middle)
            .build();
        row.append(&name);

        let size = gtk::Label::builder()
            .label(&format_size(file.length))
            .css_classes(["caption", "dim-label"])
            .build();
        row.append(&size);

        let lbrow = gtk::ListBoxRow::builder()
            .child(&row)
            .selectable(false)
            .build();
        list_box.append(&lbrow);
    }

    scroll.set_child(Some(&list_box));
    content.append(&scroll);

    // Confirm button
    let buttons = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .halign(gtk::Align::End)
        .build();

    let cancel_btn = gtk::Button::builder().label("Cancel").build();
    {
        let dialog = dialog.clone();
        cancel_btn.connect_clicked(move |_| dialog.close());
    }
    buttons.append(&cancel_btn);

    let confirm_btn = gtk::Button::builder()
        .label("Download Selected")
        .css_classes(["suggested-action"])
        .build();
    {
        let dialog = dialog.clone();
        let selected = selected.clone();
        confirm_btn.connect_clicked(move |_| {
            let indices: Vec<usize> = selected.borrow().iter()
                .enumerate()
                .filter(|(_, &s)| s)
                .map(|(i, _)| i)
                .collect();
            on_select(indices);
            dialog.close();
        });
    }
    buttons.append(&confirm_btn);
    content.append(&buttons);

    toolbar_view.set_content(Some(&content));
    dialog.set_content(Some(&toolbar_view));
    dialog.present();
}

#[allow(dead_code)]
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
