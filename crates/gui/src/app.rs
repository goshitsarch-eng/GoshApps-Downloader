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
            if let Some(display) = gdk::Display::default() {
                gtk::style_context_add_provider_for_display(
                    &display,
                    &provider,
                    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );
            } else {
                log::error!("No default display available — CSS will not be loaded");
            }

            // Initialize engine bridge and model on first activation
            if self.bridge.get().is_none() {
                let bridge = EngineBridge::new();
                let model = AppModel::new(bridge.clone());
                let _ = self.bridge.set(bridge.clone());
                let _ = self.model.set(model);

                // Start the engine
                let (sender, receiver) = async_channel::bounded(512);
                bridge.start(sender);

                // Forward engine events to the model
                if let Some(model_ref) = self.model.get().cloned() {
                    glib::spawn_future_local(async move {
                        while let Ok(event) = receiver.recv().await {
                            model_ref.handle_engine_event(event);
                        }
                    });
                }
            }

            // Create or present the window
            let window = if let Some(win) = app.active_window() {
                win
            } else if let (Some(model), Some(bridge)) = (self.model.get().cloned(), self.bridge.get().cloned()) {
                let win = crate::widgets::window::GoshFetchWindow::new(app.upcast_ref::<adw::Application>(), model, bridge);
                win.upcast()
            } else {
                log::error!("Model/bridge not initialized — cannot create window");
                return;
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
