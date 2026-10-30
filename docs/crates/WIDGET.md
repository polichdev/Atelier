# widget

Directory: [`crates/widget`](../../crates/widget/) · Cargo package: `atelier-widget`

The crate contains clock, hardware statistics, media and layered-window modules. `WidgetSubsystem` exposes polling and overlay window construction. Studio's Widgets tab manages clock, calendar, system, media, battery, notes, weather, photo and countdown widgets. A system-audio visualizer is configured in the Audio tab and uses the same window, move and resize infrastructure. The Tauri-side implementation in `crates/ui/src/desktop_widgets.rs` creates and configures the desktop widget windows; `studio-widgets.js` provides their Studio controls. Weather uses Open-Meteo, and widget settings and photo assets are stored locally. See [Configuration](../CONFIGURATION.md).
