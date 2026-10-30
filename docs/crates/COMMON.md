# atelier-common

Directory: [`crates/common`](../../crates/common/) · Cargo package: `atelier-common`

`atelier-common` defines the foundational domain models, configuration schemas, named appearance presets, and unified error handling types shared across the entire Atelier workspace.

```mermaid
classDiagram
    class AtelierConfig {
        +autostart: bool
        +taskbar: TaskbarConfig
        +validate()
        +normalize()
    }

    class TaskbarConfig {
        +position: TaskbarPosition
        +appearance: TaskbarAppearance
        +floating_dock: bool
        +dock_margin: u32
    }

    class TaskbarAppearance {
        +style: TaskbarStyle
        +color: [u8; 4]
        +blur_radius: u32
        +border_color: [u8; 4]
        +border_width: u32
        +round_corners: bool
    }

    class TaskbarStyle {
        <<enumeration>>
        Default
        Transparent
        Blur
        Acrylic
        Mica
        Colored
    }

    class TaskbarPosition {
        <<enumeration>>
        Bottom
        Top
        Left
        Right
    }

    class AtelierError {
        <<enumeration>>
        Platform(String)
        Win32(windows::core::Error)
        Io(std::io::Error)
        Json(serde_json::Error)
        NotFound(String)
        Generic(String)
    }

    AtelierConfig *-- TaskbarConfig
    TaskbarConfig *-- TaskbarAppearance
    TaskbarConfig *-- TaskbarPosition
    TaskbarAppearance *-- TaskbarStyle
```

---

## Architectural Highlights

### 1. Unified Configuration Schema (`config.rs`)
- Implements `Serialize` and `Deserialize` using Serde with backward-compatibility fallbacks.
- Provides default configurations matching stock Windows appearance.
- Includes automatic normalization methods that validate colors, ensure valid alpha channels, clamp margins to physical monitor boundaries, and sanitize user input.

```mermaid
flowchart LR
    RawJSON["settings.json (Disk)"] --> Serde["Serde Deserializer"]
    Serde --> Migration["Backward Compatibility Check"]
    Migration --> Normalizer["normalize() & validate()"]
    Normalizer --> InMem["AtelierConfig (Validated Memory Model)"]
```

---

### 2. Built-in Appearance Presets (`presets.rs`)
Provides pre-tuned appearance profiles that can be applied with a single click or keyboard shortcut:

| Preset Name | Style | Visual Description | Color & Alpha |
| :--- | :--- | :--- | :--- |
| **Default** | `Default` | Standard native Windows 10/11 taskbar appearance. | OS Native |
| **MacOsDock** | `Acrylic` | Centered floating dock with rounded corners and translucent frosted glass backdrop. | `rgba(30, 30, 30, 0.7)` |
| **Windows11Clear**| `Transparent`| Completely transparent background with crisp app icons and system tray elements. | `rgba(0, 0, 0, 0.0)` |
| **CyberpunkNeon** | `Colored` | Deep purple translucent background with hot-pink border accents. | `rgba(45, 12, 60, 0.8)` |
| **MinimalDark** | `Blur` | Subtle dark blur backdrop with zero borders for distraction-free workflows. | `rgba(18, 18, 18, 0.85)` |
| **FrostedGlass** | `Mica` | Soft Windows 11 Mica composition that adapts smoothly to dark and light desktop wallpapers. | DWM Composition |

---

### 3. Unified Error Architecture (`error.rs`)
- Uses `thiserror` to provide ergonomic, strongly-typed error variants for Win32 API failures, I/O errors, JSON parse issues, and platform incompatibilities.
- Standardizes error conversion across all workspace crates using a common `Result<T, AtelierError>` type alias.

---

## Testing

```powershell
# Run unit tests for atelier-common (3 tests)
cargo test -p atelier-common
```
Tests cover default appearance, backward-compatible configuration deserialization, and button width normalization.
