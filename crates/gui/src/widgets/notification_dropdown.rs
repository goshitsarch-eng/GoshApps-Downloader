//! Notification bell icon with dropdown popover.

use adw::prelude::*;

use crate::model::AppModel;

pub fn build_notification_button(model: &AppModel) -> gtk::MenuButton {
    let button = gtk::MenuButton::builder()
        .icon_name("bell-outline-symbolic")
        .tooltip_text("Notifications")
        .build();

    let popover = gtk::Popover::builder()
        .build();

    let popover_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .margin_start(8)
        .margin_end(8)
        .margin_top(8)
        .margin_bottom(8)
        .width_request(300)
        .build();

    // Header
    let header = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();

    let title = gtk::Label::builder()
        .label("Notifications")
        .css_classes(["heading"])
        .hexpand(true)
        .xalign(0.0)
        .build();
    header.append(&title);

    let mark_read_button = gtk::Button::builder()
        .label("Mark all read")
        .css_classes(["flat", "caption"])
        .build();
    {
        let model = model.clone();
        mark_read_button.connect_clicked(move |_| {
            model.mark_all_read();
        });
    }
    header.append(&mark_read_button);

    let clear_button = gtk::Button::builder()
        .label("Clear")
        .css_classes(["flat", "caption"])
        .build();
    {
        let model = model.clone();
        clear_button.connect_clicked(move |_| {
            model.clear_notifications();
        });
    }
    header.append(&clear_button);
    popover_box.append(&header);

    // Separator
    popover_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    // Notification list
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .max_content_height(300)
        .propagate_natural_height(true)
        .build();

    let list_box = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    let empty_label = gtk::Label::builder()
        .label("No notifications")
        .css_classes(["dim-label"])
        .margin_top(24)
        .margin_bottom(24)
        .build();

    // Set placeholder
    list_box.set_placeholder(Some(&empty_label));
    scroll.set_child(Some(&list_box));
    popover_box.append(&scroll);

    popover.set_child(Some(&popover_box));
    button.set_popover(Some(&popover));

    // Update notifications on change
    {
        let list_box = list_box.clone();
        let button = button.clone();
        let model = model.clone();
        let model_inner = model.clone();
        model.connect_notifications_changed(move || {
            let model = &model_inner;
            // Clear existing rows
            while let Some(child) = list_box.first_child() {
                list_box.remove(&child);
            }

            let notifications = model.notifications();
            for notif in notifications.iter().take(20) {
                let row = build_notification_row(notif);
                list_box.append(&row);
            }

            // Update badge
            let unread = model.unread_count();
            if unread > 0 {
                button.set_icon_name("bell-symbolic");
                button.set_tooltip_text(Some(&format!("{} unread notifications", unread)));
            } else {
                button.set_icon_name("bell-outline-symbolic");
                button.set_tooltip_text(Some("Notifications"));
            }
        });
    }

    button
}

fn build_notification_row(notif: &crate::model::AppNotification) -> gtk::ListBoxRow {
    let hbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .margin_start(8)
        .margin_end(8)
        .margin_top(6)
        .margin_bottom(6)
        .build();

    let icon_name = match notif.kind.as_str() {
        "completed" => "emblem-ok-symbolic",
        "failed" => "dialog-error-symbolic",
        "added" => "list-add-symbolic",
        "paused" => "media-playback-pause-symbolic",
        "resumed" => "media-playback-start-symbolic",
        _ => "dialog-information-symbolic",
    };

    let icon = gtk::Image::from_icon_name(icon_name);
    hbox.append(&icon);

    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();

    let title = gtk::Label::builder()
        .label(&notif.title)
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    if !notif.read {
        title.add_css_class("bold");
    }
    vbox.append(&title);

    let message = gtk::Label::builder()
        .label(&notif.message)
        .xalign(0.0)
        .css_classes(["caption", "dim-label"])
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    vbox.append(&message);
    hbox.append(&vbox);

    // Relative time
    let elapsed = chrono::Utc::now().signed_duration_since(notif.timestamp);
    let time_str = if elapsed.num_seconds() < 60 {
        "just now".to_string()
    } else if elapsed.num_minutes() < 60 {
        format!("{}m ago", elapsed.num_minutes())
    } else if elapsed.num_hours() < 24 {
        format!("{}h ago", elapsed.num_hours())
    } else {
        format!("{}d ago", elapsed.num_days())
    };
    let time_label = gtk::Label::builder()
        .label(&time_str)
        .css_classes(["caption", "dim-label"])
        .build();
    hbox.append(&time_label);

    gtk::ListBoxRow::builder().child(&hbox).build()
}
