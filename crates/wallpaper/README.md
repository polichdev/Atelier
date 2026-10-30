# atelier-wallpaper

`atelier-wallpaper` provides the low-level Windows desktop wallpaper engine for **Atelier**. It manages desktop window hierarchy injection via `WorkerW`, Direct3D 11 rendering, Media Foundation hardware-accelerated video decoding, display occlusion monitoring, and decompression of Wallpaper Engine `.pkg` archives and `.tex` textures.

```mermaid
flowchart TD
    subgraph DesktopLayering ["Windows Desktop Window Hierarchy"]
        Progman["Progman (Program Manager)"]
        WorkerW["WorkerW (Desktop Background Layer)"]
        ShellView["SHELLDLL_DefView (Desktop Icons)"]
        AtelierHost["AtelierWallpaperHostClass<br/>(Injected Wallpaper Window)"]
    end

    Progman -- "SendMessageTimeout(0x052C)" --> WorkerW
    WorkerW --> AtelierHost
    WorkerW --> ShellView

    subgraph WallpaperPipelines ["Wallpaper Engine Pipelines"]
        D3D["Direct3D 11 Pipeline<br/>(D3D11Renderer)"]
        MF["Media Foundation Pipeline<br/>(VideoWallpaperPlayer)"]
        PkgDecoder["PKG / TEX Decoder Pipeline<br/>(package.rs, texture.rs)"]
    end

    AtelierHost --> D3D
    AtelierHost --> MF
    PkgDecoder --> AtelierHost
```

---

## Architectural Highlights

### 1. WorkerW Desktop Host Window Acquisition
In Windows, desktop icons are hosted inside `SHELLDLL_DefView`. To display a live wallpaper beneath desktop icons without covering them, Atelier sends an undocumented message (`0x052C`) to `Progman`. This causes the Windows Explorer shell to spawn a `WorkerW` window sibling between `Progman` and the icons:

```mermaid
sequenceDiagram
    autonumber
    participant App as atelier-wallpaper
    participant Prog as Progman / ShellWindow
    participant Explorer as Windows Explorer
    participant Worker as WorkerW

    App->>Prog: FindWindowW("Progman", NULL)
    App->>Prog: SendMessageTimeout(0x052C, 0x0000000D, 1)
    App->>Prog: SendMessageTimeout(0x052C, 0, 0)
    Explorer->>Worker: Spawn WorkerW behind desktop icons
    App->>Explorer: EnumWindows(find WorkerW sibling of SHELLDLL_DefView)
    Explorer-->>App: Return HWND of WorkerW
    App->>Worker: SetParent(atelier_host_hwnd, workerw_hwnd)
```

If Windows 11 disables redirection bitmaps (`WS_EX_NOREDIRECTIONBITMAP`), the engine directly binds to `Progman` as fallback.

---

### 2. Wallpaper Engine Package and Texture Pipeline

```mermaid
flowchart LR
    PKG[".pkg Archive File"] --> PKG_Reader["package.rs<br/>(Header, Index Table, Entries)"]
    PKG_Reader --> JSON["scene.json<br/>(model.rs)"]
    PKG_Reader --> TEX[".tex Files"]
    TEX --> TEX_Decoder["texture.rs<br/>(LZ4 Decompression, DXT1/3/5)"]
    TEX_Decoder --> RGBA["Decoded RGBA Textures"]
    RGBA --> Canvas["Offscreen Preview / WebGL Layer"]
```

- **`package.rs`**: Parses binary package archives (`PKGV0001`, `PKGV0002`). Reads entry metadata without loading the entire archive into RAM, and extracts individual assets on demand.
- **`texture.rs`**: Decodes proprietary Wallpaper Engine `.tex` formats, supporting LZ4 decompression, DXT1 (BC1), DXT3 (BC2), DXT5 (BC3) block decompression, and v4 extra headers.
- **`model.rs`**: Strongly-typed Serde data structures representing projects, scenes, cameras, layer hierarchies, 2D sprite meshes, bone chains, and effect passes.

---

### 3. Occlusion and Power Management

```mermaid
stateDiagram-v2
    [*] --> Active : Wallpaper Running
    Active --> Paused_Fullscreen : Foreground window covers display
    Active --> Paused_Battery : System running on DC battery power
    Active --> Paused_Manual : User toggled manual pause
    Paused_Fullscreen --> Active : Foreground window minimized or resized
    Paused_Battery --> Active : AC power reconnected
    Paused_Manual --> Active : User resumed playback
```

- **`occlusion.rs`**: Checks whether a full-screen window or game is occluding the wallpaper display using `GetClientRect` and `ClientToScreen` rather than bounding boxes, preventing maximized windows with visible taskbars from triggering false pause states.
- Automatically queries `GetSystemPowerStatus` to pause live animations on battery power.

---

## Crate Modules

| Module | Purpose |
| :--- | :--- |
| [`src/workerw.rs`](src/workerw.rs) | WorkerW acquisition, fallback strategies, host window class registration. |
| [`src/occlusion.rs`](src/occlusion.rs) | Fullscreen occlusion detection, monitor bounds comparison, battery power monitoring. |
| [`src/package.rs`](src/package.rs) | Wallpaper Engine `.pkg` archive reader and asset extractor. |
| [`src/texture.rs`](src/texture.rs) | `.tex` texture decoding, LZ4 decompression, DXT1/3/5 block decompressor. |
| [`src/model.rs`](src/model.rs) | Project, scene, layer, geometry, and attachment schemas. |
| [`src/renderer.rs`](src/renderer.rs) | Direct3D 11 device creation, swapchain management, rendering loop. |
| [`src/video.rs`](src/video.rs) | Media Foundation player context and hardware video decode. |

---

## Testing

```powershell
# Run unit tests for atelier-wallpaper (10 tests)
cargo test -p atelier-wallpaper
```
Unit tests cover DXT block decoders, LZ4 compression bounds, package directory traversal safety, and occlusion calculation edge cases.
