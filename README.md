# Atelier

[![Rust](https://img.shields.io/badge/Rust-stable-orange?logo=rust)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8DB?logo=tauri&logoColor=white)](https://v2.tauri.app/)
[![Windows](https://img.shields.io/badge/Windows-10%20%2F%2011-0078D7?logo=windows)](docs/COMPATIBILITY.md)
[![Win32](https://img.shields.io/badge/Windows-Win32%20API-0078D7?logo=windows)](docs/crates/TASKBAR.md)
[![WebView2](https://img.shields.io/badge/UI-WebView2-0078D7?logo=microsoftedge)](docs/crates/UI.md)
[![C++ XAML bridge](https://img.shields.io/badge/Native-C%2B%2B%20XAML%20bridge-00599C?logo=cplusplus)](docs/crates/TASKBAR.md)
[![Direct3D 11](https://img.shields.io/badge/Graphics-Direct3D%2011-0078D7?logo=windows)](docs/crates/WALLPAPER.md)
[![WASAPI](https://img.shields.io/badge/Audio-WASAPI-7D57C1)](docs/crates/AUDIO.md)
[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue)](#license)

[![Taskbar styling](https://img.shields.io/badge/Windows%20Taskbar-Styles%20%26%20Presets-3182CE)](#features)
[![Desktop dock](https://img.shields.io/badge/macOS%20Taskbar-Desktop%20Dock-8B5CF6)](#features)
[![Multi-monitor](https://img.shields.io/badge/Dock-Multi--monitor-14B8A6)](docs/crates/UI.md)
[![Recovery](https://img.shields.io/badge/Shell-Recovery%20Helpers-22A06B)](docs/FAILURES_AND_RECOVERY.md)
[![In development](https://img.shields.io/badge/Wallpaper%20%2F%20Audio%20%2F%20Widgets-In%20Development-D97706)](#features)

Atelier is a Windows desktop customization project built with Rust and Tauri v2. Its Studio can customize the Explorer taskbar and provide an optional, separate macOS-style dock. Wallpaper, audio and widget subsystems exist in the workspace, but their Studio controls are still in development.

## Features

- **Windows Taskbar:** configure materials, floating geometry and presets. On supported Windows 11 XAML taskbars, adjust app button and icon appearance.
- **macOS Taskbar:** a separate dock for pinned shortcuts and running windows, with display and edge selection, sizing, layouts, indicators and auto-hide. Enabling it hides the native Windows taskbar, including the notification area.
- **Recovery:** the dock uses a helper process before hiding Explorer windows; native taskbar positioning has a separate guardian. See [Failure Modes & Recovery](docs/FAILURES_AND_RECOVERY.md).
- **Planned Studio controls:** wallpaper playback, audio visualization and widgets. Their presence in the workspace does not mean their complete user-facing flows are implemented.

## Architecture

```mermaid
flowchart LR
    Studio["Studio UI · WebView2"] <-->|"Tauri commands and events"| App["atelier-ui · Rust"]
    App --> Taskbar["atelier-taskbar"]
    Taskbar --> Explorer["Explorer taskbar"]
    Taskbar --> Bridge["C++ XAML bridge"]
    Bridge --> Explorer
    App --> Dock["Dock window and state"]
    Dock --> Explorer
    Dock --> Apps["Windows apps and shortcuts"]
    App --> Other["Wallpaper, audio and widget modules"]
    Dock --> Guardian["Recovery helper"]
```

The Tauri application is the primary Studio entry point. The separate `atelier-core` binary has its own tray and initialization path and is not the Tauri IPC backend. Details: [Architecture](docs/ARCHITECTURE.md).

## Documentation

### Guides and architecture

- [Architecture and data flow](docs/ARCHITECTURE.md)
- [Repository structure](docs/STRUCTURE.md)
- [Configuration and storage](docs/CONFIGURATION.md)
- [Compatibility](docs/COMPATIBILITY.md)
- [Failure modes and recovery](docs/FAILURES_AND_RECOVERY.md)
- [Development and testing](docs/DEVELOPMENT.md)
- [Security considerations](docs/SECURITY.md)
- [Contributing](CONTRIBUTING.md) · [Changelog](CHANGELOG.md)

### Crate references

The names below match the directories in `crates/`. Cargo package names retain the `atelier-` prefix.

- [`common`](docs/crates/COMMON.md) — configuration, presets and errors.
- [`core`](docs/crates/CORE.md) — experimental daemon and tray.
- [`taskbar`](docs/crates/TASKBAR.md) — Explorer styling, positioning and recovery.
- [`ui`](docs/crates/UI.md) — Studio and the separate dock.
- [`wallpaper`](docs/crates/WALLPAPER.md) — wallpaper infrastructure.
- [`audio`](docs/crates/AUDIO.md) — audio capture and spectrum analysis.
- [`widget`](docs/crates/WIDGET.md) — widget infrastructure.

## Build and run

Prerequisites: Windows 10/11, a stable Rust MSVC toolchain, Visual C++ Build Tools with a Windows SDK, and WebView2 Runtime. The XAML bridge is compiled from C++ during the Cargo build. See [Compatibility](docs/COMPATIBILITY.md) for Windows-specific limitations.

```powershell
cargo check --workspace
cargo run -p atelier-ui --bin Atelier
```

For a development copy that does not lock `target\debug\Atelier.exe`, run `scripts\run-dev.cmd`. It stops matching Atelier development processes before rebuilding; see [Development](docs/DEVELOPMENT.md).

## Tests

```powershell
cargo test -p atelier-common
cargo test -p atelier-ui --lib
cargo check --workspace
```

The repository also contains an isolated dock smoke test and Playwright UI tests. Some taskbar integration tests interact with the real Explorer taskbar; read [Development and testing](docs/DEVELOPMENT.md) before running them.

## License

The workspace declares `MIT OR Apache-2.0` in `Cargo.toml`. License text files are not currently included in the repository.
