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

| Crate | Reference |
| --- | --- |
| `atelier-common` | [Configuration and presets](crates/COMMON.md) |
| `atelier-core` | [Experimental daemon](crates/CORE.md) |
| `atelier-taskbar` | [Taskbar integration](crates/TASKBAR.md) |
| `atelier-ui` | [Studio and desktop dock](crates/UI.md) |
| `atelier-wallpaper` | [Wallpaper infrastructure](crates/WALLPAPER.md) |
| `atelier-audio` | [Audio capture](crates/AUDIO.md) |
| `atelier-widget` | [Widget infrastructure](crates/WIDGET.md) |
