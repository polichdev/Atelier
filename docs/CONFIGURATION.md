# Configuration & Storage

## Data directory

By default Studio uses `%APPDATA%\Atelier`. Set `ATELIER_DATA_DIR` before launching to choose another directory. The dock and XAML bridge have temporary-directory fallbacks in some paths when the normal data directory is unavailable.

| File | Purpose |
| --- | --- |
| `settings.json` | Taskbar configuration written after changes and resets. The Tauri application starts with default taskbar settings rather than reapplying this file at launch. |
| `dock-settings.json` | Dock configuration, pinned paths, system-button visibility and optional custom icon data. Explicitly enabled dock settings may be restored at launch. |
| `wallpapers/` | Local wallpaper library (`library.json`), imported video and scene files, per-wallpaper and global playback settings, and WebView data. |
| `widgets.json` | Widget configuration, positions and enabled state, including the Audio visualizer. |
| `widget-photos/` | Stored GIF assets for photo widgets. |
| `cache/workshop-previews/` | Cached compositions for supported downloaded Workshop scenes. |
| `atelier_xaml_<hash>.dll` | Extracted copy of the embedded XAML bridge, unless a matching copy is found beside the executable. |

The dock imports existing pinned Windows taskbar shortcuts on first enable. It attempts to use the Windows taskbar order; if that data cannot be matched reliably, it falls back to filename order. An untouched alphabetical import from an earlier version is corrected when possible. Manually reordered pins and custom icons are preserved and remain independent of Explorer's shortcuts. Windows-managed apps that do not use shortcut files may not all be imported. Resetting Windows appearance from Studio also resets the dock settings.

Dock audio effects, hidden-app preferences and the maximized-window bottom gap are stored in `dock-settings.json`. Wallpaper audio effects are stored per wallpaper in the library configuration. These files can include local application paths, custom content and widget data; treat them as personal data when sharing diagnostics.

## Environment variables

| Variable | Use |
| --- | --- |
| `ATELIER_DATA_DIR` | Override the application's data directory. |
| `ATELIER_DOCK_ONLY_TEST` | Enable isolated dock test mode in debug builds. |
| `ATELIER_DOCK_NATIVE_TEST` | Run the native dock script in debug test mode. |
| `NODE_PATH` | Optional Node module lookup path for Playwright tests. |
| `ATELIER_TEST_BROWSER` | Optional Playwright browser channel, such as `msedge`. |

The test variables are intended for development fixtures, not normal usage. Avoid manually editing settings while Atelier is running: a subsequent save can overwrite changes.
