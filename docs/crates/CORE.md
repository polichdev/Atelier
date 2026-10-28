# atelier-core

This crate contains the experimental native daemon, tray actions and IPC types. The binary starts its own taskbar manager, attempts wallpaper and audio initialization, and opens the Studio HTML in a browser. That HTML does not have Tauri IPC when opened this way. The production-oriented interactive Studio entry point is `atelier-ui`; the daemon is not a required service behind it.

The daemon applies its own initial taskbar configuration, unlike the neutral startup behavior of the Tauri Studio. Review `src/main.rs` before running it on a live desktop.
