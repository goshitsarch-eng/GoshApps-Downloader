# Gosh-Fetch Engine API Reference

This document covers the public Rust API of the `gosh-fetch-engine` library crate. The GUI application calls these functions through the `EngineBridge` channel mechanism, but they can also be used directly by any Rust code that has an `AppState` instance.

All command functions live in `crates/engine/src/commands/` and are re-exported from `gosh_fetch_engine::commands`.

---

## Download Commands

Defined in `commands/download.rs`.

#### add_download

Add an HTTP/HTTPS download. URLs are validated before reaching the engine: only `http://`, `https://`, and `magnet:` schemes are accepted, private IPs are blocked, and the maximum URL length is 8192 characters.

```rust
pub async fn add_download(
    state: &AppState,
    url: String,
    options: Option<DownloadOptions>,
) -> Result<String>
```

Returns the download GID (a unique identifier string).

#### add_urls

Add multiple downloads at once. All URLs are validated.

```rust
pub async fn add_urls(
    state: &AppState,
    urls: Vec<String>,
    options: Option<DownloadOptions>,
) -> Result<Vec<String>>
```

Returns a vector of GIDs.

#### pause_download

```rust
pub async fn pause_download(state: &AppState, gid: String) -> Result<()>
```

#### pause_all

```rust
pub async fn pause_all(state: &AppState) -> Result<()>
```

#### resume_download

```rust
pub async fn resume_download(state: &AppState, gid: String) -> Result<()>
```

#### resume_all

```rust
pub async fn resume_all(state: &AppState) -> Result<()>
```

#### remove_download

```rust
pub async fn remove_download(
    state: &AppState,
    gid: String,
    delete_files: bool,
) -> Result<()>
```

If `delete_files` is true, the downloaded file is deleted from disk.

#### get_download_status

```rust
pub async fn get_download_status(state: &AppState, gid: String) -> Result<Download>
```

Returns `Error::NotFound` if the GID does not exist.

#### get_all_downloads

```rust
pub async fn get_all_downloads(state: &AppState) -> Result<Vec<Download>>
```

Returns all downloads including active, waiting, paused, and error states.

#### get_active_downloads

```rust
pub async fn get_active_downloads(state: &AppState) -> Result<Vec<Download>>
```

#### get_global_stats

```rust
pub async fn get_global_stats(state: &AppState) -> Result<GlobalStat>
```

#### set_speed_limit

```rust
pub async fn set_speed_limit(
    state: &AppState,
    download_limit: Option<u64>,
    upload_limit: Option<u64>,
) -> Result<()>
```

Values are in bytes per second. Pass `None` for unlimited.

---

## Torrent Commands

Defined in `commands/torrent.rs`.

#### add_torrent_file

Add a download from a `.torrent` file. The file is read from disk and passed to the engine.

```rust
pub async fn add_torrent_file(
    state: &AppState,
    file_path: String,
    options: Option<DownloadOptions>,
) -> Result<String>
```

#### add_magnet

```rust
pub async fn add_magnet(
    state: &AppState,
    magnet_uri: String,
    options: Option<DownloadOptions>,
) -> Result<String>
```

#### get_torrent_files

Get the file list for a torrent download.

```rust
pub async fn get_torrent_files(
    state: &AppState,
    gid: String,
) -> Result<Vec<DownloadFile>>
```

#### select_torrent_files

Select which files to download from a multi-file torrent. Currently returns an error because gosh-dl does not support post-add file selection. File selection must be specified when adding the torrent via `DownloadOptions::select_file`.

```rust
pub async fn select_torrent_files(
    state: &AppState,
    gid: String,
    file_indices: Vec<u32>,
) -> Result<()>
```

#### parse_torrent_file

Parse a `.torrent` file without adding it as a download. Useful for previewing contents in the UI before the user confirms.

```rust
pub fn parse_torrent_file(file_path: String) -> Result<TorrentInfo>
```

Note: this is a synchronous function (no `async`).

#### parse_magnet_uri

Parse a magnet URI without adding it.

```rust
pub fn parse_magnet_uri(magnet_uri: String) -> Result<MagnetInfo>
```

Note: this is a synchronous function (no `async`).

#### get_peers

Get connected peer information for a torrent download.

```rust
pub async fn get_peers(state: &AppState, gid: String) -> Result<Vec<serde_json::Value>>
```

Each peer is returned as a JSON value with fields: `ip`, `port`, `client`, `downloadSpeed`, `uploadSpeed`.

---

## Settings Commands

Defined in `commands/settings.rs`.

#### get_settings

Get the current settings from the database.

```rust
pub async fn get_settings(state: &AppState) -> Result<Settings>
```

#### update_settings

Persist settings to the database.

```rust
pub async fn update_settings(state: &AppState, settings: Settings) -> Result<()>
```

#### apply_settings_to_engine

Apply settings to the running download engine. Call this after saving settings to make them take effect immediately without restarting.

```rust
pub async fn apply_settings_to_engine(
    state: &AppState,
    settings: Settings,
) -> Result<()>
```

#### set_close_to_tray

```rust
pub fn set_close_to_tray(state: &AppState, value: bool)
```

#### set_user_agent

Update the user agent on the running engine.

```rust
pub async fn set_user_agent(state: &AppState, user_agent: String) -> Result<()>
```

#### get_user_agent_presets

Returns a list of `(name, user_agent_string)` tuples. Available presets: gosh-dl (default), Chrome (Windows), Chrome (macOS), Firefox (Windows), Firefox (Linux), Wget, Curl.

```rust
pub fn get_user_agent_presets() -> Vec<(String, String)>
```

#### get_tracker_list

Fetch the cached tracker list. If the cache is stale, fetches from the remote source first.

```rust
pub async fn get_tracker_list(state: &AppState) -> Result<Vec<String>>
```

#### update_tracker_list

Force-fetch and update the tracker list from the remote source.

```rust
pub async fn update_tracker_list(state: &AppState) -> Result<Vec<String>>
```

---

## Database Commands

Defined in `commands/database.rs`. These methods read from and write to the SQLite database directly, bypassing the download engine.

#### db_get_completed_history

```rust
pub async fn db_get_completed_history(state: &AppState) -> Result<Vec<Download>>
```

#### db_save_download

```rust
pub async fn db_save_download(state: &AppState, download: Download) -> Result<()>
```

#### db_remove_download

```rust
pub async fn db_remove_download(state: &AppState, gid: String) -> Result<()>
```

#### db_clear_history

```rust
pub async fn db_clear_history(state: &AppState) -> Result<()>
```

#### db_get_settings

```rust
pub async fn db_get_settings(state: &AppState) -> Result<Settings>
```

#### db_save_settings

```rust
pub async fn db_save_settings(state: &AppState, settings: Settings) -> Result<()>
```

#### db_load_incomplete

Load incomplete downloads from the database for restoration on app startup.

```rust
pub async fn db_load_incomplete(state: &AppState) -> Result<Vec<Download>>
```

---

## System Commands

Defined in `commands/system.rs`.

#### get_engine_version

```rust
pub async fn get_engine_version(state: &AppState) -> Result<serde_json::Value>
```

Returns:
```json
{
  "name": "gosh-dl",
  "version": "0.3.2",
  "running": true
}
```

#### open_download_folder

Open a directory in the system file manager. The path is validated and canonicalized before being passed to `xdg-open` (Linux), `open` (macOS), or `explorer` (Windows).

```rust
pub fn open_download_folder(path: String) -> Result<()>
```

#### open_file_location

Open the containing folder of a file.

```rust
pub fn open_file_location(file_path: String) -> Result<()>
```

#### get_default_download_path

```rust
pub fn get_default_download_path() -> String
```

Returns the platform default download directory (typically `~/Downloads`).

#### get_app_version

```rust
pub fn get_app_version() -> String
```

Returns the version from `Cargo.toml`.

#### get_app_info

```rust
pub fn get_app_info() -> serde_json::Value
```

Returns:
```json
{
  "name": "Goshapps Downloader",
  "version": "3.0.0",
  "description": "Goshapps Downloader - the modern download manager powered by gosh-dl",
  "license": "AGPL-3.0",
  "repository": "https://github.com/goshitsarch-eng/Gosh-Fetch",
  "engine": {
    "name": "gosh-dl",
    "version": "0.3.2",
    "url": "https://github.com/goshitsarch-eng/gosh-dl",
    "license": "MIT",
    "description": "A fast, safe, and reliable download engine written in Rust"
  }
}
```

---

## Validation Functions

Defined in `validation.rs` and re-exported from the crate root.

#### validate_download_url

```rust
pub fn validate_download_url(url: &str) -> Result<()>
```

Validates that a URL uses an allowed scheme (`http://`, `https://`, or `magnet:`), is not empty, does not exceed 8192 characters, and does not target a private/loopback IP address.

#### validate_torrent_path

```rust
pub fn validate_torrent_path(file_path: &str) -> Result<()>
```

Validates that a file path is not empty, ends with `.torrent`, and exists on disk.

---

## Types

All types are defined in `crates/engine/src/types.rs` and re-exported from the crate root.

### DownloadOptions

Configuration options when adding a download. All fields are optional. Uses `camelCase` serialization for JSON compatibility.

```rust
pub struct DownloadOptions {
    pub dir: Option<String>,                    // Save directory
    pub out: Option<String>,                    // Output filename
    pub split: Option<String>,                  // Number of segments
    pub max_connection_per_server: Option<String>, // Connections per server
    pub user_agent: Option<String>,             // HTTP user agent
    pub referer: Option<String>,                // HTTP referer header
    pub header: Option<Vec<String>>,            // Custom headers ["Key: Value"]
    pub select_file: Option<String>,            // Torrent file indices "1,2,3"
    pub seed_ratio: Option<String>,             // Seed ratio for torrents
    pub max_download_limit: Option<String>,     // Download speed limit (bytes/sec)
    pub max_upload_limit: Option<String>,       // Upload speed limit (bytes/sec)
    pub priority: Option<String>,               // "low" | "normal" | "high" | "critical"
    pub checksum: Option<String>,               // "sha256:hex..." or "md5:hex..."
    pub mirrors: Option<Vec<String>>,           // Mirror/failover URLs
    pub sequential: Option<bool>,               // Sequential download mode
}
```

### Download

```rust
pub struct Download {
    pub id: i64,                                // Database ID
    pub gid: String,                            // Engine GID (unique identifier)
    pub name: String,                           // Display name
    pub url: Option<String>,                    // Source URL (HTTP downloads)
    pub magnet_uri: Option<String>,             // Magnet link (torrents)
    pub info_hash: Option<String>,              // BitTorrent info hash
    pub download_type: DownloadType,            // Http | Torrent | Magnet
    pub status: DownloadState,                  // Active | Waiting | Paused | Complete | Error | Removed
    pub total_size: u64,                        // Total bytes
    pub completed_size: u64,                    // Downloaded bytes
    pub download_speed: u64,                    // Bytes per second
    pub upload_speed: u64,                      // Bytes per second
    pub save_path: String,                      // Save directory
    pub created_at: String,                     // ISO 8601 timestamp
    pub completed_at: Option<String>,           // ISO 8601 timestamp
    pub error_message: Option<String>,          // Error description
    pub connections: u32,                       // Active connections
    pub seeders: u32,                           // Connected seeders (torrents)
    pub selected_files: Option<Vec<usize>>,     // Selected file indices (torrents)
}
```

### DownloadType

```rust
pub enum DownloadType {
    Http,
    Torrent,
    Magnet,
}
```

### DownloadState

```rust
pub enum DownloadState {
    Active,
    Waiting,
    Paused,
    Complete,
    Error,
    Removed,
}
```

Implements `From<&str>` for parsing and `Display` for serialization. Unknown strings default to `Waiting`.

### GlobalStat

```rust
pub struct GlobalStat {
    pub download_speed: u64,                    // Total download speed (bytes/sec)
    pub upload_speed: u64,                      // Total upload speed (bytes/sec)
    pub num_active: u32,                        // Active download count
    pub num_waiting: u32,                       // Queued download count
    pub num_stopped: u32,                       // Stopped download count
    pub num_stopped_total: u32,                 // Total stopped count (all time)
}
```

### TorrentInfo

Returned by `parse_torrent_file`.

```rust
pub struct TorrentInfo {
    pub name: String,
    pub info_hash: String,
    pub total_size: u64,
    pub files: Vec<TorrentFile>,
    pub comment: Option<String>,
    pub creation_date: Option<i64>,             // Unix timestamp
    pub announce_list: Vec<String>,             // Tracker URLs
}
```

### TorrentFile

```rust
pub struct TorrentFile {
    pub index: usize,
    pub path: String,
    pub length: u64,                            // File size in bytes
}
```

### MagnetInfo

Returned by `parse_magnet_uri`.

```rust
pub struct MagnetInfo {
    pub name: Option<String>,
    pub info_hash: String,
    pub trackers: Vec<String>,
}
```

### DownloadFile

Returned by `get_torrent_files`. Uses string representations for numeric fields.

```rust
pub struct DownloadFile {
    pub index: String,
    pub path: String,
    pub length: String,
    pub completed_length: String,
    pub selected: String,                       // "true" or "false"
    pub uris: Vec<FileUri>,
}
```

### FileUri

```rust
pub struct FileUri {
    pub uri: String,
    pub status: String,
}
```

### Settings

Defined in `crates/engine/src/db/mod.rs`.

```rust
pub struct Settings {
    pub download_path: String,                  // Default save directory
    pub max_concurrent_downloads: u32,          // 1-20, default 5
    pub max_connections_per_server: u32,        // 1-16, default 8
    pub split_count: u32,                       // Segments per download, default 8
    pub download_speed_limit: u64,              // Global download limit, 0 = unlimited
    pub upload_speed_limit: u64,                // Global upload limit, 0 = unlimited
    pub user_agent: String,                     // HTTP user agent
    pub enable_notifications: bool,             // Show completion notifications
    pub close_to_tray: bool,                    // Minimize to tray on close
    pub theme: String,                          // "dark" | "light" | "system"
    pub bt_enable_dht: bool,                    // BitTorrent DHT
    pub bt_enable_pex: bool,                    // BitTorrent Peer Exchange
    pub bt_enable_lpd: bool,                    // Local Peer Discovery
    pub bt_max_peers: u32,                      // Max peers per torrent, default 55
    pub bt_seed_ratio: f64,                     // Seed ratio, default 1.0
    pub auto_update_trackers: bool,             // Auto-fetch tracker lists
    pub delete_files_on_remove: bool,           // Delete files when removing download
    pub proxy_url: String,                      // HTTP/SOCKS proxy URL (empty = none)
    pub connect_timeout: u64,                   // Connection timeout in seconds, default 30
    pub read_timeout: u64,                      // Read timeout in seconds, default 60
    pub max_retries: u32,                       // Max retry attempts, default 3
    pub allocation_mode: String,                // "none" | "sparse" | "full", default "sparse"
}
```

---

## Error Type

Defined in `crates/engine/src/error.rs`.

```rust
pub enum Error {
    Engine(String),           // code -1: gosh-dl engine error
    EngineNotInitialized,     // code -2: engine not yet initialized
    Database(String),         // code -3: database error
    Io(std::io::Error),       // code -4: IO error
    Serialization(serde_json::Error), // code -5: JSON serialization error
    Rusqlite(rusqlite::Error), // code -6: SQLite error
    InvalidInput(String),     // code -7: input validation failure
    NotFound(String),         // code -8: resource not found
    Network(String),          // code -9: network error
}

pub type Result<T> = std::result::Result<T, Error>;
```

The `Error` type implements `Display`, `std::error::Error` (via `thiserror`), `Serialize`, and `From<gosh_dl::EngineError>`.

---

## Engine Events

The engine emits events through gosh-dl's broadcast channel. These are forwarded to the GTK UI by the `EngineBridge`. Event names:

| Event | Description |
|-------|-------------|
| `download:added` | A new download was added |
| `download:started` | Download started actively transferring |
| `download:progress` | Progress update (size, speed) |
| `download:state-changed` | Generic state transition |
| `download:completed` | Download finished successfully |
| `download:failed` | Download encountered an error |
| `download:removed` | Download was removed |
| `download:paused` | Download was paused |
| `download:resumed` | Download was resumed |

Global stats are emitted separately every second as `EngineEvent::GlobalStats(GlobalStat)`.
