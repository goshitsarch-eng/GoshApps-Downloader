use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gdk, gio, glib};
use std::cell::OnceCell;

use crate::engine_bridge::EngineBridge;
use crate::model::AppModel;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct GoshFetchApplication {
        pub model: OnceCell<AppModel>,
        pub bridge: OnceCell<EngineBridge>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GoshFetchApplication {
        const NAME: &'static str = "GoshFetchApplication";
        type Type = super::GoshFetchApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for GoshFetchApplication {}

    impl ApplicationImpl for GoshFetchApplication {
        fn activate(&self) {
            let app = self.obj();

            // Load application CSS
            let provider = gtk::CssProvider::new();
            provider.load_from_string(include_str!("css/style.css"));
            gtk::style_context_add_provider_for_display(
                &gdk::Display::default().unwrap(),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );

            // Initialize engine bridge and model on first activation
            if self.bridge.get().is_none() {
                let bridge = EngineBridge::new();
                let model = AppModel::new(bridge.clone());
                self.bridge.set(bridge.clone()).unwrap();
                self.model.set(model).unwrap();

                // Start the engine
                let (sender, receiver) = async_channel::unbounded();
                bridge.start(sender);

                // Forward engine events to the model
                let model_ref = self.model.get().unwrap().clone();
                glib::spawn_future_local(async move {
                    while let Ok(event) = receiver.recv().await {
                        model_ref.handle_engine_event(event);
                    }
                });
            }

            // Create or present the window
            let window = if let Some(win) = app.active_window() {
                win
            } else {
                let model = self.model.get().unwrap().clone();
                let bridge = self.bridge.get().unwrap().clone();
                let win = crate::widgets::window::GoshFetchWindow::new(app.upcast_ref::<adw::Application>(), model, bridge);
                win.upcast()
            };

            window.present();
        }
    }

    impl GtkApplicationImpl for GoshFetchApplication {}
    impl AdwApplicationImpl for GoshFetchApplication {}
}

glib::wrapper! {
    pub struct GoshFetchApplication(ObjectSubclass<imp::GoshFetchApplication>)
        @extends adw::Application, gtk::Application, gio::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl GoshFetchApplication {
    pub fn new(app_id: &str) -> Self {
        glib::Object::builder()
            .property("application-id", app_id)
            .property("flags", gio::ApplicationFlags::FLAGS_NONE)
            .build()
    }
}
