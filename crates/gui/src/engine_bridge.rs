//! Bridge between the GTK main loop (glib) and the async engine (tokio).
//!
//! Spawns a dedicated tokio runtime on a background thread. Commands are sent
//! from the GTK thread via an mpsc channel; engine events are forwarded back
//! via a glib::Sender.

use async_channel::Sender as GlibSender;
use gosh_fetch_engine::{AppState, commands, db::Settings};
use gosh_fetch_engine::types::{Download, DownloadOptions, GlobalStat};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

/// Events sent from the engine thread to the GTK main thread.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    /// Engine initialized successfully
    Ready,
    /// Engine failed to initialize
    InitError(String),
    /// A download was added/started/progressed/completed/failed/etc.
    DownloadEvent { event_name: String, data: Value },
    /// Global stats update (every 1 second)
    GlobalStats(GlobalStat),
    /// Response to a command
    CommandResult { id: u64, result: CommandResult },
}

#[derive(Debug, Clone)]
pub enum CommandResult {
    Ok(Value),
    /// Result that is specifically completed download history
    HistoryOk(Value),
    /// Result that is specifically active/incomplete downloads
    ActiveDownloadsOk(Value),
    Err(String),
}

/// Commands sent from the GTK thread to the engine thread.
#[derive(Debug)]
pub enum EngineCommand {
    // Download operations
    AddDownload { id: u64, url: String, options: Option<DownloadOptions> },
    AddUrls { id: u64, urls: Vec<String>, options: Option<DownloadOptions> },
    AddTorrentFile { id: u64, file_path: String, options: Option<DownloadOptions> },
    AddMagnet { id: u64, magnet_uri: String, options: Option<DownloadOptions> },
    PauseDownload { gid: String },
    PauseAll,
    ResumeDownload { gid: String },
    ResumeAll,
    RemoveDownload { gid: String, delete_files: bool },
    SetSpeedLimit { download_limit: Option<u64>, upload_limit: Option<u64> },
    SetPriority { gid: String, priority: String },

    // Queries (results come back as CommandResult)
    GetAllDownloads { id: u64 },
    GetGlobalStats { id: u64 },
    GetDownloadStatus { id: u64, gid: String },
    GetTorrentFiles { id: u64, gid: String },
    ParseTorrentFile { id: u64, file_path: String },
    ParseMagnetUri { id: u64, magnet_uri: String },

    // Settings
    GetSettings { id: u64 },
    UpdateSettings { settings: Settings },
    ApplySettings { settings: Settings },

    // Scheduling
    GetScheduleRules { id: u64 },
    SetScheduleRules { rules: Value },

    // Database
    GetCompletedHistory { id: u64 },
    SaveDownload { download: Download },
    DbRemoveDownload { gid: String },
    ClearHistory,
    LoadIncomplete { id: u64 },

    // Tracker
    GetTrackerList { id: u64 },
    UpdateTrackerList { id: u64 },

    // System
    OpenFolder { path: String },
    OpenFileLocation { file_path: String },

    /// Shutdown the engine gracefully
    Shutdown,
}

/// The bridge that owns the channel sender to the engine thread.
#[derive(Clone)]
pub struct EngineBridge {
    cmd_tx: mpsc::UnboundedSender<EngineCommand>,
    cmd_rx_holder: Arc<std::sync::Mutex<Option<mpsc::UnboundedReceiver<EngineCommand>>>>,
    next_id: Arc<std::sync::atomic::AtomicU64>,
}

impl std::fmt::Debug for EngineBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineBridge").finish()
    }
}

impl EngineBridge {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            cmd_tx: tx,
            cmd_rx_holder: Arc::new(std::sync::Mutex::new(Some(rx))),
            next_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    /// Get the next unique command ID.
    pub fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// Send a command to the engine.
    pub fn send(&self, cmd: EngineCommand) {
        if let Err(e) = self.cmd_tx.send(cmd) {
            log::error!("Failed to send command to engine: {}", e);
        }
    }

    /// Start the engine on a background tokio runtime.
    /// Events will be forwarded to the glib main loop via `event_sender`.
    pub fn start(&self, event_sender: GlibSender<EngineEvent>) {
        let rx = match self.cmd_rx_holder.lock() {
            Ok(mut guard) => match guard.take() {
                Some(rx) => rx,
                None => {
                    log::error!("EngineBridge::start called more than once");
                    return;
                }
            },
            Err(e) => {
                log::error!("Failed to lock command receiver: {}", e);
                return;
            }
        };

        let event_sender_clone = event_sender.clone();
        if let Err(e) = std::thread::Builder::new()
            .name("engine-runtime".into())
            .spawn(move || {
                match tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .thread_name("engine-worker")
                    .build()
                {
                    Ok(rt) => rt.block_on(engine_loop(rx, event_sender_clone)),
                    Err(e) => {
                        log::error!("Failed to build tokio runtime: {}", e);
                    }
                }
            })
        {
            log::error!("Failed to spawn engine thread: {}", e);
            let _ = event_sender.send_blocking(EngineEvent::InitError(
                format!("Failed to spawn engine thread: {}", e),
            ));
        }
    }

    // Convenience methods for common operations

    pub fn add_download(&self, url: String, options: Option<DownloadOptions>) -> u64 {
        let id = self.next_id();
        self.send(EngineCommand::AddDownload { id, url, options });
        id
    }

    pub fn add_torrent_file(&self, file_path: String, options: Option<DownloadOptions>) -> u64 {
        let id = self.next_id();
        self.send(EngineCommand::AddTorrentFile { id, file_path, options });
        id
    }

    pub fn add_magnet(&self, magnet_uri: String, options: Option<DownloadOptions>) -> u64 {
        let id = self.next_id();
        self.send(EngineCommand::AddMagnet { id, magnet_uri, options });
        id
    }

    pub fn pause_download(&self, gid: &str) {
        self.send(EngineCommand::PauseDownload { gid: gid.to_string() });
    }

    pub fn resume_download(&self, gid: &str) {
        self.send(EngineCommand::ResumeDownload { gid: gid.to_string() });
    }

    pub fn remove_download(&self, gid: &str, delete_files: bool) {
        self.send(EngineCommand::RemoveDownload { gid: gid.to_string(), delete_files });
    }

    pub fn pause_all(&self) {
        self.send(EngineCommand::PauseAll);
    }

    pub fn resume_all(&self) {
        self.send(EngineCommand::ResumeAll);
    }

    pub fn get_all_downloads(&self) -> u64 {
        let id = self.next_id();
        self.send(EngineCommand::GetAllDownloads { id });
        id
    }

    pub fn get_completed_history(&self) -> u64 {
        let id = self.next_id();
        self.send(EngineCommand::GetCompletedHistory { id });
        id
    }

    pub fn load_incomplete(&self) -> u64 {
        let id = self.next_id();
        self.send(EngineCommand::LoadIncomplete { id });
        id
    }

    pub fn get_settings(&self) -> u64 {
        let id = self.next_id();
        self.send(EngineCommand::GetSettings { id });
        id
    }

    pub fn update_settings(&self, settings: Settings) {
        self.send(EngineCommand::UpdateSettings { settings });
    }

    pub fn apply_settings(&self, settings: Settings) {
        self.send(EngineCommand::ApplySettings { settings });
    }

    pub fn open_folder(&self, path: String) {
        self.send(EngineCommand::OpenFolder { path });
    }

    pub fn open_file_location(&self, file_path: String) {
        self.send(EngineCommand::OpenFileLocation { file_path });
    }

    pub fn set_speed_limit(&self, download_limit: Option<u64>, upload_limit: Option<u64>) {
        self.send(EngineCommand::SetSpeedLimit { download_limit, upload_limit });
    }

    pub fn set_priority(&self, gid: &str, priority: &str) {
        self.send(EngineCommand::SetPriority {
            gid: gid.to_string(),
            priority: priority.to_string(),
        });
    }

    pub fn parse_torrent_file(&self, file_path: String) -> u64 {
        let id = self.next_id();
        self.send(EngineCommand::ParseTorrentFile { id, file_path });
        id
    }

    pub fn parse_magnet_uri(&self, magnet_uri: String) -> u64 {
        let id = self.next_id();
        self.send(EngineCommand::ParseMagnetUri { id, magnet_uri });
        id
    }

    pub fn get_schedule_rules(&self) -> u64 {
        let id = self.next_id();
        self.send(EngineCommand::GetScheduleRules { id });
        id
    }

    pub fn set_schedule_rules(&self, rules: Value) {
        self.send(EngineCommand::SetScheduleRules { rules });
    }

    pub fn get_tracker_list(&self) -> u64 {
        let id = self.next_id();
        self.send(EngineCommand::GetTrackerList { id });
        id
    }

    pub fn update_tracker_list(&self) -> u64 {
        let id = self.next_id();
        self.send(EngineCommand::UpdateTrackerList { id });
        id
    }

    pub fn clear_history(&self) {
        self.send(EngineCommand::ClearHistory);
    }

    pub fn db_remove_download(&self, gid: &str) {
        self.send(EngineCommand::DbRemoveDownload { gid: gid.to_string() });
    }

    pub fn save_download(&self, download: Download) {
        self.send(EngineCommand::SaveDownload { download });
    }

    pub fn shutdown(&self) {
        self.send(EngineCommand::Shutdown);
    }
}

/// The main engine event loop running on the tokio runtime.
async fn engine_loop(
    mut cmd_rx: mpsc::UnboundedReceiver<EngineCommand>,
    event_tx: GlibSender<EngineEvent>,
) {
    // Determine data directory
    let data_dir = match dirs::data_dir().or_else(|| {
        dirs::home_dir().map(|h| {
            if cfg!(target_os = "macos") {
                h.join("Library/Application Support")
            } else if cfg!(target_os = "windows") {
                h.join("AppData/Roaming")
            } else {
                h.join(".local/share")
            }
        })
    }) {
        Some(dir) => dir.join("com.goshapps.downloader"),
        None => {
            let msg = "Could not determine platform data directory";
            log::error!("{}", msg);
            let _ = event_tx.send(EngineEvent::InitError(msg.to_string())).await;
            return;
        }
    };

    // Create broadcast channel for engine events
    let (broadcast_tx, mut broadcast_rx) = broadcast::channel::<Value>(2048);

    // Initialize the engine
    let state = AppState::new();
    if let Err(e) = state.initialize(data_dir, broadcast_tx.clone()).await {
        let _ = event_tx.send(EngineEvent::InitError(e.to_string())).await;
        return;
    }
    let _ = event_tx.send(EngineEvent::Ready).await;

    // Spawn event forwarder: engine broadcast → glib sender
    let ev_tx = event_tx.clone();
    tokio::spawn(async move {
        loop {
            match broadcast_rx.recv().await {
                Ok(event) => {
                    let event_name = event.get("event")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let data = event.get("data").cloned().unwrap_or(Value::Null);
                    let _ = ev_tx.send(EngineEvent::DownloadEvent { event_name, data }).await;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    log::warn!("Engine event receiver lagged by {} messages", n);
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // Spawn global stats emitter (every 1 second)
    let stats_state = state.clone();
    let stats_tx = event_tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            if let Ok(adapter) = stats_state.get_adapter().await {
                let stats = adapter.get_global_stats();
                let _ = stats_tx.send(EngineEvent::GlobalStats(stats)).await;
            }
        }
    });

    // Process commands from the GTK thread
    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            EngineCommand::Shutdown => {
                if let Err(e) = state.shutdown().await {
                    log::error!("Engine shutdown error: {}", e);
                }
                break;
            }

            // Fire-and-forget download operations
            EngineCommand::PauseDownload { gid } => {
                let _ = commands::pause_download(&state, gid).await;
            }
            EngineCommand::PauseAll => {
                let _ = commands::pause_all(&state).await;
            }
            EngineCommand::ResumeDownload { gid } => {
                let _ = commands::resume_download(&state, gid).await;
            }
            EngineCommand::ResumeAll => {
                let _ = commands::resume_all(&state).await;
            }
            EngineCommand::RemoveDownload { gid, delete_files } => {
                let _ = commands::remove_download(&state, gid, delete_files).await;
            }
            EngineCommand::SetSpeedLimit { download_limit, upload_limit } => {
                let _ = commands::set_speed_limit(&state, download_limit, upload_limit).await;
            }
            EngineCommand::SetPriority { gid, priority } => {
                if let Ok(engine) = state.get_engine().await {
                    let id = gosh_fetch_engine::engine_adapter::parse_gid_public(&gid);
                    if let Ok(id) = id {
                        let p: std::result::Result<gosh_dl::DownloadPriority, _> = priority.parse();
                        if let Ok(p) = p {
                            let _ = engine.set_priority(id, p);
                        }
                    }
                }
            }

            // Commands that return results
            EngineCommand::AddDownload { id, url, options } => {
                let result = commands::add_download(&state, url, options).await;
                send_result(&event_tx, id, result.map(|gid| Value::String(gid))).await;
            }
            EngineCommand::AddUrls { id, urls, options } => {
                let result = commands::add_urls(&state, urls, options).await;
                send_result(&event_tx, id, result.map(|gids| serde_json::to_value(gids).unwrap_or_default())).await;
            }
            EngineCommand::AddTorrentFile { id, file_path, options } => {
                let result = commands::add_torrent_file(&state, file_path, options).await;
                send_result(&event_tx, id, result.map(|gid| Value::String(gid))).await;
            }
            EngineCommand::AddMagnet { id, magnet_uri, options } => {
                let result = commands::add_magnet(&state, magnet_uri, options).await;
                send_result(&event_tx, id, result.map(|gid| Value::String(gid))).await;
            }
            EngineCommand::GetAllDownloads { id } => {
                let result = commands::get_all_downloads(&state).await;
                let cmd_result = match result {
                    Ok(dl) => CommandResult::ActiveDownloadsOk(serde_json::to_value(dl).unwrap_or_default()),
                    Err(e) => CommandResult::Err(e.to_string()),
                };
                let _ = event_tx.send(EngineEvent::CommandResult { id, result: cmd_result }).await;
            }
            EngineCommand::GetGlobalStats { id } => {
                let result = commands::get_global_stats(&state).await;
                send_result(&event_tx, id, result.map(|s| serde_json::to_value(s).unwrap_or_default())).await;
            }
            EngineCommand::GetDownloadStatus { id, gid } => {
                let result = commands::get_download_status(&state, gid).await;
                send_result(&event_tx, id, result.map(|dl| serde_json::to_value(dl).unwrap_or_default())).await;
            }
            EngineCommand::GetTorrentFiles { id, gid } => {
                let result = commands::get_torrent_files(&state, gid).await;
                send_result(&event_tx, id, result.map(|f| serde_json::to_value(f).unwrap_or_default())).await;
            }
            EngineCommand::ParseTorrentFile { id, file_path } => {
                let result = commands::parse_torrent_file(file_path);
                send_result(&event_tx, id, result.map(|info| serde_json::to_value(info).unwrap_or_default())).await;
            }
            EngineCommand::ParseMagnetUri { id, magnet_uri } => {
                let result = commands::parse_magnet_uri(magnet_uri);
                send_result(&event_tx, id, result.map(|info| serde_json::to_value(info).unwrap_or_default())).await;
            }

            // Settings
            EngineCommand::GetSettings { id } => {
                let result = commands::get_settings(&state).await;
                send_result(&event_tx, id, result.map(|s| serde_json::to_value(s).unwrap_or_default())).await;
            }
            EngineCommand::UpdateSettings { settings } => {
                let _ = commands::update_settings(&state, settings).await;
            }
            EngineCommand::ApplySettings { settings } => {
                let _ = commands::apply_settings_to_engine(&state, settings).await;
            }

            // Scheduling
            EngineCommand::GetScheduleRules { id } => {
                if let Ok(engine) = state.get_engine().await {
                    let rules = engine.get_schedule_rules();
                    let val = serde_json::to_value(rules).unwrap_or_default();
                    let _ = event_tx.send(EngineEvent::CommandResult {
                        id,
                        result: CommandResult::Ok(val),
                    }).await;
                }
            }
            EngineCommand::SetScheduleRules { rules } => {
                if let Ok(engine) = state.get_engine().await {
                    if let Ok(parsed) = serde_json::from_value(rules) {
                        engine.set_schedule_rules(parsed);
                    }
                }
            }

            // Database
            EngineCommand::GetCompletedHistory { id } => {
                let result = commands::db_get_completed_history(&state).await;
                let cmd_result = match result {
                    Ok(dl) => CommandResult::HistoryOk(serde_json::to_value(dl).unwrap_or_default()),
                    Err(e) => CommandResult::Err(e.to_string()),
                };
                let _ = event_tx.send(EngineEvent::CommandResult { id, result: cmd_result }).await;
            }
            EngineCommand::SaveDownload { download } => {
                let _ = commands::db_save_download(&state, download).await;
            }
            EngineCommand::DbRemoveDownload { gid } => {
                let _ = commands::db_remove_download(&state, gid).await;
            }
            EngineCommand::ClearHistory => {
                let _ = commands::db_clear_history(&state).await;
            }
            EngineCommand::LoadIncomplete { id } => {
                let result = commands::db_load_incomplete(&state).await;
                send_result(&event_tx, id, result.map(|dl| serde_json::to_value(dl).unwrap_or_default())).await;
            }

            // Trackers
            EngineCommand::GetTrackerList { id } => {
                let result = commands::get_tracker_list(&state).await;
                send_result(&event_tx, id, result.map(|t| serde_json::to_value(t).unwrap_or_default())).await;
            }
            EngineCommand::UpdateTrackerList { id } => {
                let result = commands::update_tracker_list(&state).await;
                send_result(&event_tx, id, result.map(|t| serde_json::to_value(t).unwrap_or_default())).await;
            }

            // System
            EngineCommand::OpenFolder { path } => {
                let _ = commands::open_download_folder(path);
            }
            EngineCommand::OpenFileLocation { file_path } => {
                let _ = commands::open_file_location(file_path);
            }
        }
    }
}

async fn send_result(
    tx: &GlibSender<EngineEvent>,
    id: u64,
    result: gosh_fetch_engine::Result<Value>,
) {
    let cmd_result = match result {
        Ok(val) => CommandResult::Ok(val),
        Err(e) => CommandResult::Err(e.to_string()),
    };
    let _ = tx.send(EngineEvent::CommandResult { id, result: cmd_result }).await;
}
