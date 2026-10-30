# Wallpapers and Steam Workshop

Atelier includes a hardware-accelerated desktop wallpaper engine capable of playing looped MP4 and WebM videos as well as interactive 2D Wallpaper Engine scenes directly behind desktop icons.

```mermaid
flowchart TD
    subgraph Sources ["Wallpaper Sources"]
        LocalVideo["Local MP4 / WebM Videos<br/>(Imported from Disk)"]
        SteamWS["Steam Workshop Items<br/>(Discovered from Steam Library)"]
        Presets["Bundled Preset Catalog<br/>(wallpaper-catalog.json)"]
    end

    subgraph Core ["Atelier Wallpaper Manager (desktop_wallpaper.rs)"]
        Library["Local Wallpaper Library<br/>(%APPDATA%/Atelier/wallpapers)"]
        EngineRouter{"Wallpaper Type"}
        VideoPlayer["Hardware Video Player<br/>(Media Foundation / HTML5 Video)"]
        ScenePlayer["WebGL 2D Scene Engine<br/>(scene-player.js)"]
    end

    subgraph Desktop ["Desktop Presentation"]
        WorkerW["Windows WorkerW Layer<br/>(Behind Desktop Icons)"]
        AudioGlow["Audio Pulse & Glow<br/>(atelier-audio)"]
        OcclusionEngine["Occlusion & Pause Engine<br/>(wallpaper_performance.rs)"]
    end

    LocalVideo --> Library
    SteamWS --> Library
    Presets --> Library

    Library --> EngineRouter
    EngineRouter -- "MP4 / WebM" --> VideoPlayer
    EngineRouter -- "2D Scene" --> ScenePlayer

    VideoPlayer --> WorkerW
    ScenePlayer --> WorkerW
    AudioGlow -.-> WorkerW
    OcclusionEngine -.-> EngineRouter
```

---

## Architectural Highlights

### 1. Seamless Video Looping with Frame Crossfading
Standard HTML5 video elements often produce noticeable micro-stutters or black flashes when seeking back to timestamp zero at the end of a loop. Atelier resolves this through a frame-holding crossfading strategy:

```mermaid
sequenceDiagram
    autonumber
    participant Engine as desktop_wallpaper.rs
    participant Player as wallpaper.js Video Canvas
    participant Video as HTML5 Video Element

    Note over Video: Video playing near end of file (t = Duration - 0.2s)
    Player->>Player: Snapshot current frame to hidden offscreen buffer
    Video->>Player: 'ended' or 'timeupdate' (wrap detected)
    Player->>Player: Display offscreen buffer to avoid black flash
    Player->>Video: currentTime = 0
    Video->>Player: 'seeked' event
    Player->>Player: Crossfade from buffer into new frame over 150ms
    Video->>Player: Resume smooth playback loop
```

---

### 2. Steam Workshop Discovery & Safe Importing
Atelier can discover wallpapers you have already subscribed to and downloaded in Steam without requiring Steam to remain open or running Depot downloaders:
1. **Steam Path Discovery**: Scans Windows registry keys (`HKCU\Software\Valve\Steam`) and `libraryfolders.vdf` to discover all Steam library drives.
2. **Workshop Directory**: Scans `steamapps/workshop/content/431960/` for Wallpaper Engine items.
3. **Item Parsing**: Reads `project.json` to extract titles, previews (`preview.jpg`), type tags (`scene`, `video`, `web`), and author information.
4. **Isolated Copying**: Importing copies the video or extracts the `.pkg` into Atelier's local library (`%APPDATA%\Atelier\wallpapers\`). The wallpaper continues to work even if Steam is completely closed or offline.

---

### 3. Comprehensive Performance & Pause Decision Pipeline

```mermaid
flowchart TD
    Start["Frame Tick / State Evaluation"] --> CheckManual{"Manual Pause Toggled?"}
    CheckManual -- "Yes" --> Pause["PAUSE WALLPAPER (0% GPU / CPU)"]
    CheckManual -- "No" --> CheckBattery{"On Battery & PauseOnBattery Enabled?"}

    CheckBattery -- "Yes" --> Pause
    CheckBattery -- "No" --> CheckProcess{"Foreground App in Pause Rules?"}

    CheckProcess -- "Yes" --> Pause
    CheckProcess -- "No" --> CheckFullscreen{"Fullscreen App Active on Display?"}

    CheckFullscreen -- "Yes" --> Pause
    CheckFullscreen -- "No" --> CheckMaximized{"Maximized App & Pause Policy Active?"}

    CheckMaximized -- "Yes" --> Pause
    CheckMaximized -- "No" --> Play["RENDER FRAME (Active Playback)"]
```

### Policy Hierarchy & Override Rules
1. **Application Rules**: Explicit per-executable rules (`chrome.exe: pause`, `game.exe: pause`) always take highest priority.
2. **Global Policies**: Configured in **Wallpapers > Settings** for battery saver, fullscreen apps, and other audio playback.
3. **Per-Wallpaper Preferences**: Individual wallpaper settings (volume, fit, scaling, fullscreen toggle) inherit global defaults unless explicitly overridden.

---

## Supported Wallpaper Types

| Type | Extension / Format | Engine | Audio Reactive? |
| :--- | :--- | :--- | :--- |
| **Video** | `.mp4`, `.webm` | Hardware Video Decoder | Yes (Pulse & Glow) |
| **2D Scene** | `scene.pkg`, `scene.json` | WebGL 2D Scene Player | Yes (Pulse, Glow & Engine scripts) |
| **Web** | `index.html` | Chromium Sandbox | Under Development |
| **3D Scene** | `.pkg` (3D Models) | WebGL 3D Engine | Experimental (Partial) |
