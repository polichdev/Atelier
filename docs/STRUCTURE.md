# Repository Structure

```text
atelier/
├── Cargo.toml                    Cargo workspace and shared dependencies
├── crates/
│   ├── atelier-common/           Shared configuration, presets and errors
│   ├── atelier-core/             Separate experimental daemon and tray
│   ├── atelier-taskbar/          Explorer styling, positioning and XAML bridge
│   ├── atelier-ui/               Tauri application, dock and frontend
│   ├── atelier-wallpaper/        WorkerW, Direct3D and media infrastructure
│   ├── atelier-audio/            WASAPI capture and spectrum logic
│   └── atelier-widget/           Widget windows and data providers
├── docs/                         Architecture, guides and crate references
├── packaging/                    NSIS and MSIX packaging sources
├── scripts/                      Development and dock test scripts
└── .github/workflows/            Windows CI configuration
```

`crates/atelier-ui/ui/` contains the Studio and dock web assets. `crates/atelier-taskbar/native/` contains the C++ XAML bridge and probe. `target/` is generated output and is ignored. The root `atelier_xaml.dll`, if present, is not needed as a checked-in build artifact: `build.rs` builds the embedded bridge.

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
