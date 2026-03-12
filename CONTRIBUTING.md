# Contributing to Gosh-Fetch

Thanks for wanting to contribute. This guide covers setting up the project for development and the conventions we follow.

## Development Setup

### Prerequisites

- [Rust](https://rustup.rs/) 1.77+
- GTK4 development libraries (4.12+)
- libadwaita development libraries (1.5+)
- A C compiler (gcc or clang)
- Platform-specific dependencies (see below)

### Linux Dependencies

**Debian/Ubuntu:**

```bash
sudo apt install libgtk-4-dev libadwaita-1-dev build-essential
```

**Fedora:**

```bash
sudo dnf install gtk4-devel libadwaita-devel gcc
```

**Arch Linux:**

```bash
sudo pacman -S gtk4 libadwaita base-devel
```

SQLite is bundled automatically via the `rusqlite` `bundled` feature, so you do not need to install SQLite development headers separately.

### Getting Started

Fork and clone the repository, then build:

```bash
git clone https://github.com/YOUR_USERNAME/Gosh-Fetch.git
cd Gosh-Fetch
cargo build
```

Run the application:

```bash
cargo run
```

### Available Commands

| Command | Description |
|---------|-------------|
| `cargo build` | Build all workspace crates (debug) |
| `cargo build --release` | Build optimized release binary |
| `cargo run` | Build and run the application |
| `cargo test --workspace` | Run all tests across both crates |
| `cargo clippy --workspace` | Run the Clippy linter on all crates |
| `cargo fmt --all` | Format all Rust code |
| `cargo doc --workspace --open` | Generate and open API documentation |

### Building Individual Crates

```bash
# Engine library only
cargo build -p gosh-fetch-engine
cargo test -p gosh-fetch-engine

# GUI application only
cargo build -p gosh-fetch
```

## Project Structure

```
Gosh-Fetch/
├── Cargo.toml                        # Workspace root
├── crates/
│   ├── engine/                       # gosh-fetch-engine library crate
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs                # Public API exports
│   │   │   ├── state.rs              # AppState (engine, database, adapter)
│   │   │   ├── types.rs              # Shared types (Download, DownloadOptions, GlobalStat, etc.)
│   │   │   ├── engine_adapter.rs     # gosh-dl integration and type conversion
│   │   │   ├── error.rs              # Error types (thiserror)
│   │   │   ├── constants.rs          # Engine name, version, default user agent
│   │   │   ├── validation.rs         # URL and path validation
│   │   │   ├── utils.rs              # TrackerUpdater
│   │   │   ├── db/
│   │   │   │   └── mod.rs            # SQLite database (rusqlite), Settings struct
│   │   │   └── commands/             # Public command functions
│   │   │       ├── mod.rs
│   │   │       ├── download.rs       # Add, pause, resume, remove, query downloads
│   │   │       ├── torrent.rs        # Torrent/magnet operations, parsing
│   │   │       ├── settings.rs       # Settings management, engine config, trackers
│   │   │       ├── database.rs       # Database read/write (history, settings)
│   │   │       └── system.rs         # App info, file/folder opening, paths
│   │   └── migrations/
│   │       └── 001_initial.sql       # Database schema
│   │
│   └── gui/                          # gosh-fetch GTK4 application
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs               # Entry point, creates AdwApplication
│           ├── app.rs                # GoshFetchApplication (GObject subclass)
│           ├── engine_bridge.rs      # mpsc bridge between GTK and tokio runtime
│           ├── model.rs              # AppModel (shared state, replaces Redux)
│           ├── shortcuts.rs          # Keyboard shortcut definitions
│           ├── css/
│           │   └── style.css         # Application stylesheet
│           ├── pages/
│           │   ├── downloads.rs      # Active downloads with filtering
│           │   ├── history.rs        # Completed download history
│           │   ├── settings.rs       # All configuration options
│           │   ├── statistics.rs     # Download statistics
│           │   └── scheduler.rs      # Bandwidth scheduling rules
│           ├── widgets/
│           │   ├── window.rs         # GoshFetchWindow (main application window)
│           │   ├── sidebar.rs        # Navigation sidebar with disk space
│           │   ├── status_bar.rs     # Bottom status bar
│           │   ├── download_card.rs  # Expanded download card widget
│           │   ├── compact_download_row.rs  # Compact download row widget
│           │   └── notification_dropdown.rs # Notification popup
│           └── dialogs/
│               ├── add_download.rs   # Add download dialog
│               ├── onboarding.rs     # First-run onboarding
│               └── torrent_picker.rs # Torrent file selection dialog
│
├── data/
│   └── com.goshapps.downloader.desktop  # Desktop entry file
├── docs/                             # Documentation
└── LICENSE
```

## Code Style

### Rust

The entire codebase is Rust. Run `cargo fmt --all` and `cargo clippy --workspace` before committing. The workspace uses Rust 2021 edition.

### GTK4 Patterns

The GUI follows standard gtk4-rs conventions:

- **GObject subclassing** -- Custom widgets use the `glib::wrapper!` / `ObjectSubclass` pattern with an `imp` module for private data. See `app.rs` and `widgets/window.rs` for examples.
- **Signals and properties** -- Widgets communicate through GObject signals. The `AppModel` holds shared application state and notifies widgets of changes.
- **CSS styling** -- All visual styling uses GTK CSS loaded from `src/css/style.css` via `CssProvider`. There is no inline styling.
- **Async operations** -- The GTK main thread must never block. All engine operations go through the `EngineBridge`, which sends commands to a background Tokio runtime via `mpsc` channels. Results return to the main thread via `async-channel` and are processed in `glib::spawn_future_local`.
- **libadwaita** -- The application uses `adw::Application` as its base class for adaptive layout and GNOME integration. Widgets use Adwaita style classes where appropriate.

### Engine Library

The engine crate (`gosh-fetch-engine`) is a pure Rust library with no GTK dependencies. It exposes its API through the `commands` module, which contains async functions that take `&AppState` and return `Result<T>`. The `AppState` struct manages the download engine, database, and adapter lifecycle.

Key conventions:
- Database operations use `spawn_blocking` to avoid blocking the Tokio runtime.
- The `EngineAdapter` converts between gosh-dl internal types and the frontend-facing types in `types.rs`.
- Error handling uses `thiserror` with the `Error` enum in `error.rs`.
- Input validation (URLs, file paths) is handled by `validation.rs` before reaching the engine.

### Adding a New Engine Command

1. Add the command function in the appropriate `commands/` module (e.g., `download.rs`, `settings.rs`).
2. Re-export it from `commands/mod.rs`.
3. Add a variant to `EngineCommand` in `crates/gui/src/engine_bridge.rs`.
4. Handle the new variant in the `engine_loop` match block in `engine_bridge.rs`.
5. Optionally add a convenience method on `EngineBridge` for ergonomic use from widgets.

## Pull Request Process

1. Create a branch from `main`:
```bash
git checkout -b feature/your-feature-name
```

2. Make your changes and test them.

3. Ensure everything passes:
```bash
cargo test --workspace
cargo clippy --workspace
cargo fmt --all -- --check
```

4. Commit with a descriptive message:
```bash
git commit -m "Add: brief description of changes"
```

5. Push and open a pull request.

### Commit Message Prefixes

- `Add:` New features
- `Fix:` Bug fixes
- `Update:` Enhancements to existing features
- `Refactor:` Code restructuring
- `Docs:` Documentation changes
- `Chore:` Maintenance tasks

## Reporting Issues

When reporting an issue, include your operating system and version, GTK4/libadwaita versions (`pkg-config --modversion gtk4 libadwaita-1`), steps to reproduce the problem, what you expected versus what actually happened, and any error messages or logs. You can enable debug logging by running the application with `RUST_LOG=debug cargo run`.

## License

By contributing to Gosh-Fetch, you agree that your contributions will be licensed under the AGPL-3.0 license.
