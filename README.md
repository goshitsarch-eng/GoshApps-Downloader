# Gosh-Fetch

A native GTK4 download manager for Linux, built with Rust and powered by gosh-dl.

## Screenshots

![Screenshot 1](screenshots/img11.png)
![Screenshot 2](screenshots/img22.png)
![Screenshot 3](screenshots/img3.png)
![Screenshot 4](screenshots/img4.png)

> Screenshots may not reflect the current GTK4 UI. Updated screenshots are coming.

## Features

Gosh-Fetch handles HTTP/HTTPS and BitTorrent downloads through gosh-dl, a native Rust engine built specifically for this project. It supports magnet links, multi-segment parallel downloads, and uses GTK4 with libadwaita for a modern GNOME-native interface with dark and light themes.

There are no accounts, no telemetry, and no cloud features. Everything stays on your machine.

### Download Management

Downloads show real-time progress, speed, ETA, and connection metrics. You can pause, resume, retry, and cancel individual downloads, or use batch operations to act on multiple downloads at once with checkbox selection and select-all.

The queue supports drag-and-drop reordering, which automatically syncs with the priority system (critical, high, normal, low). Advanced per-download options include custom filename, save directory, speed limit, HTTP headers, connection count, checksum verification (SHA-256 and MD5), mirror/failover URLs, and sequential download mode for streaming media.

Completed downloads are available in the History page, where you can open files or their containing folders directly.

### BitTorrent

Full BitTorrent protocol support including torrent files and magnet links, DHT, PEX, and Local Peer Discovery. You get seeder/peer counts, configurable seed ratio, selective file download from multi-file torrents, and auto-updating tracker lists sourced from the community.

### Network and Reliability

- Concurrent downloads: 1-20 (default 5)
- Connections per server: 1-16 (default 8)
- Segments per download: 1-64 (default 8)
- Global and per-download speed limits
- HTTP/SOCKS proxy support
- Connection timeout (default 30s) and read timeout (default 60s)
- Automatic retry with configurable attempts (default 3)
- Custom user agent with browser presets (Chrome, Firefox, Wget, Curl)
- File allocation modes: none, sparse, full

### Desktop Integration

- System tray with live download/upload speed display
- Minimize to tray on close
- `.torrent` file association and `magnet:` protocol handler
- Drag and drop URLs, magnet links, or `.torrent` files onto the window
- Desktop notifications on download completion
- Keyboard shortcuts: `Ctrl+N` (add download), `Ctrl+K` (focus search), `Ctrl+,` (settings), `Ctrl+A` (select all)
- First-run onboarding with download path setup and system integration options
- Bandwidth scheduling with time-based rules

### Pages

The sidebar navigation provides access to: Downloads (with active/paused filters), History, Statistics, Scheduler, and Settings. A disk space widget in the sidebar shows remaining storage. A notification dropdown tracks download events (added, completed, failed).

## Download Engine

Gosh-Fetch uses [gosh-dl](https://github.com/goshitsarch-eng/gosh-dl), a native Rust download engine built specifically for this project.

| Feature | gosh-dl | External Tools |
|---------|---------|----------------|
| No external binaries | Yes | No |
| Memory safe | Yes (Rust) | Varies |
| Single binary distribution | Yes | No |
| Integrated error handling | Yes | Limited |

gosh-dl provides HTTP/HTTPS segmented downloads with automatic resume, full BitTorrent protocol support with DHT/PEX/LPD, async I/O built on Tokio, real-time progress events pushed to the UI, a priority queue, bandwidth scheduling, mirror/failover management, and checksum verification.

gosh-dl is licensed under MIT. See the [gosh-dl repository](https://github.com/goshitsarch-eng/gosh-dl) for details.

## Architecture

```
+----------------------------------+
|  GTK4 + libadwaita (UI)         |
|  GObject subclasses, CSS        |
+----------------------------------+
|  EngineBridge                    |
|  mpsc channels, async-channel   |
+----------------------------------+
|  gosh-fetch-engine (Rust lib)   |
|  Direct Rust API, AppState      |
|  SQLite (rusqlite)              |
+----------------------------------+
|  gosh-dl (download engine)      |
|  HTTP, BitTorrent, async I/O    |
+----------------------------------+
```

The application is a single Rust binary. The GTK4 UI runs on the main thread (glib main loop). A dedicated background thread runs a Tokio runtime that hosts the download engine. The two sides communicate through `mpsc` channels: commands flow from the GTK thread to the engine, and events flow back via `async-channel` into the glib main loop. There is no IPC, no child process, and no JSON-RPC -- everything is direct Rust API calls within the same process.

For more detail, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Tech Stack

| Layer | Technology |
|-------|------------|
| UI | GTK4 0.9, libadwaita 0.7 |
| Toolkit bindings | gtk4-rs, glib, gio, gdk4, cairo-rs |
| Async bridge | async-channel, tokio mpsc |
| Engine | gosh-fetch-engine (gosh-dl 0.3.2) |
| Database | SQLite (rusqlite, bundled) |
| Serialization | serde, serde_json |
| Build | Cargo (Rust 2021 edition) |

## Installation

### Arch Linux (AUR)

```bash
yay -S gosh-fetch-bin
```

Available as [`gosh-fetch-bin`](https://aur.archlinux.org/packages/gosh-fetch-bin) on the AUR. Installs the prebuilt binary with a desktop entry, icons, `.torrent` file association, and `magnet:` URI handler.

### Other Linux Distributions

Download the latest release from the [Releases](https://github.com/goshitsarch-eng/Gosh-Fetch/releases) page.

## Building from Source

### Prerequisites

- [Rust](https://rustup.rs/) 1.77+
- GTK4 development libraries (4.12+)
- libadwaita development libraries (1.5+)
- SQLite development libraries (bundled by default via rusqlite, so usually not needed)
- A C compiler (gcc or clang) for native dependencies

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

### Building

```bash
# Clone the repository
git clone https://github.com/goshitsarch-eng/Gosh-Fetch.git
cd Gosh-Fetch

# Debug build
cargo build

# Release build (optimized, stripped)
cargo build --release

# Run the application
cargo run --release

# Run tests
cargo test --workspace

# Run clippy linter
cargo clippy --workspace

# Format code
cargo fmt --all
```

The release binary will be at `target/release/gosh-fetch`.

## Usage

1. **Add Download** -- Click "Add Download" or press `Ctrl+N`. Enter a URL, magnet link, or browse for a `.torrent` file. Expand "Advanced Options" for filename, directory, speed limit, headers, priority, checksum, mirrors, and more.
2. **Monitor** -- Watch real-time speed, progress, ETA, and peer info. Filter by Active, Paused, or view all.
3. **Manage** -- Pause, resume, retry, or remove downloads. Select multiple with checkboxes for batch operations. Drag to reorder priority.
4. **History** -- View completed downloads and open files or folders directly.
5. **Statistics** -- View download statistics and trends.
6. **Scheduler** -- Set up bandwidth scheduling rules based on time of day.

You can also drag URLs, magnet links, or `.torrent` files directly onto the app window.

## Privacy

- No telemetry or analytics
- No data collection
- No network activity unless explicitly initiated by you
- All data stored locally on your device

## Disclaimer

This software is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0). It is provided "as is", without warranty of any kind, express or implied, including but not limited to the warranties of merchantability or fitness for a particular purpose. Use at your own risk.

## License

AGPL-3.0 - See [LICENSE](LICENSE)

The gosh-dl download engine is licensed under MIT.

## Roadmap

Planned features for future releases:

- **Browser Extension** -- One-click downloads from your browser
- **RSS Feed Support** -- Automatic downloads from RSS/podcast feeds
- **Download Categories** -- Organize downloads by type with custom save locations
- **Import/Export** -- Backup and restore download history and settings

## Contributing

Contributions welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.
