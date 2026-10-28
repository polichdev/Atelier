# atelier-ui

`atelier-ui` is the Tauri Studio. `src/main.rs` dispatches helper and test modes; `src/lib.rs` configures state, IPC, tray and window lifecycle. `ui/index.html` is the Studio; `studio-dock.js` manages its dock controls. `ui/dock.html` and `dock.js` implement the separate dock window.

`src/desktop_dock.rs` manages dock settings, pinned shortcuts, application/window enumeration, icons, placement, launch and activation. When enabled, the dock hides Explorer's taskbars after starting recovery helpers. The system tray is not replicated. The Tauri configuration declares main and dock window capabilities and requests a Mica Studio backdrop.

State files and override variables are listed in [Configuration](../CONFIGURATION.md). Unit, browser and native fixture tests are described in [Development](../DEVELOPMENT.md).
