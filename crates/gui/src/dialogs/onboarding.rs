//! Onboarding wizard shown on first run.

use adw::prelude::*;

use crate::engine_bridge::EngineBridge;

pub fn show_onboarding(parent: &adw::ApplicationWindow, _bridge: &EngineBridge) {
    let dialog = adw::Window::builder()
        .title("Welcome to Goshapps Downloader")
        .default_width(500)
        .default_height(400)
        .modal(true)
        .transient_for(parent)
        .build();

    let toolbar_view = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    toolbar_view.add_top_bar(&header);

    let carousel = adw::Carousel::builder()
        .hexpand(true)
        .vexpand(true)
        .build();

    // Page 1: Welcome
    let page1 = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .build();

    let welcome_title = gtk::Label::builder()
        .label("Welcome to Goshapps Downloader")
        .css_classes(["title-1"])
        .build();
    page1.append(&welcome_title);

    let welcome_desc = gtk::Label::builder()
        .label("A fast, modern download manager powered by Rust")
        .css_classes(["dim-label"])
        .wrap(true)
        .justify(gtk::Justification::Center)
        .build();
    page1.append(&welcome_desc);

    carousel.append(&page1);

    // Page 2: Download Path
    let page2 = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .margin_start(32)
        .margin_end(32)
        .build();

    let setup_title = gtk::Label::builder()
        .label("Setup")
        .css_classes(["title-2"])
        .build();
    page2.append(&setup_title);

    let path_label = gtk::Label::builder()
        .label("Choose your default download location:")
        .build();
    page2.append(&path_label);

    let default_path = dirs::download_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "~/Downloads".to_string());

    let path_entry = adw::EntryRow::builder()
        .title("Download Path")
        .text(&default_path)
        .build();
    let prefs_group = adw::PreferencesGroup::new();
    prefs_group.add(&path_entry);
    page2.append(&prefs_group);

    carousel.append(&page2);

    // Page 3: Theme + Finish
    let page3 = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .margin_start(32)
        .margin_end(32)
        .build();

    let theme_title = gtk::Label::builder()
        .label("Choose Your Theme")
        .css_classes(["title-2"])
        .build();
    page3.append(&theme_title);

    let theme_combo = adw::ComboRow::builder()
        .title("Theme")
        .model(&gtk::StringList::new(&["System", "Light", "Dark"]))
        .build();
    theme_combo.set_selected(2); // Default to dark
    let theme_group = adw::PreferencesGroup::new();
    theme_group.add(&theme_combo);
    page3.append(&theme_group);

    let finish_btn = gtk::Button::builder()
        .label("Get Started")
        .css_classes(["suggested-action", "pill"])
        .halign(gtk::Align::Center)
        .margin_top(16)
        .build();
    {
        let dialog = dialog.clone();
        finish_btn.connect_clicked(move |_| {
            // Write onboarding-done marker file
            if let Some(data_dir) = dirs::data_dir() {
                let dir = data_dir.join("com.goshapps.downloader");
                let _ = std::fs::create_dir_all(&dir);
                let _ = std::fs::write(dir.join("onboarding-done"), "done");
            }
            dialog.close();
        });
    }
    page3.append(&finish_btn);

    carousel.append(&page3);

    // Carousel indicator dots
    let indicator = adw::CarouselIndicatorDots::builder()
        .carousel(&carousel)
        .build();

    let main_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    main_box.append(&carousel);
    main_box.append(&indicator);

    toolbar_view.set_content(Some(&main_box));
    dialog.set_content(Some(&toolbar_view));
    dialog.present();
}
