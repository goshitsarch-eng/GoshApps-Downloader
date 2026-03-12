# Gosh-Fetch Architecture

This document describes how Gosh-Fetch is built, how its parts communicate, and where things live in the codebase.

## Overview

Gosh-Fetch is a native GTK4 download manager written entirely in Rust. It compiles to a single binary with two logical layers: a GTK4/libadwaita UI running on the glib main loop, and a download engine running on a background Tokio runtime. The two layers communicate through in-process channels -- there is no IPC, no child process, and no network protocol between them.

| Layer | Technology | Purpose |
|-------|------------|---------|
| UI | GTK4 0.9, libadwaita 0.7, gtk4-rs | User interface, GObject subclasses |
| Bridge | tokio mpsc, async-channel | Command/event channels between threads |
| Engine | gosh-fetch-engine (Rust library) | Download management, settings, database |
| Downloads | gosh-dl 0.3.2 | HTTP/BitTorrent protocol implementation |
| Database | SQLite (rusqlite, bundled) | Settings, download history, tracker metadata |

## Application Lifecycle

The application starts in `main.rs`, which creates a `GoshFetchApplication` (an `adw::Application` subclass) and calls `run()`. GTK takes over the main loop from this point.

On the first `activate` signal (in `app.rs`):

1. Application CSS is loaded from `css/style.css` via `CssProvider`.
2. An `EngineBridge` is created, which sets up the `mpsc` command channel.
3. An `AppModel` is created, which holds all shared application state.
4. `bridge.start(sender)` spawns the engine thread (see below).
5. A `glib::spawn_future_local` task listens on an `async-channel` receiver for engine events and forwards them to `AppModel::handle_engine_event`.
6. The main `GoshFetchWindow` is created and presented.

On subsequent `activate` signals (e.g., the user tries to open a second instance), the existing window is presented.

## How the Layers Communicate

```
GTK Main Thread (glib main loop)
    |                                  ^
    |  EngineCommand (mpsc::send)     |  EngineEvent (async_channel::recv)
    |                                  |
    v                                  |
Engine Thread (tokio runtime)
    |
    |  Direct Rust function calls
    v
gosh-fetch-engine commands module
    |
    |  AppState -> EngineAdapter -> gosh-dl API
    v
gosh-dl (DownloadEngine)
```

### Commands: GTK to Engine

The GTK thread sends commands to the engine via `EngineBridge::send()`, which writes an `EngineCommand` enum variant to a `tokio::sync::mpsc::UnboundedSender`. The enum includes variants for all operations: `AddDownload`, `PauseDownload`, `GetAllDownloads`, `UpdateSettings`, etc.

Commands that produce results include a `u64` ID. When the engine completes the command, it sends back an `EngineEvent::CommandResult { id, result }` so the UI can match responses to requests.

Fire-and-forget commands (pause, resume, remove, update settings) do not carry an ID and do not produce a response event.

### Events: Engine to GTK

Events flow from the engine thread to the GTK main thread via an `async_channel::Sender<EngineEvent>`. The `EngineEvent` enum covers:

- `Ready` -- engine initialized successfully
- `InitError(String)` -- engine failed to initialize
- `DownloadEvent { event_name, data }` -- a download lifecycle event (added, started, progress, completed, failed, paused, resumed, removed, state-changed)
- `GlobalStats(GlobalStat)` -- aggregate speed/count stats, emitted every second
- `CommandResult { id, result }` -- response to a query command

The GTK side receives these in a `glib::spawn_future_local` task and dispatches them to the `AppModel`, which updates its internal state and notifies bound widgets.

### Why Two Channel Types

- **tokio mpsc** (commands): The receiver must live on the Tokio runtime, so Tokio's own channel is used.
- **async-channel** (events): The receiver must be polled on the glib main loop. `async-channel` is runtime-agnostic, so it works with `glib::spawn_future_local` without pulling in Tokio on the GTK thread.

## Engine Thread

`EngineBridge::start()` spawns a dedicated OS thread named `engine-runtime` via `std::thread::Builder`. This thread creates a Tokio multi-thread runtime and blocks on `engine_loop()`.

The engine loop:

1. Determines the platform data directory (`~/.local/share/com.goshapps.downloader` on Linux).
2. Creates a `broadcast::channel` for engine events.
3. Initializes `AppState`, which opens the SQLite database, loads saved settings, configures gosh-dl's `EngineConfig`, starts the `DownloadEngine`, and wraps it in an `EngineAdapter`.
4. Spawns an **event forwarder** task: subscribes to gosh-dl's `broadcast` events and relays them as `EngineEvent::DownloadEvent` to the GTK thread.
5. Spawns a **global stats emitter** task: queries `get_global_stats()` every second and sends the result as `EngineEvent::GlobalStats`.
6. Enters the main command-processing loop, matching on `EngineCommand` variants and calling the corresponding `commands::*` functions.

On `EngineCommand::Shutdown`, the loop persists completed downloads to the database, aborts the event listener, calls `engine.shutdown()`, and exits.

## AppState

`AppState` (in `crates/engine/src/state.rs`) is the central coordinator for the engine layer. It owns:

- `Arc<DownloadEngine>` -- the gosh-dl engine instance
- `EngineAdapter` -- type conversion layer between gosh-dl and frontend types
- `Database` -- SQLite connection wrapped in `Arc<Mutex<Connection>>`
- `TrackerUpdater` -- fetches and caches community tracker lists
- Configuration flags (e.g., `close_to_tray`)

All fields are behind `Arc<RwLock<Option<T>>>` to support late initialization and safe concurrent access from multiple Tokio tasks.

Key methods:
- `initialize(data_dir, event_tx)` -- sets up the database, loads settings, creates the engine
- `shutdown()` -- persists completed downloads, stops the engine
- `get_adapter()` / `get_engine()` / `get_db()` -- accessors that return `Result<T>` (error if not yet initialized)
- `reinitialize(event_tx)` -- shuts down and re-initializes (used when settings require an engine restart)

## AppModel

`AppModel` (in `crates/gui/src/model.rs`) is the GUI's shared state container, replacing what would be a Redux store in a web application. It holds:

- The active download list (from the engine)
- Completed download history (from the database)
- Global speed stats
- In-app notifications
- Speed samples for the statistics chart
- Download ordering for drag-and-drop

The model is wrapped in `Arc` for sharing across widgets. Widgets query the model for data and the model notifies widgets when state changes through GObject signals and manual invalidation.

## Commands Module

The `commands` module in `crates/engine/src/commands/` provides the public API of the engine library. Each function takes `&AppState` as its first argument and returns `Result<T>`. The module is organized by domain:

- `download.rs` -- `add_download`, `add_urls`, `pause_download`, `pause_all`, `resume_download`, `resume_all`, `remove_download`, `get_download_status`, `get_all_downloads`, `get_active_downloads`, `get_global_stats`, `set_speed_limit`
- `torrent.rs` -- `add_torrent_file`, `add_magnet`, `get_torrent_files`, `select_torrent_files`, `parse_torrent_file`, `parse_magnet_uri`, `get_peers`
- `settings.rs` -- `get_settings`, `update_settings`, `set_close_to_tray`, `set_user_agent`, `get_tracker_list`, `update_tracker_list`, `apply_settings_to_engine`, `get_user_agent_presets`
- `database.rs` -- `db_get_completed_history`, `db_save_download`, `db_remove_download`, `db_clear_history`, `db_get_settings`, `db_save_settings`, `db_load_incomplete`
- `system.rs` -- `get_engine_version`, `open_download_folder`, `open_file_location`, `get_default_download_path`, `get_app_version`, `get_app_info`

## Database

The SQLite database (`gosh-fetch.db` in the user data directory) stores five tables:

**downloads** -- Download metadata and history. Stores the GID, name, URL/magnet URI, type (http/torrent/magnet), status, sizes, speeds, paths, timestamps, and selected files. Indexed on status, created_at, and gid.

**settings** -- Key-value configuration. All settings have defaults seeded by `001_initial.sql`. Notable defaults: download path `~/Downloads`, max concurrent downloads 5, connections per server 8, split count 8, dark theme, notifications enabled, close to tray enabled, sparse file allocation, 30s connect timeout, 60s read timeout, 3 retries.

**trackers** -- BitTorrent tracker URLs with enabled/working status.

**tracker_meta** -- Single-row table tracking when the tracker list was last updated and the source URL.

**schema_version** -- Migration version tracking for future schema upgrades.

The `Database` struct wraps a `Connection` in `Arc<Mutex<...>>`. Async database methods use `tokio::task::spawn_blocking` to run SQLite I/O on Tokio's blocking thread pool. Settings saves are wrapped in transactions for atomicity.

Note that gosh-dl maintains its own separate database (`engine.db`) for internal engine state like download segments and recovery data. The two databases serve different purposes and this separation is intentional.

## Input Validation

URL and path validation is handled by `validation.rs` before inputs reach the engine:

- **URL validation**: Only `http://`, `https://`, and `magnet:` schemes are accepted. Private/loopback IPs (127.x, 10.x, 172.16-31.x, 192.168.x, link-local, ::1, fc00::/7) are blocked. Maximum URL length is 8192 characters.
- **Torrent path validation**: Files must have a `.torrent` extension and exist on disk.
- **Path sanitization**: `open_download_folder` and `open_file_location` canonicalize paths, verify existence, and reject URL schemes before passing to the OS file manager via `xdg-open` (Linux), `open` (macOS), or `explorer` (Windows).

## Error Handling

Errors are defined in `error.rs` using `thiserror`. The `Error` enum covers engine errors, initialization failures, database errors, IO, serialization, input validation, not-found, and network errors. Each variant has a numeric error code. The `Error` type implements `Serialize` for transport in `CommandResult`.

Errors from gosh-dl's `EngineError` are automatically converted into the application's `Error` type via a `From` implementation.

## GTK4 UI Architecture

### Window and Layout

`GoshFetchWindow` is the main application window. It contains a sidebar for navigation, a content area that swaps between pages (Downloads, History, Statistics, Scheduler, Settings), a status bar, and a notification dropdown.

### Pages

Each page is a GTK4 widget in `crates/gui/src/pages/`:
- `downloads.rs` -- Active download list with active/paused filter tabs
- `history.rs` -- Completed downloads with file/folder open actions
- `settings.rs` -- All configuration options organized in sections
- `statistics.rs` -- Download/upload speed charts
- `scheduler.rs` -- Bandwidth scheduling rule editor

### Widgets

Custom widgets in `crates/gui/src/widgets/` follow the GObject subclass pattern:
- `download_card.rs` -- Expanded card showing progress, speed, ETA, and controls
- `compact_download_row.rs` -- Dense row for list view
- `sidebar.rs` -- Navigation with page links and disk space widget
- `status_bar.rs` -- Bottom bar with global speed and active download count
- `notification_dropdown.rs` -- Popup showing recent download events

### Dialogs

- `add_download.rs` -- Dialog for adding new downloads with advanced options
- `onboarding.rs` -- First-run setup wizard
- `torrent_picker.rs` -- File selection for multi-file torrents

### CSS

All visual styling uses GTK CSS loaded via `CssProvider` from `src/css/style.css`. Theme switching uses libadwaita's built-in color scheme support (`adw::StyleManager`).
