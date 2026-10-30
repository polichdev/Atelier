# atelier-core

Directory: [`crates/core`](../../crates/core/) · Cargo package: `atelier-core`

`atelier-core` contains the native background daemon, standalone CLI runner, system tray integration, and detached watchdog processes that safeguard Windows shell stability.

```mermaid
flowchart TD
    subgraph ProcessEntry ["Process Entry & Argument Router (main.rs)"]
        CLI["CLI Invocations<br/>Atelier.exe"]
        MutexCheck{"Single Instance Check<br/>(Named Mutex)"}
        ModeSwitch{"Argument Mode"}
    end

    CLI --> MutexCheck
    MutexCheck -- "First Instance" --> ModeSwitch
    MutexCheck -- "Already Running" --> Focus["Signal Running Instance & Exit"]

    subgraph Modes ["Operational Execution Modes"]
        Daemon["Default Native Daemon<br/>(Tray Icon, IPC, Engine Supervisor)"]
        DockWatchdog["--dock-watchdog<br/>(Watches Explorer & Restores Taskbars)"]
        AppBarWatchdog["--dock-appbar-watchdog<br/>(Monitors Workarea AppBar State)"]
        TaskbarGuardian["--native-taskbar-guardian<br/>(Failsafe Taskbar Restorer)"]
    end

    ModeSwitch --> Daemon
    ModeSwitch --> DockWatchdog
    ModeSwitch --> AppBarWatchdog
    ModeSwitch --> TaskbarGuardian

    subgraph NativeServices ["Native Win32 Services"]
        Tray["System Tray Icon (Shell_NotifyIconW)"]
        TaskbarMgr["atelier-taskbar Instance"]
        AudioMgr["atelier-audio Instance"]
        WallpaperEng["atelier-wallpaper Instance"]
    end

    Daemon --> Tray
    Daemon --> TaskbarMgr
    Daemon --> AudioMgr
    Daemon --> WallpaperEng
```

---

## Architectural Highlights

### 1. Guardian and Watchdog Processes
To guarantee that the user's desktop never remains in an unusable state if an unexpected crash occurs, `atelier-core` can launch itself as a lightweight detached watchdog:
- **`--dock-watchdog`**: Monitors the parent application process ID. If the parent process terminates without gracefully unhiding the Windows taskbar, the watchdog immediately unhides all `Shell_TrayWnd` and `Shell_SecondaryTrayWnd` windows.
- **`--dock-appbar-watchdog`**: Ensures that temporary desktop workarea reservations registered with the `SHAppBarMessage` API are cleanly deregistered (`ABM_REMOVE`), preventing persistent screen margins when the application closes.
- **`--native-taskbar-guardian`**: Monitors the health of Explorer's native taskbar and reapplies default taskbar window styles if custom styling hooks fail.

```mermaid
sequenceDiagram
    autonumber
    participant App as Atelier Main Process
    participant Watchdog as --dock-watchdog Process
    participant Explorer as Windows Explorer Shell

    App->>Watchdog: Spawn detached child process with parent PID
    App->>Explorer: Hide native taskbars (ShowWindow SW_HIDE)
    Note over App,Watchdog: Watchdog polls parent process handle (SYNCHRONIZE)
    alt Graceful Exit
        App->>Explorer: Restore taskbars (ShowWindow SW_SHOW)
        App->>Watchdog: Signal clean shutdown & terminate
    else Unexpected Crash / Kill
        App--xApp: Process killed unexpectedly
        Watchdog->>Watchdog: WaitForSingleObject detects parent exit
        Watchdog->>Explorer: Emergency restore all taskbars (SW_SHOW)
        Watchdog->>Watchdog: Exit cleanly
    end
```

---

### 2. Native System Tray Notification
- Creates a dedicated hidden message window to receive `WM_USER` taskbar notifications.
- Adds an icon to the Windows notification area via `Shell_NotifyIconW`.
- Provides context menu shortcuts for opening Studio, pausing wallpapers, resetting appearance to Windows defaults, or safely quitting the application.

---

## Testing & CLI Usage

```powershell
# Launch daemon in standalone mode
cargo run -p atelier-core

# Run watchdog test harness
cargo run -p atelier-core -- --dock-watchdog 1234
```
