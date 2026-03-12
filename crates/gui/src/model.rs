//! Application model — shared state that replaces Redux.
//!
//! Holds the download list, global stats, settings, and notifications.
//! Widgets bind to this model and receive updates via GObject signals.

use gosh_fetch_engine::types::Download;
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::sync::Arc;

use crate::engine_bridge::{EngineBridge, EngineEvent};

/// Speed sample for the statistics chart.
#[derive(Debug, Clone)]
pub struct SpeedSample {
    pub timestamp: f64,
    pub download_speed: u64,
    pub upload_speed: u64,
}

/// In-app notification entry.
#[derive(Debug, Clone)]
pub struct AppNotification {
    pub id: u64,
    pub kind: String,          // "added", "completed", "failed", "paused", "resumed"
    pub title: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub read: bool,
}

/// The central application model.
#[derive(Clone)]
pub struct AppModel {
    inner: Arc<AppModelInner>,
}

struct AppModelInner {
    bridge: EngineBridge,

    // Downloads (active, from engine)
    downloads: RefCell<Vec<Download>>,
    // Completed history (from database)
    completed_history: RefCell<Vec<Download>>,
    // Download ordering for drag-drop
    gid_order: RefCell<Vec<String>>,

    // Global stats
    download_speed: Cell<u64>,
    upload_speed: Cell<u64>,
    num_active: Cell<u32>,
    num_waiting: Cell<u32>,
    num_stopped: Cell<u32>,
    is_connected: Cell<bool>,

    // Speed samples for statistics chart
    speed_samples: RefCell<VecDeque<SpeedSample>>,

    // Notifications
    notifications: RefCell<Vec<AppNotification>>,
    next_notification_id: Cell<u64>,

    // Settings
    close_to_tray: Cell<bool>,

    // Callbacks for UI updates
    on_downloads_changed: RefCell<Vec<Box<dyn Fn()>>>,
    on_stats_changed: RefCell<Vec<Box<dyn Fn()>>>,
    on_history_changed: RefCell<Vec<Box<dyn Fn()>>>,
    on_notifications_changed: RefCell<Vec<Box<dyn Fn()>>>,
}

// SAFETY: AppModelInner contains RefCell and Cell types which are !Sync.
// This is safe because GTK is single-threaded: all UI callbacks and
// glib::spawn_future_local futures execute on the main thread. The Arc
// wrapper exists only so that closures connected to signals can hold
// a shared reference; no cross-thread access ever occurs.
// The EngineBridge field *is* Send+Sync (it uses mpsc channels internally).
unsafe impl Send for AppModelInner {}
unsafe impl Sync for AppModelInner {}

impl std::fmt::Debug for AppModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppModel").finish()
    }
}

impl AppModel {
    pub fn new(bridge: EngineBridge) -> Self {
        Self {
            inner: Arc::new(AppModelInner {
                bridge,
                downloads: RefCell::new(Vec::new()),
                completed_history: RefCell::new(Vec::new()),
                gid_order: RefCell::new(Vec::new()),
                download_speed: Cell::new(0),
                upload_speed: Cell::new(0),
                num_active: Cell::new(0),
                num_waiting: Cell::new(0),
                num_stopped: Cell::new(0),
                is_connected: Cell::new(true),
                speed_samples: RefCell::new(VecDeque::with_capacity(1200)),
                notifications: RefCell::new(Vec::new()),
                next_notification_id: Cell::new(1),
                close_to_tray: Cell::new(true),
                on_downloads_changed: RefCell::new(Vec::new()),
                on_stats_changed: RefCell::new(Vec::new()),
                on_history_changed: RefCell::new(Vec::new()),
                on_notifications_changed: RefCell::new(Vec::new()),
            }),
        }
    }

    pub fn bridge(&self) -> &EngineBridge {
        &self.inner.bridge
    }

    // --- Downloads ---

    pub fn downloads(&self) -> std::cell::Ref<'_, Vec<Download>> {
        self.inner.downloads.borrow()
    }

    pub fn set_downloads(&self, downloads: Vec<Download>) {
        *self.inner.downloads.borrow_mut() = downloads;
        self.notify_downloads_changed();
    }

    pub fn update_download(&self, updated: Download) {
        let mut downloads = self.inner.downloads.borrow_mut();
        if let Some(dl) = downloads.iter_mut().find(|d| d.gid == updated.gid) {
            *dl = updated;
        } else {
            downloads.push(updated);
        }
        drop(downloads);
        self.notify_downloads_changed();
    }

    pub fn remove_download_from_list(&self, gid: &str) {
        self.inner.downloads.borrow_mut().retain(|d| d.gid != gid);
        self.notify_downloads_changed();
    }

    // --- History ---

    pub fn completed_history(&self) -> std::cell::Ref<'_, Vec<Download>> {
        self.inner.completed_history.borrow()
    }

    pub fn set_completed_history(&self, history: Vec<Download>) {
        *self.inner.completed_history.borrow_mut() = history;
        self.notify_history_changed();
    }

    // --- Ordering ---

    pub fn gid_order(&self) -> std::cell::Ref<'_, Vec<String>> {
        self.inner.gid_order.borrow()
    }

    pub fn set_gid_order(&self, order: Vec<String>) {
        *self.inner.gid_order.borrow_mut() = order;
        self.notify_downloads_changed();
    }

    // --- Stats ---

    pub fn download_speed(&self) -> u64 { self.inner.download_speed.get() }
    pub fn upload_speed(&self) -> u64 { self.inner.upload_speed.get() }
    pub fn num_active(&self) -> u32 { self.inner.num_active.get() }
    pub fn num_waiting(&self) -> u32 { self.inner.num_waiting.get() }
    pub fn num_stopped(&self) -> u32 { self.inner.num_stopped.get() }
    pub fn is_connected(&self) -> bool { self.inner.is_connected.get() }

    // --- Settings ---

    pub fn close_to_tray(&self) -> bool { self.inner.close_to_tray.get() }

    pub fn set_close_to_tray(&self, val: bool) { self.inner.close_to_tray.set(val); }

    // --- Speed Samples ---

    pub fn speed_samples(&self) -> std::cell::Ref<'_, VecDeque<SpeedSample>> {
        self.inner.speed_samples.borrow()
    }

    pub fn add_speed_sample(&self, sample: SpeedSample) {
        let mut samples = self.inner.speed_samples.borrow_mut();
        samples.push_back(sample);
        // Keep max ~20 minutes of samples at 3-second intervals
        while samples.len() > 400 {
            samples.pop_front();
        }
    }

    // --- Notifications ---

    pub fn notifications(&self) -> std::cell::Ref<'_, Vec<AppNotification>> {
        self.inner.notifications.borrow()
    }

    pub fn unread_count(&self) -> usize {
        self.inner.notifications.borrow().iter().filter(|n| !n.read).count()
    }

    pub fn add_notification(&self, kind: &str, title: &str, message: &str) {
        let id = self.inner.next_notification_id.get();
        self.inner.next_notification_id.set(id + 1);
        let notif = AppNotification {
            id,
            kind: kind.to_string(),
            title: title.to_string(),
            message: message.to_string(),
            timestamp: chrono::Utc::now(),
            read: false,
        };
        self.inner.notifications.borrow_mut().insert(0, notif);
        self.notify_notifications_changed();
    }

    pub fn mark_all_read(&self) {
        for n in self.inner.notifications.borrow_mut().iter_mut() {
            n.read = true;
        }
        self.notify_notifications_changed();
    }

    pub fn clear_notifications(&self) {
        self.inner.notifications.borrow_mut().clear();
        self.notify_notifications_changed();
    }

    // --- Change notification subscriptions ---

    pub fn connect_downloads_changed(&self, f: impl Fn() + 'static) {
        self.inner.on_downloads_changed.borrow_mut().push(Box::new(f));
    }

    pub fn connect_stats_changed(&self, f: impl Fn() + 'static) {
        self.inner.on_stats_changed.borrow_mut().push(Box::new(f));
    }

    pub fn connect_history_changed(&self, f: impl Fn() + 'static) {
        self.inner.on_history_changed.borrow_mut().push(Box::new(f));
    }

    pub fn connect_notifications_changed(&self, f: impl Fn() + 'static) {
        self.inner.on_notifications_changed.borrow_mut().push(Box::new(f));
    }

    fn notify_downloads_changed(&self) {
        for f in self.inner.on_downloads_changed.borrow().iter() {
            f();
        }
    }

    fn notify_stats_changed(&self) {
        for f in self.inner.on_stats_changed.borrow().iter() {
            f();
        }
    }

    fn notify_history_changed(&self) {
        for f in self.inner.on_history_changed.borrow().iter() {
            f();
        }
    }

    fn notify_notifications_changed(&self) {
        for f in self.inner.on_notifications_changed.borrow().iter() {
            f();
        }
    }

    // --- Engine event handler ---

    pub fn handle_engine_event(&self, event: EngineEvent) {
        match event {
            EngineEvent::Ready => {
                log::info!("Engine ready");
                self.inner.is_connected.set(true);
                // Load initial data
                self.inner.bridge.get_all_downloads();
                self.inner.bridge.get_completed_history();
                self.notify_stats_changed();
                // Load close-to-tray setting from persisted settings
                if let Some(data_dir) = dirs::data_dir() {
                    let settings_path = data_dir.join("com.goshapps.downloader/settings.json");
                    if let Ok(contents) = std::fs::read_to_string(&settings_path) {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&contents) {
                            if let Some(ctt) = val.get("close_to_tray").and_then(|v| v.as_bool()) {
                                self.set_close_to_tray(ctt);
                            }
                        }
                    }
                }
            }
            EngineEvent::InitError(msg) => {
                log::error!("Engine init failed: {}", msg);
                self.inner.is_connected.set(false);
                self.notify_stats_changed();
            }
            EngineEvent::GlobalStats(stats) => {
                self.inner.download_speed.set(stats.download_speed);
                self.inner.upload_speed.set(stats.upload_speed);
                self.inner.num_active.set(stats.num_active);
                self.inner.num_waiting.set(stats.num_waiting);
                self.inner.num_stopped.set(stats.num_stopped);
                self.inner.is_connected.set(true);
                self.notify_stats_changed();
            }
            EngineEvent::DownloadEvent { event_name, data } => {
                self.handle_download_event(&event_name, &data);
            }
            EngineEvent::CommandResult { id, result } => {
                self.handle_command_result(id, result);
            }
        }
    }

    fn handle_download_event(&self, event_name: &str, data: &Value) {
        // Refresh the full download list on significant events
        match event_name {
            "download:added" => {
                if let Some(name) = data.get("name").and_then(|v| v.as_str()) {
                    self.add_notification("added", "Download Added", name);
                }
                self.inner.bridge.get_all_downloads();
            }
            "download:completed" => {
                if let Some(name) = data.get("name").and_then(|v| v.as_str()) {
                    self.add_notification("completed", "Download Complete", name);
                }
                self.inner.bridge.get_all_downloads();
                self.inner.bridge.get_completed_history();
            }
            "download:failed" => {
                if let Some(name) = data.get("name").and_then(|v| v.as_str()) {
                    let msg = data.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
                    self.add_notification("failed", &format!("Failed: {}", name), msg);
                }
                self.inner.bridge.get_all_downloads();
            }
            "download:removed" => {
                self.inner.bridge.get_all_downloads();
            }
            "download:progress" | "download:state-changed" | "download:started"
            | "download:paused" | "download:resumed" => {
                // For progress updates, update the individual download in place
                if let Ok(dl) = serde_json::from_value::<Download>(data.clone()) {
                    self.update_download(dl);
                } else {
                    // Fallback: refresh all
                    self.inner.bridge.get_all_downloads();
                }
            }
            _ => {}
        }
    }

    fn handle_command_result(&self, _id: u64, result: crate::engine_bridge::CommandResult) {
        match result {
            crate::engine_bridge::CommandResult::ActiveDownloadsOk(val) => {
                if let Ok(downloads) = serde_json::from_value::<Vec<Download>>(val) {
                    self.set_downloads(downloads);
                }
            }
            crate::engine_bridge::CommandResult::HistoryOk(val) => {
                if let Ok(downloads) = serde_json::from_value::<Vec<Download>>(val) {
                    self.set_completed_history(downloads);
                }
            }
            crate::engine_bridge::CommandResult::Ok(val) => {
                // Generic result — try to interpret as Vec<Download> with heuristic
                if let Ok(downloads) = serde_json::from_value::<Vec<Download>>(val) {
                    if !downloads.is_empty() {
                        if downloads.iter().all(|d| d.status == gosh_fetch_engine::types::DownloadState::Complete) {
                            self.set_completed_history(downloads);
                        } else {
                            self.set_downloads(downloads);
                        }
                    }
                }
            }
            crate::engine_bridge::CommandResult::Err(msg) => {
                log::error!("Command error: {}", msg);
            }
        }
    }
}
