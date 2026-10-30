# atelier-ui

`atelier-ui` is the primary interactive user-facing application for **Atelier**, built on the **Tauri 2.0** framework and Microsoft WebView2. It hosts the Studio control panel, the floating desktop dock, customizable desktop overlay widgets, and the background live wallpaper runtime window.

```mermaid
flowchart TD
    subgraph Windows ["Tauri Multi-Window Architecture"]
        MainWin["Main Window ('main')<br/>Studio Control Panel<br/>ui/index.html"]
        DockWin["Dock Window ('desktop-dock')<br/>Floating Layered AppBar<br/>ui/dock.html"]
        WidgetWins["Widget Windows ('widget-*')<br/>Movable Overlay Windows<br/>ui/widget.html"]
        WallpaperWin["Wallpaper Window ('desktop-wallpaper')<br/>WorkerW Injected Canvas<br/>ui/wallpaper.html"]
    end

    subgraph State ["Tauri Managed State"]
        AppState["AppState<br/>(Taskbar, Audio, Core Mutex)"]
        DockState["DockState<br/>(Running Apps, Pins, AppBar State)"]
        WallState["Wallpapers<br/>(Active Wallpaper, Settings, Profiles)"]
        WidgetState["Widgets<br/>(Positions, Sizes, Enabled States)"]
    end

    subgraph Protocols ["Asynchronous Custom URI Protocols"]
        P1["wallpaper-media://<br/>HTTP Range Video Streaming"]
        P2["workshop-media://<br/>Steam Workshop Asset Streaming"]
        P3["wallpaper-scene://<br/>PKG On-Demand Scene Decompression"]
        P4["widget-photo://<br/>Local User Photo Asset Serving"]
    end

    MainWin --> State
    DockWin --> State
    WidgetWins --> State
    WallpaperWin --> State

    WallpaperWin -.-> P1
    WallpaperWin -.-> P3
    MainWin -.-> P2
    WidgetWins -.-> P4
```

---

## Architectural Highlights

### 1. Multi-Window Lifecycle & Window Management
- **Mica Studio Shell**: The main configuration window utilizes Windows 11 Mica backdrop composition with dark mode styling, providing access to Taskbar, Dock, Wallpapers, Widgets, Audio, and General settings.
- **Desktop Dock Window**: Operates as a transparent, layered, frameless window anchored to display edges (bottom, top, left, right). It communicates directly with Windows shell services using custom AppBars to reserve work areas.
- **Desktop Widgets**: Each enabled desktop widget runs in its own lightweight WebView window with transparent background, click-through toggles, and DPI-aware positioning.
- **Desktop Wallpaper Window**: Injected directly behind the desktop icons into the Windows `WorkerW` layer (between `SHELLDLL_DefView` and the desktop background).

```mermaid
sequenceDiagram
    autonumber
    participant Main as Studio (ui/index.html)
    participant Rust as Tauri Rust Backend (crates/ui)
    participant WorkerW as Windows WorkerW Layer
    participant Webview as Wallpaper Host (ui/wallpaper.html)

    Main->>Rust: invoke("wallpaper_apply", { id, path, mode })
    Rust->>Rust: Check performance policies & battery state
    Rust->>WorkerW: Acquire WorkerW HWND via Progman 0x052C
    Rust->>Webview: Create/reposition transparent webview window
    Rust->>WorkerW: SetParent(webview_hwnd, workerw_hwnd)
    Rust->>Webview: Send URL (wallpaper-scene:// or wallpaper-media://)
    Webview->>Rust: Fetch asset chunks via custom protocol
    Rust-->>Webview: 200 OK / 206 Partial Content
    Webview-->>Main: emit("wallpaper-status", { playing: true })
```

---

## Key Backend Modules

| Module | Source File | Description |
| :--- | :--- | :--- |
| **Desktop Dock** | [`src/desktop_dock.rs`](src/desktop_dock.rs) | Taskbar enumeration, Explorer icon extraction, UWP ApplicationFrameWindow matching, pinned shortcut persistence, Windows AppBar space reservation, and UI Automation system tray popups. |
| **Desktop Wallpaper** | [`src/desktop_wallpaper.rs`](src/desktop_wallpaper.rs) | Live wallpaper lifecycle controller, profile persistence, audio reactivity options, and `wallpaper-media://` byte-range video streaming. |
| **Scene Engine Bridge** | [`src/wallpaper_scene.rs`](src/wallpaper_scene.rs) | Extraction of Wallpaper Engine `.pkg` archives, JSON project parsing, and `wallpaper-scene://` protocol asset server. |
| **Scene Geometry** | [`src/scene_geometry.rs`](src/scene_geometry.rs) | Scene layer transformations, parent-child hierarchies, 2D bone rigs, and mesh deformation resolution. |
| **Scene Effects** | [`src/scene_effects.rs`](src/scene_effects.rs) | Shader effect passes, uniform binding evaluation, author composite switches, and arithmetic expressions. |
| **Scene Preview** | [`src/scene_preview.rs`](src/scene_preview.rs) | Offscreen CPU/GPU thumbnail composition engine generating full-fidelity PNG previews of 2D scenes. |
| **Performance Watcher**| [`src/wallpaper_performance.rs`](src/wallpaper_performance.rs) | Real-time monitoring of fullscreen/maximized applications, battery status, process execution pause rules, and display frame rate caps. |
| **Steam Workshop** | [`src/steam_workshop.rs`](src/steam_workshop.rs) | Automatic detection of Steam install folders and Wallpaper Engine Workshop downloads (`workshop/content/431960`), VDF parsing, and safe importing. |
| **Desktop Widgets** | [`src/desktop_widgets.rs`](src/desktop_widgets.rs) | Widget position normalization, multi-monitor bounds validation, drag-and-drop coordinate persistence, and audio spectrum dispatch. |
| **Desktop Background** | [`src/desktop_background.rs`](src/desktop_background.rs) | Safe backup and atomic restoration of the native Windows wallpaper when live wallpapers are toggled or on application exit. |

---

## Custom Protocol Handlers

`atelier-ui` registers four asynchronous URI scheme protocols to safely stream local assets to WebViews without triggering Chromium cross-origin or file-access security errors:

1. **`wallpaper-media://`**: Supports HTTP `Range: bytes=start-end` requests, allowing seamless hardware-accelerated video streaming and loop seeking for large MP4 and WebM videos.
2. **`wallpaper-scene://`**: Resolves local scene directories and `.pkg` archive contents on the fly, streaming unpacked images, models, and JSON definitions directly into the WebGL player.
3. **`workshop-media://`**: Provides sandboxed access to preview images and media files stored in Steam Workshop directories.
4. **`widget-photo://`**: Serves local GIF and image uploads for the desktop photo widget.

---

## Frontend Web Architecture

Located in [`ui/`](ui/):
- **Studio UI (`index.html`)**: Vanilla JavaScript SPA with Tailwind-styled glassmorphism UI, tabbed panels, live sliders, and reactive color pickers.
- **Dock UI (`dock.html`, `dock.js`, `dock-audio.js`)**: CSS flexbox dock with smooth magnification physics, badge indicators, Start/Search buttons, system tray popups, and audio glow effects.
- **Scene Player (`scene-player.js`, `scene-properties.js`)**: Canvas/WebGL 2D scene playback engine supporting multi-layer blending, sprite animations, particle simulations, bone transforms, and author property bindings.
- **Audio Visualizer (`audio-visualizer.js`, `widget.js`)**: Canvas-based 60 FPS audio visualizer rendering bars, waves, dots, circular spectrums, and symmetric pulses.

---

## Building and Testing

```powershell
# Run Rust unit tests for atelier-ui (47 tests)
cargo test -p atelier-ui

# Run JavaScript frontend unit tests (30 tests)
node --test crates/ui/tests/*.test.cjs

# Launch developer mode with live reload
scripts/run-dev.cmd
```
