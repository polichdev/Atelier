# atelier-taskbar

Directory: [`crates/taskbar`](../../crates/taskbar/) · Cargo package: `atelier-taskbar`

`atelier-taskbar` is the native taskbar manipulation and styling engine for **Atelier**. It discovers Windows taskbar windows, applies DWM composition styles (Acrylic, Mica, Transparent), computes centered floating dock window clipping regions, coordinates Explorer crash recovery, and loads a C++20 XAML visual tree injection bridge for Windows 11.

```mermaid
flowchart TD
    subgraph Discovery ["Taskbar Discovery (discovery.rs)"]
        Primary["Shell_TrayWnd (Primary Display Taskbar)"]
        Secondary["Shell_SecondaryTrayWnd (Multi-Monitor Taskbars)"]
        TrayArea["TrayNotifyWnd (Notification Area)"]
    end

    subgraph Composition ["Window Composition Engine"]
        Accent["accent.rs<br/>SetWindowCompositionAttribute<br/>(AccentPolicy: Acrylic, Mica, Blur)"]
        Style["style.rs<br/>DWM Tint & Border Composition"]
        DockRegion["dock.rs<br/>SetWindowRgn Floating Dock Margins"]
    end

    subgraph NativeBridge ["Windows 11 XAML Bridge"]
        XamlRS["xaml.rs<br/>Embedded DLL Extractor & Loader"]
        CppBridge["native/xaml_bridge.cpp<br/>(C++20 WinUI Tree Hook)"]
    end

    subgraph Guardians ["Reliability & Recovery"]
        Recovery["recovery.rs<br/>Listens for 'TaskbarCreated' Message"]
        Guardian["position.rs<br/>--native-taskbar-guardian Process"]
    end

    Primary --> Accent
    Secondary --> Accent
    Accent --> Style
    Primary --> DockRegion
    Primary --> XamlRS
    XamlRS --> CppBridge
    Recovery -.-> Primary
    Guardian -.-> Primary
```

---

## Architectural Highlights

### 1. Dual-Path Taskbar Styling (DWM + XAML Bridge)
Windows 10 and Windows 11 handle taskbar rendering differently:
- **Windows 10 & Win11 (DWM Accent Policy)**: Invokes the undocumented `SetWindowCompositionAttribute` API to inject `ACCENT_ENABLE_BLURBEHIND`, `ACCENT_ENABLE_ACRYLICBLURBEHIND`, or `ACCENT_ENABLE_TRANSPARENTGRADIENT`.
- **Windows 11 Modern Taskbar (XAML Visual Tree)**: The Windows 11 taskbar is hosted inside `Taskbar.ExperienceHost` and uses WinUI/XAML. Standard Win32 calls cannot alter internal XAML button layouts. Atelier uses an embedded C++20 bridge (`atelier_xaml.dll`) to safely traverse the modern visual tree and apply spacing, margin, and backdrop adjustments.

```mermaid
sequenceDiagram
    autonumber
    participant App as atelier-taskbar
    participant Win32 as Win32 API
    participant Bridge as C++ XAML Bridge (atelier_xaml.dll)
    participant Explorer as Windows Explorer (Shell_TrayWnd)

    App->>Win32: FindWindowW("Shell_TrayWnd", NULL)
    Win32-->>App: HWND of taskbar
    App->>Win32: SetWindowCompositionAttribute(HWND, AccentPolicy)
    App->>Bridge: ApplyMacOsDockStyle(HWND, width, margin, radius)
    Bridge->>Explorer: Inspect XAML Visual Tree elements
    Bridge->>Explorer: Apply corner radius & margins
    Bridge-->>App: OK (Original visual state cached for restore)
```

---

### 2. Floating Dock Region Clipping
To transform the full-width rectangular Windows taskbar into an elegant centered floating dock:
- `dock.rs` calculates content boundaries based on active app icons and pinned shortcuts.
- Generates a rounded rectangular region using `CreateRoundRectRgn`.
- Applies the clipping region to `Shell_TrayWnd` via `SetWindowRgn(hwnd, rgn, TRUE)`.
- Updates dynamically as applications launch or close, keeping the dock centered.

---

### 3. Explorer Crash Recovery & Guardian Processes
If Windows Explorer crashes or restarts:
- All custom regions and DWM composition attributes are wiped out by the shell restart.
- `recovery.rs` registers for the broadcast message `RegisterWindowMessageW("TaskbarCreated")`.
- Upon receipt of `TaskbarCreated`, the crate automatically rediscovers the new taskbar handles and reapplies all active styles.
- An isolated guardian process (`--native-taskbar-guardian`) monitors the parent application and guarantees the taskbar is restored to stock Windows appearance if Atelier exits unexpectedly.

---

## Crate Modules

| Module | Source File | Description |
| :--- | :--- | :--- |
| **Discovery** | [`src/discovery.rs`](../../crates/taskbar/src/discovery.rs) | Locates primary and secondary taskbars, tray notification areas, and start buttons. |
| **Accent Composition** | [`src/accent.rs`](../../crates/taskbar/src/accent.rs) | Wraps `SetWindowCompositionAttribute` and `ACCENT_POLICY` definitions. |
| **Style Manager** | [`src/style.rs`](../../crates/taskbar/src/style.rs) | High-level styling presets (Acrylic, Mica, Blur, Clear, Colored). |
| **Dock Region** | [`src/dock.rs`](../../crates/taskbar/src/dock.rs) | Centered floating dock shape generation using Win32 GDI regions. |
| **Position & Guardian** | [`src/position.rs`](../../crates/taskbar/src/position.rs) | Screen edge positioning, workarea preservation, and background guardian monitoring. |
| **Recovery** | [`src/recovery.rs`](../../crates/taskbar/src/recovery.rs) | Event loop listening for `TaskbarCreated` shell recreation events. |
| **XAML Loader** | [`src/xaml.rs`](../../crates/taskbar/src/xaml.rs) | Runtime extractor and loader for the embedded C++20 XAML bridge DLL. |
| **Native Bridge** | [`native/xaml_bridge.cpp`](../../crates/taskbar/native/xaml_bridge.cpp) | Modern Windows 11 WinUI visual tree traversal and element manipulation. |

---

## Testing

```powershell
# Run unit tests for atelier-taskbar
cargo test -p atelier-taskbar
```
Live desktop tests are ignored by default to avoid altering your running environment during CI.
