# Repository Structure

```text
atelier/
├── Cargo.toml                    Cargo workspace and shared dependencies
├── crates/
│   ├── common/           Shared configuration, presets and errors
│   ├── core/             Separate experimental daemon and tray
│   ├── taskbar/          Explorer styling, positioning and XAML bridge
│   ├── ui/               Tauri application, dock and frontend
│   ├── wallpaper/        WorkerW, Direct3D and media infrastructure
│   ├── audio/            WASAPI capture and spectrum logic
│   └── widget/           Widget windows and data providers
├── docs/                         Architecture, guides and crate references
├── packaging/                    NSIS and MSIX packaging sources
├── scripts/                      Development and dock test scripts
└── .github/workflows/            Windows CI configuration
```

`crates/ui/ui/` contains the Studio and dock web assets. `crates/taskbar/native/` contains the C++ XAML bridge and probe. `target/` is generated output and is ignored. The root `atelier_xaml.dll`, if present, is not needed as a checked-in build artifact: `build.rs` builds the embedded bridge.

## Crate reference

Directory names and Cargo package names differ. Use the package name with `cargo -p`.

| Directory | Cargo package | Reference |
| --- | --- | --- |
| `crates/common` | `atelier-common` | [Configuration and presets](crates/COMMON.md) |
| `crates/core` | `atelier-core` | [Experimental daemon](crates/CORE.md) |
| `crates/taskbar` | `atelier-taskbar` | [Taskbar integration](crates/TASKBAR.md) |
| `crates/ui` | `atelier-ui` | [Studio and desktop dock](crates/UI.md) |
| `crates/wallpaper` | `atelier-wallpaper` | [Wallpaper infrastructure](crates/WALLPAPER.md) |
| `crates/audio` | `atelier-audio` | [Audio capture](crates/AUDIO.md) |
| `crates/widget` | `atelier-widget` | [Widget infrastructure](crates/WIDGET.md) |
